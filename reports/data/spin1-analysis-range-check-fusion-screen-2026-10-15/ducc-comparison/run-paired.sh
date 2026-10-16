#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: $0 RUST_M1B_BINARY" >&2
    exit 2
fi

binary="$1"
report_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="$(cd "$report_dir/../../../.." && pwd)"
lock_file="$repo_dir/target/rustsht-benchmark.lock"
lmax=1023
ntheta=1024
nphi=2049
cd "$repo_dir"

exec 9>"$lock_file"
flock -x 9

run_native() {
    local pair="$1"
    local threads="$2"
    local cpuset=${RUSTSHT_CPUSET_Q1:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}
    if [[ "$threads" == "8" ]]; then
        cpuset=${RUSTSHT_CPUSET_Q8:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}
    fi
    local output="$report_dir/spin-1-t${lmax}-q${threads}-${pair}-native"
    taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" /usr/bin/time -f 'elapsed_s=%e max_rss_kb=%M' \
        -o "${output}.time" \
        "$binary" 1 "$lmax" "$threads" analysis GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only > "${output}.log"
}

run_ducc() {
    local pair="$1"
    local threads="$2"
    local cpuset=${RUSTSHT_CPUSET_Q1:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}
    if [[ "$threads" == "8" ]]; then
        cpuset=${RUSTSHT_CPUSET_Q8:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}
    fi
    local output="$report_dir/spin-1-t${lmax}-q${threads}-${pair}-ducc"
    taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" /usr/bin/time -f 'elapsed_s=%e max_rss_kb=%M' \
        -o "${output}.time" \
        env RAYON_NUM_THREADS="$threads" RUSTSHT_PERF_RAW=1 uv run python \
        examples/python_perf_baseline.py "$lmax" 1 "$threads" GL 2 plan analysis ducc STANDARD "$nphi" "$ntheta" \
        > "${output}.log"
}

for threads in 1 8; do
    for pair in 1 2 3 4 5; do
        if (( pair % 2 == 1 )); then
            run_native "$pair" "$threads"
            run_ducc "$pair" "$threads"
        else
            run_ducc "$pair" "$threads"
            run_native "$pair" "$threads"
        fi
    done
done
