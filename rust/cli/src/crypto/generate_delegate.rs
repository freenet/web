use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use p256::ecdsa::{self, signature::Signer};
use crate::armor;
use serde::Serialize;
use ciborium::ser::into_writer;
use crate::crypto::{CryptoError, extract_bytes_from_armor};
use log::debug;
use crate::crypto::ghost_key::DelegateKeyCertificate;

pub fn generate_delegate_key(master_signing_key_pem: &str, info: &str) -> Result<(String, String), CryptoError> {
    debug!("Generating delegate key with info: {}", info);
    debug!("Master signing key PEM: {}", master_signing_key_pem);

    let master_signing_key_bytes = extract_bytes_from_armor(master_signing_key_pem, "MASTER SIGNING KEY")?;
    debug!("Extracted bytes: {:?}", master_signing_key_bytes);

    let master_signing_key = SigningKey::from_slice(&master_signing_key_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;
    debug!("Created SigningKey successfully");

    // Generate the delegate key pair
    let delegate_signing_key = SigningKey::random(&mut OsRng);
    let delegate_verifying_key = VerifyingKey::from(&delegate_signing_key);

    // Serialize the verifying key and info
    let verifying_key_bytes = delegate_verifying_key.to_encoded_point(false).as_bytes().to_vec();
    let certificate_data = DelegateKeyCertificate {
        verifying_key: verifying_key_bytes.clone(),
        info: info.to_string(),
        signature: vec![],
    };
    let mut certificate_data_bytes = Vec::new();
    into_writer(&certificate_data, &mut certificate_data_bytes)
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    // Sign the certificate data
    let signature: ecdsa::Signature = master_signing_key.sign(&certificate_data_bytes);
    let signed_certificate_data = DelegateKeyCertificate {
        verifying_key: verifying_key_bytes,
        info: info.to_string(),
        signature: signature.to_vec(),
    };

    // Serialize the signed certificate data using bincode
    let mut signed_certificate_bytes = Vec::new();
    into_writer(&signed_certificate_data, &mut signed_certificate_bytes)
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    debug!("Serialized certificate: {:?}", signed_certificate_bytes);

    // Armor the signed certificate and delegate signing key
    let armored_delegate_certificate = armor(&signed_certificate_bytes, "DELEGATE CERTIFICATE", "DELEGATE CERTIFICATE");
    let armored_delegate_signing_key = armor(&delegate_signing_key.to_bytes(), "DELEGATE SIGNING KEY", "DELEGATE SIGNING KEY");

    Ok((armored_delegate_certificate, armored_delegate_signing_key))
}
