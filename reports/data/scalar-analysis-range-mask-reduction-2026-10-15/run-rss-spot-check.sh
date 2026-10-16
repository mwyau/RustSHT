#!/usr/bin/env bash
set -euo pipefail
base="${RUSTSHT_WORKTREE_DIR:?set RUSTSHT_WORKTREE_DIR}/target/m1b_bench-range-mask-baseline"
candidate="${RUSTSHT_WORKTREE_DIR:?set RUSTSHT_WORKTREE_DIR}/target/m1b_bench-range-mask-candidate"
out="${RUSTSHT_WORKTREE_DIR:?set RUSTSHT_WORKTREE_DIR}/reports/data/scalar-analysis-range-mask-reduction-2026-10-15/rss-spot-check.txt"
{
  printf 'Started: '; date -Is
  printf 'Load before: '; uptime
  for spec in '1 8' '8 8-15'; do
    read -r threads affinity <<< "$spec"
    for variant in baseline candidate; do
      if [[ "$variant" == baseline ]]; then binary="$base"; else binary="$candidate"; fi
      printf '\nTHREADS=%s ORDER=%s\n' "$threads" "$variant"
      taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" /usr/bin/time -f 'elapsed_s=%e max_rss_kb=%M' "$binary" 0 1023 "$threads" analysis GL STANDARD 1024 2049 1023 requested-only
    done
  done
  printf '\nLoad after: '; uptime
  printf 'Finished: '; date -Is
} > "$out" 2>&1
