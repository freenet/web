pub mod blind;

use ed25519_dalek::*;
use rand_core::OsRng;

use serde::{Serialize, Deserialize};
use crate::errors::GhostkeyError;
use crate::armorable::*;

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
pub fn sign_with_hash<T: Serialize + for<'de> Deserialize<'de>>(signing_key: &SigningKey, data: &T) -> Result<Signature, Box<GhostkeyError>> {
    let bytes = data.to_bytes().map_err(|e| GhostkeyError::SerializationError(e.to_string()))?;
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
pub fn verify_with_hash<T: Serialize + for<'de> Deserialize<'de>>(verifying_key: &VerifyingKey, data: &T, signature: &Signature) -> Result<bool, Box<GhostkeyError>> {
    let bytes = data.to_bytes().map_err(|e| GhostkeyError::SerializationError(e.to_string()))?;
    Ok(verifying_key.verify(bytes.as_slice(), signature).is_ok())
}

#[cfg(test)]
mod tests {
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
}
