#!/bin/bash
#
# Publish the next few months of a notary schedule to the API's notary
# directory. Run as root from a daily timer (see rust/api/README.md).
#
#   publish_notary_window.sh <schedule-dir> <notary-dir> <owner> [months-ahead]
#
# <schedule-dir> holds the full schedule from generate_notary_keys.sh and
# should be readable by root only. Only months up to <months-ahead> (default 1)
# past the current UTC month are published, so a compromise of the API process
# exposes the current and next month's notary keys, not the whole schedule.
#
# Each month is copied to a hidden directory in <notary-dir>, given to <owner>,
# then renamed into place, so the API never sees a partial month. A month that
# is already published is never touched: donations quoted from it may still be
# in checkout, and replacing its keys would break them after the charge.
#
# NOTARY_WINDOW_NOW=YYYY-MM overrides the current month, for tests.

set -euo pipefail

if [ $# -lt 3 ] || [ $# -gt 4 ]; then
    echo "Usage: $0 <schedule-dir> <notary-dir> <owner> [months-ahead]" >&2
    exit 1
fi
schedule="$1"
notary="$2"
owner="$3"
ahead="${4:-1}"
if ! [[ "$ahead" =~ ^[0-9]+$ ]]; then
    echo "Error: months-ahead must be a whole number" >&2
    exit 1
fi

now="${NOTARY_WINDOW_NOW:-$(date -u +%Y-%m)}"
last=$(date -u -d "$now-01 +$ahead month" +%Y-%m)

published=0
for src in "$schedule"/[0-9][0-9][0-9][0-9]-[0-9][0-9]; do
    [ -d "$src" ] || continue
    month=$(basename "$src")
    # YYYY-MM compares correctly as a string.
    if [[ "$month" > "$last" ]] || [ -e "$notary/$month" ]; then
        continue
    fi
    partial="$notary/.$month.publishing"
    rm -rf "$partial"
    cp -a "$src" "$partial"
    chown -R "$owner" "$partial"
    chmod 700 "$partial"
    chmod 600 "$partial"/*
    mv "$partial" "$notary/$month"
    echo "published $month"
    published=$((published + 1))
done

remaining=$(find "$schedule" -mindepth 1 -maxdepth 1 -type d -name '[0-9][0-9][0-9][0-9]-[0-9][0-9]' -printf '%f\n' |
    awk -v now="$now" '$0 >= now' | wc -l)
first=$(find "$schedule" -mindepth 1 -maxdepth 1 -type d -name '[0-9][0-9][0-9][0-9]-[0-9][0-9]' -printf '%f\n' |
    sort | head -n1)
if [ -n "$first" ] && [[ "$first" > "$now" ]]; then
    echo "schedule starts at $first; the API issues from the flat files until then"
elif [ ! -d "$schedule/$now" ]; then
    echo "ERROR: the schedule has no $now; the API is issuing from an older month. Generate more months." >&2
    exit 1
elif [ "$remaining" -le 12 ]; then
    echo "WARNING: the schedule has $remaining months left from $now. Generate more months." >&2
fi
echo "$published published; schedule covers $remaining months from $now"
