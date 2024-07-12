use p256::ecdsa::{SigningKey, VerifyingKey};
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
use crate::crypto::{CryptoError, extract_base64_from_armor};

#[derive(Serialize, Deserialize)]
pub struct DelegateKeyCertificate {
    pub verifying_key: Vec<u8>,
    pub attributes: String,
    pub signature: Vec<u8>,
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
