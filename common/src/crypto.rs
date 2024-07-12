use rmp_serde::{Serializer, Deserializer};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct GhostkeyCertificate {
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
        ghostkey_verifying_key: general_purpose::STANDARD.encode(ghostkey_verifying_key.to_bytes()),
        signature: String::new(), // We'll fill this in shortly
    };

    // Serialize the certificate (without the signature) to MessagePack
    let mut buf = Vec::new();
    ghostkey_certificate.serialize(&mut Serializer::new(&mut buf))
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    // Sign the serialized certificate
    let signature = delegate_signing_key.sign(&buf);

    // Create the final certificate with the signature
    let final_certificate = GhostkeyCertificate {
        delegate_certificate: ghostkey_certificate.delegate_certificate,
        ghostkey_verifying_key: ghostkey_certificate.ghostkey_verifying_key,
        signature: general_purpose::STANDARD.encode(signature.to_bytes()),
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
