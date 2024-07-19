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
use log::{warn, debug};

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

pub fn extract_bytes_from_armor(armored_text: &str, expected_armor_type: &str) -> Result<Vec<u8>, CryptoError> {
    let start_line = format!("-----BEGIN {}-----", expected_armor_type);
    let end_line = format!("-----END {}-----", expected_armor_type);

    let start_index = armored_text.find(&start_line)
        .ok_or_else(|| CryptoError::InvalidInput(format!("Missing start of {}", expected_armor_type)))?;
    let end_index = armored_text[start_index..].find(&end_line)
        .ok_or_else(|| CryptoError::InvalidInput(format!("Missing end of {}", expected_armor_type)))?
        + start_index + end_line.len();

    let armored_section = &armored_text[start_index..end_index];
    let lines: Vec<&str> = armored_section.lines().collect();

    if lines.len() < 3 {
        return Err(CryptoError::InvalidInput("Invalid armored text".to_string()));
    }

    let base64_data = lines[1..lines.len() - 1].join("");
    general_purpose::STANDARD.decode(base64_data)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))
}

pub fn validate_delegate_key(master_verifying_key_pem: &str, delegate_certificate: &str) -> Result<String, CryptoError> {
    debug!("Master verifying key PEM: {}", master_verifying_key_pem);
    debug!("Delegate certificate: {}", delegate_certificate);

    let master_verifying_key_bytes = extract_bytes_from_armor(master_verifying_key_pem, "MASTER VERIFYING KEY")?;
    debug!("Extracted master verifying key bytes: {:?}", master_verifying_key_bytes);

    let master_verifying_key = VerifyingKey::from_sec1_bytes(&master_verifying_key_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    debug!("Extracting delegate certificate bytes");
    let certificate_bytes = extract_bytes_from_armor(delegate_certificate, "DELEGATE CERTIFICATE")?;
    debug!("Extracted delegate certificate bytes: {:?}", certificate_bytes);

    let certificate: DelegateKeyCertificate = match rmp_serde::from_slice(&certificate_bytes) {
        Ok(cert) => {
            debug!("Successfully deserialized certificate: {:?}", cert);
            cert
        },
        Err(e) => {
            warn!("Failed to deserialize certificate: {}", e);
            debug!("Certificate bytes: {:?}", certificate_bytes);
            // Try to deserialize as a string
            match String::from_utf8(certificate_bytes.clone()) {
                Ok(s) => debug!("Certificate as string: {}", s),
                Err(_) => warn!("Certificate is not valid UTF-8"),
            }
            return Err(CryptoError::DeserializationError(e.to_string()));
        }
    };

    // Recreate the certificate data for verification
    let certificate_data = DelegateKeyCertificate {
        verifying_key: certificate.verifying_key.clone(),
        info: certificate.info.clone(),
        signature: vec![],
    };
    let mut buf = Vec::new();
    certificate_data.serialize(&mut Serializer::new(&mut buf))
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    let signature = ecdsa::Signature::from_slice(&certificate.signature)
        .map_err(|e| CryptoError::InvalidInput(e.to_string()))?;

    master_verifying_key.verify(&buf, &signature)
        .map_err(|e| CryptoError::InvalidInput(format!("Invalid signature: {}", e)))?;

    Ok(certificate.info)
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
        let _ = env_logger::builder().is_test(true).try_init();

        let (signing_key, _) = generate_master_key().unwrap();
        println!("Generated signing key: {}", signing_key);
        
        let random_x = rand::random::<[u8; 32]>();
        let random_y = rand::random::<[u8; 32]>();
        let blinded_verifying_key = json!({
            "x": general_purpose::STANDARD.encode(random_x),
            "y": general_purpose::STANDARD.encode(random_y)
        });
        println!("Blinded verifying key: {}", blinded_verifying_key);
        
        let signature = sign_with_key(&blinded_verifying_key, &signing_key);
        match signature {
            Ok(sig) => {
                println!("Signature generated successfully: {}", sig);
                assert!(!sig.is_empty());
            },
            Err(e) => {
                println!("Error generating signature: {:?}", e);
                panic!("Failed to generate signature: {:?}", e);
            }
        }
    }

    #[test]
    fn test_sign_with_key_armored() {
        let _ = env_logger::builder().is_test(true).try_init();

        let (armored_signing_key, _) = generate_master_key().unwrap();
        println!("Generated armored signing key: {}", armored_signing_key);
        
        let random_x = rand::random::<[u8; 32]>();
        let random_y = rand::random::<[u8; 32]>();
        let blinded_verifying_key = json!({
            "x": general_purpose::STANDARD.encode(random_x),
            "y": general_purpose::STANDARD.encode(random_y)
        });
        println!("Blinded verifying key: {}", blinded_verifying_key);
        
        let signature = sign_with_key(&blinded_verifying_key, &armored_signing_key);
        match signature {
            Ok(sig) => {
                println!("Signature generated successfully: {}", sig);
                assert!(!sig.is_empty());
            },
            Err(e) => {
                println!("Error generating signature: {:?}", e);
                panic!("Failed to generate signature: {:?}", e);
            }
        }
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
        let (delegate_certificate, _) = generate_delegate_key(&master_signing_key, "test_info").unwrap();
        assert!(!delegate_certificate.is_empty());
    }

    #[test]
    fn test_extract_bytes_from_armor() {
        let armored_key = "-----BEGIN TEST KEY-----\nYWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXo=\n-----END TEST KEY-----";
        let result = extract_bytes_from_armor(armored_key, "TEST KEY").unwrap();
        assert!(!result.is_empty());

        // Test for armor type mismatch
        let result = extract_bytes_from_armor(armored_key, "WRONG KEY");
        assert!(result.is_err());
    }
}
