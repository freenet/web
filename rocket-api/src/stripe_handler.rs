use rocket::serde::{Deserialize, Serialize};
use stripe::{
    Client, CreatePaymentIntent, Currency, PaymentIntent,
    PaymentIntentStatus, CreateCustomer, Customer,
};
use stripe::StripeError;
use std::str::FromStr;
use p256::{ecdsa::{SigningKey, Signature, signature::Signer}, elliptic_curve::sec1::ToEncodedPoint};

#[derive(Deserialize)]
pub struct SignCertificateRequest {
    payment_intent_id: String,
    blinded_public_key: String,
}

#[derive(Serialize)]
pub struct SignCertificateResponse {
    blind_signature: String,
}

pub async fn sign_certificate(request: SignCertificateRequest) -> Result<SignCertificateResponse, Box<dyn std::error::Error>> {
    let secret_key = std::env::var("STRIPE_SECRET_KEY").expect("Missing STRIPE_SECRET_KEY in env");
    let client = Client::new(secret_key);

    // Verify payment intent
    let pi = PaymentIntent::retrieve(&client, &stripe::PaymentIntentId::from_str(&request.payment_intent_id)?, &[]).await?;
    if pi.status != PaymentIntentStatus::Succeeded {
        return Err("Payment not successful".into());
    }

    // Load the server's signing key
    let server_secret_key = std::env::var("SERVER_SIGNING_KEY").expect("Missing SERVER_SIGNING_KEY in env");
    let signing_key = SigningKey::from_slice(&hex::decode(server_secret_key)?)?;

    // Parse the blinded public key
    let blinded_public_key = p256::PublicKey::from_sec1_bytes(&hex::decode(request.blinded_public_key)?)?;

    // Sign the blinded public key
    let blind_signature: Signature = signing_key.sign(blinded_public_key.as_affine().to_encoded_point(false).as_bytes());

    Ok(SignCertificateResponse {
        blind_signature: hex::encode(blind_signature.to_bytes()),
    })
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
