#!/usr/bin/env bash
set -euo pipefail
repo_dir=$(git rev-parse --show-toplevel)
evidence_dir="$repo_dir/reports/data/m6-spin1-routes"
test_bin="$repo_dir/target/m6-spin1-routes/cargo-target/release/deps/rustsht-159837ac1c3bd1f7"
lmax=${1:?lmax required}
threads=${2:?thread count required}
stem="t${lmax}-q${threads}"
cpu_set=${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}

if [[ "$threads" != 1 && "$threads" != 8 ]]; then
    printf 'supported thread counts: 1 or 8\n' >&2
    exit 2
fi
preflight="$evidence_dir/$stem.preflight"
stdout="$evidence_dir/$stem.stdout"
time_file="$evidence_dir/$stem.time"
{
    printf 'cell=%s affinity=%s requested\n' "$stem" "$cpu_set"
    date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
    uptime
    cat /proc/pressure/cpu
    sha256sum "$test_bin"
    printf 'processes by CPU use:\n'
    ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 20
    printf 'matching compiler and benchmark processes:\n'
    ps -eo pid,pcpu,stat,comm,args | awk '$4 == "rustc" || $4 == "cargo" || $4 == "cc1" || $4 == "cc1plus" || $4 == "make" || $4 == "cmake" || $4 == "ninja" || $4 ~ /^m1b_bench/ || $4 ~ /^time_SHT/ || $4 ~ /^ducc/ {print}'
} > "$preflight"
psi_avg10=$(awk '/^some / {for (i=1; i<=NF; ++i) if ($i ~ /^avg10=/) {sub(/^avg10=/, "", $i); print $i}}' "$preflight")
if ! awk -v value="$psi_avg10" 'BEGIN {exit !(value <= 0.50)}'; then
    printf 'REFUSED: cpu some avg10=%s exceeds fixed 0.50 threshold\n' "$psi_avg10" >> "$preflight"
    exit 75
fi
if awk 'NR > 1 && ($4 == "rustc" || $4 == "cargo" || $4 == "cc1" || $4 == "cc1plus" || $4 == "make" || $4 == "cmake" || $4 == "ninja" || $4 ~ /^m1b_bench/ || $4 ~ /^time_SHT/ || $4 ~ /^ducc/) {found=1} END {exit !found}' < <(ps -eo pid,pcpu,stat,comm,args); then
    printf 'REFUSED: competing compiler or benchmark process present\n' >> "$preflight"
    exit 75
fi
printf 'command=taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" env RUSTSHT_ROUTE_LMAX=%s RUSTSHT_ROUTE_THREADS=%s %s --exact legendre::route_profile::m6_spin1_synthesis_routes --ignored --nocapture\n' \
    "$cpu_set" "$lmax" "$threads" "$test_bin" >> "$preflight"
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" /usr/bin/time -v -o "$time_file" \
    env RUSTSHT_ROUTE_LMAX="$lmax" RUSTSHT_ROUTE_THREADS="$threads" \
    "$test_bin" --exact legendre::route_profile::m6_spin1_synthesis_routes --ignored --nocapture \
    > "$stdout" 2>&1
