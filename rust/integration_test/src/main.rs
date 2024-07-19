use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use thirtyfour::prelude::*;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Check if Hugo is already running
    if is_port_in_use(1313) {
        println!("Hugo is already running on port 1313. Do you want to kill it? (y/n)");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() == "y" {
            kill_process_on_port(1313)?;
        } else {
            return Err(anyhow::anyhow!("Hugo is already running. Please stop it and try again."));
        }
    }

    // Start Hugo
    let hugo_handle = start_hugo()?;

    // Start API
    let api_handle = start_api()?;

    // Run the browser test
    run_browser_test().await?;

    // Clean up
    hugo_handle.kill()?;
    api_handle.kill()?;

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

fn start_hugo() -> Result<std::process::Child> {
    Command::new("hugo")
        .args(&["server", "--disableFastRender"])
        .current_dir("../hugo-site")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start Hugo")
}

fn start_api() -> Result<std::process::Child> {
    Command::new("cargo")
        .args(&["run"])
        .current_dir("api")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start API")
}

async fn run_browser_test() -> Result<()> {
    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new("http://localhost:9515", caps).await?;

    // Navigate to the donation page
    driver.goto("http://localhost:1313/donate/ghostkey/").await?;

    // Wait for the Stripe form to load
    let form = driver.find(By::Id("payment-form")).await?;
    form.wait_until().displayed().await?;

    // Select donation amount
    let amount_radio = form.find(By::Css("input[name='amount'][value='20']")).await?;
    amount_radio.click().await?;

    // Select currency
    let currency_select = form.find(By::Id("currency")).await?;
    currency_select.select_by_value("usd").await?;

    // Fill out credit card information
    let card_number = driver.find(By::Css("input[name='cardnumber']")).await?;
    card_number.send_keys("4242424242424242").await?;

    let card_expiry = driver.find(By::Css("input[name='exp-date']")).await?;
    card_expiry.send_keys("1225").await?;

    let card_cvc = driver.find(By::Css("input[name='cvc']")).await?;
    card_cvc.send_keys("123").await?;

    // Submit the form
    let submit_button = form.find(By::Id("submit")).await?;
    submit_button.click().await?;

    // Wait for the success message
    let success_message = driver.find(By::Css(".donation-success")).await?;
    success_message.wait_until().displayed().await?;

    // Close the browser
    driver.quit().await?;

    Ok(())
}
