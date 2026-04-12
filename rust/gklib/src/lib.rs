pub mod armorable;
/// Deprecated module path. Use [`notary_certificate`] instead. Will be removed in a future release.
#[allow(deprecated)]
pub mod delegate_certificate;
pub mod errors;
pub mod ghost_key_certificate;
pub mod notary_certificate;
pub mod signed_message;
pub mod util;

pub const FREENET_MASTER_VERIFYING_KEY_BASE64: &str =
    "WCBinZei3Yki9ezxKPNLoCar/m6F3Q8nnSrWDaRSxLL6cw==";
