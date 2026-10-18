#!/usr/bin/env bash
set -euo pipefail
repo=$(git rev-parse --show-toplevel)
candidate=${RUSTSHT_CANDIDATE_DIR:?set RUSTSHT_CANDIDATE_DIR}
baseline=${RUSTSHT_BASELINE_DIR:?set RUSTSHT_BASELINE_DIR}
out="$repo/reports/data/m6-legendre-redesign/prototype-screen"
version=${1:?baseline or candidate required}
lmax=${2:?lmax required}
run_id=${3:?run id required}
case "$version" in
  baseline) package="$baseline/python" ;;
  candidate) package="$candidate/python" ;;
  *) echo "unknown version $version" >&2; exit 2 ;;
esac
threads=1
ntheta=$((lmax + 1))
nphi=$((2 * (lmax + 1)))
cpuset=${RUSTSHT_CPUSET_Q1:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}
frequency_cpu=${RUSTSHT_CPU_INDEX:-${cpuset%%[-,]*}}
frequency_khz() {
    [[ "$frequency_cpu" =~ ^[0-9]+$ ]] || { printf 'unavailable'; return; }
    cat "/sys/devices/system/cpu/cpu${frequency_cpu}/cpufreq/scaling_cur_freq" 2>/dev/null || printf 'unavailable'
}
stem="t${lmax}-s0-q1-${run_id}-${version}"
pre="$out/$stem.preflight"
post="$out/$stem.postflight"
{
  printf 'run=%s version=%s requested_affinity=%s actual_affinity=' "$stem" "$version" "$cpuset"
  taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" sh -c 'taskset -pc $$'
  date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
  uptime
  cat /proc/pressure/cpu
  printf 'cpu.max='; cat /sys/fs/cgroup/cpu.max 2>/dev/null || true
  printf 'cpu.stat='; cat /sys/fs/cgroup/cpu.stat 2>/dev/null || true
  printf 'selected_cpu_frequency_khz='; frequency_khz
  printf 'top_processes:\n'
  ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 25
  printf 'matching_compiler_or_benchmark_processes:\n'
  ps -eo pid,pcpu,stat,comm,args | awk '$4 ~ /^(cargo|rustc|cc1|cc1plus|maturin|ninja|make|cmake)$/ || ($4 ~ /^python/ && $0 ~ /python_perf_baseline[.]py/) || $4 ~ /^m1b_bench/ || $4 ~ /^time_SHT/ {print}'
} > "$pre"
if ! awk '/^some / {for (i=1;i<=NF;i++) if ($i ~ /^avg10=/) {split($i,a,"="); if (a[2]+0 > 0.50) exit 75}}' /proc/pressure/cpu >/dev/null; then
  printf 'refused: preflight cpu PSI some avg10 exceeds 0.50\n' > "$out/$stem.refusal"
  exit 75
fi
if ps -eo pid,pcpu,stat,comm,args | awk '$4 ~ /^(cargo|rustc|cc1|cc1plus|maturin|ninja|make|cmake)$/ || ($4 ~ /^python/ && $0 ~ /python_perf_baseline[.]py/) || $4 ~ /^m1b_bench/ || $4 ~ /^time_SHT/ {found=1} END {exit found ? 0 : 1}'; then
  printf 'refused: competing compiler or benchmark visible in preflight\n' > "$out/$stem.refusal"
  exit 75
fi
printf 'command: taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" /usr/bin/time -v -o %s/%s.time env TMPDIR=%s/target/tmp PYTHONPATH=%s/python:%s/target/m6-references/ducc-build RAYON_NUM_THREADS=1 RUSTSHT_PERF_RAW=1 RUSTSHT_BENCH_PHI0=0.37 RUSTSHT_SHTNS_LIB=%s/target/m6-references/shtns-build/libshtns_omp.so %s/.venv/bin/python %s/examples/python_perf_baseline.py %s 0 1 GL 1 into synthesis all STANDARD %s %s\n' \
  "$cpuset" "$out" "$stem" "$repo" "$package" "$repo" "$repo" "$repo" "$repo" "$lmax" "$nphi" "$ntheta" > "$out/$stem.command"
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" /usr/bin/time -v -o "$out/$stem.time" \
  env TMPDIR="$repo/target/tmp" \
    PYTHONPATH="$package:$repo/target/m6-references/ducc-build" \
    RAYON_NUM_THREADS="$threads" \
    RUSTSHT_PERF_RAW=1 \
    RUSTSHT_BENCH_PHI0=0.37 \
    RUSTSHT_SHTNS_LIB="$repo/target/m6-references/shtns-build/libshtns_omp.so" \
    "$repo/.venv/bin/python" "$repo/examples/python_perf_baseline.py" \
    "$lmax" 0 "$threads" GL 1 into synthesis all STANDARD "$nphi" "$ntheta" \
    > "$out/$stem.stdout" 2>&1
{
  printf 'run=%s version=%s affinity=pinned\n' "$stem" "$version"
  date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
  uptime
  cat /proc/pressure/cpu
  printf 'cpu.stat='; cat /sys/fs/cgroup/cpu.stat 2>/dev/null || true
  printf 'selected_cpu_frequency_khz='; frequency_khz
  ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 20
} > "$post"
if ! awk '/^some / {for (i=1;i<=NF;i++) if ($i ~ /^avg10=/) {split($i,a,"="); if (a[2]+0 > 0.50) exit 75}}' "$post"; then
  printf 'failed: postflight cpu PSI some avg10 exceeds 0.50\n' > "$out/$stem.postflight-failed"
  exit 76
fi
python3 "$out/check-stability.py" "$out/$stem.stdout"
printf 'completed %s\n' "$stem"
