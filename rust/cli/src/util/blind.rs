// blind.rs

extern crate curve25519_dalek;
extern crate sha2;
extern crate rand;
extern crate serde;
extern crate serde_json;

use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use sha2::{Sha512, Digest};
use rand::rngs::OsRng;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyPair {
    pub sk: Scalar,
    pub pk: RistrettoPoint,
}

pub fn generate_keypair() -> KeyPair {
    let sk = Scalar::random(&mut OsRng);
    let pk = &sk * &RISTRETTO_BASEPOINT_POINT;
    KeyPair { sk, pk }
}

pub fn blind_message(message: &[u8], r: &Scalar, a: &Scalar, b: &Scalar) -> (Scalar, RistrettoPoint) {
    let m = Scalar::hash_from_bytes::<Sha512>(message);
    let R_prime = r * RISTRETTO_BASEPOINT_POINT;
    let R = a * R_prime + b * RISTRETTO_BASEPOINT_POINT;
    let x = Scalar::from_bytes_mod_order(R.compress().as_bytes()[..32].try_into().unwrap());
    let blinded_message = a.invert() * x * m;
    (blinded_message, R)
}

pub fn sign_blinded_message(sk: &Scalar, blinded_message: &Scalar, k: &Scalar) -> Scalar {
    let s_prime = sk * blinded_message + k;
    s_prime
}

pub fn unblind_signature(a: &Scalar, b: &Scalar, blind_signature: &Scalar) -> Scalar {
    a * blind_signature + b
}

pub fn verify_signature(message: &[u8], signature: &Scalar, R: &RistrettoPoint, pk: &RistrettoPoint) -> bool {
    let h = Scalar::hash_from_bytes::<Sha512>(message);
    let x = Scalar::from_bytes_mod_order(R.compress().as_bytes()[..32].try_into().unwrap());
    signature * RISTRETTO_BASEPOINT_POINT == R + x * h * pk
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blind_signature() {
        // Generate keypair
        let keypair = generate_keypair();

        // Original message
        let message = b"Hello, world!";

        // Blinding
        let r = Scalar::random(&mut OsRng);
        let a = Scalar::random(&mut OsRng);
        let b = Scalar::random(&mut OsRng);
        let (blinded_message, R) = blind_message(message, &r, &a, &b);

        // Blind signature
        let k = Scalar::random(&mut OsRng);
        let blind_signature = sign_blinded_message(&keypair.sk, &blinded_message, &k);

        // Unblinding
        let signature = unblind_signature(&a, &b, &blind_signature);

        // Verification
        assert!(verify_signature(message, &signature, &R, &keypair.pk));
    }

    #[test]
    fn test_invalid_signature() {
        // Generate keypair
        let keypair = generate_keypair();

        // Original message
        let message = b"Hello, world!";

        // Blinding
        let r = Scalar::random(&mut OsRng);
        let a = Scalar::random(&mut OsRng);
        let b = Scalar::random(&mut OsRng);
        let (blinded_message, R) = blind_message(message, &r, &a, &b);

        // Blind signature
        let k = Scalar::random(&mut OsRng);
        let blind_signature = sign_blinded_message(&keypair.sk, &blinded_message, &k);

        // Unblinding with wrong factor
        let wrong_a = Scalar::random(&mut OsRng);
        let signature = unblind_signature(&wrong_a, &b, &blind_signature);

        // Verification should fail
        assert!(!verify_signature(message, &signature, &R, &keypair.pk));
    }
}

