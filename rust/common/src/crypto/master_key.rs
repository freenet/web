use super::*;

pub fn generate_master_verifying_key(master_signing_key_pem: &str) -> Result<String, CryptoError> {
    let signing_key_bytes = extract_bytes_from_armor(master_signing_key_pem, "MASTER SIGNING KEY")?;

    let signing_key_bytes = general_purpose::STANDARD.decode(&signing_key_bytes)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let field_bytes = FieldBytes::from_slice(&signing_key_bytes);
    let signing_key = SigningKey::from_bytes(field_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    let verifying_key = VerifyingKey::from(&signing_key);
    let encoded_point = verifying_key.to_encoded_point(false);
    let verifying_key_bytes = encoded_point.as_bytes();
    let verifying_key_base64 = general_purpose::STANDARD.encode(verifying_key_bytes);

    Ok(armor(&verifying_key_base64.as_bytes(), "MASTER VERIFYING KEY", "MASTER VERIFYING KEY"))
}

pub fn generate_master_key() -> Result<(String, String), CryptoError> {
    // Generate the master signing key
    let master_signing_key = SigningKey::random(&mut OsRng);
    let master_verifying_key = VerifyingKey::from(&master_signing_key);

    // Encode the keys in base64
    let master_signing_key_base64 = general_purpose::STANDARD.encode(master_signing_key.to_bytes());
    let master_verifying_key_base64 = general_purpose::STANDARD.encode(master_verifying_key.to_encoded_point(false).as_bytes());

    // Armor the keys
    let armored_master_signing_key = format!("-----BEGIN MASTER SIGNING KEY-----\n{}\n-----END MASTER SIGNING KEY-----", master_signing_key_base64);
    let armored_master_verifying_key = format!("-----BEGIN MASTER VERIFYING KEY-----\n{}\n-----END MASTER VERIFYING KEY-----", master_verifying_key_base64);

    Ok((armored_master_signing_key, armored_master_verifying_key))
}
