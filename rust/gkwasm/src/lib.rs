use base64::Engine;
use wasm_bindgen::prelude::*;
use js_sys::{Object, Reflect};
use ciborium::ser::into_writer;
use ed25519_dalek::SigningKey as ECSigningKey;
use ed25519_dalek::VerifyingKey as ECVerifyingKey;
use rand::rngs::OsRng;

#[wasm_bindgen]
pub fn generate_keypair_with_blinded() -> Object {
    use blind_rsa_signatures::{KeyPair, Options};
    let options = Options::default();
    let rng = &mut OsRng;

    let ec_signing_key = ECSigningKey::generate(&mut OsRng);
    let ec_verifying_key = ECVerifyingKey::from(&ec_signing_key);

    let ec_signing_key_base64 = base64::prelude::BASE64_STANDARD.encode(ec_signing_key.to_bytes());

    let ec_verifying_key_base64 = base64::prelude::BASE64_STANDARD.encode(ec_verifying_key.to_bytes());

    let blinded_verifying_key = todo!("NEED DELEGATE PUBLIC KEY");

    // Create a javascript object with the rsa_keypair and the blinded_public_key
    let obj = Object::new();
    Reflect::set(&obj, &"ec_signing_key".into(), &ec_signing_key_base64.into()).unwrap();
    Reflect::set(&obj, &"ec_verifying_key".into(), &ec_verifying_key_base64.into()).unwrap();
    
    
    obj
}

/*

  {
     "rsa_keypair": "BASE64 STRING",
     "blinded_public_key": "BLINDED PUBLIC KEY RAW BASE64 STRING"
  }

 */

// Add more functions here to expose CLI functionality as needed
