//! Startup smoke test for HTTPS mode.
//!
//! # Why this test exists
//!
//! gkapi links `rustls` with BOTH provider features enabled: `aws-lc-rs` (via
//! `axum-server`'s `tls-rustls`) and `ring` (via `reqwest`'s `rustls-tls`).
//! When both are on, rustls 0.23 deliberately refuses to guess and
//! `CryptoProvider::from_crate_features()` returns `None`, so the first TLS
//! construction panics:
//!
//! ```text
//! no process-level CryptoProvider available -- call
//! CryptoProvider::install_default() before this point
//! ```
//!
//! That killed the process at startup in HTTPS mode, taking down every gkapi
//! endpoint (donations and cert-signing, not just invites). It was caught in
//! review, not by tests, because **no unit test constructs a `RustlsConfig`**
//! and `cargo run` without `--tls-cert` takes the plain-HTTP branch. The whole
//! failure lives in the gap between "the binary compiles and its unit tests
//! pass" and "the binary actually serves TLS".
//!
//! So this test spawns the real binary with real TLS material and makes a real
//! HTTPS request. It is deliberately end-to-end: a narrower test would not have
//! caught the bug it exists to prevent.
//!
//! Adding any dependency that enables another `rustls` provider feature will
//! fail here rather than in production.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Minimal self-signed cert/key so the server has something to serve.
/// Generated via `openssl` at test time to avoid committing key material.
fn generate_self_signed(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    let status = Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            key.to_str().unwrap(),
            "-out",
            cert.to_str().unwrap(),
            "-days",
            "1",
            "-nodes",
            "-subj",
            "/CN=localhost",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("openssl must be available to run this test");
    assert!(status.success(), "openssl failed to generate a test cert");
    (cert, key)
}

/// The binary must come up in HTTPS mode and serve a request.
///
/// Pins `install_crypto_provider()` in `main.rs`. Removing that call makes this
/// test fail with the process dying on the CryptoProvider panic.
#[test]
fn https_mode_starts_without_crypto_provider_panic() {
    let dir = tempfile::tempdir().unwrap();
    let (cert, key) = generate_self_signed(dir.path());
    let notary = dir.path().join("notary");
    std::fs::create_dir_all(&notary).unwrap();

    // A high port so the test needs no privileges. Deliberately NOT passing
    // --challenge-dir: that would bind :80 and fail without root, masking what
    // we are actually testing.
    let port = 18443;
    let bin = env!("CARGO_BIN_EXE_ghostkey-api");

    let mut child = Command::new(bin)
        .args([
            "--tls-cert",
            cert.to_str().unwrap(),
            "--tls-key",
            key.to_str().unwrap(),
            "--notary-dir",
            notary.to_str().unwrap(),
            "--port",
            &port.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ghostkey-api");

    // Poll until it serves, or it died.
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut served = false;
    let mut early_exit = None;
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().expect("try_wait failed") {
            early_exit = Some(status);
            break;
        }
        let out = Command::new("curl")
            .args([
                "-sk",
                "--max-time",
                "2",
                &format!("https://127.0.0.1:{port}/health"),
            ])
            .output();
        if let Ok(out) = out {
            if out.status.success() && !out.stdout.is_empty() {
                served = true;
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }

    let _ = child.kill();
    let output = child.wait_with_output().expect("wait failed");
    let stderr = String::from_utf8_lossy(&output.stderr);

    if let Some(status) = early_exit {
        // Surface the actual panic text -- this is the whole point of the test.
        panic!(
            "ghostkey-api exited during HTTPS startup ({status}).\n\
             If this mentions 'no process-level CryptoProvider available', a \
             dependency has enabled a second rustls provider feature and \
             install_crypto_provider() is no longer sufficient.\n\
             stderr:\n{stderr}"
        );
    }

    assert!(
        served,
        "ghostkey-api never served an HTTPS request within 30s.\nstderr:\n{stderr}"
    );

    // Belt and braces: the panic must not appear even if we somehow served.
    assert!(
        !stderr.contains("no process-level CryptoProvider"),
        "rustls CryptoProvider panic present in stderr:\n{stderr}"
    );
    std::io::stdout().flush().ok();
}
