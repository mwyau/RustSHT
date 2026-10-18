#!/usr/bin/env bash
set -euo pipefail
repo=$(git rev-parse --show-toplevel)
rep="$repo/reports/data/m6-m2-spin1-fused-visitor/matrix"
harness="$repo/target/m6-m2-spin1-fused-visitor/source/examples/python_perf_baseline.py"
ducc="$repo/target/m6-references/ducc-build"
python="$repo/.venv/bin/python"
base="$repo/target/m6-m2-spin1-fused-visitor/baseline-python"
candidate="$repo/target/m6-m2-spin1-fused-visitor/source/python"
recovery_index=0

preflight() {
  local stem=$1 label=$2 lmax=$3 spin=$4 threads=$5 side=$6 package=$7
  local cpuset=${RUSTSHT_CPUSET_Q1:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}} ntheta nphi psi workloads
  [[ "$threads" == 8 ]] && cpuset=${RUSTSHT_CPUSET_Q8:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}
  ntheta=$((lmax + 1)); nphi=$((2 * ntheta))
  {
    printf 'run=%s cell=%s lmax=%s spin=%s threads=%s cpuset=%s side=%s package=%s\n' "$stem" "$label" "$lmax" "$spin" "$threads" "$cpuset" "$side" "$package"
    date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
    printf '\nCPU pressure:\n'; cat /proc/pressure/cpu
    printf '\nUptime:\n'; uptime
    printf '\nTop processes:\n'; ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 20
    printf '\nActive compiler or benchmark processes:\n'
    ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | awk 'NR == 1 || $8 == "cargo" || $8 == "rustc" || $8 == "maturin" || $8 == "cc1" || $8 == "cc1plus" || (($8 == "python" || $8 ~ /^python3/) && $0 ~ /(python_perf_baseline|verify-map.py)/)'
    printf '\nRequested affinity: taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" "$cpuset"
    printf 'Settings: GL STANDARD f64, ntheta=%s nphi=%s mmax=%s phi0=0.37, reusable plan, synthesis_into, runs=1, 5 raw samples/process, backend=Rust+DUCC\n' "$ntheta" "$nphi" "$lmax"
  } > "$rep/$stem.preflight"
  psi=$(awk '$1 == "some" { for (i=2;i<=NF;i++) if ($i ~ /^avg10=/) { split($i,a,"="); print a[2] } }' "$rep/$stem.preflight")
  if [[ -z "$psi" ]] || ! awk -v psi="$psi" 'BEGIN { exit !(psi <= 0.50) }'; then
    printf 'REFUSED: cpu PSI some avg10=%s exceeds or misses fixed 0.50 gate; no process launched\n' "${psi:-unavailable}" >> "$rep/$stem.preflight"
    return 75
  fi
  workloads=$(ps -eo comm,args | awk 'NR > 1 && ($1 == "cargo" || $1 == "rustc" || $1 == "maturin" || $1 == "cc1" || $1 == "cc1plus" || (($1 == "python" || $1 ~ /^python3/) && $0 ~ /(python_perf_baseline|verify-map.py)/))')
  if [[ -n "$workloads" ]]; then
    printf 'REFUSED: active workload process found; no process launched\n%s\n' "$workloads" >> "$rep/$stem.preflight"
    return 76
  fi
  return 0
}

run_one() {
  local stem=$1 label=$2 lmax=$3 spin=$4 threads=$5 side=$6 package=$7
  local cpuset=${RUSTSHT_CPUSET_Q1:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}} ntheta nphi
  [[ "$threads" == 8 ]] && cpuset=${RUSTSHT_CPUSET_Q8:-${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}}
  ntheta=$((lmax + 1)); nphi=$((2 * ntheta))
  preflight "$stem" "$label" "$lmax" "$spin" "$threads" "$side" "$package" || return $?
  if /usr/bin/time -v -o "$rep/$stem.time" taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" env \
    PYTHONDONTWRITEBYTECODE=1 \
    TMPDIR="$repo/target/tmp" \
    PYTHONPATH="$package:$ducc" \
    RAYON_NUM_THREADS="$threads" \
    RUSTSHT_PERF_RAW=1 \
    RUSTSHT_BENCH_PHI0=0.37 \
    "$python" "$harness" "$lmax" "$spin" "$threads" GL 1 into synthesis both STANDARD "$nphi" "$ntheta" \
    > "$rep/$stem.stdout" 2>&1; then
    printf 'completed process=%s timestamp=' "$stem"; date -u '+%Y-%m-%dT%H:%M:%SZ'
    return 0
  else
    local rc=$?
    printf 'FAILED: timed process exited status=%s\n' "$rc" >> "$rep/$stem.preflight"
    return 90
  fi
}

capture_recovery() {
  local stem status psi workloads
  recovery_index=$((recovery_index + 1))
  printf -v stem 'recovery-check-%03d' "$recovery_index"
  {
    printf 'recovery_check=%s\n' "$recovery_index"
    date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
    awk '$1 == "some" || $1 == "full"' /proc/pressure/cpu
    printf 'Compiler/benchmark processes:\n'
    ps -eo pid,ppid,psr,pcpu,stat,comm,args | awk 'NR == 1 || $6 == "cargo" || $6 == "rustc" || $6 == "maturin" || $6 == "cc1" || $6 == "cc1plus" || (($6 == "python" || $6 ~ /^python3/) && $0 ~ /(python_perf_baseline|verify-map.py)/)'
  } > "$rep/$stem.preflight"
  psi=$(awk '$1 == "some" { for (i=2;i<=NF;i++) if ($i ~ /^avg10=/) { split($i,a,"="); print a[2] } }' "$rep/$stem.preflight")
  workloads=$(ps -eo comm,args | awk 'NR > 1 && ($1 == "cargo" || $1 == "rustc" || $1 == "maturin" || $1 == "cc1" || $1 == "cc1plus" || (($1 == "python" || $1 ~ /^python3/) && $0 ~ /(python_perf_baseline|verify-map.py)/))')
  status=clean
  if [[ -z "$psi" ]] || ! awk -v psi="$psi" 'BEGIN { exit !(psi <= 0.50) }' || [[ -n "$workloads" ]]; then status=not_clean; fi
  printf 'recovery_status=%s psi_some_avg10=%s\n' "$status" "${psi:-unavailable}" >> "$rep/$stem.preflight"
  printf '%s %s %s\n' "$stem" "$status" "${psi:-unavailable}"
  [[ "$status" == clean ]]
}

recover_two_clean() {
  local consecutive=0
  printf 'waiting for two clean recovery snapshots after fixed PSI/process refusal\n'
  while (( consecutive < 2 )); do
    sleep 15
    if capture_recovery; then consecutive=$((consecutive + 1)); else consecutive=0; fi
  done
}

run_pair() {
  local label=$1 lmax=$2 spin=$3 threads=$4 pair=$5 first second first_pkg second_pkg
  local attempt=1 first_stem second_stem rc
  if (( pair % 2 == 1 )); then first=baseline; second=candidate; else first=candidate; second=baseline; fi
  while true; do
    printf -v pair_id '%02d' "$pair"
    printf -v attempt_id '%02d' "$attempt"
    if [[ "$first" == baseline ]]; then first_pkg=$base; second_pkg=$candidate; else first_pkg=$candidate; second_pkg=$base; fi
    first_stem="$label-pair${pair_id}-attempt${attempt_id}-$first"
    second_stem="$label-pair${pair_id}-attempt${attempt_id}-$second"
    printf 'pair_start cell=%s pair=%s attempt=%s order=%s,%s\n' "$label" "$pair_id" "$attempt_id" "$first" "$second"
    if run_one "$first_stem" "$label" "$lmax" "$spin" "$threads" "$first" "$first_pkg"; then
      :
    else
      rc=$?
      if (( rc != 75 && rc != 76 )); then printf 'fatal first-side failure rc=%s stem=%s\n' "$rc" "$first_stem"; return "$rc"; fi
      printf 'pair_attempt_refused cell=%s pair=%s attempt=%s refused_side=%s rc=%s no_pair_samples_selected\n' "$label" "$pair_id" "$attempt_id" "$first" "$rc"
      recover_two_clean
      attempt=$((attempt + 1))
      continue
    fi
    if run_one "$second_stem" "$label" "$lmax" "$spin" "$threads" "$second" "$second_pkg"; then
      printf 'pair_complete cell=%s pair=%s attempt=%s order=%s,%s\n' "$label" "$pair_id" "$attempt_id" "$first" "$second"
      break
    else
      rc=$?
      if (( rc != 75 && rc != 76 )); then printf 'fatal second-side failure rc=%s stem=%s\n' "$rc" "$second_stem"; return "$rc"; fi
      printf 'pair_attempt_refused cell=%s pair=%s attempt=%s completed_orphan_side=%s refused_side=%s rc=%s; preserve orphan and restart pair\n' "$label" "$pair_id" "$attempt_id" "$first" "$second" "$rc"
      recover_two_clean
      attempt=$((attempt + 1))
    fi
  done
}

run_cell() {
  local label=$1 lmax=$2 spin=$3 threads=$4 pair
  printf 'cell_start label=%s lmax=%s spin=%s threads=%s pairs=12\n' "$label" "$lmax" "$spin" "$threads"
  for pair in $(seq 1 12); do run_pair "$label" "$lmax" "$spin" "$threads" "$pair"; done
  printf 'cell_complete label=%s pairs=12\n' "$label"
}

printf 'matrix_start timestamp='; date -u '+%Y-%m-%dT%H:%M:%SZ'
printf 'protocol=12_pairs_per_cell cells=12 synthesis_only alternating fresh processes psi_some_avg10<=0.50\n'
run_cell spin0-t511-q1 511 0 1
run_cell spin1-t511-q1 511 1 1
run_cell spin0-t511-q8 511 0 8
run_cell spin1-t511-q8 511 1 8
run_cell spin0-t1023-q1 1023 0 1
run_cell spin1-t1023-q1 1023 1 1
run_cell spin0-t1023-q8 1023 0 8
run_cell spin1-t1023-q8 1023 1 8
run_cell spin0-t2047-q1 2047 0 1
run_cell spin1-t2047-q1 2047 1 1
run_cell spin0-t2047-q8 2047 0 8
run_cell spin1-t2047-q8 2047 1 8
printf 'matrix_complete timestamp='; date -u '+%Y-%m-%dT%H:%M:%SZ'
