use rocket::serde::{Deserialize, Serialize};
use async_stripe::{
    Client, CreatePaymentIntent, Currency, PaymentIntent,
    PaymentIntentStatus,
};
use std::str::FromStr;

#[derive(Deserialize)]
pub struct DonationRequest {
    amount: u64,
    currency: String,
}

#[derive(Serialize)]
pub struct DonationResponse {
    client_secret: String,
}

pub async fn create_payment_intent(donation: DonationRequest) -> Result<DonationResponse, async_stripe::Error> {
    let secret_key = std::env::var("STRIPE_SECRET_KEY").expect("Missing STRIPE_SECRET_KEY in env");
    let client = Client::new(secret_key);

    let currency = Currency::from_str(&donation.currency)
        .map_err(|_| async_stripe::Error::Unexpected("Invalid currency".to_string()))?;

    let mut create_intent = CreatePaymentIntent::new(donation.amount, currency);
    create_intent.statement_descriptor = Some("Freenet Donation");

    let pi = PaymentIntent::create(&client, create_intent).await?;

    match pi.status {
        PaymentIntentStatus::Succeeded | PaymentIntentStatus::RequiresPaymentMethod => {
            Ok(DonationResponse {
                client_secret: pi.client_secret.unwrap(),
            })
        }
        _ => Err(async_stripe::Error::Unexpected(format!("Unexpected payment intent status: {:?}", pi.status)))
    }
}
