use p256::ecdsa::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use crate::errors::GhostkeyError;
use crate::util::create_keypair;
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
    pub fn new(master_signing_key : SigningKey, info : String) -> Result<(Self, SigningKey), Box<GhostkeyError>> {
        let (delegate_signing_key, delegate_verifying_key) = create_keypair().unwrap();
        
        let payload = DelegatePayload {
            delegate_verifying_key : SerializableVerifyingKey::from(delegate_verifying_key),
            info,
        };
        
        todo!()
        
    }
}