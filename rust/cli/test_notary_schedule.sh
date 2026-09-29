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

# A schedule across a year (and quarter) boundary.
check "3-month schedule succeeds" "run --start-month 2026-11 --months 3"
for m in 2026-11 2026-12 2027-01; do
    check "$m has all 8 tiers" "[ \$(ls $D/$m/notary_certificate_*.pem | wc -l) -eq 8 ]"
done
check "monthly tier changes every month" \
    "! same $D/2026-11/notary_certificate_5.pem $D/2026-12/notary_certificate_5.pem"
check "monthly tier dated the 1st" \
    "[ \"\$(info $D/2026-12/notary_certificate_5.pem)\" = '{\"action\":\"freenet-donation\",\"amount\":5,\"delegate-key-created\":\"2026-12-01 00:00:00\"}' ]"
check "quarterly tier shared within a quarter" \
    "same $D/2026-11/notary_certificate_20.pem $D/2026-12/notary_certificate_20.pem && same $D/2026-11/notary_signing_key_20.pem $D/2026-12/notary_signing_key_20.pem"
check "quarterly tier changes with the quarter" \
    "! same $D/2026-12/notary_certificate_20.pem $D/2027-01/notary_certificate_20.pem"
check "quarterly tier dated the first day of its quarter" \
    "[ \"\$(info $D/2026-11/notary_certificate_10000.pem)\" = '{\"action\":\"freenet-donation\",\"amount\":10000,\"delegate-key-created\":\"2026-10-01 00:00:00\"}' ] && [ \"\$(info $D/2027-01/notary_certificate_10000.pem)\" = '{\"action\":\"freenet-donation\",\"amount\":10000,\"delegate-key-created\":\"2027-01-01 00:00:00\"}' ]"

# Extending mid-quarter in a later run must reuse that quarter's pairs.
check "extension run succeeds" "run --start-month 2027-02 --months 1"
check "extension reuses the quarter's pair" \
    "same $D/2027-01/notary_certificate_100.pem $D/2027-02/notary_certificate_100.pem && same $D/2027-01/notary_signing_key_100.pem $D/2027-02/notary_signing_key_100.pem"
check "extension still mints new monthly pairs" \
    "! same $D/2027-01/notary_certificate_1.pem $D/2027-02/notary_certificate_1.pem"

# A reused quarterly pair is checked as a pair, not just its certificate: swap
# in another valid signing key and an extension into that quarter must refuse it.
cp -p "$D/2027-01/notary_signing_key_100.pem" "$tmp/key100.bak"
cp "$D/2027-01/notary_signing_key_5.pem" "$D/2027-01/notary_signing_key_100.pem"
cp "$D/2027-01/notary_signing_key_5.pem" "$D/2027-02/notary_signing_key_100.pem"
check "a reused quarterly pair whose key does not match is refused" \
    "! run --start-month 2027-03 --months 1 && grep -q 'a ghost key issued by the existing 2027-01 quarter pair' $tmp/out && [ ! -e $D/2027-03 ]"
for m in 2027-01 2027-02; do cp -p "$tmp/key100.bak" "$D/$m/notary_signing_key_100.pem"; done
check "with the pair restored, the quarter can be completed" "run --start-month 2027-03 --months 1"
check "the completed quarter shares one pair" \
    "same $D/2027-01/notary_certificate_100.pem $D/2027-03/notary_certificate_100.pem"

# Months are only ever added.
# shellcheck disable=SC2034 # read inside check's eval
before=$(find "$D" -type f -exec sha256sum {} + | sort)
check "existing month is refused" "! run --start-month 2027-03 --months 1"
check "--overwrite is refused with a schedule" "! run --start-month 2027-04 --months 1 --overwrite"
check "refusals changed nothing" "[ \"\$(find $D -type f -exec sha256sum {} + | sort)\" = \"\$before\" ]"

# Input validation.
check "zero-padded --months is refused" "! run --start-month 2027-04 --months 08"
check "invalid amount is refused" "! run --start-month 2027-04 --months 1 --amounts 05"
check "duplicate amount is refused" "! run --start-month 2027-04 --months 1 --amounts 5 --quarterly-amounts 5"
check "invalid month is refused" "! run --start-month 2027-13 --months 1"
check "schedule without --notary-dir is refused" \
    "! bash $GEN --master-key $tmp/master/master_signing_key.pem --start-month 2027-04 --months 1 >$tmp/out 2>&1 && grep -q 'explicit --notary-dir' $tmp/out"

# Wrong master key (no --master-verifying-key: checked against the real
# Freenet key) fails and leaves no month behind.
check "wrong master key is refused" \
    "! bash $GEN --master-key $tmp/master/master_signing_key.pem --notary-dir $D --start-month 2027-04 --months 1 >$tmp/out 2>&1"
check "failed run leaves no month directory" "[ ! -e $D/2027-04 ]"

# Changing the dating policy of an existing schedule is refused, both ways.
check "quarterly to monthly is refused" "! run --start-month 2027-04 --months 1 --amounts 1 5 20 --quarterly-amounts 50 100 500 2500 10000"
check "monthly to quarterly is refused" "! run --start-month 2027-04 --months 1 --amounts 1 --quarterly-amounts 5 20 50 100 500 2500 10000"
check "refused policy change leaves no month directory" "[ ! -e $D/2027-04 ]"

# A first run that fails outright does not pin its policy.
F="$tmp/failed-first"
check "failed first run" \
    "! bash $GEN --master-key $tmp/master/master_signing_key.pem --notary-dir $F --start-month 2027-01 --months 1 --amounts 1 >$tmp/out 2>&1"
check "failed first run records no policy" "[ ! -e $F/.schedule-policy ]"
check "a corrected retry with other amounts is accepted" \
    "bash $GEN ${MASTER[*]} --notary-dir $F --start-month 2027-01 --months 1 >$tmp/out 2>&1 && [ -f $F/.schedule-policy ]"

# Belt and braces for a schedule without a policy file: a quarter whose
# existing month dated a quarterly tier some other way is an error, not silently
# shared. (February: a monthly date on a quarter's first month would equal the
# quarterly one.)
P="$tmp/nopolicy"
runp() { bash "$GEN" "${MASTER[@]}" --notary-dir "$P" "$@" >"$tmp/out" 2>&1; }
check "tier dated monthly in 2028-02" "runp --start-month 2028-02 --months 1 --amounts 20 --quarterly-amounts"
rm -f "$P/.schedule-policy"
check "conflicting existing month is refused" "! runp --start-month 2028-03 --months 1"
check "conflict leaves no month directory" "[ ! -e $P/2028-03 ]"

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
ln -s /nonexistent "$L/2027-03"
check "a planted symlink is neither followed nor replaced" "publish 2027-02 && [ -L $L/2027-03 ]"
# The notary directory's owner can swap its path for a symlink onto another
# filesystem, where mv would fall back to a copy that follows symlinks.
shm=$(mktemp -d -p /dev/shm)
ln -s "$shm" "$tmp/live-elsewhere"
check "notary dir on another filesystem via symlink is refused" \
    "! NOTARY_WINDOW_NOW=2027-01 bash $PUB $D $tmp/live-elsewhere $me >$tmp/pub 2>&1 && grep -q 'same filesystem' $tmp/pub && [ -z \"\$(ls -A $shm)\" ]"
rm -rf "$shm"
chmod 755 "$D"
check "a schedule that is not private is refused" "! publish 2027-01 && grep -q 'mode 700' $tmp/pub"
chmod 700 "$D"
mkdir -m 777 "$tmp/open" && chmod 777 "$tmp/open" && mkdir -m 700 "$tmp/open/schedule"
check "a schedule under a directory others can rewrite is refused" \
    "! NOTARY_WINDOW_NOW=2027-01 bash $PUB $tmp/open/schedule $L $me >$tmp/pub 2>&1 && grep -q 'writable by someone else' $tmp/pub"
check "numeric owner is accepted" "NOTARY_WINDOW_NOW=2027-01 bash $PUB $D $L $(id -u):$(id -g) >$tmp/pub 2>&1"
# Modes are set on the way in, not inherited from the source.
S="$tmp/loose"
L2="$tmp/live2"
mkdir -m 700 "$S" "$L2"
cp -R "$D/2027-01" "$S/2027-01"
chmod 777 "$S/2027-01"
chmod 755 "$S"/2027-01/*
check "loose source modes are normalised" \
    "NOTARY_WINDOW_NOW=2027-01 bash $PUB $S $L2 $me >$tmp/pub 2>&1 && [ \"\$(stat -c %a $L2/2027-01)\" = 700 ] && [ -z \"\$(find $L2/2027-01 -type f ! -perm 600)\" ]"
ln -s /etc/passwd "$S/2027-01/notary_extra.pem"
rm -rf "$L2/2027-01"
check "a month containing a symlink is refused" \
    "! NOTARY_WINDOW_NOW=2027-01 bash $PUB $S $L2 $me >$tmp/pub 2>&1 && grep -q 'other than files' $tmp/pub && [ ! -e $L2/2027-01 ]"
check "warns when 12 or fewer months remain" "publish 2027-01 && grep -q 'WARNING: the schedule has 3 months left' $tmp/pub"
check "month missing from the schedule fails loudly" "! publish 2029-06 && grep -q 'no 2029-06' $tmp/pub"
check "no publishing directories left" "[ -z \"\$(find $L -maxdepth 1 -name '.*' ! -name . )\" ]"

# Quarters in the middle of the year (and the 10# base in quarter_start: 08
# and 09 are not octal).
Q="$tmp/midyear"
runq() { bash "$GEN" "${MASTER[@]}" --notary-dir "$Q" "$@" >"$tmp/out" 2>&1; }
check "mid-year schedule succeeds" "runq --start-month 2029-06 --months 4"
check "June belongs to the April quarter" \
    "[ \"\$(info $Q/2029-06/notary_certificate_50.pem)\" = '{\"action\":\"freenet-donation\",\"amount\":50,\"delegate-key-created\":\"2029-04-01 00:00:00\"}' ]"
check "a new quarter starts in July" \
    "! same $Q/2029-06/notary_certificate_50.pem $Q/2029-07/notary_certificate_50.pem"
check "August and September share July's pair" \
    "same $Q/2029-07/notary_certificate_50.pem $Q/2029-08/notary_certificate_50.pem && same $Q/2029-07/notary_certificate_50.pem $Q/2029-09/notary_certificate_50.pem"
check "September's pair is dated 1 July" \
    "[ \"\$(info $Q/2029-09/notary_certificate_50.pem)\" = '{\"action\":\"freenet-donation\",\"amount\":50,\"delegate-key-created\":\"2029-07-01 00:00:00\"}' ]"

check "no scratch or partial directories left" "[ -z \"\$(find $D $P -maxdepth 1 -type d -name '.*')\" ]"
check "signing keys are private" "[ -z \"\$(find $D -name 'notary_signing_key_*' ! -perm 600)\" ]"

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
