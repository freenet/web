use super::errors::GhostkeyError;
use super::util::{sign_with_hash, verify_with_hash};
use blind_rsa_signatures::{
    KeyPair as RSAKeyPair, PublicKey as RSAVerifyingKey, SecretKey as RSASigningKey,
};
use ed25519_dalek::*;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use crate::armorable::Armorable;
use crate::FREENET_MASTER_VERIFYING_KEY_BASE64;

#[derive(Serialize, Deserialize, Clone)]
pub struct DelegateCertificateV1 {
    pub payload: DelegatePayload,
    /// The payload signed by the master signing key
    pub signature: Signature,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DelegatePayload {
    pub delegate_verifying_key: RSAVerifyingKey,
    pub info: String,
}

impl DelegateCertificateV1 {
    pub fn new(
        master_signing_key: &SigningKey,
        info: &String,
    ) -> Result<(Self, RSASigningKey), Box<GhostkeyError>> {
        let delegate_keypair = RSAKeyPair::generate(&mut OsRng, 2048)
            .map_err(|e| GhostkeyError::RSAError(e.to_string()))?;

        let payload = DelegatePayload {
            delegate_verifying_key: delegate_keypair.pk,
            info: info.clone(),
        };

        let signature = sign_with_hash(&master_signing_key, &payload)?;

        let certificate = DelegateCertificateV1 {
            payload,
            signature: Signature::from(signature),
        };

        Ok((certificate, delegate_keypair.sk))
    }

    /// Verifies the delegate certificate using the master verifying key. If the verification is
    /// successful, the info field of the payload is returned. Uses Freenet master verifying key
    /// if no key is provided.
    pub fn verify(
        &self,
        &master_verifying_key: &Option<VerifyingKey>,
    ) -> Result<String, Box<GhostkeyError>> {
        let master_verifying_key = master_verifying_key.unwrap_or(VerifyingKey::from_base64(FREENET_MASTER_VERIFYING_KEY_BASE64).unwrap());
        
        let verification = verify_with_hash(&master_verifying_key, &self.payload, &self.signature)?;
        if verification {
            Ok(self.payload.info.clone())
        } else {
            Err(Box::new(GhostkeyError::SignatureVerificationError(
                "Failed to verify delegate certificate".to_string(),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::create_keypair;
    use super::*;

    #[test]
    fn test_delegate_certificate_creation_and_verification() {
        // Create a master key pair
        let (master_signing_key, master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (certificate, _delegate_signing_key) =
            DelegateCertificateV1::new(&master_signing_key, &info).unwrap();

        // Verify the certificate
        let verified_info = certificate.verify(&Some(master_verifying_key)).unwrap();
        assert_eq!(verified_info, info);
    }

    #[test]
    fn test_delegate_certificate_invalid_signature() {
        // Create two sets of key pairs
        let (master_signing_key, _) = create_keypair(&mut OsRng).unwrap();
        let (_, wrong_verifying_key) = create_keypair(&mut OsRng).unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (certificate, _delegate_signing_key) =
            DelegateCertificateV1::new(&master_signing_key, &info).unwrap();

        // Try to verify with the wrong key
        let result = certificate.verify(&Some(wrong_verifying_key));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().as_ref(),
            GhostkeyError::SignatureVerificationError(_)
        ));
    }

    #[test]
    fn test_delegate_certificate_payload_integrity() {
        // Create a master key pair
        let (master_signing_key, master_verifying_key) = create_keypair(&mut OsRng).unwrap();

        // Create a delegate certificate
        let info = "Test Delegate".to_string();
        let (mut certificate, _delegate_signing_key) =
            DelegateCertificateV1::new(&master_signing_key, &info).unwrap();

        // Verify the original certificate
        assert!(certificate.verify(&Some(master_verifying_key)).is_ok());

        // Tamper with the payload
        certificate.payload.info = "Tampered Info".to_string();

        // Verify the tampered certificate
        let result = certificate.verify(&Some(master_verifying_key));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().as_ref(),
            GhostkeyError::SignatureVerificationError(_)
        ));
    }
}
