use rocket::serde::{json::Json, Deserialize, Serialize};
use stripe::{Client, CreatePaymentIntent, Currency, PaymentIntent};

#[derive(Deserialize)]
pub struct DonationRequest {
    amount: u64,
    currency: String,
}

#[derive(Serialize)]
pub struct DonationResponse {
    client_secret: String,
}

pub async fn create_payment_intent(donation: DonationRequest) -> Result<DonationResponse, stripe::Error> {
    let secret_key = std::env::var("STRIPE_SECRET_KEY").expect("Missing STRIPE_SECRET_KEY in env");
    let client = Client::new(secret_key);

    let mut payment_intent = CreatePaymentIntent::new(donation.amount, Currency::from_str(&donation.currency).unwrap());
    payment_intent.statement_descriptor("Freenet Donation");

    let pi: PaymentIntent = PaymentIntent::create(&client, payment_intent).await?;

    Ok(DonationResponse {
        client_secret: pi.client_secret.unwrap(),
    })
}
