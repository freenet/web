use super::*;
use crate::crypto::armorable_wrappers::ArmorableKey;

pub fn generate_master_verifying_key(master_signing_key_pem: &str) -> Result<String, CryptoError> {
    let signing_key = SigningKey::from_armored(master_signing_key_pem)?;
    let verifying_key = VerifyingKey::from(&signing_key);
    verifying_key.to_armored()
}

pub fn generate_master_key() -> Result<(String, String), CryptoError> {
    // Generate the master signing key
    let master_signing_key = SigningKey::random(&mut OsRng);
    let master_verifying_key = VerifyingKey::from(&master_signing_key);

    // Armor the keys
    let armored_master_signing_key = master_signing_key.to_armored()?;
    let armored_master_verifying_key = master_verifying_key.to_armored()?;

    Ok((armored_master_signing_key, armored_master_verifying_key))
}
