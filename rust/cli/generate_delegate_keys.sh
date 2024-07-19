#!/bin/bash

set -e
set -x  # Enable debugging output

# Default values
DEFAULT_AMOUNTS=(5 20 50 100)
DEFAULT_DELEGATE_DIR="$HOME/.config/ghostkey/delegates"
OVERWRITE=false

# Function to display usage information
usage() {
    echo "Usage: $0 --master-key <master_signing_key_file> [--delegate-dir <delegate_dir>] [--amounts <amount1> <amount2> ...] [--overwrite]"
    echo "  --master-key <master_signing_key_file>: Path to the master signing key file"
    echo "  --delegate-dir <delegate_dir>: Directory to store delegate keys and certificates (default: $DEFAULT_DELEGATE_DIR)"
    echo "  --amounts: List of monetary values (default: ${DEFAULT_AMOUNTS[*]})"
    echo "  --overwrite: Allow overwriting existing files"
    exit 1
}

# Parse command-line arguments
MASTER_KEY_FILE=""
DELEGATE_DIR="$DEFAULT_DELEGATE_DIR"

while [ $# -gt 0 ]; do
    case "$1" in
        --master-key)
            MASTER_KEY_FILE="$2"
            shift 2
            ;;
        --delegate-dir)
            DELEGATE_DIR="$2"
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
if [ -z "$MASTER_KEY_FILE" ]; then
    echo "Error: Master key file is required."
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

echo "Master key file: $MASTER_KEY_FILE"
echo "Delegate directory: $DELEGATE_DIR"
echo "Amounts: ${AMOUNTS[*]}"

# Create output directory
mkdir -p "$DELEGATE_DIR"

# Set appropriate permissions for delegate directory
chmod 700 "$DELEGATE_DIR"

# Generate delegate keys for each amount
for amount in "${AMOUNTS[@]}"; do
    current_date=$(date -u +"%Y-%m-%d %H:%M:%S")
    info="{\"action\":\"freenet-donation\",\"amount\":$amount,\"delegate-key-created\":\"$current_date\"}"
    
    signing_key_file="$DELEGATE_DIR/delegate_signing_key_$amount.pem"
    cert_file="$DELEGATE_DIR/delegate_certificate_$amount.pem"
    
    if [ -f "$signing_key_file" ] || [ -f "$cert_file" ]; then
        if [ "$OVERWRITE" = false ]; then
            echo "Error: Output files already exist for amount $amount. Use --overwrite to replace." >&2
            exit 1
        fi
    fi
    
    echo "Generating delegate key for amount: $amount"
    if ! cargo run --quiet -- generate-delegate-key \
        --master-signing-key-file "$MASTER_KEY_FILE" \
        --info "$info" \
        --output-dir "$DELEGATE_DIR"; then
        echo "Error: Failed to generate delegate key for amount $amount" >&2
        exit 1
    fi
    
    # Check if files were generated
    if [ ! -f "$DELEGATE_DIR/delegate_signing_key.pem" ] || [ ! -f "$DELEGATE_DIR/delegate_certificate.pem" ]; then
        echo "Error: Expected files were not generated for amount $amount" >&2
        exit 1
    }
    
    # Rename the generated files
    mv "$DELEGATE_DIR/delegate_signing_key.pem" "$signing_key_file" || {
        echo "Error: Failed to rename delegate signing key for amount $amount" >&2
        exit 1
    }
    mv "$DELEGATE_DIR/delegate_certificate.pem" "$cert_file" || {
        echo "Error: Failed to rename delegate certificate for amount $amount" >&2
        exit 1
    }
    
    # Set appropriate permissions for the signing key and certificate
    chmod 600 "$signing_key_file"
    chmod 600 "$cert_file"
    
    echo "Generated delegate key for amount $amount"
done

echo "Delegate keys generated successfully in $DELEGATE_DIR"
