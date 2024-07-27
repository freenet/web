use wasm_bindgen::prelude::*;
use js_sys::Object;
use blind_rsa_signatures::{KeyPair, Options};

#[wasm_bindgen]
pub fn generate_keypair_with_blinded() -> Object {
    
    todo!()
}

/*

  {
     "private_key": "PRIVATE KEY ARMORED BASE64 STRING",
     "blinded_public_key": "BLINDED PUBLIC KEY RAW BASE64 STRING"
  }

 */

// Add more functions here to expose CLI functionality as needed
