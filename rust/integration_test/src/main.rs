use anyhow::{Context, Result};
use std::process::{Command as ProcessCommand, Stdio, Child};
use clap::{Command as ClapCommand, Arg, ArgAction};
use std::time::Duration;
use fantoccini::{Client, ClientBuilder, Locator};
use std::thread;
use std::time::Instant;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use serde::{Deserialize, Serialize};
use colored::*;

const API_PORT: u16 = 8000;
const API_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() -> Result<()> {
    match run().await {
        Ok(_) => {
            println!("Integration test completed successfully");
            Ok(())
        }
        Err(e) => {
            eprintln!("Integration test failed: {}", e);
            Err(e)
        }
    }
}

async fn run() -> Result<()> {
    println!("Starting integration test...");
    let (headless, wait_on_failure, visible) = parse_arguments();
    
    print!("Setting up environment... ");
    let temp_dir = setup_environment().await?;
    println!("{}", "Ok".green());

    print!("Starting services... ");
    let (mut hugo_handle, mut api_handle, chromedriver_handle) = start_services(&temp_dir).await?;
    println!("{}", "Ok".green());

    print!("Setting up delegate keys... ");
    setup_delegate_keys(&temp_dir).context("Failed to setup delegate keys")?;
    println!("{}", "Ok".green());

    print!("Running browser test... ");
    let result = run_browser_test(headless, wait_on_failure, visible, &temp_dir).await;
    match &result {
        Ok(_) => println!("{}", "Ok".green()),
        Err(e) => println!("{}", format!("Failed: {}", e).red()),
    }

    print!("Cleaning up processes... ");
    cleanup_processes(&mut hugo_handle, &mut api_handle, chromedriver_handle).await;
    println!("{}", "Ok".green());

    result
}

fn parse_arguments() -> (bool, bool, bool) {
    let matches = ClapCommand::new("Integration Test")
        .arg(Arg::new("visible")
            .long("visible")
            .help("Run browser in visible mode (non-headless)")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("wait-on-failure")
            .long("wait-on-failure")
            .help("Wait for user input if the test fails")
            .action(ArgAction::SetTrue))
        .get_matches();
    (!matches.get_flag("visible"), matches.get_flag("wait-on-failure"), matches.get_flag("visible"))
}

async fn setup_environment() -> Result<std::path::PathBuf> {
    let temp_dir = env::temp_dir().join("ghostkey_test");
    fs::create_dir_all(&temp_dir)?;
    Ok(temp_dir)
}

async fn start_services(temp_dir: &std::path::Path) -> Result<(Child, Child, Option<Child>)> {
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
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok(Some(handle))
    } else {
        Ok(None)
    }
}

async fn kill_process_if_running(port: u16, process_name: &str) -> Result<()> {
    if is_port_in_use(port) {
        println!("Attempting to kill {} process on port {}", process_name, port);
        kill_process_on_port(port)?;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Ok(())
}

async fn cleanup_processes(hugo_handle: &mut Child, api_handle: &mut Child, chromedriver_handle: Option<Child>) {
    println!("Cleaning up processes...");
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


fn setup_delegate_keys(temp_dir: &std::path::Path) -> Result<()> {
    let delegate_dir = temp_dir.join("delegates");

    print!("Generating master key... ");
    let master_key_file = generate_master_key(temp_dir)?;
    println!("{}", "Ok".green());

    print!("Generating delegate keys... ");
    generate_delegate_keys(&master_key_file, &delegate_dir)?;
    println!("{}", "Ok".green());

    env::set_var("GHOSTKEY_DELEGATE_DIR", delegate_dir.to_str().unwrap());
    Ok(())
}

fn generate_master_key(temp_dir: &std::path::Path) -> Result<std::path::PathBuf> {
    let master_key_file = temp_dir.join("master_signing_key.pem");
    println!("Generating master key in directory: {:?}", temp_dir);
    let cli_dir = std::env::current_dir()?.join("../cli");
    let output = ProcessCommand::new("cargo")
        .args(&["run", "--quiet", "--manifest-path", cli_dir.join("Cargo.toml").to_str().unwrap(), "--", "generate-master-key", "--output-dir"])
        .arg(temp_dir)
        .current_dir(&cli_dir)
        .output()
        .context("Failed to execute generate-master-key command")?;

    if !output.status.success() {
        let error_msg = format!("Failed to generate master key: {}", String::from_utf8_lossy(&output.stderr));
        println!("Error: {}", error_msg);
        println!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
        return Err(anyhow::anyhow!(error_msg));
    }

    Ok(master_key_file)
}

fn generate_delegate_keys(master_key_file: &std::path::Path, delegate_dir: &std::path::Path) -> Result<()> {
    println!("Generating delegate keys in: {:?}", delegate_dir);
    let output = ProcessCommand::new("bash")
        .arg("../cli/generate_delegate_keys.sh")
        .args(&["--master-key", master_key_file.to_str().unwrap()])
        .arg("--delegate-dir")
        .arg(delegate_dir)
        .arg("--overwrite")
        .output()
        .context("Failed to execute generate_delegate_keys.sh")?;

    if !output.status.success() {
        let error_msg = format!(
            "Failed to generate delegate keys. Exit status: {}\nStdout: {}\nStderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        println!("Error: {}", error_msg);
        return Err(anyhow::anyhow!(error_msg));
    }

    println!("Delegate keys generated successfully");
    Ok(())
}

fn is_port_in_use(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_err()
}

fn kill_process_on_port(port: u16) -> Result<()> {
    let output = ProcessCommand::new("lsof")
        .args(&["-t", "-i", &format!(":{}", port)])
        .output()?;
    let pid = String::from_utf8(output.stdout)?.trim().to_string();
    if !pid.is_empty() {
        ProcessCommand::new("kill").arg(&pid).output()?;
    }
    Ok(())
}

fn start_hugo() -> Result<Child> {
    ProcessCommand::new("hugo")
        .args(&["server", "--disableFastRender"])
        .current_dir("../../hugo-site")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start Hugo")
}

async fn start_api(temp_dir: &std::path::Path) -> Result<Child> {
    let delegate_dir = temp_dir.join("delegates");
    let mut child = ProcessCommand::new("cargo")
        .args(&["run", "--manifest-path", "../api/Cargo.toml", "--", "--delegate-dir", delegate_dir.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn API process")?;

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Spawn threads to read stdout and stderr
    let stdout_thread = thread::spawn(move || {
        stdout_reader.lines().collect::<Result<Vec<_>, _>>().unwrap_or_default()
    });

    let stderr_thread = thread::spawn(move || {
        stderr_reader.lines().collect::<Result<Vec<_>, _>>().unwrap_or_default()
    });

    // Wait for the API to start
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
        tokio::time::sleep(Duration::from_millis(500)).await;
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
    ProcessCommand::new("chromedriver")
        .arg("--port=9515")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start ChromeDriver")
}

async fn wait_for_element(client: &Client, locator: Locator<'_>, timeout: Duration) -> Result<fantoccini::elements::Element> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Ok(element) = client.find(locator.clone()).await {
            if element.is_displayed().await? {
                return Ok(element);
            }
        }
        println!("Element not found, waiting... Elapsed time: {:?}", start.elapsed());
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err(anyhow::anyhow!("Timeout waiting for element with selector: {:?}", locator))
}


async fn run_browser_test(_headless: bool, wait_on_failure: bool, visible: bool, temp_dir: &std::path::Path) -> Result<()> {
    let _temp_dir = temp_dir.to_path_buf(); // Convert to owned PathBuf
    use serde_json::json;
    let mut caps = serde_json::map::Map::new();
    let chrome_args = if !visible {
        json!(["--headless", "--disable-gpu"])
    } else {
        json!([])
    };
    caps.insert("goog:chromeOptions".to_string(), json!({"args": chrome_args}));

    let c = ClientBuilder::native()
        .capabilities(caps)
        .connect("http://localhost:9515")
        .await?;

    // Wrap the entire test in a closure
    let test_result = (|| async {
        print!("Navigating to donation page... ");
        c.goto("http://localhost:1313/donate/ghostkey/").await?;
        println!("{}", "Ok".green());

        print!("Filling out donation form... ");
        let form = wait_for_element(&c, Locator::Id("payment-form"), Duration::from_secs(30)).await?;
        let amount_radio = wait_for_element(&c, Locator::Css("input[name='amount'][value='20']"), Duration::from_secs(10)).await?;
        amount_radio.click().await?;
        let currency_select = wait_for_element(&c, Locator::Id("currency"), Duration::from_secs(10)).await?;
        let currency_option = wait_for_element(&currency_select, Locator::Css("option[value='usd']"), Duration::from_secs(10)).await?;
        currency_option.click().await?;
        println!("{}", "Ok".green());

        print!("Filling out Stripe payment form... ");
        let stripe_iframe = wait_for_element(&c, Locator::Css("iframe[name^='__privateStripeFrame']"), Duration::from_secs(30)).await?;
        let iframes = c.find_all(Locator::Css("iframe")).await?;
        let iframe_index = iframes.iter().position(|e| e.element_id() == stripe_iframe.element_id()).unwrap() as u16;
        c.enter_frame(Some(iframe_index)).await?;
        
        // Wait for Stripe form to be fully loaded
        wait_for_element(&c, Locator::Css("input[name='number']"), Duration::from_secs(30)).await?;
        
        let card_number = wait_for_element(&c, Locator::Css("input[name='number']"), Duration::from_secs(10)).await?;
        card_number.send_keys("4242424242424242").await?;
        let card_expiry = wait_for_element(&c, Locator::Css("input[name='expiry']"), Duration::from_secs(10)).await?;
        card_expiry.send_keys("1225").await?;
        let card_cvc = wait_for_element(&c, Locator::Css("input[name='cvc']"), Duration::from_secs(10)).await?;
        card_cvc.send_keys("123").await?;
        let postal_code = wait_for_element(&c, Locator::Css("input[name='postalCode']"), Duration::from_secs(10)).await?;
        postal_code.send_keys("12345").await?;
        c.enter_frame(None).await?;
        println!("{}", "Ok".green());

        print!("Submitting payment... ");
        let submit_button = wait_for_element(&c, Locator::Id("submit"), Duration::from_secs(10)).await?;
        submit_button.click().await?;
        println!("{}", "Ok".green());

        print!("Checking for errors... ");
        if let Ok(error_element) = c.find(Locator::Id("errorMessage")).await {
            if let Ok(error_text) = error_element.text().await {
                if !error_text.trim().is_empty() {
                    println!("{}", "Failed".red());
                    return Err(anyhow::anyhow!("Error occurred on the page: {}", error_text));
                }
            }
        }
        println!("{}", "Ok".green());

        print!("Waiting for ghost key certificate... ");
        let combined_key_result = wait_for_element(&c, Locator::Css("textarea#combinedKey"), Duration::from_secs(120)).await;
        match combined_key_result {
            Ok(_) => println!("{}", "Ok".green()),
            Err(e) => {
                println!("{}", "Failed".red());
                println!("Error: {}", e);
                // Add debugging information
                if let Ok(body) = c.source().await {
                    println!("Page source:\n{}", body);
                }
                if let Ok(url) = c.current_url().await {
                    println!("Current URL: {}", url);
                }
                return Err(anyhow::anyhow!("Failed to find ghost key certificate: {}", e));
            }
        }

        print!("Saving ghost key certificate... ");
        let combined_key_content = c.execute(
            "return document.querySelector('textarea#combinedKey').value;",
            vec![],
        ).await?.as_str().unwrap_or("").to_string();
        let temp_dir = env::temp_dir().join("ghostkey_test");
        let output_file = temp_dir.join("ghostkey_certificate.pem");
        std::fs::write(&output_file, combined_key_content.clone())?;
        println!("{}", "Ok".green());
        
        print!("Verifying ghost key certificate... ");
        let master_verifying_key_file = temp_dir.join("master_verifying_key.pem");
        let output = ProcessCommand::new("cargo")
            .args(&[
                "run",
                "--manifest-path",
                "../cli/Cargo.toml",
                "--",
                "verify-ghost-key",
                "--master-verifying-key",
                master_verifying_key_file.to_str().unwrap(),
                "--ghost-certificate",
                output_file.to_str().unwrap(),
            ])
            .output()?;

        if !output.status.success() {
            println!("{}", "Failed".red());
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("Ghost key validation failed.");
            println!("Stderr: {}", stderr);
            println!("Stdout: {}", stdout);
            let error_details = analyze_validation_error(&stderr, &stdout);
            println!("Detailed error analysis: {}", error_details);
            return Err(anyhow::anyhow!("Ghost key validation failed: {}. Aborting test.", error_details));
        }
        println!("{}", "Ok".green());

        Ok(())
    })().await;

    if test_result.is_err() && wait_on_failure {
        println!("Test failed. Browser window left open for debugging.");
        println!("Press Enter to close the browser and end the test.");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
    }

    c.close().await?;
    test_result
}

#[derive(Debug, Serialize, Deserialize)]
struct CertificateInfo {
    version: u8,
    amount: u64,
    currency: String,
}

fn analyze_validation_error(stderr: &str, stdout: &str) -> String {
    if stderr.contains("Signature verification failed") {
        "The ghost key certificate signature is invalid. This could be due to tampering, use of an incorrect master key, or a mismatch between the certificate data and the signature."
    } else if stderr.contains("Invalid certificate format") {
        "The ghost key certificate has an invalid format. It may be corrupted or not properly encoded."
    } else if stderr.contains("Delegate certificate validation failed") {
        "The delegate certificate within the ghost key certificate is invalid. This could indicate an issue with the delegate key generation or signing process."
    } else if stderr.contains("Invalid ghostkey verifying key") {
        "The ghost key verifying key in the certificate is invalid. This could be due to incorrect key generation or corruption of the certificate."
    } else if stdout.contains("amount mismatch") {
        "The amount in the ghost key certificate does not match the expected value. This could indicate tampering or an error in the certificate generation process."
    } else {
        "An unknown error occurred during ghost key validation. Please check the full error message for more details."
    }.to_string()
}
