use ed25519_dalek::VerifyingKey;
use armorable::*;

pub mod armorable;
pub mod delegate_certificate;
pub mod ghost_key_certificate;
pub mod errors;
pub mod util;

pub const FREENET_MASTER_VERIFYING_KEY: &VerifyingKey = Armorable::from_base64("WCBinZei3Yki9ezxKPNLoCar/m6F3Q8nnSrWDaRSxLL6cw==").unwrap();
