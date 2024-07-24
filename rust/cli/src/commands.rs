use std::path::Path;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use log::info;
use crate::master::create_master_keypair;
use crate::wrappers::signing_key::SerializableSigningKey;
use crate::wrappers::verifying_key::SerializableVerifyingKey;
use crate::armorable::*;

pub fn generate_master_key_cmd(output_dir: &Path, ignore_permissions: bool) -> Result<(), Box<dyn std::error::Error>> {
    let (signing_key, verifying_key) = create_master_keypair()?;
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


fn require_strict_permissions(file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = fs::metadata(file_path)?;
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
