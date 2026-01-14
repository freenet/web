use base64::Engine;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use ghostkey_lib::armorable::Armorable;
use ghostkey_lib::util::create_keypair;
use ghostkey_lib::signed_message::SignedMessage;
use ghostkey_lib::FREENET_MASTER_VERIFYING_KEY_BASE64;
use blind_rsa_signatures::{BlindSignature, Options, Secret};
use ghostkey_lib::delegate_certificate::DelegateCertificateV1;
use ghostkey_lib::ghost_key_certificate::GhostkeyCertificateV1;
use ed25519_dalek::{Signer, Verifier};
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

    #[test]
    fn test_sign_and_verify_message() {
        let mut rng = ChaCha20Rng::from_seed([1u8; 32]);
        let (master_signing_key, master_verifying_key) = create_keypair(&mut rng).unwrap();
        let (delegate_certificate, delegate_signing_key) = DelegateCertificateV1::new(&master_signing_key, &"Test Delegate $50".to_string()).unwrap();

        // Generate ghostkey
        let delegate_certificate_base64 = delegate_certificate.to_base64().unwrap();
        let seed = [1u8; 32].to_vec();
        let keypair_result = generate_keypair_and_blind_core(delegate_certificate_base64.clone(), seed).unwrap();

        let blinded_signing_key = BlindSignature::from_base64(&keypair_result.blinded_signing_key).unwrap();
        let blinded_signature = delegate_signing_key.blind_sign(&mut rng, blinded_signing_key, &Options::default()).unwrap();

        let cert_result = generate_ghost_key_certificate_core(
            delegate_certificate_base64,
            blinded_signature.to_base64().unwrap(),
            keypair_result.blinding_secret,
            keypair_result.ec_verifying_key,
            keypair_result.ec_signing_key.clone(),
        ).unwrap();

        // Sign a message
        let message = b"Hello, Freenet!".to_vec();
        let signed_message_armored = sign_message_core(
            cert_result.armored_ghost_key_cert.clone(),
            cert_result.armored_ghost_key_signing_key.clone(),
            message.clone(),
        ).unwrap();

        // Verify the signed message
        let master_key_base64 = master_verifying_key.to_base64().unwrap();
        let verify_result = verify_signed_message_core(
            signed_message_armored,
            Some(master_key_base64),
        ).unwrap();

        assert!(verify_result.valid);
        assert_eq!(verify_result.info, "Test Delegate $50");
        assert_eq!(verify_result.message, message);
    }

    #[test]
    fn test_sign_with_wrong_key_fails() {
        let mut rng = ChaCha20Rng::from_seed([2u8; 32]);
        let (master_signing_key, _master_verifying_key) = create_keypair(&mut rng).unwrap();
        let (delegate_certificate, delegate_signing_key) = DelegateCertificateV1::new(&master_signing_key, &"Test".to_string()).unwrap();

        // Generate ghostkey
        let delegate_certificate_base64 = delegate_certificate.to_base64().unwrap();
        let seed = [2u8; 32].to_vec();
        let keypair_result = generate_keypair_and_blind_core(delegate_certificate_base64.clone(), seed).unwrap();

        let blinded_signing_key = BlindSignature::from_base64(&keypair_result.blinded_signing_key).unwrap();
        let blinded_signature = delegate_signing_key.blind_sign(&mut rng, blinded_signing_key, &Options::default()).unwrap();

        let cert_result = generate_ghost_key_certificate_core(
            delegate_certificate_base64,
            blinded_signature.to_base64().unwrap(),
            keypair_result.blinding_secret,
            keypair_result.ec_verifying_key,
            keypair_result.ec_signing_key.clone(),
        ).unwrap();

        // Try to sign with a different key
        let (wrong_signing_key, _) = create_keypair(&mut rng).unwrap();
        let wrong_key_armored = wrong_signing_key.to_armored_string().unwrap();

        let result = sign_message_core(
            cert_result.armored_ghost_key_cert,
            wrong_key_armored,
            b"test".to_vec(),
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not match"));
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct GhostKeyCertificateResult {
    armored_ghost_key_cert: String,
    armored_ghost_key_signing_key: String,
}

// ============================================================================
// Message Signing and Verification
// ============================================================================

#[allow(dead_code)]
fn sign_message_core(
    ghost_certificate_armored: String,
    ghost_signing_key_armored: String,
    message: Vec<u8>,
) -> Result<String, String> {
    // Parse the certificate
    let certificate = GhostkeyCertificateV1::from_armored_string(&ghost_certificate_armored)
        .map_err(|e| format!("Invalid ghost certificate: {}", e))?;

    // Parse the signing key
    let signing_key = ed25519_dalek::SigningKey::from_armored_string(&ghost_signing_key_armored)
        .map_err(|e| format!("Invalid signing key: {}", e))?;

    // Verify the signing key matches the certificate
    if signing_key.verifying_key() != certificate.verifying_key {
        return Err("Signing key does not match certificate".to_string());
    }

    // Sign the message
    let signature = signing_key.sign(&message);

    // Create the signed message
    let signed_message = SignedMessage {
        certificate,
        message,
        signature,
    };

    // Return armored signed message
    signed_message.to_armored_string()
        .map_err(|e| format!("Failed to armor signed message: {}", e))
}

/// Sign a message with a ghostkey.
///
/// Takes armored certificate and signing key PEM strings, plus the message bytes.
/// Returns an armored SignedMessage PEM string.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_sign_message(
    ghost_certificate_armored: String,
    ghost_signing_key_armored: String,
    message: Vec<u8>,
) -> Result<JsValue, JsValue> {
    match sign_message_core(ghost_certificate_armored, ghost_signing_key_armored, message) {
        Ok(armored) => Ok(JsValue::from_str(&armored)),
        Err(err) => Err(JsValue::from_str(&err)),
    }
}

#[allow(dead_code)]
struct VerifyResult {
    valid: bool,
    info: String,
    message: Vec<u8>,
}

#[allow(dead_code)]
fn verify_signed_message_core(
    signed_message_armored: String,
    master_verifying_key_base64: Option<String>,
) -> Result<VerifyResult, String> {
    // Parse the signed message
    let signed_message = SignedMessage::from_armored_string(&signed_message_armored)
        .map_err(|e| format!("Invalid signed message: {}", e))?;

    // Determine which master key to use
    let master_key = match master_verifying_key_base64 {
        Some(key) => Some(
            ed25519_dalek::VerifyingKey::from_base64(&key)
                .map_err(|e| format!("Invalid master verifying key: {}", e))?
        ),
        None => Some(
            ed25519_dalek::VerifyingKey::from_base64(FREENET_MASTER_VERIFYING_KEY_BASE64)
                .map_err(|e| format!("Invalid default master key: {}", e))?
        ),
    };

    // Verify the certificate chain (master -> delegate -> ghostkey)
    let info = signed_message.certificate.verify(&master_key)
        .map_err(|e| format!("Certificate verification failed: {}", e))?;

    // Verify the message signature
    signed_message.certificate.verifying_key
        .verify(&signed_message.message, &signed_message.signature)
        .map_err(|_| "Message signature verification failed".to_string())?;

    Ok(VerifyResult {
        valid: true,
        info,
        message: signed_message.message,
    })
}

/// Verify a signed message.
///
/// Takes an armored SignedMessage PEM string and optional master verifying key.
/// If no master key is provided, uses the default Freenet master key.
///
/// Returns an object with:
/// - valid: boolean (always true if no error)
/// - info: string (delegate info from certificate)
/// - message: Uint8Array (the original message bytes)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_verify_signed_message(
    signed_message_armored: String,
    master_verifying_key_base64: Option<String>,
) -> Result<JsValue, JsValue> {
    match verify_signed_message_core(signed_message_armored, master_verifying_key_base64) {
        Ok(result) => {
            let return_obj = js_sys::Object::new();
            js_sys::Reflect::set(&return_obj, &JsValue::from_str("valid"), &JsValue::TRUE).unwrap();
            js_sys::Reflect::set(&return_obj, &JsValue::from_str("info"), &JsValue::from_str(&result.info)).unwrap();
            js_sys::Reflect::set(
                &return_obj,
                &JsValue::from_str("message"),
                &js_sys::Uint8Array::from(&result.message[..])
            ).unwrap();
            Ok(JsValue::from(return_obj))
        }
        Err(err) => Err(JsValue::from_str(&err)),
    }
}
