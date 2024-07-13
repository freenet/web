pub mod generate_delegate;
pub mod ghost_key;
pub mod signature;
pub mod crypto_error;
pub mod master_key;
pub mod sign_with_key;

use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use base64::{engine::general_purpose, Engine as _};
use serde_json::Value;
use sha2::{Sha256, Digest};
use p256::{SecretKey, FieldBytes};
use p256::ecdsa::{self, signature::{Signer, Verifier}};
use crate::armor;
use serde::Serialize;
use rmp_serde::Serializer;
use crate::crypto::crypto_error::CryptoError;
use crate::crypto::generate_delegate::DelegateKeyCertificate;

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

fn extract_base64_from_armor(armored_key: &str, expected_armor_type: &str) -> Result<Vec<u8>, CryptoError> {
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
    let content = content_lines.join("");
    general_purpose::STANDARD.decode(content)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))
}

pub fn validate_delegate_key(master_verifying_key_pem: &str, delegate_certificate: &str) -> Result<String, CryptoError> {
    println!("Master verifying key PEM: {}", master_verifying_key_pem);
    println!("Delegate certificate: {}", delegate_certificate);

    let master_verifying_key_base64 = extract_base64_from_armor(master_verifying_key_pem, "MASTER VERIFYING KEY")?;
    println!("Extracted master verifying key base64: {}", master_verifying_key_base64);

    let master_verifying_key_bytes = general_purpose::STANDARD.decode(&master_verifying_key_base64)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let master_verifying_key = VerifyingKey::from_sec1_bytes(&master_verifying_key_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    println!("Extracting delegate certificate base64");
    let delegate_certificate_base64 = extract_base64_from_armor(delegate_certificate, "DELEGATE CERTIFICATE")?;
    println!("Extracted delegate certificate base64: {}", delegate_certificate_base64);

    println!("Attempting to decode delegate certificate");
    let certificate_bytes = general_purpose::STANDARD.decode(&delegate_certificate_base64)
        .map_err(|e| {
            println!("Failed to decode delegate certificate: {}", e);
            CryptoError::Base64DecodeError(e.to_string())
        })?;
    
    println!("Decoded certificate bytes: {:?}", certificate_bytes);

    let certificate: DelegateKeyCertificate = match rmp_serde::from_slice(&certificate_bytes) {
        Ok(cert) => {
            println!("Successfully deserialized certificate: {:?}", cert);
            cert
        },
        Err(e) => {
            println!("Failed to deserialize certificate: {}", e);
            println!("Certificate bytes: {:?}", certificate_bytes);
            // Try to deserialize as a string
            match String::from_utf8(certificate_bytes.clone()) {
                Ok(s) => println!("Certificate as string: {}", s),
                Err(_) => println!("Certificate is not valid UTF-8"),
            }
            return Err(CryptoError::DeserializationError(e.to_string()));
        }
    };

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
    use crate::crypto::generate_delegate::generate_delegate_key;
    use crate::crypto::master_key::generate_master_key;
    use crate::crypto::sign_with_key::sign_with_key;

    #[test]
    fn test_generate_master_key() {
        let (signing_key, verifying_key) = generate_master_key().unwrap();
        assert!(signing_key.contains("-----BEGIN MASTER SIGNING KEY-----"));
        assert!(verifying_key.contains("-----BEGIN MASTER VERIFYING KEY-----"));
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
        let delegate_certificate = generate_delegate_key(&master_signing_key, "test_attributes").unwrap();
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
