use p256::ecdsa::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
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
        let verification = verify_with_hash(&self.verifying_key.as_ref(), &info, self.signature.as_ref())
            .map_err(|e| SignatureVerificationError(format!("Failed to verify ghostkey: {}", e)))?;
        
        if verification {
            Ok(info)
        } else {
            Err(Box::new(SignatureVerificationError("Failed to verify ghostkey certificate".to_string())))
        }
    }
}