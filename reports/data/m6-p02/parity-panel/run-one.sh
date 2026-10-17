#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
report_dir="$repo_dir/reports/data/m6-p02/parity-panel"
binary=${1:?benchmark binary required}
lmax=${2:?lmax required}
spin=${3:?spin required}
pair=${4:?pair required}
variant=${5:?variant required}
ntheta=$((lmax + 1))
nphi=$((2 * (lmax + 1)))
stem="t${lmax}-s${spin}-q1-pair${pair}-${variant}"

{
    printf '\n--- run=%s binary=%s ---\n' "$stem" "$binary"
    date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
    cat /proc/pressure/cpu
    ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 20
} >> "$report_dir/preflight-processes.log"

psi=$(awk '$1 == "some" { for (i = 2; i <= NF; i++) if ($i ~ /^avg10=/) { sub("avg10=", "", $i); print $i } }' /proc/pressure/cpu)
awk -v psi="$psi" 'BEGIN { exit !(psi < 8.0) }' || {
    printf 'CONTENDED: refusing %s at CPU PSI some avg10=%s%%\n' "$stem" "$psi" >> "$report_dir/preflight-processes.log"
    exit 75
}

busy=$(ps -eo pid,pcpu,stat,comm,args | awk '
    $2 + 0 >= 8 || $4 ~ /^(rustc|cargo|cc1|cc1plus|make|cmake|ninja|m1b_bench|ducc_bench|time_SHTns)$/ { print }
')
if [[ -n "$busy" ]]; then
    printf 'CONTENDED: refusing %s due to active workload(s):\n%s\n' "$stem" "$busy" >> "$report_dir/preflight-processes.log"
    exit 75
fi

taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} /usr/bin/time -v -o "$report_dir/$stem.time" \
    env RAYON_NUM_THREADS=1 RUSTSHT_BENCH_PHI0=0.37 \
    "$binary" "$spin" "$lmax" 1 synthesis GL STANDARD "$ntheta" "$nphi" "$lmax" requested-only \
    > "$report_dir/$stem.stdout" 2>&1
