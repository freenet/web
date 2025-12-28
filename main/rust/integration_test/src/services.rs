use anyhow::{Context, Result};
use std::process::{Command, Stdio, Child};
use std::path::Path;
use std::time::{Duration, Instant};
use std::thread;
use std::io::{BufRead, BufReader};
use tokio::time::sleep;
use colored::*;

const API_PORT: u16 = 8000;
const API_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn start_services(temp_dir: &Path) -> Result<(Child, Child, Option<Child>)> {
    let chromedriver_handle = start_chromedriver_if_needed().await?;
    kill_process_if_running(1313, "Hugo").await?;
    let hugo_handle = start_hugo()?;
    kill_process_if_running(API_PORT, "API").await?;
    let api_handle = start_api(temp_dir).await?;
    Ok((hugo_handle, api_handle, chromedriver_handle))
}

async fn start_chromedriver_if_needed() -> Result<Option<Child>> {
    if !is_port_in_use(9515) {
        let handle = start_chromedriver()?;
        sleep(Duration::from_secs(2)).await;
        Ok(Some(handle))
    } else {
        Ok(None)
    }
}

async fn kill_process_if_running(port: u16, _process_name: &str) -> Result<()> {
    if is_port_in_use(port) {
        kill_process_on_port(port)?;
        sleep(Duration::from_secs(2)).await;
    }
    Ok(())
}

pub async fn cleanup_processes(hugo_handle: &mut Child, api_handle: &mut Child, chromedriver_handle: Option<Child>) {
    kill_process(hugo_handle, "Hugo");
    kill_process(api_handle, "API");
    if let Some(mut handle) = chromedriver_handle {
        kill_process(&mut handle, "ChromeDriver");
    }
}

fn kill_process(handle: &mut Child, process_name: &str) {
    if let Err(e) = handle.kill() {
        eprintln!("Failed to kill {} process: {}", process_name, e);
    }
}

fn is_port_in_use(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_err()
}

fn kill_process_on_port(port: u16) -> Result<()> {
    let output = Command::new("lsof")
        .args(&["-t", "-i", &format!(":{}", port)])
        .output()?;
    let pid = String::from_utf8(output.stdout)?.trim().to_string();
    if !pid.is_empty() {
        Command::new("kill").arg(&pid).output()?;
    }
    Ok(())
}

fn start_hugo() -> Result<Child> {
    Command::new("hugo")
        .args(&["server", "--disableFastRender"])
        .current_dir("../../hugo-site")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start Hugo")
}

async fn start_api(temp_dir: &Path) -> Result<Child> {
    let delegate_dir = temp_dir.join("delegates");
    let mut child = Command::new("cargo")
        .args(&["run", "--manifest-path", "../api/Cargo.toml", "--", "--delegate-dir", delegate_dir.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn API process")?;

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    let stdout_thread = thread::spawn(move || {
        stdout_reader.lines().collect::<Result<Vec<_>, _>>().unwrap_or_default()
    });

    let stderr_thread = thread::spawn(move || {
        stderr_reader.lines().collect::<Result<Vec<_>, _>>().unwrap_or_default()
    });

    let start_time = Instant::now();
    while start_time.elapsed() < API_STARTUP_TIMEOUT {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = stdout_thread.join().unwrap();
                let stderr = stderr_thread.join().unwrap();
                println!("{}", "Failed".red());
                println!("API process exited unexpectedly with status: {}", status);
                println!("API stdout:");
                stdout.iter().for_each(|line| println!("{}", line));
                println!("API stderr:");
                stderr.iter().for_each(|line| eprintln!("{}", line));
                return Err(anyhow::anyhow!("API process exited unexpectedly"));
            }
            Ok(None) => {
                if is_api_ready().await.is_ok() {
                    return Ok(child);
                }
            }
            Err(e) => {
                let stdout = stdout_thread.join().unwrap();
                let stderr = stderr_thread.join().unwrap();
                println!("{}", "Failed".red());
                println!("Error checking API process status: {}", e);
                println!("API stdout:");
                stdout.iter().for_each(|line| println!("{}", line));
                println!("API stderr:");
                stderr.iter().for_each(|line| eprintln!("{}", line));
                return Err(anyhow::anyhow!("Error checking API process status"));
            }
        }
        sleep(Duration::from_millis(500)).await;
    }

    let stdout = stdout_thread.join().unwrap();
    let stderr = stderr_thread.join().unwrap();
    println!("{}", "Failed".red());
    println!("API failed to start within the timeout period");
    println!("API stdout:");
    stdout.iter().for_each(|line| println!("{}", line));
    println!("API stderr:");
    stderr.iter().for_each(|line| eprintln!("{}", line));
    Err(anyhow::anyhow!("API failed to start within the timeout period"))
}

async fn is_api_ready() -> Result<()> {
    let client = reqwest::Client::new();
    client.get(&format!("http://localhost:{}/health", API_PORT))
        .send()
        .await
        .context("Failed to connect to API health endpoint")?
        .error_for_status()
        .context("API health check failed")?;
    Ok(())
}

fn start_chromedriver() -> Result<Child> {
    Command::new("chromedriver")
        .arg("--port=9515")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start ChromeDriver")
}
