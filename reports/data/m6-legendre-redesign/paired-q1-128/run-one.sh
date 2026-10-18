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
out="$outdir/$label"
mkdir -p "$outdir"

snapshot() {
  local file=$1
  {
    echo "timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "label=$label phase=$file variant=$variant T=$lmax requested_affinity=pinned"
    echo "uptime=$(cat /proc/uptime)"
    echo "cpu_pressure:"; cat /proc/pressure/cpu
    echo "cgroup_cpu_max=$(cat /sys/fs/cgroup/cpu.max 2>/dev/null || echo unavailable)"
    echo "cgroup_cpu_stat:"; cat /sys/fs/cgroup/cpu.stat 2>/dev/null || true
    echo "cpuset_effective=$(cat /sys/fs/cgroup/cpuset.cpus.effective 2>/dev/null || echo unavailable)"
    echo "selected_cpu_frequency_khz=$(frequency_khz)"
    echo "shell_allowed_cpus=$(awk '/Cpus_allowed_list/ {print $2}' /proc/self/status)"
    echo "processes:"
    ps -eo pid,ppid,stat,psr,pcpu,comm,args --sort=-pcpu | head -35
  } > "$out.$file"
}

competing() {
  ps -eo pid,comm,args | awk '
    $2 ~ /^(cargo|rustc|cc|cc1|cc1plus|maturin|ninja|make|cmake|m1b_bench|time_SHT)$/ {print; found=1}
    $2 ~ /^python/ && $0 ~ /(python_perf_baseline\.py|m1b_bench|time_SHT)/ {print; found=1}
    END {exit !found}'
}

snapshot preflight
psi=$(awk '/^some / {for (i=1;i<=NF;i++) if ($i ~ /^avg10=/) {split($i,a,"="); print a[2]}}' /proc/pressure/cpu)
if awk -v x="$psi" 'BEGIN {exit !(x > 0.50)}'; then
  echo "REFUSED: preflight CPU PSI some avg10=$psi exceeds 0.50" | tee "$out.status"
  exit 20
fi
if processes=$(competing); then
  printf 'REFUSED: competing workload detected\n%s\n' "$processes" | tee "$out.status"
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
  echo "requested_taskset=0"
  while kill -0 "$runner_pid" 2>/dev/null; do
    echo "sample_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    ps -eo pid,ppid,stat,psr,pcpu,comm,args | awk '/python_perf_baseline\.py/ && !/awk/ {print}'
    ps -eo pid,ppid,stat,psr,pcpu,comm,args | awk -v p="$runner_pid" '$1==p {print}'
    for p in $(ps -eo pid,args | awk '/python_perf_baseline\.py/ && !/awk/ {print $1}'); do
      echo "python_pid=$p"
      grep -E '^(Name|Cpus_allowed_list|voluntary_ctxt_switches|nonvoluntary_ctxt_switches):' "/proc/$p/status" 2>/dev/null || true
      taskset -pc "$p" 2>&1 || true
    done
    echo "selected_cpu_frequency_khz=$(frequency_khz)"
    echo "cpu_psi=$(tr '\n' ';' </proc/pressure/cpu)"
    sleep 2
  done
} > "$out.during"
wait "$runner_pid"
rc=$?
set -e
snapshot postflight
psi_after=$(awk '/^some / {for (i=1;i<=NF;i++) if ($i ~ /^avg10=/) {split($i,a,"="); print a[2]}}' /proc/pressure/cpu)
if awk -v x="$psi_after" 'BEGIN {exit !(x > 0.50)}'; then
  echo "FAIL: postflight CPU PSI some avg10=$psi_after exceeds 0.50" | tee "$out.status"
  exit 22
fi
if (( rc != 0 )); then
  echo "FAIL: process exit=$rc" | tee "$out.status"
  exit "$rc"
fi
if processes=$(competing); then
  printf 'FAIL: competing workload seen at postflight\n%s\n' "$processes" | tee "$out.status"
  exit 23
fi
python3 "$outdir/check-stability.py" "$out.stdout" > "$out.stability-check"
stability_rc=$?
if (( stability_rc != 0 )); then
  echo "FAIL: backend stability gate failed; see $out.stability-check" | tee "$out.status"
  exit 24
fi
echo "PASS: pre/post CPU PSI <=0.50, no competing workload, all backend stability gates passed; exit=0" | tee "$out.status"
