//! Notary certificate — the PKI intermediate in the chain
//! `master → notary → ghost key`.
//!
//! Historically this was called "delegate certificate"; it was renamed to
//! "notary" in 0.2.0 to deconflict with Freenet's own `Delegate` (WASM agent)
//! concept. See issue freenet/web#24.
//!
//! Wire-format compatibility: the CBOR field names inside the signed payload
//! are frozen as `delegate_verifying_key` (and the enclosing `delegate` field
//! on `GhostkeyCertificateV1`) via `#[serde(rename = "...")]`. Bumping to a V2
//! format was rejected as unnecessary churn — only the Rust source names were
//! wrong, not the bytes. Because signatures cover the serialized payload
//! bytes, changing the CBOR key names would invalidate every existing cert.

use super::errors::GhostkeyError;
use super::util::{sign_with_hash, verify_with_hash};
use crate::armorable::Armorable;
use crate::FREENET_MASTER_VERIFYING_KEY_BASE64;
use blind_rsa_signatures::{
    KeyPair as RSAKeyPair, PublicKey as RSAVerifyingKey, SecretKey as RSASigningKey,
};
use ed25519_dalek::*;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct NotaryCertificateV1 {
    pub payload: NotaryPayload,
    /// The payload signed by the master signing key
    pub signature: Signature,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NotaryPayload {
    #[serde(rename = "delegate_verifying_key")]
    pub notary_verifying_key: RSAVerifyingKey,
    pub info: String,
}

impl NotaryCertificateV1 {
    pub fn new(
        master_signing_key: &SigningKey,
        info: &String,
    ) -> Result<(Self, RSASigningKey), Box<GhostkeyError>> {
        let notary_keypair = RSAKeyPair::generate(&mut OsRng, 2048)
            .map_err(|e| GhostkeyError::RSAError(e.to_string()))?;

        let payload = NotaryPayload {
            notary_verifying_key: notary_keypair.pk,
            info: info.clone(),
        };

        let signature = sign_with_hash(&master_signing_key, &payload)?;

        let certificate = NotaryCertificateV1 {
            payload,
            signature: Signature::from(signature),
        };

        Ok((certificate, notary_keypair.sk))
    }

    /// Verifies the notary certificate using the master verifying key. If the
    /// verification is successful, the info field of the payload is returned.
    /// Uses the Freenet master verifying key if no key is provided.
    pub fn verify(
        &self,
        &master_verifying_key: &Option<VerifyingKey>,
    ) -> Result<String, Box<GhostkeyError>> {
        let master_verifying_key = master_verifying_key
            .unwrap_or(VerifyingKey::from_base64(FREENET_MASTER_VERIFYING_KEY_BASE64).unwrap());

        let verification = verify_with_hash(&master_verifying_key, &self.payload, &self.signature)?;
        if verification {
            Ok(self.payload.info.clone())
        } else {
            Err(Box::new(GhostkeyError::SignatureVerificationError(
                "Failed to verify notary certificate".to_string(),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::create_keypair;

    #[test]
    fn test_notary_certificate_creation_and_verification() {
        let (master_signing_key, master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        let info = "Test Notary".to_string();
        let (certificate, _notary_signing_key) =
            NotaryCertificateV1::new(&master_signing_key, &info).unwrap();

        let verified_info = certificate.verify(&Some(master_verifying_key)).unwrap();
        assert_eq!(verified_info, info);
    }

    #[test]
    fn test_notary_certificate_invalid_signature() {
        let (master_signing_key, _) = create_keypair(&mut OsRng).unwrap();
        let (_, wrong_verifying_key) = create_keypair(&mut OsRng).unwrap();

        let info = "Test Notary".to_string();
        let (certificate, _notary_signing_key) =
            NotaryCertificateV1::new(&master_signing_key, &info).unwrap();

        let result = certificate.verify(&Some(wrong_verifying_key));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().as_ref(),
            GhostkeyError::SignatureVerificationError(_)
        ));
    }

    #[test]
    fn test_notary_certificate_payload_integrity() {
        let (master_signing_key, master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        let info = "Test Notary".to_string();
        let (mut certificate, _notary_signing_key) =
            NotaryCertificateV1::new(&master_signing_key, &info).unwrap();

        assert!(certificate.verify(&Some(master_verifying_key)).is_ok());

        certificate.payload.info = "Tampered Info".to_string();

        let result = certificate.verify(&Some(master_verifying_key));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().as_ref(),
            GhostkeyError::SignatureVerificationError(_)
        ));
    }
}
