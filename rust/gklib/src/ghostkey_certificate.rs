use super::delegate_certificate::DelegateCertificate;
use super::errors::GhostkeyError;
use super::errors::GhostkeyError::{RSAError, SignatureVerificationError};
use super::util::{create_keypair, unblinded_rsa_sign};
use blind_rsa_signatures::{
    KeyPair, Options, SecretKey as RSASigningKey, Signature as RSASignature,
};
use ed25519_dalek::*;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GhostkeyCertificate {
    pub delegate: DelegateCertificate,
    pub verifying_key: VerifyingKey,
    /// signing_key signed by the delegate signing key
    pub signature: RSASignature,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ghostkey_certificate_creation_and_verification() {
        // Create a master key pair
        let (master_signing_key, master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (delegate_certificate, delegate_signing_key) =
            DelegateCertificate::new(&master_signing_key, &info).unwrap();

        // Create a ghostkey certificate
        let (ghostkey_certificate, ghostkey_signing_key) =
            GhostkeyCertificate::new(&delegate_certificate, &delegate_signing_key);

        // Verify the ghostkey certificate
        let verified_info = ghostkey_certificate
            .verify(&master_verifying_key)
            .unwrap();
        assert_eq!(verified_info, info);
    }

    #[test]
    fn test_ghostkey_certificate_invalid_master_key() {
        // Create two sets of key pairs
        let (master_signing_key, _) = create_keypair(&mut OsRng).unwrap();
        let (_, wrong_master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (delegate_certificate, delegate_signing_key) =
            DelegateCertificate::new(&master_signing_key, &info).unwrap();

        // Create a ghostkey certificate
        let (ghostkey_certificate, _ghostkey_signing_key) =
            GhostkeyCertificate::new(&delegate_certificate, &delegate_signing_key);

        // Try to verify with the wrong master key
        let result = ghostkey_certificate.verify(&wrong_master_verifying_key);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().as_ref(),
            GhostkeyError::SignatureVerificationError(_)
        ));
    }

    #[test]
    fn test_ghostkey_certificate_tampered_delegate() {
        // Create a master key pair
        let (master_signing_key, master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (delegate_certificate, delegate_signing_key): (DelegateCertificate, RSASigningKey) =
            DelegateCertificate::new(&master_signing_key, &info).unwrap();

        // Create a ghostkey certificate
        let (mut ghostkey_certificate, _ghostkey_signing_key) =
            GhostkeyCertificate::new(&delegate_certificate, &delegate_signing_key);

        // Tamper with the delegate certificate
        ghostkey_certificate.delegate.payload.info = "Tampered Info".to_string();

        // Try to verify the tampered certificate
        let result = ghostkey_certificate.verify(&master_verifying_key);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().as_ref(),
            GhostkeyError::SignatureVerificationError(_)
        ));
    }

    #[test]
    fn test_ghostkey_certificate_tampered_ghostkey() {
        // Create a master key pair
        let (master_signing_key, master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (delegate_certificate, delegate_signing_key) =
            DelegateCertificate::new(&master_signing_key, &info).unwrap();

        // Create a ghostkey certificate
        let (mut ghostkey_certificate, _ghostkey_signing_key) =
            GhostkeyCertificate::new(&delegate_certificate, &delegate_signing_key);

        // Tamper with the ghostkey verifying key
        let (_, tampered_verifying_key) = create_keypair(&mut OsRng).unwrap();
        ghostkey_certificate.verifying_key = VerifyingKey::from(tampered_verifying_key);

        // Try to verify the tampered certificate
        let result = ghostkey_certificate.verify(&master_verifying_key);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().as_ref(),
            SignatureVerificationError(_)
        ));
    }
}

impl GhostkeyCertificate {
    pub fn new(
        delegate_certificate: &DelegateCertificate,
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
                signature: unblinded_rsa_sign(&delegate_keypair, &ghost_verifying_key.to_bytes())
                    .unwrap(),
                //   signature: Signature::from(sign_with_hash(&delegate_signing_key, &ghost_verifying_key).unwrap()),
            },
            ghost_signing_key.clone(),
        )
    }

    pub fn verify(
        &self,
        master_verifying_key: &VerifyingKey,
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
                &self.verifying_key.to_bytes(),
                &Options::default(),
            )
            .map_err(|e| RSAError(format!("Failed to verify ghostkey: {}", e)));

        if verification.is_ok() {
            Ok(info)
        } else {
            Err(Box::new(SignatureVerificationError(
                "Failed to verify ghostkey certificate".to_string(),
            )))
        }
    }
}
