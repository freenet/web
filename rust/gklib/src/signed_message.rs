use ed25519_dalek::Signature;
use serde::{Deserialize, Serialize};

use crate::ghost_key_certificate::GhostkeyCertificateV1;

/// A message signed with a ghostkey.
///
/// Contains the full certificate chain for verification, the original message,
/// and the Ed25519 signature.
#[derive(Serialize, Deserialize, Clone)]
pub struct SignedMessage {
    /// The ghostkey certificate (includes delegate and can be verified back to master)
    pub certificate: GhostkeyCertificateV1,
    /// The original message bytes
    pub message: Vec<u8>,
    /// Ed25519 signature over the message
    pub signature: Signature,
}
