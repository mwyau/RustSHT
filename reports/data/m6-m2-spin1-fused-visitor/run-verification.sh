#!/usr/bin/env bash
set -euo pipefail
repo=$(git rev-parse --show-toplevel)
rep="$repo/reports/data/m6-m2-spin1-fused-visitor"
verify="$rep/verify-map.py"
ducc="$repo/target/m6-references/ducc-build"
python="$repo/.venv/bin/python"
run_one() {
  label=$1; lmax=$2; threads=$3; package=$4; output=$5
  cpuset=${RUSTSHT_CPUSET_Q1:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}
  if [[ "$threads" == 8 ]]; then cpuset=${RUSTSHT_CPUSET_Q8:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}; fi
  preflight="$rep/$label.preflight"
  {
    printf 'label=%s lmax=%s threads=%s package=%s cpuset=%s\n' "$label" "$lmax" "$threads" "$package" "$cpuset"
    date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
    cat /proc/pressure/cpu
    printf '\nactive workload/compiler processes:\n'
    ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | awk 'NR == 1 || $10 == "cargo" || $10 == "rustc" || $10 == "maturin" || $10 == "cc1" || $10 == "cc1plus" || (($10 == "python" || $10 ~ /^python3/) && $0 ~ /(python_perf_baseline|verify-map.py)/)'
    printf '\nrequested affinity: %s\n' "$cpuset"
  } > "$preflight"
  psi=$(awk '$1 == "some" { for (i=2;i<=NF;i++) if ($i ~ /^avg10=/) { split($i,a,"="); print a[2] } }' "$preflight")
  awk -v psi="$psi" 'BEGIN { if (psi > 0.50) exit 1 }' || {
    printf 'refused: cpu PSI some avg10=%s exceeds fixed 0.50 gate\n' "$psi" >> "$preflight"
    return 75
  }
  /usr/bin/time -v -o "$rep/$label.time" taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" env \
    PYTHONPATH="$package:$ducc" \
    RAYON_NUM_THREADS="$threads" \
    "$python" "$verify" "$lmax" "$threads" "$repo/target/m6-m2-spin1-fused-visitor/$output" \
    > "$rep/$label.stdout" 2>&1
}
run_one verify-t2047-q8-baseline 2047 8 "$repo/target/m6-m2-spin1-fused-visitor/baseline-python" baseline-t2047-q8.npy
run_one verify-t2047-q8-candidate 2047 8 "$repo/target/m6-m2-spin1-fused-visitor/source/python" candidate-t2047-q8.npy
run_one verify-t511-q1-baseline 511 1 "$repo/target/m6-m2-spin1-fused-visitor/baseline-python" baseline-t511-q1.npy
run_one verify-t511-q1-candidate 511 1 "$repo/target/m6-m2-spin1-fused-visitor/source/python" candidate-t511-q1.npy
