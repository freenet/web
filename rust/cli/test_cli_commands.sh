#!/bin/bash

set -e

# Initialize counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Check if verbose output is requested
VERBOSE=0
KEEP_TEMP=0
for arg in "$@"; do
    case $arg in
        --verbose)
            VERBOSE=1
            ;;
        --keep-temp)
            KEEP_TEMP=1
            ;;
    esac
done

# Create a temporary directory
TEST_DIR=$(mktemp -d)
echo "Created temporary directory: $TEST_DIR"

# Cleanup function
cleanup() {
    if [ $KEEP_TEMP -eq 0 ]; then
        echo "Cleaning up temporary directory: $TEST_DIR"
        rm -rf "$TEST_DIR"
    else
        echo "Keeping temporary directory: $TEST_DIR"
    fi
}

# Set trap to call cleanup function on exit
trap cleanup EXIT

# Function to run a command and check its exit status
run_command() {
    local test_name="$1"
    local command="$2"
    ((TOTAL_TESTS++))
    echo -n "Testing $test_name... "
    if output=$(eval "$command" 2>&1); then
        echo "OK"
        ((PASSED_TESTS++))
        if [ $VERBOSE -eq 1 ]; then
            echo "Command: $command"
            echo "Output:"
            echo "$output"
            echo ""
        fi
        return 0
    else
        echo "FAILED"
        ((FAILED_TESTS++))
        echo "Command: $command"
        echo "Output:"
        echo "$output"
        echo "Error details:"
        echo "$output"
        return 1
    fi
}

# Function to run a command and expect it to fail
expect_failure() {
    local test_name="$1"
    local command="$2"
    ((TOTAL_TESTS++))
    echo -n "Testing $test_name (expecting failure)... "
    if output=$(eval "$command" 2>&1); then
        echo "FAILED (unexpected success)"
        ((FAILED_TESTS++))
        echo "Command: $command"
        echo "Output:"
        echo "$output"
        return 1
    else
        echo "OK"
        ((PASSED_TESTS++))
        if [ $VERBOSE -eq 1 ]; then
            echo "Command: $command"
            echo "Output:"
            echo "$output"
            echo ""
        fi
        return 0
    fi
}

# Generate master keys
run_command "generate master key" "cargo run --quiet -- generate-master-key $TEST_DIR/master"
run_command "generate wrong master key" "cargo run --quiet -- generate-master-key $TEST_DIR/wrong_master"

# Generate delegate keys
run_command "generate delegate key" "cargo run --quiet -- generate-delegate-key $TEST_DIR/master/master_signing_key.pem test-delegate $TEST_DIR/delegate"
run_command "generate wrong delegate key" "cargo run --quiet -- generate-delegate-key $TEST_DIR/wrong_master/master_signing_key.pem wrong-delegate $TEST_DIR/wrong_delegate"

# Generate ghost keys
run_command "generate ghost key" "cargo run --quiet -- generate-ghost-key $TEST_DIR/delegate $TEST_DIR/ghost"
run_command "generate wrong ghost key" "cargo run --quiet -- generate-ghost-key $TEST_DIR/wrong_delegate $TEST_DIR/wrong_ghost"

# Validate delegate key (correct)
run_command "validate correct delegate key" "cargo run --quiet -- validate-delegate-key $TEST_DIR/master/master_verifying_key.pem $TEST_DIR/delegate/delegate_certificate.pem"

# Validate delegate key (wrong master key)
expect_failure "validate delegate key with wrong master key" "cargo run --quiet -- validate-delegate-key $TEST_DIR/wrong_master/master_verifying_key.pem $TEST_DIR/delegate/delegate_certificate.pem"

# Test validating delegate key with missing certificate
expect_failure "validate delegate key with missing certificate" "cargo run --quiet -- validate-delegate-key $TEST_DIR/master/master_verifying_key.pem $TEST_DIR/nonexistent_certificate.pem"

# Validate ghost key (correct)
run_command "validate correct ghost key" "cargo run --quiet -- validate-ghost-key $TEST_DIR/master/master_verifying_key.pem $TEST_DIR/ghost/ghostkey_certificate.pem"

# Validate ghost key (wrong master key)
expect_failure "validate ghost key with wrong master key" "cargo run --quiet -- validate-ghost-key $TEST_DIR/wrong_master/master_verifying_key.pem $TEST_DIR/ghost/ghostkey_certificate.pem"

# Validate ghost key (wrong delegate key)
expect_failure "validate ghost key with wrong delegate key" "cargo run --quiet -- validate-ghost-key $TEST_DIR/master/master_verifying_key.pem $TEST_DIR/wrong_ghost/ghostkey_certificate.pem"

# Test generating ghost key with missing delegate files
expect_failure "generate ghost key with missing delegate files" "cargo run --quiet -- generate-ghost-key $TEST_DIR/nonexistent_dir $TEST_DIR/output"

# Test generating ghost key with invalid delegate files
mkdir -p $TEST_DIR/invalid_delegate
echo "Invalid content" > $TEST_DIR/invalid_delegate/delegate_certificate.pem
echo "Invalid content" > $TEST_DIR/invalid_delegate/delegate_signing_key.pem
expect_failure "generate ghost key with invalid delegate files" "cargo run --quiet -- generate-ghost-key $TEST_DIR/invalid_delegate $TEST_DIR/output"

# Test invalid delegate key validation
expect_failure "validate invalid delegate key" "cargo run --quiet -- validate-delegate-key $TEST_DIR/master/master_verifying_key.pem $TEST_DIR/master/master_signing_key.pem"

# Test invalid ghost key validation
expect_failure "validate invalid ghost key" "cargo run --quiet -- validate-ghost-key $TEST_DIR/master/master_signing_key.pem $TEST_DIR/ghost/ghostkey_certificate.pem"

# Test missing file errors
expect_failure "validate delegate key with missing file" "cargo run --quiet -- validate-delegate-key $TEST_DIR/nonexistent_file.pem $TEST_DIR/delegate/delegate_certificate.pem"
expect_failure "validate ghost key with missing file" "cargo run --quiet -- validate-ghost-key $TEST_DIR/master/master_verifying_key.pem $TEST_DIR/nonexistent_file.pem"

# Test incorrect file format errors
echo "Invalid content" > $TEST_DIR/invalid_file.pem
expect_failure "validate delegate key with invalid file format" "cargo run --quiet -- validate-delegate-key $TEST_DIR/invalid_file.pem $TEST_DIR/delegate/delegate_certificate.pem"
expect_failure "validate ghost key with invalid file format" "cargo run --quiet -- validate-ghost-key $TEST_DIR/master/master_verifying_key.pem $TEST_DIR/invalid_file.pem"

# Test with tampered certificates
sed 's/./X/1' $TEST_DIR/delegate/delegate_certificate.pem > $TEST_DIR/tampered_delegate_certificate.pem
expect_failure "validate tampered delegate certificate" "cargo run --quiet -- validate-delegate-key $TEST_DIR/master/master_verifying_key.pem $TEST_DIR/tampered_delegate_certificate.pem"

sed 's/./X/1' $TEST_DIR/ghost/ghostkey_certificate.pem > $TEST_DIR/tampered_ghostkey_certificate.pem
expect_failure "validate tampered ghost certificate" "cargo run --quiet -- validate-ghost-key $TEST_DIR/master/master_verifying_key.pem $TEST_DIR/tampered_ghostkey_certificate.pem"

# Test with mismatched keys
expect_failure "validate delegate key with mismatched keys" "cargo run --quiet -- validate-delegate-key $TEST_DIR/wrong_master/master_verifying_key.pem $TEST_DIR/delegate/delegate_certificate.pem"
expect_failure "validate ghost key with mismatched keys" "cargo run --quiet -- validate-ghost-key $TEST_DIR/wrong_master/master_verifying_key.pem $TEST_DIR/ghost/ghostkey_certificate.pem"

# Test with empty files
touch $TEST_DIR/empty_file.pem
expect_failure "validate delegate key with empty file" "cargo run --quiet -- validate-delegate-key $TEST_DIR/empty_file.pem $TEST_DIR/delegate/delegate_certificate.pem"
expect_failure "validate ghost key with empty file" "cargo run --quiet -- validate-ghost-key $TEST_DIR/master/master_verifying_key.pem $TEST_DIR/empty_file.pem"

# Test with very large input files
dd if=/dev/urandom of=$TEST_DIR/large_file.pem bs=1M count=10 2>/dev/null
expect_failure "validate delegate key with large file" "cargo run --quiet -- validate-delegate-key $TEST_DIR/large_file.pem $TEST_DIR/delegate/delegate_certificate.pem"
expect_failure "validate ghost key with large file" "cargo run --quiet -- validate-ghost-key $TEST_DIR/master/master_verifying_key.pem $TEST_DIR/large_file.pem"

# Test with non-existent files
expect_failure "validate delegate key with non-existent file" "cargo run --quiet -- validate-delegate-key $TEST_DIR/nonexistent.pem $TEST_DIR/delegate/delegate_certificate.pem"
expect_failure "validate ghost key with non-existent file" "cargo run --quiet -- validate-ghost-key $TEST_DIR/master/master_verifying_key.pem $TEST_DIR/nonexistent.pem"

# Test with insufficient permissions
chmod 000 $TEST_DIR/master/master_verifying_key.pem
expect_failure "validate delegate key with insufficient permissions" "cargo run --quiet -- validate-delegate-key $TEST_DIR/master/master_verifying_key.pem $TEST_DIR/delegate/delegate_certificate.pem"
chmod 644 $TEST_DIR/master/master_verifying_key.pem

echo "All tests completed"
echo "Temporary directory: $TEST_DIR"

# Print summary
echo "Test Summary:"
echo "Total tests: $TOTAL_TESTS"
echo "Passed tests: $PASSED_TESTS"
echo "Failed tests: $FAILED_TESTS"

# Set exit status based on test results
if [ $FAILED_TESTS -eq 0 ]; then
    exit 0
else
    exit 1
fi
