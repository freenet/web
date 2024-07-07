use rocket::serde::{Deserialize, Serialize};
use stripe::{Client, PaymentIntent, PaymentIntentStatus, Metadata};
use std::str::FromStr;
use std::collections::HashMap;
use p256::{
    ecdsa::{SigningKey, Signature, signature::Signer},
    elliptic_curve::sec1::ToEncodedPoint,
    PublicKey,
};
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Deserialize)]
pub struct SignCertificateRequest {
    payment_intent_id: String,
    blinded_public_key: String,
}

#[derive(Debug, Serialize)]
pub struct SignCertificateResponse {
    blind_signature: String,
}

pub async fn sign_certificate(request: SignCertificateRequest) -> Result<SignCertificateResponse, Box<dyn std::error::Error>> {
    let secret_key = std::env::var("STRIPE_SECRET_KEY").expect("Missing STRIPE_SECRET_KEY in env");
    let client = Client::new(secret_key);

    // Verify payment intent
    let mut pi = PaymentIntent::retrieve(&client, &stripe::PaymentIntentId::from_str(&request.payment_intent_id)?, &[]).await?;
    if pi.status != PaymentIntentStatus::Succeeded {
        return Err("Payment not successful".into());
    }

    // Check if the certificate has already been signed
    if pi.metadata.get("certificate_signed").is_some() {
        return Err("Certificate already signed for this payment".into());
    }

    // Mark the payment intent as used for certificate signing
    let mut metadata = HashMap::new();
    metadata.insert("certificate_signed".to_string(), "true".to_string());
    let params = stripe::PaymentIntentUpdateParams {
        metadata: Some(Metadata::from(metadata)),
        ..Default::default()
    };
    pi = PaymentIntent::update(&client, &pi.id, params).await?;

    // Load the server's signing key
    let server_secret_key = std::env::var("SERVER_SIGNING_KEY").expect("Missing SERVER_SIGNING_KEY in env");
    let signing_key = SigningKey::from_slice(&general_purpose::STANDARD.decode(server_secret_key)?)?;

    // Parse the blinded public key
    let blinded_public_key = PublicKey::from_sec1_bytes(&general_purpose::STANDARD.decode(&request.blinded_public_key)?)?;

    // Sign the blinded public key
    let blind_signature: Signature = signing_key.sign(blinded_public_key.as_affine().to_encoded_point(false).as_bytes());

    Ok(SignCertificateResponse {
        blind_signature: general_purpose::STANDARD.encode(blind_signature.to_bytes()),
    })
}

// The create_payment_intent function and associated structs have been removed
