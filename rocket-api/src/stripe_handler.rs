use rocket::serde::{Deserialize, Serialize};
use stripe::{
    Client, PaymentIntent, PaymentIntentStatus, Currency,
};
use stripe::StripeError;
use std::str::FromStr;
use p256::{
    ecdsa::{SigningKey, Signature, signature::Signer},
    elliptic_curve::sec1::ToEncodedPoint,
    PublicKey,
};

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
    let blinded_public_key = PublicKey::from_sec1_bytes(&hex::decode(request.blinded_public_key)?)?;

    // Sign the blinded public key
    let blind_signature: Signature = signing_key.sign(blinded_public_key.as_affine().to_encoded_point(false).as_bytes());

    Ok(SignCertificateResponse {
        blind_signature: hex::encode(blind_signature.to_bytes()),
    })
}

// The create_payment_intent function and associated structs have been removed
