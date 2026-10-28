//! A length a CBOR header claims must not decide how much memory parsing
//! reserves: in a contract, linear memory never shrinks, so a 50-byte input
//! that reserved a megabyte would cost it a megabyte for good.
//!
//! Its own test binary, because it installs a counting global allocator.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use ghostkey_verify::GhostkeyCertificate;

struct Largest;

static LARGEST: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Largest {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LARGEST.fetch_max(layout.size(), Ordering::Relaxed);
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new: usize) -> *mut u8 {
        LARGEST.fetch_max(new, Ordering::Relaxed);
        System.realloc(ptr, layout, new)
    }
}

#[global_allocator]
static GLOBAL: Largest = Largest;

/// `{"delegate": {"payload": {"delegate_verifying_key": {"n": [<2^32-1 items>`
/// and nothing more.
fn claims_huge_modulus() -> Vec<u8> {
    let mut b = vec![0xa1, 0x68];
    b.extend_from_slice(b"delegate");
    b.extend_from_slice(&[0xa1, 0x67]);
    b.extend_from_slice(b"payload");
    b.extend_from_slice(&[0xa1, 0x76]);
    b.extend_from_slice(b"delegate_verifying_key");
    b.extend_from_slice(&[0xa1, 0x61, b'n', 0x9a, 0xff, 0xff, 0xff, 0xff]);
    b
}

/// A certificate whose RSA signature claims 2^32-1 bytes, after a complete
/// notary certificate and ghost key.
fn claims_huge_signature() -> Vec<u8> {
    let mut b = vec![0xa3, 0x68];
    b.extend_from_slice(b"delegate");
    b.extend_from_slice(&[0xa2, 0x67]);
    b.extend_from_slice(b"payload");
    b.extend_from_slice(&[0xa2, 0x76]);
    b.extend_from_slice(b"delegate_verifying_key");
    b.extend_from_slice(&[0xa2, 0x61, b'n', 0x81, 0x01, 0x61, b'e', 0x81, 0x03]);
    b.extend_from_slice(&[0x64]);
    b.extend_from_slice(b"info");
    b.push(0x60);
    b.extend_from_slice(&[0x69]);
    b.extend_from_slice(b"signature");
    b.extend_from_slice(&[0x98, 0x40]);
    b.extend(std::iter::repeat_n(0x00, 64));
    b.extend_from_slice(&[0x6d]);
    b.extend_from_slice(b"verifying_key");
    b.extend_from_slice(&[0x58, 0x20]);
    // A valid point: the Ed25519 base point.
    b.extend_from_slice(&[
        0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
        0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
        0x66, 0x66,
    ]);
    b.extend_from_slice(&[0x69]);
    b.extend_from_slice(b"signature");
    b.extend_from_slice(&[0x9a, 0xff, 0xff, 0xff, 0xff]);
    b
}

#[test]
fn claimed_lengths_do_not_reserve_memory() {
    for (name, input) in [
        ("modulus", claims_huge_modulus()),
        ("signature", claims_huge_signature()),
    ] {
        LARGEST.store(0, Ordering::Relaxed);
        assert!(GhostkeyCertificate::from_cbor(&input).is_err(), "{name}");
        let largest = LARGEST.load(Ordering::Relaxed);
        // Room for the largest genuine sequence (128 u32 digits, 512
        // signature bytes) and ciborium's scratch, nowhere near serde's 1 MiB.
        assert!(
            largest <= 16 * 1024,
            "{name}: parsing {} bytes reserved {largest} bytes",
            input.len()
        );
    }
}
