#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
report_dir="$repo_dir/reports/data/m6-current-baseline/t2047-q8-extension"
lmax=${1:?lmax required}
spin=${2:?spin required}
run_id=${3:?run id required}
threads=8
ntheta=$((lmax + 1))
nphi=$((2 * (lmax + 1)))
stem="t${lmax}-s${spin}-q${threads}-${run_id}"

{
    printf 'run=%s 8 pinned CPU cores\n' "$stem"
    date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
    uptime
    cat /proc/pressure/cpu
    ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 20
    printf 'matching compiler/benchmark processes:\n'
    ps -eo pid,pcpu,stat,comm,args | awk '$4 == "rustc" || $4 == "cargo" || $4 == "cc1" || $4 == "cc1plus" || $4 == "make" || $4 == "cmake" || $4 == "ninja" || $4 ~ /^m1b_bench/ || $4 ~ /^time_SHT/ || $4 ~ /^ducc/ {print}'
} > "$report_dir/$stem.preflight"

taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} /usr/bin/time -v -o "$report_dir/$stem.time" \
    env TMPDIR="$repo_dir/target/tmp" \
        PYTHONPATH="$repo_dir/target/m6-references/ducc-build" \
        RAYON_NUM_THREADS="$threads" \
        RUSTSHT_PERF_RAW=1 \
        RUSTSHT_SHTNS_LIB="$repo_dir/target/m6-references/shtns-build/libshtns_omp.so" \
        "$repo_dir/.venv/bin/python" "$repo_dir/examples/python_perf_baseline.py" \
        "$lmax" "$spin" "$threads" GL 3 into both all STANDARD "$nphi" "$ntheta" \
        > "$report_dir/$stem.stdout" 2>&1
