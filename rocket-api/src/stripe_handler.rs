use rocket::serde::{Deserialize, Serialize};
use stripe::{Client, PaymentIntent, PaymentIntentStatus};
use std::str::FromStr;
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose};
use std::error::Error as StdError;
use crate::fn_key_util::{DelegatedKey, Certificate, sign_certificate as util_sign_certificate, PublicKey};

#[derive(Debug)]
pub enum CertificateError {
    StripeError(stripe::StripeError),
    PaymentNotSuccessful,
    CertificateAlreadySigned,
    SigningError(String),
    Base64Error(base64::DecodeError),
    KeyError(String),
    ParseIdError(stripe::ParseIdError),
    DelegatedKeyNotFound,
}

impl std::fmt::Display for CertificateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CertificateError::StripeError(e) => write!(f, "Stripe error: {}", e),
            CertificateError::PaymentNotSuccessful => write!(f, "Payment not successful"),
            CertificateError::CertificateAlreadySigned => write!(f, "Certificate already signed"),
            CertificateError::SigningError(e) => write!(f, "Signing error: {}", e),
            CertificateError::Base64Error(e) => write!(f, "Base64 decoding error: {}", e),
            CertificateError::KeyError(e) => write!(f, "Key error: {}", e),
            CertificateError::ParseIdError(e) => write!(f, "Parse ID error: {}", e),
            CertificateError::DelegatedKeyNotFound => write!(f, "Delegated key not found"),
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
    public_key: String,
}

#[derive(Debug, Serialize)]
pub struct SignCertificateResponse {
    pub certificate: String,
}

pub async fn sign_certificate(request: SignCertificateRequest) -> Result<SignCertificateResponse, CertificateError> {
    log::info!("Starting sign_certificate function with request: {:?}", request);

    let stripe_secret_key = std::env::var("STRIPE_SECRET_KEY")
        .map_err(|e| {
            log::error!("Environment variable STRIPE_SECRET_KEY not found: {}", e);
            CertificateError::KeyError("STRIPE_SECRET_KEY not found".to_string())
        })?;
    let client = Client::new(stripe_secret_key);

    // Verify payment intent
    let pi = PaymentIntent::retrieve(&client, &stripe::PaymentIntentId::from_str(&request.payment_intent_id)?, &[]).await?;
    if pi.status != PaymentIntentStatus::Succeeded {
        return Err(CertificateError::PaymentNotSuccessful);
    }

    // Check if the certificate has already been signed
    if pi.metadata.get("certificate_signed").is_some() {
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

    let delegated_key = load_delegated_key()?;
    let public_key = general_purpose::STANDARD.decode(&request.public_key)?;

    let certificate = util_sign_certificate(&delegated_key, &public_key);

    let certificate_bytes = serde_json::to_vec(&certificate)
        .map_err(|e| CertificateError::SigningError(e.to_string()))?;

    log::info!("Certificate signed successfully");

    Ok(SignCertificateResponse {
        certificate: general_purpose::STANDARD.encode(certificate_bytes),
    })
}

fn load_delegated_key() -> Result<DelegatedKey, CertificateError> {
    // In a production environment, you would load this from a secure storage
    // For this example, we'll load it from an environment variable
    let delegated_key_base64 = std::env::var("DELEGATED_KEY")
        .map_err(|_| CertificateError::DelegatedKeyNotFound)?;

    let delegated_key_bytes = general_purpose::STANDARD.decode(delegated_key_base64)?;

    serde_json::from_slice(&delegated_key_bytes)
        .map_err(|e| CertificateError::KeyError(e.to_string()))
}
