//! A stand-in for a Freenet contract that verifies Ghost Key proofs, used by
//! CI to measure what linking this crate costs a contract module (its size,
//! and its imports, which must be none) and to run the verification under
//! wasm32, where the bignum arithmetic uses 32-bit limbs and so a different
//! code path from the native tests.
//!
//! It takes its input the way a contract does, through linear memory, so the
//! compiler cannot fold any of the verification away.
//!
//! A crate of its own, not an example, because examples build with the
//! dev-dependencies, and those (ghostkey_lib, for the differential tests)
//! pull in `getrandom`. This crate's dependency tree is exactly what a
//! contract depending on ghostkey-verify gets.
//!
//! ```text
//! cargo build --release --target wasm32-unknown-unknown
//! python3 ../scripts/check_wasm_imports.py target/wasm32-unknown-unknown/release/ghostkey_verify_wasm_probe.wasm
//! node ../scripts/run_wasm_vectors.mjs target/wasm32-unknown-unknown/release/ghostkey_verify_wasm_probe.wasm ../target/tmp/wasm-vectors
//! ```

use ghostkey_verify::{verify_payload_signature, Error, GhostkeyCertificate, VerifyingKey};

/// Reserve `len` bytes for the host to write input into.
#[no_mangle]
pub extern "C" fn probe_alloc(len: usize) -> *mut u8 {
    let mut buf = vec![0u8; len];
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Verify one proof. The input is the 32-byte master key, the armoured
/// certificate's length as a little-endian u32, the certificate, a 64-byte
/// signature by the ghost key, and the signed payload.
///
/// Returns the notary key's creation month shifted left 32 bits when the
/// chain and the payload signature verify and the info parses; otherwise a
/// negative code: -1 to -3 for input framing, else [`code`] of the error.
///
/// # Safety
///
/// `ptr` must point to `len` initialised bytes.
#[no_mangle]
pub unsafe extern "C" fn probe_verify(ptr: *const u8, len: usize) -> i64 {
    let input = std::slice::from_raw_parts(ptr, len);
    match verify(input) {
        Ok(month) => i64::from(month) << 32,
        Err(code) => code,
    }
}

fn verify(input: &[u8]) -> Result<u8, i64> {
    let (master, rest) = split(input, 32)?;
    let (cert_len, rest) = split(rest, 4)?;
    let cert_len = u32::from_le_bytes(cert_len.try_into().unwrap()) as usize;
    let (cert, rest) = split(rest, cert_len)?;
    let (sig, payload) = split(rest, 64)?;

    let master = VerifyingKey::from_bytes(master.try_into().unwrap()).map_err(|_| -2)?;
    let cert = std::str::from_utf8(cert).map_err(|_| -3)?;
    let cert = GhostkeyCertificate::from_armored(cert).map_err(code)?;
    let verified = cert.verify(&master).map_err(code)?;
    verify_payload_signature(&verified.ghost_key, payload, sig).map_err(code)?;
    let info = verified.notary_info().map_err(code)?;
    Ok(info.created_month.month)
}

/// One code per error, so a wasm-only regression that moves a refusal to a
/// different check shows up. `tests/wasm_vectors.rs` keeps the same table.
pub fn code(e: Error) -> i64 {
    match e {
        Error::TooLarge => -10,
        Error::Armor => -11,
        Error::Malformed => -12,
        Error::WeakMasterKey => -13,
        Error::NotarySignature => -14,
        Error::NotaryKey => -15,
        Error::GhostKeySignature => -16,
        Error::WeakGhostKey => -17,
        Error::PayloadSignatureLength => -18,
        Error::PayloadSignature => -19,
        Error::Info => -20,
        _ => -99,
    }
}

fn split(input: &[u8], at: usize) -> Result<(&[u8], &[u8]), i64> {
    if input.len() < at {
        return Err(-1);
    }
    Ok(input.split_at(at))
}
