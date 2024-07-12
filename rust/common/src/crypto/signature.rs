use super::*;

pub fn verify_signature(verifying_key_pem: &str, message: &str, signature: &str) -> Result<bool, CryptoError> {
    let verifying_key_base64 = extract_base64_from_armor(verifying_key_pem, "SERVER VERIFYING KEY")?;
    let verifying_key_bytes = general_purpose::STANDARD.decode(&verifying_key_base64)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let verifying_key = VerifyingKey::from_sec1_bytes(&verifying_key_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    let signature_bytes = extract_base64_from_armor(signature, "SIGNATURE")?;
    let signature_bytes = general_purpose::STANDARD.decode(&signature_bytes)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let signature = ecdsa::Signature::from_slice(&signature_bytes)
        .map_err(|e| CryptoError::InvalidInput(e.to_string()))?;

    Ok(verifying_key.verify(message.as_bytes(), &signature).is_ok())
}

pub fn sign_message(signing_key_pem: &str, message: &str) -> Result<String, CryptoError> {
    let signing_key_base64 = extract_base64_from_armor(signing_key_pem, "SERVER SIGNING KEY")?;
    let signing_key_bytes = general_purpose::STANDARD.decode(&signing_key_base64)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let field_bytes = FieldBytes::from_slice(&signing_key_bytes);
    let signing_key = SigningKey::from_bytes(field_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    let signature: ecdsa::Signature = signing_key.sign(message.as_bytes());
    let signature_bytes = signature.to_vec();
    let armored_signature = armor(&signature_bytes, "SIGNATURE", "SIGNATURE");

    Ok(armored_signature)
}
use clap::{Command, Arg};
use std::fs::read_to_string;
use crate::save_key_to_file;

pub fn setup_sign_message_command() -> Command {
    Command::new("sign-message")
        .about("Signs a message using a signing key and outputs the signature")
        .arg(Arg::new("signing-key-file")
            .long("signing-key-file")
            .help("The file containing the signing key (master or delegate)")
            .required(true)
            .value_name("FILE"))
        .arg(Arg::new("message")
            .long("message")
            .help("The message to sign (required if --message-file is not provided)")
            .required_unless_present("message-file")
            .conflicts_with("message-file")
            .value_name("STRING"))
        .arg(Arg::new("message-file")
            .long("message-file")
            .help("The file containing the message to sign (required if --message is not provided)")
            .required_unless_present("message")
            .conflicts_with("message")
            .value_name("FILE"))
        .arg(Arg::new("output-file")
            .long("output-file")
            .help("The file to output the signature (if omitted, signature is sent to stdout)")
            .required(false)
            .value_name("FILE"))
}

pub fn sign_message_command(signing_key_file: &str, message: Option<&str>, message_file: Option<&str>, output_file: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let signing_key = read_to_string(signing_key_file)?;
    
    let message_content = if let Some(msg) = message {
        msg.to_string()
    } else if let Some(file) = message_file {
        read_to_string(file)?
    } else {
        return Err("Either message or message-file must be provided".into());
    };

    let signature = sign_message(&signing_key, &message_content)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    match output_file {
        Some(file) => {
            save_key_to_file("", file, &signature)?;
            println!("Message signed successfully. Signature saved to: {}", file);
        },
        None => {
            println!("{}", signature);
        }
    }
    Ok(())
}
