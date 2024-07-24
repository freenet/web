use p256::ecdsa::{SigningKey, VerifyingKey, Signature, signature::Signer, signature::Verifier};
use rand_core::OsRng;
use crate::errors::GhostkeyError;
use crate::armorable::Armorable;
use blake3::Hash;

pub fn create_keypair() -> Result<(SigningKey, VerifyingKey), GhostkeyError> {
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);
    Ok((signing_key, verifying_key))
}

pub fn sign<T: Armorable>(signing_key: &SigningKey, data: &T) -> Result<Signature, GhostkeyError> {
    let bytes = data.to_bytes().map_err(|e| GhostkeyError::SerializationError(e.to_string()))?;
    let hash = blake3::hash(&bytes);
    Ok(signing_key.sign(hash.as_bytes()))
}

pub fn verify<T: Armorable>(verifying_key: &VerifyingKey, data: &T, signature: &Signature) -> Result<bool, GhostkeyError> {
    let bytes = data.to_bytes().map_err(|e| GhostkeyError::SerializationError(e.to_string()))?;
    let hash = blake3::hash(&bytes);
    Ok(verifying_key.verify(hash.as_bytes(), signature).is_ok())
}
