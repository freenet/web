use p256::ecdsa::{VerifyingKey};
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{self, Visitor};
use std::fmt;
use std::convert::TryFrom;
use base64::engine::general_purpose;
use base64::Engine;
use crate::armorable::Armorable;

#[derive(Clone)]
pub struct SerializableVerifyingKey(pub VerifyingKey);

impl Armorable for SerializableVerifyingKey {}

impl From<VerifyingKey> for SerializableVerifyingKey {
    fn from(key: VerifyingKey) -> Self {
        SerializableVerifyingKey(key)
    }
}

impl TryFrom<SerializableVerifyingKey> for VerifyingKey {
    type Error = p256::ecdsa::Error;

    fn try_from(serializable_key: SerializableVerifyingKey) -> Result<Self, Self::Error> {
        Ok(serializable_key.0)
    }
}

impl SerializableVerifyingKey {
    pub fn as_verifying_key(&self) -> &VerifyingKey {
        &self.0
    }
}

// Implementing Serialize manually
impl Serialize for SerializableVerifyingKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = self.0.to_sec1_bytes();
        serializer.serialize_bytes(&bytes)
    }
}

// Implementing Deserialize manually
impl<'de> Deserialize<'de> for SerializableVerifyingKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SerializableVerifyingKeyVisitor;

        impl<'de> Visitor<'de> for SerializableVerifyingKeyVisitor {
            type Value = SerializableVerifyingKey;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid ECDSA verifying key in byte array form")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let verifying_key = bytes_to_verifying_key(v).map_err(de::Error::custom)?;
                Ok(SerializableVerifyingKey(verifying_key))
            }
        }

        deserializer.deserialize_bytes(SerializableVerifyingKeyVisitor)
    }
}

impl AsRef<VerifyingKey> for SerializableVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use p256::ecdsa::SigningKey;
    use rand_core::OsRng;

    #[test]
    fn test_serializable_verifying_key_roundtrip() {
        // Generate a random signing key and get its verifying key
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        // Create a SerializableVerifyingKey
        let serializable_key = SerializableVerifyingKey::from(*verifying_key);

        // Serialize to JSON
        let serialized = serde_json::to_string(&serializable_key).expect("Failed to serialize");

        // Deserialize from JSON
        let deserialized: SerializableVerifyingKey = serde_json::from_str(&serialized).expect("Failed to deserialize");

        // Compare the original and deserialized keys
        assert_eq!(
            verifying_key.to_encoded_point(false).as_bytes(),
            deserialized.as_ref().to_encoded_point(false).as_bytes()
        );
    }

    #[test]
    fn test_serializable_verifying_key_display() {
        // Generate a random signing key and get its verifying key
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        // Create a SerializableVerifyingKey
        let serializable_key = SerializableVerifyingKey::from(*verifying_key);

        // Get the display string
        let display_string = format!("{}", serializable_key);

        // Ensure the display string is not empty and is a valid base64 string
        assert!(!display_string.is_empty());
        assert!(base64::decode(&display_string).is_ok());
    }

    #[test]
    fn test_serializable_verifying_key_as_ref() {
        // Generate a random signing key and get its verifying key
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        // Create a SerializableVerifyingKey
        let serializable_key = SerializableVerifyingKey::from(*verifying_key);

        // Test as_ref method
        let key_ref: &VerifyingKey = serializable_key.as_ref();
        assert_eq!(
            verifying_key.to_encoded_point(false).as_bytes(),
            key_ref.to_encoded_point(false).as_bytes()
        );
    }
}

impl std::fmt::Display for SerializableVerifyingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", general_purpose::STANDARD.encode(self.0.to_sec1_bytes()))
    }
}

use p256::elliptic_curve::generic_array::GenericArray;
use p256::elliptic_curve::generic_array::typenum::U33;

fn bytes_to_verifying_key(bytes: &[u8]) -> Result<VerifyingKey, p256::ecdsa::Error> {
    let array: GenericArray<u8, U33> = GenericArray::clone_from_slice(bytes);
    VerifyingKey::from_sec1_bytes(&array)
}
