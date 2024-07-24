use p256::ecdsa::SigningKey;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{self, Visitor};
use std::fmt;
use std::convert::TryFrom;
use base64::engine::general_purpose;
use base64::Engine;
use crate::armorable::Armorable;

#[derive(Clone)]
pub struct SerializableSigningKey(pub SigningKey);

impl Armorable for SerializableSigningKey {}

impl From<SigningKey> for SerializableSigningKey {
    fn from(key: SigningKey) -> Self {
        Self(key)
    }
}

impl TryFrom<SerializableSigningKey> for SigningKey {
    type Error = p256::ecdsa::Error;

    fn try_from(serializable_key: SerializableSigningKey) -> Result<Self, Self::Error> {
        Ok(serializable_key.0)
    }
}

impl SerializableSigningKey {
    pub fn as_signing_key(&self) -> &SigningKey {
        &self.0
    }
}

impl Serialize for SerializableSigningKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = self.0.to_bytes();
        serializer.serialize_bytes(&bytes)
    }
}

impl<'de> Deserialize<'de> for SerializableSigningKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SerializableSigningKeyVisitor;

        impl<'de> Visitor<'de> for SerializableSigningKeyVisitor {
            type Value = SerializableSigningKey;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid ECDSA signing key in byte array form")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                SigningKey::from_bytes(v.into())
                    .map(SerializableSigningKey)
                    .map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_bytes(SerializableSigningKeyVisitor)
    }
}

impl AsRef<SigningKey> for SerializableSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use rand_core::OsRng;

    #[test]
    fn test_serializable_signing_key_roundtrip() {
        // Generate a random signing key
        let signing_key = SigningKey::random(&mut OsRng);

        // Create a SerializableSigningKey
        let serializable_key = SerializableSigningKey::from(signing_key);

        // Serialize to JSON
        let serialized = serde_json::to_string(&serializable_key).expect("Failed to serialize");

        // Deserialize from JSON
        let deserialized: SerializableSigningKey = serde_json::from_str(&serialized).expect("Failed to deserialize");

        // Compare the original and deserialized keys
        assert_eq!(
            serializable_key.as_ref().to_bytes().as_slice(),
            deserialized.as_ref().to_bytes().as_slice()
        );
    }

    #[test]
    fn test_serializable_signing_key_display() {
        // Generate a random signing key
        let signing_key = SigningKey::random(&mut OsRng);

        // Create a SerializableSigningKey
        let serializable_key = SerializableSigningKey::from(signing_key);

        // Get the display string
        let display_string = format!("{}", serializable_key);

        // Ensure the display string is not empty and is a valid base64 string
        assert!(!display_string.is_empty());
        assert!(general_purpose::STANDARD.decode(&display_string).is_ok());
    }

    #[test]
    fn test_serializable_signing_key_as_ref() {
        // Generate a random signing key
        let signing_key = SigningKey::random(&mut OsRng);

        // Create a SerializableSigningKey
        let serializable_key = SerializableSigningKey::from(signing_key);

        // Test as_ref method
        let key_ref: &SigningKey = serializable_key.as_ref();
        assert_eq!(serializable_key.as_ref().to_bytes().as_slice(), key_ref.to_bytes().as_slice());
    }
}

impl std::fmt::Display for SerializableSigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", general_purpose::STANDARD.encode(self.0.to_bytes()))
    }
}
