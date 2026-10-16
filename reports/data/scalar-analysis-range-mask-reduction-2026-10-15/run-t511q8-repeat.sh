#!/usr/bin/env bash
set -euo pipefail
base="/home/albert/RustSHT-sht-performance/target/m1b_bench-range-mask-baseline"
candidate="/home/albert/RustSHT-sht-performance/target/m1b_bench-range-mask-candidate"
out="/home/albert/RustSHT-sht-performance/target/range-mask-screen/t511-q8-repeat.txt"
{
  printf 'Started: '; date -Is
  printf 'Load before: '; uptime
  for pair in 6 7 8 9 10; do
    if (( pair % 2 )); then first=candidate; second=baseline; else first=baseline; second=candidate; fi
    for variant in "$first" "$second"; do
      if [[ "$variant" == baseline ]]; then binary="$base"; else binary="$candidate"; fi
      printf '\nPAIR=%s DEGREE=511 THREADS=8 ORDER=%s\n' "$pair" "$variant"
      taskset -c 8-15 "$binary" 0 511 8 analysis GL STANDARD 512 1025 511 requested-only
    done
  done
  printf '\nLoad after: '; uptime
  printf 'Finished: '; date -Is
} > "$out"
