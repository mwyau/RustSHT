#!/usr/bin/env bash
set -euo pipefail
if [[ $# -ne 2 ]]; then
    echo "usage: $0 BASELINE_BINARY CANDIDATE_BINARY" >&2
    exit 2
fi
report_dir="$(cd "$(dirname "$0")" && pwd)"
baseline="$1"
candidate="$2"
run_one() {
    local binary="$1"
    local label="$2"
    local output="$report_dir/pair-${pair}-${label}"
    taskset -c 0-7 /usr/bin/time -f 'elapsed_s=%e max_rss_kb=%M' -o "${output}.time" "$binary" > "${output}.log"
}
for pair in 1 2 3 4 5; do
    if (( pair % 2 == 1 )); then
        run_one "$candidate" candidate
        run_one "$baseline" baseline
    else
        run_one "$baseline" baseline
        run_one "$candidate" candidate
    fi
done
