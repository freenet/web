use GhostkeyError::*;

#[derive(Debug, PartialEq)]
pub enum GhostkeyError {
    KeyCreationError(String),
    SignatureError(String),
    SignatureVerificationError(String),
    Base64DecodeError(String),
    SerializationError(String),
    DeserializationError(String),
    InvalidInput(String),
    ArmorError(String),
    ValidationError(String),
    DecodingError(String),
    IOError(String),
}


impl std::error::Error for GhostkeyError {}

impl std::fmt::Display for GhostkeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            KeyCreationError(msg) => write!(f, "Key Creation Error: {}", msg),
            SignatureError(msg) => write!(f, "Signature Error: {}", msg),
            SignatureVerificationError(msg) => write!(f, "Signature Verification Error: {}", msg),
            Base64DecodeError(msg) => write!(f, "Base64 Decode Error: {}", msg),
            SerializationError(msg) => write!(f, "Serialization Error: {}", msg),
            DeserializationError(msg) => write!(f, "Deserialization Error: {}", msg),
            InvalidInput(msg) => write!(f, "Invalid Input: {}", msg),
            ArmorError(msg) => write!(f, "Armor Error: {}", msg),
            ValidationError(msg) => write!(f, "Validation Error: {}", msg),
            DecodingError(msg) => write!(f, "Decoding Error: {}", msg),
            IOError(msg) => write!(f, "IO Error: {}", msg),
        }
    }
}

impl From<String> for GhostkeyError {
    fn from(error: String) -> Self {
        GhostkeyError::InvalidInput(error)
    }
}
