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


#[allow(dead_code)]
pub fn generate_master_key() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::random(&mut rand::thread_rng());
    let signing_key_clone = signing_key.clone();
    let verifying_key = signing_key_clone.verifying_key();
    (signing_key, *verifying_key)
}

#[allow(dead_code)]
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

#[allow(dead_code)]
pub fn save_delegated_key(key: &DelegatedKey, filename: &str) -> std::io::Result<()> {
    let mut file = File::create(filename)?;
    let buf = serde_json::to_vec(key).unwrap();
    file.write_all(&buf)
}

#[allow(dead_code)]
pub fn load_delegated_key(filename: &str) -> std::io::Result<DelegatedKey> {
    let mut file = File::open(filename)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(serde_json::from_slice(&buf).unwrap())
}

#[allow(dead_code)]
pub fn save_certificate(cert: &Certificate, filename: &str) -> std::io::Result<()> {
    let mut file = File::create(filename)?;
    let buf = serde_json::to_vec(cert).unwrap();
    file.write_all(&buf)
}

#[allow(dead_code)]
pub fn load_certificate(filename: &str) -> std::io::Result<Certificate> {
    let mut file = File::open(filename)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(serde_json::from_slice(&buf).unwrap())
}

#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

#[allow(dead_code)]
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
    println!("This is a placeholder main function.");
}
