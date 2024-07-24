use serde::{Deserialize, Serialize};
use crate::wrappers::signature::SerializableSignature;

#[derive(Serialize, Deserialize)]
pub struct DelegateCertificate {
    pub payload : DelegatePayload,
    pub signature : SerializableSignature,
}

#[derive(Serialize, Deserialize)]
pub struct DelegatePayload {
    
}