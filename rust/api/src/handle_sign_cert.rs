use std::collections::HashMap;
use std::str::FromStr;

use blind_rsa_signatures::BlindedMessage;
use serde::{Deserialize, Serialize};
use stripe::{Client, PaymentIntent, PaymentIntentStatus};

use ghostkey_lib::armorable::Armorable;

use crate::delegates::sign_with_delegate_key;
pub use crate::errors::CertificateError;

#[derive(Debug, Deserialize)]
pub struct SignCertificateRequest {
    payment_intent_id: String,
    blinded_ghost_key_base64: String,
}

#[derive(Debug, Serialize)]
pub struct SignCertificateResponse {
    pub blind_signature_base64: String,
    pub delegate_certificate_base64: String,
    pub amount: u64,
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

    let blinded_ghostkey = BlindedMessage::from_base64(&request.blinded_ghost_key_base64)
        .map_err(|e| {
            log::error!("Error in from_base64: {:?}", e);
            CertificateError::MiscError(e.to_string())
        })?;

    let amount_cents = pi.amount as u64;
    let amount_dollars = amount_cents / 100;
    let blind_signature = sign_with_delegate_key(&blinded_ghostkey, amount_dollars)
        .map_err(|e| {
            log::error!("Error in sign_with_delegate_key: {:?}", e);
            e
        })?;

    let (delegate_certificate, _) = crate::delegates::get_delegate(amount_dollars)?;
    
    Ok(SignCertificateResponse {
        blind_signature_base64: blind_signature.to_base64().map_err(|e| CertificateError::MiscError(e.to_string()))?,
        // TODO: Shouldn't be needed if this is being stored in localstorage
        delegate_certificate_base64: delegate_certificate.to_base64().map_err(|e| CertificateError::MiscError(e.to_string()))?,
        amount: amount_cents,
    })
}
