use base64::Engine;
use wasm_bindgen::prelude::*;
use js_sys::{Object, Reflect};
use ciborium::ser::into_writer;
use ed25519_dalek::SigningKey as ECSigningKey;
use ed25519_dalek::VerifyingKey as ECVerifyingKey;
use rand::rngs::OsRng;
use ghostkey::armorable::Armorable;

#[wasm_bindgen]
pub fn generate_keypair_with_blinded() -> JsValue {
    use blind_rsa_signatures::{KeyPair, Options};
    let options = Options::default();
    let rng = &mut OsRng;

    let ec_signing_key = ECSigningKey::generate(&mut OsRng);
    let ec_verifying_key = ECVerifyingKey::from(&ec_signing_key);
    

    // Create a javascript object with the rsa_keypair and the blinded_public_key
    let return_obj = Object::new();
    Reflect::set(&return_obj, &"ec_signing_key".into(), &ec_signing_key.to_base64().unwrap().into()).unwrap();
    Reflect::set(&return_obj, &"ec_verifying_key".into(), &ec_verifying_key.to_base64().unwrap().into()).unwrap();
    
    JsValue::from(return_obj)
}

/*

  {
     "rsa_keypair": "BASE64 STRING",
     "blinded_public_key": "BLINDED PUBLIC KEY RAW BASE64 STRING"
  }

 */

// Add more functions here to expose CLI functionality as needed
