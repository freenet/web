use std::path::Path;
use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use base64::{engine::general_purpose, Engine as _};
use std::fs::{File, create_dir_all};
use std::io::Write;

pub fn generate_signing_key(output_dir: &str) {
    // Generate the signing key
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);

    // Encode the keys in base64
    let signing_key_base64 = general_purpose::STANDARD.encode(signing_key.to_bytes());
    let verifying_key_base64 = general_purpose::STANDARD.encode(verifying_key.to_encoded_point(false).as_bytes());

    // Armor the keys
    let armored_signing_key = format!("-----BEGIN SERVER SIGNING KEY-----\n{}\n-----END SERVER SIGNING KEY-----", signing_key_base64);
    let armored_verifying_key = format!("-----BEGIN SERVER VERIFYING KEY-----\n{}\n-----END SERVER VERIFYING KEY-----", verifying_key_base64);

    // Create the output directory if it doesn't exist
    if let Err(e) = create_dir_all(output_dir) {
        eprintln!("Error: Unable to create output directory: {}", e);
        return;
    }
    let signing_key_path = Path::new(output_dir).join("server_signing_key.pem");
    let verifying_key_path = Path::new(output_dir).join("server_public_key.pem");

    // Check if files already exist
    if signing_key_path.exists() || verifying_key_path.exists() {
        eprintln!("Error: One or both key files already exist in the specified directory.");
        return;
    }

    // Write the keys to files
    let mut signing_key_file = File::create(&signing_key_path).expect("Unable to create signing key file");
    signing_key_file.write_all(armored_signing_key.as_bytes()).expect("Unable to write signing key");

    let mut verifying_key_file = File::create(&verifying_key_path).expect("Unable to create public key file");
    verifying_key_file.write_all(armored_verifying_key.as_bytes()).expect("Unable to write public key");

    println!("SERVER_SIGNING_KEY and public key generated successfully.");
}
