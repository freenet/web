use wasm_bindgen::prelude::*;
use js_sys::Object;
use blind_rsa_signatures::{KeyPair, Options};
use blind_rsa_signatures::reexports::rand::rngs::OsRng;

#[wasm_bindgen]
pub fn generate_keypair_with_blinded() -> Object {
    use blind_rsa_signatures::{KeyPair, Options};
    let options = Options::default();
    let rng = &mut rand::thread_rng();

    // [SERVER]: Generate a RSA-2048 key pair
    let kp = KeyPair::generate(rng, 2048).unwrap();
    let (pk, sk) = (kp.pk, kp.sk);

    // [CLIENT]: create a random message and blind it for the server whose public key is `pk`.
    // The client must store the message and the secret.
    let msg = b"test";
    let blinding_result = pk.blind(rng, msg, true, &options).unwrap();

    // [SERVER]: compute a signature for a blind message, to be sent to the client.
    // The client secret should not be sent to the server.
    let blind_sig = sk.blind_sign(rng, &blinding_result.blind_msg, &options).unwrap();

    // [CLIENT]: later, when the client wants to redeem a signed blind message,
    // using the blinding secret, it can locally compute the signature of the
    // original message.
    // The client then owns a new valid (message, signature) pair, and the
    // server cannot link it to a previous(blinded message, blind signature) pair.
    // Note that the finalization function also verifies that the new signature
    // is correct for the server public key.
    let sig = pk.finalize(
        &blind_sig,
        &blinding_result.secret,
        blinding_result.msg_randomizer,
        &msg,
        &options,
    ).unwrap();

    // [SERVER]: a non-blind signature can be verified using the server's public key.
    sig.verify(&pk, blinding_result.msg_randomizer, msg, &options).unwrap();
    
    todo!()
}

/*

  {
     "private_key": "PRIVATE KEY ARMORED BASE64 STRING",
     "blinded_public_key": "BLINDED PUBLIC KEY RAW BASE64 STRING"
  }

 */

// Add more functions here to expose CLI functionality as needed
