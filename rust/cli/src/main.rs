use clap::{Command, Arg};
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::path::Path;
use common::crypto::generate_delegate::generate_delegate_key;
use common::crypto::master_key::{generate_master_key, generate_master_verifying_key};
use colored::Colorize;
use common::crypto::ghost_key::{generate_ghostkey, validate_ghost_key};
use common::crypto::signature::{sign_message, verify_signature};
use common::crypto::validate_delegate_key;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("Freenet Ghost Key Utility")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Performs various ghost key-related tasks")
        .subcommand(Command::new("sign-message")
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
                .value_name("FILE")))
        .subcommand(Command::new("verify-signature")
            .about("Verifies a signature for a message using a verifying key")
            .arg(Arg::new("verifying-key-file")
                .long("verifying-key-file")
                .help("The file containing the verifying key (master or delegate)")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("message")
                .long("message")
                .help("The message to verify (required if --message-file is not provided)")
                .required_unless_present("message-file")
                .conflicts_with("message-file")
                .value_name("STRING"))
            .arg(Arg::new("message-file")
                .long("message-file")
                .help("The file containing the message to verify (required if --message is not provided)")
                .required_unless_present("message")
                .conflicts_with("message")
                .value_name("FILE"))
            .arg(Arg::new("signature-file")
                .long("signature-file")
                .help("The file containing the signature to verify")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("master-verifying-key-file")
                .long("master-verifying-key-file")
                .help("The file containing the master verifying key (optional, for delegate key validation)")
                .required(false)
                .value_name("FILE")))
        .subcommand(Command::new("generate-master-key")
            .about("Generates a new SERVER_MASTER_KEY and public key")
            .arg(Arg::new("output-dir")
                .long("output-dir")
                .help("The directory to output the keys")
                .required(true)
                .value_name("DIR")))
        .subcommand(Command::new("generate-delegate-key")
            .about("Generates a new delegate key and certificate")
            .arg(Arg::new("master-signing-key-file")
                .long("master-signing-key-file")
                .help("The file containing the master signing key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("info")
                .long("info")
                .help("The info string to be included in the delegate key certificate")
                .required(true)
                .value_name("STRING"))
            .arg(Arg::new("output-dir")
                .long("output-dir")
                .help("The directory to output the delegate keys and certificate")
                .required(true)
                .value_name("DIR")))
        .subcommand(Command::new("validate-delegate-key")
            .about("Validates a delegate key certificate using the master verifying key")
            .arg(Arg::new("master-verifying-key-file")
                .long("master-verifying-key-file")
                .help("The file containing the master verifying key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("delegate-certificate-file")
                .long("delegate-certificate-file")
                .help("The file containing the delegate certificate")
                .required(true)
                .value_name("FILE")))
        .subcommand(Command::new("generate-verifying-key")
            .about("Generates a verifying key from a master signing key")
            .arg(Arg::new("master-signing-key-file")
                .long("master-signing-key-file")
                .help("The file containing the master signing key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("output-file")
                .long("output-file")
                .help("The file to output the master verifying key")
                .required(true)
                .value_name("FILE")))
        .subcommand(Command::new("generate-ghost-key")
            .about("Generates a ghost key from a delegate signing key")
            .arg(Arg::new("delegate-certificate-file")
                .long("delegate-certificate-file")
                .help("The file containing the delegate certificate")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("output-dir")
                .long("output-dir")
                .help("The directory to output the ghost key files")
                .required(true)
                .value_name("DIR")))
        .subcommand(Command::new("validate-ghost-key")
            .about("Validates a ghost key certificate using the master verifying key")
            .arg(Arg::new("master-verifying-key-file")
                .long("master-verifying-key-file")
                .help("The file containing the master verifying key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("ghost-certificate-file")
                .long("ghost-certificate-file")
                .help("The file containing the ghost key certificate")
                .required(true)
                .value_name("FILE")))
        .get_matches();

    match matches.subcommand() {
        Some(("generate-master-key", sub_matches)) => {
            let output_dir = sub_matches.get_one::<String>("output-dir").unwrap();
            generate_and_save_master_key(output_dir)?;
        }
        Some(("generate-delegate-key", sub_matches)) => {
            let master_signing_key_file = sub_matches.get_one::<String>("master-signing-key-file").unwrap();
            let info = sub_matches.get_one::<String>("info").unwrap();
            let output_dir = sub_matches.get_one::<String>("output-dir").unwrap();
            generate_and_save_delegate_key(master_signing_key_file, info, output_dir)?;
        }
        Some(("validate-delegate-key", sub_matches)) => {
            let master_verifying_key_file = sub_matches.get_one::<String>("master-verifying-key-file").unwrap();
            let delegate_certificate_file = sub_matches.get_one::<String>("delegate-certificate-file").unwrap();
            validate_delegate_key_command(master_verifying_key_file, delegate_certificate_file)?;
        }
        Some(("generate-verifying-key", sub_matches)) => {
            let master_signing_key_file = sub_matches.get_one::<String>("master-signing-key-file").unwrap();
            let output_file = sub_matches.get_one::<String>("output-file").unwrap();
            generate_master_verifying_key_command(master_signing_key_file, output_file)?;
        }
        Some(("generate-ghost-key", sub_matches)) => {
            let delegate_certificate_file = sub_matches.get_one::<String>("delegate-certificate-file").unwrap();
            let output_dir = sub_matches.get_one::<String>("output-dir").unwrap();
            generate_ghostkey_command(delegate_certificate_file, output_dir)?;
        }
        Some(("validate-ghost-key", sub_matches)) => {
            let master_verifying_key_file = sub_matches.get_one::<String>("master-verifying-key-file").unwrap();
            let ghost_certificate_file = sub_matches.get_one::<String>("ghost-certificate-file").unwrap();
            validate_ghost_key_command(master_verifying_key_file, ghost_certificate_file)?;
        }
        Some(("sign-message", sub_matches)) => {
            let signing_key_file = sub_matches.get_one::<String>("signing-key-file").unwrap();
            let message = sub_matches.get_one::<String>("message");
            let message_file = sub_matches.get_one::<String>("message-file");
            let output_file = sub_matches.get_one::<String>("output-file");
            sign_message_command(signing_key_file, message.map(|s| s.as_str()), message_file.map(|s| s.as_str()), output_file.map(|s| s.as_str()))?;
        }
        Some(("verify-signature", sub_matches)) => {
            let verifying_key_file = sub_matches.get_one::<String>("verifying-key-file").unwrap();
            let message = sub_matches.get_one::<String>("message");
            let message_file = sub_matches.get_one::<String>("message-file");
            let signature_file = sub_matches.get_one::<String>("signature-file").unwrap();
            let master_verifying_key_file = sub_matches.get_one::<String>("master-verifying-key-file");
            verify_signature_command(verifying_key_file, message.map(|s| s.as_str()), message_file.map(|s| s.as_str()), signature_file, master_verifying_key_file.map(|s| s.as_str()))?;
        }
        _ => {
            println!("No valid subcommand provided. Use --help for usage information.");
        }
    }

    Ok(())
}

fn sign_message_command(signing_key_file: &str, message: Option<&str>, message_file: Option<&str>, output_file: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let signing_key = std::fs::read_to_string(signing_key_file)?;
    
    let message_content = if let Some(msg) = message {
        msg.to_string()
    } else if let Some(file) = message_file {
        std::fs::read_to_string(file)?
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

fn validate_delegate_key_command(master_verifying_key_file: &str, delegate_certificate_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let master_verifying_key = std::fs::read_to_string(master_verifying_key_file)?;
    let delegate_certificate = std::fs::read_to_string(delegate_certificate_file)?;
    
    match validate_delegate_key(&master_verifying_key, &delegate_certificate) {
        Ok(info) => {
            println!("Delegate key certificate is {}.", "valid".green());
            println!("Info: {}", info);
            Ok(())
        }
        Err(e) => {
            println!("Failed to validate delegate key certificate: {}", e);
            Err(Box::new(e))
        }
    }
}

fn generate_and_save_master_key(output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (private_key, public_key) = generate_master_key().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    save_key_to_file(output_dir, "master_signing_key.pem", &private_key)?;
    save_key_to_file(output_dir, "master_verifying_key.pem", &public_key)?;
    println!("MASTER_SIGNING_KEY and MASTER_VERIFYING_KEY generated successfully.");
    Ok(())
}

fn generate_and_save_delegate_key(master_key_file: &str, info: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let master_signing_key = std::fs::read_to_string(master_key_file)?;
    let delegate_certificate = generate_delegate_key(&master_signing_key, info)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    save_key_to_file(output_dir, "delegate_certificate.pem", &delegate_certificate)?;
    println!("Delegate certificate generated successfully.");
    Ok(())
}


fn save_key_to_file(output_dir: &str, filename: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    create_dir_all(output_dir)?;
    let file_path = Path::new(output_dir).join(filename);
    let mut file = File::create(file_path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}
fn verify_signature_command(verifying_key_file: &str, message: Option<&str>, message_file: Option<&str>, signature_file: &str, master_verifying_key_file: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let verifying_key = std::fs::read_to_string(verifying_key_file)?;
    let signature = std::fs::read_to_string(signature_file)?;
    
    let message_content = if let Some(msg) = message {
        msg.to_string()
    } else if let Some(file) = message_file {
        std::fs::read_to_string(file)?
    } else {
        return Err("Either message or message-file must be provided".into());
    };

    if let Some(master_key_file) = master_verifying_key_file {
        let master_verifying_key = std::fs::read_to_string(master_key_file)?;
        validate_delegate_key(&master_verifying_key, &verifying_key)?;
        println!("Delegate key validated successfully.");
    }

    match verify_signature(&verifying_key, &message_content, &signature) {
        Ok(true) => {
            println!("Signature is {}.", "valid".green());
            Ok(())
        },
        Ok(false) => {
            println!("Signature is {}.", "invalid".red());
            Ok(())
        },
        Err(e) => {
            println!("Failed to verify signature: {}", e);
            Err(Box::new(e))
        }
    }
}

fn generate_master_verifying_key_command(master_signing_key_file: &str, output_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let master_signing_key = std::fs::read_to_string(master_signing_key_file)?;
    let master_verifying_key = generate_master_verifying_key(&master_signing_key)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    save_key_to_file("", output_file, &master_verifying_key)?;
    
    println!("Server Master Verifying Key generated successfully and saved to: {}", output_file);
    Ok(())
}

fn generate_ghostkey_command(delegate_certificate_file: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let delegate_certificate = std::fs::read_to_string(delegate_certificate_file)
        .map_err(|e| format!("Failed to read delegate certificate file: {}", e))?;
    
    let ghostkey_certificate = generate_ghostkey(&delegate_certificate)
        .map_err(|e| format!("Failed to generate ghostkey: {}", e))?;
    
    save_key_to_file(output_dir, "ghostkey_certificate.pem", &ghostkey_certificate)
        .map_err(|e| format!("Failed to save ghostkey certificate: {}", e))?;
    
    println!("Ghostkey generated successfully.");
    println!("Ghostkey certificate saved to: {}/ghostkey_certificate.pem", output_dir);
    Ok(())
}

fn validate_ghost_key_command(master_verifying_key_file: &str, ghost_certificate_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let master_verifying_key = std::fs::read_to_string(master_verifying_key_file)?;
    let ghost_certificate = std::fs::read_to_string(ghost_certificate_file)?;

    match validate_ghost_key(&master_verifying_key, &ghost_certificate) {
        Ok(info) => {
            println!("Ghost key certificate is {}.", "valid".green());
            println!("Info: {}", info);
            Ok(())
        }
        Err(e) => {
            println!("Ghost key certificate is {}.", "invalid".red());
            println!("Reason: {}", e);
            Err(Box::new(e))
        }
    }
}
