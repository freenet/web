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
        key_util::generate_signing_key();
    }

}
mod key_util;
