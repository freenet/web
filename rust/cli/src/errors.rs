use GhostkeyError::*;

#[derive(Debug, PartialEq)]
pub enum GhostkeyError {
    KeyCreationError(String, i32),
    SignatureError(String, i32),
    SignatureVerificationError(String, i32),
    Base64DecodeError(String, i32),
    SerializationError(String, i32),
    DeserializationError(String, i32),
    InvalidInput(String, i32),
    ArmorError(String, i32),
    ValidationError(String, i32),
    DecodingError(String, i32),
    IOError(String, i32),
}

impl GhostkeyError {
    pub fn exit_code(&self) -> i32 {
        match self {
            GhostkeyError::KeyCreationError(_, code) => *code,
            GhostkeyError::SignatureError(_, code) => *code,
            GhostkeyError::SignatureVerificationError(_, code) => *code,
            GhostkeyError::Base64DecodeError(_, code) => *code,
            GhostkeyError::SerializationError(_, code) => *code,
            GhostkeyError::DeserializationError(_, code) => *code,
            GhostkeyError::InvalidInput(_, code) => *code,
            GhostkeyError::ArmorError(_, code) => *code,
            GhostkeyError::ValidationError(_, code) => *code,
            GhostkeyError::DecodingError(_, code) => *code,
            GhostkeyError::IOError(_, code) => *code,
        }
    }
}


impl std::error::Error for GhostkeyError {}

impl std::fmt::Display for GhostkeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            KeyCreationError(msg, _) => write!(f, "Key Creation Error: {}", msg),
            SignatureError(msg, _) => write!(f, "Signature Error: {}", msg),
            SignatureVerificationError(msg, _) => write!(f, "Signature Verification Error: {}", msg),
            Base64DecodeError(msg, _) => write!(f, "Base64 Decode Error: {}", msg),
            SerializationError(msg, _) => write!(f, "Serialization Error: {}", msg),
            DeserializationError(msg, _) => write!(f, "Deserialization Error: {}", msg),
            InvalidInput(msg, _) => write!(f, "Invalid Input: {}", msg),
            ArmorError(msg, _) => write!(f, "Armor Error: {}", msg),
            ValidationError(msg, _) => write!(f, "Validation Error: {}", msg),
            DecodingError(msg, _) => write!(f, "Decoding Error: {}", msg),
            IOError(msg, _) => write!(f, "IO Error: {}", msg),
        }
    }
}

impl From<String> for GhostkeyError {
    fn from(error: String) -> Self {
        GhostkeyError::InvalidInput(error, 1)
    }
}
