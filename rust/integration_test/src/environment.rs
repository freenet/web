use anyhow::{Context, Result};
use std::process::Command;
use std::path::{Path, PathBuf};
use std::env;
use std::fs;
use std::io::Write;
use colored::Colorize;

pub async fn setup_environment() -> Result<PathBuf> {
    let temp_dir = env::temp_dir().join("ghost_key_test");
    fs::create_dir_all(&temp_dir)?;
    Ok(temp_dir)
}

pub fn setup_delegate_keys(temp_dir: &Path) -> Result<()> {
    let delegate_dir = temp_dir.join("delegates");

    print_task("Generating master key");
    let master_key_file = generate_master_key(temp_dir)?;
    print_result(true);

    print_task("Generating delegate keys");
    generate_delegate_keys(&master_key_file, &delegate_dir)?;
    print_result(true);

    env::set_var("GHOSTKEY_DELEGATE_KEY_DIR", delegate_dir.to_str().unwrap());
    Ok(())
}

fn generate_master_key(temp_dir: &Path) -> Result<PathBuf> {
    let master_key_file = temp_dir.join("master_signing_key.pem");
    let cli_dir = std::env::current_dir()?.join("../cli");
    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--manifest-path", cli_dir.join("Cargo.toml").to_str().unwrap(), "--", "generate-master-key", "--output-dir"])
        .arg(temp_dir)
        .current_dir(&cli_dir)
        .output()
        .context("Failed to execute generate-master-key command")?;

    if !output.status.success() {
        let error_msg = format!("Failed to generate master key: {}", String::from_utf8_lossy(&output.stderr));
        return Err(anyhow::anyhow!(error_msg));
    }

    Ok(master_key_file)
}

fn generate_delegate_keys(master_key_file: &Path, delegate_dir: &Path) -> Result<()> {
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
        return Err(anyhow::anyhow!(error_msg));
    }

    Ok(())
}

pub fn print_task(description: &str) {
    print!("{}... ", description);
    std::io::stdout().flush().unwrap();
}

pub fn print_result(success: bool) {
    if success {
        println!("{}", "Ok".green());
    } else {
        println!("{}", "Failed".red());
    }
}
