#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
out_dir="$repo_dir/reports/data/m6-p01/codegen-diagnosis/cachegrind"
variants=(baseline default-p01 ordered-p01)
binaries=(
  "$repo_dir/target/m6-p01-baseline-bench"
  "$repo_dir/target/m6-p01-candidate-bench"
  "$repo_dir/target/m6-p01-link-order-bench"
)

for index in "${!variants[@]}"; do
  variant=${variants[$index]}
  binary=${binaries[$index]}
  prefix="$out_dir/$variant"
  for suffix in cachegrind.out cachegrind.log annotate.txt stdout.txt time.txt; do
    if [[ -e "$prefix.$suffix" ]]; then
      printf 'refusing to overwrite %s\n' "$prefix.$suffix" >&2
      exit 2
    fi
  done

  some_avg10=$(awk '/some/ {for (i=1; i<=NF; i++) if ($i ~ /^avg10=/) {sub("avg10=", "", $i); print $i}}' /proc/pressure/cpu)
  {
    printf '\n--- variant=%s timestamp=%s ---\n' "$variant" "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    sha256sum "$binary"
    printf 'cpu_some_avg10=%s\n' "$some_avg10"
    cat /proc/pressure/cpu
    ps -eo pid,ppid,psr,pcpu,stat,comm,args --sort=-pcpu | head -n 16
  } >> "$out_dir/preflight-processes.log"
  if awk -v value="$some_avg10" 'BEGIN {exit !(value >= 8.0)}'; then
    printf 'CPU PSI some avg10 is %s%%; refusing run\n' "$some_avg10" >&2
    exit 75
  fi

  taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} /usr/bin/time -v -o "$prefix.time.txt" \
    env RAYON_NUM_THREADS=1 RUSTSHT_BENCH_PHI0=0.37 \
    valgrind --tool=cachegrind --cache-sim=yes --branch-sim=yes \
      --cachegrind-out-file="$prefix.cachegrind.out" \
      --log-file="$prefix.cachegrind.log" \
      "$binary" 1 511 1 synthesis GL STANDARD 512 1024 511 requested-only \
      > "$prefix.stdout.txt" 2>&1

  cg_annotate --auto=yes "$prefix.cachegrind.out" > "$prefix.annotate.txt"
done
