use blind_rsa_signatures::SecretKey as RSASigningKey;
use clap::{Arg, ArgAction, Command};
use colored::Colorize;
use ed25519_dalek::*;
use ghostkey::commands::{
    generate_ghost_key_cmd, generate_master_key_cmd, generate_notary_cmd, resolve_notary_file,
    sign_message_cmd, verify_ghost_key_cmd, verify_notary_cmd, verify_signed_message_cmd,
    LEGACY_DELEGATE_CERT_FILENAME, LEGACY_DELEGATE_SIGNING_KEY_FILENAME, NOTARY_CERT_FILENAME,
    NOTARY_SIGNING_KEY_FILENAME,
};
use ghostkey_lib::armorable::Armorable;
use ghostkey_lib::ghost_key_certificate::GhostkeyCertificateV1;
use ghostkey_lib::notary_certificate::NotaryCertificateV1;
use log::info;
use std::fs;
use std::path::Path;
use std::process;

const CMD_GENERATE_MASTER_KEY: &str = "generate-master-key";
const CMD_GENERATE_NOTARY: &str = "generate-notary";
const CMD_VERIFY_NOTARY: &str = "verify-notary";
const CMD_GENERATE_GHOST_KEY: &str = "generate-ghost-key";
const CMD_VERIFY_GHOST_KEY: &str = "verify-ghost-key";
const CMD_SIGN_MESSAGE: &str = "sign-message";
const CMD_VERIFY_SIGNED_MESSAGE: &str = "verify-signed-message";

// Legacy subcommand names — still parsed via clap aliases for backward
// compatibility, warned on via pre-parse in main().
const LEGACY_CMD_GENERATE_DELEGATE: &str = "generate-delegate";
const LEGACY_CMD_VERIFY_DELEGATE: &str = "verify-delegate";

const ARG_OUTPUT_DIR: &str = "output-dir";
const ARG_IGNORE_PERMISSIONS: &str = "ignore-permissions";
const ARG_MASTER_SIGNING_KEY: &str = "master-signing-key";
const ARG_INFO: &str = "info";
const ARG_MASTER_VERIFYING_KEY: &str = "master-verifying-key";
const ARG_NOTARY_CERTIFICATE: &str = "notary-certificate";
const ARG_NOTARY_DIR: &str = "notary-dir";
const ARG_GHOST_CERTIFICATE: &str = "ghost-certificate";

// Legacy flag names accepted as aliases.
const LEGACY_ARG_DELEGATE_CERTIFICATE: &str = "delegate-certificate";
const LEGACY_ARG_DELEGATE_DIR: &str = "delegate-dir";

/// Scan raw argv for legacy (pre-0.1.5) subcommand and flag names and emit a
/// one-shot deprecation warning before clap parses. Clap's `Command::alias`
/// and `Arg::alias` report the canonical name in matches, so this pre-scan is
/// the only way to notice which spelling the user typed.
fn warn_on_legacy_argv() {
    let legacy_tokens = [LEGACY_CMD_GENERATE_DELEGATE, LEGACY_CMD_VERIFY_DELEGATE];
    let legacy_flags = ["--delegate-certificate", "--delegate-dir"];

    let args: Vec<String> = std::env::args().collect();
    let mut warned = false;
    for arg in args.iter().skip(1) {
        if legacy_tokens.contains(&arg.as_str()) {
            eprintln!(
                "{}: subcommand '{}' is deprecated and will be removed in 0.2.0. \
                 Use '{}' instead. See freenet/web#24.",
                "warning".yellow(),
                arg,
                arg.replace("delegate", "notary"),
            );
            warned = true;
        }
        if legacy_flags
            .iter()
            .any(|f| arg == f || arg.starts_with(&format!("{}=", f)))
        {
            let flag = arg.split('=').next().unwrap_or(arg);
            eprintln!(
                "{}: flag '{}' is deprecated and will be removed in 0.2.0. \
                 Use '{}' instead. See freenet/web#24.",
                "warning".yellow(),
                flag,
                flag.replace("delegate", "notary"),
            );
            warned = true;
        }
    }
    let _ = warned;
}

fn main() {
    warn_on_legacy_argv();
    let exit_code = run();
    process::exit(exit_code);
}

fn run() -> i32 {
    let matches = Command::new("Freenet Ghost Key Utility")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Ian Clarke <ian@freenet.org>")
        .about("Utility for generating and verifying Freenet ghost keys. Use 'ghostkey <subcommand> -h' for help on specific subcommands.")
        .subcommand(
            Command::new(CMD_VERIFY_GHOST_KEY)
                .about("Verifies a ghost certificate")
                .arg(
                    Arg::new(ARG_MASTER_VERIFYING_KEY)
                        .long(ARG_MASTER_VERIFYING_KEY)
                        .help("Optionally override the master verifying key")
                        .required(false)
                        .value_name("FILE"),
                )
                .arg(
                    Arg::new(ARG_GHOST_CERTIFICATE)
                        .long(ARG_GHOST_CERTIFICATE)
                        .help("The file containing the ghost certificate")
                        .required(true)
                        .value_name("FILE"),
                ),
        )
        .subcommand(
            Command::new(CMD_GENERATE_MASTER_KEY)
                .about("Generate a new master keypair")
                .arg(
                    Arg::new(ARG_OUTPUT_DIR)
                        .long(ARG_OUTPUT_DIR)
                        .help("The directory to output the keys")
                        .required(true)
                        .value_name("DIR"),
                )
                .arg(
                    Arg::new(ARG_IGNORE_PERMISSIONS)
                        .long(ARG_IGNORE_PERMISSIONS)
                        .help("Ignore file permission checks")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new(CMD_GENERATE_NOTARY)
                .alias(LEGACY_CMD_GENERATE_DELEGATE)
                .about("Generates a new notary signing key and certificate")
                .arg(
                    Arg::new(ARG_MASTER_SIGNING_KEY)
                        .long(ARG_MASTER_SIGNING_KEY)
                        .help("The file containing the master signing key")
                        .required(true)
                        .value_name("FILE"),
                )
                .arg(
                    Arg::new(ARG_INFO)
                        .long(ARG_INFO)
                        .help("The info string to be included in the notary certificate")
                        .required(true)
                        .value_name("STRING"),
                )
                .arg(
                    Arg::new(ARG_OUTPUT_DIR)
                        .long(ARG_OUTPUT_DIR)
                        .help("The directory to output the notary keys and certificate")
                        .required(true)
                        .value_name("DIR"),
                )
                .arg(
                    Arg::new(ARG_IGNORE_PERMISSIONS)
                        .long(ARG_IGNORE_PERMISSIONS)
                        .help("Ignore file permission checks")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new(CMD_VERIFY_NOTARY)
                .alias(LEGACY_CMD_VERIFY_DELEGATE)
                .about("Verifies a notary certificate using the master verifying key")
                .arg(
                    Arg::new(ARG_MASTER_VERIFYING_KEY)
                        .long(ARG_MASTER_VERIFYING_KEY)
                        .help("Optionally override the master verifying key")
                        .required(false)
                        .value_name("FILE"),
                )
                .arg(
                    Arg::new(ARG_NOTARY_CERTIFICATE)
                        .long(ARG_NOTARY_CERTIFICATE)
                        .alias(LEGACY_ARG_DELEGATE_CERTIFICATE)
                        .help("The file containing the notary certificate")
                        .required(true)
                        .value_name("FILE"),
                ),
        )
        .subcommand(
            Command::new(CMD_GENERATE_GHOST_KEY)
                .about("Generates a ghost key from a notary signing key")
                .arg(
                    Arg::new(ARG_NOTARY_DIR)
                        .long(ARG_NOTARY_DIR)
                        .alias(LEGACY_ARG_DELEGATE_DIR)
                        .help("The directory containing the notary certificate and signing key")
                        .required(true)
                        .value_name("DIR"),
                )
                .arg(
                    Arg::new(ARG_OUTPUT_DIR)
                        .long(ARG_OUTPUT_DIR)
                        .help("The directory to output the ghost key files")
                        .required(true)
                        .value_name("DIR"),
                ),
        )
        .subcommand(
            Command::new(CMD_SIGN_MESSAGE)
                .about("Signs a message using a ghost key")
                .arg(
                    Arg::new("ghost_certificate")
                        .long("ghost-certificate")
                        .help("The file containing the ghost certificate")
                        .required(true)
                        .value_name("FILE"),
                )
                .arg(
                    Arg::new("ghost_signing_key")
                        .long("ghost-signing-key")
                        .help("The file containing the ghost signing key")
                        .required(true)
                        .value_name("FILE"),
                )
                .arg(
                    Arg::new("message")
                        .long("message")
                        .help("The message to sign (either a file path or a string)")
                        .required(true)
                        .value_name("MESSAGE"),
                )
                .arg(
                    Arg::new("output")
                        .long("output")
                        .help("The file to output the signed message")
                        .required(true)
                        .value_name("FILE"),
                ),
        )
        .subcommand(
            Command::new(CMD_VERIFY_SIGNED_MESSAGE)
                .about("Verifies a signed message")
                .arg(
                    Arg::new("signed_message")
                        .long("signed-message")
                        .help("The file containing the signed message")
                        .required(true)
                        .value_name("FILE"),
                )
                .arg(
                    Arg::new("master_verifying_key")
                        .long("master-verifying-key")
                        .help("Optionally override the master verifying key")
                        .required(false)
                        .value_name("FILE"),
                )
                .arg(
                    Arg::new("output")
                        .long("output")
                        .help("The file to output the verified message (if not provided, the message will be printed to stdout)")
                        .required(false)
                        .value_name("FILE"),
                ),
        )
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
                println!(
                    "{}",
                    "Master key generation completed successfully.".green()
                );
            }
            result
        }
        Some((CMD_GENERATE_NOTARY, sub_matches)) => {
            let master_signing_key_file = Path::new(
                sub_matches
                    .get_one::<String>(ARG_MASTER_SIGNING_KEY)
                    .unwrap(),
            );
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

            let result =
                generate_notary_cmd(&master_signing_key, info, output_dir, ignore_permissions);
            if result == 0 {
                println!(
                    "{}",
                    "Notary key generation completed successfully.".green()
                );
            }
            result
        }
        Some((CMD_VERIFY_NOTARY, sub_matches)) => {
            let master_verifying_key: Option<VerifyingKey> =
                if let Some(key_file) = sub_matches.get_one::<String>(ARG_MASTER_VERIFYING_KEY) {
                    match VerifyingKey::from_file(Path::new(key_file)) {
                        Ok(key) => Some(key),
                        Err(e) => {
                            println!("{} to read master verifying key: {}", "Failed".red(), e);
                            return 1;
                        }
                    }
                } else {
                    None
                };
            let notary_certificate_file = Path::new(
                sub_matches
                    .get_one::<String>(ARG_NOTARY_CERTIFICATE)
                    .unwrap(),
            );
            let notary_certificate = match NotaryCertificateV1::from_file(notary_certificate_file) {
                Ok(cert) => cert,
                Err(e) => {
                    println!("{} to read notary certificate: {}", "Failed".red(), e);
                    return 1;
                }
            };
            verify_notary_cmd(&master_verifying_key, &notary_certificate)
        }
        Some((CMD_GENERATE_GHOST_KEY, sub_matches)) => {
            let notary_dir = sub_matches.get_one::<String>(ARG_NOTARY_DIR).unwrap();
            let notary_dir_path = Path::new(notary_dir);
            let notary_certificate_file = resolve_notary_file(
                notary_dir_path,
                NOTARY_CERT_FILENAME,
                LEGACY_DELEGATE_CERT_FILENAME,
            );
            let notary_certificate = match NotaryCertificateV1::from_file(&notary_certificate_file)
            {
                Ok(cert) => cert,
                Err(e) => {
                    eprintln!("{} to read notary certificate: {}", "Failed".red(), e);
                    return 1;
                }
            };
            let notary_signing_key_file = resolve_notary_file(
                notary_dir_path,
                NOTARY_SIGNING_KEY_FILENAME,
                LEGACY_DELEGATE_SIGNING_KEY_FILENAME,
            );
            let notary_signing_key = match RSASigningKey::from_file(&notary_signing_key_file) {
                Ok(key) => key,
                Err(e) => {
                    eprintln!("{} to read notary signing key: {}", "Failed".red(), e);
                    return 1;
                }
            };

            let output_dir = Path::new(sub_matches.get_one::<String>(ARG_OUTPUT_DIR).unwrap());
            if let Err(e) = std::fs::create_dir_all(output_dir) {
                eprintln!("{} to create output directory: {}", "Failed".red(), e);
                return 1;
            }

            generate_ghost_key_cmd(&notary_certificate, &notary_signing_key, &output_dir)
        }
        Some((CMD_VERIFY_GHOST_KEY, sub_matches)) => {
            let master_verifying_key: Option<VerifyingKey> =
                if let Some(key_file) = sub_matches.get_one::<String>(ARG_MASTER_VERIFYING_KEY) {
                    match VerifyingKey::from_file(Path::new(key_file)) {
                        Ok(key) => Some(key),
                        Err(e) => {
                            eprintln!("{} to read master verifying key: {}", "Failed".red(), e);
                            return 1;
                        }
                    }
                } else {
                    None
                };
            let ghost_certificate_file = Path::new(
                sub_matches
                    .get_one::<String>(ARG_GHOST_CERTIFICATE)
                    .unwrap(),
            );
            let ghost_certificate = match GhostkeyCertificateV1::from_file(ghost_certificate_file) {
                Ok(cert) => cert,
                Err(e) => {
                    eprintln!("{} to read ghost certificate: {}", "Failed".red(), e);
                    return 1;
                }
            };
            verify_ghost_key_cmd(&master_verifying_key, &ghost_certificate)
        }
        Some((CMD_SIGN_MESSAGE, sub_matches)) => {
            let ghost_certificate_file =
                Path::new(sub_matches.get_one::<String>("ghost_certificate").unwrap());
            let ghost_certificate = match GhostkeyCertificateV1::from_file(ghost_certificate_file) {
                Ok(cert) => cert,
                Err(e) => {
                    eprintln!("{} to read ghost certificate: {}", "Failed".red(), e);
                    return 1;
                }
            };
            let ghost_signing_key_file =
                Path::new(sub_matches.get_one::<String>("ghost_signing_key").unwrap());
            let ghost_signing_key = match SigningKey::from_file(ghost_signing_key_file) {
                Ok(key) => key,
                Err(e) => {
                    eprintln!("{} to read ghost signing key: {}", "Failed".red(), e);
                    return 1;
                }
            };
            let message = sub_matches.get_one::<String>("message").unwrap();
            let message_content = if Path::new(message).is_file() {
                match fs::read(message) {
                    Ok(content) => content,
                    Err(e) => {
                        eprintln!("{} to read message file: {}", "Failed".red(), e);
                        return 1;
                    }
                }
            } else {
                message.as_bytes().to_vec()
            };
            let output_file = Path::new(sub_matches.get_one::<String>("output").unwrap());
            sign_message_cmd(
                ghost_certificate,
                &ghost_signing_key,
                &message_content,
                output_file,
            )
        }
        Some((CMD_VERIFY_SIGNED_MESSAGE, sub_matches)) => {
            let signed_message_file =
                Path::new(sub_matches.get_one::<String>("signed_message").unwrap());
            let master_verifying_key =
                if let Some(key_file) = sub_matches.get_one::<String>("master_verifying_key") {
                    match VerifyingKey::from_file(Path::new(key_file)) {
                        Ok(key) => Some(key),
                        Err(e) => {
                            eprintln!("{} to read master verifying key: {}", "Failed".red(), e);
                            return 1;
                        }
                    }
                } else {
                    None
                };
            let output_file = sub_matches
                .get_one::<String>("output")
                .map(|s| Path::new(s));
            verify_signed_message_cmd(signed_message_file, &master_verifying_key, output_file)
        }
        _ => {
            info!("No valid subcommand provided. Use --help for usage information.");
            0
        }
    }
}
