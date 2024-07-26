use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::edwards::EdwardsPoint;
use rand_core::OsRng;
use sha2::Sha512;

// Function to blind the message
fn blind_message(message: &[u8], blinding_factor: &Scalar) -> Scalar {
    // Convert the message to a scalar
    let message_scalar = Scalar::hash_from_bytes::<Sha512>(message);

    // Blind the message
    message_scalar * blinding_factor
}

// Function to sign the blinded message
fn sign_blinded_message(signing_key: &Scalar, blinded_message: &Scalar) -> (EdwardsPoint, Scalar) {
    // Use the signing key directly for signing without hashing
    let r = Scalar::random(&mut OsRng);
    let r_point = EdwardsPoint::mul_base(&r);
    let k = Scalar::hash_from_bytes::<Sha512>(r_point.compress().as_bytes());
    let s = r + k * signing_key;

    (r_point, s)
}

// Function to unblind the signature
fn unblind_signature(r: EdwardsPoint, s: Scalar, blinding_factor: &Scalar) -> (EdwardsPoint, Scalar) {
    // Compute the inverse of the blinding factor
    let blinding_factor_inv = blinding_factor.invert();
    let s_unblinded = s * blinding_factor_inv;

    (r, s_unblinded)
}

// Function to verify the signature
fn verify_signature(
    public_key: &EdwardsPoint,
    message: &[u8],
    r: EdwardsPoint,
    s: Scalar,
) -> bool {
    let k = Scalar::hash_from_bytes::<Sha512>(r.compress().as_bytes());
    let s_point = EdwardsPoint::mul_base(&s);
    let k_point = public_key * k;

    s_point == r + k_point
}