use clap::{Arg, Command};
use rand::Rng;
use rand::distributions::Alphanumeric;

fn main() {
    let matches = Command::new("Freenet Key Utility")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Performs various Freenet-related tasks")
        .subcommand(Command::new("generate-key")
            .about("Generates a new STRIPE_SECRET_KEY"))
        .get_matches();

    if let Some(_) = matches.subcommand_matches("generate-key") {
        generate_stripe_secret_key();
    }
}

fn generate_stripe_secret_key() {
    let key: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    println!("Generated STRIPE_SECRET_KEY: {}", key);
}
