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
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("generate-master-key") {
        let output_dir = matches.get_one::<String>("output_dir").unwrap();
        key_util::generate_signing_key(output_dir);
    }

}
mod key_util;
