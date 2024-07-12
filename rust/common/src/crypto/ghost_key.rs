use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use base64::{engine::general_purpose, Engine as _};
use p256::{FieldBytes};
use p256::ecdsa::{self, signature::{Signer, Verifier}};
use crate::armor;
use serde::{Serialize, Deserialize};
use rmp_serde::{Serializer};
use crate::crypto::{CryptoError, extract_base64_from_armor};

#[derive(Serialize, Deserialize)]
struct DelegateCertificate {
    delegate_verifying_key: String,
    // Add other fields as needed
}

#[derive(Serialize, Deserialize)]
pub struct GhostkeyCertificate {
    delegate_certificate: String,
    ghostkey_verifying_key: String,
    signature: String,
}

pub fn generate_ghostkey(delegate_signing_key_pem: &str) -> Result<(String, String), CryptoError> {
    let delegate_signing_key_base64 = extract_base64_from_armor(delegate_signing_key_pem, "DELEGATE SIGNING KEY")?;
    let delegate_signing_key_bytes = general_purpose::STANDARD.decode(&delegate_signing_key_base64)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let field_bytes = FieldBytes::from_slice(&delegate_signing_key_bytes);
    let delegate_signing_key = SigningKey::from_bytes(field_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    // Generate the ghostkey signing key
    let ghostkey_signing_key = SigningKey::random(&mut OsRng);
    let ghostkey_verifying_key = VerifyingKey::from(&ghostkey_signing_key);

    // Create the certificate
    let ghostkey_certificate = GhostkeyCertificate {
        delegate_certificate: delegate_signing_key_pem.to_string(),
        ghostkey_verifying_key: general_purpose::STANDARD.encode(ghostkey_verifying_key.to_sec1_bytes()),
        signature: String::new(), // We'll fill this in shortly
    };

    // Serialize the certificate (without the signature) to MessagePack
    let mut buf = Vec::new();
    ghostkey_certificate.serialize(&mut Serializer::new(&mut buf))
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    // Sign the serialized certificate

    let signature: ecdsa::Signature = delegate_signing_key.sign(&buf);

    // Create the final certificate with the signature
    let final_certificate = GhostkeyCertificate {
        delegate_certificate: ghostkey_certificate.delegate_certificate,
        ghostkey_verifying_key: general_purpose::STANDARD.encode(ghostkey_verifying_key.to_sec1_bytes()),
        signature: general_purpose::STANDARD.encode(signature.to_der()),
    };

    // Serialize the final certificate to MessagePack
    let mut final_buf = Vec::new();
    final_certificate.serialize(&mut Serializer::new(&mut final_buf))
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    // Encode the keys and certificate
    let ghostkey_signing_key_pem = armor(&ghostkey_signing_key.to_bytes(), "GHOSTKEY SIGNING KEY", "GHOSTKEY SIGNING KEY");
    let ghostkey_certificate_base64 = general_purpose::STANDARD.encode(final_buf);

    Ok((ghostkey_signing_key_pem, ghostkey_certificate_base64))
}

pub fn validate_ghost_key(master_verifying_key_pem: &str, ghostkey_certificate_base64: &str) -> Result<String, CryptoError> {
    // Decode the base64 ghostkey certificate
    let ghostkey_certificate_bytes = general_purpose::STANDARD.decode(ghostkey_certificate_base64)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;

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

pub fn validate_delegate_certificate(_master_verifying_key_pem: &str, _delegate_certificate: &str) -> Result<String, CryptoError> {
    // TODO: Implement the validation of the delegate certificate using the master verifying key
    // This function should return the attributes of the delegate key if validation is successful
    Err(CryptoError::NotImplemented("Delegate certificate validation not implemented".to_string()))
}

pub fn verify_ghostkey_signature(ghostkey_certificate: &GhostkeyCertificate, delegate_certificate: &str) -> Result<(), CryptoError> {
    // Extract the delegate verifying key from the delegate certificate
    let delegate_verifying_key = extract_delegate_verifying_key(delegate_certificate)?;

    // Recreate the certificate data that was originally signed
    let certificate_data = GhostkeyCertificate {
        delegate_certificate: ghostkey_certificate.delegate_certificate.clone(),
        ghostkey_verifying_key: ghostkey_certificate.ghostkey_verifying_key.clone(),
        signature: String::new(),
    };

    // Serialize the certificate data
    let mut buf = Vec::new();
    certificate_data.serialize(&mut Serializer::new(&mut buf))
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    // Decode the signature
    let signature_bytes = general_purpose::STANDARD.decode(&ghostkey_certificate.signature)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let signature = ecdsa::Signature::from_der(&signature_bytes)
        .map_err(|e| CryptoError::SignatureError(e.to_string()))?;

    // Verify the signature
    delegate_verifying_key.verify(&buf, &signature)
        .map_err(|e| CryptoError::SignatureVerificationError(e.to_string()))?;

    Ok(())
}

pub fn extract_delegate_verifying_key(delegate_certificate: &str) -> Result<VerifyingKey, CryptoError> {
    let delegate_certificate_base64 = extract_base64_from_armor(delegate_certificate, "DELEGATE CERTIFICATE")?;
    let delegate_certificate_bytes = general_purpose::STANDARD.decode(&delegate_certificate_base64)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;

    let delegate_certificate: DelegateCertificate = rmp_serde::from_slice(&delegate_certificate_bytes)
        .map_err(|e| CryptoError::DeserializationError(e.to_string()))?;

    let verifying_key_bytes = general_purpose::STANDARD.decode(&delegate_certificate.delegate_verifying_key)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;

    VerifyingKey::from_sec1_bytes(&verifying_key_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))
}
pub fn validate_ghost_key_command(master_verifying_key_pem: &str, ghostkey_certificate_base64: &str) -> Result<String, CryptoError> {
    validate_ghost_key(master_verifying_key_pem, ghostkey_certificate_base64)
}
