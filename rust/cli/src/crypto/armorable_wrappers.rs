use serde::{Serialize, Deserialize};
use p256::ecdsa::{SigningKey, VerifyingKey, Signature};
use crate::armorable::Armorable;
use crate::crypto::crypto_error::CryptoError;

#[derive(Serialize, Deserialize)]
pub struct ArmorableSigningKey(#[serde(with = "serde_bytes")] Vec<u8>);

impl From<&SigningKey> for ArmorableSigningKey {
    fn from(sk: &SigningKey) -> Self {
        ArmorableSigningKey(sk.to_bytes().to_vec())
    }
}

impl TryFrom<ArmorableSigningKey> for SigningKey {
    type Error = p256::elliptic_curve::Error;

    fn try_from(ask: ArmorableSigningKey) -> Result<Self, Self::Error> {
        SigningKey::from_bytes(ask.0.as_slice().into())
    }
}

impl Armorable for ArmorableSigningKey {
    fn armor_label() -> &'static str {
        "SERVER SIGNING KEY"
    }
}

#[derive(Serialize, Deserialize)]
pub struct ArmorableVerifyingKey(#[serde(with = "serde_bytes")] Vec<u8>);

impl From<&VerifyingKey> for ArmorableVerifyingKey {
    fn from(vk: &VerifyingKey) -> Self {
        ArmorableVerifyingKey(vk.to_sec1_bytes().to_vec())
    }
}

impl TryFrom<ArmorableVerifyingKey> for VerifyingKey {
    type Error = p256::elliptic_curve::Error;

    fn try_from(avk: ArmorableVerifyingKey) -> Result<Self, Self::Error> {
        VerifyingKey::from_sec1_bytes(&avk.0)
    }
}

impl Armorable for ArmorableVerifyingKey {
    fn armor_label() -> &'static str {
        "SERVER VERIFYING KEY"
    }
}

#[derive(Serialize, Deserialize)]
pub struct ArmorableSignature(#[serde(with = "serde_bytes")] Vec<u8>);

impl From<&Signature> for ArmorableSignature {
    fn from(sig: &Signature) -> Self {
        ArmorableSignature(sig.to_vec())
    }
}

impl TryFrom<ArmorableSignature> for Signature {
    type Error = p256::elliptic_curve::Error;

    fn try_from(as_: ArmorableSignature) -> Result<Self, Self::Error> {
        Signature::from_slice(&as_.0)
    }
}

impl Armorable for ArmorableSignature {
    fn armor_label() -> &'static str {
        "SIGNATURE"
    }
}

pub trait ArmorableKey<T> {
    fn to_armored(&self) -> Result<String, CryptoError>;
    fn from_armored(armored: &str) -> Result<T, CryptoError>;
}

impl ArmorableKey<SigningKey> for SigningKey {
    fn to_armored(&self) -> Result<String, CryptoError> {
        ArmorableSigningKey::from(self).to_base64_armored()
            .map_err(|e| CryptoError::SerializationError(e.to_string()))
    }

    fn from_armored(armored: &str) -> Result<SigningKey, CryptoError> {
        let ask = ArmorableSigningKey::from_base64_armored(armored)
            .map_err(|e| CryptoError::DeserializationError(e.to_string()))?;
        SigningKey::try_from(ask)
            .map_err(|e| CryptoError::KeyCreationError(e.to_string()))
    }
}

impl ArmorableKey<VerifyingKey> for VerifyingKey {
    fn to_armored(&self) -> Result<String, CryptoError> {
        ArmorableVerifyingKey::from(self).to_base64_armored()
            .map_err(|e| CryptoError::SerializationError(e.to_string()))
    }

    fn from_armored(armored: &str) -> Result<VerifyingKey, CryptoError> {
        let avk = ArmorableVerifyingKey::from_base64_armored(armored)
            .map_err(|e| CryptoError::DeserializationError(e.to_string()))?;
        VerifyingKey::try_from(avk)
            .map_err(|e| CryptoError::KeyCreationError(e.to_string()))
    }
}

impl ArmorableKey<Signature> for Signature {
    fn to_armored(&self) -> Result<String, CryptoError> {
        ArmorableSignature::from(self).to_base64_armored()
            .map_err(|e| CryptoError::SerializationError(e.to_string()))
    }

    fn from_armored(armored: &str) -> Result<Signature, CryptoError> {
        let as_ = ArmorableSignature::from_base64_armored(armored)
            .map_err(|e| CryptoError::DeserializationError(e.to_string()))?;
        Signature::try_from(as_)
            .map_err(|e| CryptoError::InvalidInput(e.to_string()))
    }
}
