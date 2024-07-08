use rocket::serde::{Deserialize, Serialize};
use stripe::{Client, PaymentIntent, PaymentIntentStatus};
use std::str::FromStr;
use std::collections::HashMap;
use p256::{
    ecdsa::{self, SigningKey, Signature},
    elliptic_curve::sec1::ToEncodedPoint,
    PublicKey, SecretKey,
};
use rand_core::OsRng;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

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

pub async fn sign_certificate(request: SignCertificateRequest) -> Result<SignCertificateResponse, Box<dyn std::error::Error>> {
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
        return Err("Payment not successful".into());
    }

    // Check if the certificate has already been signed
    if pi.metadata.get("certificate_signed").is_some() {
        return Err("CERTIFICATE_ALREADY_SIGNED".into());
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

    let signature = match sign_with_key(&request.blinded_public_key) {
        Ok(sig) => {
            log::info!("Certificate signed successfully");
            sig
        },
        Err(e) => {
            log::error!("Error in sign_with_key: {}", e);
            return Err(Box::new(e));
        }
    };

    Ok(SignCertificateResponse { blind_signature: signature })
}

fn sign_with_key(blinded_public_key: &str) -> Result<String, Box<dyn std::error::Error>> {
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

    let signing_key = match SigningKey::from_slice(&general_purpose::STANDARD.decode(pad_base64(&server_secret_key)).map_err(|e| {
        log::error!("Failed to decode SERVER_SIGNING_KEY: {}", e);
        e
    })?) {
        Ok(key) => key,
        Err(e) => {
            log::error!("Failed to create signing key: {}", e);
            return Err(Box::new(e));
        }
    };

    // Parse the blinded public key
    let blinded_public_key = match PublicKey::from_sec1_bytes(&general_purpose::STANDARD.decode(pad_base64(blinded_public_key)).map_err(|e| {
        log::error!("Failed to decode blinded public key: {}", e);
        e
    })?) {
        Ok(key) => key,
        Err(e) => {
            log::error!("Failed to parse blinded public key: {}", e);
            return Err(e);
        }
    };

    // Generate a random nonce
    let nonce = SecretKey::random(&mut OsRng);
    let nonce_bytes = nonce.to_bytes();

    // Combine the blinded public key and nonce, and hash them
    let mut hasher = Sha256::new();
    hasher.update(blinded_public_key.as_affine().to_encoded_point(false).as_bytes());
    hasher.update(&nonce_bytes);
    let message = hasher.finalize();

    // Sign the hash
    let blind_signature: Signature = match ecdsa::signature::Signer::sign(&signing_key, &message) {
        Ok(sig) => sig,
        Err(e) => {
            log::error!("Failed to sign the message: {}", e);
            return Err(Box::new(e));
        }
    };

    // Combine the signature and nonce
    let mut combined = blind_signature.to_bytes().to_vec();
    combined.extend_from_slice(&nonce_bytes);

    Ok(general_purpose::STANDARD.encode(combined))
}

// The create_payment_intent function and associated structs have been removed
