#!/usr/bin/env bash
set -euo pipefail
repo_dir=$(git rev-parse --show-toplevel)
report_dir="$repo_dir/reports/data/m6-p01/disabled-path-isolation"
stem=t1023-s0-q1-disabled-synthesis
{
    printf '\n--- run=%s affinity=pinned binary=%s ---\n' "$stem" "$repo_dir/target/m6-p01-disabled-bench"
    date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
    uptime
    cat /proc/pressure/cpu
    ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 20
    printf 'matching compiler/benchmark processes:\n'
    ps -eo pid,pcpu,stat,comm,args | awk '$4 == "rustc" || $4 == "cargo" || $4 == "cc1" || $4 == "cc1plus" || $4 == "make" || $4 == "cmake" || $4 == "ninja" || $4 ~ /^m1b_bench/ || $4 ~ /^time_SHT/ || $4 ~ /^ducc/ {print}'
} >> "$report_dir/scalar-check-preflight.log"
busy=$(ps -eo pid,pcpu,stat,comm,args | awk -v self="$$" '$1 != self && (($2 + 0) >= 8 || $4 ~ /^(rustc|cargo|cc1|cc1plus|make|cmake|ninja|m1b_bench|time_SHTns|ducc_bench)$/) {print}')
if [[ -n "$busy" ]]; then
    printf 'CONTENDED: refusing scalar check due to live workload(s):\n%s\n' "$busy" >> "$report_dir/scalar-check-preflight.log"
    exit 75
fi
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} /usr/bin/time -v -o "$report_dir/$stem.time" \
    env RAYON_NUM_THREADS=1 RUSTSHT_BENCH_PHI0=0.37 \
        "$repo_dir/target/m6-p01-disabled-bench" 0 1023 1 synthesis GL STANDARD 1024 2048 1023 requested-only \
        > "$report_dir/$stem.stdout" 2>&1
