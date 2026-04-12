//! Notary key lookup for the donation flow.
//!
//! This module was historically named "delegates" because the intermediate
//! PKI signing key was called a delegate. It was renamed to "notary" in 0.2.0
//! (issue freenet/web#24) to deconflict with Freenet's own `Delegate` (WASM
//! agent) concept. The module filename and the `DELEGATE_DIR` env var are
//! kept for backward compatibility with existing deployments; `NOTARY_DIR`
//! is the canonical name going forward.

use std::path::{Path, PathBuf};

use blind_rsa_signatures::{BlindSignature, BlindedMessage, Options, SecretKey as RSASigningKey};
use rand_core::OsRng;

use ghostkey_lib::armorable::*;
use ghostkey_lib::notary_certificate::NotaryCertificateV1;

use crate::handle_sign_cert::CertificateError;

/// Which naming scheme the per-amount files on disk use.
///
/// Resolved once per directory, not per file, so a partial migration on disk
/// cannot pair a new-named certificate with a legacy-named signing key. That
/// mismatch would produce ghost-key certificates whose signatures don't chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NamingScheme {
    /// `notary_certificate_{amount}.pem` / `notary_signing_key_{amount}.pem`
    Notary,
    /// `delegate_certificate_{amount}.pem` / `delegate_signing_key_{amount}.pem`
    LegacyDelegate,
}

impl NamingScheme {
    fn cert_filename(&self, amount: u64) -> String {
        match self {
            Self::Notary => format!("notary_certificate_{}.pem", amount),
            Self::LegacyDelegate => format!("delegate_certificate_{}.pem", amount),
        }
    }

    fn signing_key_filename(&self, amount: u64) -> String {
        match self {
            Self::Notary => format!("notary_signing_key_{}.pem", amount),
            Self::LegacyDelegate => format!("delegate_signing_key_{}.pem", amount),
        }
    }
}

/// Resolve the notary directory from env, preferring `NOTARY_DIR` and
/// falling back to the legacy `DELEGATE_DIR` with a deprecation warning.
fn notary_dir() -> Result<PathBuf, CertificateError> {
    if let Ok(dir) = std::env::var("NOTARY_DIR") {
        return Ok(PathBuf::from(dir));
    }
    if let Ok(dir) = std::env::var("DELEGATE_DIR") {
        log::warn!(
            "reading legacy DELEGATE_DIR env var; rename to NOTARY_DIR — \
             DELEGATE_DIR will be removed in a future release (freenet/web#24)"
        );
        return Ok(PathBuf::from(dir));
    }
    log::error!("neither NOTARY_DIR nor DELEGATE_DIR is set");
    Err(CertificateError::KeyError(
        "NOTARY_DIR environment variable not set".to_string(),
    ))
}

/// Decide which naming scheme a directory uses for a given amount. Prefers
/// the canonical `notary_*` pair; falls back to the legacy `delegate_*` pair
/// only if BOTH legacy files exist AND neither canonical file does.
///
/// A partial migration where only the cert has been renamed (or only the
/// signing key) resolves to the canonical scheme and the reader surfaces a
/// "not found" error referencing the new name — that's safer than silently
/// pairing a new cert with a legacy signing key, which would issue ghost
/// certs whose signatures don't verify.
fn pick_scheme(dir: &Path, amount: u64) -> NamingScheme {
    let notary_cert = dir.join(NamingScheme::Notary.cert_filename(amount));
    let notary_key = dir.join(NamingScheme::Notary.signing_key_filename(amount));
    if notary_cert.exists() && notary_key.exists() {
        return NamingScheme::Notary;
    }

    let legacy_cert = dir.join(NamingScheme::LegacyDelegate.cert_filename(amount));
    let legacy_key = dir.join(NamingScheme::LegacyDelegate.signing_key_filename(amount));
    if legacy_cert.exists() && legacy_key.exists() {
        log::warn!(
            "reading legacy per-amount files delegate_certificate_{0}.pem / \
             delegate_signing_key_{0}.pem; rename to notary_*_{0}.pem — the \
             legacy filenames will be removed in a future release (freenet/web#24)",
            amount
        );
        return NamingScheme::LegacyDelegate;
    }

    // Neither complete pair exists. Default to canonical so the downstream
    // read surfaces a clean "not found" error referencing the new name.
    NamingScheme::Notary
}

pub(crate) fn get_notary(
    amount: u64,
) -> Result<(NotaryCertificateV1, RSASigningKey), CertificateError> {
    let dir = notary_dir()?;
    let scheme = pick_scheme(&dir, amount);

    let cert_path = dir.join(scheme.cert_filename(amount));
    let cert = NotaryCertificateV1::from_file(&cert_path).map_err(|e| {
        CertificateError::KeyError(format!(
            "Unable to read notary certificate from {}: {}",
            cert_path.display(),
            e
        ))
    })?;

    let signing_key_path = dir.join(scheme.signing_key_filename(amount));
    let signing_key = RSASigningKey::from_file(&signing_key_path).map_err(|e| {
        CertificateError::KeyError(format!(
            "Unable to read notary signing key from {}: {}",
            signing_key_path.display(),
            e
        ))
    })?;
    Ok((cert, signing_key))
}

pub(crate) fn sign_with_notary_key(
    blinded_ghostkey: &BlindedMessage,
    amount_dollars: u64,
) -> Result<BlindSignature, CertificateError> {
    let (_, notary_signing_key) = get_notary(amount_dollars)?;

    let options = Options::default();

    let blind_sig = notary_signing_key
        .blind_sign(&mut OsRng, blinded_ghostkey, &options)
        .map_err(|e| CertificateError::MiscError(format!("Failed to blind sign: {}", e)))?;

    Ok(blind_sig)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn touch(path: &Path) {
        std::fs::write(path, b"placeholder").unwrap();
    }

    #[test]
    fn pick_scheme_prefers_notary_when_both_complete_pairs_exist() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join("notary_certificate_20.pem"));
        touch(&dir.path().join("notary_signing_key_20.pem"));
        touch(&dir.path().join("delegate_certificate_20.pem"));
        touch(&dir.path().join("delegate_signing_key_20.pem"));
        assert_eq!(pick_scheme(dir.path(), 20), NamingScheme::Notary);
    }

    #[test]
    fn pick_scheme_falls_back_to_legacy_when_notary_absent() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join("delegate_certificate_20.pem"));
        touch(&dir.path().join("delegate_signing_key_20.pem"));
        assert_eq!(pick_scheme(dir.path(), 20), NamingScheme::LegacyDelegate);
    }

    #[test]
    fn pick_scheme_refuses_mismatched_partial_notary_pair() {
        // Only the notary cert has been renamed; signing key is still legacy.
        // Must NOT return LegacyDelegate (would pair a notary cert with a
        // legacy-named signing key for the wrong amount or worse). Instead
        // return Notary so the caller surfaces a clean "signing key not
        // found" error against the canonical filename.
        let dir = tempdir().unwrap();
        touch(&dir.path().join("notary_certificate_20.pem"));
        touch(&dir.path().join("delegate_signing_key_20.pem"));
        assert_eq!(pick_scheme(dir.path(), 20), NamingScheme::Notary);
    }

    #[test]
    fn pick_scheme_refuses_mismatched_partial_legacy_pair() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join("delegate_certificate_20.pem"));
        touch(&dir.path().join("notary_signing_key_20.pem"));
        // Neither a complete notary pair nor a complete legacy pair;
        // default to canonical so the failure message points at the new name.
        assert_eq!(pick_scheme(dir.path(), 20), NamingScheme::Notary);
    }

    #[test]
    fn pick_scheme_missing_directory_defaults_to_notary() {
        let dir = tempdir().unwrap();
        assert_eq!(pick_scheme(dir.path(), 20), NamingScheme::Notary);
    }

    #[test]
    fn naming_scheme_filenames_are_exactly_as_documented() {
        assert_eq!(
            NamingScheme::Notary.cert_filename(20),
            "notary_certificate_20.pem"
        );
        assert_eq!(
            NamingScheme::Notary.signing_key_filename(20),
            "notary_signing_key_20.pem"
        );
        assert_eq!(
            NamingScheme::LegacyDelegate.cert_filename(20),
            "delegate_certificate_20.pem"
        );
        assert_eq!(
            NamingScheme::LegacyDelegate.signing_key_filename(20),
            "delegate_signing_key_20.pem"
        );
    }
}
