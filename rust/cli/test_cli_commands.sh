#!/bin/bash

set -e

# Create a temporary directory
TEST_DIR=$(mktemp -d)
echo "Created temporary directory: $TEST_DIR"

# Function to run a command and check its exit status
run_command() {
    echo "Running: $1"
    if eval "$1"; then
        echo "Command succeeded"
    else
        echo "Command failed"
        exit 1
    fi
}

# Generate master key
run_command "cargo run -- generate-master-key --output-dir $TEST_DIR"

# Generate delegate key
run_command "cargo run -- generate-delegate-key --master-signing-key-file $TEST_DIR/master_signing_key.pem --attributes 'test-delegate' --output-dir $TEST_DIR"

# Validate delegate key
run_command "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"

# Generate ghost key
run_command "cargo run -- generate-ghost-key --delegate-certificate-file $TEST_DIR/delegate_certificate.pem --output-dir $TEST_DIR"

# Validate ghost key
run_command "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/ghostkey_certificate.pem"

echo "All tests completed successfully"
echo "Temporary directory: $TEST_DIR"
