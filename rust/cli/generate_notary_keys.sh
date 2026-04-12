#!/bin/bash
#
# Generate a set of per-amount notary certificates and signing keys for the
# Freenet donation API.
#
# Renamed from generate_delegate_keys.sh in 0.1.5 (issue freenet/web#24).
# The old name is preserved as a stub that execs this script with a
# deprecation warning. The --delegate-dir flag is accepted as a legacy
# alias for --notary-dir.

set -e

# Default values
DEFAULT_AMOUNTS=(1 5 20 50 100)
TODAYS_DATE=$(date +%Y%m%d)
DEFAULT_NOTARY_DIR="$HOME/code/freenet/keys/mnt/ghostkey-${TODAYS_DATE}/notaries"
OVERWRITE=false

usage() {
    echo "Usage: $0 --master-key <master_signing_key_file> [--notary-dir <notary_dir>] [--amounts <amount1> <amount2> ...] [--overwrite]" >&2
    exit 1
}

MASTER_KEY_FILE=""
NOTARY_DIR="$DEFAULT_NOTARY_DIR"

while [ $# -gt 0 ]; do
    case "$1" in
        --master-key)
            MASTER_KEY_FILE="$2"
            shift 2
            ;;
        --notary-dir)
            NOTARY_DIR="$2"
            shift 2
            ;;
        --delegate-dir)
            echo "warning: --delegate-dir is deprecated, use --notary-dir (freenet/web#24)" >&2
            NOTARY_DIR="$2"
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
            echo "Unknown option: $1" >&2
            usage
            ;;
    esac
done

if [ -z "$MASTER_KEY_FILE" ]; then
    echo "Error: Master key file is required." >&2
    usage
fi

if [ ${#AMOUNTS[@]} -eq 0 ]; then
    AMOUNTS=("${DEFAULT_AMOUNTS[@]}")
fi

if [ ! -f "$MASTER_KEY_FILE" ]; then
    echo "Error: Master signing key file not found: $MASTER_KEY_FILE" >&2
    exit 1
fi

mkdir -p "$NOTARY_DIR"
chmod 700 "$NOTARY_DIR"

for amount in "${AMOUNTS[@]}"; do
    current_date=$(date -u +"%Y-%m-%d %H:%M:%S")
    # NOTE: the JSON key "delegate-key-created" is baked into the cert `info`
    # field of every donation ever minted and is parsed by River's UI. DO NOT
    # rename it or we lose backward compatibility with every historical ghost
    # key in the wild. See freenet/web#24.
    info="{\"action\":\"freenet-donation\",\"amount\":$amount,\"delegate-key-created\":\"$current_date\"}"

    signing_key_file="$NOTARY_DIR/notary_signing_key_$amount.pem"
    cert_file="$NOTARY_DIR/notary_certificate_$amount.pem"

    if [ -f "$signing_key_file" ] || [ -f "$cert_file" ]; then
        if [ "$OVERWRITE" = false ]; then
            echo "Error: Output files already exist for amount $amount. Use --overwrite to replace." >&2
            exit 1
        fi
    fi

    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    if ! cargo run --quiet --manifest-path "$script_dir/Cargo.toml" -- generate-notary \
        --master-signing-key "$MASTER_KEY_FILE" \
        --info "$info" \
        --output-dir "$NOTARY_DIR" \
        --ignore-permissions >/dev/null 2>&1; then
        echo "Error: Failed to generate notary key for amount $amount" >&2
        exit 1
    fi

    # generate-notary writes unsuffixed canonical filenames; rename them to
    # the per-amount names the API expects.
    if [ ! -f "$NOTARY_DIR/notary_signing_key.pem" ] || [ ! -f "$NOTARY_DIR/notary_certificate.pem" ]; then
        echo "Error: Expected files were not generated for amount $amount" >&2
        exit 1
    fi

    mv "$NOTARY_DIR/notary_signing_key.pem" "$signing_key_file" || {
        echo "Error: Failed to rename notary signing key for amount $amount" >&2
        exit 1
    }
    mv "$NOTARY_DIR/notary_certificate.pem" "$cert_file" || {
        echo "Error: Failed to rename notary certificate for amount $amount" >&2
        exit 1
    }

    chmod 600 "$signing_key_file"
    chmod 600 "$cert_file"
done
