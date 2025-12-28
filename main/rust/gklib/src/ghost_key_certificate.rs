use super::delegate_certificate::DelegateCertificateV1;
use super::errors::GhostkeyError;
use super::errors::GhostkeyError::{RSAError, SignatureVerificationError};
use super::util::{create_keypair, unblinded_rsa_sign};
use blind_rsa_signatures::{
    KeyPair, Options, SecretKey as RSASigningKey, Signature as RSASignature,
};
use ed25519_dalek::*;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use crate::armorable::Armorable;

#[derive(Serialize, Deserialize, Clone)]
pub struct GhostkeyCertificateV1 {
    pub delegate: DelegateCertificateV1,
    pub verifying_key: VerifyingKey,
    /// signing_key signed by the delegate signing key
    pub signature: RSASignature,
}

impl GhostkeyCertificateV1 {
    pub fn new(
        delegate_certificate: &DelegateCertificateV1,
        delegate_signing_key: &RSASigningKey,
    ) -> (Self, SigningKey) {
        let delegate_keypair = KeyPair::new(
            delegate_signing_key.public_key().unwrap(),
            delegate_signing_key.clone(),
        );
        let (ghost_signing_key, ghost_verifying_key) = create_keypair(&mut OsRng).unwrap();
        let ghost_signing_key = SigningKey::from(ghost_signing_key);
        let ghost_verifying_key = VerifyingKey::from(ghost_verifying_key);

        (
            Self {
                delegate: delegate_certificate.clone(),
                verifying_key: ghost_verifying_key.clone(),
                signature: unblinded_rsa_sign(&delegate_keypair, &Armorable::to_bytes(&ghost_verifying_key).unwrap())
                    .unwrap(),
            },
            ghost_signing_key.clone(),
        )
    }

    pub fn verify(
        &self,
        master_verifying_key: &Option<VerifyingKey>,
    ) -> Result<String, Box<GhostkeyError>> {
        // Verify delegate certificate
        let info = self
            .delegate
            .verify(master_verifying_key)
            .map_err(|e| SignatureVerificationError(format!("Failed to verify delegate: {}", e)))?;

        // Verify ghostkey certificate
        let verification = self
            .delegate
            .payload
            .delegate_verifying_key
            .verify(
                &self.signature,
                None,
                Armorable::to_bytes(&self.verifying_key).unwrap(),
                &Options::default(),
            )
            .map_err(|e| RSAError(format!("Failed to verify ghostkey: {}", e)));

        match verification {
            Ok(_) => Ok(info),
            Err(e) => Err(Box::new(SignatureVerificationError(
                format!("Failed to verify ghostkey certificate: {}", e),
            ))),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ghost_key_certificate_creation_and_verification() {
        // Create a master key pair
        let (master_signing_key, master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (delegate_certificate, delegate_signing_key) =
            DelegateCertificateV1::new(&master_signing_key, &info).unwrap();

        // Create a ghostkey certificate
        let (ghost_key_certificate, _ghost_key_signing_key) =
            GhostkeyCertificateV1::new(&delegate_certificate, &delegate_signing_key);

        // Verify the ghostkey certificate
        let verified_info = ghost_key_certificate
            .verify(&Some(master_verifying_key))
            .unwrap();
        assert_eq!(verified_info, info);
    }

    #[test]
    fn test_ghost_key_certificate_invalid_master_key() {
        // Create two sets of key pairs
        let (master_signing_key, _) = create_keypair(&mut OsRng).unwrap();
        let (_, wrong_master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (delegate_certificate, delegate_signing_key) =
            DelegateCertificateV1::new(&master_signing_key, &info).unwrap();

        // Create a ghostkey certificate
        let (ghost_key_certificate, _ghost_key_signing_key) =
            GhostkeyCertificateV1::new(&delegate_certificate, &delegate_signing_key);

        // Try to verify with the wrong master key
        let result = ghost_key_certificate.verify(&Some(wrong_master_verifying_key));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().as_ref(),
            SignatureVerificationError(_)
        ));
    }

    #[test]
    fn test_ghost_key_certificate_tampered_delegate() {
        // Create a master key pair
        let (master_signing_key, master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (delegate_certificate, delegate_signing_key): (DelegateCertificateV1, RSASigningKey) =
            DelegateCertificateV1::new(&master_signing_key, &info).unwrap();

        // Create a ghostkey certificate
        let (mut ghost_key_certificate, _ghost_key_signing_key) =
            GhostkeyCertificateV1::new(&delegate_certificate, &delegate_signing_key);

        // Tamper with the delegate certificate
        ghost_key_certificate.delegate.payload.info = "Tampered Info".to_string();

        // Try to verify the tampered certificate
        let result = ghost_key_certificate.verify(&Some(master_verifying_key));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().as_ref(),
            SignatureVerificationError(_)
        ));
    }

    #[test]
    fn test_ghost_key_certificate_tampered_ghostkey() {
        // Create a master key pair
        let (master_signing_key, master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (delegate_certificate, delegate_signing_key) =
            DelegateCertificateV1::new(&master_signing_key, &info).unwrap();

        // Create a ghostkey certificate
        let (mut ghost_key_certificate, _ghost_key_signing_key) =
            GhostkeyCertificateV1::new(&delegate_certificate, &delegate_signing_key);

        // Tamper with the ghostkey verifying key
        let (_, tampered_verifying_key) = create_keypair(&mut OsRng).unwrap();
        ghost_key_certificate.verifying_key = VerifyingKey::from(tampered_verifying_key);

        // Try to verify the tampered certificate
        let result = ghost_key_certificate.verify(&Some(master_verifying_key));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().as_ref(),
            SignatureVerificationError(_)
        ));
    }
}

