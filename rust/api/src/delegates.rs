//! Notary key lookup for the donation flow.
//!
//! This module was historically named "delegates" because the intermediate
//! PKI signing key was called a delegate. It was renamed to "notary" in 0.1.5
//! (issue freenet/web#24) to deconflict with Freenet's own `Delegate` (WASM
//! agent) concept. The module filename and the `DELEGATE_DIR` env var are
//! kept for backward compatibility with existing deployments; `NOTARY_DIR`
//! is the canonical name going forward.

use std::path::{Path, PathBuf};

use blind_rsa_signatures::{BlindedMessage, BlindSignature, Options, SecretKey as RSASigningKey};
use rand_core::OsRng;

use ghostkey_lib::armorable::*;
use ghostkey_lib::notary_certificate::NotaryCertificateV1;

use crate::handle_sign_cert::CertificateError;

/// Resolve the notary directory from env, preferring `NOTARY_DIR` and
/// falling back to the legacy `DELEGATE_DIR` with a deprecation warning.
fn notary_dir() -> Result<PathBuf, CertificateError> {
    if let Ok(dir) = std::env::var("NOTARY_DIR") {
        return Ok(PathBuf::from(dir));
    }
    if let Ok(dir) = std::env::var("DELEGATE_DIR") {
        log::warn!(
            "reading legacy DELEGATE_DIR env var; rename to NOTARY_DIR — \
             DELEGATE_DIR will be removed in 0.2.0 (freenet/web#24)"
        );
        return Ok(PathBuf::from(dir));
    }
    log::error!("neither NOTARY_DIR nor DELEGATE_DIR is set");
    Err(CertificateError::KeyError(
        "NOTARY_DIR environment variable not set".to_string(),
    ))
}

/// Resolve a per-amount notary file, preferring the canonical
/// `notary_{kind}_{amount}.pem` name and falling back to the legacy
/// `delegate_{kind}_{amount}.pem` name with a deprecation warning.
fn resolve_amount_file(dir: &Path, kind: &str, amount: u64) -> PathBuf {
    let new_name = format!("notary_{}_{}.pem", kind, amount);
    let new_path = dir.join(&new_name);
    if new_path.exists() {
        return new_path;
    }
    let old_name = format!("delegate_{}_{}.pem", kind, amount);
    let old_path = dir.join(&old_name);
    if old_path.exists() {
        log::warn!(
            "reading legacy per-amount file {}; rename to {} — \
             old filenames will be removed in 0.2.0 (freenet/web#24)",
            old_path.display(),
            new_name
        );
        return old_path;
    }
    // Fall through with the canonical path so the caller gets a clean
    // "not found" error referencing the new name.
    new_path
}

pub(crate) fn get_notary(amount: u64) -> Result<(NotaryCertificateV1, RSASigningKey), CertificateError> {
    let dir = notary_dir()?;
    let cert_path = resolve_amount_file(&dir, "certificate", amount);
    let cert = NotaryCertificateV1::from_file(&cert_path).map_err(|e| {
        CertificateError::KeyError(format!(
            "Unable to read notary certificate from {}: {}",
            cert_path.display(),
            e
        ))
    })?;

    let signing_key_path = resolve_amount_file(&dir, "signing_key", amount);
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
