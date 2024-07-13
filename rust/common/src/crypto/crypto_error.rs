
#[derive(Debug, PartialEq)]
pub enum CryptoError {
    KeyCreationError(String),
    SignatureError(String),
    SignatureVerificationError(String),
    Base64DecodeError(String),
    SerializationError(String),
    DeserializationError(String),
    InvalidInput(String),
    ArmorError(String),
}


impl std::error::Error for CryptoError {}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CryptoError::KeyCreationError(msg) => write!(f, "Key Creation Error: {}", msg),
            CryptoError::SignatureError(msg) => write!(f, "Signature Error: {}", msg),
            CryptoError::SignatureVerificationError(msg) => write!(f, "Signature Verification Error: {}", msg),
            CryptoError::Base64DecodeError(msg) => write!(f, "Base64 Decode Error: {}", msg),
            CryptoError::SerializationError(msg) => write!(f, "Serialization Error: {}", msg),
            CryptoError::DeserializationError(msg) => write!(f, "Deserialization Error: {}", msg),
            CryptoError::InvalidInput(msg) => write!(f, "Invalid Input: {}", msg),
            CryptoError::ArmorError(msg) => write!(f, "Armor Error: {}", msg),
        }
    }
}

impl From<String> for CryptoError {
    fn from(error: String) -> Self {
        CryptoError::InvalidInput(error)
    }
}
