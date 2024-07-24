use p256::ecdsa::{Signature, signature::Signer, signature::Verifier, SigningKey, VerifyingKey};
use rand_core::OsRng;

use crate::armorable::Armorable;
use crate::errors::GhostkeyError;

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

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Serialize, Deserialize)]
    struct TestData {
        field1: String,
        field2: i32,
    }

    impl Armorable for TestData {}

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
        let signature = sign(&signing_key, &test_data).unwrap();

        // Verify the signature
        let is_valid = verify(&verifying_key, &test_data, &signature).unwrap();
        assert!(is_valid);

        // Modify the data and verify again (should fail)
        let modified_data = TestData {
            field1: "Hello".to_string(),
            field2: 43,
        };
        let is_valid = verify(&verifying_key, &modified_data, &signature).unwrap();
        assert!(!is_valid);
    }
}
