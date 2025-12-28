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
        echo ""
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
        echo ""
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

# Test generate-master-key command
run_command "generate master key" "cargo run --quiet -- generate-master-key --output-dir $TEST_DIR"
run_command "generate master key (ignore permissions)" "cargo run --quiet -- generate-master-key --output-dir $TEST_DIR --ignore-permissions"
expect_failure "generate master key (invalid directory)" "cargo run --quiet -- generate-master-key --output-dir /nonexistent/directory"

# Test generate-delegate command
run_command "generate delegate key" "cargo run --quiet -- generate-delegate --master-signing-key $TEST_DIR/master_signing_key.pem --info 'test-delegate' --output-dir $TEST_DIR"
expect_failure "generate delegate key (invalid master key)" "cargo run --quiet -- generate-delegate --master-signing-key $TEST_DIR/nonexistent_key.pem --info 'test-delegate' --output-dir $TEST_DIR"
expect_failure "generate delegate key (invalid output directory)" "cargo run --quiet -- generate-delegate --master-signing-key $TEST_DIR/master_signing_key.pem --info 'test-delegate' --output-dir /nonexistent/directory"

# Test verify-delegate command
run_command "verify delegate key" "cargo run --quiet -- verify-delegate --master-verifying-key $TEST_DIR/master_verifying_key.pem --delegate-certificate $TEST_DIR/delegate_certificate.pem"
expect_failure "verify delegate key (invalid master key)" "cargo run --quiet -- verify-delegate --master-verifying-key $TEST_DIR/nonexistent_key.pem --delegate-certificate $TEST_DIR/delegate_certificate.pem"
expect_failure "verify delegate key (invalid certificate)" "cargo run --quiet -- verify-delegate --master-verifying-key $TEST_DIR/master_verifying_key.pem --delegate-certificate $TEST_DIR/nonexistent_certificate.pem"

# Test with tampered certificates
sed 's/./X/1' $TEST_DIR/delegate_certificate.pem > $TEST_DIR/tampered_delegate_certificate.pem
expect_failure "verify tampered delegate certificate" "cargo run --quiet -- verify-delegate --master-verifying-key $TEST_DIR/master_verifying_key.pem --delegate-certificate $TEST_DIR/tampered_delegate_certificate.pem"

# Test with empty files
touch $TEST_DIR/empty_file.pem
expect_failure "verify delegate key with empty master key" "cargo run --quiet -- verify-delegate --master-verifying-key $TEST_DIR/empty_file.pem --delegate-certificate $TEST_DIR/delegate_certificate.pem"
expect_failure "verify delegate key with empty certificate" "cargo run --quiet -- verify-delegate --master-verifying-key $TEST_DIR/master_verifying_key.pem --delegate-certificate $TEST_DIR/empty_file.pem"

# Test with very large input files
dd if=/dev/urandom of=$TEST_DIR/large_file.pem bs=1M count=10 2>/dev/null
expect_failure "verify delegate key with large master key file" "cargo run --quiet -- verify-delegate --master-verifying-key $TEST_DIR/large_file.pem --delegate-certificate $TEST_DIR/delegate_certificate.pem"
expect_failure "verify delegate key with large certificate file" "cargo run --quiet -- verify-delegate --master-verifying-key $TEST_DIR/master_verifying_key.pem --delegate-certificate $TEST_DIR/large_file.pem"

# Test with insufficient permissions
chmod 000 $TEST_DIR/master_verifying_key.pem
expect_failure "verify delegate key with insufficient permissions" "cargo run --quiet -- verify-delegate --master-verifying-key $TEST_DIR/master_verifying_key.pem --delegate-certificate $TEST_DIR/delegate_certificate.pem"
chmod 644 $TEST_DIR/master_verifying_key.pem

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
