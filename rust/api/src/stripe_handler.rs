use std::collections::HashMap;
use std::error::Error as StdError;
use std::path::PathBuf;
use std::str::FromStr;

use base64::{Engine as _, engine::general_purpose};
use ed25519_dalek::SigningKey;
use rand_core::OsRng;
use rocket::serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use stripe::{Client, PaymentIntent, PaymentIntentStatus};

use ghostkey::armorable::*;
use ghostkey::delegate_certificate::DelegateCertificate;

#[derive(Debug)]
pub enum CertificateError {
    StripeError(stripe::StripeError),
    PaymentNotSuccessful,
    PaymentMethodMissing,
    CertificateAlreadySigned,
    Base64Error(base64::DecodeError),
    KeyError(String),
    ParseIdError(stripe::ParseIdError),
}

impl std::fmt::Display for CertificateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CertificateError::StripeError(e) => write!(f, "Stripe error: {}", e),
            CertificateError::PaymentNotSuccessful => write!(f, "Payment not successful"),
            CertificateError::PaymentMethodMissing => write!(f, "Payment method is missing"),
            CertificateError::CertificateAlreadySigned => write!(f, "Certificate already signed"),
            CertificateError::Base64Error(e) => write!(f, "Base64 decoding error: {}", e),
            CertificateError::KeyError(e) => write!(f, "Key error: {}", e),
            CertificateError::ParseIdError(e) => write!(f, "Parse ID error: {}", e),
        }
    }
}

impl StdError for CertificateError {}

impl From<stripe::StripeError> for CertificateError {
    fn from(error: stripe::StripeError) -> Self {
        CertificateError::StripeError(error)
    }
}

impl From<base64::DecodeError> for CertificateError {
    fn from(error: base64::DecodeError) -> Self {
        CertificateError::Base64Error(error)
    }
}

impl From<stripe::ParseIdError> for CertificateError {
    fn from(error: stripe::ParseIdError) -> Self {
        CertificateError::ParseIdError(error)
    }
}

#[derive(Debug, Deserialize)]
pub struct SignCertificateRequest {
    payment_intent_id: String,
    blinded_public_key: Value,
}

#[derive(Debug, Serialize)]
pub struct DelegateInfo {
    pub certificate: String,
    pub amount: u64,
}

impl Default for DelegateInfo {
    fn default() -> Self {
        DelegateInfo {
            certificate: String::new(),
            amount: 0,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SignCertificateResponse {
    pub blind_signature: String,
    pub delegate_info: DelegateInfo,
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

    let amount = pi.amount;
    let (signature, delegate_info) = sign_with_delegate_key(&request.blinded_public_key, amount)
        .map_err(|e| {
            log::error!("Error in sign_with_delegate_key: {:?}", e);
            e
        })?;

    log::info!("Certificate signed successfully");
    log::debug!("Signature: {}", signature);

    Ok(SignCertificateResponse { 
        blind_signature: signature,
        delegate_info,
    })
}

fn sign_with_delegate_key(blinded_verifying_key_bytes: &[u8], amount: i64) -> Result<(String, DelegateInfo), CertificateError> {
    let delegate_dir = PathBuf::from(std::env::var("DELEGATE_DIR").map_err(|e| {
        log::error!("DELEGATE_DIR environment variable not set: {}", e);
        CertificateError::KeyError("DELEGATE_DIR environment variable not set".to_string())
    })?);

    let delegate_amount = (amount / 100) as u64; // Convert cents to dollars
    let delegate_cert_path = delegate_dir.join(format!("delegate_certificate_{}.pem", delegate_amount));
    let delegate_key_path = delegate_dir.join(format!("delegate_signing_key_{}.pem", delegate_amount));

    log::info!("Reading delegate certificate from: {:?}", delegate_cert_path);
    log::info!("Reading delegate key from: {:?}", delegate_key_path);
    
    let delegate_cert = DelegateCertificate::from_file(&delegate_cert_path)
        .map_err(|e| {
            log::error!("Failed to read delegate certificate from {:?}: {}", delegate_cert_path, e);
            CertificateError::KeyError(format!("Failed to read delegate certificate: {}", e))
        })?;

    let delegate_key = SigningKey::from_file(&delegate_key_path)
        .map_err(|e| {
            log::error!("Failed to read delegate key from {:?}: {}", delegate_key_path, e);
            CertificateError::KeyError(format!("Failed to read delegate key: {}", e))
        })?;

    log::info!("Successfully read both delegate certificate and key");
    log::debug!("Starting sign_with_delegate_key function with blinded_verifying_key, length: {:?}", blinded_verifying_key_bytes.len());


    log::info!("Successfully created signing key");
 
    todo!()
    /*
    let blinded_verifying_key_bytes = 

    log::debug!("Decoded blinded verifying key bytes: {:?}", blinded_verifying_key_bytes);


    
    Ok((general_purpose::STANDARD.encode(&combined), delegate_info)) */
}

// The create_payment_intent function and associated structs have been removed
