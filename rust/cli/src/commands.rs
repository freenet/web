use std::path::Path;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use log::{info, error};
use p256::ecdsa::{SigningKey, VerifyingKey};
use crate::util::create_keypair;
use crate::wrappers::signing_key::SerializableSigningKey;
use crate::wrappers::verifying_key::SerializableVerifyingKey;
use crate::armorable::*;
use crate::delegate_certificate::DelegateCertificate;
use crate::errors::GhostkeyError;
use colored::Colorize;

pub fn generate_master_key_cmd(output_dir: &Path, ignore_permissions: bool) -> i32 {
    let (signing_key, verifying_key) = match create_keypair() {
        Ok(keypair) => keypair,
        Err(e) => {
            error!("{} {}", "Failed to create keypair:".red(), e);
            return 1;
        }
    };
    let signing_key : SerializableSigningKey = signing_key.into();
    let verifying_key : SerializableVerifyingKey = verifying_key.into();
    let signing_key_file = output_dir.join("master_signing_key.pem");
    let verifying_key_file = output_dir.join("master_verifying_key.pem");
    info!("Writing master signing key to {}", signing_key_file.display());
    if let Err(e) = signing_key.to_file(&signing_key_file) {
        error!("{} {}", "Failed to write master signing key:".red(), e);
        return 1;
    }
    info!("Writing master verifying key to {}", verifying_key_file.display());
    if let Err(e) = verifying_key.to_file(&verifying_key_file) {
        error!("{} {}", "Failed to write master verifying key:".red(), e);
        return 1;
    }
    if !ignore_permissions {
        if let Err(e) = require_strict_permissions(&signing_key_file) {
            error!("{} {}", "Failed to set permissions on master signing key file:".red(), e);
            return 1;
        }
    } else {
        info!("Ignoring permission checks for {}", signing_key_file.display());
    }
    0
}

pub fn generate_delegate_cmd(
    master_signing_key : &SigningKey,
    info : &String,
    output_dir : &Path,
    ignore_permissions : bool
) -> i32 {
    let (delegate_certificate, delegate_signing_key) = match DelegateCertificate::new(&master_signing_key, &info) {
        Ok(result) => result,
        Err(e) => {
            error!("{} {}", "Failed to create delegate certificate:".red(), e);
            return 1;
        }
    };
    let delegate_signing_key : SerializableSigningKey = delegate_signing_key.into();
    let delegate_certificate_file = output_dir.join("delegate_certificate.pem");
    let delegate_signing_key_file = output_dir.join("delegate_signing_key.pem");
    info!("Writing delegate certificate to {}", delegate_certificate_file.display());
    if let Err(e) = delegate_certificate.to_file(&delegate_certificate_file) {
        error!("{} {}", "Failed to write delegate certificate:".red(), e);
        return 1;
    }
    info!("Writing delegate signing key to {}", delegate_signing_key_file.display());
    if let Err(e) = delegate_signing_key.to_file(&delegate_signing_key_file) {
        error!("{} {}", "Failed to write delegate signing key:".red(), e);
        return 1;
    }
    if !ignore_permissions {
        if let Err(e) = require_strict_permissions(&delegate_signing_key_file) {
            error!("{} {}", "Failed to set permissions on delegate signing key file:".red(), e);
            return 1;
        }
    } else {
        info!("Ignoring permission checks for {}", delegate_certificate_file.display());
        info!("Ignoring permission checks for {}", delegate_signing_key_file.display());
    }
    0
}

pub fn verify_delegate_cmd(master_verifying_key: &VerifyingKey, delegate_certificate: &DelegateCertificate) -> i32 {
    match delegate_certificate.verify(master_verifying_key) {
        Ok(info) => {
            info!("Delegate certificate verified. Info: {}", info);
            0
        },
        Err(e) => {
            error!("{} {}", "Failed to verify delegate certificate:".red(), e);
            1
        }
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
            file_path.display(), file_path.display()
        ).into());
    }
    Ok(())
}
