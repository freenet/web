use super::*;
use crate::crypto::armorable_wrappers::ArmorableKey;

pub fn verify_signature(verifying_key_pem: &str, message: &str, signature: &str) -> Result<bool, CryptoError> {
    let verifying_key = VerifyingKey::from_armored(verifying_key_pem)?;
    let signature = ecdsa::Signature::from_armored(signature)?;

    Ok(verifying_key.verify(message.as_bytes(), &signature).is_ok())
}

pub fn sign_message(signing_key_pem: &str, message: &str) -> Result<String, CryptoError> {
    let signing_key = SigningKey::from_armored(signing_key_pem)?;

    let signature: ecdsa::Signature = signing_key.sign(message.as_bytes());
    signature.to_armored()
}
