use p256::ecdsa::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use crate::armorable::Armorable;
use crate::delegate_certificate::DelegateCertificate;
use crate::errors::GhostkeyError;
use crate::errors::GhostkeyError::SignatureVerificationError;
use crate::util::{create_keypair, sign_with_hash, verify_with_hash};
use crate::wrappers::signature::SerializableSignature;
use crate::wrappers::signing_key::SerializableSigningKey;
use crate::wrappers::verifying_key::SerializableVerifyingKey;

#[derive(Serialize, Deserialize)]
pub struct GhostkeyCertificate {
    pub delegate : DelegateCertificate,
    pub verifying_key : SerializableVerifyingKey,
    /// signing_key signed by the delegate signing key
    pub signature : SerializableSignature,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::create_keypair;

    #[test]
    fn test_ghostkey_certificate_creation_and_verification() {
        // Create a master key pair
        let (master_signing_key, master_verifying_key) = create_keypair().unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (delegate_certificate, delegate_signing_key) = DelegateCertificate::new(&master_signing_key, &info).unwrap();

        // Create a ghostkey certificate
        let (ghostkey_certificate, _ghostkey_signing_key) = GhostkeyCertificate::new(delegate_certificate, &delegate_signing_key);

        // Verify the ghostkey certificate
        let verified_info = ghostkey_certificate.verify(&Some(master_verifying_key)).unwrap();
        assert_eq!(verified_info, info);
    }

    #[test]
    fn test_ghostkey_certificate_invalid_master_key() {
        // Create two sets of key pairs
        let (master_signing_key, _) = create_keypair().unwrap();
        let (_, wrong_master_verifying_key) = create_keypair().unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (delegate_certificate, delegate_signing_key) = DelegateCertificate::new(&master_signing_key, &info).unwrap();

        // Create a ghostkey certificate
        let (ghostkey_certificate, _ghostkey_signing_key) = GhostkeyCertificate::new(delegate_certificate, &delegate_signing_key);

        // Try to verify with the wrong master key
        let result = ghostkey_certificate.verify(&Some(wrong_master_verifying_key));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().as_ref(),
                         GhostkeyError::SignatureVerificationError(_)));
    }

    #[test]
    fn test_ghostkey_certificate_tampered_delegate() {
        // Create a master key pair
        let (master_signing_key, master_verifying_key) = create_keypair().unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (delegate_certificate, delegate_signing_key) = DelegateCertificate::new(&master_signing_key, &info).unwrap();

        // Create a ghostkey certificate
        let (mut ghostkey_certificate, _ghostkey_signing_key) = GhostkeyCertificate::new(delegate_certificate, &delegate_signing_key);

        // Tamper with the delegate certificate
        ghostkey_certificate.delegate.payload.info = "Tampered Info".to_string();

        // Try to verify the tampered certificate
        let result = ghostkey_certificate.verify(&Some(master_verifying_key));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().as_ref(),
                         GhostkeyError::SignatureVerificationError(_)));
    }

    #[test]
    fn test_ghostkey_certificate_tampered_ghostkey() {
        // Create a master key pair
        let (master_signing_key, master_verifying_key) = create_keypair().unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (delegate_certificate, delegate_signing_key) = DelegateCertificate::new(&master_signing_key, &info).unwrap();

        // Create a ghostkey certificate
        let (mut ghostkey_certificate, _ghostkey_signing_key) = GhostkeyCertificate::new(delegate_certificate, &delegate_signing_key);

        // Tamper with the ghostkey verifying key
        let (_, tampered_verifying_key) = create_keypair().unwrap();
        ghostkey_certificate.verifying_key = SerializableVerifyingKey::from(tampered_verifying_key);

        // Try to verify the tampered certificate
        let result = ghostkey_certificate.verify(&Some(master_verifying_key));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().as_ref(),
                         GhostkeyError::SignatureVerificationError(_)));
    }
}

impl GhostkeyCertificate {
    pub fn new(delegate_certificate: DelegateCertificate, delegate_signing_key: &SigningKey) -> (Self, SigningKey) {
        let (ghost_signing_key, ghost_verifying_key) = create_keypair().unwrap();
        let ghost_signing_key = SerializableSigningKey::from(ghost_signing_key);
        let ghost_verifying_key = SerializableVerifyingKey::from(ghost_verifying_key);
        
        (Self {
            delegate: delegate_certificate,
            verifying_key: ghost_verifying_key.clone(),
            signature: SerializableSignature::from(sign_with_hash(&delegate_signing_key, &ghost_verifying_key).unwrap()),
        }, ghost_signing_key.as_ref().clone())
    }
    
    pub fn verify(&self, master_verifying_key: &Option<VerifyingKey>) -> Result<String, Box<GhostkeyError>> {
        // Verify delegate certificate
        let info = self.delegate.verify(master_verifying_key.as_ref().unwrap())
            .map_err(|e| SignatureVerificationError(format!("Failed to verify delegate: {}", e)))?;
        
        // Verify ghostkey certificate
        let verification = verify_with_hash(self.delegate.payload.delegate_verifying_key.as_ref(), &self.verifying_key, self.signature.as_ref())
            .map_err(|e| SignatureVerificationError(format!("Failed to verify ghostkey: {}", e)))?;
        
        if verification {
            Ok(info)
        } else {
            Err(Box::new(SignatureVerificationError("Failed to verify ghostkey certificate".to_string())))
        }
    }
}