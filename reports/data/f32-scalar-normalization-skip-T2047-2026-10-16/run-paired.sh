#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
set -euo pipefail

if [[ $# -ne 6 ]]; then
  echo "usage: $0 BASELINE_BINARY CANDIDATE_BINARY OUTPUT_DIRECTORY NPHI PAIRS SAMPLES" >&2
  exit 2
fi

baseline_binary=$1
candidate_binary=$2
output_directory=$3
nphi=$4
pairs=$5
samples=$6
mkdir -p "$output_directory"

run_one() {
  local strategy=$1
  local binary=$2
  local spin=$3
  local label=$4
  printf 'start=%s strategy=%s spin=%s\n' "$label" "$strategy" "$spin"
  RUSTSHT_M1_SAMPLES="$samples" taskset -c 0-7 /usr/bin/time \
    -f 'elapsed_process_s=%e max_rss_kib=%M' -- \
    "$binary" "$spin" 2047 "$nphi" 8 \
    > "$output_directory/$label.log" \
    2> "$output_directory/$label.time"
  printf 'finish=%s strategy=%s spin=%s\n' "$label" "$strategy" "$spin"
}

for pair in $(seq 1 "$pairs"); do
  if (( pair % 2 == 1 )); then
    run_one baseline "$baseline_binary" 0 "pair-$pair-baseline-spin0"
    run_one candidate "$candidate_binary" 0 "pair-$pair-candidate-spin0"
    run_one baseline "$baseline_binary" 1 "pair-$pair-baseline-spin1"
    run_one candidate "$candidate_binary" 1 "pair-$pair-candidate-spin1"
  else
    run_one candidate "$candidate_binary" 0 "pair-$pair-candidate-spin0"
    run_one baseline "$baseline_binary" 0 "pair-$pair-baseline-spin0"
    run_one candidate "$candidate_binary" 1 "pair-$pair-candidate-spin1"
    run_one baseline "$baseline_binary" 1 "pair-$pair-baseline-spin1"
  fi
done
