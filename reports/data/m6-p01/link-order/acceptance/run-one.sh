#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
report_dir="$repo_dir/reports/data/m6-p01/link-order/acceptance"
binary=${1:?benchmark binary required}
lmax=${2:?lmax required}
spin=${3:?spin required}
threads=${4:?thread count required}
pair=${5:?pair number required}
variant=${6:?variant required}

if [[ "$variant" != baseline && "$variant" != ordered ]]; then
    printf 'variant must be baseline or ordered\n' >&2
    exit 2
fi
if [[ "$threads" == 1 ]]; then cpuset=${RUSTSHT_CPUSET_Q1:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}; elif [[ "$threads" == 8 ]]; then cpuset=${RUSTSHT_CPUSET_Q8:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}; else
    printf 'supported thread counts: 1 or 8\n' >&2
    exit 2
fi

ntheta=$((lmax + 1))
nphi=$((2 * (lmax + 1)))
stem="t${lmax}-s${spin}-q${threads}-pair${pair}-${variant}"
stdout="$report_dir/$stem.stdout"
timing="$report_dir/$stem.time"
if [[ -e "$stdout" || -e "$timing" ]]; then
    if [[ -f "$stdout" && -f "$timing" ]] && rg -q 'Exit status:.*0' "$timing"; then
        printf 'retained existing process: %s\n' "$stem"
        exit 0
    fi
    printf 'incomplete prior process record exists for %s; refusing to overwrite\n' "$stem" >&2
    exit 2
fi

{
    printf '\n--- run=%s affinity=%s binary=%s ---\n' "$stem" "$cpuset" "$binary"
    date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
    uptime
    cat /proc/pressure/cpu
    ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 20
    printf 'matching compiler/benchmark processes:\n'
    ps -eo pid,pcpu,stat,comm,args | awk '$4 == "rustc" || $4 == "cargo" || $4 == "cc1" || $4 == "cc1plus" || $4 == "make" || $4 == "cmake" || $4 == "ninja" || $4 ~ /^m1b_bench/ || $4 ~ /^time_SHT/ || $4 ~ /^ducc/ {print}'
} >> "$report_dir/preflight-processes.log"

busy=$(ps -eo pid,pcpu,stat,comm,args | awk -v self="$$" '
    $1 != self && (($2 + 0) >= 8 ||
      $4 ~ /^(rustc|cargo|cc1|cc1plus|make|cmake|ninja|m1b_bench|time_SHTns|ducc_bench)$/ ||
      $5 ~ /(^|\/)(rustc|cargo|m1b_bench|ducc_bench|time_SHTns)( |$)/) { print }
')
some_avg10=$(awk '/some/ {for (i = 1; i <= NF; i++) if ($i ~ /^avg10=/) {sub("avg10=", "", $i); print $i}}' /proc/pressure/cpu)
if [[ -n "$busy" ]] || awk -v value="$some_avg10" 'BEGIN {exit !(value >= 8.0)}'; then
    {
        printf 'CONTENDED: refusing %s, cpu_some_avg10=%s\n' "$stem" "$some_avg10"
        printf '%s\n' "$busy"
    } >> "$report_dir/preflight-processes.log"
    printf 'preflight found an active workload or elevated CPU PSI; see preflight-processes.log\n' >&2
    exit 75
fi

taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" /usr/bin/time -v -o "$timing" \
    env RAYON_NUM_THREADS="$threads" RUSTSHT_BENCH_PHI0=0.37 \
        "$binary" "$spin" "$lmax" "$threads" both GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only \
        > "$stdout" 2>&1
