#!/usr/bin/env bash
set -Eeuo pipefail

label=$1
variant=$2
lmax=$3
outdir=$(cd "$(dirname "$0")" && pwd)
root=$(git rev-parse --show-toplevel)
cpuset=${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}
frequency_cpu=${RUSTSHT_CPU_INDEX:-${cpuset%%[-,]*}}

frequency_khz() {
  [[ "$frequency_cpu" =~ ^[0-9]+$ ]] || { printf 'unavailable'; return; }
  cat "/sys/devices/system/cpu/cpu${frequency_cpu}/cpufreq/scaling_cur_freq" 2>/dev/null || printf 'unavailable'
}
case "$variant" in
  baseline) package=${RUSTSHT_BASELINE_DIR:?set RUSTSHT_BASELINE_DIR}/python ;;
  candidate) package=${RUSTSHT_CANDIDATE_DIR:?set RUSTSHT_CANDIDATE_DIR}/python ;;
  *) echo "unknown variant: $variant" >&2; exit 2 ;;
esac
case "$lmax" in 1023|2047) ;; *) echo "v2 cells are T1023 and T2047 only" >&2; exit 2 ;; esac
out="$outdir/$label"

psi_some() {
  awk '/^some / {for (i=1;i<=NF;i++) if ($i ~ /^avg10=/) {split($i,a,"="); print a[2]}}' /proc/pressure/cpu
}

competing() {
  local exclude_csv=""
  local pid
  for pid in "$@"; do
    [[ -n "$pid" ]] || continue
    exclude_csv+="${exclude_csv:+,}$pid"
  done
  ps -eo pid,comm,args | awk -v exclude_csv="$exclude_csv" '
    BEGIN { if (exclude_csv != "") {count=split(exclude_csv, ids, ","); for (i=1;i<=count;i++) excluded[ids[i]]=1} }
    !excluded[$1] && $2 ~ /^(cargo|rustc|cc|cc1|cc1plus|maturin|ninja|make|cmake|m1b_bench|time_SHT)$/ {print; found=1}
    !excluded[$1] && $2 ~ /^python/ && $0 ~ /(python_perf_baseline\.py|m1b_bench|time_SHT)/ {print; found=1}
    END {exit !found}'
}

snapshot() {
  local phase=$1
  {
    echo "timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "label=$label phase=$phase variant=$variant T=$lmax requested_affinity=$cpuset"
    echo "actual_affinity=$(taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" sh -c 'taskset -pc $$' 2>/dev/null || echo unavailable)"
    echo "uptime=$(cat /proc/uptime)"
    echo "cpu_pressure:"; cat /proc/pressure/cpu
    echo "cgroup_cpu_max=$(cat /sys/fs/cgroup/cpu.max 2>/dev/null || echo unavailable)"
    echo "cgroup_cpu_stat:"; cat /sys/fs/cgroup/cpu.stat 2>/dev/null || true
    echo "cpuset_effective=$(cat /sys/fs/cgroup/cpuset.cpus.effective 2>/dev/null || echo unavailable)"
    echo "selected_cpu_frequency_khz=$(frequency_khz)"
    echo "shell_allowed_cpus=$(awk '/Cpus_allowed_list/ {print $2}' /proc/self/status)"
    echo "processes_by_cpu:"
    ps -eLo pid,ppid,tid,stat,psr,pcpu,comm,args --sort=-pcpu | head -45
  } > "$out.$phase"
}

mkdir -p "$outdir"
printf 'label=%s\n' "$label" > "$out.interference"
snapshot preflight
psi_before=$(psi_some)
if awk -v x="$psi_before" 'BEGIN {exit !(x > 2.00)}'; then
  echo "REFUSED: preflight CPU PSI some avg10=$psi_before exceeds 2.00" | tee "$out.status"
  exit 20
fi
if awk -v x="$psi_before" 'BEGIN {exit !(x >= 0.50 && x <= 2.00)}'; then
  echo "WARN: preflight CPU PSI some avg10=$psi_before is in [0.50,2.00]" | tee -a "$out.warnings"
fi
if processes=$(competing); then
  printf 'REFUSED: competing build/benchmark detected before launch\n%s\n' "$processes" | tee "$out.status"
  exit 21
fi

t0=$(date -u +%Y-%m-%dT%H:%M:%SZ)
set +e
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" /usr/bin/time -v -o "$out.time" env \
  TMPDIR="$root/target/tmp" \
  PYTHONPATH="$package:$root/target/m6-references/ducc-build" \
  RAYON_NUM_THREADS=1 \
  RUSTSHT_PERF_RAW=1 \
  RUSTSHT_BENCH_PHI0=0.37 \
  RUSTSHT_SHTNS_LIB="$root/target/m6-references/shtns-build/libshtns_omp.so" \
  OPENBLAS_NUM_THREADS=1 \
  "$root/.venv/bin/python" "$root/examples/python_perf_baseline.py" \
  "$lmax" 0 1 GL 1 into synthesis all STANDARD "$((2*(lmax+1)))" "$((lmax+1))" \
  > "$out.stdout" 2>&1 &
runner_pid=$!
{
  echo "start_utc=$t0"
  echo "wrapper_pid=$runner_pid"
  echo "requested_taskset=$cpuset"
  while kill -0 "$runner_pid" 2>/dev/null; do
    echo "sample_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "cpu_pressure:"; cat /proc/pressure/cpu
    echo "cgroup_cpu_max=$(cat /sys/fs/cgroup/cpu.max 2>/dev/null || echo unavailable)"
    echo "cgroup_cpu_stat:"; cat /sys/fs/cgroup/cpu.stat 2>/dev/null || true
    echo "selected_cpu_frequency_khz=$(frequency_khz)"
    echo "top_processes_by_cpu:"
    ps -eo pid,ppid,psr,pcpu,comm,args --sort=-pcpu | head -25
    echo "top_process_threads:"
    ps -eLo pid,ppid,tid,stat,psr,pcpu,comm,args --sort=-pcpu | head -30
    mapfile -t pids < <(ps -eo pid,comm,args | awk -v script="$root/examples/python_perf_baseline.py" '$2 ~ /^(time|python|python3)$/ && index($0,script) {print $1}')
    exclude_pids=("$runner_pid" "${pids[@]}")
    if interval_processes=$(competing "${exclude_pids[@]}"); then
      {
        echo "timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ) phase=during"
        printf '%s\n' "$interval_processes"
      } >> "$out.interference"
    fi
    for pid in "${pids[@]}"; do
      [[ -r "/proc/$pid/status" ]] || continue
      echo "observed_pid=$pid"
      ps -L -p "$pid" -o pid,tid,psr,pcpu,stat,comm,args
      for status in /proc/"$pid"/task/*/status; do
        [[ -r "$status" ]] || continue
        tid=${status%/status}; tid=${tid##*/}
        echo "thread_status_pid=$pid tid=$tid"
        grep -E '^(Name|Cpus_allowed_list|voluntary_ctxt_switches|nonvoluntary_ctxt_switches):' "$status" || true
        taskset -pc "$tid" 2>&1 || true
      done
    done
    sleep 2
  done
} > "$out.during"
wait "$runner_pid"
rc=$?
set -e
snapshot postflight
psi_after=$(psi_some)
if processes=$(competing); then
  {
    echo "timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ) phase=postflight"
    printf '%s\n' "$processes"
  } >> "$out.interference"
fi
if awk -v x="$psi_after" 'BEGIN {exit !(x >= 0.50 && x <= 2.00)}'; then
  echo "WARN: postflight CPU PSI some avg10=$psi_after is in [0.50,2.00]" | tee -a "$out.warnings"
fi

set +e
python3 "$outdir/check-stability-v2.py" "$out.stdout" > "$out.stability-check-v2"
stability_rc=$?
set -e

if awk -v x="$psi_after" 'BEGIN {exit !(x > 2.00)}'; then
  echo "FAIL: postflight CPU PSI some avg10=$psi_after exceeds 2.00" | tee "$out.status"
  exit 22
fi
if [[ $(wc -l < "$out.interference") -gt 1 ]]; then
  echo "INCONCLUSIVE: competing build/benchmark detected during or after process; inspect $out.interference and stop the cell" | tee "$out.status"
  exit 23
fi
if (( rc != 0 )); then
  echo "FAIL: benchmark process exit=$rc" | tee "$out.status"
  exit "$rc"
fi
if (( stability_rc != 0 )); then
  echo "FAIL: backend CV/drift stability gate failed; see $out.stability-check-v2" | tee "$out.status"
  exit 24
fi
# Every completed process pauses for review of placement and interference records.
# PSI warnings do not invalidate the completed sample, but require review before continuation.
echo "REVIEW: numerical and PSI gates passed; inspect local preflight, during, postflight, and warning records before launching another process" | tee "$out.status"
exit 25
