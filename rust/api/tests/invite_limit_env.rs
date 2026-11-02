//! `INVITES_PER_IP_PER_DAY` has to reach the running service. The unit tests in
//! `rate_limit.rs` and `routes.rs` set the limit directly, so they stay green
//! if the clap argument or its plumbing through `main.rs` is broken.

use ed25519_dalek::SigningKey;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

fn gkapi(dir: &std::path::Path) -> Command {
    let notary = dir.join("notary");
    std::fs::create_dir_all(&notary).unwrap();
    let signing_key = dir.join("room_signing_key.bin");
    std::fs::write(&signing_key, [7u8; 32]).unwrap();
    let owner = SigningKey::from_bytes(&[9; 32]).verifying_key();

    let mut command = Command::new(env!("CARGO_BIN_EXE_ghostkey-api"));
    command
        .current_dir(dir)
        .args(["--port", "0", "--notary-dir", notary.to_str().unwrap()])
        .env_remove("INVITES_PER_IP_PER_DAY")
        .env("ROOM_SIGNING_KEY_FILE", signing_key)
        .env(
            "ROOM_OWNER_VK",
            bs58::encode(owner.as_bytes()).into_string(),
        )
        .env("RATE_LIMIT_FILE", dir.join("rate_limits.json"))
        .env("TOR_EXIT_CACHE", dir.join("tor_exits.txt"))
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}

/// Start gkapi and wait for the line reporting the limit it is enforcing.
fn reported_limit(mut command: Command) -> Option<String> {
    let mut child = command.spawn().expect("failed to spawn ghostkey-api");
    let stderr = child.stderr.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let line = BufReader::new(stderr)
            .lines()
            .map_while(Result::ok)
            .find(|line| line.contains("Per-IP invite limit"));
        let _ = sender.send(line);
    });
    let line = receiver
        .recv_timeout(Duration::from_secs(30))
        .ok()
        .flatten();
    child.kill().unwrap();
    child.wait().unwrap();
    line
}

#[test]
fn env_var_sets_the_per_ip_limit() {
    let dir = tempfile::tempdir().unwrap();
    let mut command = gkapi(dir.path());
    command.env("INVITES_PER_IP_PER_DAY", "7");

    let line = reported_limit(command).expect("gkapi never reported its limit");
    assert!(line.contains("limit: 7 per 24 hours"), "got: {line}");
}

#[test]
fn per_ip_limit_defaults_to_four_when_unset() {
    let dir = tempfile::tempdir().unwrap();

    let line = reported_limit(gkapi(dir.path())).expect("gkapi never reported its limit");
    assert!(line.contains("limit: 4 per 24 hours"), "got: {line}");
}

/// The global ceiling uses 0 to mean "off". Here it would refuse everyone, so
/// it must stop the service at startup rather than take invites down quietly.
#[test]
fn zero_per_ip_limit_is_refused_at_startup() {
    let dir = tempfile::tempdir().unwrap();
    let mut command = gkapi(dir.path());
    command.env("INVITES_PER_IP_PER_DAY", "0");

    let output = command.output().expect("failed to run ghostkey-api");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invites-per-ip-per-day"), "got: {stderr}");
}
