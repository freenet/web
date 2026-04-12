#!/bin/bash
#
# DEPRECATED — renamed to generate_notary_keys.sh in 0.1.5.
# Execs the new script unchanged. Will be removed in 0.2.0.
# See freenet/web#24.

echo "warning: generate_delegate_keys.sh is deprecated, use generate_notary_keys.sh (freenet/web#24)" >&2
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec bash "$script_dir/generate_notary_keys.sh" "$@"
