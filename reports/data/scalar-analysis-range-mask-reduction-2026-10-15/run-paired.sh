#!/usr/bin/env bash
set -euo pipefail
base="/home/albert/RustSHT-sht-performance/target/m1b_bench-range-mask-baseline"
candidate="/home/albert/RustSHT-sht-performance/target/m1b_bench-range-mask-candidate"
out="/home/albert/RustSHT-sht-performance/target/range-mask-screen/raw.txt"
{
  printf 'Started: '; date -Is
  printf 'Baseline: d188b772587ed377fbe5b9772ab31c79b324b4bb\n'
  printf 'Candidate: '
  git -C /home/albert/RustSHT-sht-performance rev-parse HEAD
  printf 'Affinity: q1 CPU 8; q8 CPUs 8-15\n'
  printf 'Load before: '
  uptime
  for spec in '511 1 8' '511 8 8-15' '1023 1 8' '1023 8 8-15'; do
    read -r lmax threads affinity <<< "$spec"
    for pair in 1 2 3 4 5; do
      if (( pair % 2 )); then
        first=baseline
        second=candidate
      else
        first=candidate
        second=baseline
      fi
      for variant in "$first" "$second"; do
        if [[ "$variant" == baseline ]]; then binary="$base"; else binary="$candidate"; fi
        printf '\nPAIR=%s DEGREE=%s THREADS=%s ORDER=%s\n' "$pair" "$lmax" "$threads" "$variant"
        taskset -c "$affinity" "$binary" 0 "$lmax" "$threads" analysis GL STANDARD "$((lmax + 1))" "$((2 * lmax + 3))" "$lmax" requested-only
      done
    done
  done
  printf '\nLoad after: '
  uptime
  printf 'Finished: '
  date -Is
} > "$out"
