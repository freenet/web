#!/bin/bash
#
# Publish the next few months of a notary schedule to the API's notary
# directory. Run as root from a daily timer (see rust/api/README.md).
#
#   publish_notary_window.sh <schedule-dir> <notary-dir> <owner> [months-ahead]
#
# <schedule-dir> holds the full schedule from generate_notary_keys.sh and must
# be private to the caller, down a path nobody else can rewrite. Only months up
# to <months-ahead> (default 1) past the current UTC month are published, so a
# compromise of the API process exposes the current and next month's notary
# keys, not the whole schedule.
#
# The API's user owns <notary-dir> and can rename or replace anything on the
# path to it, so:
#   - nothing is staged there: each month is copied, given private modes and
#     handed to <owner> inside <schedule-dir>/.staging, which that user cannot
#     reach;
#   - <notary-dir> is pinned once (cd -P) and afterwards addressed only as ".";
#   - the month is renamed in with mv --no-copy, which fails rather than fall
#     back to a copy (and its path-following syscalls) if the rename cannot be
#     done in one step, e.g. across filesystems or bind mounts.
# A month that is already published is never touched: donations quoted from it
# may still be in checkout, and replacing its keys would break them after the
# charge.
#
# NOTARY_WINDOW_NOW=YYYY-MM overrides the current month, for tests.

set -euo pipefail
unset CDPATH

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
owner_uid=$(id -u "${owner%%:*}")

for d in "$schedule" "$notary"; do
    if [ ! -d "$d" ]; then
        echo "Error: $d is not a directory" >&2
        exit 1
    fi
done

# The schedule holds every future key, and the lock and staging live in it, so
# it must be private to the caller (root, in production), and so must every
# directory above it: otherwise its owner could swap the path out from under us.
schedule=$(cd -P -- "$schedule" && pwd)
if [ "$(stat -c '%u %a' "$schedule")" != "$(id -u) 700" ]; then
    echo "Error: $schedule must be owned by $(id -un) with mode 700" >&2
    exit 1
fi
d="$schedule"
while [ "$d" != / ]; do
    d=$(dirname "$d")
    read -r uid mode <<<"$(stat -c '%u %a' "$d")"
    # Writable by group or others is only acceptable with the sticky bit
    # (like /tmp), where nobody else can rename our entries.
    if { [ "$uid" != 0 ] && [ "$uid" != "$(id -u)" ]; } ||
        { (((8#$mode & 8#022) != 0)) && (((8#$mode & 8#1000) == 0)); }; then
        echo "Error: $d (above $schedule) is owned or writable by someone else" >&2
        exit 1
    fi
done

cd -P -- "$notary"
if [ "$(stat -c %d .)" != "$(stat -c %d "$schedule")" ]; then
    echo "Error: $notary and $schedule must be on the same filesystem, so a month can be renamed into place" >&2
    exit 1
fi
if [ "$(stat -c %u .)" != "$owner_uid" ]; then
    echo "Error: $notary is not owned by ${owner%%:*}" >&2
    exit 1
fi

# One run at a time: an overlapping manual run and timer run would share staging.
exec 9>"$schedule/.lock"
flock -n 9 || { echo "Error: another publish is running" >&2; exit 1; }

staging="$schedule/.staging"
[ -d "$staging" ] || mkdir -m 700 "$staging"

now="${NOTARY_WINDOW_NOW:-$(date -u +%Y-%m)}"
last=$(date -u -d "$now-01 +$ahead month" +%Y-%m)

published=0
for src in "$schedule"/[0-9][0-9][0-9][0-9]-[0-9][0-9]; do
    [ -d "$src" ] || continue
    month=$(basename "$src")
    # YYYY-MM compares correctly as a string.
    if [[ "$month" > "$last" ]] || [ -e "./$month" ] || [ -L "./$month" ]; then
        continue
    fi
    # The generator writes only directories and regular files; anything else
    # (a symlink, a device node) would be handed to <owner> as is.
    if [ -n "$(find "$src" ! -type f ! -type d -print -quit)" ]; then
        echo "Error: $src contains something other than files and directories" >&2
        exit 1
    fi
    rm -rf "${staging:?}/$month"
    # Contents only, not modes, ACLs or xattrs from wherever the schedule has
    # been; staging is unreachable to <owner>, so setting modes here is safe.
    cp -R --preserve=timestamps "$src" "$staging/$month"
    find "$staging/$month" -type d -exec chmod 700 {} +
    find "$staging/$month" -type f -exec chmod 600 {} +
    chown -R "$owner" "$staging/$month"
    mv --no-copy -T "$staging/$month" "./$month"
    echo "published $month"
    published=$((published + 1))
done

rmdir "$staging" 2>/dev/null || true

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
