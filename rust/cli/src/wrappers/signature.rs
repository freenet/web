use p256::ecdsa::Signature;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{self, Visitor};
use std::fmt;
use std::convert::TryFrom;
use p256::elliptic_curve::generic_array::GenericArray;
use base64::engine::general_purpose;
use base64::Engine;

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
        serializer.serialize_bytes(&bytes)
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
                formatter.write_str("a valid ECDSA signature in byte array form")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // The length for a p256 signature is 64 bytes.
                if v.len() != 64 {
                    return Err(de::Error::invalid_length(v.len(), &self));
                }

                let sig_bytes: GenericArray<u8, typenum::U64> = GenericArray::clone_from_slice(v);
                let sig = Signature::from_bytes(&sig_bytes).map_err(de::Error::custom)?;
                Ok(SerializableSignature(sig))
            }
        }

        deserializer.deserialize_bytes(SerializableSignatureVisitor)
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
