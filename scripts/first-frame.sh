#!/usr/bin/env bash
#
# How long an application takes to show its first frame, over several launches
# (milestone 696, #211).
#
# Launches a built frus program N times with FRUS_EXIT_AFTER_FIRST_FRAME=1, so that each
# launch ends once its first frame is presented, reads the line the shell logs then
# ("first frame at ... ms after launch — ..."), and prints every launch, then the fewest,
# the median and the most milliseconds of each stage. One launch is luck; the median of
# several is a number.
#
# Usage:  bash scripts/first-frame.sh [-n runs] [-b budget_ms] [-t timeout_s] program [args...]
#
#   -n runs       launches (default 7)
#   -b budget_ms  fail (exit 1) when the median time to the first frame is over it
#   -t timeout_s  how long one launch may take before it is counted a failure (default 120)
#
# The first launch after a build is usually the slowest: the system's caches are cold, and
# on Vulkan the pipelines are compiled once and kept for the next launches. It is shown, and
# counted like the others; the median is what to compare.
#
# Example (desktop, release build):
#
#   cargo build --release -p frus-hello
#   bash scripts/first-frame.sh -n 10 target/release/frus-hello
set -eu

RUNS=7
BUDGET=""
TIMEOUT=120
while getopts "n:b:t:" opt; do
    case "$opt" in
        n) RUNS="$OPTARG" ;;
        b) BUDGET="$OPTARG" ;;
        t) TIMEOUT="$OPTARG" ;;
        *) echo "usage: first-frame.sh [-n runs] [-b budget_ms] [-t timeout_s] program [args...]" >&2; exit 2 ;;
    esac
done
shift $((OPTIND - 1))
[ "$#" -ge 1 ] || { echo "usage: first-frame.sh [-n runs] [-b budget_ms] [-t timeout_s] program [args...]" >&2; exit 2; }
[ -x "$1" ] || [ -x "$1.exe" ] || { echo "error: no program at $1" >&2; exit 2; }

# The stages, as the line names them: a label for the table, and the words before the
# number in the line.
LABELS="total shell window gpu adapter device surface pipelines warm-up fonts build layout layout-in present"
pattern() {
    case "$1" in
        total) echo "first frame at" ;;
        shell) echo "shell at" ;;
        window) echo "window at" ;;
        gpu) echo "GPU ready at" ;;
        adapter) echo "adapter" ;;
        device) echo "device" ;;
        surface) echo "surface" ;;
        pipelines) echo "pipelines" ;;
        warm-up) echo "warm-up" ;;
        fonts) echo "fonts" ;;
        build) echo "built in" ;;
        layout) echo "laid out at" ;;
        layout-in) echo "laid out at [0-9]+ ms \\(in" ;;
        present) echo "presented in" ;;
    esac
}
# The milliseconds after a stage's words in a line, or "-" when the line does not have it.
value() {
    local v
    v="$(printf '%s\n' "$2" | sed -nE "s/.*(^|[ (;])$(pattern "$1") ([0-9]+) ms.*/\\2/p" | head -n 1)"
    echo "${v:--}"
}

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
TIMEOUT_CMD=""
command -v timeout > /dev/null 2>&1 && TIMEOUT_CMD="timeout $TIMEOUT"

printf '%-4s' "run"
for label in $LABELS; do printf ' %9s' "$label"; done
printf '\n'
for run in $(seq 1 "$RUNS"); do
    out="$WORK/run-$run.log"
    # Only the startup line, whatever RUST_LOG the caller has: warnings and errors still show.
    if ! FRUS_EXIT_AFTER_FIRST_FRAME=1 RUST_LOG="warn,frus_shell::startup=info" \
        $TIMEOUT_CMD "$@" > "$out" 2>&1; then
        echo "launch $run failed or timed out after ${TIMEOUT} s; its output:" >&2
        tail -n 30 "$out" >&2
        exit 2
    fi
    line="$(grep -m 1 "first frame at" "$out" || true)"
    if [ -z "$line" ]; then
        echo "launch $run logged no first frame; its output:" >&2
        tail -n 30 "$out" >&2
        exit 2
    fi
    printf '%-4s' "$run"
    for label in $LABELS; do
        v="$(value "$label" "$line")"
        printf ' %9s' "$v"
        [ "$v" = "-" ] || echo "$v" >> "$WORK/$label"
    done
    printf '\n'
done

# The fewest, the median (the middle one, or the mean of the two middle ones) and the most.
stats() {
    sort -n "$1" | awk '{ v[NR] = $1 } END {
        m = (NR % 2) ? v[(NR + 1) / 2] : (v[NR / 2] + v[NR / 2 + 1]) / 2
        printf "%d %d %d", v[1], m, v[NR]
    }'
}
echo
printf '%-10s %8s %8s %8s\n' "stage (ms)" "min" "median" "max"
for label in $LABELS; do
    [ -s "$WORK/$label" ] || continue
    read -r lo mid hi <<EOF
$(stats "$WORK/$label")
EOF
    printf '%-10s %8s %8s %8s\n' "$label" "$lo" "$mid" "$hi"
    [ "$label" = total ] && MEDIAN="$mid"
done

if [ -n "$BUDGET" ]; then
    echo
    echo "budget:     $BUDGET ms for the median time to the first frame"
    if [ "$MEDIAN" -gt "$BUDGET" ]; then
        echo "OVER: the median, $MEDIAN ms, is $((MEDIAN - BUDGET)) ms over budget." >&2
        exit 1
    fi
    echo "OK: the median, $MEDIAN ms, is $((BUDGET - MEDIAN)) ms under budget."
fi
