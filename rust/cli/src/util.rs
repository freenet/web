use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use crate::errors::GhostkeyError;

pub fn create_keypair() -> Result<(SigningKey, VerifyingKey), GhostkeyError> {
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);
    Ok((signing_key, verifying_key))
}
