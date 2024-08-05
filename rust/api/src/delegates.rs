use std::path::PathBuf;

use blind_rsa_signatures::{BlindedMessage, BlindSignature, Options, SecretKey as RSASigningKey};
use rand_core::OsRng;

use ghostkey_lib::armorable::*;
use ghostkey_lib::delegate_certificate::DelegateCertificate;

use crate::handle_sign_cert::CertificateError;

pub(crate) fn get_delegate(amount: u64) -> Result<(DelegateCertificate, RSASigningKey), CertificateError> {
    let delegate_dir = PathBuf::from(std::env::var("DELEGATE_DIR").map_err(|e| {
        log::error!("DELEGATE_DIR environment variable not set: {}", e);
        CertificateError::KeyError("DELEGATE_DIR environment variable not set".to_string())
    })?);
    let cert_path = delegate_dir.join(format!("delegate_certificate_{}.pem", amount));
    let cert = DelegateCertificate::from_file(&cert_path)
        .map_err(|e| CertificateError::KeyError(format!("Unable to read certificate from {}: {}", cert_path.display(), e)))?;

    let signing_key_path = delegate_dir.join(format!("delegate_signing_key_{}.pem", amount));
    let signing_key = RSASigningKey::from_file(&signing_key_path)
        .map_err(|e| CertificateError::KeyError(format!("Unable to read signing key from {}: {}", signing_key_path.display(), e)))?;
    Ok((cert, signing_key))
}

pub(crate) fn sign_with_delegate_key(blinded_ghostkey: &BlindedMessage, amount_dollars: u64) -> Result<BlindSignature, CertificateError> {
    let (_, delegate_signing_key) = get_delegate(amount_dollars)?;

    let options = Options::default();

    let blind_sig = delegate_signing_key.blind_sign(&mut OsRng, blinded_ghostkey, &options)
        .map_err(|e| CertificateError::MiscError(format!("Failed to blind sign: {}", e)))?;

    Ok(blind_sig)
}