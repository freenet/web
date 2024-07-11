use clap::Command;

fn main() {
    let matches = Command::new("Freenet Key Utility")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Performs various Freenet-related tasks")
        .subcommand(Command::new("generate-key")
            .about("Generates a new key"))
        .subcommand(Command::new("generate-signing-key")
            .about("Generates a new SERVER_SIGNING_KEY and public key")
            .arg(clap::Arg::new("output_dir")
                .about("The directory to output the keys")
                .required(true)
                .index(1)))
        .get_matches();

    if let Some(_) = matches.subcommand_matches("generate-key") {
        // Existing generate-key functionality
    } else if let Some(matches) = matches.subcommand_matches("generate-signing-key") {
        let output_dir = matches.value_of("output_dir").unwrap();
        key_util::generate_signing_key(output_dir);
    }

}
mod key_util;
