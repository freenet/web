use clap::Command;

fn main() {
    let matches = Command::new("Freenet Key Utility")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Performs various Freenet-related tasks")
        .subcommand(Command::new("generate-master-key")
            .about("Generates a new SERVER_MASTER_KEY and public key")
            .arg(clap::Arg::new("output_dir")
                .help("The directory to output the keys")
                .required(true)
                .index(1)))
        .subcommand(Command::new("generate-delegate-key")
            .about("Generates a new delegate key and certificate")
            .arg(clap::Arg::new("master_key_dir")
                .help("The directory containing the master keys")
                .required(true)
                .index(1))
            .arg(clap::Arg::new("attributes")
                .help("The attributes string to be included in the delegate key certificate")
                .required(true)
                .index(2))
            .arg(clap::Arg::new("delegate_key_dir")
                .help("The directory to output the delegate keys and certificate")
                .required(true)
                .index(3)))
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("generate-delegate-key") {
        let master_key_dir = matches.get_one::<String>("master_key_dir").unwrap();
        let attributes = matches.get_one::<String>("attributes").unwrap();
        let delegate_key_dir = matches.get_one::<String>("delegate_key_dir").unwrap();
        key_util::generate_delegate_key(master_key_dir, attributes, delegate_key_dir);
    }

    if let Some(matches) = matches.subcommand_matches("generate-master-key") {
        let output_dir = matches.get_one::<String>("output_dir").unwrap();
        key_util::generate_signing_key(output_dir);
    }

}
mod key_util;

use key_util::generate_delegate_key;