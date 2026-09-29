use ghostkey_lib::errors::GhostkeyError;
use serde::de::StdError;

#[derive(Debug)]
pub enum CertificateError {
    StripeError(stripe::StripeError),
    PaymentNotSuccessful,
    PaymentMethodMissing,
    CertificateAlreadySigned,
    /// The client blinded against a different notary certificate than the one
    /// its PaymentIntent was quoted, so a signature would not unblind.
    NotaryMismatch,
    Base64Error(base64::DecodeError),
    KeyError(String),
    ParseIdError(stripe::ParseIdError),
    MiscError(String),
    GhostkeyError(GhostkeyError),
}

impl std::fmt::Display for CertificateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CertificateError::StripeError(e) => write!(f, "Stripe error: {}", e),
            CertificateError::PaymentNotSuccessful => write!(f, "Payment not successful"),
            CertificateError::PaymentMethodMissing => write!(f, "Payment method is missing"),
            CertificateError::CertificateAlreadySigned => write!(f, "Certificate already signed"),
            CertificateError::NotaryMismatch => {
                write!(f, "Notary certificate does not match this payment")
            }
            CertificateError::Base64Error(e) => write!(f, "Base64 decoding error: {}", e),
            CertificateError::KeyError(e) => write!(f, "Key error: {}", e),
            CertificateError::ParseIdError(e) => write!(f, "Parse ID error: {}", e),
            CertificateError::MiscError(e) => write!(f, "Miscellaneous error: {}", e),
            CertificateError::GhostkeyError(e) => write!(f, "Ghost Key error: {}", e),
        }
    }
}

impl StdError for CertificateError {}

impl From<stripe::StripeError> for CertificateError {
    fn from(error: stripe::StripeError) -> Self {
        CertificateError::StripeError(error)
    }
}

impl From<base64::DecodeError> for CertificateError {
    fn from(error: base64::DecodeError) -> Self {
        CertificateError::Base64Error(error)
    }
}

impl From<stripe::ParseIdError> for CertificateError {
    fn from(error: stripe::ParseIdError) -> Self {
        CertificateError::ParseIdError(error)
    }
}

impl From<GhostkeyError> for CertificateError {
    fn from(error: GhostkeyError) -> Self {
        CertificateError::GhostkeyError(error)
    }
}
