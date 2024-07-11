use clap::{Command, Arg};
use base64::{Engine as _, engine::general_purpose};
use p256::{
    ecdsa::{SigningKey, Signature, signature::Verifier, VerifyingKey, signature::Signer},
    PublicKey,
};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::fs::File;
use std::io::{Read, Write};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DelegatedKeyMetadata {
    pub creation_date: DateTime<Utc>,
    pub purpose: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DelegatedKey {
    pub public_key: Vec<u8>,
    pub metadata: DelegatedKeyMetadata,
    pub master_signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Certificate {
    pub delegated_key: DelegatedKey,
    pub certified_public_key: Vec<u8>,
    pub signature: Vec<u8>,
}


pub fn generate_master_key() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::random(&mut rand::thread_rng());
    let signing_key_clone = signing_key.clone();
    let verifying_key = signing_key_clone.verifying_key();
    (signing_key, *verifying_key)
}

pub fn generate_delegated_key(master_key: &SigningKey, purpose: &str) -> DelegatedKey {
    let signing_key = SigningKey::random(&mut rand::thread_rng());
    let public_key = signing_key.verifying_key().to_sec1_bytes().to_vec();
    
    let metadata = DelegatedKeyMetadata {
        creation_date: Utc::now(),
        purpose: purpose.to_string(),
    };

    let mut buf = Vec::new();
    buf.extend_from_slice(&serde_json::to_vec(&metadata).unwrap());
    buf.extend_from_slice(&public_key);

    let master_signature = <SigningKey as Signer<Signature>>::sign(&master_key, &buf).to_vec();

    DelegatedKey {
        public_key,
        metadata,
        master_signature,
    }
}

pub fn sign_certificate(delegated_key: &DelegatedKey, public_key: &PublicKey) -> Certificate {
    let signing_key = SigningKey::from_slice(&delegated_key.public_key).unwrap();
    
    let signature = <SigningKey as Signer<Signature>>::sign(&signing_key, public_key.to_sec1_bytes().as_ref()).to_vec();

    Certificate {
        delegated_key: delegated_key.clone(),
        certified_public_key: public_key.to_sec1_bytes().to_vec(),
        signature,
    }
}

pub fn save_delegated_key(key: &DelegatedKey, filename: &str) -> std::io::Result<()> {
    let mut file = File::create(filename)?;
    let buf = serde_json::to_vec(key).unwrap();
    file.write_all(&buf)
}

pub fn load_delegated_key(filename: &str) -> std::io::Result<DelegatedKey> {
    let mut file = File::open(filename)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(serde_json::from_slice(&buf).unwrap())
}

pub fn save_certificate(cert: &Certificate, filename: &str) -> std::io::Result<()> {
    let mut file = File::create(filename)?;
    let buf = serde_json::to_vec(cert).unwrap();
    file.write_all(&buf)
}

pub fn load_certificate(filename: &str) -> std::io::Result<Certificate> {
    let mut file = File::open(filename)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(serde_json::from_slice(&buf).unwrap())
}

pub fn verify_certificate(cert: &Certificate, master_public_key: &VerifyingKey) -> bool {
    // Verify master signature on delegated key
    let mut buf = Vec::new();
    buf.extend_from_slice(&serde_json::to_vec(&cert.delegated_key.metadata).unwrap());
    buf.extend_from_slice(&cert.delegated_key.public_key);
    
    if master_public_key.verify(&buf, &Signature::from_slice(&cert.delegated_key.master_signature).unwrap()).is_err() {
        return false;
    }

    // Verify delegated key signature on certified public key
    let delegated_verifying_key = VerifyingKey::from_sec1_bytes(&cert.delegated_key.public_key).unwrap();
    delegated_verifying_key.verify(&cert.certified_public_key, &Signature::from_slice(&cert.signature).unwrap()).is_ok()
}
pub fn load_master_key(filename: &str) -> std::io::Result<SigningKey> {
    let mut file = File::open(filename)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    let armored_key = String::from_utf8(buf).unwrap();
    let key_bytes = unarmor_key("MASTER PRIVATE KEY", &armored_key).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    use p256::elliptic_curve::generic_array::GenericArray;
    use p256::elliptic_curve::consts::U32;

    let key_bytes: &GenericArray<u8, U32> = GenericArray::from_slice(&key_bytes);
    Ok(SigningKey::from_bytes(key_bytes).unwrap())
}
fn armor_key(key_type: &str, key_bytes: &[u8]) -> String {
    let encoded = general_purpose::STANDARD.encode(key_bytes);
    let wrapped = encoded.chars().collect::<Vec<_>>().chunks(64).map(|chunk| chunk.iter().collect::<String>()).collect::<Vec<_>>().join("\n");
    format!(
        "-----BEGIN {}-----\n{}\n-----END {}-----",
        key_type,
        wrapped,
        key_type
    )
}

fn unarmor_key(expected_type: &str, armored_key: &str) -> Result<Vec<u8>, String> {
    let lines: Vec<&str> = armored_key.lines().collect();
    if lines.len() < 3 {
        return Err("Invalid armored key format".to_string());
    }
    if lines[0] != format!("-----BEGIN {}-----", expected_type) || lines[lines.len() - 1] != format!("-----END {}-----", expected_type) {
        return Err("Armored key type mismatch".to_string());
    }
    let key_base64 = lines[1..lines.len() - 1].join("");
    general_purpose::STANDARD.decode(&key_base64).map_err(|e| e.to_string())
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
            .arg(Arg::new("master-key")
                .short('m')
                .long("master-key")
                .value_name("MASTER_KEY")
                .help("Path to the master key file")
                .required(true))
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
            let (master_key, master_public_key) = generate_master_key();
            let armored_master_key = armor_key("MASTER PRIVATE KEY", &master_key.to_bytes());
            let armored_master_public_key = armor_key("MASTER PUBLIC KEY", &master_public_key.to_sec1_bytes());
            println!("Generated master key:\n{}\n{}", armored_master_key, armored_master_public_key);
        },
        Some(("generate-delegated-key", sub_matches)) => {
            let purpose = sub_matches.get_one::<String>("purpose").unwrap();
            let master_key_file = sub_matches.get_one::<String>("master-key").unwrap();
            let master_key = load_master_key(master_key_file).unwrap();
            let delegated_key = generate_delegated_key(&master_key, purpose);
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
    let matches = Command::new("Freenet Key Utility")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Performs various Freenet-related tasks")
        .subcommand(Command::new("generate-master-key")
            .about("Generates a new master key"))
        .subcommand(Command::new("generate-delegated-key")
            .about("Generates a new delegated key")
            .arg(Arg::new("master-key")
                .short('m')
                .long("master-key")
                .value_name("MASTER_KEY")
                .help("Path to the master key file")
                .required(true))
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
            let (master_key, master_public_key) = generate_master_key();
            let armored_master_key = armor_key("MASTER PRIVATE KEY", &master_key.to_bytes());
            let armored_master_public_key = armor_key("MASTER PUBLIC KEY", &master_public_key.to_sec1_bytes());
            println!("Generated master key:\n{}\n{}", armored_master_key, armored_master_public_key);
        },
        Some(("generate-delegated-key", sub_matches)) => {
            let purpose = sub_matches.get_one::<String>("purpose").unwrap();
            let master_key_file = sub_matches.get_one::<String>("master-key").unwrap();
            let master_key = load_master_key(master_key_file).unwrap();
            let delegated_key = generate_delegated_key(&master_key, purpose);
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
