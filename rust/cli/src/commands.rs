use std::path::Path;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use log::info;
use p256::ecdsa::SigningKey;
use crate::util::create_keypair;
use crate::wrappers::signing_key::SerializableSigningKey;
use crate::wrappers::verifying_key::SerializableVerifyingKey;
use crate::armorable::*;
use crate::delegate_certificate::DelegateCertificate;
use crate::errors::GhostkeyError;

pub fn generate_master_key_cmd(output_dir: &Path, ignore_permissions: bool) -> Result<(), Box<dyn std::error::Error>> {
    let (signing_key, verifying_key) = create_keypair()?;
    let signing_key : SerializableSigningKey = signing_key.into();
    let verifying_key : SerializableVerifyingKey = verifying_key.into();
    let signing_key_file = output_dir.join("master_signing_key.pem");
    let verifying_key_file = output_dir.join("master_verifying_key.pem");
    info!("Writing master signing key to {}", signing_key_file.display());
    signing_key.to_file(&signing_key_file)?;
    info!("Writing master verifying key to {}", verifying_key_file.display());
    verifying_key.to_file(&verifying_key_file)?;
    if !ignore_permissions {
        require_strict_permissions(&signing_key_file)?;
    } else {
        info!("Ignoring permission checks for {}", signing_key_file.display());
    }
    Ok(())
}

pub fn generate_delegate_cmd(
    master_signing_key : &SigningKey,
    info : &String,
    output_dir : &Path,
    ignore_permissions : bool
) -> Result<(), GhostkeyError> {
    let (delegate_certificate, delegate_signing_key) = DelegateCertificate::new(&master_signing_key, &info).unwrap();
    let delegate_signing_key : SerializableSigningKey = delegate_signing_key.into();
    let delegate_certificate_file = output_dir.join("delegate_certificate.pem");
    let delegate_signing_key_file = output_dir.join("delegate_signing_key.pem");
    info!("Writing delegate certificate to {}", delegate_certificate_file.display());
    delegate_certificate.to_file(&delegate_certificate_file)?;
    info!("Writing delegate signing key to {}", delegate_signing_key_file.display());
    delegate_signing_key.to_file(&delegate_signing_key_file).unwrap();
    if !ignore_permissions {
        require_strict_permissions(&delegate_signing_key_file)?;
    } else {
        info!("Ignoring permission checks for {}", delegate_certificate_file.display());
        info!("Ignoring permission checks for {}", delegate_signing_key_file.display());
    }
    Ok(())
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
