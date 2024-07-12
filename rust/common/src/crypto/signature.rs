use super::*;

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
