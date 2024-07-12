use p256::ecdsa::{SigningKey, VerifyingKey, SigningKey as PrivateKey, VerifyingKey as PublicKey};
use rand_core::OsRng;
use base64::{engine::general_purpose, Engine as _};
use std::fs::{File, create_dir_all, read_to_string};
use std::io::Write;
use std::path::Path;
use serde_json::Value;
use sha2::{Sha256, Digest};
use p256::{SecretKey, FieldBytes};
use p256::ecdsa::{self, signature::Signer};
use std::env;
use crate::armor;
use serde::{Serialize, Deserialize};
use serde_json::to_vec as to_vec_named;
use generic_array::GenericArray;

pub fn generate_master_key(output_dir: &str) {
    // Generate the master private key
    let master_private_key = PrivateKey::random(&mut OsRng);
    let master_public_key = PublicKey::from(&master_private_key);

    // Encode the keys in base64
    let master_private_key_base64 = general_purpose::STANDARD.encode(master_private_key.to_bytes());
    let master_public_key_base64 = general_purpose::STANDARD.encode(master_public_key.to_encoded_point(false).as_bytes());

    // Armor the keys
    let armored_master_private_key = format!("-----BEGIN SERVER MASTER PRIVATE KEY-----\n{}\n-----END SERVER MASTER PRIVATE KEY-----", master_private_key_base64);
    let armored_master_public_key = format!("-----BEGIN SERVER MASTER VERIFYING KEY-----\n{}\n-----END SERVER MASTER VERIFYING KEY-----", master_public_key_base64);

    // Create the output directory if it doesn't exist
    if let Err(e) = create_dir_all(output_dir) {
        eprintln!("Error: Unable to create output directory: {}", e);
        return;
    }
    let master_private_key_path = Path::new(output_dir).join("server_master_private_key.pem");
    let master_public_key_path = Path::new(output_dir).join("server_master_public_key.pem");

    // Check if files already exist
    if master_private_key_path.exists() || master_public_key_path.exists() {
        eprintln!("Error: One or both key files already exist in the specified directory.");
        return;
    }

    // Write the keys to files
    let mut master_private_key_file = File::create(&master_private_key_path).expect("Unable to create master private key file");
    master_private_key_file.write_all(armored_master_private_key.as_bytes()).expect("Unable to write master private key");

    let mut master_public_key_file = File::create(&master_public_key_path).expect("Unable to create master public key file");
    master_public_key_file.write_all(armored_master_public_key.as_bytes()).expect("Unable to write master public key");

    println!("SERVER_MASTER_PRIVATE_KEY and SERVER_MASTER_VERIFYING_KEY generated successfully.");
}

pub fn sign_with_key(blinded_public_key: &Value) -> Result<String, String> {
    let server_master_private_key = match env::var("SERVER_MASTER_PRIVATE_KEY") {
        Ok(key) => key,
        Err(e) => return Err(format!("Environment variable SERVER_MASTER_PRIVATE_KEY not found: {}", e)),
    };

    let decoded_key = general_purpose::STANDARD.decode(&server_master_private_key).map_err(|e| e.to_string())?;
    let field_bytes: FieldBytes = GenericArray::clone_from_slice(&decoded_key);
    let master_private_key = PrivateKey::from_bytes(&field_bytes)
        .map_err(|e| format!("Failed to create master private key: {}", e))?;

    let blinded_public_key_bytes = match blinded_public_key {
        Value::String(s) => general_purpose::STANDARD.decode(s).map_err(|e| format!("Failed to decode blinded public key: {}", e))?,
        Value::Object(obj) => {
            let x = obj.get("x").and_then(Value::as_str).ok_or_else(|| "Missing 'x' coordinate".to_string())?;
            let y = obj.get("y").and_then(Value::as_str).ok_or_else(|| "Missing 'y' coordinate".to_string())?;

            let mut bytes = Vec::new();
            bytes.extend_from_slice(&general_purpose::STANDARD.decode(x).map_err(|e| format!("Failed to decode 'x' coordinate: {}", e))?);
            bytes.extend_from_slice(&general_purpose::STANDARD.decode(y).map_err(|e| format!("Failed to decode 'y' coordinate: {}", e))?);
            bytes
        },
        _ => return Err("Invalid blinded public key format".to_string()),
    };

    // Generate a random nonce
    let nonce = SecretKey::random(&mut OsRng);
    let nonce_bytes = nonce.to_bytes();

    // Combine the blinded public key and nonce, and hash them
    let mut hasher = Sha256::new();
    hasher.update(&blinded_public_key_bytes);
    hasher.update(&nonce_bytes);
    let message = hasher.finalize();

    // Sign the hash
    let blind_signature: ecdsa::Signature = master_private_key.sign(&message);

    // Combine the signature and nonce
    let mut combined = blind_signature.to_vec();
    combined.extend_from_slice(&nonce_bytes);

    Ok(general_purpose::STANDARD.encode(combined))
}


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
    let armored_signing_key = armor(&signing_key_base64.as_bytes(), "SERVER SIGNING KEY", "SERVER SIGNING KEY");
    let armored_verifying_key = armor(&verifying_key_base64.as_bytes(), "SERVER VERIFYING KEY", "SERVER VERIFYING KEY");

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
    let master_private_key_bytes = general_purpose::STANDARD.decode(&master_private_key_pem).expect("Failed to decode master private key");
    let field_bytes: FieldBytes = GenericArray::clone_from_slice(&master_private_key_bytes);
    let master_private_key = SigningKey::from_bytes(&field_bytes).expect("Failed to create master private key");

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
    let armored_delegate_signing_key = armor(&delegate_signing_key_base64.as_bytes(), "DELEGATE SIGNING KEY", "DELEGATE SIGNING KEY");

    let mut delegate_signing_key_file = File::create(&delegate_signing_key_path).expect("Unable to create delegate signing key file");
    delegate_signing_key_file.write_all(armored_delegate_signing_key.as_bytes()).expect("Unable to write delegate signing key");

    let mut delegate_certificate_file = File::create(&delegate_certificate_path).expect("Unable to create delegate certificate file");
    delegate_certificate_file.write_all(signed_certificate_base64.as_bytes()).expect("Unable to write delegate certificate");

    println!("Delegate signing key and certificate generated successfully.");
}
