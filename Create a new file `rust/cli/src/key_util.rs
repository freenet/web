use rand::RngCore;
use base64::{Engine as _, engine::general_purpose};

pub fn generate_stripe_secret_key() -> String {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    let encoded_key = general_purpose::STANDARD.encode(&key);
    encoded_key.trim_end_matches('=').to_string()
}
use rand::RngCore;
use base64::{Engine as _, engine::general_purpose};

pub fn generate_stripe_secret_key() -> String {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    let encoded_key = general_purpose::STANDARD.encode(&key);
    encoded_key.trim_end_matches('=').to_string()
}
