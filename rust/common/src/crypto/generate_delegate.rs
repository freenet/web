use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use base64::{engine::general_purpose, Engine as _};
use p256::ecdsa::{self, signature::Signer};
use crate::armor;
use serde::{Serialize, Deserialize};
use rmp_serde;
use crate::crypto::{CryptoError, extract_base64_from_armor};

#[derive(Serialize, Deserialize, Debug)]
pub struct DelegateKeyCertificate {
    pub verifying_key: Vec<u8>,
    pub attributes: String,
    pub signature: Vec<u8>,
}

pub fn generate_delegate_key(master_signing_key_pem: &str, attributes: &str) -> Result<String, CryptoError> {
    println!("Generating delegate key with attributes: {}", attributes);
    println!("Master signing key PEM: {}", master_signing_key_pem);

    let master_signing_key_base64 = extract_base64_from_armor(master_signing_key_pem, "MASTER SIGNING KEY")?;
    println!("Extracted base64: {}", master_signing_key_base64);

    // Trim any whitespace from the base64 string
    let trimmed_base64 = master_signing_key_base64.trim();
    println!("Trimmed base64: {}", trimmed_base64);

    let master_signing_key_bytes = general_purpose::STANDARD.decode(trimmed_base64)
        .map_err(|e| {
            println!("Base64 decode error: {}. Attempted to decode: {}", e, trimmed_base64);
            CryptoError::Base64DecodeError(format!("{}: {}", e, trimmed_base64))
        })?;
    println!("Decoded key bytes: {:?}", master_signing_key_bytes);

    let master_signing_key = SigningKey::from_slice(&master_signing_key_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;
    println!("Created SigningKey successfully");

    // Generate the delegate key pair
    let delegate_signing_key = SigningKey::random(&mut OsRng);
    let delegate_verifying_key = VerifyingKey::from(&delegate_signing_key);

    // Serialize the verifying key and attributes
    let verifying_key_bytes = delegate_verifying_key.to_encoded_point(false).as_bytes().to_vec();
    let certificate_data = DelegateKeyCertificate {
        verifying_key: verifying_key_bytes.clone(),
        attributes: attributes.to_string(),
        signature: vec![],
    };
    let certificate_data_bytes = rmp_serde::to_vec(&certificate_data)
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    // Sign the certificate data
    let signature: ecdsa::Signature = master_signing_key.sign(&certificate_data_bytes);
    let signed_certificate_data = DelegateKeyCertificate {
        verifying_key: verifying_key_bytes,
        attributes: attributes.to_string(),
        signature: signature.to_vec(),
    };

    // Serialize the signed certificate data using bincode
    let signed_certificate_bytes = rmp_serde::to_vec(&signed_certificate_data)
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    println!("Serialized certificate: {:?}", signed_certificate_bytes);

    // Armor the signed certificate directly (without base64 encoding)
    let armored_delegate_certificate = armor(&signed_certificate_bytes, "DELEGATE CERTIFICATE", "DELEGATE CERTIFICATE");

    Ok(armored_delegate_certificate)
}
