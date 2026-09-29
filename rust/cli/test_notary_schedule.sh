#!/bin/bash
#
# Tests for generate_notary_keys.sh's schedule mode, run against a throwaway
# master key (never a real one). Needs cargo and jq.

set -uo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GEN="$script_dir/generate_notary_keys.sh"

pass=0
fail=0
ok() { echo "ok   - $1"; pass=$((pass + 1)); }
bad() { echo "FAIL - $1"; fail=$((fail + 1)); }
check() { if eval "$2"; then ok "$1"; else bad "$1"; fi; }
same() { cmp -s "$1" "$2"; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

cargo build --release --quiet --manifest-path "$script_dir/Cargo.toml" --bin ghostkey
GHOSTKEY=$(cargo build --release --quiet --manifest-path "$script_dir/Cargo.toml" --bin ghostkey \
    --message-format=json | jq -r 'select(.executable != null) | .executable')
"$GHOSTKEY" generate-master-key --output-dir "$tmp/master" >/dev/null
MASTER=(--master-key "$tmp/master/master_signing_key.pem"
    --master-verifying-key "$tmp/master/master_verifying_key.pem")
D="$tmp/notaries"

run() { bash "$GEN" "${MASTER[@]}" --notary-dir "$D" "$@" >"$tmp/out" 2>&1; }
info() {
    NO_COLOR=1 "$GHOSTKEY" verify-notary --master-verifying-key "$tmp/master/master_verifying_key.pem" \
        --notary-certificate "$1" | sed -n 's/^Info: //p'
}

# A schedule across a year boundary.
check "3-month schedule succeeds" "run --start-month 2026-11 --months 3"
for m in 2026-11 2026-12 2027-01; do
    check "$m has all 8 tiers" "[ \$(ls $D/$m/notary_certificate_*.pem | wc -l) -eq 8 ]"
done
check "monthly tier changes every month" \
    "! same $D/2026-11/notary_certificate_5.pem $D/2026-12/notary_certificate_5.pem"
check "monthly tier dated the 1st" \
    "[ \"\$(info $D/2026-12/notary_certificate_5.pem)\" = '{\"action\":\"freenet-donation\",\"amount\":5,\"delegate-key-created\":\"2026-12-01 00:00:00\"}' ]"
check "yearly tier shared within a year" \
    "same $D/2026-11/notary_certificate_20.pem $D/2026-12/notary_certificate_20.pem && same $D/2026-11/notary_signing_key_20.pem $D/2026-12/notary_signing_key_20.pem"
check "yearly tier changes with the year" \
    "! same $D/2026-12/notary_certificate_20.pem $D/2027-01/notary_certificate_20.pem"
check "yearly tier dated 1 January" \
    "[ \"\$(info $D/2027-01/notary_certificate_10000.pem)\" = '{\"action\":\"freenet-donation\",\"amount\":10000,\"delegate-key-created\":\"2027-01-01 00:00:00\"}' ]"

# Extending mid-year in a later run must reuse that year's yearly pairs.
check "extension run succeeds" "run --start-month 2027-02 --months 2"
check "extension reuses the year's yearly pair" \
    "same $D/2027-01/notary_certificate_100.pem $D/2027-03/notary_certificate_100.pem && same $D/2027-01/notary_signing_key_100.pem $D/2027-03/notary_signing_key_100.pem"
check "extension still mints new monthly pairs" \
    "! same $D/2027-01/notary_certificate_1.pem $D/2027-02/notary_certificate_1.pem"

# Months are only ever added.
# shellcheck disable=SC2034 # read inside check's eval
before=$(find "$D" -type f -exec sha256sum {} + | sort)
check "existing month is refused" "! run --start-month 2027-03 --months 1"
check "--overwrite is refused with a schedule" "! run --start-month 2027-04 --months 1 --overwrite"
check "refusals changed nothing" "[ \"\$(find $D -type f -exec sha256sum {} + | sort)\" = \"\$before\" ]"

# Input validation.
check "zero-padded --months is refused" "! run --start-month 2027-04 --months 08"
check "invalid amount is refused" "! run --start-month 2027-04 --months 1 --amounts 05"
check "duplicate amount is refused" "! run --start-month 2027-04 --months 1 --amounts 5 --yearly-amounts 5"
check "invalid month is refused" "! run --start-month 2027-13 --months 1"

# Wrong master key (no --master-verifying-key: checked against the real
# Freenet key) fails and leaves no month behind.
check "wrong master key is refused" \
    "! bash $GEN --master-key $tmp/master/master_signing_key.pem --notary-dir $D --start-month 2027-04 --months 1 >$tmp/out 2>&1"
check "failed run leaves no month directory" "[ ! -e $D/2027-04 ]"

# A year whose existing month dated a yearly tier some other way is an error,
# not silently shared.
# (February: a January monthly date would equal the yearly one.)
check "tier dated monthly in 2028-02" "run --start-month 2028-02 --months 1 --amounts 20 --yearly-amounts"
check "policy conflict is refused" "! run --start-month 2028-03 --months 1"
check "policy conflict leaves no month directory" "[ ! -e $D/2028-03 ]"

# publish_notary_window.sh: the rolling window the API actually reads.
PUB="$script_dir/../api/publish_notary_window.sh"
L="$tmp/live"
mkdir -m 700 "$L"
me="$(id -un):$(id -gn)"
publish() { NOTARY_WINDOW_NOW="$1" bash "$PUB" "$D" "$L" "$me" >"$tmp/pub" 2>&1; }
check "before the schedule starts: publishes the first month, no error" \
    "publish 2026-10 && [ -d $L/2026-11 ] && [ ! -e $L/2026-12 ] && grep -q 'starts at 2026-11' $tmp/pub"
check "publish succeeds" "publish 2026-12"
check "publishes past, current and next month" "[ -d $L/2026-11 ] && [ -d $L/2026-12 ] && [ -d $L/2027-01 ]"
check "does not publish beyond the window" "[ ! -e $L/2027-02 ]"
check "published month is complete" "[ \$(ls $L/2027-01 | wc -l) -eq 16 ]"
check "published keys are private" "[ -z \"\$(find $L -name 'notary_signing_key_*' ! -perm 600)\" ]"
touch -d '2000-01-01' "$L/2027-01/notary_certificate_5.pem"
check "rerun is a no-op" "publish 2026-12 && grep -q '^0 published' $tmp/pub"
check "published month is never replaced" \
    "[ \"\$(stat -c %Y $L/2027-01/notary_certificate_5.pem)\" = \"\$(date -d 2000-01-01 +%s)\" ]"
check "window advances with the month" "publish 2027-01 && [ -d $L/2027-02 ] && [ ! -e $L/2027-03 ]"
check "month missing from the schedule fails loudly" "! publish 2029-06 && grep -q 'no 2029-06' $tmp/pub"
check "no publishing directories left" "[ -z \"\$(find $L -maxdepth 1 -name '.*' ! -name . )\" ]"

check "no scratch or partial directories left" "[ -z \"\$(find $D -maxdepth 1 -name '.*' ! -name . )\" ]"
check "signing keys are private" "[ -z \"\$(find $D -name 'notary_signing_key_*' ! -perm 600)\" ]"

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
