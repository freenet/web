use std::process;
use clap::{Command, Arg, ArgAction};
use std::path::Path;
use colored::Colorize;
use log::info;
use ed25519_dalek::*;
use ghostkey::armorable::Armorable;
use ghostkey::commands::{generate_delegate_cmd, generate_ghostkey_cmd, generate_master_key_cmd, verify_delegate_cmd, verify_ghostkey_cmd};
use ghostkey::delegate_certificate::DelegateCertificate;
use ghostkey::ghostkey_certificate::GhostkeyCertificate;

const CMD_GENERATE_MASTER_KEY: &str = "generate-master-key";
const CMD_GENERATE_DELEGATE: &str = "generate-delegate";
const CMD_VERIFY_DELEGATE: &str = "verify-delegate";
const CMD_GENERATE_GHOST_KEY: &str = "generate-ghost-key";
const CMD_VERIFY_GHOST_KEY: &str = "verify-ghost-key";

const ARG_OUTPUT_DIR: &str = "output-dir";
const ARG_IGNORE_PERMISSIONS: &str = "ignore-permissions";
const ARG_MASTER_SIGNING_KEY: &str = "master-signing-key";
const ARG_INFO: &str = "info";
const ARG_MASTER_VERIFYING_KEY: &str = "master-verifying-key";
const ARG_DELEGATE_CERTIFICATE: &str = "delegate-certificate";
const ARG_DELEGATE_DIR: &str = "delegate-dir";
const ARG_GHOST_CERTIFICATE: &str = "ghost-certificate";

fn main() {
    let exit_code = run();
    process::exit(exit_code);
}

fn run() -> i32 {
    let matches = Command::new("Freenet Ghost Key Utility")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Performs various ghost key-related tasks")
        .subcommand(Command::new(CMD_GENERATE_MASTER_KEY)
            .about("Generates a new SERVER_MASTER_KEY and public key")
            .arg(Arg::new(ARG_OUTPUT_DIR)
                .long(ARG_OUTPUT_DIR)
                .help("The directory to output the keys")
                .required(true)
                .value_name("DIR"))
            .arg(Arg::new(ARG_IGNORE_PERMISSIONS)
                .long(ARG_IGNORE_PERMISSIONS)
                .help("Ignore file permission checks")
                .action(ArgAction::SetTrue)))
        .subcommand(Command::new(CMD_GENERATE_DELEGATE)
            .about("Generates a new delegate signing key and certificate")
            .arg(Arg::new(ARG_MASTER_SIGNING_KEY)
                .long(ARG_MASTER_SIGNING_KEY)
                .help("The file containing the master signing key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new(ARG_INFO)
                .long(ARG_INFO)
                .help("The info string to be included in the delegate key certificate")
                .required(true)
                .value_name("STRING"))
            .arg(Arg::new(ARG_OUTPUT_DIR)
                .long(ARG_OUTPUT_DIR)
                .help("The directory to output the delegate keys and certificate")
                .required(true)
                .value_name("DIR"))
            .arg(Arg::new(ARG_IGNORE_PERMISSIONS)
                .long(ARG_IGNORE_PERMISSIONS)
                .help("Ignore file permission checks")
                .action(ArgAction::SetTrue)))
        .subcommand(Command::new(CMD_VERIFY_DELEGATE)
            .about("Verifies a delegate key certificate using the master verifying key")
            .arg(Arg::new(ARG_MASTER_VERIFYING_KEY)
                .long(ARG_MASTER_VERIFYING_KEY)
                .help("The file containing the master verifying key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new(ARG_DELEGATE_CERTIFICATE)
                .long(ARG_DELEGATE_CERTIFICATE)
                .help("The file containing the delegate certificate")
                .required(true)
                .value_name("FILE")))
        .subcommand(Command::new(CMD_GENERATE_GHOST_KEY)
            .about("Generates a ghost key from a delegate signing key")
            .arg(Arg::new(ARG_DELEGATE_DIR)
                .long(ARG_DELEGATE_DIR)
                .help("The directory containing the delegate certificate and signing key")
                .required(true)
                .value_name("DIR"))
            .arg(Arg::new(ARG_OUTPUT_DIR)
                .long(ARG_OUTPUT_DIR)
                .help("The directory to output the ghost key files")
                .required(true)
                .value_name("DIR")))
        .subcommand(Command::new(CMD_VERIFY_GHOST_KEY)
            .about("Verifies a ghost key certificate using the master verifying key")
            .arg(Arg::new(ARG_MASTER_VERIFYING_KEY)
                .long(ARG_MASTER_VERIFYING_KEY)
                .help("The file containing the master verifying key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new(ARG_GHOST_CERTIFICATE)
                .long(ARG_GHOST_CERTIFICATE)
                .help("The file containing the ghost key certificate")
                .required(true)
                .value_name("FILE")))
        .get_matches();

    match matches.subcommand() {
        Some((CMD_GENERATE_MASTER_KEY, sub_matches)) => {
            let output_dir = Path::new(sub_matches.get_one::<String>(ARG_OUTPUT_DIR).unwrap());

            if let Err(e) = std::fs::create_dir_all(output_dir) {
                eprintln!("{} to create output directory: {}", "Failed".red(), e);
                return 1;
            }

            let ignore_permissions = sub_matches.get_flag(ARG_IGNORE_PERMISSIONS);

            let result = generate_master_key_cmd(output_dir, ignore_permissions);
            if result == 0 {
                println!("{}", "Master key generation completed successfully.".green());
            }
            result
        }
        Some((CMD_GENERATE_DELEGATE, sub_matches)) => {
            let master_signing_key_file = Path::new(sub_matches.get_one::<String>(ARG_MASTER_SIGNING_KEY).unwrap());
            let master_signing_key = match SigningKey::from_file(master_signing_key_file) {
                Ok(key) => key,
                Err(e) => {
                    eprintln!("{} to read master signing key: {}", "Failed".red(), e);
                    return 1;
                }
            };
            let info = sub_matches.get_one::<String>(ARG_INFO).unwrap();
            let output_dir = Path::new(sub_matches.get_one::<String>(ARG_OUTPUT_DIR).unwrap());
            if let Err(e) = std::fs::create_dir_all(output_dir) {
                eprintln!("{} to create output directory: {}", "Failed".red(), e);
                return 1;
            }

            let ignore_permissions = sub_matches.get_flag(ARG_IGNORE_PERMISSIONS);

            let result = generate_delegate_cmd(&master_signing_key, info, output_dir, ignore_permissions);
            if result == 0 {
                println!("{}", "Delegate key generation completed successfully.".green());
            }
            result
        }
        Some((CMD_VERIFY_DELEGATE, sub_matches)) => {
            let master_verifying_key_file = Path::new(sub_matches.get_one::<String>(ARG_MASTER_VERIFYING_KEY).unwrap());
            let master_verifying_key = match VerifyingKey::from_file(master_verifying_key_file) {
                Ok(key) => key,
                Err(e) => {
                    println!("{} to read master verifying key: {}", "Failed".red(), e);
                    return 1;
                }
            };
            let delegate_certificate_file = Path::new(sub_matches.get_one::<String>(ARG_DELEGATE_CERTIFICATE).unwrap());
            let delegate_certificate = match DelegateCertificate::from_file(delegate_certificate_file) {
                Ok(cert) => cert,
                Err(e) => {
                    println!("{} to read delegate certificate: {}", "Failed".red(), e);
                    return 1;
                }
            };
            verify_delegate_cmd(&master_verifying_key, &delegate_certificate)
        }
        Some((CMD_GENERATE_GHOST_KEY, sub_matches)) => {
            let delegate_dir = sub_matches.get_one::<String>(ARG_DELEGATE_DIR).unwrap();
            let delegate_certificate_file = Path::new(delegate_dir).join("delegate_certificate.pem");
            let delegate_certificate = match DelegateCertificate::from_file(&delegate_certificate_file) {
                Ok(cert) => cert,
                Err(e) => {
                    eprintln!("{} to read delegate certificate: {}", "Failed".red(), e);
                    return 1;
                }
            };
            let delegate_signing_key_file = Path::new(delegate_dir).join("delegate_signing_key.pem");
            let delegate_signing_key = match SigningKey::from_file(&delegate_signing_key_file) {
                Ok(key) => key,
                Err(e) => {
                    eprintln!("{} to read delegate signing key: {}", "Failed".red(), e);
                    return 1;
                }
            };

            let output_dir = Path::new(sub_matches.get_one::<String>(ARG_OUTPUT_DIR).unwrap());
            if let Err(e) = std::fs::create_dir_all(output_dir) {
                eprintln!("{} to create output directory: {}", "Failed".red(), e);
                return 1;
            }

            generate_ghostkey_cmd(&delegate_certificate, &delegate_signing_key, &output_dir)
        }
        Some((CMD_VERIFY_GHOST_KEY, sub_matches)) => {
            let master_verifying_key_file = Path::new(sub_matches.get_one::<String>(ARG_MASTER_VERIFYING_KEY).unwrap());
            let master_verifying_key = match VerifyingKey::from_file(master_verifying_key_file) {
                Ok(key) => key,
                Err(e) => {
                    eprintln!("{} to read master verifying key: {}", "Failed".red(), e);
                    return 1;
                }
            };
            let ghost_certificate_file = Path::new(sub_matches.get_one::<String>(ARG_GHOST_CERTIFICATE).unwrap());
            let ghost_certificate = match GhostkeyCertificate::from_file(ghost_certificate_file) {
                Ok(cert) => cert,
                Err(e) => {
                    eprintln!("{} to read ghost key certificate: {}", "Failed".red(), e);
                    return 1;
                }
            };
            verify_ghostkey_cmd(&master_verifying_key, &ghost_certificate)
        }
        _ => {
            info!("No valid subcommand provided. Use --help for usage information.");
            0
        }
    }
}
