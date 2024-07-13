#!/bin/bash

set -e

# Default values
DEFAULT_AMOUNTS=(5 20 50 100)
CERT_DIR="../../hugo-site/static/certs"
OVERWRITE=false

# Function to display usage information
usage() {
    echo "Usage: $0 --master-key <master_signing_key_file> --signing-keys-dir <signing_keys_dir> [--cert-dir <cert_dir>] [--amounts <amount1> <amount2> ...] [--overwrite]"
    echo "  --master-key <master_signing_key_file>: Path to the master signing key file"
    echo "  --signing-keys-dir <signing_keys_dir>: Directory to store delegate signing keys (must be outside the git repository)"
    echo "  --cert-dir <cert_dir>: Directory to store delegate certificates (default: $CERT_DIR)"
    echo "  --amounts: List of monetary values (default: ${DEFAULT_AMOUNTS[*]})"
    echo "  --overwrite: Allow overwriting existing files"
    exit 1
}

# Parse command-line arguments
MASTER_KEY_FILE=""
SIGNING_KEYS_DIR=""

while [ $# -gt 0 ]; do
    case "$1" in
        --master-key)
            MASTER_KEY_FILE="$2"
            shift 2
            ;;
        --signing-keys-dir)
            SIGNING_KEYS_DIR="$2"
            shift 2
            ;;
        --cert-dir)
            CERT_DIR="$2"
            shift 2
            ;;
        --amounts)
            shift
            AMOUNTS=()
            while [[ $# -gt 0 && ! "$1" =~ ^-- ]]; do
                AMOUNTS+=("$1")
                shift
            done
            ;;
        --overwrite)
            OVERWRITE=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

# Check if required arguments are provided
if [ -z "$MASTER_KEY_FILE" ] || [ -z "$SIGNING_KEYS_DIR" ]; then
    echo "Error: Master key file and signing keys directory are required."
    usage
fi

# Use default amounts if not provided
if [ ${#AMOUNTS[@]} -eq 0 ]; then
    AMOUNTS=("${DEFAULT_AMOUNTS[@]}")
fi

# Validate master signing key file
if [ ! -f "$MASTER_KEY_FILE" ]; then
    echo "Error: Master signing key file not found: $MASTER_KEY_FILE" >&2
    exit 1
fi

# Create output directories
mkdir -p "$SIGNING_KEYS_DIR"
mkdir -p "$CERT_DIR"

# Set appropriate permissions for signing keys directory
chmod 700 "$SIGNING_KEYS_DIR"

# Generate delegate keys for each amount
for amount in "${AMOUNTS[@]}"; do
    current_date=$(date -u +"%Y-%m-%d %H:%M:%S")
    info="{\"action\":\"freenet-donation\",\"amount\":$amount,\"delegate-key-created\":\"$current_date\"}"
    
    signing_key_file="$SIGNING_KEYS_DIR/delegate_signing_key_$amount.pem"
    cert_file="$CERT_DIR/delegate_certificate_$amount.pem"
    
    if [ -f "$signing_key_file" ] || [ -f "$cert_file" ]; then
        if [ "$OVERWRITE" = false ]; then
            echo "Error: Output files already exist for amount $amount. Use --overwrite to replace." >&2
            exit 1
        fi
    fi
    
    echo "Generating delegate key for amount: $amount"
    cargo run --quiet -- generate-delegate-key "$MASTER_KEY_FILE" "$info" "$SIGNING_KEYS_DIR" > /dev/null
    
    # Rename the generated files
    mv "$SIGNING_KEYS_DIR/delegate_signing_key.pem" "$signing_key_file"
    mv "$SIGNING_KEYS_DIR/delegate_certificate.pem" "$cert_file"
    
    # Set appropriate permissions for the signing key
    chmod 600 "$signing_key_file"
done

echo "Delegate keys generated successfully."
