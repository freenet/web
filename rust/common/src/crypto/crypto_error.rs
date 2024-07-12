use super::*;

#[derive(Debug, PartialEq)]
pub enum CryptoError {
    IoError(String),
    Base64DecodeError(String),
    KeyCreationError(String),
    SerializationError(String),
    DeserializationError(String),
    InvalidInput(String),
    SignatureError(String),
    SignatureVerificationError(String),
    NotImplemented(String),
}

impl std::error::Error for CryptoError {}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CryptoError::IoError(e) => write!(f, "{}", e.red()),
            CryptoError::Base64DecodeError(e) => write!(f, "{}", e.red()),
            CryptoError::KeyCreationError(e) => write!(f, "{}", e.red()),
            CryptoError::SerializationError(e) => write!(f, "{}", e.red()),
            CryptoError::InvalidInput(e) => write!(f, "{}", e.red()),
        }
    }
}

impl From<String> for CryptoError {
    fn from(error: String) -> Self {
        CryptoError::InvalidInput(error)
    }
}
