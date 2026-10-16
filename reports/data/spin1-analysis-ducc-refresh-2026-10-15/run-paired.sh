#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: $0 RUST_M1B_BINARY" >&2
    exit 2
fi
report_dir="$(cd "$(dirname "$0")" && pwd)"
binary="$1"

run_native() {
    local pair="$1"
    local thread_count="$2"
    local cpuset=0
    if [[ "$thread_count" == "8" ]]; then
        cpuset=0-7
    fi
    local output="$report_dir/pair-q${thread_count}-${pair}-native"
    taskset -c "$cpuset" /usr/bin/time -f 'elapsed_s=%e max_rss_kb=%M' -o "${output}.time" \
        "$binary" 1 511 "$thread_count" analysis GL STANDARD 512 1025 511 requested-only > "${output}.log"
}

run_ducc() {
    local pair="$1"
    local thread_count="$2"
    local cpuset=0
    if [[ "$thread_count" == "8" ]]; then
        cpuset=0-7
    fi
    local output="$report_dir/pair-q${thread_count}-${pair}-ducc"
    taskset -c "$cpuset" /usr/bin/time -f 'elapsed_s=%e max_rss_kb=%M' -o "${output}.time" \
        env RAYON_NUM_THREADS="$thread_count" RUSTSHT_PERF_RAW=1 uv run python \
        examples/python_perf_baseline.py 511 1 "$thread_count" GL 2 plan analysis ducc STANDARD 1025 512 \
        > "${output}.log"
}

for thread_count in 1 8; do
    for pair in 1 2 3 4 5; do
        if (( pair % 2 == 1 )); then
            run_native "$pair" "$thread_count"
            run_ducc "$pair" "$thread_count"
        else
            run_ducc "$pair" "$thread_count"
            run_native "$pair" "$thread_count"
        fi
    done
done
