use rocket::serde::{Deserialize, Serialize};
use stripe::{Client, PaymentIntent, PaymentIntentStatus};
use std::str::FromStr;
use std::collections::HashMap;
use p256::{
    ecdsa::{self, SigningKey, signature::Signer},
    elliptic_curve::sec1::ToEncodedPoint,
    PublicKey, SecretKey,
};
use rand_core::OsRng;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};
use std::error::Error as StdError;

#[derive(Debug)]
pub enum CertificateError {
    StripeError(stripe::StripeError),
    PaymentNotSuccessful,
    CertificateAlreadySigned,
    SigningError(String),
    Base64Error(base64::DecodeError),
    KeyError(String),
    ParseIdError(stripe::ParseIdError),
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
    blinded_public_key: String,
}

fn pad_base64(base64_str: &str) -> String {
    let mut padded = base64_str.to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }
    padded
}

#[derive(Debug, Serialize)]
pub struct SignCertificateResponse {
    pub blind_signature: String,
}

pub async fn sign_certificate(request: SignCertificateRequest) -> Result<SignCertificateResponse, CertificateError> {
    log::info!("Starting sign_certificate function with request: {:?}", request);

    let stripe_secret_key = match std::env::var("STRIPE_SECRET_KEY") {
        Ok(key) => {
            log::info!("STRIPE_SECRET_KEY found: {}", key);
            key
        },
        Err(e) => {
            log::error!("Environment variable STRIPE_SECRET_KEY not found: {}", e);
            log::error!("Current environment variables: {:?}", std::env::vars().collect::<Vec<_>>());
            panic!("STRIPE_SECRET_KEY environment variable not set");
        }
    };
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

    let signature = sign_with_key(&request.blinded_public_key).map_err(|e| {
        log::error!("Error in sign_with_key: {:?}", e);
        match e {
            CertificateError::Base64Error(be) => {
                log::error!("Base64 decoding error: {}", be);
                CertificateError::Base64Error(be)
            },
            CertificateError::KeyError(ke) => {
                log::error!("Key error: {}", ke);
                CertificateError::KeyError(ke)
            },
            _ => e,
        }
    })?;

    log::info!("Certificate signed successfully");
    log::debug!("Signature: {}", signature);

    Ok(SignCertificateResponse { blind_signature: signature })
}

fn sign_with_key(blinded_public_key: &str) -> Result<String, CertificateError> {
    let server_secret_key = match std::env::var("SERVER_SIGNING_KEY") {
        Ok(key) => {
            log::info!("SERVER_SIGNING_KEY found");
            key
        },
        Err(e) => {
            log::error!("Environment variable SERVER_SIGNING_KEY not found: {}", e);
            log::error!("Current environment variables: {:?}", std::env::vars().collect::<Vec<_>>());
            panic!("SERVER_SIGNING_KEY environment variable not set");
        }
    };
    log::info!("Starting sign_with_key function with blinded_public_key: {}", blinded_public_key);

    let signing_key = SigningKey::from_slice(&general_purpose::STANDARD.decode(pad_base64(&server_secret_key))?)
        .map_err(|e| {
            log::error!("Failed to create signing key: {}", e);
            CertificateError::KeyError(e.to_string())
        })?;

    // Parse the blinded public key from JWK format
    let blinded_public_key_json: serde_json::Value = serde_json::from_str(blinded_public_key)
        .map_err(|e| {
            log::error!("Failed to parse blinded public key JSON: {}. Received blinded_public_key: '{}'", e, blinded_public_key);
            CertificateError::KeyError(format!("Invalid JSON: {}", e))
        })?;

    log::debug!("Parsed blinded public key JSON: {:?}", blinded_public_key_json);

    let x = blinded_public_key_json["x"].as_str()
        .ok_or_else(|| {
            log::error!("Missing 'x' coordinate in blinded public key JSON");
            CertificateError::KeyError("Missing 'x' coordinate".to_string())
        })?;
    let y = blinded_public_key_json["y"].as_str()
        .ok_or_else(|| {
            log::error!("Missing 'y' coordinate in blinded public key JSON");
            CertificateError::KeyError("Missing 'y' coordinate".to_string())
        })?;

    let x = general_purpose::STANDARD.decode(x)
        .map_err(|e| {
            log::error!("Failed to decode 'x' coordinate: {}", e);
            CertificateError::Base64Error(e)
        })?;
    let y = general_purpose::STANDARD.decode(y)
        .map_err(|e| {
            log::error!("Failed to decode 'y' coordinate: {}", e);
            CertificateError::Base64Error(e)
        })?;

    let mut public_key_bytes = vec![0x04]; // Uncompressed point format
    public_key_bytes.extend_from_slice(&x);
    public_key_bytes.extend_from_slice(&y);

    let blinded_public_key = PublicKey::from_sec1_bytes(&public_key_bytes)
        .map_err(|e| {
            log::error!("Failed to create public key from bytes: {}", e);
            CertificateError::KeyError(e.to_string())
        })?;
    log::debug!("Parsed blinded public key: {:?}", blinded_public_key.to_encoded_point(false));

    // Generate a random nonce
    let nonce = SecretKey::random(&mut OsRng);
    let nonce_bytes = nonce.to_bytes();

    // Combine the blinded public key and nonce, and hash them
    let mut hasher = Sha256::new();
    hasher.update(blinded_public_key.to_encoded_point(false).as_bytes());
    hasher.update(&nonce_bytes);
    let message = hasher.finalize();

    // Sign the hash
    let blind_signature: ecdsa::Signature = signing_key.sign(&message);

    // Combine the signature and nonce
    let mut combined = blind_signature.to_vec();
    combined.extend_from_slice(&nonce_bytes);

    Ok(general_purpose::STANDARD.encode(combined))
}

// The create_payment_intent function and associated structs have been removed
