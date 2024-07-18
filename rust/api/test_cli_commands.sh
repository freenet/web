#!/bin/bash

set -e

# Initialize counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Check if verbose output is requested
VERBOSE=0
for arg in "$@"; do
    case $arg in
        --verbose)
            VERBOSE=1
            ;;
    esac
done

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

# Start the API server (assuming it's a cargo run command)
echo "Starting API server..."
cargo run --bin api &
API_PID=$!

# Wait for the server to start
sleep 5

# Test API endpoints
run_command "API health check" "curl -s -o /dev/null -w '%{http_code}' http://localhost:8000/health"
run_command "Generate master key" "curl -s -X POST http://localhost:8000/generate-master-key"
run_command "Generate delegate key" "curl -s -X POST -H 'Content-Type: application/json' -d '{\"master_signing_key\":\"test_key\", \"info\":\"test_info\"}' http://localhost:8000/generate-delegate-key"
run_command "Generate ghost key" "curl -s -X POST -H 'Content-Type: application/json' -d '{\"delegate_certificate\":\"test_cert\", \"delegate_signing_key\":\"test_key\"}' http://localhost:8000/generate-ghost-key"
run_command "Validate delegate key" "curl -s -X POST -H 'Content-Type: application/json' -d '{\"master_verifying_key\":\"test_key\", \"delegate_certificate\":\"test_cert\"}' http://localhost:8000/validate-delegate-key"
run_command "Validate ghost key" "curl -s -X POST -H 'Content-Type: application/json' -d '{\"master_verifying_key\":\"test_key\", \"ghost_certificate\":\"test_cert\"}' http://localhost:8000/validate-ghost-key"

# Stop the API server
echo "Stopping API server..."
kill $API_PID

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
