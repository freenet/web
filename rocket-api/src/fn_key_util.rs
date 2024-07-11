use clap::{Command, Arg};
use rand::RngCore;
use base64::{Engine as _, engine::general_purpose};
use p256::{
    ecdsa::{SigningKey, Signature, signature::Signer},
    PublicKey,
};
use rmp_serde::{Serializer, Deserializer};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::fs::File;
use std::io::{Read, Write};

#[derive(Serialize, Deserialize, Debug)]
pub struct DelegatedKeyMetadata {
    pub creation_date: DateTime<Utc>,
    pub purpose: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DelegatedKey {
    pub public_key: PublicKey,
    pub metadata: DelegatedKeyMetadata,
    pub master_signature: Signature,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Certificate {
    pub delegated_key: DelegatedKey,
    pub certified_public_key: PublicKey,
    pub signature: Signature,
}

fn main() {
    let matches = Command::new("Freenet Key Utility")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Performs various Freenet-related tasks")
        .subcommand(Command::new("generate-master-key")
            .about("Generates a new master key"))
        .subcommand(Command::new("generate-delegated-key")
            .about("Generates a new delegated key")
            .arg(Arg::new("purpose")
                .short('p')
                .long("purpose")
                .value_name("PURPOSE")
                .help("Sets the purpose of the delegated key")
                .required(true)))
        .subcommand(Command::new("sign-certificate")
            .about("Signs a certificate with a delegated key")
            .arg(Arg::new("delegated-key")
                .short('d')
                .long("delegated-key")
                .value_name("DELEGATED_KEY_FILE")
                .help("Path to the delegated key file")
                .required(true))
            .arg(Arg::new("public-key")
                .short('p')
                .long("public-key")
                .value_name("PUBLIC_KEY")
                .help("Public key to be certified")
                .required(true)))
        .get_matches();

    match matches.subcommand() {
        Some(("generate-master-key", _)) => {
            let key = generate_master_key();
            println!("Generated master key: {}", general_purpose::STANDARD.encode(&key.to_bytes()));
        },
        Some(("generate-delegated-key", sub_matches)) => {
            let purpose = sub_matches.get_one::<String>("purpose").unwrap();
            let delegated_key = generate_delegated_key(purpose);
            let filename = format!("delegated_key_{}.bin", purpose);
            save_delegated_key(&delegated_key, &filename).unwrap();
            println!("Generated delegated key saved to: {}", filename);
        },
        Some(("sign-certificate", sub_matches)) => {
            let delegated_key_file = sub_matches.get_one::<String>("delegated-key").unwrap();
            let public_key_base64 = sub_matches.get_one::<String>("public-key").unwrap();
            let delegated_key = load_delegated_key(delegated_key_file).unwrap();
            let public_key = PublicKey::from_sec1_bytes(&general_purpose::STANDARD.decode(public_key_base64).unwrap()).unwrap();
            let certificate = sign_certificate(&delegated_key, &public_key);
            let filename = "certificate.bin";
            save_certificate(&certificate, filename).unwrap();
            println!("Certificate saved to: {}", filename);
        },
        _ => println!("No valid subcommand provided"),
    }
}

pub fn generate_master_key() -> SigningKey {
    SigningKey::random(&mut rand::thread_rng())
}

pub fn generate_delegated_key(purpose: &str) -> DelegatedKey {
    let signing_key = SigningKey::random(&mut rand::thread_rng());
    let public_key = signing_key.verifying_key().to_owned();
    
    let metadata = DelegatedKeyMetadata {
        creation_date: Utc::now(),
        purpose: purpose.to_string(),
    };

    let mut buf = Vec::new();
    metadata.serialize(&mut Serializer::new(&mut buf)).unwrap();
    public_key.serialize(&mut Serializer::new(&mut buf)).unwrap();

    let master_key = generate_master_key(); // In practice, this should be loaded from a secure location
    let master_signature = master_key.sign(&buf);

    DelegatedKey {
        public_key,
        metadata,
        master_signature,
    }
}

pub fn sign_certificate(delegated_key: &DelegatedKey, public_key: &PublicKey) -> Certificate {
    let signing_key = SigningKey::from_bytes(&delegated_key.public_key.to_bytes()).unwrap();
    
    let mut buf = Vec::new();
    public_key.serialize(&mut Serializer::new(&mut buf)).unwrap();
    
    let signature = signing_key.sign(&buf);

    Certificate {
        delegated_key: delegated_key.clone(),
        certified_public_key: *public_key,
        signature,
    }
}

pub fn save_delegated_key(key: &DelegatedKey, filename: &str) -> std::io::Result<()> {
    let mut file = File::create(filename)?;
    let mut buf = Vec::new();
    key.serialize(&mut Serializer::new(&mut buf)).unwrap();
    file.write_all(&buf)
}

pub fn load_delegated_key(filename: &str) -> std::io::Result<DelegatedKey> {
    let mut file = File::open(filename)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    let mut de = Deserializer::new(&buf[..]);
    Ok(DelegatedKey::deserialize(&mut de).unwrap())
}

pub fn save_certificate(cert: &Certificate, filename: &str) -> std::io::Result<()> {
    let mut file = File::create(filename)?;
    let mut buf = Vec::new();
    cert.serialize(&mut Serializer::new(&mut buf)).unwrap();
    file.write_all(&buf)
}

pub fn load_certificate(filename: &str) -> std::io::Result<Certificate> {
    let mut file = File::open(filename)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    let mut de = Deserializer::new(&buf[..]);
    Ok(Certificate::deserialize(&mut de).unwrap())
}

pub fn verify_certificate(cert: &Certificate, master_public_key: &PublicKey) -> bool {
    // Verify master signature on delegated key
    let mut buf = Vec::new();
    cert.delegated_key.metadata.serialize(&mut Serializer::new(&mut buf)).unwrap();
    cert.delegated_key.public_key.serialize(&mut Serializer::new(&mut buf)).unwrap();
    
    if !master_public_key.verify(&buf, &cert.delegated_key.master_signature).is_ok() {
        return false;
    }

    // Verify delegated key signature on certified public key
    let mut buf = Vec::new();
    cert.certified_public_key.serialize(&mut Serializer::new(&mut buf)).unwrap();
    
    cert.delegated_key.public_key.verify(&buf, &cert.signature).is_ok()
}
