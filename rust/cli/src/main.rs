use clap::Command;
mod key_util;

fn main() {
    let matches = Command::new("Freenet Key Utility")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Performs various Freenet-related tasks")
        .subcommand(Command::new("generate-key")
            .about("Generates a new key"))
        .get_matches();

    if let Some(_) = matches.subcommand_matches("generate-key") {
        /
    }
}
