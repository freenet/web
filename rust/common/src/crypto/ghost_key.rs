use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use p256::ecdsa::{self, signature::{Signer, Verifier}};
use crate::armor;
use serde::{Serialize, Deserialize};
use rmp_serde::Serializer;
use crate::crypto::{CryptoError, extract_bytes_from_armor};
use rmp_serde;

#[derive(Serialize, Deserialize, Debug)]
struct DelegateCertificate {
    delegate_verifying_key: String,
    // Add other fields as needed
}

#[derive(Serialize, Deserialize, Debug)]
struct DelegateKeyCertificate {
    pub verifying_key: Vec<u8>,
    pub attributes: String,
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct GhostkeyCertificate {
    delegate_certificate: Vec<u8>,
    ghostkey_verifying_key: Vec<u8>,
    signature: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct GhostkeySigningData {
    delegate_certificate: Vec<u8>,
    ghostkey_verifying_key: Vec<u8>,
}

pub fn generate_ghostkey(delegate_certificate: &str) -> Result<String, CryptoError> {
    // Extract the delegate certificate bytes
    let delegate_certificate_bytes = extract_bytes_from_armor(delegate_certificate, "DELEGATE CERTIFICATE")?;

    // Deserialize the delegate certificate
    let delegate_cert: DelegateKeyCertificate = rmp_serde::from_slice(&delegate_certificate_bytes)
        .map_err(|e| CryptoError::DeserializationError(e.to_string()))?;

    // Extract the delegate verifying key
    let _delegate_verifying_key = VerifyingKey::from_sec1_bytes(&delegate_cert.verifying_key)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    // Generate the ghostkey key pair
    let ghostkey_signing_key = SigningKey::random(&mut OsRng);
    let ghostkey_verifying_key = VerifyingKey::from(&ghostkey_signing_key);

    // Create the signing data
    let ghostkey_signing_data = GhostkeySigningData {
        delegate_certificate: delegate_certificate_bytes.clone(),
        ghostkey_verifying_key: ghostkey_verifying_key.to_sec1_bytes().to_vec(),
    };

    // Serialize the signing data to MessagePack
    let mut buf = Vec::new();
    ghostkey_signing_data.serialize(&mut Serializer::new(&mut buf))
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    // Sign the serialized data with the ghostkey signing key
    let signature: ecdsa::Signature = ghostkey_signing_key.sign(&buf);

    // Create the final certificate with the signature
    let final_certificate = GhostkeyCertificate {
        delegate_certificate: delegate_certificate_bytes,
        ghostkey_verifying_key: ghostkey_signing_data.ghostkey_verifying_key,
        signature: signature.to_der().as_bytes().to_vec(),
    };

    // Serialize the final certificate to MessagePack
    let mut final_buf = Vec::new();
    final_certificate.serialize(&mut Serializer::new(&mut final_buf))
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    // Encode the certificate
    let ghostkey_certificate_armored = armor(&final_buf, "GHOSTKEY CERTIFICATE", "GHOSTKEY CERTIFICATE");

    Ok(ghostkey_certificate_armored)
}

fn extract_delegate_signing_key(delegate_certificate: &str) -> Result<SigningKey, CryptoError> {
    let delegate_certificate_bytes = extract_bytes_from_armor(delegate_certificate, "DELEGATE CERTIFICATE")
        .map_err(|e| CryptoError::ArmorError(format!("Failed to extract bytes from armor: {}", e)))?;

    // Deserialize as DelegateKeyCertificate
    let _delegate_cert: DelegateKeyCertificate = rmp_serde::from_slice(&delegate_certificate_bytes)
        .map_err(|e| CryptoError::DeserializationError(format!("Failed to deserialize DelegateKeyCertificate: {}", e)))?;

    // The verifying_key in the certificate is actually the public key
    // We cannot derive the signing key from it, so we need to return an error
    Err(CryptoError::KeyCreationError("Cannot extract signing key from delegate certificate. Only the public key is available.".to_string()))
}

pub fn validate_ghost_key(master_verifying_key_pem: &str, ghostkey_certificate_armored: &str) -> Result<String, CryptoError> {
    // Extract the base64 encoded ghostkey certificate
    let ghostkey_certificate_bytes = extract_bytes_from_armor(ghostkey_certificate_armored, "GHOSTKEY CERTIFICATE")?;

    // Deserialize the ghostkey certificate
    let ghostkey_certificate: GhostkeyCertificate = rmp_serde::from_slice(&ghostkey_certificate_bytes)
        .map_err(|e| CryptoError::DeserializationError(e.to_string()))?;

    // Extract the delegate certificate
    let delegate_certificate = &ghostkey_certificate.delegate_certificate;

    // Validate the delegate certificate using the master verifying key
    let delegate_attributes = validate_delegate_certificate(master_verifying_key_pem, delegate_certificate)?;

    // Verify the ghostkey signature
    verify_ghostkey_signature(&ghostkey_certificate, delegate_certificate)?;

    Ok(delegate_attributes)
}

pub fn validate_delegate_certificate(master_verifying_key_pem: &str, delegate_certificate: &[u8]) -> Result<String, CryptoError> {
    // Extract the base64 encoded master verifying key
    let master_verifying_key_bytes = extract_bytes_from_armor(master_verifying_key_pem, "MASTER VERIFYING KEY")?;
    let master_verifying_key = VerifyingKey::from_sec1_bytes(&master_verifying_key_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    // Deserialize the delegate certificate
    let delegate_cert: DelegateKeyCertificate = rmp_serde::from_slice(delegate_certificate)
        .map_err(|e| {
            println!("Deserialization error: {:?}", e);
            println!("Delegate certificate bytes: {:?}", delegate_certificate);
            CryptoError::DeserializationError(e.to_string())
        })?;

    println!("Deserialized delegate certificate: {:?}", delegate_cert);

    // Recreate the certificate data that was originally signed
    let certificate_data = DelegateKeyCertificate {
        verifying_key: delegate_cert.verifying_key.clone(),
        attributes: delegate_cert.attributes.clone(),
        signature: vec![],
    };

    // Serialize the certificate data
    let buf = rmp_serde::to_vec(&certificate_data)
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    // Verify the signature
    let signature = ecdsa::Signature::from_der(&delegate_cert.signature)
        .map_err(|e| CryptoError::SignatureError(e.to_string()))?;
    master_verifying_key.verify(&buf, &signature)
        .map_err(|e| CryptoError::SignatureVerificationError(e.to_string()))?;

    // If verification is successful, return the attributes
    Ok(delegate_cert.attributes)
}

pub fn verify_ghostkey_signature(ghostkey_certificate: &GhostkeyCertificate, delegate_certificate: &[u8]) -> Result<(), CryptoError> {
    // Extract the delegate verifying key from the delegate certificate
    let delegate_verifying_key = extract_delegate_verifying_key(delegate_certificate)?;

    // Recreate the certificate data that was originally signed
    let certificate_data = GhostkeyCertificate {
        delegate_certificate: ghostkey_certificate.delegate_certificate.clone(),
        ghostkey_verifying_key: ghostkey_certificate.ghostkey_verifying_key.clone(),
        signature: Vec::new(),
    };

    // Serialize the certificate data
    let mut buf = Vec::new();
    certificate_data.serialize(&mut Serializer::new(&mut buf))
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    // Create the signature from the stored bytes
    let signature = ecdsa::Signature::from_der(&ghostkey_certificate.signature)
        .map_err(|e| CryptoError::SignatureError(e.to_string()))?;

    // Verify the signature
    delegate_verifying_key.verify(&buf, &signature)
        .map_err(|e| CryptoError::SignatureVerificationError(e.to_string()))?;

    Ok(())
}

pub fn extract_delegate_verifying_key(delegate_certificate: &[u8]) -> Result<VerifyingKey, CryptoError> {
    let delegate_cert: DelegateKeyCertificate = rmp_serde::from_slice(delegate_certificate)
        .map_err(|e| CryptoError::DeserializationError(e.to_string()))?;

    VerifyingKey::from_sec1_bytes(&delegate_cert.verifying_key)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))
}
/// Validates an armored ghost key certificate using the provided master verifying key.
///
/// # Arguments
///
/// * `master_verifying_key_pem` - The master verifying key in PEM format
/// * `ghostkey_certificate_armored` - The ghost key certificate in armored format
///
/// # Returns
///
/// The delegate attributes as a string if validation is successful, or a CryptoError if validation fails.
pub fn validate_armored_ghost_key_command(master_verifying_key_pem: &str, ghostkey_certificate_armored: &str) -> Result<String, CryptoError> {
    validate_ghost_key(master_verifying_key_pem, ghostkey_certificate_armored)
}
