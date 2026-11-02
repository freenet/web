//! `INVITES_PER_IP_PER_DAY` has to reach the running service. The unit tests in
//! `rate_limit.rs` and `routes.rs` set the limit directly, so they stay green
//! if the clap argument or its plumbing through `main.rs` is broken.
//!
//! These read the startup log rather than getting a request refused. The log
//! reports the value held by the limiter, and the unit tests cover the limiter
//! from there on.

use ed25519_dalek::SigningKey;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn gkapi(dir: &std::path::Path) -> Command {
    let notary = dir.join("notary");
    std::fs::create_dir_all(&notary).unwrap();
    let signing_key = dir.join("room_signing_key.bin");
    std::fs::write(&signing_key, [7u8; 32]).unwrap();
    let owner = SigningKey::from_bytes(&[9; 32]).verifying_key();

    let mut command = Command::new(env!("CARGO_BIN_EXE_ghostkey-api"));
    command
        .current_dir(dir)
        // Nothing inherited: the machine running the tests may set the other
        // invite variables, and this one, to anything.
        .env_clear()
        .args(["--port", "0", "--notary-dir", notary.to_str().unwrap()])
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

/// Start gkapi and collect its log up to the line reporting the limit it is
/// enforcing. Empty if that line never arrives.
fn log_through_limit(mut command: Command) -> Vec<String> {
    let mut child = command.spawn().expect("failed to spawn ghostkey-api");
    let stderr = child.stderr.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut log = Vec::new();
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            log.push(line);
            if log.last().unwrap().contains("Per-IP invite limit") {
                let _ = sender.send(log);
                return;
            }
        }
    });
    let log = receiver
        .recv_timeout(Duration::from_secs(30))
        .unwrap_or_default();
    child.kill().unwrap();
    child.wait().unwrap();
    log
}

fn assert_startup_reports(log: &[String], limit: u32) {
    let line = log.last().expect("gkapi never reported its limit");
    assert!(
        line.contains(&format!("limit: {limit} per 24 hours")),
        "got: {line}"
    );
}

fn warns_about_override(log: &[String]) -> bool {
    log.iter().any(|line| line.contains("not the default of 4"))
}

#[test]
fn startup_log_reports_the_limit_from_the_env_var() {
    let dir = tempfile::tempdir().unwrap();
    let mut command = gkapi(dir.path());
    command.env("INVITES_PER_IP_PER_DAY", "7");

    let log = log_through_limit(command);
    assert_startup_reports(&log, 7);
    assert!(warns_about_override(&log), "got: {log:?}");
}

#[test]
fn highest_accepted_limit_starts() {
    let dir = tempfile::tempdir().unwrap();
    let mut command = gkapi(dir.path());
    command.env("INVITES_PER_IP_PER_DAY", "100");

    assert_startup_reports(&log_through_limit(command), 100);
}

#[test]
fn per_ip_limit_defaults_to_four_when_unset() {
    let dir = tempfile::tempdir().unwrap();

    let log = log_through_limit(gkapi(dir.path()));
    assert_startup_reports(&log, 4);
    assert!(!warns_about_override(&log), "got: {log:?}");
}

/// Run gkapi expecting it to refuse its configuration and exit. Bounded, so a
/// value that is wrongly accepted fails here instead of serving forever.
fn stderr_of_refused_start(value: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let mut command = gkapi(dir.path());
    command.env("INVITES_PER_IP_PER_DAY", value);
    let mut child = command.spawn().expect("failed to spawn ghostkey-api");

    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("gkapi started with INVITES_PER_IP_PER_DAY={value:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(!status.success(), "value {value:?}");
    let output = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The global ceiling uses 0 to mean "off". Here it would refuse everyone, so
/// it must stop the service at startup rather than take invites down quietly.
/// A blank value is what a half-edited unit file produces.
#[test]
fn invalid_per_ip_limits_are_refused_at_startup() {
    for value in ["0", "101", "abc", ""] {
        let stderr = stderr_of_refused_start(value);
        assert!(
            stderr.contains("invites-per-ip-per-day"),
            "value {value:?}, got: {stderr}"
        );
    }
}
