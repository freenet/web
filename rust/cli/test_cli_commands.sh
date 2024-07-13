#!/bin/bash

set -e

# Check if verbose output is requested
VERBOSE=0
if [ "$1" = "--verbose" ]; then
    VERBOSE=1
fi

# Create a temporary directory
TEST_DIR=$(mktemp -d)
echo "Created temporary directory: $TEST_DIR"

# Function to run a command and check its exit status
run_command() {
    local test_name="$1"
    local command="$2"
    echo -n "Testing $test_name... "
    if output=$(eval "$command" 2>&1); then
        echo "OK"
        if [ $VERBOSE -eq 1 ]; then
            echo "Command: $command"
            echo "Output:"
            echo "$output" | grep -v "Compiling" | grep -v "Finished" | grep -v "Running"
            echo ""
        fi
        return 0
    else
        echo "FAILED"
        echo "Command: $command"
        echo "Output:"
        echo "$output" | grep -v "Compiling" | grep -v "Finished" | grep -v "Running"
        return 1
    fi
}

# Function to run a command and expect it to fail
expect_failure() {
    local test_name="$1"
    local command="$2"
    echo -n "Testing $test_name (expecting failure)... "
    if output=$(eval "$command" 2>&1); then
        echo "FAILED (unexpected success)"
        echo "Command: $command"
        echo "Output:"
        echo "$output" | grep -v "Compiling" | grep -v "Finished" | grep -v "Running"
        return 1
    else
        echo "OK"
        if [ $VERBOSE -eq 1 ]; then
            echo "Command: $command"
            echo "Output:"
            echo "$output" | grep -v "Compiling" | grep -v "Finished" | grep -v "Running"
            echo ""
        fi
        return 0
    fi
}

# Function to handle errors
handle_error() {
    echo "Error occurred in test: $1"
    echo "Command: $2"
    echo "Error message: $3"
    echo "Exit code: $4"
    echo "Line number: $5"
    echo "Function name: $6"
    exit 1
}

# Trap for error handling
trap 'handle_error "${BASH_COMMAND}" "$_" "$?" "$?" "${LINENO}" "${FUNCNAME[0]}"' ERR

# Generate master keys
run_command "generate master key" "cargo run -- generate-master-key --output-dir $TEST_DIR" || handle_error "generate master key" "cargo run -- generate-master-key --output-dir $TEST_DIR" "$?"
run_command "generate wrong master key" "cargo run -- generate-master-key --output-dir $TEST_DIR/wrong_master" || handle_error "generate wrong master key" "cargo run -- generate-master-key --output-dir $TEST_DIR/wrong_master" "$?"

# Generate delegate keys
run_command "generate delegate key" "cargo run -- generate-delegate-key --master-signing-key-file $TEST_DIR/master_signing_key.pem --info 'test-delegate' --output-dir $TEST_DIR" || handle_error "generate delegate key" "cargo run -- generate-delegate-key --master-signing-key-file $TEST_DIR/master_signing_key.pem --info 'test-delegate' --output-dir $TEST_DIR" "$?"
run_command "generate wrong delegate key" "cargo run -- generate-delegate-key --master-signing-key-file $TEST_DIR/wrong_master/master_signing_key.pem --info 'wrong-delegate' --output-dir $TEST_DIR/wrong_delegate" || handle_error "generate wrong delegate key" "cargo run -- generate-delegate-key --master-signing-key-file $TEST_DIR/wrong_master/master_signing_key.pem --info 'wrong-delegate' --output-dir $TEST_DIR/wrong_delegate" "$?"

# Generate ghost keys
run_command "generate ghost key" "cargo run -- generate-ghost-key --delegate-certificate-file $TEST_DIR/delegate_certificate.pem --output-dir $TEST_DIR" || handle_error "generate ghost key" "cargo run -- generate-ghost-key --delegate-certificate-file $TEST_DIR/delegate_certificate.pem --output-dir $TEST_DIR" "$?"
run_command "generate wrong ghost key" "cargo run -- generate-ghost-key --delegate-certificate-file $TEST_DIR/wrong_delegate/delegate_certificate.pem --output-dir $TEST_DIR/wrong_ghost" || handle_error "generate wrong ghost key" "cargo run -- generate-ghost-key --delegate-certificate-file $TEST_DIR/wrong_delegate/delegate_certificate.pem --output-dir $TEST_DIR/wrong_ghost" "$?"

# Add debugging information
echo "Current directory: $(pwd)"
echo "Contents of TEST_DIR:"
ls -R "$TEST_DIR"

# Validate delegate key (correct)
run_command "validate correct delegate key" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"

# Validate delegate key (wrong master key)
expect_failure "validate delegate key with wrong master key" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/wrong_master/master_verifying_key.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"

# Validate ghost key (correct)
run_command "validate correct ghost key" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/ghostkey_certificate.pem"

# Validate ghost key (wrong master key)
expect_failure "validate ghost key with wrong master key" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/wrong_master/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/ghostkey_certificate.pem"

# Validate ghost key (wrong delegate key)
expect_failure "validate ghost key with wrong delegate key" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/wrong_ghost/ghostkey_certificate.pem"

# Test invalid delegate key validation
expect_failure "validate invalid delegate key" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --delegate-certificate-file $TEST_DIR/master_signing_key.pem"

# Test invalid ghost key validation
expect_failure "validate invalid ghost key" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_signing_key.pem --ghost-certificate-file $TEST_DIR/ghostkey_certificate.pem"

# Test missing file errors
expect_failure "validate delegate key with missing file" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/nonexistent_file.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"
expect_failure "validate ghost key with missing file" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/nonexistent_file.pem"

# Test incorrect file format errors
echo "Invalid content" > $TEST_DIR/invalid_file.pem
expect_failure "validate delegate key with invalid file format" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/invalid_file.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"
expect_failure "validate ghost key with invalid file format" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/invalid_file.pem"

# Test with tampered certificates
sed 's/./X/1' $TEST_DIR/delegate_certificate.pem > $TEST_DIR/tampered_delegate_certificate.pem
expect_failure "validate tampered delegate certificate" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --delegate-certificate-file $TEST_DIR/tampered_delegate_certificate.pem"

sed 's/./X/1' $TEST_DIR/ghostkey_certificate.pem > $TEST_DIR/tampered_ghostkey_certificate.pem
expect_failure "validate tampered ghost certificate" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/tampered_ghostkey_certificate.pem"

# Test with mismatched keys
expect_failure "validate delegate key with mismatched keys" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/wrong_master/master_verifying_key.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"
expect_failure "validate ghost key with mismatched keys" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/wrong_master/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/ghostkey_certificate.pem"

# Test with empty files
touch $TEST_DIR/empty_file.pem
expect_failure "validate delegate key with empty file" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/empty_file.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"
expect_failure "validate ghost key with empty file" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/empty_file.pem"

# Test with very large input files
dd if=/dev/urandom of=$TEST_DIR/large_file.pem bs=1M count=10 2>/dev/null
expect_failure "validate delegate key with large file" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/large_file.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"
expect_failure "validate ghost key with large file" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/large_file.pem"

# Test with non-existent files
expect_failure "validate delegate key with non-existent file" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/nonexistent.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"
expect_failure "validate ghost key with non-existent file" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/nonexistent.pem"

# Test with insufficient permissions
chmod 000 $TEST_DIR/master_verifying_key.pem
expect_failure "validate delegate key with insufficient permissions" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"
chmod 644 $TEST_DIR/master_verifying_key.pem

# Test with different encodings
echo "Invalid UTF-8 content" | iconv -f UTF-8 -t UTF-16 > $TEST_DIR/utf16_file.pem
expect_failure "validate delegate key with UTF-16 encoding" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/utf16_file.pem --delegate-certificate-file $TEST_DIR/delegate_certificate.pem"

# Test with malformed armored input
echo "-----BEGIN INVALID ARMOR-----" > $TEST_DIR/malformed_armor.pem
echo "SGVsbG8gV29ybGQh" >> $TEST_DIR/malformed_armor.pem
echo "-----END INVALID ARMOR-----" >> $TEST_DIR/malformed_armor.pem
expect_failure "validate delegate key with malformed armor" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --delegate-certificate-file $TEST_DIR/malformed_armor.pem"
expect_failure "validate ghost key with malformed armor" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/malformed_armor.pem"

# Test with truncated input
head -c 100 $TEST_DIR/delegate_certificate.pem > $TEST_DIR/truncated_delegate.pem
head -c 100 $TEST_DIR/ghostkey_certificate.pem > $TEST_DIR/truncated_ghost.pem
expect_failure "validate truncated delegate certificate" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --delegate-certificate-file $TEST_DIR/truncated_delegate.pem"
expect_failure "validate truncated ghost certificate" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/truncated_ghost.pem"

# Test with modified signature
cp $TEST_DIR/delegate_certificate.pem $TEST_DIR/modified_delegate_certificate.pem
cp $TEST_DIR/ghostkey_certificate.pem $TEST_DIR/modified_ghostkey_certificate.pem
sed -i 's/[A-Za-z0-9]/X/g' $TEST_DIR/modified_delegate_certificate.pem
sed -i 's/[A-Za-z0-9]/X/g' $TEST_DIR/modified_ghostkey_certificate.pem
expect_failure "validate delegate key with modified signature" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --delegate-certificate-file $TEST_DIR/modified_delegate_certificate.pem"
expect_failure "validate ghost key with modified signature" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/modified_ghostkey_certificate.pem"

# Test with swapped certificates
expect_failure "validate delegate key with swapped certificate" "cargo run -- validate-delegate-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --delegate-certificate-file $TEST_DIR/ghostkey_certificate.pem"
expect_failure "validate ghost key with swapped certificate" "cargo run -- validate-ghost-key --master-verifying-key-file $TEST_DIR/master_verifying_key.pem --ghost-certificate-file $TEST_DIR/delegate_certificate.pem"

echo "All tests completed"
echo "Temporary directory: $TEST_DIR"
