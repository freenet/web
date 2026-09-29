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
#   current month's directory (see rust/api/README.md for installing it).
#
#   In a schedule, --amounts tiers are dated monthly (the 1st of the month,
#   00:00:00 UTC) and --quarterly-amounts tiers are dated quarterly (1 January,
#   April, July or October): one keypair per quarter, shared by that quarter's
#   three month directories. The date is visible to anyone who verifies a ghost
#   key, so it partitions each tier's anonymity set. Only tiers with plenty of
#   donors per month can afford a monthly date; on a tier with one or two
#   donors a month, the date would let whoever holds the payment records link a
#   ghost key to its donor. Quarterly rather than yearly keeps the date useful
#   to an application that wants a recent key: a key is never shown more than
#   three months older than it is.
#
#   A schedule run only ever adds months. An existing month is an error (it may
#   be live, and replacing its keys would break donations quoted from it), and
#   a quarter that already has a month in --notary-dir reuses that month's
#   quarterly keypairs instead of minting a second set, so a schedule can be
#   extended later without splitting a quarter's anonymity set. Each month is built in a
#   hidden directory and renamed into place only when complete.
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
DEFAULT_SCHEDULE_QUARTERLY_AMOUNTS=(20 50 100 500 2500 10000)
TODAYS_DATE=$(date +%Y%m%d)
DEFAULT_NOTARY_DIR="$HOME/code/freenet/keys/mnt/ghostkey-${TODAYS_DATE}/notaries"
OVERWRITE=false

usage() {
    echo "Usage: $0 --master-key <master_signing_key_file> [--notary-dir <notary_dir>]" >&2
    echo "          [--amounts <amount1> <amount2> ...] [--overwrite]" >&2
    echo "          [--start-month YYYY-MM --months N [--quarterly-amounts <amount1> ...]]" >&2
    echo "          [--master-verifying-key <file>]" >&2
    exit 1
}

MASTER_KEY_FILE=""
MASTER_VERIFYING_KEY_FILE=""
NOTARY_DIR="$DEFAULT_NOTARY_DIR"
NOTARY_DIR_SET=false
AMOUNTS=()
AMOUNTS_SET=false
QUARTERLY_AMOUNTS=()
QUARTERLY_AMOUNTS_SET=false
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
            NOTARY_DIR_SET=true
            shift 2
            ;;
        --delegate-dir)
            echo "warning: --delegate-dir is deprecated, use --notary-dir (freenet/web#24)" >&2
            NOTARY_DIR="$2"
            NOTARY_DIR_SET=true
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
        --quarterly-amounts)
            shift
            QUARTERLY_AMOUNTS_SET=true
            while [[ $# -gt 0 && ! "$1" =~ ^-- ]]; do
                QUARTERLY_AMOUNTS+=("$1")
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

SCHEDULE=false
if [ -n "$START_MONTH" ] || [ -n "$MONTHS" ]; then
    SCHEDULE=true
    if ! [[ "$START_MONTH" =~ ^[0-9]{4}-(0[1-9]|1[0-2])$ ]] || ! [[ "$MONTHS" =~ ^[1-9][0-9]*$ ]]; then
        echo "Error: --start-month YYYY-MM and --months N (N >= 1) must be given together." >&2
        usage
    fi
    # The default directory is named after today's date, so a later extension
    # run would start an empty schedule and mint a second set of quarterly keys.
    if [ "$NOTARY_DIR_SET" = false ]; then
        echo "Error: a schedule needs an explicit --notary-dir (the same one every run)." >&2
        exit 1
    fi
    if [ "$OVERWRITE" = true ]; then
        echo "Error: --overwrite is not supported with a schedule; months are only ever added." >&2
        exit 1
    fi
    if [ "$AMOUNTS_SET" = false ]; then
        AMOUNTS=("${DEFAULT_SCHEDULE_MONTHLY_AMOUNTS[@]}")
    fi
    if [ "$QUARTERLY_AMOUNTS_SET" = false ]; then
        QUARTERLY_AMOUNTS=("${DEFAULT_SCHEDULE_QUARTERLY_AMOUNTS[@]}")
    fi
else
    if [ "$QUARTERLY_AMOUNTS_SET" = true ]; then
        echo "Error: --quarterly-amounts only applies with --start-month/--months." >&2
        usage
    fi
    if [ "$AMOUNTS_SET" = false ]; then
        AMOUNTS=("${DEFAULT_AMOUNTS[@]}")
    fi
fi

# The API looks pairs up by the amount formatted as an integer, and the amount
# is interpolated into the certificate's JSON, so only plain positive integers.
ALL_AMOUNTS=(${AMOUNTS[@]+"${AMOUNTS[@]}"} ${QUARTERLY_AMOUNTS[@]+"${QUARTERLY_AMOUNTS[@]}"})
if [ ${#ALL_AMOUNTS[@]} -eq 0 ]; then
    echo "Error: no amounts to generate." >&2
    exit 1
fi
for a in "${ALL_AMOUNTS[@]}"; do
    if ! [[ "$a" =~ ^[1-9][0-9]*$ ]]; then
        echo "Error: invalid amount '$a' (whole dollars, no leading zeros)." >&2
        exit 1
    fi
done
if [ "$(printf '%s\n' "${ALL_AMOUNTS[@]}" | sort | uniq -d)" != "" ]; then
    echo "Error: an amount is listed more than once (across --amounts and --quarterly-amounts)." >&2
    exit 1
fi

VERIFY_ARGS=()
if [ -n "$MASTER_VERIFYING_KEY_FILE" ]; then
    VERIFY_ARGS=(--master-verifying-key "$MASTER_VERIFYING_KEY_FILE")
fi

# Build once and call the binary directly: a schedule is thousands of calls.
# The first build shows any compiler errors; the second is a no-op that reports
# the binary's path.
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cargo build --release --quiet --manifest-path "$script_dir/Cargo.toml" --bin ghostkey
GHOSTKEY=$(cargo build --release --quiet --manifest-path "$script_dir/Cargo.toml" --bin ghostkey \
    --message-format=json | jq -r 'select(.executable != null) | .executable')
if [ ! -x "$GHOSTKEY" ]; then
    echo "Error: failed to build the ghostkey CLI" >&2
    exit 1
fi
export NO_COLOR=1

ghostkey_verify() {
    "$GHOSTKEY" "$1" ${VERIFY_ARGS[@]+"${VERIFY_ARGS[@]}"} "${@:2}"
}

mkdir -p "$NOTARY_DIR"
chmod 700 "$NOTARY_DIR"

# A schedule keeps one dating policy for its whole life. Moving a tier between
# monthly and quarterly part-way would split that tier's anonymity set, so the
# first run records the policy and later runs must match it.
if [ "$SCHEDULE" = true ]; then
    sorted() { printf '%s\n' "$@" | sort -n | tr '\n' ' '; }
    policy="monthly: $(sorted ${AMOUNTS[@]+"${AMOUNTS[@]}"})| quarterly: $(sorted ${QUARTERLY_AMOUNTS[@]+"${QUARTERLY_AMOUNTS[@]}"})"
    policy_file="$NOTARY_DIR/.schedule-policy"
    if [ -f "$policy_file" ] && [ "$(cat "$policy_file")" != "$policy" ]; then
        echo "Error: this schedule was generated with policy '$(cat "$policy_file")';" >&2
        echo "       this run asks for '$policy'. Pass the same --amounts/--quarterly-amounts." >&2
        exit 1
    fi
    # Recorded once the first month is in place (below), so a run that fails
    # outright does not pin whatever it was given.
fi

# Holds copies of signing keys during the pair check, so keep it inside the
# (protected) output directory rather than /tmp.
scratch=$(mktemp -d -p "$NOTARY_DIR" .verify.XXXXXX)
partial=""
trap 'rm -rf "$scratch" ${partial:+"$partial"}' EXIT

info_for() {
    local amount="$1" created="$2"
    # NOTE: the JSON key "delegate-key-created" is baked into the cert `info`
    # field of every donation ever minted and is parsed by the ghostkeys Vault
    # UI (and Harvest) as "YYYY-MM-DD HH:MM:SS". DO NOT rename it or we lose
    # backward compatibility with every historical ghost key in the wild.
    # See freenet/web#24.
    echo "{\"action\":\"freenet-donation\",\"amount\":$amount,\"delegate-key-created\":\"$created\"}"
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

# Fail unless <cert> verifies against the master key with exactly <info>.
verify_cert() {
    local cert="$1" info="$2" verified
    if ! verified=$(ghostkey_verify verify-notary --notary-certificate "$cert" 2>&1); then
        echo "Error: $cert does not verify against the master verifying key:" >&2
        echo "$verified" >&2
        exit 1
    fi
    if ! grep -qxF "Info: $info" <<<"$verified"; then
        echo "Error: $cert has unexpected info (wanted $info):" >&2
        echo "$verified" >&2
        exit 1
    fi
}

# Fail unless a ghost key issued from the pair in <dir> (canonical
# notary_{certificate,signing_key}.pem names) verifies end to end, which proves
# the signing key belongs to the certificate.
verify_pair() {
    local dir="$1" what="$2"
    rm -rf "$scratch/ghost"
    if ! "$GHOSTKEY" generate-ghost-key --notary-dir "$dir" \
        --output-dir "$scratch/ghost" >/dev/null 2>&1 \
        || ! ghostkey_verify verify-ghost-key \
            --ghost-certificate "$scratch/ghost/ghost_key_certificate.pem" >/dev/null 2>&1; then
        echo "Error: a ghost key issued by $what does not verify" >&2
        exit 1
    fi
}

# generate_pair <dir> <amount> <created: "YYYY-MM-DD HH:MM:SS">
generate_pair() {
    local dir="$1" amount="$2" created="$3"
    local info
    info=$(info_for "$amount" "$created")

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

    verify_cert "$scratch/notary/notary_certificate.pem" "$info"

    verify_pair "$scratch/notary" "the new notary for amount $amount"

    mv "$scratch/notary/notary_signing_key.pem" "$dir/notary_signing_key_$amount.pem"
    mv "$scratch/notary/notary_certificate.pem" "$dir/notary_certificate_$amount.pem"
    chmod 600 "$dir/notary_signing_key_$amount.pem" "$dir/notary_certificate_$amount.pem"
}

# The first month (YYYY-MM) of the quarter <month> is in.
quarter_start() {
    local month="$1" m
    m=$((10#${month#*-}))
    printf '%s-%02d' "${month%-*}" $(((m - 1) / 3 * 3 + 1))
}

# The completed month directory of the quarter starting <qstart> already
# holding the quarterly pair for <amount>, if any. Verified, so a month
# generated under a different policy (say, this amount dated monthly) is an
# error rather than silently shared.
existing_quarterly_dir() {
    local qstart="$1" amount="$2" d k
    for k in 0 1 2; do
        d="$NOTARY_DIR/$(date -u -d "$qstart-01 +$k month" +%Y-%m)"
        if [ -f "$d/notary_certificate_$amount.pem" ]; then
            verify_cert "$d/notary_certificate_$amount.pem" "$(info_for "$amount" "$qstart-01 00:00:00")"
            # And that its signing key really is that certificate's, before
            # copying the pair into more months.
            rm -rf "$scratch/reuse"
            mkdir -m 700 "$scratch/reuse"
            cp "$d/notary_certificate_$amount.pem" "$scratch/reuse/notary_certificate.pem"
            cp "$d/notary_signing_key_$amount.pem" "$scratch/reuse/notary_signing_key.pem"
            verify_pair "$scratch/reuse" "the existing $qstart quarter pair for amount $amount in $d"
            rm -rf "$scratch/reuse"
            echo "$d"
            return
        fi
    done
}

if [ "$SCHEDULE" = true ]; then
    for ((i = 0; i < MONTHS; i++)); do
        month=$(date -u -d "$START_MONTH-01 +$i month" +%Y-%m)
        qstart=$(quarter_start "$month")
        dir="$NOTARY_DIR/$month"
        if [ -e "$dir" ]; then
            echo "Error: $dir already exists. A schedule run only adds months; start after the last one." >&2
            exit 1
        fi

        partial="$NOTARY_DIR/.$month.partial"
        rm -rf "$partial"
        mkdir -m 700 "$partial"

        for amount in ${AMOUNTS[@]+"${AMOUNTS[@]}"}; do
            generate_pair "$partial" "$amount" "$month-01 00:00:00"
        done

        reused=0
        for amount in ${QUARTERLY_AMOUNTS[@]+"${QUARTERLY_AMOUNTS[@]}"}; do
            src=$(existing_quarterly_dir "$qstart" "$amount")
            if [ -n "$src" ]; then
                cp -p "$src/notary_signing_key_$amount.pem" "$src/notary_certificate_$amount.pem" "$partial/"
                reused=$((reused + 1))
            else
                generate_pair "$partial" "$amount" "$qstart-01 00:00:00"
            fi
        done

        mv "$partial" "$dir"
        partial=""
        [ -f "$policy_file" ] || echo "$policy" >"$policy_file"
        echo "$month: ${#AMOUNTS[@]} monthly and ${#QUARTERLY_AMOUNTS[@]} quarterly notary keypairs ($reused reused from earlier in the quarter)"
    done
else
    for amount in "${AMOUNTS[@]}"; do
        generate_pair "$NOTARY_DIR" "$amount" "$(date -u +"%Y-%m-%d %H:%M:%S")"
    done
    echo "${#AMOUNTS[@]} notary keypairs generated and verified in $NOTARY_DIR"
fi
