use serde::{Deserialize, Serialize};
use ghostkey_lib::ghost_key_certificate::GhostkeyCertificateV1;
use ed25519_dalek::Signature;

#[derive(Serialize, Deserialize)]
pub struct SignedMessage {
    pub certificate: GhostkeyCertificateV1,
    pub message: Vec<u8>,
    pub signature: Signature,
}
