#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
set -euo pipefail

if [[ $# -ne 6 ]]; then
  echo "usage: $0 BASELINE_BINARY CANDIDATE_BINARY OUTPUT_DIRECTORY LMAX NPHI PAIRS" >&2
  exit 2
fi

baseline_binary=$1
candidate_binary=$2
output_directory=$3
lmax=$4
nphi=$5
pairs=$6
mkdir -p "$output_directory"

run_one() {
  local strategy=$1
  local binary=$2
  local label=$3
  taskset -c 0-7 /usr/bin/time \
    -f 'elapsed_process_s=%e max_rss_kib=%M' -- \
    "$binary" 0 "$lmax" "$nphi" 8 \
    > "$output_directory/$label.log" \
    2> "$output_directory/$label.time"
  printf 'pair=%s strategy=%s log=%s\n' "$label" "$strategy" "$output_directory/$label.log"
}

for pair in $(seq 1 "$pairs"); do
  if (( pair % 2 == 1 )); then
    run_one baseline "$baseline_binary" "pair-$pair-baseline"
    run_one candidate "$candidate_binary" "pair-$pair-candidate"
  else
    run_one candidate "$candidate_binary" "pair-$pair-candidate"
    run_one baseline "$baseline_binary" "pair-$pair-baseline"
  fi
done
