#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
report_dir="$repo_dir/reports/data/m6-m2-general-visitor"
binary=$1
lmax=$2
spin=$3
pair=$4
variant=$5

if [[ "$variant" != baseline && "$variant" != candidate ]]; then
    printf 'variant must be baseline or candidate\n' >&2
    exit 2
fi
if [[ "$spin" != 0 && "$spin" != 1 ]]; then
    printf 'supported spin controls: 0 or 1\n' >&2
    exit 2
fi
if [[ "$lmax" != 511 && "$lmax" != 1023 ]]; then
    printf 'supported screen limits: 511 or 1023\n' >&2
    exit 2
fi
if [[ "$lmax" == 1023 && "$spin" != 0 ]]; then
    printf 'T1023 screen is scalar only\n' >&2
    exit 2
fi

ntheta=$((lmax + 1))
nphi=$((2 * (lmax + 1)))
cpuset=${RUSTSHT_CPUSET_Q1:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}
stem="t$lmax-s$spin-q1-pair$pair-$variant"

{
    printf '\n--- run=%s affinity=%s binary=%s ---\n' "$stem" "$cpuset" "$binary"
    date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
    uptime
    cat /proc/pressure/cpu
    taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" sh -c 'taskset -pc $$'
    ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 20
} >> "$report_dir/preflight-processes.log"

busy=$(ps -eo pid,pcpu,comm,args | awk -v self="$$" '
    $1 != self && (($2 + 0) >= 8 ||
      $3 ~ /^(rustc|cargo|cc1|cc1plus|make|cmake|ninja|m1b_bench|ducc_bench|time_SHTns)$/) {
        print
    }
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
    env RAYON_NUM_THREADS=1 RUSTSHT_BENCH_PHI0=0.37 \
        "$binary" "$spin" "$lmax" 1 synthesis GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only \
        > "$report_dir/$stem.stdout" 2>&1
