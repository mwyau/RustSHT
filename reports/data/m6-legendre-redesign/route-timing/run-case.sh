#!/usr/bin/env bash
set -euo pipefail

label=$1
lmax=$2
report_dir=$(git rev-parse --show-toplevel)/reports/data/m6-legendre-redesign/route-timing
binary=${RUSTSHT_CANDIDATE_DIR:?set RUSTSHT_CANDIDATE_DIR}/target/release/deps/rustsht-159837ac1c3bd1f7
cpuset=${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}
frequency_cpu=${RUSTSHT_CPU_INDEX:-${cpuset%%[-,]*}}
frequency_khz() {
    [[ "$frequency_cpu" =~ ^[0-9]+$ ]] || { printf 'unavailable'; return; }
    cat "/sys/devices/system/cpu/cpu${frequency_cpu}/cpufreq/scaling_cur_freq" 2>/dev/null || printf 'unavailable'
}
test_name=legendre::synthesis_fast::tests::m6_fast_route_counts
source_hash=d94bb0b9b861c1ad0debade62180b2ad35ddbdd3a0c848deb3cd47b390389a85
binary_hash=cb6eb68e1ef86ab41a4591d81b22d6b67105e7f04b09deb39effd2bd559514f8
base="$report_dir/$label"

capture_snapshot() {
    local path=$1
    {
        printf 'run=%s requested_affinity=pinned\n' "$label"
        date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
        uptime
        cat /proc/pressure/cpu
        printf 'cpu.max='; cat /sys/fs/cgroup/cpu.max 2>/dev/null || echo unavailable
        printf 'cpu.stat:\n'; cat /sys/fs/cgroup/cpu.stat 2>/dev/null || echo unavailable
        printf 'selected_cpu_frequency_khz='; frequency_khz
        printf 'taskset_runner_affinity='; taskset -pc $$
        printf 'top_processes:\n'
        ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -25
        printf 'matching_build_or_benchmark_processes:\n'
        ps -eo pid,pcpu,stat,comm,args | awk '$4 ~ /^(cargo|rustc|cc1|cc1plus|maturin|ninja|make|cmake)$/ || $4 ~ /^rustsht-/ || ($4 ~ /^python/ && $0 ~ /python_perf_baseline[.]py/) || $4 ~ /^m1b_bench/ || $4 ~ /^time_SHT/ {print}'
    } > "$path"
}

capture_snapshot "$base.preflight"
pre_psi=$(awk '/^some / {for (i=1;i<=NF;i++) if ($i ~ /^avg10=/) {split($i,a,"="); print a[2]}}' "$base.preflight")
if [[ -z "$pre_psi" ]] || ! awk -v value="$pre_psi" 'BEGIN {exit !(value <= 0.50)}'; then
    printf 'refused: preflight cpu PSI some avg10=%s exceeds or lacks 0.50 limit\n' "${pre_psi:-unavailable}" > "$base.refusal"
    exit 75
fi
if ps -eo pid,pcpu,stat,comm,args | awk '$4 ~ /^(cargo|rustc|cc1|cc1plus|maturin|ninja|make|cmake)$/ || $4 ~ /^rustsht-/ || ($4 ~ /^python/ && $0 ~ /python_perf_baseline[.]py/) || $4 ~ /^m1b_bench/ || $4 ~ /^time_SHT/ {found=1} END {exit found ? 0 : 1}'; then
    printf 'refused: competing compiler or benchmark visible in preflight\n' > "$base.refusal"
    exit 75
fi

{
    printf 'label=%s lmax=%s threads=1 requested_affinity=pinned\n' "$label" "$lmax"
    printf 'binary=%s\n' "$binary"
    printf 'binary_sha256='; sha256sum "$binary"
    printf 'source_sha256=%s\n' "$source_hash"
    printf 'command: taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" env RUSTSHT_FAST_ROUTE_LMAX=%s RUSTSHT_FAST_ROUTE_THREADS=1 %s --ignored --exact %s --nocapture\n' "$lmax" "$binary" "$test_name"
} > "$base.command"

taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" env RUSTSHT_FAST_ROUTE_LMAX="$lmax" RUSTSHT_FAST_ROUTE_THREADS=1 \
    "$binary" --ignored --exact "$test_name" --nocapture > "$base.stdout" 2>&1 &
pid=$!
printf 'pid=%s\n' "$pid" >> "$base.command"
taskset -apc "$pid" > "$base.running-worker-sized CPU affinity>&1 || true
ps -L -p "$pid" -o pid,tid,psr,pcpu,stat,comm,args > "$base.running-threads" 2>&1 || true
if wait "$pid"; then
    status=0
else
    status=$?
fi
printf 'exit_status=%s\n' "$status" >> "$base.command"

capture_snapshot "$base.postflight"
if [[ $status -ne 0 ]]; then
    printf 'failed: test process exit status %s\n' "$status" > "$base.failure"
    exit "$status"
fi
post_psi=$(awk '/^some / {for (i=1;i<=NF;i++) if ($i ~ /^avg10=/) {split($i,a,"="); print a[2]}}' "$base.postflight")
if [[ -z "$post_psi" ]] || ! awk -v value="$post_psi" 'BEGIN {exit !(value <= 0.50)}'; then
    printf 'failed: postflight cpu PSI some avg10=%s exceeds or lacks 0.50 limit\n' "${post_psi:-unavailable}" > "$base.postflight-failed"
    exit 76
fi
python3 "$report_dir/check-stability.py" "$base.stdout"
printf 'completed %s\n' "$label"
