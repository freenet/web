use p256::ecdsa::{SigningKey, VerifyingKey, SigningKey as PrivateKey, VerifyingKey as PublicKey};
use rand_core::OsRng;
use base64::{engine::general_purpose, Engine as _};
use serde_json::Value;
use sha2::{Sha256, Digest};
use p256::{SecretKey, FieldBytes};
use p256::ecdsa::{self, signature::{Signer, Verifier}};
use crate::armor;
use serde::{Serialize, Deserialize};
use rmp_serde::{Serializer, Deserializer};
use colored::Colorize;

use std::fmt;
use std::io::Cursor;

pub fn generate_master_verifying_key(master_signing_key_pem: &str) -> Result<String, CryptoError> {
    let signing_key_base64 = extract_base64_from_armor(master_signing_key_pem, "SERVER MASTER SIGNING KEY")?;

    let signing_key_bytes = general_purpose::STANDARD.decode(&signing_key_base64)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let field_bytes = FieldBytes::from_slice(&signing_key_bytes);
    let signing_key = SigningKey::from_bytes(field_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    let verifying_key = VerifyingKey::from(&signing_key);
    let encoded_point = verifying_key.to_encoded_point(false);
    let verifying_key_bytes = encoded_point.as_bytes();
    let verifying_key_base64 = general_purpose::STANDARD.encode(verifying_key_bytes);

    Ok(armor(&verifying_key_base64.as_bytes(), "SERVER MASTER VERIFYING KEY", "SERVER MASTER VERIFYING KEY"))
}

pub fn verify_signature(verifying_key_pem: &str, message: &str, signature: &str) -> Result<bool, CryptoError> {
    let verifying_key_base64 = extract_base64_from_armor(verifying_key_pem, "SERVER VERIFYING KEY")?;
    let verifying_key_bytes = general_purpose::STANDARD.decode(&verifying_key_base64)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let verifying_key = VerifyingKey::from_sec1_bytes(&verifying_key_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    let signature_bytes = extract_base64_from_armor(signature, "SIGNATURE")?;
    let signature_bytes = general_purpose::STANDARD.decode(&signature_bytes)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let signature = ecdsa::Signature::from_slice(&signature_bytes)
        .map_err(|e| CryptoError::InvalidInput(e.to_string()))?;

    Ok(verifying_key.verify(message.as_bytes(), &signature).is_ok())
}

pub fn sign_message(signing_key_pem: &str, message: &str) -> Result<String, CryptoError> {
    let signing_key_base64 = extract_base64_from_armor(signing_key_pem, "SERVER SIGNING KEY")?;
    let signing_key_bytes = general_purpose::STANDARD.decode(&signing_key_base64)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let field_bytes = FieldBytes::from_slice(&signing_key_bytes);
    let signing_key = SigningKey::from_bytes(field_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    let signature: ecdsa::Signature = signing_key.sign(message.as_bytes());
    let signature_bytes = signature.to_vec();
    let armored_signature = armor(&signature_bytes, "SIGNATURE", "SIGNATURE");

    Ok(armored_signature)
}

#[derive(Debug, PartialEq)]
pub enum CryptoError {
    IoError(String),
    Base64DecodeError(String),
    KeyCreationError(String),
    SerializationError(String),
    InvalidInput(String),
}

impl std::error::Error for CryptoError {}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CryptoError::IoError(e) => write!(f, "{}", e.red()),
            CryptoError::Base64DecodeError(e) => write!(f, "{}", e.red()),
            CryptoError::KeyCreationError(e) => write!(f, "{}", e.red()),
            CryptoError::SerializationError(e) => write!(f, "{}", e.red()),
            CryptoError::InvalidInput(e) => write!(f, "{}", e.red()),
        }
    }
}

impl From<String> for CryptoError {
    fn from(error: String) -> Self {
        CryptoError::InvalidInput(error)
    }
}

#[derive(Serialize, Deserialize)]
struct DelegateKeyCertificate {
    verifying_key: Vec<u8>,
    attributes: String,
    signature: Vec<u8>,
}

pub fn generate_master_key() -> Result<(String, String), CryptoError> {
    // Generate the master signing key
    let master_signing_key = PrivateKey::random(&mut OsRng);
    let master_verifying_key = PublicKey::from(&master_signing_key);

    // Encode the keys in base64
    let master_signing_key_base64 = general_purpose::STANDARD.encode(master_signing_key.to_bytes());
    let master_verifying_key_base64 = general_purpose::STANDARD.encode(master_verifying_key.to_encoded_point(false).as_bytes());

    // Armor the keys
    let armored_master_signing_key = format!("-----BEGIN SERVER MASTER SIGNING KEY-----\n{}\n-----END SERVER MASTER SIGNING KEY-----", master_signing_key_base64);
    let armored_master_verifying_key = format!("-----BEGIN SERVER MASTER VERIFYING KEY-----\n{}\n-----END SERVER MASTER VERIFYING KEY-----", master_verifying_key_base64);

    Ok((armored_master_signing_key, armored_master_verifying_key))
}

pub fn sign_with_key(blinded_verifying_key: &Value, server_master_signing_key: &str) -> Result<String, CryptoError> {
    let decoded_key = extract_base64_from_armor(server_master_signing_key, "SERVER MASTER SIGNING KEY")?;
    let decoded_key = general_purpose::STANDARD.decode(&decoded_key).map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let field_bytes = FieldBytes::from_slice(&decoded_key);
    let master_signing_key = PrivateKey::from_bytes(field_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    let blinded_verifying_key_bytes = match blinded_verifying_key {
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
        _ => return Err(CryptoError::InvalidInput("Invalid blinded verifying key format".to_string())),
    };

    // Generate a random nonce
    let nonce = SecretKey::random(&mut OsRng);
    let nonce_bytes = nonce.to_bytes();

    // Combine the blinded verifying key and nonce, and hash them
    let mut hasher = Sha256::new();
    hasher.update(&blinded_verifying_key_bytes);
    hasher.update(&nonce_bytes);
    let message = hasher.finalize();

    // Sign the hash
    let blind_signature: ecdsa::Signature = master_signing_key.sign(&message);

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

pub fn generate_delegate_key(master_signing_key_pem: &str, attributes: &str) -> Result<(String, String), CryptoError> {
    let master_signing_key_base64 = extract_base64_from_armor(master_signing_key_pem, "SERVER MASTER SIGNING KEY")?;
    let master_signing_key_bytes = general_purpose::STANDARD.decode(&master_signing_key_base64).map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let field_bytes = FieldBytes::from_slice(&master_signing_key_bytes);
    let master_signing_key = SigningKey::from_bytes(field_bytes)
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
    let mut buf = Vec::new();
    certificate_data.serialize(&mut Serializer::new(&mut buf))
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;
    let certificate_data_bytes = buf;

    // Sign the certificate data
    let signature: ecdsa::Signature = master_signing_key.sign(&certificate_data_bytes);
    let mut signed_certificate_data = certificate_data;
    signed_certificate_data.signature = signature.to_vec();

    // Encode the signed certificate data in base64
    let mut buf = Vec::new();
    signed_certificate_data.serialize(&mut Serializer::new(&mut buf))
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;
    let signed_certificate_base64 = general_purpose::STANDARD.encode(buf);

    // Encode the delegate signing key
    let delegate_signing_key_base64 = general_purpose::STANDARD.encode(delegate_signing_key.to_bytes());
    let armored_delegate_signing_key = armor(&delegate_signing_key_base64.as_bytes(), "DELEGATE SIGNING KEY", "DELEGATE SIGNING KEY");

    Ok((armored_delegate_signing_key, signed_certificate_base64))
}

fn extract_base64_from_armor(armored_key: &str, expected_armor_type: &str) -> Result<String, CryptoError> {
    let lines: Vec<&str> = armored_key.lines().collect();
    if lines.len() < 3 {
        return Err(CryptoError::InvalidInput(format!("Invalid armored key format. Expected at least 3 lines, found {}.", lines.len())));
    }

    let start_line = format!("-----BEGIN {}-----", expected_armor_type);
    let end_line = format!("-----END {}-----", expected_armor_type);

    if !lines[0].trim().eq(&start_line) || !lines[lines.len() - 1].trim().eq(&end_line) {
        let actual_start = lines[0].trim();
        let actual_end = lines[lines.len() - 1].trim();
        return Err(CryptoError::InvalidInput(format!(
            "Armor type mismatch. Expected: '{}' and '{}', but found '{}' and '{}'.",
            start_line, end_line, actual_start, actual_end
        )));
    }
    
    let content_lines = &lines[1..lines.len() - 1];
    Ok(content_lines.join(""))
}

pub fn validate_delegate_key(master_verifying_key_pem: &str, delegate_certificate: &str) -> Result<String, CryptoError> {
    let master_verifying_key_base64 = extract_base64_from_armor(master_verifying_key_pem, "SERVER MASTER VERIFYING KEY")?;
    let master_verifying_key_bytes = general_purpose::STANDARD.decode(&master_verifying_key_base64)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let master_verifying_key = VerifyingKey::from_sec1_bytes(&master_verifying_key_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    let certificate_bytes = general_purpose::STANDARD.decode(delegate_certificate)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    
    let mut deserializer = Deserializer::new(Cursor::new(certificate_bytes));
    let certificate: DelegateKeyCertificate = Deserialize::deserialize(&mut deserializer)
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    // Recreate the certificate data for verification
    let certificate_data = DelegateKeyCertificate {
        verifying_key: certificate.verifying_key.clone(),
        attributes: certificate.attributes.clone(),
        signature: vec![],
    };
    let mut buf = Vec::new();
    certificate_data.serialize(&mut Serializer::new(&mut buf))
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    let signature = ecdsa::Signature::from_slice(&certificate.signature)
        .map_err(|e| CryptoError::InvalidInput(e.to_string()))?;

    master_verifying_key.verify(&buf, &signature)
        .map_err(|e| CryptoError::InvalidInput(format!("Invalid signature: {}", e)))?;

    Ok(certificate.attributes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_master_key() {
        let (signing_key, verifying_key) = generate_master_key().unwrap();
        assert!(signing_key.contains("-----BEGIN SERVER MASTER SIGNING KEY-----"));
        assert!(verifying_key.contains("-----BEGIN SERVER MASTER VERIFYING KEY-----"));
    }

    #[test]
    fn test_sign_with_key() {
        let (signing_key, _) = generate_master_key().unwrap();
        let blinded_verifying_key = json!({
            "x": general_purpose::STANDARD.encode([1u8; 32]),
            "y": general_purpose::STANDARD.encode([2u8; 32])
        });
        let signature = sign_with_key(&blinded_verifying_key, &signing_key).unwrap();
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
        let (master_signing_key, _) = generate_master_key().unwrap();
        let (delegate_signing_key, delegate_certificate) = generate_delegate_key(&master_signing_key, "test_attributes").unwrap();
        assert!(delegate_signing_key.contains("-----BEGIN DELEGATE SIGNING KEY-----"));
        assert!(!delegate_certificate.is_empty());
    }

    #[test]
    fn test_extract_base64_from_armor() {
        let armored_key = "-----BEGIN TEST KEY-----\nYWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXo=\n-----END TEST KEY-----";
        let result = extract_base64_from_armor(armored_key, "TEST KEY").unwrap();
        assert_eq!(result, "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXo=");

        // Test for armor type mismatch
        let result = extract_base64_from_armor(armored_key, "WRONG KEY");
        assert!(result.is_err());
    }
}
