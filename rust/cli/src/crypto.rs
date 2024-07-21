pub mod generate_delegate;
pub mod ghost_key;
pub mod signature;
pub mod crypto_error;
pub mod master_key;
pub mod sign_with_key;

use crate::armorable::Armorable;
use p256::ecdsa::{SigningKey, VerifyingKey};
use serde::{Serialize, Deserialize};

impl Armorable for SigningKey {
    fn armor_label() -> &'static str {
        "SERVER SIGNING KEY"
    }
}

impl Armorable for VerifyingKey {
    fn armor_label() -> &'static str {
        "SERVER VERIFYING KEY"
    }
}

#[derive(Serialize, Deserialize)]
struct ArmorableVerifyingKey(#[serde(with = "serde_bytes")] Vec<u8>);

impl From<&VerifyingKey> for ArmorableVerifyingKey {
    fn from(vk: &VerifyingKey) -> Self {
        ArmorableVerifyingKey(vk.to_sec1_bytes().to_vec())
    }
}

impl TryFrom<ArmorableVerifyingKey> for VerifyingKey {
    type Error = p256::Error;

    fn try_from(avk: ArmorableVerifyingKey) -> Result<Self, Self::Error> {
        VerifyingKey::from_sec1_bytes(&avk.0)
    }
}

#[derive(Serialize, Deserialize)]
struct ArmorableSigningKey(#[serde(with = "serde_bytes")] Vec<u8>);

impl From<&SigningKey> for ArmorableSigningKey {
    fn from(sk: &SigningKey) -> Self {
        ArmorableSigningKey(sk.to_bytes().to_vec())
    }
}

impl TryFrom<ArmorableSigningKey> for SigningKey {
    type Error = p256::Error;

    fn try_from(ask: ArmorableSigningKey) -> Result<Self, Self::Error> {
        SigningKey::from_bytes(&ask.0)
    }
}

use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use serde_json::Value;
use sha2::{Sha256, Digest};
use p256::{SecretKey, FieldBytes};
use p256::ecdsa::{self, signature::{Signer, Verifier}};
use crate::armorable::Armorable;
use ciborium::ser::into_writer;
use crate::crypto::crypto_error::CryptoError;
use crate::crypto::ghost_key::DelegateKeyCertificate;
use log::debug;

pub fn generate_signing_key() -> Result<(String, String), CryptoError> {
    // Generate the signing key
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);

    // Armor the keys
    let armored_signing_key = signing_key.to_base64_armored()
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;
    let armored_verifying_key = verifying_key.to_base64_armored()
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    Ok((armored_signing_key, armored_verifying_key))
}

pub fn validate_delegate_key(master_verifying_key_pem: &str, delegate_certificate: &str) -> Result<String, CryptoError> {
    debug!("Master verifying key PEM: {}", master_verifying_key_pem);
    debug!("Delegate certificate: {}", delegate_certificate);

    let master_verifying_key = VerifyingKey::from_base64_armored(master_verifying_key_pem)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    debug!("Extracting delegate certificate");
    let certificate: DelegateKeyCertificate = DelegateKeyCertificate::from_base64_armored(delegate_certificate)
        .map_err(|e| CryptoError::DeserializationError(e.to_string()))?;

    debug!("Successfully deserialized certificate: {:?}", certificate);

    // Recreate the certificate data for verification
    let certificate_data = DelegateKeyCertificate {
        verifying_key: certificate.verifying_key.clone(),
        info: certificate.info.clone(),
        signature: vec![],
    };
    let mut buf = Vec::new();
    into_writer(&certificate_data, &mut buf)
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
