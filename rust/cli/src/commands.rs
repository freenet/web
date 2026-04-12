use blind_rsa_signatures::SecretKey as RSASigningKey;
use colored::Colorize;
use ed25519_dalek::*;
use ghostkey_lib::armorable::*;
use ghostkey_lib::errors::GhostkeyError;
use ghostkey_lib::ghost_key_certificate::GhostkeyCertificateV1;
use ghostkey_lib::notary_certificate::NotaryCertificateV1;
use ghostkey_lib::signed_message::SignedMessage;
use ghostkey_lib::util::create_keypair;
use log::info;
use rand_core::OsRng;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Canonical on-disk filenames for the notary certificate and signing key.
pub const NOTARY_CERT_FILENAME: &str = "notary_certificate.pem";
pub const NOTARY_SIGNING_KEY_FILENAME: &str = "notary_signing_key.pem";

/// Legacy (pre-0.2.0) on-disk filenames, still accepted for reads.
pub const LEGACY_DELEGATE_CERT_FILENAME: &str = "delegate_certificate.pem";
pub const LEGACY_DELEGATE_SIGNING_KEY_FILENAME: &str = "delegate_signing_key.pem";

/// Resolve a notary file from a directory, accepting the legacy filename as a
/// fallback with a deprecation warning. Returns the path that exists.
///
/// Note that this resolves cert and signing-key files independently. In the
/// CLI that's fine because `generate-notary` always writes both with the
/// same scheme in the same call, so partial migrations aren't created by
/// this code path. The API server (which DOES face partial-migration risk
/// across per-amount files) uses a different directory-level resolver in
/// `rust/api/src/delegates.rs::pick_scheme`.
pub fn resolve_notary_file(dir: &Path, canonical: &str, legacy: &str) -> PathBuf {
    let new_path = dir.join(canonical);
    if new_path.exists() {
        return new_path;
    }
    let old_path = dir.join(legacy);
    if old_path.exists() {
        eprintln!(
            "{}: reading legacy file {}. Rename to {} — the old name will \
             be removed in a future release. See freenet/web#24.",
            "warning".yellow(),
            old_path.display(),
            canonical,
        );
        return old_path;
    }
    // Return the canonical path so the caller sees a clean "not found" error.
    new_path
}

pub fn generate_master_key_cmd(output_dir: &Path, ignore_permissions: bool) -> i32 {
    let (signing_key, verifying_key) = match create_keypair(&mut OsRng) {
        Ok(keypair) => keypair,
        Err(e) => {
            eprintln!("{} to create keypair: {}", "Failed".red(), e);
            return 1;
        }
    };
    let signing_key: SigningKey = signing_key.into();
    let verifying_key: VerifyingKey = verifying_key.into();
    let signing_key_file = output_dir.join("master_signing_key.pem");
    let verifying_key_file = output_dir.join("master_verifying_key.pem");
    info!(
        "Writing master signing key to {}",
        signing_key_file.display()
    );
    if let Err(e) = signing_key.to_file(&signing_key_file) {
        eprintln!("{} to write master signing key: {}", "Failed".red(), e);
        return 1;
    }
    if let Err(e) = fs::set_permissions(&signing_key_file, fs::Permissions::from_mode(0o600)) {
        eprintln!(
            "{} to set permissions on master signing key file: {}",
            "Failed".red(),
            e
        );
        return 1;
    }
    println!(
        "{} written {}: {}",
        "Master signing key",
        "successfully".green(),
        signing_key_file.display().to_string().yellow()
    );
    info!(
        "Writing master verifying key to {}",
        verifying_key_file.display()
    );
    if let Err(e) = verifying_key.to_file(&verifying_key_file) {
        eprintln!("{} to write master verifying key: {}", "Failed".red(), e);
        return 1;
    }
    println!(
        "{} written {}: {}",
        "Master verifying key",
        "successfully".green(),
        verifying_key_file.display().to_string().yellow()
    );
    if !ignore_permissions {
        if let Err(e) = require_strict_permissions(&signing_key_file) {
            eprintln!(
                "{} to set permissions on master signing key file: {}",
                "Failed".red(),
                e
            );
            return 1;
        }
    } else {
        info!(
            "Ignoring permission checks for {}",
            signing_key_file.display()
        );
    }
    0
}

pub fn generate_notary_cmd(
    master_signing_key: &SigningKey,
    info: &String,
    output_dir: &Path,
    ignore_permissions: bool,
) -> i32 {
    let (notary_certificate, notary_signing_key) =
        match NotaryCertificateV1::new(&master_signing_key, &info) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("{} to create notary certificate: {}", "Failed".red(), e);
                return 1;
            }
        };
    let notary_certificate_file = output_dir.join(NOTARY_CERT_FILENAME);
    let notary_signing_key_file = output_dir.join(NOTARY_SIGNING_KEY_FILENAME);
    info!(
        "Writing notary certificate to {}",
        notary_certificate_file.display()
    );
    if let Err(e) = notary_certificate.to_file(&notary_certificate_file) {
        eprintln!("{} to write notary certificate: {}", "Failed".red(), e);
        return 1;
    }
    println!(
        "{} written {}: {}",
        "Notary certificate",
        "successfully".green(),
        notary_certificate_file.display().to_string().yellow()
    );
    info!(
        "Writing notary signing key to {}",
        notary_signing_key_file.display()
    );
    if let Err(e) = notary_signing_key.to_file(&notary_signing_key_file) {
        eprintln!("{} to write notary signing key: {}", "Failed".red(), e);
        return 1;
    }
    if let Err(e) = fs::set_permissions(&notary_signing_key_file, fs::Permissions::from_mode(0o600))
    {
        eprintln!(
            "{} to set permissions on notary signing key file: {}",
            "Failed".red(),
            e
        );
        return 1;
    }
    println!(
        "{} written {}: {}",
        "Notary signing key",
        "successfully".green(),
        notary_signing_key_file.display().to_string().yellow()
    );
    if !ignore_permissions {
        if let Err(e) = require_strict_permissions(&notary_signing_key_file) {
            eprintln!(
                "{} to set permissions on notary signing key file: {}",
                "Failed".red(),
                e
            );
            return 1;
        }
    } else {
        info!(
            "Ignoring permission checks for {}",
            notary_certificate_file.display()
        );
        info!(
            "Ignoring permission checks for {}",
            notary_signing_key_file.display()
        );
    }
    0
}

pub fn verify_notary_cmd(
    master_verifying_key: &Option<VerifyingKey>,
    notary_certificate: &NotaryCertificateV1,
) -> i32 {
    match notary_certificate.verify(master_verifying_key) {
        Ok(info) => {
            println!("Notary certificate {}", "verified".green());
            println!("Info: {}", info.blue());
            0
        }
        Err(e) => {
            eprintln!("{} to verify notary certificate: {}", "Failed".red(), e);
            1
        }
    }
}

pub fn sign_message_cmd(
    ghost_certificate: GhostkeyCertificateV1,
    ghost_signing_key: &SigningKey,
    message: &[u8],
    output_file: &Path,
) -> i32 {
    if ghost_signing_key.verifying_key() != ghost_certificate.verifying_key {
        eprintln!(
            "{}: Ghost signing key does not match ghost verifying key",
            "Error".red()
        );
        return 1;
    }

    let signature = ghost_signing_key.sign(message);
    let signed_message = SignedMessage {
        certificate: ghost_certificate,
        message: message.to_vec(),
        signature,
    };

    match signed_message.to_file(output_file) {
        Ok(_) => {
            println!("{} written {}", "Signed message", "successfully".green());
            0
        }
        Err(e) => {
            eprintln!("{} to write signed message: {}", "Failed".red(), e);
            1
        }
    }
}

pub fn verify_signed_message_cmd(
    signed_message_file: &Path,
    master_verifying_key: &Option<VerifyingKey>,
    output_file: Option<&Path>,
) -> i32 {
    let signed_message = match SignedMessage::from_file(signed_message_file) {
        Ok(sm) => sm,
        Err(e) => {
            eprintln!("{} to read signed message: {}", "Failed".red(), e);
            return 1;
        }
    };

    match signed_message.certificate.verify(master_verifying_key) {
        Ok(info) => {
            println!("Ghost certificate {}", "verified".green());
            println!("Info: {}", info.blue());

            let verifying_key = signed_message.certificate.verifying_key;
            match verifying_key.verify(&signed_message.message, &signed_message.signature) {
                Ok(_) => {
                    println!("Signature {}", "verified".green());
                    match output_file {
                        Some(file) => {
                            if let Err(e) = fs::write(file, &signed_message.message) {
                                eprintln!("{} to write message to file: {}", "Failed".red(), e);
                                return 1;
                            }
                            println!("Message written to {}", file.display());
                        }
                        None => {
                            println!(
                                "Message: {}",
                                String::from_utf8_lossy(&signed_message.message)
                            );
                        }
                    }
                    0
                }
                Err(e) => {
                    eprintln!("{} to verify signature: {}", "Failed".red(), e);
                    1
                }
            }
        }
        Err(e) => {
            eprintln!("{} to verify ghost certificate: {}", "Failed".red(), e);
            1
        }
    }
}

pub fn generate_ghost_key_cmd(
    notary_certificate: &NotaryCertificateV1,
    notary_signing_key: &RSASigningKey,
    output_dir: &Path,
) -> i32 {
    if notary_signing_key.public_key().unwrap() != notary_certificate.payload.notary_verifying_key {
        eprintln!(
            "{}: Notary signing key does not match notary verifying key",
            "Error".red()
        );
        return 1;
    }

    let (ghost_key_certificate, ghost_key_signing_key) =
        GhostkeyCertificateV1::new(notary_certificate, notary_signing_key);
    let ghost_key_certificate_file = output_dir.join("ghost_key_certificate.pem");
    let ghost_key_signing_key_file = output_dir.join("ghost_key_signing_key.pem");
    info!(
        "Writing ghostkey certificate to {}",
        ghost_key_certificate_file.display()
    );
    if let Err(e) = ghost_key_certificate.to_file(&ghost_key_certificate_file) {
        eprintln!("{} to write ghostkey certificate: {}", "Failed".red(), e);
        return 1;
    }
    println!(
        "{} written {}: {}",
        "Ghost Key certificate",
        "successfully".green(),
        ghost_key_certificate_file.display().to_string().yellow()
    );
    info!(
        "Writing ghostkey signing key to {}",
        ghost_key_signing_key_file.display()
    );
    if let Err(e) = ghost_key_signing_key.to_file(&ghost_key_signing_key_file) {
        eprintln!("{} to write ghostkey signing key: {}", "Failed".red(), e);
        return 1;
    }
    if let Err(e) = fs::set_permissions(
        &ghost_key_signing_key_file,
        fs::Permissions::from_mode(0o600),
    ) {
        eprintln!(
            "{} to set permissions on ghostkey signing key file: {}",
            "Failed".red(),
            e
        );
        return 1;
    }
    println!(
        "{} written {}: {}",
        "Ghost signing key",
        "successfully".green(),
        ghost_key_signing_key_file.display().to_string().yellow()
    );
    0
}

pub fn verify_ghost_key_cmd(
    master_verifying_key: &Option<VerifyingKey>,
    ghost_certificate: &GhostkeyCertificateV1,
) -> i32 {
    match ghost_certificate.verify(&master_verifying_key.clone()) {
        Ok(info) => {
            println!("Ghost certificate {}", "verified".green());
            println!("Info: {}", info.blue());
            0
        }
        Err(e) => {
            eprintln!("{} to verify ghost certificate: {}", "Failed".red(), e);
            1
        }
    }
}

#[cfg(test)]
mod resolve_tests {
    use super::*;
    use tempfile::tempdir;

    fn touch(path: &Path) {
        std::fs::write(path, b"placeholder").unwrap();
    }

    #[test]
    fn resolve_prefers_canonical_when_both_exist() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join(NOTARY_CERT_FILENAME));
        touch(&dir.path().join(LEGACY_DELEGATE_CERT_FILENAME));
        let resolved = resolve_notary_file(
            dir.path(),
            NOTARY_CERT_FILENAME,
            LEGACY_DELEGATE_CERT_FILENAME,
        );
        assert_eq!(resolved, dir.path().join(NOTARY_CERT_FILENAME));
    }

    #[test]
    fn resolve_falls_back_to_legacy_filename() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join(LEGACY_DELEGATE_CERT_FILENAME));
        let resolved = resolve_notary_file(
            dir.path(),
            NOTARY_CERT_FILENAME,
            LEGACY_DELEGATE_CERT_FILENAME,
        );
        assert_eq!(resolved, dir.path().join(LEGACY_DELEGATE_CERT_FILENAME));
    }

    #[test]
    fn resolve_returns_canonical_on_not_found() {
        // Nothing exists — return the canonical path so the downstream
        // open() error references the new name, not the legacy one.
        let dir = tempdir().unwrap();
        let resolved = resolve_notary_file(
            dir.path(),
            NOTARY_CERT_FILENAME,
            LEGACY_DELEGATE_CERT_FILENAME,
        );
        assert_eq!(resolved, dir.path().join(NOTARY_CERT_FILENAME));
    }

    #[test]
    fn resolve_handles_signing_key_filename() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join(LEGACY_DELEGATE_SIGNING_KEY_FILENAME));
        let resolved = resolve_notary_file(
            dir.path(),
            NOTARY_SIGNING_KEY_FILENAME,
            LEGACY_DELEGATE_SIGNING_KEY_FILENAME,
        );
        assert_eq!(
            resolved,
            dir.path().join(LEGACY_DELEGATE_SIGNING_KEY_FILENAME)
        );
    }
}

fn require_strict_permissions(file_path: &Path) -> Result<(), GhostkeyError> {
    let metadata = fs::metadata(file_path).map_err(|e| GhostkeyError::IOError(e.to_string()))?;
    let permissions = metadata.permissions();
    let mode = permissions.mode();

    if mode & 0o077 != 0 {
        return Err(format!(
            "The file '{}' has incorrect permissions. \
        It should not be readable or writable by group or others. \
        Use \"chmod 600 {}\" to set the correct permissions.",
            file_path.display(),
            file_path.display()
        )
        .into());
    }
    Ok(())
}
