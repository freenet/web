use base64::Engine;
use js_sys::{JsString, Object, Reflect};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use gklib::armorable::Armorable;
use gklib::util::create_keypair;
use blind_rsa_signatures::{BlindSignature, Options, PublicKey, Secret};
use gklib::delegate_certificate::DelegateCertificate;
use gklib::ghostkey_certificate::GhostkeyCertificate;
use base64::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Debug)]
struct KeypairAndBlindResult {
    ec_signing_key: String,
    ec_verifying_key: String,
    blinded_signing_key: String,
    blinding_secret: String,
}

fn generate_keypair_and_blind_core(delegate_certificate_base64: String, seed: Vec<u8>) -> Result<KeypairAndBlindResult, String> {
    // Ensure the seed is exactly 32 bytes
    if seed.len() != 32 {
        return Err("Seed must be exactly 32 bytes".to_string());
    }

    // Initialize the RNG with the provided seed
    let mut rng = ChaCha20Rng::from_seed(seed.try_into().expect("Seed must be 32 bytes"));

    let (ec_signing_key, ec_verifying_key) = match create_keypair(&mut rng) {
        Ok(keypair) => keypair,
        Err(_) => return Err("Failed to create keypair".to_string()),
    };

    let delegate_certificate = match DelegateCertificate::from_base64(&delegate_certificate_base64) {
        Ok(cert) => cert,
        Err(e) => return Err(format!("generate_keypair_and_blind: Invalid delegate certificate from delegate_certificate_base64 parameter: {}", e.to_string()))
    };

    let verifying_key_bytes = match Armorable::to_bytes(&ec_verifying_key) {
        Ok(bytes) => bytes,
        Err(_) => return Err("Failed to convert verifying key to bytes".to_string()),
    };

    println!("generate_keypair_and_blind_core(): Verifying Key Bytes: {:?}", verifying_key_bytes);

    let blinding_result = match delegate_certificate.payload.delegate_verifying_key.blind(&mut rng, verifying_key_bytes, false, &Options::default()) {
        Ok(result) => result,
        Err(_) => return Err("Blinding operation failed".to_string()),
    };

    println!("generate_keypair_and_blind_core(): Blinded Signing Key: {:?}", blinding_result.blind_msg);
    println!("generate_keypair_and_blind_core(): Blinding Secret: {:?}", blinding_result.secret);

    Ok(KeypairAndBlindResult {
        ec_signing_key: ec_signing_key.to_base64().unwrap(),
        ec_verifying_key: ec_verifying_key.to_base64().unwrap(),
        blinded_signing_key: blinding_result.blind_msg.to_base64().unwrap(),
        blinding_secret: BASE64_STANDARD.encode(blinding_result.secret.0),
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_generate_keypair_and_blind(delegate_certificate_base64: String, seed: Vec<u8>) -> JsValue {
    match generate_keypair_and_blind_core(delegate_certificate_base64, seed) {
        Ok(result) => {
            let return_obj = Object::new();
            Reflect::set(&return_obj, &JsString::from("ec_signing_key"), &JsString::from(result.ec_signing_key)).unwrap();
            Reflect::set(&return_obj, &JsString::from("ec_verifying_key"), &JsString::from(result.ec_verifying_key)).unwrap();
            Reflect::set(&return_obj, &JsString::from("blinded_signing_key"), &JsString::from(result.blinded_signing_key)).unwrap();
            Reflect::set(&return_obj, &JsString::from("blinding_secret"), &JsString::from(result.blinding_secret)).unwrap();
            return_obj.into()
        }
        Err(err) => JsValue::from_str(&err),
    }
}

fn generate_ghostkey_certificate_core(
    delegate_certificate_base64: String,
    blinded_signature_base64: String,
    blinding_secret_base64: String,
    ec_verifying_key_base64: String
) -> Result<String, String> {
    let blind_signature = match BlindSignature::from_base64(&blinded_signature_base64) {
        Ok(sig) => sig,
        Err(_) => return Err("Invalid blinded signature".to_string()),
    };

    let delegate_certificate = match DelegateCertificate::from_base64(&delegate_certificate_base64) {
        Ok(cert) => cert,
        Err(e) => return Err(format!("generate_ghostkey: Invalid delegate certificate: {}", e)),
    };

    let delegate_verifying_key = &delegate_certificate.clone().payload.delegate_verifying_key;

    let blinding_secret: Secret = Secret(BASE64_STANDARD.decode(blinding_secret_base64).unwrap());

    let ec_verifying_key = match ed25519_dalek::VerifyingKey::from_base64(&ec_verifying_key_base64) {
        Ok(key) => key,
        Err(_) => return Err("Invalid EC verifying key".to_string()),
    };

    let verifying_key_bytes = match Armorable::to_bytes(&ec_verifying_key) {
        Ok(bytes) => bytes,
        Err(_) => return Err("Failed to convert verifying key to bytes".to_string()),
    };

    println!("generate_ghostkey_certificate_core(): Blinding Secret: {:?}", blinding_secret);
    println!("generate_ghostkey_certificate_core(): Verifying Key Bytes for Finalize: {:?}", verifying_key_bytes);
    println!("generate_ghostkey_certificate_core(): Blind Signature: {:?}", blind_signature);

    let unblinded_signature = match delegate_verifying_key.finalize(
        &blind_signature,
        &blinding_secret,
        None,
        verifying_key_bytes,
        &Options::default(),
    ) {
        Ok(sig) => sig,
        Err(e) => return Err(format!("Unblinding operation failed: {}", e)),
    };

    println!("generate_ghostkey_certificate_core(): Unblinded Signature: {:?}", unblinded_signature);

    let ghostkey_certificate = GhostkeyCertificate {
        delegate: delegate_certificate.clone(),
        verifying_key: ec_verifying_key,
        signature: unblinded_signature,
    };

    Ok(ghostkey_certificate.to_base64().unwrap())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_generate_ghostkey_certificate(
    delegate_certificate_base64: String,
    blinded_signature_base64: String,
    blinding_secret_base64: String,
    ec_verifying_key_base64: String
) -> JsValue {
    match generate_ghostkey_certificate_core(
        delegate_certificate_base64,
        blinded_signature_base64,
        blinding_secret_base64,
        ec_verifying_key_base64,
    ) {
        Ok(cert) => JsValue::from_str(&cert),
        Err(err) => JsValue::from_str(&err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip() {
        let mut rng = ChaCha20Rng::from_seed([0u8; 32]);
        let (master_signing_key, master_verifying_key) = create_keypair(&mut rng).unwrap();
        let (delegate_certificate, delegate_signing_key) = DelegateCertificate::new(&master_signing_key, &"Test Delegate".to_string()).unwrap();

        let delegate_certificate_base64 = delegate_certificate.to_base64().unwrap();

        let seed = [0u8; 32].to_vec();
        let result = generate_keypair_and_blind_core(delegate_certificate_base64.clone(), seed).unwrap();

        println!("EC Signing Key: {}", result.ec_signing_key);
        println!("EC Verifying Key: {}", result.ec_verifying_key);
        println!("Blinded Signing Key: {}", result.blinded_signing_key);
        println!("Blinding Secret: {}", result.blinding_secret);

        let blinded_signing_key : BlindSignature = BlindSignature::from_base64(&result.blinded_signing_key).unwrap();

        let blinded_signature = delegate_signing_key.blind_sign(&mut rng, blinded_signing_key, &Options::default()).unwrap();

        let ghostkey_certificate_base64 = generate_ghostkey_certificate_core(
            delegate_certificate_base64,
            blinded_signature.to_base64().unwrap(),
            result.blinding_secret,
            result.ec_verifying_key,
        ).unwrap();

        let ghostkey_certificate = GhostkeyCertificate::from_base64(&ghostkey_certificate_base64).unwrap();

        let verified = ghostkey_certificate.verify(&master_verifying_key);

        // Assert it's ok or print the error
        assert!(verified.is_ok(), "Verification failed: {:?}", verified.unwrap_err());
    }
}
