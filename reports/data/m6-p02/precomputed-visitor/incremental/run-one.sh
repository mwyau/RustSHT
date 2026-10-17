#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
report_dir="$repo_dir/reports/data/m6-p02/precomputed-visitor/incremental"
binary=${1:?benchmark binary required}
lmax=${2:?lmax required}
spin=${3:?spin required}
pair=${4:?pair number required}
variant=${5:?variant required}
if [[ "$variant" != p01 && "$variant" != p02 ]]; then
    printf 'variant must be p01 or p02\n' >&2
    exit 2
fi

ntheta=$((lmax + 1))
nphi=$((2 * (lmax + 1)))
stem="t${lmax}-s${spin}-q1-pair${pair}-${variant}"
self_pid=$$
{
    printf '\n--- run=%s affinity=pinned binary=%s ---\n' "$stem" "$binary"
    date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
    uptime
    cat /proc/pressure/cpu
    ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 20
    printf 'matching compiler/benchmark processes:\n'
    ps -eo pid,pcpu,stat,comm,args | awk '$4 == "rustc" || $4 == "cargo" || $4 == "cc1" || $4 == "cc1plus" || $4 == "make" || $4 == "cmake" || $4 == "ninja" || $4 ~ /^m1b_bench/ || $4 ~ /^time_SHT/ || $4 ~ /^ducc/ {print}'
} >> "$report_dir/preflight-processes.log"

busy=$(ps -eo pid,pcpu,stat,comm,args | awk -v self="$self_pid" '
    $1 != self && (($2 + 0) >= 8 ||
      $4 ~ /^(rustc|cargo|cc1|cc1plus|make|cmake|ninja|m1b_bench|time_SHTns|ducc_bench)$/ ||
      $5 ~ /(^|\/)(rustc|cargo|m1b_bench|ducc_bench|time_SHTns)( |$)/) { print }
')
if [[ -n "$busy" ]]; then
    {
        printf 'CONTENDED: refusing %s due to live workload(s):\n' "$stem"
        printf '%s\n' "$busy"
    } >> "$report_dir/preflight-processes.log"
    printf 'preflight found active CPU/build workload; see %s/preflight-processes.log\n' "$report_dir" >&2
    exit 75
fi

taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} /usr/bin/time -v -o "$report_dir/$stem.time" \
    env RAYON_NUM_THREADS=1 RUSTSHT_BENCH_PHI0=0.37 \
        "$binary" "$spin" "$lmax" 1 synthesis GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only \
        > "$report_dir/$stem.stdout" 2>&1
