use clap::Command;
use rand::RngCore;
use base64::{Engine as _, engine::general_purpose};

#[allow(dead_code)]
fn main() {
    let matches = Command::new("Freenet Key Utility")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Performs various Freenet-related tasks")
        .subcommand(Command::new("generate-key")
            .about("Generates a new STRIPE_SECRET_KEY"))
        .get_matches();

    if let Some(_) = matches.subcommand_matches("generate-key") {
fn main() {
    let matches = Command::new("Freenet Key Utility")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Performs various Freenet-related tasks")
        .subcommand(Command::new("generate-key")
            .about("Generates a new STRIPE_SECRET_KEY"))
        .get_matches();

    if let Some(_) = matches.subcommand_matches("generate-key") {
        let key = generate_stripe_secret_key();
        println!("Generated STRIPE_SECRET_KEY: {}", key);
    }
}

#[allow(dead_code)]
#[allow(dead_code)]
#[allow(dead_code)]
fn generate_stripe_secret_key() -> String {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    general_purpose::STANDARD.encode(&key)
}

        let key = generate_stripe_secret_key();
        println!("Generated STRIPE_SECRET_KEY: {}", key);
    }
}

fn generate_stripe_secret_key() -> String {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    general_purpose::STANDARD.encode(&key)
}
