use p256::ecdsa::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use crate::errors::GhostkeyError;
use crate::util::{create_keypair, sign_with_hash, verify_with_hash};
use crate::wrappers::signature::SerializableSignature;
use crate::wrappers::verifying_key::SerializableVerifyingKey;

#[derive(Serialize, Deserialize)]
pub struct DelegateCertificate {
    pub payload : DelegatePayload,
    /// The payload signed by the master signing key
    pub signature : SerializableSignature,
}

#[derive(Serialize, Deserialize)]
pub struct DelegatePayload {
    pub delegate_verifying_key : SerializableVerifyingKey,
    pub info : String,
}

impl DelegateCertificate {
    pub fn new(_master_signing_key: &SigningKey, info: &String) -> Result<(Self, SigningKey), Box<GhostkeyError>> {
        let (delegate_signing_key, delegate_verifying_key) = create_keypair()?;
        
        let payload = DelegatePayload {
            delegate_verifying_key: SerializableVerifyingKey::from(delegate_verifying_key),
            info: info.clone(),
        };
        
        let signature = sign_with_hash(&_master_signing_key, &payload)?;
        
        let certificate = DelegateCertificate {
            payload,
            signature: SerializableSignature::from(signature),
        };
        
        Ok((certificate, delegate_signing_key))
    }
    
    /// Verifies the delegate certificate using the master verifying key. If the verification is 
    /// successful, the info field of the payload is returned.
    pub fn verify(&self, &master_verifying_key: &VerifyingKey) -> Result<String, Box<GhostkeyError>> {
        let verification = verify_with_hash(&master_verifying_key, &self.payload, self.signature.as_ref())?;
        if verification {
            Ok(self.payload.info.clone())
        } else {
            Err(Box::new(GhostkeyError::SignatureVerificationError("Failed to verify delegate certificate".to_string())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::create_keypair;

    #[test]
    fn test_delegate_certificate_creation_and_verification() {
        // Create a master key pair
        let (master_signing_key, master_verifying_key) = create_keypair().unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (certificate, _delegate_signing_key) = DelegateCertificate::new(&master_signing_key, &info).unwrap();

        // Verify the certificate
        let verified_info = certificate.verify(&master_verifying_key).unwrap();
        assert_eq!(verified_info, info);
    }

    #[test]
    fn test_delegate_certificate_invalid_signature() {
        // Create two sets of key pairs
        let (master_signing_key, _) = create_keypair().unwrap();
        let (_, wrong_verifying_key) = create_keypair().unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (certificate, _delegate_signing_key) = DelegateCertificate::new(&master_signing_key, &info).unwrap();

        // Try to verify with the wrong key
        let result = certificate.verify(&wrong_verifying_key);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().as_ref(),
                         GhostkeyError::SignatureVerificationError(_)));
    }

    #[test]
    fn test_delegate_certificate_payload_integrity() {
        // Create a master key pair
        let (master_signing_key, master_verifying_key) = create_keypair().unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (mut certificate, _delegate_signing_key) = DelegateCertificate::new(&master_signing_key, &info).unwrap();

        // Verify the original certificate
        assert!(certificate.verify(&master_verifying_key).is_ok());

        // Tamper with the payload
        certificate.payload.info = "Tampered Info".to_string();

        // Verify the tampered certificate
        let result = certificate.verify(&master_verifying_key);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().as_ref(),
                         GhostkeyError::SignatureVerificationError(_)));
    }
}