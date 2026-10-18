#!/usr/bin/env bash
set -euo pipefail

label=$1
lmax=$2
threads=$3
cpus=$4
report_dir=$(git rev-parse --show-toplevel)/reports/data/m6-legendre-redesign/route-accounting
test_bin=${RUSTSHT_CANDIDATE_DIR:?set RUSTSHT_CANDIDATE_DIR}/target/release/deps/rustsht-159837ac1c3bd1f7
test_name=legendre::synthesis_fast::tests::m6_fast_route_counts
base="$report_dir/$label"

capture_snapshot() {
    local output=$1
    {
        date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
        cat /proc/pressure/cpu
        cat /sys/fs/cgroup/cpu.max
        cat /sys/fs/cgroup/cpu.stat
        ps -eo pid,ppid,psr,pcpu,stat,comm,args --sort=-pcpu | head -25
    } > "$output"
}

{
    printf 'label=%s lmax=%s threads=%s requested_affinity=%s\n' "$label" "$lmax" "$threads" "$cpus"
    printf 'test_binary=%s\n' "$test_bin"
    printf 'test_binary_sha256='; sha256sum "$test_bin"
    printf 'test_name=%s\n' "$test_name"
    printf 'command: taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" env RUSTSHT_FAST_ROUTE_LMAX=%q RUSTSHT_FAST_ROUTE_THREADS=%q %q --ignored --exact %q --nocapture\n' \
        "$cpus" "$lmax" "$threads" "$test_bin" "$test_name"
} > "$base.command"

capture_snapshot "$base.preflight"
taskset -pc $$ >> "$base.preflight"

if ps -eo pid=,comm=,args= | awk -v self="$$" \
    '$1 != self && ($2 ~ /^(cargo|rustc|rustsht-[0-9a-f]+)$/ || $0 ~ /python_perf_baseline\.py/)' \
    > "$base.concurrent" && [[ -s "$base.concurrent" ]]; then
    printf 'refused: competing build or benchmark process detected\n' >> "$base.concurrent"
    exit 77
else
    printf 'no competing build or benchmark process detected\n' > "$base.concurrent"
fi

taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" env \
    RUSTSHT_FAST_ROUTE_LMAX="$lmax" \
    RUSTSHT_FAST_ROUTE_THREADS="$threads" \
    "$test_bin" --ignored --exact "$test_name" --nocapture > "$base.stdout" 2>&1 &
pid=$!
printf 'pid=%s\n' "$pid" >> "$base.command"
taskset -pc "$pid" > "$base.running-affinity" || true
ps -L -p "$pid" -o pid,tid,psr,pcpu,stat,comm,args > "$base.running-threads" || true
if wait "$pid"; then
    status=0
else
    status=$?
fi
printf 'exit_status=%s\n' "$status" >> "$base.command"

capture_snapshot "$base.postflight"
if [[ $status -ne 0 ]]; then
    exit "$status"
fi
