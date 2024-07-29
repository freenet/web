use base64::Engine;
use wasm_bindgen::prelude::*;
use js_sys::{JsString, Object, Reflect};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use gklib::armorable::Armorable;
use gklib::util::create_keypair;
use blind_rsa_signatures::{BlindSignature, Options, Secret};
use gklib::delegate_certificate::DelegateCertificate;
use gklib::ghostkey_certificate::GhostkeyCertificate;
use base64::prelude::*;

#[wasm_bindgen]
pub fn generate_keypair_and_blind(delegate_certificate_base64: String, seed: Vec<u8>) -> JsValue {
    // Ensure the seed is exactly 32 bytes
    if seed.len() != 32 {
        return JsValue::from_str("Seed must be exactly 32 bytes");
    }

    // Initialize the RNG with the provided seed
    let mut rng = ChaCha20Rng::from_seed(seed.try_into().expect("Seed must be 32 bytes"));

    let (ec_signing_key, ec_verifying_key) = match create_keypair(&mut rng) {
        Ok(keypair) => keypair,
        Err(_) => return JsValue::from_str("Failed to create keypair"),
    };

    let delegate_certificate = match DelegateCertificate::from_base64(&delegate_certificate_base64) {
        Ok(cert) => cert,
        Err(_) => return JsValue::from_str("Invalid delegate certificate"),
    };

    let verifying_key_bytes = match Armorable::to_bytes(&ec_verifying_key) {
        Ok(bytes) => bytes,
        Err(_) => return JsValue::from_str("Failed to convert verifying key to bytes"),
    };

    let blinding_result = match delegate_certificate.payload.delegate_verifying_key.blind(&mut rng, verifying_key_bytes, false, &Options::default()) {
        Ok(result) => result,
        Err(_) => return JsValue::from_str("Blinding operation failed"),
    };

    // Create a JavaScript object with the ec_signing_key, ec_verifying_key, and blinded_signing_key
    let return_obj = Object::new();
    Reflect::set(&return_obj, &JsString::from("ec_signing_key"), &JsString::from(ec_signing_key.to_base64().unwrap())).unwrap();
    Reflect::set(&return_obj, &JsString::from("ec_verifying_key"), &JsString::from(ec_verifying_key.to_base64().unwrap())).unwrap();
    Reflect::set(&return_obj, &JsString::from("blinded_signing_key"), &JsString::from(blinding_result.blind_msg.to_base64().unwrap())).unwrap();
    // Note: BlindingResult.msg_randomizer should be null so no need to return it
    Reflect::set(&return_obj, &JsString::from("blinding_secret_vec"), &JsString::from(BASE64_STANDARD.encode(blinding_result.secret.0))).unwrap();

    return_obj.into()
}

#[wasm_bindgen]
pub fn generate_ghostkey_certificate(
    delegate_certificate_base64: String,
    blinded_signature_base64: String,
    blinding_secret_vec_base64 : String,
    ec_verifying_key_base64 : String
) -> JsValue {
    let blind_signature = match BlindSignature::from_base64(&blinded_signature_base64) {
        Ok(sig) => sig,
        Err(_) => return "Invalid blinded signature".into(),
    };



    let delegate_certificate = match DelegateCertificate::from_base64(&delegate_certificate_base64) {
        Ok(cert) => cert,
        Err(_) => return "Invalid delegate certificate".into(),
    };

    let delegate_verifying_key = &delegate_certificate.clone().payload.delegate_verifying_key;

    let blinding_secret : Secret = Secret(BASE64_STANDARD.decode(blinding_secret_vec_base64.as_bytes()).unwrap());

    let ec_verifying_key = match Armorable::from_base64(&ec_verifying_key_base64) {
        Ok(key) => key,
        Err(_) => return "Invalid EC verifying key".into(),
    };

    let verifying_key_bytes = match Armorable::to_bytes(&ec_verifying_key) {
        Ok(bytes) => bytes,
        Err(_) => return JsValue::from_str("Failed to convert verifying key to bytes"),
    };

    let unblinded_signature = delegate_verifying_key.finalize(
        &blind_signature,
        &blinding_secret,
        None,
        verifying_key_bytes,
        &Options::default()
    ).unwrap();

    let ghostkey_certificate = GhostkeyCertificate {
        delegate: delegate_certificate.clone(),
        verifying_key: ec_verifying_key,
        signature: unblinded_signature,
    };

    JsValue::from_str(&ghostkey_certificate.to_base64().unwrap())
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip() {
        let mut rng = ChaCha20Rng::from_seed([0u8; 32]);
        let (master_signing_key, master_verifying_key) = create_keypair(&mut rng).unwrap();
        let delegate_certificate = DelegateCertificate::new(&master_signing_key, &"Test Delegate".to_string()).unwrap();
        let seed = [0u8; 32].to_vec();
        let result = generate_keypair_and_blind(delegate_certificate.to_base64().unwrap(), seed);
        assert!(result.is_object());
        // extract fields of the object
        let result_obj = Object::from(result);
        let ec_signing_key = Reflect::get(&result_obj, &JsString::from("ec_signing_key")).unwrap();
        let ec_verifying_key = Reflect::get(&result_obj, &JsString::from("ec_verifying_key")).unwrap();
        let blinded_signing_key = Reflect::get(&result_obj, &JsString::from("blinded_signing_key")).unwrap();
        let blinding_secret_vec = Reflect::get(&result_obj, &JsString::from("blinding_secret_vec")).unwrap();

        let ghostkey_certificate_base64 = generate_ghostkey_certificate(
            delegate_certificate.to_base64().unwrap(),
            blinded_signing_key.as_string().unwrap(),
            blinding_secret_vec.as_string().unwrap(),
            ec_verifying_key.as_string().unwrap()
        );

        let ghostkey_certificate = GhostkeyCertificate::from_base64(&ghostkey_certificate_base64.as_string().unwrap()).unwrap();

        assert!(ghostkey_certificate.verify(&master_verifying_key).is_ok());
    }

}