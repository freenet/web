use std::process;
use clap::{Command, Arg, ArgAction};
use std::path::Path;
use colored::Colorize;
use log::{info, error};
use p256::ecdsa::SigningKey;
use ghostkey::armorable::Armorable;
use ghostkey::commands::{generate_delegate_cmd, generate_master_key_cmd};
use ghostkey::wrappers::signing_key::SerializableSigningKey;

fn main() {
    let exit_code = run();
    process::exit(exit_code.try_into().unwrap());
}

fn run() -> u32 {
    let matches = Command::new("Freenet Ghost Key Utility")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Performs various ghost key-related tasks")
        .subcommand(Command::new("generate-master-key")
            .about("Generates a new SERVER_MASTER_KEY and public key")
            .arg(Arg::new("output-dir")
                .long("output-dir")
                .help("The directory to output the keys")
                .required(true)
                .value_name("DIR"))
            .arg(Arg::new("ignore-permissions")
                .long("ignore-permissions")
                .help("Ignore file permission checks")
                .action(ArgAction::SetTrue)))
        .subcommand(Command::new("generate-delegate")
            .about("Generates a new delegate signing key and certificate")
            .arg(Arg::new("master-signing-key")
                .long("master-signing-key")
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
        .subcommand(Command::new("verify-delegate-key")
            .about("Verifies a delegate key certificate using the master verifying key")
            .arg(Arg::new("master-verifying-key")
                .long("master-verifying-key")
                .help("The file containing the master verifying key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("delegate-certificate")
                .long("delegate-certificate")
                .help("The file containing the delegate certificate")
                .required(true)
                .value_name("FILE")))
        .subcommand(Command::new("generate-verifying-key")
            .about("Generates a verifying key from a master signing key")
            .arg(Arg::new("master-signing-key")
                .long("master-signing-key")
                .help("The file containing the master signing key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("output")
                .long("output")
                .help("The file to output the master verifying key")
                .required(true)
                .value_name("FILE")))
        .subcommand(Command::new("generate-ghost-key")
            .about("Generates a ghost key from a delegate signing key")
            .arg(Arg::new("delegate-dir")
                .long("delegate-dir")
                .help("The directory containing the delegate certificate and signing key")
                .required(true)
                .value_name("DIR"))
            .arg(Arg::new("output-dir")
                .long("output-dir")
                .help("The directory to output the ghost key files")
                .required(true)
                .value_name("DIR"))
            .arg(Arg::new("overwrite")
                .long("overwrite")
                .help("Overwrite existing ghost key file if it exists")
                .action(ArgAction::SetTrue)))
        .subcommand(Command::new("verify-ghost-key")
            .about("Verifies a ghost key certificate using the master verifying key")
            .arg(Arg::new("master-verifying-key")
                .long("master-verifying-key")
                .help("The file containing the master verifying key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("ghost-certificate")
                .long("ghost-certificate")
                .help("The file containing the ghost key certificate")
                .required(true)
                .value_name("FILE")))
        .get_matches();

    match matches.subcommand() {
        Some(("generate-master-key", sub_matches)) => {
            let output_dir = Path::new(sub_matches.get_one::<String>("output-dir").unwrap());
            let ignore_permissions = sub_matches.get_flag("ignore-permissions");

            generate_master_key_cmd(output_dir, ignore_permissions)
        }
        Some(("generate-delegate", sub_matches)) => {
            let master_signing_key_file = Path::new(sub_matches.get_one::<String>("master-signing-key").unwrap());
            let serializable_signing_key = match SerializableSigningKey::from_file(master_signing_key_file) {
                Ok(key) => key,
                Err(e) => {
                    error!("{} {}", "Failed to read master signing key:".red(), e);
                    return 1;
                }
            };
            let master_signing_key: &SigningKey = serializable_signing_key.as_signing_key();            
            let info = sub_matches.get_one::<String>("info").unwrap();
            let output_dir = Path::new(sub_matches.get_one::<String>("output-dir").unwrap());
            let ignore_permissions = sub_matches.get_flag("ignore-permissions");
            generate_delegate_cmd(master_signing_key, info, output_dir, ignore_permissions)
        }
        Some(("verify-delegate-key", sub_matches)) => {
            let _master_verifying_key_file = sub_matches.get_one::<String>("master-verifying-key").unwrap();
            let _delegate_certificate_file = sub_matches.get_one::<String>("delegate-certificate").unwrap();
            error!("verify-delegate-key command not implemented yet");
            1
        }
        Some(("generate-verifying-key", sub_matches)) => {
            let _master_signing_key_file = sub_matches.get_one::<String>("master-signing-key").unwrap();
            let _output_file = sub_matches.get_one::<String>("output").unwrap();
            error!("generate-verifying-key command not implemented yet");
            1
        }
        Some(("generate-ghost-key", sub_matches)) => {
            let _delegate_dir = sub_matches.get_one::<String>("delegate-dir").unwrap();
            let _output_dir = sub_matches.get_one::<String>("output-dir").unwrap();
            let _overwrite = sub_matches.get_flag("overwrite");
            error!("generate-ghost-key command not implemented yet");
            1
        }
        Some(("verify-ghost-key", sub_matches)) => {
            let _master_verifying_key_file = sub_matches.get_one::<String>("master-verifying-key").unwrap();
            let _ghost_certificate_file = sub_matches.get_one::<String>("ghost-certificate").unwrap();
            error!("verify-ghost-key command not implemented yet");
            1
        }
        _ => {
            info!("No valid subcommand provided. Use --help for usage information.");
            0
        }
    }
}

