#!/usr/bin/env bash
set -euo pipefail
repo=$(git rev-parse --show-toplevel)
rep="$repo/reports/data/m6-m2-spin1-fused-visitor"
harness="$repo/target/m6-m2-spin1-fused-visitor/source/examples/python_perf_baseline.py"
ducc="$repo/target/m6-references/ducc-build"
python="$repo/.venv/bin/python"
base="$repo/target/m6-m2-spin1-fused-visitor/baseline-python"
candidate="$repo/target/m6-m2-spin1-fused-visitor/source/python"
run_one() {
  stem=$1; package=$2; lmax=511; threads=1; cpuset=${RUSTSHT_CPUSET_Q1:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}; ntheta=512; nphi=1024
  preflight="$rep/$stem.preflight"
  {
    printf 'run=%s lmax=511 spin=1 threads=1 affinity=pinned package=%s\n' "$stem" "$package"
    date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
    printf '\nCPU pressure:\n'; cat /proc/pressure/cpu
    printf '\nUptime:\n'; uptime
    printf '\nTop processes:\n'; ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 20
    printf '\nActive compiler or benchmark processes:\n'
    ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | awk 'NR == 1 || $8 == "cargo" || $8 == "rustc" || $8 == "maturin" || $8 == "cc1" || $8 == "cc1plus" || (($8 == "python" || $8 ~ /^python3/) && $0 ~ /(python_perf_baseline|verify-map.py)/)'
    printf '\nRequested affinity: taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET}\nSettings: GL STANDARD f64, ntheta=512 nphi=1024 mmax=511 phi0=0.37, reusable plan, synthesis_into, runs=1, 5 raw samples/process, backend=Rust+DUCC\n'
  } > "$preflight"
  psi=$(awk '$1 == "some" { for (i=2;i<=NF;i++) if ($i ~ /^avg10=/) { split($i,a,"="); print a[2] } }' "$preflight")
  if ! awk -v psi="$psi" 'BEGIN { exit !(psi <= 0.50) }'; then printf 'REFUSED: cpu PSI some avg10=%s exceeds fixed 0.50 gate; no process launched\n' "$psi" >> "$preflight"; return 75; fi
  workloads=$(ps -eo comm,args | awk 'NR > 1 && ($1 == "cargo" || $1 == "rustc" || $1 == "maturin" || $1 == "cc1" || $1 == "cc1plus" || (($1 == "python" || $1 ~ /^python3/) && $0 ~ /(python_perf_baseline|verify-map.py)/))')
  if [[ -n "$workloads" ]]; then printf 'REFUSED: active workload process found; no process launched\n%s\n' "$workloads" >> "$preflight"; return 76; fi
  /usr/bin/time -v -o "$rep/$stem.time" taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} env PYTHONDONTWRITEBYTECODE=1 TMPDIR="$repo/target/tmp" PYTHONPATH="$package:$ducc" RAYON_NUM_THREADS=1 RUSTSHT_PERF_RAW=1 RUSTSHT_BENCH_PHI0=0.37 "$python" "$harness" 511 1 1 GL 1 into synthesis both STANDARD 1024 512 > "$rep/$stem.stdout" 2>&1
}
run_one control-t511-q1-pair06-retry1-candidate "$candidate"
run_one control-t511-q1-pair06-retry1-baseline "$base"
