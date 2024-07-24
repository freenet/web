use std::path::{Path, PathBuf};
use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;

pub(crate) fn generate_master_key(dir : PathBuf) -> Result<(SigningKey, VerifyingKey), Box<dyn std::error::Error>> {
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let signing_key_file = dir.join("master_signing_key.pem");
    let verifying_key_file = dir.join("master_verifying_key.pem");
    todo!()
}