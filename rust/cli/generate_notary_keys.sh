#!/bin/bash
#
# Generate a set of per-amount notary certificates and signing keys for the
# Freenet donation API.
#
# Two modes:
#
#   Single set (default): one keypair per amount, dated now, written flat into
#   --notary-dir as notary_{certificate,signing_key}_{amount}.pem.
#
#   Schedule (--start-month YYYY-MM --months N): one complete set per calendar
#   month, written to --notary-dir/YYYY-MM/. This lets the master key stay
#   offline for years: generate the schedule once, and the API picks the
#   current month's directory.
#
#   In a schedule, --amounts tiers are dated monthly (the 1st of the month,
#   00:00:00 UTC) and --yearly-amounts tiers are dated yearly (1 January): one
#   keypair per year, copied into each of that year's month directories. The
#   date is visible to anyone who verifies a ghost key, so it partitions each
#   tier's anonymity set. Only tiers with plenty of donors per month can afford
#   a monthly date; on a tier with one or two donors a month, the date would
#   let whoever holds the payment records link a ghost key to its donor.
#
# Every generated pair is checked before the script moves on: the certificate
# must verify against the master verifying key (by default the Freenet master
# key compiled into the CLI, so a real run also proves the master signing key
# is the real one), its info string must be exactly what was requested, and a
# ghost key issued from the pair must verify end to end, which proves the
# signing key belongs to the certificate.
#
# Renamed from generate_delegate_keys.sh in 0.2.0 (issue freenet/web#24).
# The old name is preserved as a stub that execs this script with a
# deprecation warning. The --delegate-dir flag is accepted as a legacy
# alias for --notary-dir.

set -euo pipefail

# Together these must cover the tiers in
# hugo-site/themes/freenet/layouts/shortcodes/stripe-donation-form.html
DEFAULT_AMOUNTS=(1 5 20 50 100 500 2500 10000)
DEFAULT_SCHEDULE_MONTHLY_AMOUNTS=(1 5)
DEFAULT_SCHEDULE_YEARLY_AMOUNTS=(20 50 100 500 2500 10000)
TODAYS_DATE=$(date +%Y%m%d)
DEFAULT_NOTARY_DIR="$HOME/code/freenet/keys/mnt/ghostkey-${TODAYS_DATE}/notaries"
OVERWRITE=false

usage() {
    echo "Usage: $0 --master-key <master_signing_key_file> [--notary-dir <notary_dir>]" >&2
    echo "          [--amounts <amount1> <amount2> ...] [--overwrite]" >&2
    echo "          [--start-month YYYY-MM --months N [--yearly-amounts <amount1> ...]]" >&2
    echo "          [--master-verifying-key <file>]" >&2
    exit 1
}

MASTER_KEY_FILE=""
MASTER_VERIFYING_KEY_FILE=""
NOTARY_DIR="$DEFAULT_NOTARY_DIR"
AMOUNTS=()
AMOUNTS_SET=false
YEARLY_AMOUNTS=()
YEARLY_AMOUNTS_SET=false
START_MONTH=""
MONTHS=""

while [ $# -gt 0 ]; do
    case "$1" in
        --master-key)
            MASTER_KEY_FILE="$2"
            shift 2
            ;;
        --master-verifying-key)
            MASTER_VERIFYING_KEY_FILE="$2"
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
            AMOUNTS_SET=true
            while [[ $# -gt 0 && ! "$1" =~ ^-- ]]; do
                AMOUNTS+=("$1")
                shift
            done
            ;;
        --yearly-amounts)
            shift
            YEARLY_AMOUNTS_SET=true
            while [[ $# -gt 0 && ! "$1" =~ ^-- ]]; do
                YEARLY_AMOUNTS+=("$1")
                shift
            done
            ;;
        --start-month)
            START_MONTH="$2"
            shift 2
            ;;
        --months)
            MONTHS="$2"
            shift 2
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

if [ ! -f "$MASTER_KEY_FILE" ]; then
    echo "Error: Master signing key file not found: $MASTER_KEY_FILE" >&2
    exit 1
fi

if [ -n "$START_MONTH" ] || [ -n "$MONTHS" ]; then
    if ! [[ "$START_MONTH" =~ ^[0-9]{4}-(0[1-9]|1[0-2])$ ]] || ! [[ "$MONTHS" =~ ^[1-9][0-9]*$ ]]; then
        echo "Error: --start-month YYYY-MM and --months N (N >= 1) must be given together." >&2
        usage
    fi
    if [ "$AMOUNTS_SET" = false ]; then
        AMOUNTS=("${DEFAULT_SCHEDULE_MONTHLY_AMOUNTS[@]}")
    fi
    if [ "$YEARLY_AMOUNTS_SET" = false ]; then
        YEARLY_AMOUNTS=("${DEFAULT_SCHEDULE_YEARLY_AMOUNTS[@]}")
    fi
    for a in "${AMOUNTS[@]}"; do
        for y in "${YEARLY_AMOUNTS[@]}"; do
            if [ "$a" = "$y" ]; then
                echo "Error: amount $a is in both --amounts and --yearly-amounts." >&2
                exit 1
            fi
        done
    done
else
    if [ "$YEARLY_AMOUNTS_SET" = true ]; then
        echo "Error: --yearly-amounts only applies with --start-month/--months." >&2
        usage
    fi
    if [ "$AMOUNTS_SET" = false ]; then
        AMOUNTS=("${DEFAULT_AMOUNTS[@]}")
    fi
fi

VERIFY_ARGS=()
if [ -n "$MASTER_VERIFYING_KEY_FILE" ]; then
    VERIFY_ARGS=(--master-verifying-key "$MASTER_VERIFYING_KEY_FILE")
fi

# Build once and call the binary directly: a schedule is thousands of calls.
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GHOSTKEY=$(cargo build --release --quiet --manifest-path "$script_dir/Cargo.toml" --bin ghostkey \
    --message-format=json | jq -r 'select(.executable != null) | .executable')
if [ ! -x "$GHOSTKEY" ]; then
    echo "Error: failed to build the ghostkey CLI" >&2
    exit 1
fi
export NO_COLOR=1

mkdir -p "$NOTARY_DIR"
chmod 700 "$NOTARY_DIR"

# Holds copies of signing keys during the pair check, so keep it inside the
# (protected) output directory rather than /tmp.
scratch=$(mktemp -d -p "$NOTARY_DIR" .verify.XXXXXX)
trap 'rm -rf "$scratch"' EXIT

make_dir() {
    mkdir -p "$1"
    chmod 700 "$1"
}

# Refuse to clobber an existing pair unless --overwrite.
check_target() {
    local dir="$1" amount="$2"
    if [ -f "$dir/notary_signing_key_$amount.pem" ] || [ -f "$dir/notary_certificate_$amount.pem" ]; then
        if [ "$OVERWRITE" = false ]; then
            echo "Error: Output files already exist for amount $amount in $dir. Use --overwrite to replace." >&2
            exit 1
        fi
    fi
}

# generate_pair <dir> <amount> <created: "YYYY-MM-DD HH:MM:SS">
generate_pair() {
    local dir="$1" amount="$2" created="$3"
    # NOTE: the JSON key "delegate-key-created" is baked into the cert `info`
    # field of every donation ever minted and is parsed by the ghostkeys Vault
    # UI (and Harvest) as "YYYY-MM-DD HH:MM:SS". DO NOT rename it or we lose
    # backward compatibility with every historical ghost key in the wild.
    # See freenet/web#24.
    local info="{\"action\":\"freenet-donation\",\"amount\":$amount,\"delegate-key-created\":\"$created\"}"

    check_target "$dir" "$amount"

    rm -rf "$scratch/notary" "$scratch/ghost"
    if ! "$GHOSTKEY" generate-notary \
        --master-signing-key "$MASTER_KEY_FILE" \
        --info "$info" \
        --output-dir "$scratch/notary" \
        --ignore-permissions >/dev/null 2>&1; then
        echo "Error: Failed to generate notary key for amount $amount ($dir)" >&2
        exit 1
    fi

    local verified
    if ! verified=$("$GHOSTKEY" verify-notary "${VERIFY_ARGS[@]}" \
        --notary-certificate "$scratch/notary/notary_certificate.pem" 2>&1); then
        echo "Error: notary certificate for amount $amount does not verify against the master verifying key:" >&2
        echo "$verified" >&2
        exit 1
    fi
    if ! grep -qxF "Info: $info" <<<"$verified"; then
        echo "Error: notary certificate for amount $amount has unexpected info:" >&2
        echo "$verified" >&2
        exit 1
    fi

    if ! "$GHOSTKEY" generate-ghost-key --notary-dir "$scratch/notary" \
        --output-dir "$scratch/ghost" >/dev/null 2>&1 \
        || ! "$GHOSTKEY" verify-ghost-key "${VERIFY_ARGS[@]}" \
            --ghost-certificate "$scratch/ghost/ghost_key_certificate.pem" >/dev/null 2>&1; then
        echo "Error: a ghost key issued by the new notary for amount $amount does not verify" >&2
        exit 1
    fi

    mv "$scratch/notary/notary_signing_key.pem" "$dir/notary_signing_key_$amount.pem"
    mv "$scratch/notary/notary_certificate.pem" "$dir/notary_certificate_$amount.pem"
    chmod 600 "$dir/notary_signing_key_$amount.pem" "$dir/notary_certificate_$amount.pem"
}

# copy_pair <from_dir> <to_dir> <amount>
copy_pair() {
    local from="$1" to="$2" amount="$3"
    check_target "$to" "$amount"
    cp -p "$from/notary_signing_key_$amount.pem" "$from/notary_certificate_$amount.pem" "$to/"
}

if [ -n "$START_MONTH" ]; then
    yearly_year=""
    yearly_dir=""
    for ((i = 0; i < MONTHS; i++)); do
        month=$(date -u -d "$START_MONTH-01 +$i month" +%Y-%m)
        year=${month%-*}
        dir="$NOTARY_DIR/$month"
        make_dir "$dir"

        for amount in "${AMOUNTS[@]}"; do
            generate_pair "$dir" "$amount" "$month-01 00:00:00"
        done

        # One yearly keypair, generated in the first month of the year that the
        # schedule reaches and copied into the rest.
        if [ "$year" != "$yearly_year" ]; then
            for amount in "${YEARLY_AMOUNTS[@]}"; do
                generate_pair "$dir" "$amount" "$year-01-01 00:00:00"
            done
            yearly_year="$year"
            yearly_dir="$dir"
        else
            for amount in "${YEARLY_AMOUNTS[@]}"; do
                copy_pair "$yearly_dir" "$dir" "$amount"
            done
        fi

        echo "$month: $(( ${#AMOUNTS[@]} + ${#YEARLY_AMOUNTS[@]} )) notary keypairs in place (${#AMOUNTS[@]} monthly, ${#YEARLY_AMOUNTS[@]} yearly)"
    done
else
    for amount in "${AMOUNTS[@]}"; do
        generate_pair "$NOTARY_DIR" "$amount" "$(date -u +"%Y-%m-%d %H:%M:%S")"
    done
    echo "${#AMOUNTS[@]} notary keypairs generated and verified in $NOTARY_DIR"
fi
