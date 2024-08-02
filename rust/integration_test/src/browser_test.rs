use anyhow::{Context, Result};
use fantoccini::{Client, ClientBuilder, Locator};
use std::time::{Duration, Instant};
use std::path::Path;
use std::process::Command;
use std::env;
use serde_json::json;
use colored::*;

pub async fn run_browser_test(cli_args: &crate::cli::CliArgs, temp_dir: &Path) -> Result<()> {
    let mut caps = serde_json::map::Map::new();
    let chrome_args = if !cli_args.visible {
        json!(["--headless", "--disable-gpu"])
    } else {
        json!([])
    };
    caps.insert("goog:chromeOptions".to_string(), json!({"args": chrome_args}));

    let c = ClientBuilder::native()
        .capabilities(caps)
        .connect("http://localhost:9515")
        .await?;

    let test_result = run_test(&c, temp_dir).await;

    if (test_result.is_err() && cli_args.wait_on_failure) || cli_args.wait {
        let message = if test_result.is_err() {
            "Test failed. Browser window left open for debugging.".yellow()
        } else {
            "Test succeeded. Browser window left open for inspection.".green()
        };
        println!("{}", message);
        println!("Press Enter to close the browser and end the test.");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
    }

    c.close().await?;
    test_result
}

async fn run_test(c: &Client, temp_dir: &Path) -> Result<()> {
    crate::environment::print_task("Navigating to donation page");
    c.goto("http://localhost:1313/donate/ghostkey/").await?;
    crate::environment::print_result(true);

    crate::environment::print_task("Filling out donation form");
    let _form = wait_for_element(c, Locator::Id("payment-form"), Duration::from_secs(30)).await?;
    let amount_radio = wait_for_element(c, Locator::Css("input[name='amount'][value='20']"), Duration::from_secs(10)).await?;
    amount_radio.click().await?;
    let currency_select = wait_for_element(c, Locator::Id("currency"), Duration::from_secs(10)).await?;
    currency_select.select_by_value("usd").await?;
    crate::environment::print_result(true);

    crate::environment::print_task("Filling out Stripe payment form");
    
    wait_for_element(c, Locator::Id("submit"), Duration::from_secs(30)).await?;
    
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    let stripe_iframe = wait_for_element(c, Locator::Css("iframe[name^='__privateStripeFrame']"), Duration::from_secs(30)).await?;
    let iframes = c.find_all(Locator::Css("iframe")).await?;
    let iframe_index = iframes.iter().position(|e| e.element_id() == stripe_iframe.element_id()).unwrap() as u16;
    c.enter_frame(Some(iframe_index)).await?;
    
    wait_for_element(c, Locator::Css("input[name='number']"), Duration::from_secs(30)).await?;
    
    let card_number = wait_for_element(c, Locator::Css("input[name='number']"), Duration::from_secs(10)).await?;
    card_number.send_keys("4242424242424242").await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let card_expiry = wait_for_element(c, Locator::Css("input[name='expiry']"), Duration::from_secs(10)).await?;
    card_expiry.send_keys("1225").await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let card_cvc = wait_for_element(c, Locator::Css("input[name='cvc']"), Duration::from_secs(10)).await?;
    card_cvc.send_keys("123").await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let postal_code = wait_for_element(c, Locator::Css("input[name='postalCode']"), Duration::from_secs(10)).await?;
    postal_code.send_keys("12345").await?;
    c.enter_frame(None).await?;
    crate::environment::print_result(true);

    crate::environment::print_task("Submitting payment");
    let submit_button = wait_for_element(c, Locator::Id("submit"), Duration::from_secs(10)).await?;
    submit_button.click().await?;
    crate::environment::print_result(true);

    crate::environment::print_task("Checking for errors");
    if let Ok(error_element) = c.find(Locator::Id("errorMessage")).await {
        if let Ok(error_text) = error_element.text().await {
            if !error_text.trim().is_empty() {
                crate::environment::print_result(false);
                return Err(anyhow::anyhow!("Error occurred on the page: {}", error_text));
            }
        }
    }
    crate::environment::print_result(true);

    crate::environment::print_task("Waiting for ghostkey certificate");
    let combined_key_result = wait_for_element(c, Locator::Css("textarea#combinedKey"), Duration::from_secs(120)).await;
    if combined_key_result.is_err() {
        let e = combined_key_result.unwrap_err();
        println!("Error: {}", e);
        if let Ok(body) = c.source().await {
            println!("Page source:\n{}", body);
        }
        if let Ok(url) = c.current_url().await {
            println!("Current URL: {}", url);
        }
        crate::environment::print_result(false);
        return Err(anyhow::anyhow!("Failed to find ghostkey certificate: {}", e));
    }
    crate::environment::print_result(true);

    let current_url = c.current_url().await?;
    if !current_url.as_str().contains("/donate/ghostkey/success") {
        println!("Error: Not on the success page. Current page: {}", current_url);
        println!("Page source:");
        println!("{}", c.source().await?);
        return Err(anyhow::anyhow!("Failed to reach the success page"));
    }

    crate::environment::print_task("Saving ghostkey certificate");
    let combined_key_content = c.execute(
        "return document.querySelector('textarea#combinedKey').value;",
        vec![],
    ).await?.as_str().unwrap_or("").to_string();
    let output_file = temp_dir.join("ghost_key_certificate.pem");
    std::fs::write(&output_file, combined_key_content.clone())?;
    crate::environment::print_result(true);
    
    crate::environment::print_task("Verifying ghostkey certificate");
    let master_verifying_key_file = temp_dir.join("master_verifying_key.pem");
    let output = Command::new("cargo")
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("{}", "Ghost Key validation failed.".red());
        println!("Stderr: {}", stderr);
        println!("Stdout: {}", stdout);
        let error_details = analyze_validation_error(&stderr, &stdout);
        println!("Detailed error analysis: {}", error_details);
        crate::environment::print_result(false);
        return Err(anyhow::anyhow!("Ghost Key validation failed: {}. Aborting test.", error_details));
    }

    crate::environment::print_result(true);
    println!("Ghost Key certificate {}.", "verified".green());
    Ok(())
}

async fn wait_for_element(client: &Client, locator: Locator<'_>, timeout: Duration) -> Result<fantoccini::elements::Element> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Ok(element) = client.find(locator.clone()).await {
            if element.is_displayed().await? {
                return Ok(element);
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err(anyhow::anyhow!("Timeout waiting for element with selector: {:?}", locator))
}

fn analyze_validation_error(stderr: &str, stdout: &str) -> String {
    if stderr.contains("Signature verification failed") {
        "The ghostkey certificate signature is invalid. This could be due to tampering, use of an incorrect master key, or a mismatch between the certificate data and the signature."
    } else if stderr.contains("Invalid certificate format") {
        "The ghostkey certificate has an invalid format. It may be corrupted or not properly encoded."
    } else if stderr.contains("Delegate certificate validation failed") {
        "The delegate certificate within the ghostkey certificate is invalid. This could indicate an issue with the delegate key generation or signing process."
    } else if stderr.contains("Invalid ghostkey verifying key") {
        "The ghostkey verifying key in the certificate is invalid. This could be due to incorrect key generation or corruption of the certificate."
    } else if stdout.contains("amount mismatch") {
        "The amount in the ghostkey certificate does not match the expected value. This could indicate tampering or an error in the certificate generation process."
    } else {
        "An unknown error occurred during ghostkey validation. Please check the full error message for more details."
    }.to_string()
}
