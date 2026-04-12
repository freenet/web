//! Deprecated module path — renamed to [`crate::notary_certificate`] in 0.1.5.
//!
//! See issue freenet/web#24 for the rename rationale. This stub preserves the
//! public path `ghostkey_lib::delegate_certificate::DelegateCertificateV1` for
//! one release so downstream code compiles with a deprecation warning. Plan is
//! to remove this module in 0.2.0.

#[deprecated(
    since = "0.1.5",
    note = "use `ghostkey_lib::notary_certificate::NotaryCertificateV1`"
)]
pub use crate::notary_certificate::NotaryCertificateV1 as DelegateCertificateV1;

#[deprecated(
    since = "0.1.5",
    note = "use `ghostkey_lib::notary_certificate::NotaryPayload`"
)]
pub use crate::notary_certificate::NotaryPayload as DelegatePayload;
