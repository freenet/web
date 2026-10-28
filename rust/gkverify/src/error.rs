use core::fmt;

/// Why a certificate, signature or `info` string was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The input is over [`crate::MAX_CERTIFICATE_BYTES`].
    TooLarge,
    /// No armoured certificate block decodes.
    Armor,
    /// The bytes are not a Ghost Key certificate.
    Malformed,
    /// The master key given is weak: anyone could sign for it.
    WeakMasterKey,
    /// The master key did not sign the notary certificate.
    NotarySignature,
    /// The notary's RSA key is outside the accepted parameters.
    NotaryKey,
    /// The notary did not sign the ghost key.
    GhostKeySignature,
    /// The certificate certifies a weak key, which anyone can sign for.
    WeakGhostKey,
    /// A payload signature is not 64 bytes.
    PayloadSignatureLength,
    /// The ghost key did not sign the payload.
    PayloadSignature,
    /// The notary `info` is not in the current JSON format.
    Info,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::TooLarge => "the certificate is larger than the limit",
            Error::WeakMasterKey => "the master key is weak",
            Error::Armor => "no armoured Ghost Key certificate decodes",
            Error::Malformed => "not a Ghost Key certificate",
            Error::NotarySignature => "the master key did not sign the notary certificate",
            Error::NotaryKey => "the notary RSA key is outside the accepted parameters",
            Error::GhostKeySignature => "the notary did not sign the ghost key",
            Error::WeakGhostKey => "the certificate certifies a weak key",
            Error::PayloadSignatureLength => "a payload signature must be 64 bytes",
            Error::PayloadSignature => "the ghost key did not sign the payload",
            Error::Info => "the notary info is not in the current format",
        })
    }
}

impl std::error::Error for Error {}
