use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use crate::crypto_error::CryptoError;

pub fn create_master_keypair() -> Result<(SigningKey, VerifyingKey), CryptoError> {
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);
    Ok((signing_key, verifying_key))
}