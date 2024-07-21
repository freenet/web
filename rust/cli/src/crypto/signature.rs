use super::*;

pub fn verify_signature(verifying_key_pem: &str, message: &str, signature: &str) -> Result<bool, CryptoError> {
    let verifying_key = VerifyingKey::from_base64_armored(verifying_key_pem)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    let signature = ecdsa::Signature::from_base64_armored(signature)
        .map_err(|e| CryptoError::InvalidInput(e.to_string()))?;

    Ok(verifying_key.verify(message.as_bytes(), &signature).is_ok())
}

pub fn sign_message(signing_key_pem: &str, message: &str) -> Result<String, CryptoError> {
    let signing_key = SigningKey::from_base64_armored(signing_key_pem)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    let signature: ecdsa::Signature = signing_key.sign(message.as_bytes());
    let armored_signature = signature.to_base64_armored()
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    Ok(armored_signature)
}
