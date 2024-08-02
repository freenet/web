mod cli;
mod environment;
mod services;
mod browser_test;

use std::path::PathBuf;

use anyhow::Result;
use colored::*;

#[tokio::main]
async fn main() -> Result<()> {
    match run().await {
        Ok(_) => {
            println!("Integration test completed {}", "successfully".green());
            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}", "Integration test failed".red(), e);
            eprintln!("Error details: {:?}", e);
            Err(e)
        }
    }
}

async fn run() -> Result<()> {
    println!("Starting integration test...");
    let cli_args = cli::parse_arguments();
    
    let temp_dir = environment::setup_environment().await?;
    
    let tls_config = if let (Some(cert), Some(key)) = (cli_args.tls_cert, cli_args.tls_key) {
        Some((PathBuf::from(cert), PathBuf::from(key)))
    } else {
        None
    };
    
    let (mut hugo_handle, mut api_handle, chromedriver_handle) = services::start_services(&temp_dir, tls_config).await?;
    environment::setup_delegate_keys(&temp_dir)?;
    
    environment::print_task(&format!("Starting {} browser", if cli_args.visible { "visible" } else { "headless" }));
    environment::print_result(true);

    let result = browser_test::run_browser_test(&cli_args, &temp_dir).await;
    
    services::cleanup_processes(&mut hugo_handle, &mut api_handle, chromedriver_handle).await;

    result
}
