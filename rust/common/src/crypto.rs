use p256::ecdsa::{SigningKey as PrivateKey, VerifyingKey as PublicKey};
use rand_core::OsRng;
use base64::{engine::general_purpose, Engine as _};
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::path::Path;
use serde_json::Value;
use sha2::{Sha256, Digest};
use p256::SecretKey;
use p256::ecdsa::{self, signature::Signer};
use std::env;

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

    let master_private_key = PrivateKey::from_bytes(&general_purpose::STANDARD.decode(pad_base64(&server_master_private_key)).map_err(|e| e.to_string())?)
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
    let nonce_bytes = nonce.to_be_bytes();

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

fn pad_base64(base64_str: &str) -> String {
    let mut padded = base64_str.to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }
    padded
}
