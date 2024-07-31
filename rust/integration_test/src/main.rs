use anyhow::{Context, Result};
use std::process::{Command as ProcessCommand, Stdio, Child};
use clap::{Command as ClapCommand, Arg};
use std::time::Duration;
use fantoccini::{Client, ClientBuilder, Locator};
use std::thread;
use std::time::Instant;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use gklib::armorable::Armorable;
use gklib::delegate_certificate::DelegateCertificate;
use gklib::ghostkey_certificate::GhostkeyCertificate;

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
    let headless = parse_arguments();
    let temp_dir = setup_environment().await?;
    let (mut hugo_handle, mut api_handle, chromedriver_handle) = start_services(&temp_dir).await?;

    // Setup delegate keys
    setup_delegate_keys(&temp_dir).context("Failed to setup delegate keys")?;

    // Run the browser test
    let result = run_browser_test(headless, &temp_dir).await;

    // Keep the browser open for debugging, regardless of the test result
    wait_for_user_input("Test completed. Browser window left open for debugging. Press Enter to close the browser and end the test.");

    // Clean up
    cleanup_processes(&mut hugo_handle, &mut api_handle, chromedriver_handle).await;

    // Return the result of the browser test
    println!("Integration test finished. Result: {:?}", result);
    result
}

fn parse_arguments() -> bool {
    let matches = ClapCommand::new("Integration Test")
        .arg(Arg::new("headless")
            .long("headless")
            .help("Run browser in headless mode"))
        .get_matches();
    matches.contains_id("headless")
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

fn wait_for_user_input(message: &str) {
    println!("{}", message);
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("Failed to read line");
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


fn verify_ghost_key_certificate(cert_file: &std::path::Path, master_key_file: &std::path::Path) -> Result<()> {
    let output = ProcessCommand::new("cargo")
        .args(&[
            "run",
            "--manifest-path",
            "../cli/Cargo.toml",
            "--",
            "verify-ghost-key",
            "--master-verifying-key",
            master_key_file.to_str().unwrap(),
            "--ghost-certificate",
            cert_file.to_str().unwrap(),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Ghost key validation failed: {}", stderr);
        Err(anyhow::anyhow!("Ghost key validation failed"))
    } else {
        println!("Ghost key verifyd successfully");
        Ok(())
    }
}

fn setup_delegate_keys(temp_dir: &std::path::Path) -> Result<()> {
    let delegate_dir = temp_dir.join("delegates");

    println!("Temporary directory: {:?}", temp_dir);

    // Generate master key
    let master_key_file = generate_master_key(temp_dir)?;

    // Generate delegate keys
    generate_delegate_keys(&master_key_file, &delegate_dir)?;

    println!("Successfully generated delegate keys");
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
    println!("Starting API with delegate_dir: {}", delegate_dir.display());
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

    // Collect stdout and stderr
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();

    // Spawn threads to read stdout and stderr
    let stdout_thread = thread::spawn(move || {
        stdout_reader.lines().for_each(|line| {
            if let Ok(line) = line {
                stdout_lines.push(line);
            }
        });
        stdout_lines
    });

    let stderr_thread = thread::spawn(move || {
        stderr_reader.lines().for_each(|line| {
            if let Ok(line) = line {
                stderr_lines.push(line);
            }
        });
        stderr_lines
    });

    // Wait for the API to start
    let start_time = Instant::now();
    while start_time.elapsed() < API_STARTUP_TIMEOUT {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Print collected stdout and stderr if API fails to start
                let stdout = stdout_thread.join().unwrap();
                let stderr = stderr_thread.join().unwrap();
                println!("API stdout:");
                stdout.iter().for_each(|line| println!("{}", line));
                println!("API stderr:");
                stderr.iter().for_each(|line| eprintln!("{}", line));
                return Err(anyhow::anyhow!("API process exited unexpectedly with status: {}", status));
            }
            Ok(None) => {
                // Check if the API is responding
                if is_api_ready().await.is_ok() {
                    println!("API started successfully");
                    return Ok(child);
                }
            }
            Err(e) => {
                // Print collected stdout and stderr if API fails to start
                let stdout = stdout_thread.join().unwrap();
                let stderr = stderr_thread.join().unwrap();
                println!("API stdout:");
                stdout.iter().for_each(|line| println!("{}", line));
                println!("API stderr:");
                stderr.iter().for_each(|line| eprintln!("{}", line));
                return Err(anyhow::anyhow!("Error checking API process status: {}", e));
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Print collected stdout and stderr if API fails to start
    let stdout = stdout_thread.join().unwrap();
    let stderr = stderr_thread.join().unwrap();
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
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(anyhow::anyhow!("Timeout waiting for element with selector: {:?}", locator))
}


async fn run_browser_test(headless: bool, temp_dir: &std::path::Path) -> Result<()> {
    let _temp_dir = temp_dir.to_path_buf(); // Convert to owned PathBuf
    use serde_json::json;
    let mut caps = serde_json::map::Map::new();
    let chrome_args = if headless {
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

        // Navigate to the donation page
        c.goto("http://localhost:1313/donate/ghostkey/").await?;

        // Wait for the Stripe form to load with a timeout
        let form = wait_for_element(&c, Locator::Id("payment-form"), Duration::from_secs(10)).await?;

        // Select donation amount
        let amount_radio = form.find(Locator::Css("input[name='amount'][value='20']")).await?;
        amount_radio.click().await?;

        // Select currency
        let currency_select = form.find(Locator::Id("currency")).await?;
        let currency_option = currency_select.find(Locator::Css("option[value='usd']")).await?;
        currency_option.click().await?;

        // Wait for the Stripe iframe to be present
        let stripe_iframe = wait_for_element(&c, Locator::Css("iframe[name^='__privateStripeFrame']"), Duration::from_secs(10)).await?;

        // Switch to the Stripe iframe
        let iframes = c.find_all(Locator::Css("iframe")).await?;
        let iframe_index = iframes.iter().position(|e| e.element_id() == stripe_iframe.element_id()).unwrap() as u16;
        c.enter_frame(Some(iframe_index)).await?;

        // Wait for the card number input to be present and visible inside the iframe
        let card_number = wait_for_element(&c, Locator::Css("input[name='number']"), Duration::from_secs(10)).await?;
        card_number.send_keys("4242424242424242").await?;

        let card_expiry = wait_for_element(&c, Locator::Css("input[name='expiry']"), Duration::from_secs(5)).await?;
        card_expiry.send_keys("1225").await?;

        let card_cvc = wait_for_element(&c, Locator::Css("input[name='cvc']"), Duration::from_secs(5)).await?;
        card_cvc.send_keys("123").await?;

        let postal_code = wait_for_element(&c, Locator::Css("input[name='postalCode']"), Duration::from_secs(5)).await?;
        postal_code.send_keys("12345").await?;

        // Switch back to the default content
        c.enter_frame(None).await?;

        // Submit the form
        let submit_button = form.find(Locator::Id("submit")).await?;
        submit_button.click().await?;

        // Check for error message
        if let Ok(error_element) = c.find(Locator::Id("errorMessage")).await {
            if let Ok(error_text) = error_element.text().await {
                if !error_text.trim().is_empty() {
                    println!("Error occurred on the page: {}", error_text);
                    return Err(anyhow::anyhow!("Error occurred on the page: {}", error_text));
                }
            }
        }

        // Wait for the combined key textarea
        let _combined_key_element = wait_for_element(&c, Locator::Css("textarea#combinedKey"), Duration::from_secs(10)).await?;

        // Get the content of the textarea using JavaScript
        let combined_key_content = c.execute(
            "return document.querySelector('textarea#combinedKey').value;",
            vec![],
        ).await?.as_str().unwrap_or("").to_string();

        // Save the content to a file in the temporary directory
        let temp_dir = env::temp_dir().join("ghostkey_test");
        let output_file = temp_dir.join("ghostkey_certificate.pem");
        std::fs::write(&output_file, combined_key_content.clone())?;
        println!("Ghost key certificate saved to: {}", output_file.display());
        
        // Verify the ghost key certificate using the CLI
        let master_verifying_key_file = temp_dir.join("master_verifying_key.pem");
        println!("Master verifying key file: {:?}", master_verifying_key_file);
        println!("Ghost certificate file: {:?}", output_file);

        // Log the contents of the master verifying key file
        println!("Contents of master verifying key file:");
        if let Ok(contents) = std::fs::read_to_string(&master_verifying_key_file) {
            println!("{}", contents);
        } else {
            println!("Failed to read master verifying key file");
        }

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

        println!("Validation command executed. Exit status: {:?}", output.status);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("Ghost key validation failed.");
            println!("Stderr: {}", stderr);
            println!("Stdout: {}", stdout);

            // Print the contents of the ghost certificate file
            println!("Contents of ghost certificate file:");
            if let Ok(contents) = std::fs::read_to_string(&output_file) {
                println!("{}", contents);
            } else {
                println!("Failed to read ghost certificate file");
            }

            // Log the base64 representation of the ghost certificate file
            println!("Base64 representation of ghost certificate file:");
            if let Ok(contents) = std::fs::read(&output_file) {
                use base64::{engine::general_purpose::STANDARD, Engine as _};
                println!("{}", STANDARD.encode(&contents));
            } else {
                println!("Failed to read ghost certificate file for base64 representation");
            }

            // Analyze the error message for more detailed explanation
            let error_details = analyze_validation_error(&stderr, &stdout);
            println!("Detailed error analysis: {}", error_details);

            return Err(anyhow::anyhow!("Ghost key validation failed: {}. Aborting test.", error_details));
        } else {
            println!("Ghost key certificate validation succeeded");
        }

        // Generate a ghost key using the CLI
        println!("Generating ghost key using CLI...");
        let cli_output = ProcessCommand::new("cargo")
            .args(&[
                "run",
                "--manifest-path",
                "../cli/Cargo.toml",
                "--",
                "generate-ghostkey",
                "--delegate-dir",
                temp_dir.join("delegates").join("20").to_str().unwrap(),
                "--output",
                temp_dir.join("cli_ghostkey_certificate.pem").to_str().unwrap(),
            ])
            .output()?;

        if !cli_output.status.success() {
            let stderr = String::from_utf8_lossy(&cli_output.stderr);
            println!("Failed to generate ghost key using CLI: {}", stderr);
            return Err(anyhow::anyhow!("Failed to generate ghost key using CLI"));
        }

        println!("Ghost key generated successfully using CLI");

        // Verify the CLI-generated ghost key
        let cli_validation_output = ProcessCommand::new("cargo")
            .args(&[
                "run",
                "--manifest-path",
                "../cli/Cargo.toml",
                "--",
                "verify-ghost-key",
                "--master-verifying-key",
                master_verifying_key_file.to_str().unwrap(),
                "--ghost-certificate",
                temp_dir.join("cli_ghostkey_certificate.pem").to_str().unwrap(),
            ])
            .output()?;

        if !cli_validation_output.status.success() {
            let stderr = String::from_utf8_lossy(&cli_validation_output.stderr);
            println!("CLI-generated ghost key validation failed: {}", stderr);
        } else {
            println!("CLI-generated ghost key verified successfully");
        }

        // Compare and verify the CLI-generated ghost key with the browser-generated one
        println!("Comparing and validating CLI-generated and browser-generated ghost keys...");
        let cli_ghost_key = std::fs::read_to_string(temp_dir.join("cli_ghostkey_certificate.pem"))?;
        let browser_ghost_key = std::fs::read_to_string(&output_file)?;

        let master_verifying_key = VerifyingKey::from_file(temp_dir.join("master_verifying_key.pem").as_path()).unwrap();
        
        println!("Inspecting CLI-generated ghost key certificate:");
        let cli_cert_info = inspect_ghost_key_certificate(&cli_ghost_key, master_verifying_key)?;
        
        // Parse cli_cert_info as JSON before inspecting it
        let cert_info: CertificateInfo = serde_json::from_str(&cli_cert_info)?;
        
        
        // Compare relevant parts of the certificates
        if cli_cert_info.version == cert_info.version &&
            cli_cert_info.amount == browser_cert_info.amount &&
            cli_cert_info.currency == browser_cert_info.currency {
            println!("CLI-generated and browser-generated ghost keys have matching version, amount, and currency");
        } else {
            println!("Warning: CLI-generated and browser-generated ghost keys differ in version, amount, or currency");
            println!("CLI-generated: {:?}", cli_cert_info);
            println!("Browser-generated: {:?}", browser_cert_info);
        }

        // Verify both certificates
        println!("\nValidating CLI-generated ghost key:");
        verify_ghost_key_certificate(&temp_dir.join("cli_ghostkey_certificate.pem"), &master_verifying_key_file)?;

        println!("\nValidating browser-generated ghost key:");
        verify_ghost_key_certificate(&output_file, &master_verifying_key_file)?;

        Ok(())
    })().await;

    // Print the message and wait for user input regardless of the test result
    println!("Test completed. Browser window left open for debugging.");
    println!("Press Enter to close the browser and end the test.");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    // Close the browser
    c.close().await?;

    // Return the test result
    test_result
}

fn inspect_ghost_key_certificate(combined_key_text: &str, master_verifying_key : VerifyingKey) -> Result<String> {
    use std::path::Path;
    use gklib::armorable::*;

    println!("Starting ghost key certificate inspection");

    // Extract the ghost key certificate from the combined key
    let ghost_key_cert_base64 = combined_key_text.lines()
        .skip_while(|line| !line.starts_with("-----BEGIN GHOSTKEY CERTIFICATE-----"))
        .take_while(|line| !line.starts_with("-----END GHOSTKEY CERTIFICATE-----"))
        .filter(|line| !line.starts_with("-----"))
        .collect::<Vec<&str>>()
        .join("");

    println!("Extracted base64 ghost key certificate. Length: {}", ghost_key_cert_base64.len());

    let ghostkey_certificate = GhostkeyCertificate::from_armored_string(combined_key_text)?;

    println!("Ghost Key Certificate info: {}", ghostkey_certificate.delegate.payload.info);

    // Load the delegate key from file
    let delegate_key_path = Path::new("/tmp/ghostkey_test/delegates/delegate_certificate_20.pem");
    let delegate_certificate = DelegateCertificate::from_file(delegate_key_path)?;

    println!("Loaded delegate key from file. Byte length: {}", delegate_certificate.to_bytes().unwrap().len());

    // Verify the ghost key certificate

    let certificate_info = ghostkey_certificate.verify(&master_verifying_key)?;
    
    Ok(certificate_info)
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
