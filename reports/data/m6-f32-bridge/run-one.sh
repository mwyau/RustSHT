#!/usr/bin/env bash
set -euo pipefail

repo_dir=${1:?worktree path required}
binary=${2:?benchmark binary required}
output_root=${3:?output root required}
lmax=${4:?lmax required}
spin=${5:?spin required}
threads=${6:?thread count required}
operation=${7:?operation required}

if [[ "$threads" == 1 ]]; then
    cpuset=${RUSTSHT_CPUSET_Q1:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}
elif [[ "$threads" == 8 ]]; then
    cpuset=${RUSTSHT_CPUSET_Q8:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}
else
    printf 'supported thread counts: 1 or 8\n' >&2
    exit 2
fi

stem="t${lmax}-s${spin}-q${threads}-${operation}"
raw_dir="$repo_dir/reports/data/m6-f32-bridge/raw"
dump_dir="$output_root/dumps"
mkdir -p "$raw_dir" "$dump_dir"

exec 9>"$output_root/rustsht-benchmark.lock"
flock -n 9 || {
    printf 'benchmark lock is held\n' >&2
    exit 75
}

psi=$(awk '$1 == "some" { for (i = 2; i <= NF; i++) if ($i ~ /^avg10=/) { sub("avg10=", "", $i); print $i } }' /proc/pressure/cpu)
awk -v psi="$psi" 'BEGIN { exit !(psi < 8.0) }' || {
    printf 'CONTENDED: refusing %s at CPU PSI some avg10=%s%%\n' "$stem" "$psi" >> "$raw_dir/preflight.log"
    exit 75
}

busy=$(ps -eo pid,pcpu,stat,comm,args | awk '
    $2 + 0 >= 8 || $4 ~ /^(rustc|cargo|cc1|cc1plus|make|cmake|ninja|m1b_bench|m6_f32_bridge_bench|ducc_bench|time_SHTns)$/ { print }
')
if [[ -n "$busy" ]]; then
    printf 'CONTENDED: refusing %s due to active workload(s):\n%s\n' "$stem" "$busy" >> "$raw_dir/preflight.log"
    exit 75
fi

{
    printf '\n--- run=%s affinity=%s ---\n' "$stem" "$cpuset"
    date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
    printf 'CPU_PSI_before=%s\n' "$psi"
    sha256sum "$binary"
    ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 20
} >> "$raw_dir/preflight.log"

taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" /usr/bin/time -v -o "$raw_dir/$stem.time" \
    "$binary" "$lmax" "$spin" "$threads" "$operation" "$dump_dir" \
    > "$raw_dir/$stem.stdout" 2>&1

printf 'CPU_PSI_after=%s\n' "$(awk '$1 == "some" { for (i = 2; i <= NF; i++) if ($i ~ /^avg10=/) { sub("avg10=", "", $i); print $i } }' /proc/pressure/cpu)" \
    >> "$raw_dir/preflight.log"
