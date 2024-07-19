use anyhow::{Context, Result};
use std::process::{Command, Stdio, Child};
use std::time::Duration;
use fantoccini::{Client, ClientBuilder, Locator};
use std::thread;
use std::time::Instant;
use std::env;
use std::fs;
// use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Check if ChromeDriver is running, start it if not
    let mut chromedriver_handle = None;
    if !is_port_in_use(9515) {
        chromedriver_handle = Some(start_chromedriver()?);
        thread::sleep(Duration::from_secs(2)); // Give ChromeDriver time to start
    }

    // Always attempt to kill Hugo if it's running
    if is_port_in_use(1313) {
        println!("Attempting to kill Hugo process on port 1313");
        kill_process_on_port(1313)?;
    }

    // Start Hugo
    let mut hugo_handle = start_hugo()?;
    println!("Hugo started successfully");

    // Start API
    let mut api_handle = start_api()?;

    // Setup delegate keys
    setup_delegate_keys().context("Failed to setup delegate keys")?;

    // Run the browser test
    run_browser_test().await?;

    // Clean up
    hugo_handle.kill()?;
    api_handle.kill()?;

    // Stop ChromeDriver if we started it
    if let Some(mut handle) = chromedriver_handle {
        handle.kill()?;
    }

    Ok(())
}

fn setup_delegate_keys() -> Result<()> {
    let temp_dir = env::temp_dir().join("ghostkey_test");
    let delegate_dir = temp_dir.join("delegates");
    
    println!("Temporary directory: {:?}", temp_dir);

    // Clean up the temporary directory if it exists
    if temp_dir.exists() {
        println!("Removing existing temporary directory");
        fs::remove_dir_all(&temp_dir)?;
    }
    println!("Creating temporary directory");
    fs::create_dir_all(&temp_dir)?;

    // Generate master key
    let master_key_file = generate_master_key(&temp_dir)?;

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
    let output = Command::new("cargo")
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
    let output = Command::new("bash")
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

fn start_api() -> Result<Child> {
    Command::new("cargo")
        .args(&["run"])
        .current_dir("../api")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start API")
}

fn start_chromedriver() -> Result<Child> {
    Command::new("chromedriver")
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


async fn run_browser_test() -> Result<()> {
    let c = ClientBuilder::native().connect("http://localhost:9515").await?;

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

    // Close the browser
    c.close().await?;

    // Inspect the ghost key certificate
    inspect_ghost_key_certificate(&combined_key_content)?;

    // Validate the ghost key certificate using the CLI
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

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--manifest-path",
            "../cli/Cargo.toml",
            "--",
            "validate-ghost-key",
            "--master-verifying-key-file",
            master_verifying_key_file.to_str().unwrap(),
            "--ghost-certificate-file",
            output_file.to_str().unwrap(),
            "--debug",
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
        
        // Log the hex representation of the ghost certificate file
        println!("Hex representation of ghost certificate file:");
        if let Ok(contents) = std::fs::read(&output_file) {
            for (i, byte) in contents.iter().enumerate() {
                print!("{:02x} ", byte);
                if (i + 1) % 16 == 0 {
                    println!();
                }
            }
            println!();
        } else {
            println!("Failed to read ghost certificate file for hex representation");
        }
        
        return Err(anyhow::anyhow!("Ghost key validation failed: {}", stderr));
    }

    println!("Ghost key certificate validated successfully");

    Ok(())
}

fn inspect_ghost_key_certificate(combined_key_text: &str) -> Result<()> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use rmp_serde::Deserializer;
    use serde::Deserialize;
    use serde_json::Value;

    // Extract the ghost key certificate from the combined key
    let ghost_key_cert_base64 = combined_key_text.lines()
        .skip_while(|line| !line.starts_with("-----BEGIN GHOSTKEY CERTIFICATE-----"))
        .take_while(|line| !line.starts_with("-----END GHOSTKEY CERTIFICATE-----"))
        .filter(|line| !line.starts_with("-----"))
        .collect::<Vec<&str>>()
        .join("");

    let ghost_key_cert_bytes = STANDARD.decode(ghost_key_cert_base64)?;

    // Deserialize the ghost key certificate
    #[derive(Debug, Deserialize)]
    struct GhostkeyCertificate {
        version: u8,
        delegate_certificate: Vec<u8>,
        ghostkey_verifying_key: Vec<u8>,
        signature: Vec<u8>,
    }

    let mut deserializer = Deserializer::new(&ghost_key_cert_bytes[..]);
    let ghost_key_cert: GhostkeyCertificate = Deserialize::deserialize(&mut deserializer)?;

    println!("Ghost Key Certificate:");
    println!("Version: {}", ghost_key_cert.version);
    println!("Delegate Certificate Length: {}", ghost_key_cert.delegate_certificate.len());
    println!("Ghostkey Verifying Key Length: {}", ghost_key_cert.ghostkey_verifying_key.len());
    println!("Signature Length: {}", ghost_key_cert.signature.len());

    // Print the delegate certificate bytes for debugging
    println!("\nDelegate Certificate Bytes:");
    for (i, byte) in ghost_key_cert.delegate_certificate.iter().enumerate() {
        print!("{:02x} ", byte);
        if (i + 1) % 16 == 0 {
            println!();
        }
    }
    println!();

    // Attempt to deserialize the delegate certificate
    let mut deserializer = Deserializer::new(&ghost_key_cert.delegate_certificate[..]);
    let delegate_cert: Vec<Value> = Deserialize::deserialize(&mut deserializer)?;

    println!("\nDelegate Certificate (deserialized):");
    for (i, value) in delegate_cert.iter().enumerate() {
        println!("Item {}: {:?}", i, value);
    }

    // Extract and parse the JSON string containing the certificate info
    if let Value::String(info_str) = &delegate_cert[1] {
        let info: serde_json::Value = serde_json::from_str(info_str)?;
        println!("\nCertificate Info:");
        println!("{}", serde_json::to_string_pretty(&info)?);

        // Verify that the delegate certificate contains the correct amount
        if let Some(amount) = info.get("amount").and_then(|v| v.as_u64()) {
            if amount == 20 {
                println!("Delegate certificate contains the correct amount: $20");
            } else {
                println!("Warning: Delegate certificate contains an unexpected amount: ${}", amount);
            }
        } else {
            println!("Warning: Couldn't find or parse the amount in the delegate certificate info");
        }
    } else {
        println!("Warning: Couldn't find the certificate info string in the delegate certificate");
    }

    Ok(())
}
