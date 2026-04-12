use super::errors::GhostkeyError;
use super::errors::GhostkeyError::{RSAError, SignatureVerificationError};
use super::notary_certificate::NotaryCertificateV1;
use super::util::{create_keypair, unblinded_rsa_sign};
use crate::armorable::Armorable;
use blind_rsa_signatures::{
    KeyPair, Options, SecretKey as RSASigningKey, Signature as RSASignature,
};
use ed25519_dalek::*;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct GhostkeyCertificateV1 {
    /// The notary certificate that attests to this ghost key.
    ///
    /// The CBOR field name is frozen as `"delegate"` via `#[serde(rename)]`
    /// for backward compatibility with certs issued before the 0.1.5 rename.
    /// See [`crate::notary_certificate`] for the rename rationale.
    #[serde(rename = "delegate")]
    pub notary: NotaryCertificateV1,
    pub verifying_key: VerifyingKey,
    /// signing_key signed by the notary signing key
    pub signature: RSASignature,
}

impl GhostkeyCertificateV1 {
    pub fn new(
        notary_certificate: &NotaryCertificateV1,
        notary_signing_key: &RSASigningKey,
    ) -> (Self, SigningKey) {
        let notary_keypair = KeyPair::new(
            notary_signing_key.public_key().unwrap(),
            notary_signing_key.clone(),
        );
        let (ghost_signing_key, ghost_verifying_key) = create_keypair(&mut OsRng).unwrap();
        let ghost_signing_key = SigningKey::from(ghost_signing_key);
        let ghost_verifying_key = VerifyingKey::from(ghost_verifying_key);

        (
            Self {
                notary: notary_certificate.clone(),
                verifying_key: ghost_verifying_key.clone(),
                signature: unblinded_rsa_sign(
                    &notary_keypair,
                    &Armorable::to_bytes(&ghost_verifying_key).unwrap(),
                )
                .unwrap(),
            },
            ghost_signing_key.clone(),
        )
    }

    pub fn verify(
        &self,
        master_verifying_key: &Option<VerifyingKey>,
    ) -> Result<String, Box<GhostkeyError>> {
        // Verify notary certificate
        let info = self
            .notary
            .verify(master_verifying_key)
            .map_err(|e| SignatureVerificationError(format!("Failed to verify notary: {}", e)))?;

        // Verify ghostkey certificate
        let verification = self
            .notary
            .payload
            .notary_verifying_key
            .verify(
                &self.signature,
                None,
                Armorable::to_bytes(&self.verifying_key).unwrap(),
                &Options::default(),
            )
            .map_err(|e| RSAError(format!("Failed to verify ghostkey: {}", e)));

        match verification {
            Ok(_) => Ok(info),
            Err(e) => Err(Box::new(SignatureVerificationError(format!(
                "Failed to verify ghostkey certificate: {}",
                e
            )))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ghost_key_certificate_creation_and_verification() {
        let (master_signing_key, master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        let info = "Test Notary".to_string();
        let (notary_certificate, notary_signing_key) =
            NotaryCertificateV1::new(&master_signing_key, &info).unwrap();

        let (ghost_key_certificate, _ghost_key_signing_key) =
            GhostkeyCertificateV1::new(&notary_certificate, &notary_signing_key);

        let verified_info = ghost_key_certificate
            .verify(&Some(master_verifying_key))
            .unwrap();
        assert_eq!(verified_info, info);
    }

    #[test]
    fn test_ghost_key_certificate_invalid_master_key() {
        let (master_signing_key, _) = create_keypair(&mut OsRng).unwrap();
        let (_, wrong_master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        let info = "Test Notary".to_string();
        let (notary_certificate, notary_signing_key) =
            NotaryCertificateV1::new(&master_signing_key, &info).unwrap();

        let (ghost_key_certificate, _ghost_key_signing_key) =
            GhostkeyCertificateV1::new(&notary_certificate, &notary_signing_key);

        let result = ghost_key_certificate.verify(&Some(wrong_master_verifying_key));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().as_ref(),
            SignatureVerificationError(_)
        ));
    }

    #[test]
    fn test_ghost_key_certificate_tampered_notary() {
        let (master_signing_key, master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        let info = "Test Notary".to_string();
        let (notary_certificate, notary_signing_key): (NotaryCertificateV1, RSASigningKey) =
            NotaryCertificateV1::new(&master_signing_key, &info).unwrap();

        let (mut ghost_key_certificate, _ghost_key_signing_key) =
            GhostkeyCertificateV1::new(&notary_certificate, &notary_signing_key);

        ghost_key_certificate.notary.payload.info = "Tampered Info".to_string();

        let result = ghost_key_certificate.verify(&Some(master_verifying_key));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().as_ref(),
            SignatureVerificationError(_)
        ));
    }

    #[test]
    fn test_ghost_key_certificate_tampered_ghostkey() {
        let (master_signing_key, master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        let info = "Test Notary".to_string();
        let (notary_certificate, notary_signing_key) =
            NotaryCertificateV1::new(&master_signing_key, &info).unwrap();

        let (mut ghost_key_certificate, _ghost_key_signing_key) =
            GhostkeyCertificateV1::new(&notary_certificate, &notary_signing_key);

        let (_, tampered_verifying_key) = create_keypair(&mut OsRng).unwrap();
        ghost_key_certificate.verifying_key = VerifyingKey::from(tampered_verifying_key);

        let result = ghost_key_certificate.verify(&Some(master_verifying_key));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().as_ref(),
            SignatureVerificationError(_)
        ));
    }
}
