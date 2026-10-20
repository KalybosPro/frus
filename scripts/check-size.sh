#!/usr/bin/env bash
#
# Compare a built library's size against the budget committed in the repository
# (milestone 508, #9). Prints the measurement and the budget on every run, a passing one
# included -- a check that only speaks when it is angry teaches nobody what normal looks
# like -- and fails when the library is over budget.
#
# Usage:  bash scripts/check-size.sh <library.so> [budget file]
#
# The budget file defaults to ci/size-budget.toml and is read for one key, `budget_bytes`.
set -eu

LIB="${1:?usage: check-size.sh <library.so> [budget file]}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUDGET_FILE="${2:-$ROOT/ci/size-budget.toml}"

[ -f "$LIB" ] || { echo "error: no library at $LIB" >&2; exit 2; }
BUDGET="$(sed -n 's/^[[:space:]]*budget_bytes[[:space:]]*=[[:space:]]*\([0-9][0-9]*\).*/\1/p' "$BUDGET_FILE" | head -n 1)"
[ -n "$BUDGET" ] || { echo "error: no budget_bytes in $BUDGET_FILE" >&2; exit 2; }

SIZE="$(wc -c < "$LIB" | tr -d '[:space:]')"
# What the library adds to a compressed package, for information only: an APK is a zip.
COMPRESSED="$(gzip -9 -c "$LIB" | wc -c | tr -d '[:space:]')"
USED=$(( SIZE * 100 / BUDGET ))

echo "library:    $LIB"
echo "size:       $SIZE bytes ($COMPRESSED compressed)"
# What a web server sends when it compresses with brotli, where the tool is there (#211).
if command -v brotli > /dev/null 2>&1; then
    echo "brotli:     $(brotli -c -q 11 "$LIB" | wc -c | tr -d '[:space:]') bytes"
fi
echo "budget:     $BUDGET bytes ($BUDGET_FILE)"
echo "used:       $USED% of the budget"

if [ "$SIZE" -gt "$BUDGET" ]; then
    echo "FAIL: $(( SIZE - BUDGET )) bytes over budget." >&2
    echo "If the growth is deliberate, raise budget_bytes in $BUDGET_FILE in the same" >&2
    echo "change and say why; if it is not, something came along that should not have." >&2
    exit 1
fi
echo "OK: $(( BUDGET - SIZE )) bytes of headroom."
