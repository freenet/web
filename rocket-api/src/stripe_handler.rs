use rocket::serde::{Deserialize, Serialize};
use stripe::{
    Client, CreatePaymentIntent, Currency, PaymentIntent,
    PaymentIntentStatus, CreateCustomer, Customer,
};
use stripe::StripeError;
use std::str::FromStr;

pub async fn verify_payment_intent(payment_intent_id: String) -> Result<bool, Box<dyn std::error::Error>> {
    let secret_key = std::env::var("STRIPE_SECRET_KEY").expect("Missing STRIPE_SECRET_KEY in env");
    let client = Client::new(secret_key);

    let pi = PaymentIntent::retrieve(&client, &payment_intent_id, &[]).await?;

    Ok(pi.status == PaymentIntentStatus::Succeeded)
}

#[derive(Deserialize)]
pub struct DonationRequest {
    amount: u64,
    currency: String,
    name: Option<String>,
    email: Option<String>,
}

#[derive(Serialize)]
pub struct DonationResponse {
    client_secret: String,
    customer_id: String,
}

pub async fn create_payment_intent(donation: DonationRequest) -> Result<DonationResponse, Box<dyn std::error::Error>> {
    let secret_key = std::env::var("STRIPE_SECRET_KEY").expect("Missing STRIPE_SECRET_KEY in env");
    let client = Client::new(secret_key);

    // Create a customer
    let customer = Customer::create(
        &client,
        CreateCustomer {
            name: donation.name.as_deref(),
            email: donation.email.as_deref(),
            description: Some("Freenet Donor"),
            metadata: Some(std::collections::HashMap::from([
                (String::from("donation_source"), String::from("freenet_website")),
            ])),
            ..Default::default()
        },
    )
    .await?;

    let currency = Currency::from_str(&donation.currency)
        .map_err(|_| StripeError::ClientError("Invalid currency".to_string()))?;

    let mut create_intent = CreatePaymentIntent::new(donation.amount as i64, currency);
    create_intent.statement_descriptor_suffix = Some("Freenet");
    create_intent.customer = Some(customer.id.clone());
    create_intent.metadata = Some(std::collections::HashMap::from([
        (String::from("donation_type"), String::from("one_time")),
    ]));

    let pi = PaymentIntent::create(&client, create_intent).await?;

    match pi.status {
        PaymentIntentStatus::RequiresPaymentMethod => {
            Ok(DonationResponse {
                client_secret: pi.client_secret.unwrap(),
                customer_id: customer.id.to_string(),
            })
        }
        _ => Err(Box::new(StripeError::ClientError("Unexpected PaymentIntent status".to_string())))
    }
}
