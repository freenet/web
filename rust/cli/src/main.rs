use clap::Command;

fn main() {
    let matches = Command::new("Freenet Key Utility")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Performs various Freenet-related tasks")
        .subcommand(Command::new("generate-key")
            .about("Generates a new key"))
        .subcommand(Command::new("generate-signing-key")
            .about("Generates a new SERVER_SIGNING_KEY and public key"))
        .get_matches();

    if let Some(_) = matches.subcommand_matches("generate-key") {
        // Existing generate-key functionality
    } else if let Some(_) = matches.subcommand_matches("generate-signing-key") {
        generate_signing_key();
    }

}
use p256::{
    ecdsa::{SigningKey, VerifyingKey},
    elliptic_curve::sec1::ToEncodedPoint,
};
use rand_core::OsRng;
use base64::{engine::general_purpose, Engine as _};
use std::fs::File;
use std::io::Write;

fn generate_signing_key() {
    // Generate the signing key
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);

    // Encode the keys in base64
    let signing_key_base64 = general_purpose::STANDARD.encode(signing_key.to_bytes());
    let verifying_key_base64 = general_purpose::STANDARD.encode(verifying_key.to_encoded_point(false).as_bytes());

    // Armor the keys
    let armored_signing_key = format!("-----BEGIN SERVER SIGNING KEY-----\n{}\n-----END SERVER SIGNING KEY-----", signing_key_base64);
    let armored_verifying_key = format!("-----BEGIN SERVER PUBLIC KEY-----\n{}\n-----END SERVER PUBLIC KEY-----", verifying_key_base64);

    // Write the keys to files
    let mut signing_key_file = File::create("server_signing_key.pem").expect("Unable to create signing key file");
    signing_key_file.write_all(armored_signing_key.as_bytes()).expect("Unable to write signing key");

    let mut verifying_key_file = File::create("server_public_key.pem").expect("Unable to create public key file");
    verifying_key_file.write_all(armored_verifying_key.as_bytes()).expect("Unable to write public key");

    println!("SERVER_SIGNING_KEY and public key generated successfully.");
}
