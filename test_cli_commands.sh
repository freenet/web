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
    TOTAL_TESTS=$((TOTAL_TESTS+1))
    echo -n "Testing $test_name... "
    if output=$(eval "$command" 2>&1); then
        echo "OK"
        PASSED_TESTS=$((PASSED_TESTS+1))
        if [ $VERBOSE -eq 1 ]; then
            echo "Command: $command"
            echo "Output:"
            echo "$output"
            echo ""
        fi
        return 0
    else
        echo "FAILED"
        FAILED_TESTS=$((FAILED_TESTS+1))
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
    TOTAL_TESTS=$((TOTAL_TESTS+1))
    echo -n "Testing $test_name (expecting failure)... "
    if output=$(eval "$command" 2>&1); then
        echo "FAILED (unexpected success)"
        FAILED_TESTS=$((FAILED_TESTS+1))
        echo "Command: $command"
        echo "Output:"
        echo "$output"
        echo ""
        return 1
    else
        echo "OK"
        PASSED_TESTS=$((PASSED_TESTS+1))
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

# Test generate-notary command (canonical spelling)
run_command "generate notary key" "cargo run --quiet -- generate-notary --master-signing-key $TEST_DIR/master_signing_key.pem --info 'test-notary' --output-dir $TEST_DIR"
expect_failure "generate notary key (invalid master key)" "cargo run --quiet -- generate-notary --master-signing-key $TEST_DIR/nonexistent_key.pem --info 'test-notary' --output-dir $TEST_DIR"
expect_failure "generate notary key (invalid output directory)" "cargo run --quiet -- generate-notary --master-signing-key $TEST_DIR/master_signing_key.pem --info 'test-notary' --output-dir /nonexistent/directory"

# Test legacy generate-delegate alias (deprecated, should still work)
run_command "generate delegate key (legacy alias)" "cargo run --quiet -- generate-delegate --master-signing-key $TEST_DIR/master_signing_key.pem --info 'test-delegate' --output-dir $TEST_DIR"

# Test verify-notary command (canonical spelling with --notary-certificate)
run_command "verify notary key" "cargo run --quiet -- verify-notary --master-verifying-key $TEST_DIR/master_verifying_key.pem --notary-certificate $TEST_DIR/notary_certificate.pem"
expect_failure "verify notary key (invalid master key)" "cargo run --quiet -- verify-notary --master-verifying-key $TEST_DIR/nonexistent_key.pem --notary-certificate $TEST_DIR/notary_certificate.pem"
expect_failure "verify notary key (invalid certificate)" "cargo run --quiet -- verify-notary --master-verifying-key $TEST_DIR/master_verifying_key.pem --notary-certificate $TEST_DIR/nonexistent_certificate.pem"

# Test legacy verify-delegate alias with legacy --delegate-certificate flag
run_command "verify delegate key (legacy alias)" "cargo run --quiet -- verify-delegate --master-verifying-key $TEST_DIR/master_verifying_key.pem --delegate-certificate $TEST_DIR/notary_certificate.pem"

# Test with tampered certificates
sed 's/./X/1' $TEST_DIR/notary_certificate.pem > $TEST_DIR/tampered_notary_certificate.pem
expect_failure "verify tampered notary certificate" "cargo run --quiet -- verify-notary --master-verifying-key $TEST_DIR/master_verifying_key.pem --notary-certificate $TEST_DIR/tampered_notary_certificate.pem"

# Test with empty files
touch $TEST_DIR/empty_file.pem
expect_failure "verify notary key with empty master key" "cargo run --quiet -- verify-notary --master-verifying-key $TEST_DIR/empty_file.pem --notary-certificate $TEST_DIR/notary_certificate.pem"
expect_failure "verify notary key with empty certificate" "cargo run --quiet -- verify-notary --master-verifying-key $TEST_DIR/master_verifying_key.pem --notary-certificate $TEST_DIR/empty_file.pem"

# Test with very large input files
dd if=/dev/urandom of=$TEST_DIR/large_file.pem bs=1M count=10 2>/dev/null
expect_failure "verify notary key with large master key file" "cargo run --quiet -- verify-notary --master-verifying-key $TEST_DIR/large_file.pem --notary-certificate $TEST_DIR/notary_certificate.pem"
expect_failure "verify notary key with large certificate file" "cargo run --quiet -- verify-notary --master-verifying-key $TEST_DIR/master_verifying_key.pem --notary-certificate $TEST_DIR/large_file.pem"

# Test with insufficient permissions
chmod 000 $TEST_DIR/master_verifying_key.pem
expect_failure "verify notary key with insufficient permissions" "cargo run --quiet -- verify-notary --master-verifying-key $TEST_DIR/master_verifying_key.pem --notary-certificate $TEST_DIR/notary_certificate.pem"
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
