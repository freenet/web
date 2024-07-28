use std::collections::HashMap;
use std::error::Error as StdError;
use std::str::FromStr;

use base64::Engine as _;
use base64::Engine;
use blind_rsa_signatures::BlindedMessage;
use rocket::serde::{Deserialize, Serialize};
use sha2::Digest;
use stripe::{Client, PaymentIntent, PaymentIntentStatus};

use ghostkey::armorable::*;
use ghostkey::armorable::Armorable;

use crate::delegates::sign_with_delegate_key;
use crate::errors::CertificateError;

#[derive(Debug, Deserialize)]
pub struct SignCertificateRequest {
    payment_intent_id: String,
    blinded_ghostkey_base64: String,
}

#[derive(Debug, Serialize)]
pub struct SignCertificateResponse {
    pub blind_signature_base64: String,
}

pub async fn sign_certificate(request: SignCertificateRequest) -> Result<SignCertificateResponse, CertificateError> {
    log::info!("Starting sign_certificate function with request: {:?}", request);
    log::debug!("Current working directory: {:?}", std::env::current_dir());
    log::debug!("HOME environment variable: {:?}", std::env::var("HOME"));

    let stripe_secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(|e| {
        log::error!("Environment variable STRIPE_SECRET_KEY not found: {}", e);
        log::error!("Current environment variables: {:?}", std::env::vars().collect::<Vec<_>>());
        CertificateError::KeyError("STRIPE_SECRET_KEY environment variable not set".to_string())
    })?;

    log::info!("STRIPE_SECRET_KEY found");
    let client = Client::new(stripe_secret_key);

    // Verify payment intent
    let pi = PaymentIntent::retrieve(&client, &stripe::PaymentIntentId::from_str(&request.payment_intent_id)?, &[]).await
        .map_err(|e| {
            log::error!("Failed to retrieve PaymentIntent: {:?}", e);
            CertificateError::StripeError(e)
        })?;

    log::info!("Retrieved PaymentIntent: {:?}", pi);
    log::info!("PaymentIntent status: {:?}", pi.status);

    match pi.status {
        PaymentIntentStatus::Succeeded => {
            // Proceed with certificate signing
        },
        PaymentIntentStatus::RequiresPaymentMethod => {
            log::error!("Payment method is missing. Status: {:?}", pi.status);
            return Err(CertificateError::PaymentMethodMissing);
        },
        _ => {
            log::error!("Payment not successful. Status: {:?}", pi.status);
            return Err(CertificateError::PaymentNotSuccessful);
        }
    }

    // Check if the certificate has already been signed
    if pi.metadata.get("certificate_signed").is_some() {
        log::warn!("Certificate already signed for PaymentIntent: {}", pi.id);
        return Err(CertificateError::CertificateAlreadySigned);
    }

    // Mark the payment intent as used for certificate signing
    let mut metadata = HashMap::new();
    metadata.insert("certificate_signed".to_string(), "true".to_string());
    let params = stripe::UpdatePaymentIntent {
        metadata: Some(metadata),
        ..Default::default()
    };
    PaymentIntent::update(&client, &pi.id, params).await?;

    // Sign the certificate
    log::info!("Payment intent verified successfully");

    let blinded_ghostkey = BlindedMessage::from_base64(&request.blinded_ghostkey_base64)
        .map_err(|e| {
            log::error!("Error in from_base64: {:?}", e);
            CertificateError::Base64Error(e)
        })?;

    let amount = pi.amount;
    let blind_signature = sign_with_delegate_key(&blinded_ghostkey, amount)
        .map_err(|e| {
            log::error!("Error in sign_with_delegate_key: {:?}", e);
            e
        })?;

    Ok(SignCertificateResponse {
        blind_signature_base64: blind_signature.to_base64(),
    })
}