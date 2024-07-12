use p256::ecdsa::{SigningKey, VerifyingKey, SigningKey as PrivateKey, VerifyingKey as PublicKey};
use rand_core::OsRng;
use base64::{engine::general_purpose, Engine as _};
use std::fs::{File, create_dir_all, read_to_string};
use std::io::Write;
use std::path::Path;
use serde_json::Value;
use sha2::{Sha256, Digest};
use p256::{SecretKey, FieldBytes};
use p256::ecdsa::{self, signature::Signer};
use crate::armor;
use serde::{Serialize, Deserialize};
use serde_json::to_vec as to_vec_named;

#[derive(Debug, PartialEq)]
pub enum CryptoError {
    IoError(String),
    Base64DecodeError(String),
    KeyCreationError(String),
    SerializationError(String),
    InvalidInput(String),
}

#[derive(Serialize, Deserialize)]
struct DelegateKeyCertificate {
    verifying_key: Vec<u8>,
    attributes: String,
    signature: Vec<u8>,
}

pub fn generate_master_key() -> Result<(String, String), CryptoError> {
    // Generate the master private key
    let master_private_key = PrivateKey::random(&mut OsRng);
    let master_public_key = PublicKey::from(&master_private_key);

    // Encode the keys in base64
    let master_private_key_base64 = general_purpose::STANDARD.encode(master_private_key.to_bytes());
    let master_public_key_base64 = general_purpose::STANDARD.encode(master_public_key.to_encoded_point(false).as_bytes());

    // Armor the keys
    let armored_master_private_key = format!("-----BEGIN SERVER MASTER PRIVATE KEY-----\n{}\n-----END SERVER MASTER PRIVATE KEY-----", master_private_key_base64);
    let armored_master_public_key = format!("-----BEGIN SERVER MASTER VERIFYING KEY-----\n{}\n-----END SERVER MASTER VERIFYING KEY-----", master_public_key_base64);

    Ok((armored_master_private_key, armored_master_public_key))
}

pub fn sign_with_key(blinded_public_key: &Value, server_master_private_key: &str) -> Result<String, CryptoError> {
    let decoded_key = general_purpose::STANDARD.decode(server_master_private_key)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let field_bytes = FieldBytes::from_slice(&decoded_key);
    let master_private_key = PrivateKey::from_bytes(field_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    let blinded_public_key_bytes = match blinded_public_key {
        Value::String(s) => general_purpose::STANDARD.decode(s)
            .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?,
        Value::Object(obj) => {
            let x = obj.get("x").and_then(Value::as_str)
                .ok_or_else(|| CryptoError::InvalidInput("Missing 'x' coordinate".to_string()))?;
            let y = obj.get("y").and_then(Value::as_str)
                .ok_or_else(|| CryptoError::InvalidInput("Missing 'y' coordinate".to_string()))?;

            let mut bytes = Vec::new();
            bytes.extend_from_slice(&general_purpose::STANDARD.decode(x)
                .map_err(|e| CryptoError::Base64DecodeError(format!("Failed to decode 'x' coordinate: {}", e)))?);
            bytes.extend_from_slice(&general_purpose::STANDARD.decode(y)
                .map_err(|e| CryptoError::Base64DecodeError(format!("Failed to decode 'y' coordinate: {}", e)))?);
            bytes
        },
        _ => return Err(CryptoError::InvalidInput("Invalid blinded public key format".to_string())),
    };

    // Generate a random nonce
    let nonce = SecretKey::random(&mut OsRng);
    let nonce_bytes = nonce.to_bytes();

    // Combine the blinded public key and nonce, and hash them
    let mut hasher = Sha256::new();
    hasher.update(&blinded_public_key_bytes);
    hasher.update(&nonce_bytes);
    let message = hasher.finalize();

    // Sign the hash
    let blind_signature: ecdsa::Signature = master_private_key.sign(&message);

    // Combine the signature and nonce
    let mut combined = blind_signature.to_vec();
    combined.extend_from_slice(&nonce_bytes);

    Ok(general_purpose::STANDARD.encode(combined))
}

pub fn generate_signing_key() -> Result<(String, String), CryptoError> {
    // Generate the signing key
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);

    // Encode the keys in base64
    let signing_key_base64 = general_purpose::STANDARD.encode(signing_key.to_bytes());
    let verifying_key_base64 = general_purpose::STANDARD.encode(verifying_key.to_encoded_point(false).as_bytes());

    // Armor the keys
    let armored_signing_key = armor(&signing_key_base64.as_bytes(), "SERVER SIGNING KEY", "SERVER SIGNING KEY");
    let armored_verifying_key = armor(&verifying_key_base64.as_bytes(), "SERVER VERIFYING KEY", "SERVER VERIFYING KEY");

    Ok((armored_signing_key, armored_verifying_key))
}

pub fn generate_delegate_key(master_private_key_pem: &str, attributes: &str) -> Result<(String, String), CryptoError> {
    let master_private_key_bytes = general_purpose::STANDARD.decode(master_private_key_pem)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let field_bytes = FieldBytes::from_slice(&master_private_key_bytes);
    let master_private_key = SigningKey::from_bytes(field_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    // Generate the delegate signing key
    let delegate_signing_key = SigningKey::random(&mut OsRng);
    let delegate_verifying_key = VerifyingKey::from(&delegate_signing_key);

    // Serialize the verifying key and attributes
    let verifying_key_bytes = delegate_verifying_key.to_encoded_point(false).as_bytes().to_vec();
    let certificate_data = DelegateKeyCertificate {
        verifying_key: verifying_key_bytes.clone(),
        attributes: attributes.to_string(),
        signature: vec![],
    };
    let certificate_data_bytes = to_vec_named(&certificate_data)
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    // Sign the certificate data
    let signature: ecdsa::Signature = master_private_key.sign(&certificate_data_bytes);
    let mut signed_certificate_data = certificate_data;
    signed_certificate_data.signature = signature.to_vec();

    // Encode the signed certificate data in base64
    let signed_certificate_data_bytes = to_vec_named(&signed_certificate_data)
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;
    let signed_certificate_base64 = general_purpose::STANDARD.encode(signed_certificate_data_bytes);

    // Encode the delegate signing key
    let delegate_signing_key_base64 = general_purpose::STANDARD.encode(delegate_signing_key.to_bytes());
    let armored_delegate_signing_key = armor(&delegate_signing_key_base64.as_bytes(), "DELEGATE SIGNING KEY", "DELEGATE SIGNING KEY");

    Ok((armored_delegate_signing_key, signed_certificate_base64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_master_key() {
        let (private_key, public_key) = generate_master_key().unwrap();
        assert!(private_key.contains("-----BEGIN SERVER MASTER PRIVATE KEY-----"));
        assert!(public_key.contains("-----BEGIN SERVER MASTER VERIFYING KEY-----"));
    }

    #[test]
    fn test_sign_with_key() {
        let (private_key, _) = generate_master_key().unwrap();
        let blinded_public_key = json!({
            "x": general_purpose::STANDARD.encode([1u8; 32]),
            "y": general_purpose::STANDARD.encode([2u8; 32])
        });
        let signature = sign_with_key(&blinded_public_key, &private_key).unwrap();
        assert!(!signature.is_empty());
    }

    #[test]
    fn test_generate_signing_key() {
        let (signing_key, verifying_key) = generate_signing_key().unwrap();
        assert!(signing_key.contains("-----BEGIN SERVER SIGNING KEY-----"));
        assert!(verifying_key.contains("-----BEGIN SERVER VERIFYING KEY-----"));
    }

    #[test]
    fn test_generate_delegate_key() {
        let (master_private_key, _) = generate_master_key().unwrap();
        let (delegate_signing_key, delegate_certificate) = generate_delegate_key(&master_private_key, "test_attributes").unwrap();
        assert!(delegate_signing_key.contains("-----BEGIN DELEGATE SIGNING KEY-----"));
        assert!(!delegate_certificate.is_empty());
    }
}
