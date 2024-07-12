use std::path::Path;
use p256::ecdsa::{SigningKey, VerifyingKey, signature::Signer};
use rand_core::OsRng;
use base64::{engine::general_purpose, Engine as _};
use std::fs::{File, create_dir_all};
use std::io::Write;
use rmp_serde::encode::to_vec_named;
use serde::{Serialize, Deserialize};
use std::fs::read_to_string;

#[derive(Serialize, Deserialize)]
struct DelegateKeyCertificate {
    verifying_key: Vec<u8>,
    attributes: String,
    signature: Vec<u8>,
}

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
pub fn generate_delegate_key(master_key_dir: &str, attributes: &str, delegate_key_dir: &str) {
    // Read the master private key
    let master_private_key_path = Path::new(master_key_dir).join("server_master_private_key.pem");
    let master_private_key_pem = read_to_string(&master_private_key_path).expect("Unable to read master private key file");
    let master_private_key_bytes = general_purpose::STANDARD.decode(master_private_key_pem).expect("Failed to decode master private key");
    let master_private_key = SigningKey::from_bytes(&master_private_key_bytes).expect("Failed to create master private key");

    // Generate the delegate signing key
    let delegate_signing_key = SigningKey::random(&mut OsRng);
    let delegate_verifying_key = VerifyingKey::from(&delegate_signing_key);

    // Serialize the verifying key and attributes
    let verifying_key_bytes = delegate_verifying_key.to_encoded_point(false).as_bytes().to_vec();
    let certificate_data = DelegateKeyCertificate {
        verifying_key: verifying_key_bytes.clone(),
        attributes: attributes.to_string(),
        signature: vec![],
    };
    let certificate_data_bytes = to_vec_named(&certificate_data).expect("Failed to serialize certificate data");

    // Sign the certificate data
    let signature = master_private_key.sign(&certificate_data_bytes);
    let mut signed_certificate_data = certificate_data;
    signed_certificate_data.signature = signature.to_vec();

    // Encode the signed certificate data in base64
    let signed_certificate_data_bytes = to_vec_named(&signed_certificate_data).expect("Failed to serialize signed certificate data");
    let signed_certificate_base64 = general_purpose::STANDARD.encode(signed_certificate_data_bytes);

    // Create the output directory if it doesn't exist
    if let Err(e) = create_dir_all(delegate_key_dir) {
        eprintln!("Error: Unable to create output directory: {}", e);
        return;
    }
    let delegate_signing_key_path = Path::new(delegate_key_dir).join("delegate_signing_key.pem");
    let delegate_certificate_path = Path::new(delegate_key_dir).join("delegate_certificate.pem");

    // Check if files already exist
    if delegate_signing_key_path.exists() || delegate_certificate_path.exists() {
        eprintln!("Error: One or both key files already exist in the specified directory.");
        return;
    }

    // Write the delegate signing key and certificate to files
    let delegate_signing_key_base64 = general_purpose::STANDARD.encode(delegate_signing_key.to_bytes());
    let armored_delegate_signing_key = format!("-----BEGIN DELEGATE SIGNING KEY-----\n{}\n-----END DELEGATE SIGNING KEY-----", delegate_signing_key_base64);

    let mut delegate_signing_key_file = File::create(&delegate_signing_key_path).expect("Unable to create delegate signing key file");
    delegate_signing_key_file.write_all(armored_delegate_signing_key.as_bytes()).expect("Unable to write delegate signing key");

    let mut delegate_certificate_file = File::create(&delegate_certificate_path).expect("Unable to create delegate certificate file");
    delegate_certificate_file.write_all(signed_certificate_base64.as_bytes()).expect("Unable to write delegate certificate");

    println!("Delegate signing key and certificate generated successfully.");
}
