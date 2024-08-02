#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Counters for passed and failed tests
pass_count=0
fail_count=0
total_tests=0

# Function to run a command and check its exit status
run_test() {
    local description="$1"
    local command="$2"
    local expected_status="$3"

    ((total_tests++))
    echo -n "Running test: $description... "

    output=$(eval "$command" 2>&1)
    status=$?

    if [ $status -eq $expected_status ]; then
        echo -e "${GREEN}Ok${NC}"
        ((pass_count++))
    else
        echo -e "${RED}Failed${NC}"
        echo "Command: $command"
        echo "Expected status: $expected_status, Got: $status"
        echo "Command output:"
        echo "$output"
        echo "----------------------------------------"
        ((fail_count++))
    fi
}

# Function to check if files exist
check_files() {
    local dir="$1"
    shift
    for file in "$@"; do
        if [ ! -f "$dir/$file" ]; then
            echo -e "${RED}File $file does not exist in $dir${NC}"
            ((fail_count++))
        fi
    done
}

# Create a temporary directory
temp_dir=$(mktemp -d)
echo "Using temporary directory: $temp_dir"

# Test generate-master-key
run_test "Generate master key" "cargo run --bin ghostkey -- generate-master-key --output-dir $temp_dir/master-1" 0
check_files "$temp_dir/master-1" "master_signing_key.pem" "master_verifying_key.pem"

# Test generate-delegate
run_test "Generate delegate" "cargo run --bin ghostkey -- generate-delegate --master-signing-key $temp_dir/master-1/master_signing_key.pem --info 'Test Delegate' --output-dir $temp_dir/delegate-1" 0
check_files "$temp_dir/delegate-1" "delegate_certificate.pem" "delegate_signing_key.pem"

# Test verify-delegate (should succeed)
run_test "Verify delegate with valid certificate" "cargo run --bin ghostkey -- verify-delegate --master-verifying-key $temp_dir/master-1/master_verifying_key.pem --delegate-certificate $temp_dir/delegate-1/delegate_certificate.pem" 0

# Test verify-delegate with invalid certificate (should fail)
echo "Invalid certificate" > $temp_dir/INVALID_CERTIFICATE
run_test "Verify delegate with invalid certificate (should fail)" "cargo run --bin ghostkey -- verify-delegate --master-verifying-key $temp_dir/master-1/master_verifying_key.pem --delegate-certificate $temp_dir/INVALID_CERTIFICATE" 1

# Test generate-ghost-key
run_test "Generate ghost key" "cargo run --bin ghostkey -- generate-ghost-key --delegate-dir $temp_dir/delegate-1 --output-dir $temp_dir/ghost-1" 0
check_files "$temp_dir/ghost-1" "ghost_key_certificate.pem" "ghost_key_signing_key.pem"

# Test verify-ghost-key
run_test "Verify ghost key" "cargo run --bin ghostkey -- verify-ghost-key --master-verifying-key $temp_dir/master-1/master_verifying_key.pem --ghost-certificate $temp_dir/ghost-1/ghost_key_certificate.pem" 0

# Generate a second master key
run_test "Generate second master key" "cargo run --bin ghostkey -- generate-master-key --output-dir $temp_dir/master-2" 0
check_files "$temp_dir/master-2" "master_signing_key.pem" "master_verifying_key.pem"

# Test verify-delegate with wrong master key (should fail)
run_test "Verify delegate with wrong master key (should fail)" "cargo run --bin ghostkey -- verify-delegate --master-verifying-key $temp_dir/master-2/master_verifying_key.pem --delegate-certificate $temp_dir/delegate-1/delegate_certificate.pem" 1

# Test verify-ghost-key with wrong master key (should fail)
run_test "Verify ghost key with wrong master key (should fail)" "cargo run --bin ghostkey -- verify-ghost-key --master-verifying-key $temp_dir/master-2/master_verifying_key.pem --ghost-certificate $temp_dir/ghost-1/ghost_key_certificate.pem" 1

# Clean up
echo "Cleaning up temporary directory"
rm -rf "$temp_dir"

# Summary of test results
echo "----------------------------------------"
echo "Test Summary: ${pass_count}/${total_tests} tests passed, ${fail_count} tests failed."

if [ $fail_count -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
else
    echo -e "${RED}Some tests failed.${NC}"
fi
