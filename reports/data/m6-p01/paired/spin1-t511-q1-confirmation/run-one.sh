#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
report_dir="$repo_dir/reports/data/m6-p01/paired/spin1-t511-q1-confirmation"
binary=${1:?benchmark binary required}
lmax=${2:?lmax required}
spin=${3:?spin required}
threads=${4:?thread count required}
pair=${5:?pair number required}
variant=${6:?variant required}

ntheta=$((lmax + 1))
nphi=$((2 * (lmax + 1)))
cpuset=${RUSTSHT_CPUSET_Q1:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}
if [[ "$threads" == 8 ]]; then cpuset=${RUSTSHT_CPUSET_Q8:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}; fi
if [[ "$threads" != 1 && "$threads" != 8 ]]; then
    printf 'supported thread counts: 1 or 8\n' >&2
    exit 2
fi
if [[ "$variant" != baseline && "$variant" != candidate ]]; then
    printf 'variant must be baseline or candidate\n' >&2
    exit 2
fi

stem="t${lmax}-s${spin}-q${threads}-pair${pair}-${variant}"
{
    printf '\n--- run=%s affinity=%s binary=%s ---\n' "$stem" "$cpuset" "$binary"
    date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
    uptime
    cat /proc/pressure/cpu
    ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 20
    printf 'matching compiler/benchmark processes:\n'
    ps -eo pid,pcpu,stat,comm,args | awk '$4 == "rustc" || $4 == "cargo" || $4 == "cc1" || $4 == "cc1plus" || $4 == "make" || $4 == "cmake" || $4 == "ninja" || $4 ~ /^m1b_bench/ || $4 ~ /^time_SHT/ || $4 ~ /^ducc/ {print}'
} >> "$report_dir/preflight-processes.log"

# Refuse to add a measurement while a likely build/benchmark process is live or
# any unrelated process is consuming a material share of a CPU. The full
# snapshot above remains available to explain a skipped run.
busy=$(ps -eo pid,pcpu,stat,comm,args | awk -v self="$$" '
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

taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" /usr/bin/time -v -o "$report_dir/$stem.time" \
    env RAYON_NUM_THREADS="$threads" RUSTSHT_BENCH_PHI0=0.37 \
        "$binary" "$spin" "$lmax" "$threads" both GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only \
        > "$report_dir/$stem.stdout" 2>&1
