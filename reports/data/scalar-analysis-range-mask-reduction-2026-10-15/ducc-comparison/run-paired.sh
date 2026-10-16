#!/usr/bin/env bash
set -euo pipefail
lmax=1023
spin=0
ntheta=1024
nphi=2049
binary="/home/albert/RustSHT-sht-performance/target/m1b_bench-range-mask-candidate"
report_dir="$(cd "$(dirname "$0")" && pwd)"

task_cpuset() {
    if [[ "$1" == "1" ]]; then printf '8'; else printf '8-15'; fi
}

run_native() {
    local pair="$1"
    local thread_count="$2"
    local cpuset
    cpuset="$(task_cpuset "$thread_count")"
    local output="$report_dir/scalar-t${lmax}-q${thread_count}-${pair}-native"
    taskset -c "$cpuset" /usr/bin/time -f 'elapsed_s=%e max_rss_kb=%M' -o "${output}.time" \
        "$binary" "$spin" "$lmax" "$thread_count" analysis GL STANDARD "$ntheta" "$nphi" "$lmax" requested-only > "${output}.log"
}

run_ducc() {
    local pair="$1"
    local thread_count="$2"
    local cpuset
    cpuset="$(task_cpuset "$thread_count")"
    local output="$report_dir/scalar-t${lmax}-q${thread_count}-${pair}-ducc"
    taskset -c "$cpuset" /usr/bin/time -f 'elapsed_s=%e max_rss_kb=%M' -o "${output}.time" \
        env TMPDIR=/home/albert/RustSHT-sht-performance/target/rustsht-test-tmp RAYON_NUM_THREADS="$thread_count" RUSTSHT_PERF_RAW=1 uv run python \
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
