use ed25519_dalek::*;
use rand_core::{CryptoRng, RngCore};

use crate::armorable::*;
use crate::blind_rsa::{DefaultRng, SecretKey as RSASigningKey, Signature as RSASignature};
use crate::errors::GhostkeyError;
use serde::{Deserialize, Serialize};

/// Creates a new ECDSA keypair for signing and verification.
///
/// # Returns
///
/// A tuple containing a `SigningKey` and its corresponding `VerifyingKey`,
/// or a `GhostkeyError` if key creation fails.
pub fn create_keypair<R>(rng: &mut R) -> Result<(SigningKey, VerifyingKey), GhostkeyError>
where
    R: RngCore + CryptoRng,
{
    let signing_key = SigningKey::generate(rng);
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
pub fn sign_with_hash<T: Serialize + for<'de> Deserialize<'de> + 'static>(
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
pub fn verify_with_hash<T: Serialize + for<'de> Deserialize<'de> + 'static>(
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
    signing_key: &RSASigningKey,
    msg: &[u8],
) -> Result<RSASignature, Box<GhostkeyError>> {
    let public_key = signing_key
        .public_key()
        .map_err(|e| GhostkeyError::RSAError(e.to_string()))?;

    let blinding_result = public_key
        .blind(&mut DefaultRng, msg)
        .map_err(|e| GhostkeyError::RSAError(e.to_string()))?;

    let blind_sig = signing_key
        .blind_sign(&blinding_result.blind_message)
        .map_err(|e| GhostkeyError::RSAError(e.to_string()))?;

    let sig = public_key
        .finalize(&blind_sig, &blinding_result, msg)
        .map_err(|e| GhostkeyError::RSAError(e.to_string()))?;

    Ok(sig)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;
    use rand_core::OsRng;

    #[derive(Serialize, Deserialize)]
    struct TestData {
        field1: String,
        field2: i32,
    }

    #[test]
    fn test_create_keypair() {
        let result = create_keypair(&mut OsRng);
        assert!(result.is_ok());
        let (signing_key, verifying_key) = result.unwrap();
        assert_eq!(VerifyingKey::from(&signing_key), verifying_key);
    }

    #[test]
    fn test_sign_and_verify() {
        let (signing_key, verifying_key) = create_keypair(&mut OsRng).unwrap();
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
        let (pk, sk) = RSASigningKey::generate().unwrap();
        let msg = b"test";
        let signature = unblinded_rsa_sign(&sk, msg).unwrap();
        assert!(pk.verify_signature(&signature, msg).is_ok());
        assert!(pk.verify_signature(&signature, b"tesu").is_err());
    }
}
