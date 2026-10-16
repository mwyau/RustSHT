#!/usr/bin/env bash
set -euo pipefail
base="/home/albert/RustSHT-sht-performance/target/m1b_bench-range-mask-baseline"
candidate="/home/albert/RustSHT-sht-performance/target/m1b_bench-range-mask-candidate"
out="/home/albert/RustSHT-sht-performance/target/range-mask-screen/q8-repeat.txt"
{
  printf 'Started: '; date -Is
  printf 'Load before: '; uptime
  for spec in '511 11 20' '1023 6 10'; do
    read -r lmax first_pair last_pair <<< "$spec"
    for pair in $(seq "$first_pair" "$last_pair"); do
      if (( pair % 2 )); then first=candidate; second=baseline; else first=baseline; second=candidate; fi
      for variant in "$first" "$second"; do
        if [[ "$variant" == baseline ]]; then binary="$base"; else binary="$candidate"; fi
        printf '\nPAIR=%s DEGREE=%s THREADS=8 ORDER=%s\n' "$pair" "$lmax" "$variant"
        taskset -c 8-15 "$binary" 0 "$lmax" 8 analysis GL STANDARD "$((lmax + 1))" "$((2 * lmax + 3))" "$lmax" requested-only
      done
    done
  done
  printf '\nLoad after: '; uptime
  printf 'Finished: '; date -Is
} > "$out"
