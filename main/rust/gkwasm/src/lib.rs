use base64::Engine;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use ghostkey_lib::armorable::Armorable;
use ghostkey_lib::util::create_keypair;
use blind_rsa_signatures::{BlindSignature, Options, Secret};
use ghostkey_lib::delegate_certificate::DelegateCertificateV1;
use ghostkey_lib::ghost_key_certificate::GhostkeyCertificateV1;
use base64::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use js_sys::{Object, Reflect, JsString};

#[derive(Debug)]
#[allow(dead_code)]
struct KeypairAndBlindResult {
    ec_signing_key: String,
    ec_verifying_key: String,
    blinded_signing_key: String,
    blinding_secret: String,
}

#[allow(dead_code)]
fn generate_keypair_and_blind_core(delegate_certificate_base64: String, seed: Vec<u8>) -> Result<KeypairAndBlindResult, String> {
    if seed.len() != 32 {
        return Err("Seed must be exactly 32 bytes".to_string());
    }

    let mut rng = ChaCha20Rng::from_seed(seed.try_into().expect("Seed must be 32 bytes"));
    let (ec_signing_key, ec_verifying_key) = create_keypair(&mut rng).map_err(|_| "Failed to create keypair".to_string())?;

    let delegate_certificate = DelegateCertificateV1::from_base64(&delegate_certificate_base64)
        .map_err(|e| format!("Invalid delegate certificate: {}", e))?;

    let verifying_key_bytes = Armorable::to_bytes(&ec_verifying_key)
        .map_err(|_| "Failed to convert verifying key to bytes".to_string())?;

    let blinding_result = delegate_certificate.payload.delegate_verifying_key
        .blind(&mut rng, verifying_key_bytes, false, &Options::default())
        .map_err(|_| "Blinding operation failed".to_string())?;

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

#[allow(dead_code)]
fn generate_ghost_key_certificate_core(
    delegate_certificate_base64: String,
    blinded_signature_base64: String,
    blinding_secret_base64: String,
    ec_verifying_key_base64: String,
    ec_signing_key_base64: String
) -> Result<GhostKeyCertificateResult, String> {
    let blind_signature = BlindSignature::from_base64(&blinded_signature_base64)
        .map_err(|_| "Invalid blinded signature".to_string())?;

    let delegate_certificate = DelegateCertificateV1::from_base64(&delegate_certificate_base64)
        .map_err(|e| format!("Invalid delegate certificate: {}", e))?;

    let delegate_verifying_key = &delegate_certificate.clone().payload.delegate_verifying_key;
    let blinding_secret = Secret(BASE64_STANDARD.decode(blinding_secret_base64).unwrap());

    let ec_verifying_key = ed25519_dalek::VerifyingKey::from_base64(&ec_verifying_key_base64)
        .map_err(|_| "Invalid EC verifying key".to_string())?;

    let ec_signing_key = ed25519_dalek::SigningKey::from_base64(&ec_signing_key_base64)
        .map_err(|_| "Invalid EC signing key".to_string())?;

    let verifying_key_bytes = Armorable::to_bytes(&ec_verifying_key)
        .map_err(|_| "Failed to convert verifying key to bytes".to_string())?;

    let unblinded_signature = delegate_verifying_key.finalize(
        &blind_signature,
        &blinding_secret,
        None,
        verifying_key_bytes,
        &Options::default(),
    ).map_err(|e| format!("Unblinding operation failed: {}", e))?;

    let ghost_key_certificate = GhostkeyCertificateV1 {
        delegate: delegate_certificate.clone(),
        verifying_key: ec_verifying_key,
        signature: unblinded_signature,
    };
    
    let armored_certificate = ghost_key_certificate.to_armored_string()
        .map_err(|_| "Failed to armor ghostkey certificate".to_string())?;
    let armored_signing_key = ec_signing_key.to_armored_string()
        .map_err(|_| "Failed to armor signing key".to_string())?;

    Ok(GhostKeyCertificateResult {
        armored_ghost_key_cert: armored_certificate,
        armored_ghost_key_signing_key: armored_signing_key,
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_generate_ghost_key_certificate(
    delegate_certificate_base64: String,
    blinded_signature_base64: String,
    blinding_secret_base64: String,
    ec_verifying_key_base64: String,
    ec_signing_key_base64: String
) -> Result<JsValue, JsValue> {
    match generate_ghost_key_certificate_core(
        delegate_certificate_base64,
        blinded_signature_base64,
        blinding_secret_base64,
        ec_verifying_key_base64,
        ec_signing_key_base64,
    ) {
        Ok(result) => {
            let return_obj = js_sys::Object::new();
            js_sys::Reflect::set(&return_obj, &JsValue::from_str("armored_ghost_key_cert"), &JsValue::from_str(&result.armored_ghost_key_cert)).unwrap();
            js_sys::Reflect::set(&return_obj, &JsValue::from_str("armored_ghost_key_signing_key"), &JsValue::from_str(&result.armored_ghost_key_signing_key)).unwrap();
            Ok(JsValue::from(return_obj))
        },
        Err(err) => Err(JsValue::from_str(&format!("Error: {}", err))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip() {
        let mut rng = ChaCha20Rng::from_seed([0u8; 32]);
        let (master_signing_key, master_verifying_key) = create_keypair(&mut rng).unwrap();
        let (delegate_certificate, delegate_signing_key) = DelegateCertificateV1::new(&master_signing_key, &"Test Delegate".to_string()).unwrap();

        let delegate_certificate_base64 = delegate_certificate.to_base64().unwrap();
        let seed = [0u8; 32].to_vec();
        let result = generate_keypair_and_blind_core(delegate_certificate_base64.clone(), seed).unwrap();

        let blinded_signing_key = BlindSignature::from_base64(&result.blinded_signing_key).unwrap();
        let blinded_signature = delegate_signing_key.blind_sign(&mut rng, blinded_signing_key, &Options::default()).unwrap();

        let generated = generate_ghost_key_certificate_core(
            delegate_certificate_base64,
            blinded_signature.to_base64().unwrap(),
            result.blinding_secret,
            result.ec_verifying_key,
            result.ec_signing_key,
        ).unwrap();

        let ghost_key_certificate = GhostkeyCertificateV1::from_armored_string(&generated.armored_ghost_key_cert).unwrap();
        let verified = ghost_key_certificate.verify(&Some(master_verifying_key));

        assert!(verified.is_ok(), "Verification failed: {:?}", verified.unwrap_err());
        assert_eq!(verified.unwrap(), "Test Delegate");
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct GhostKeyCertificateResult {
    armored_ghost_key_cert: String,
    armored_ghost_key_signing_key: String,
}
