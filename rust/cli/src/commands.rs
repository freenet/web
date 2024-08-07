use ghostkey_lib::armorable::*;
use ghostkey_lib::delegate_certificate::DelegateCertificate;
use ghostkey_lib::errors::GhostkeyError;
use ghostkey_lib::ghost_key_certificate::GhostkeyCertificate;
use ghostkey_lib::util::create_keypair;
use blind_rsa_signatures::SecretKey as RSASigningKey;
use colored::Colorize;
use ed25519_dalek::*;
use log::info;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use rand_core::OsRng;

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

pub fn generate_delegate_cmd(
    master_signing_key: &SigningKey,
    info: &String,
    output_dir: &Path,
    ignore_permissions: bool,
) -> i32 {
    let (delegate_certificate, delegate_signing_key) =
        match DelegateCertificate::new(&master_signing_key, &info) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("{} to create delegate certificate: {}", "Failed".red(), e);
                return 1;
            }
        };
    let delegate_certificate_file = output_dir.join("delegate_certificate.pem");
    let delegate_signing_key_file = output_dir.join("delegate_signing_key.pem");
    info!(
        "Writing delegate certificate to {}",
        delegate_certificate_file.display()
    );
    if let Err(e) = delegate_certificate.to_file(&delegate_certificate_file) {
        eprintln!("{} to write delegate certificate: {}", "Failed".red(), e);
        return 1;
    }
    println!(
        "{} written {}: {}",
        "Delegate certificate",
        "successfully".green(),
        delegate_certificate_file.display().to_string().yellow()
    );
    info!(
        "Writing delegate signing key to {}",
        delegate_signing_key_file.display()
    );
    if let Err(e) = delegate_signing_key.to_file(&delegate_signing_key_file) {
        eprintln!("{} to write delegate signing key: {}", "Failed".red(), e);
        return 1;
    }
    if let Err(e) = fs::set_permissions(
        &delegate_signing_key_file,
        fs::Permissions::from_mode(0o600),
    ) {
        eprintln!(
            "{} to set permissions on delegate signing key file: {}",
            "Failed".red(),
            e
        );
        return 1;
    }
    println!(
        "{} written {}: {}",
        "Delegate signing key",
        "successfully".green(),
        delegate_signing_key_file.display().to_string().yellow()
    );
    if !ignore_permissions {
        if let Err(e) = require_strict_permissions(&delegate_signing_key_file) {
            eprintln!(
                "{} to set permissions on delegate signing key file: {}",
                "Failed".red(),
                e
            );
            return 1;
        }
    } else {
        info!(
            "Ignoring permission checks for {}",
            delegate_certificate_file.display()
        );
        info!(
            "Ignoring permission checks for {}",
            delegate_signing_key_file.display()
        );
    }
    0
}

pub fn verify_delegate_cmd(
    master_verifying_key: &VerifyingKey,
    delegate_certificate: &DelegateCertificate,
) -> i32 {
    match delegate_certificate.verify(master_verifying_key) {
        Ok(info) => {
            println!("Delegate certificate {}", "verified".green());
            println!("Info: {}", info.blue());
            0
        }
        Err(e) => {
            eprintln!("{} to verify delegate certificate: {}", "Failed".red(), e);
            1
        }
    }
}

pub fn generate_ghost_key_cmd(
    delegate_certificate: &DelegateCertificate,
    delegate_signing_key: &RSASigningKey,
    output_dir: &Path,
) -> i32 {
    let (ghost_key_certificate, ghost_key_signing_key) =
        GhostkeyCertificate::new(delegate_certificate, delegate_signing_key);
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
    master_verifying_key: &VerifyingKey,
    ghost_certificate: &GhostkeyCertificate,
) -> i32 {
    match ghost_certificate.verify(&master_verifying_key.clone()) {
        Ok(info) => {
            println!("Ghost certificate {}", "verified".green());
            println!("Info: {}", info.blue());
            0
        }
        Err(e) => {
            eprintln!("{} to verify ghost key certificate: {}", "Failed".red(), e);
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
            file_path.display(),
            file_path.display()
        )
        .into());
    }
    Ok(())
}
