use ghostkey_lib::armorable::*;
use ghostkey_lib::delegate_certificate::DelegateCertificateV1;
use ghostkey_lib::errors::GhostkeyError;
use ghostkey_lib::ghost_key_certificate::GhostkeyCertificateV1;
use ghostkey_lib::util::create_keypair;
use blind_rsa_signatures::SecretKey as RSASigningKey;
use colored::Colorize;
use ed25519_dalek::*;
use log::info;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use rand_core::OsRng;
use crate::signed_message::SignedMessage;

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
        match DelegateCertificateV1::new(&master_signing_key, &info) {
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
    master_verifying_key: &Option<VerifyingKey>,
    delegate_certificate: &DelegateCertificateV1,
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

pub fn sign_message_cmd(
    ghost_certificate: GhostkeyCertificateV1,
    ghost_signing_key: &SigningKey,
    message: &[u8],
    output_file: &Path,
) -> i32 {
    if ghost_signing_key.verifying_key() != ghost_certificate.verifying_key {
        eprintln!(
            "{}: The signing key does not match the ghost certificate",
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
            println!(
                "{} written {}",
                "Signed message",
                "successfully".green()
            );
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
                            println!("Message: {}", String::from_utf8_lossy(&signed_message.message));
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
    delegate_certificate_path: &Path,
    delegate_signing_key_path: &Path,
    output_dir: &Path,
) -> i32 {
    let delegate_certificate = match DelegateCertificateV1::from_file(delegate_certificate_path) {
        Ok(cert) => cert,
        Err(e) => {
            eprintln!("{} to read delegate certificate: {}", "Failed".red(), e);
            return 1;
        }
    };

    let delegate_signing_key = match RSASigningKey::from_file(delegate_signing_key_path) {
        Ok(key) => key,
        Err(e) => {
            eprintln!("{} to read delegate signing key: {}", "Failed".red(), e);
            return 1;
        }
    };

    if delegate_signing_key.public_key().unwrap() != delegate_certificate.payload.delegate_verifying_key {
        eprintln!(
            "{}: The signing key does not match the delegate certificate",
            "Error".red()
        );
        return 1;
    }
    
    let (ghost_key_certificate, ghost_key_signing_key) =
        GhostkeyCertificateV1::new(delegate_certificate, delegate_signing_key);
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
