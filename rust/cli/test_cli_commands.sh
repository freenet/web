#!/bin/bash

set -e

# Create a temporary directory
TEST_DIR=$(mktemp -d)
echo "Created temporary directory: $TEST_DIR"

# Function to run a command and check its exit status
run_command() {
    echo "Running: $1"
    if eval "$1"; then
        echo "Command succeeded with exit code $?"
        return 0
    else
        echo "Command failed with exit code $?"
        return 1
    fi
}

# Function to run a command and expect it to fail
expect_failure() {
    echo "Running (expecting failure): $1"
    if eval "$1"; then
        echo "Command unexpectedly succeeded with exit code $?"
        return 1
    else
        echo "Command failed as expected with exit code $?"
        return 0
    fi
}

# Generate master keys
run_command "cargo run -- generate-master-key --output-dir $TEST_DIR"
run_command "cargo run -- generate-master-key --output-dir $TEST_DIR/wrong_master"

# Generate delegate keys
run_command "cargo run -- generate-delegate-key --master-signing-key-file $TEST_DIR/master_signing_key.pem --info 'test-delegate' --output-dir $TEST_DIR"
run_command "cargo run -- generate-delegate-key --master-signing-key-file $TEST_DIR/wrong_master/master_signing_key.pem --info 'wrong-delegate' --output-dir $TEST_DIR/wrong_delegate"

# Generate ghost keys
run_command "cargo run -- generate-ghost-key --delegate-certificate-file $TEST_DIR/delegate_certificate.pem --output-dir $TEST_DIR"
run_command "cargo run -- generate-ghost-key --delegate-certificate-file $TEST_DIR/wrong_delegate/delegate_certificate.pem --output-dir $TEST_DIR/wrong_ghost"

# Validate delegate key (correct)
run_command "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"

# Validate delegate key (wrong master key)
expect_failure "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/wrong_master/master_verifying_key.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"

# Validate ghost key (correct)
run_command "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/ghostkey_certificate.pem"

# Validate ghost key (wrong master key)
expect_failure "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/wrong_master/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/ghostkey_certificate.pem"

# Validate ghost key (wrong delegate key)
expect_failure "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/wrong_ghost/ghostkey_certificate.pem"

# Test invalid delegate key validation
expect_failure "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --delegate-certificate-file $TEST_DIR/master_signing_key.pem"

# Test invalid ghost key validation
expect_failure "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_signing_key.pem --ghost-certificate-file $TEST_DIR/ghostkey_certificate.pem"

# Test missing file errors
expect_failure "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/nonexistent_file.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"
expect_failure "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/nonexistent_file.pem"

# Test incorrect file format errors
echo "Invalid content" > $TEST_DIR/invalid_file.pem
expect_failure "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/invalid_file.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"
expect_failure "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/invalid_file.pem"

echo "All tests completed"
echo "Temporary directory: $TEST_DIR"
