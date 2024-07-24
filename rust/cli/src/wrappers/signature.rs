use p256::ecdsa::Signature;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{self, Visitor};
use std::fmt;
use std::convert::TryFrom;
use p256::elliptic_curve::generic_array::GenericArray;
use base64::{engine::general_purpose, Engine as _};

#[derive(Clone)]
pub struct SerializableSignature(pub Signature);

impl From<Signature> for SerializableSignature {
    fn from(sig: Signature) -> Self {
        SerializableSignature(sig)
    }
}

impl TryFrom<SerializableSignature> for Signature {
    type Error = p256::ecdsa::Error;

    fn try_from(serializable_sig: SerializableSignature) -> Result<Self, Self::Error> {
        Ok(serializable_sig.0)
    }
}

impl SerializableSignature {
    pub fn as_signature(&self) -> &Signature {
        &self.0
    }
}

// Implementing Serialize manually
impl Serialize for SerializableSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = self.0.to_bytes();
        let base64 = general_purpose::STANDARD.encode(bytes);
        serializer.serialize_str(&base64)
    }
}

// Implementing Deserialize manually
impl<'de> Deserialize<'de> for SerializableSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SerializableSignatureVisitor;

        impl<'de> Visitor<'de> for SerializableSignatureVisitor {
            type Value = SerializableSignature;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a base64 encoded ECDSA signature")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let bytes = general_purpose::STANDARD.decode(v).map_err(de::Error::custom)?;
                if bytes.len() != 64 {
                    return Err(de::Error::invalid_length(bytes.len(), &self));
                }

                let sig_bytes: GenericArray<u8, typenum::U64> = GenericArray::clone_from_slice(&bytes);
                let sig = Signature::from_bytes(&sig_bytes).map_err(de::Error::custom)?;
                Ok(SerializableSignature(sig))
            }
        }

        deserializer.deserialize_str(SerializableSignatureVisitor)
    }
}

impl AsRef<Signature> for SerializableSignature {
    fn as_ref(&self) -> &Signature {
        &self.0
    }
}

impl std::fmt::Display for SerializableSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sig_bytes = self.0.to_bytes();
        write!(f, "{}", general_purpose::STANDARD.encode(sig_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use p256::ecdsa::{SigningKey, signature::Signer};
    use rand_core::OsRng;

    #[test]
    fn test_serializable_signature_roundtrip() {
        // Generate a random signature
        let signing_key = SigningKey::random(&mut OsRng);
        let message = b"test message";
        let signature: Signature = signing_key.sign(message);

        // Create a SerializableSignature
        let serializable_sig = SerializableSignature::from(signature);

        // Serialize to JSON
        let serialized = serde_json::to_string(&serializable_sig).expect("Failed to serialize");

        // Deserialize from JSON
        let deserialized: SerializableSignature = serde_json::from_str(&serialized).expect("Failed to deserialize");

        // Compare the original and deserialized signatures
        assert_eq!(signature, deserialized.as_signature().clone());
    }

    #[test]
    fn test_serializable_signature_display() {
        // Generate a random signature
        let signing_key = SigningKey::random(&mut OsRng);
        let message = b"test message";
        let signature: Signature = signing_key.sign(message);

        // Create a SerializableSignature
        let serializable_sig = SerializableSignature::from(signature);

        // Get the display string
        let display_string = format!("{}", serializable_sig);

        // Ensure the display string is not empty and is a valid base64 string
        assert!(!display_string.is_empty());
        assert!(general_purpose::STANDARD.decode(&display_string).is_ok());
    }

    #[test]
    fn test_serializable_signature_as_ref() {
        // Generate a random signature
        let signing_key = SigningKey::random(&mut OsRng);
        let message = b"test message";
        let signature: Signature = signing_key.sign(message);

        // Create a SerializableSignature
        let serializable_sig = SerializableSignature::from(signature);

        // Test as_ref method
        let sig_ref: &Signature = serializable_sig.as_ref();
        assert_eq!(&signature, sig_ref);
    }
}
