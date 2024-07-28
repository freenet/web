pub mod blind;


use ed25519_dalek::*;
use rand_core::OsRng;

use crate::armorable::*;
use crate::errors::GhostkeyError;
use blind_rsa_signatures::{KeyPair as RSAKeyPair, Options, Signature as RSASignature};
use serde::{Deserialize, Serialize};

/// Creates a new ECDSA keypair for signing and verification.
///
/// # Returns
///
/// A tuple containing a `SigningKey` and its corresponding `VerifyingKey`,
/// or a `GhostkeyError` if key creation fails.
pub fn create_keypair() -> Result<(SigningKey, VerifyingKey), GhostkeyError> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);
    Ok((signing_key, verifying_key))
}

/// Signs the given data using the provided signing key.
///
/// # Arguments
///
/// * `signing_key` - The `SigningKey` to use for signing.
/// * `data` - The data to be signed, which must implement the `Serialize` trait.
///
/// # Returns
///
/// A `Signature` if signing is successful, or a `GhostkeyError` if it fails.
///
/// # Note
///
/// This function uses blake3 to hash the data before signing.
pub fn sign_with_hash<T: Serialize + for<'de> Deserialize<'de>>(
    signing_key: &SigningKey,
    data: &T,
) -> Result<Signature, Box<GhostkeyError>> {
    let bytes = data
        .to_bytes()
        .map_err(|e| GhostkeyError::SerializationError(e.to_string()))?;
    Ok(signing_key.sign(bytes.as_slice()))
}

/// Verifies a signature for the given data using the provided verifying key.
///
/// # Arguments
///
/// * `verifying_key` - The `VerifyingKey` to use for verification.
/// * `data` - The data to be verified, which must implement the `Serialize` trait.
/// * `signature` - The `Signature` to verify.
///
/// # Returns
///
/// A boolean indicating whether the signature is valid (`true`) or not (`false`),
/// or a `GhostkeyError` if verification fails.
///
/// # Note
///
/// This function uses blake3 to hash the data before verification.
pub fn verify_with_hash<T: Serialize + for<'de> Deserialize<'de>>(
    verifying_key: &VerifyingKey,
    data: &T,
    signature: &Signature,
) -> Result<bool, Box<GhostkeyError>> {
    let bytes = data
        .to_bytes()
        .map_err(|e| GhostkeyError::SerializationError(e.to_string()))?;
    Ok(verifying_key.verify(bytes.as_slice(), signature).is_ok())
}

/// Signs the given data using the provided RSA signing key, uses blind signature internally
/// to guarantee compatibility with actual blind signatures, even if it's less efficient.
pub fn unblinded_rsa_sign(
    signing_keypair: &RSAKeyPair,
    msg: &[u8],
) -> Result<RSASignature, Box<GhostkeyError>> {
    let options = Options::default();

    let blinding_result = signing_keypair
        .pk
        .blind(&mut OsRng, msg, false, &options)
        .map_err(|e| GhostkeyError::RSAError(e.to_string()))?;

    let blind_sig = signing_keypair
        .sk
        .blind_sign(&mut OsRng, &blinding_result.blind_msg, &options)
        .map_err(|e| GhostkeyError::RSAError(e.to_string()))?;

    let sig = signing_keypair
        .pk
        .finalize(
            &blind_sig,
            &blinding_result.secret,
            blinding_result.msg_randomizer,
            msg,
            &options,
        )
        .map_err(|e| GhostkeyError::RSAError(e.to_string()))?;

    Ok(sig)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;
    use log::info;
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Serialize, Deserialize)]
    struct TestData {
        field1: String,
        field2: i32,
    }

    #[test]
    fn test_create_keypair() {
        let result = create_keypair();
        assert!(result.is_ok());
        let (signing_key, verifying_key) = result.unwrap();
        assert_eq!(VerifyingKey::from(&signing_key), verifying_key);
    }

    #[test]
    fn test_sign_and_verify() {
        let (signing_key, verifying_key) = create_keypair().unwrap();
        let test_data = TestData {
            field1: "Hello".to_string(),
            field2: 42,
        };

        // Sign the data
        let signature = sign_with_hash(&signing_key, &test_data).unwrap();

        // Verify the signature
        let is_valid = verify_with_hash(&verifying_key, &test_data, &signature).unwrap();
        assert!(is_valid);

        // Modify the data and verify again (should fail)
        let modified_data = TestData {
            field1: "Hello".to_string(),
            field2: 43,
        };
        let is_valid = verify_with_hash(&verifying_key, &modified_data, &signature).unwrap();
        assert!(!is_valid);
    }

    #[test]
    fn test_rsa_sign_and_verify() {
        let _ = env_logger::try_init();
        
        info!("Generating RSA keypair...");
        let start = Instant::now();
        let keypair = RSAKeyPair::generate(&mut OsRng, 2048).unwrap();
        let msg = b"test";
        info!("RSA keypair generated in {}ms", start.elapsed().as_millis());

        info!("Signing message...");

        let start = Instant::now();
        let signature = unblinded_rsa_sign(&keypair, msg).unwrap();
        info!("Message signed in {}ms", start.elapsed().as_millis());

        info!("Verifying signature...");
        let start = Instant::now();
        let is_valid = keypair
            .pk
            .verify(&signature, None, msg, &Default::default());

        info!("Signature verified in {}ms", start.elapsed().as_millis());
        assert!(is_valid.is_ok());
    }
}
