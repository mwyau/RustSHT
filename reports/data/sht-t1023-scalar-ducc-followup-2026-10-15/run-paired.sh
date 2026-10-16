#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: $0 SPIN LMAX RUST_M1B_BINARY" >&2
    exit 2
fi
spin="$1"
lmax="$2"
binary="$3"
ntheta=$((lmax + 1))
nphi=$((2 * lmax + 3))
report_dir="$(cd "$(dirname "$0")" && pwd)"

run_native() {
    local pair="$1"
    local thread_count="$2"
    local cpuset=0
    if [[ "$thread_count" == "8" ]]; then
        cpuset=8-15
    fi
    local output="$report_dir/spin-${spin}-t${lmax}-q${thread_count}-${pair}-native"
    taskset -c "$cpuset" /usr/bin/time -f 'elapsed_s=%e max_rss_kb=%M' -o "${output}.time" \
        "$binary" "$spin" "$lmax" "$thread_count" analysis GL STANDARD "$ntheta" "$nphi" "$lmax" requested-only > "${output}.log"
}

run_ducc() {
    local pair="$1"
    local thread_count="$2"
    local cpuset=0
    if [[ "$thread_count" == "8" ]]; then
        cpuset=8-15
    fi
    local output="$report_dir/spin-${spin}-t${lmax}-q${thread_count}-${pair}-ducc"
    taskset -c "$cpuset" /usr/bin/time -f 'elapsed_s=%e max_rss_kb=%M' -o "${output}.time" \
        env RAYON_NUM_THREADS="$thread_count" RUSTSHT_PERF_RAW=1 uv run python \
        examples/python_perf_baseline.py "$lmax" "$spin" "$thread_count" GL 2 plan analysis ducc STANDARD "$nphi" "$ntheta" \
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
