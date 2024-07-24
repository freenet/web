#!/bin/bash

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to run a command and check its exit status
run_test() {
    local description="$1"
    local command="$2"
    local expected_status="$3"

    echo "Running test: $description"
    echo "Command: $command"
    
    output=$(eval "$command" 2>&1)
    status=$?

    echo "Command exit status: $status"
    echo "Command output:"
    echo "$output"

    if [ $status -eq $expected_status ]; then
        echo -e "${GREEN}PASSED${NC}"
    else
        echo -e "${RED}FAILED${NC}"
        echo "Expected status: $expected_status, Got: $status"
    fi
    echo "----------------------------------------"
}

# Function to check if files exist
check_files() {
    local dir="$1"
    shift
    for file in "$@"; do
        if [ -f "$dir/$file" ]; then
            echo "File $file exists in $dir"
        else
            echo "File $file does not exist in $dir"
        fi
    done
}

# Create a temporary directory
temp_dir=$(mktemp -d)
echo "Using temporary directory: $temp_dir"

# Test generate-master-key
run_test "generate-master-key" "cargo run --bin ghostkey -- generate-master-key --output-dir $temp_dir/master" 0
check_files "$temp_dir" "SERVER_MASTER_KEY" "SERVER_MASTER_KEY.pub"

# Test generate-delegate
run_test "generate-delegate" "cargo run --bin ghostkey -- generate-delegate --master-signing-key $temp_dir/master/master_signing_key.pem --info 'Test Delegate' --output-dir $temp_dir/delegate" 0

# Test verify-delegate (should succeed)
run_test "verify-delegate (valid)" "cargo run --bin ghostkey -- verify-delegate --master-verifying-key $temp_dir/SERVER_MASTER_KEY.pub --delegate-certificate $temp_dir/DELEGATE_CERTIFICATE" 0

# Test verify-delegate with invalid certificate (should fail)
echo "Invalid certificate" > $temp_dir/INVALID_CERTIFICATE
run_test "verify-delegate (invalid)" "cargo run --bin ghostkey -- verify-delegate --master-verifying-key $temp_dir/SERVER_MASTER_KEY.pub --delegate-certificate $temp_dir/INVALID_CERTIFICATE" 1

# Test generate-ghost-key (not implemented yet, should fail)
run_test "generate-ghost-key" "cargo run --bin ghostkey -- generate-ghost-key --delegate-dir $temp_dir --output-dir $temp_dir" 1

# Test verify-ghost-key (not implemented yet, should fail)
run_test "verify-ghost-key" "cargo run --bin ghostkey -- verify-ghost-key --master-verifying-key $temp_dir/SERVER_MASTER_KEY.pub --ghost-certificate $temp_dir/GHOST_CERTIFICATE" 1

# Clean up
echo "Cleaning up temporary directory"
rm -rf "$temp_dir"

echo "All tests completed."