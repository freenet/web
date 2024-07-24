use p256::ecdsa::{SigningKey, signature::Signer};
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{self, Visitor};
use std::fmt;
use std::convert::TryFrom;
use p256::elliptic_curve::generic_array::GenericArray;
use p256::elliptic_curve::subtle::Choice;
use p256::elliptic_curve::rand_core::RngCore;
use p256::elliptic_curve::rand_core::OsRng;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::elliptic_curve::FieldBytes;
use base64;

#[derive(Clone)]
pub struct SerializableSigningKey(pub SigningKey);

impl From<SigningKey> for SerializableSigningKey {
    fn from(key: SigningKey) -> Self {
        SerializableSigningKey(key)
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

// Implementing Serialize manually
impl Serialize for SerializableSigningKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = self.0.to_bytes();
        serializer.serialize_bytes(&bytes)
    }
}

// Implementing Deserialize manually
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
                let key_bytes: FieldBytes = GenericArray::clone_from_slice(v);
                let key = SigningKey::from_bytes(&key_bytes).map_err(de::Error::custom)?;
                Ok(SerializableSigningKey(key))
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

impl std::fmt::Display for SerializableSigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key_bytes = self.0.to_bytes();
        write!(f, "{}", base64::encode(key_bytes))
    }
}
