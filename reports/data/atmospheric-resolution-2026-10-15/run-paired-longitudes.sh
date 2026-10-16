#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"
root=reports/data/atmospheric-resolution-2026-10-15
out_dir="$root/native/paired-longitudes"
bin=target/release/examples/m1b_bench
mkdir -p "$out_dir"
sha256sum "$bin" > "$out_dir/binary.sha256"
git rev-parse HEAD > "$out_dir/source-commit.txt"

run_case() {
  local group=$1 pair=$2 nphi_label=$3 spin=$4 lmax=$5 ntheta=$6 nphi=$7
  local label="${group}-pair${pair}-T${lmax}-nphi${nphi_label}-spin${spin}"
  local log="$out_dir/${label}.log"
  local tim="$out_dir/${label}.time"
  if [[ -s "$log" && -s "$tim" ]]; then
    return
  fi
  echo "case=$label"
  taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} /usr/bin/time \
    -f 'wall_seconds=%e peak_rss_kib=%M user_seconds=%U system_seconds=%S' \
    -o "$tim" "$bin" "$spin" "$lmax" 8 both GL STANDARD "$ntheta" "$nphi" "$lmax" \
    > "$log" 2>&1
}

paired_grid() {
  local group=$1 lmax=$2 ntheta=$3 odd=$4 even=$5 fft=${6:-}
  local pair spin first second
  for pair in 1 2; do
    if [[ "$pair" == 1 ]]; then
      first=odd
      second=even
    else
      first=even
      second=odd
    fi
    for spin in 0 1; do
      for variant in "$first" "$second"; do
        if [[ "$variant" == odd ]]; then
          run_case "$group" "$pair" "$odd" "$spin" "$lmax" "$ntheta" "$odd"
        else
          run_case "$group" "$pair" "$even" "$spin" "$lmax" "$ntheta" "$even"
        fi
      done
    done
  done
  if [[ -n "$fft" ]]; then
    for pair in 1 2; do
      if [[ "$pair" == 1 ]]; then
        first=fft
        second=even
      else
        first=even
        second=fft
      fi
      for spin in 0 1; do
        for variant in "$first" "$second"; do
          if [[ "$variant" == fft ]]; then
            run_case "fft-vs-even" "$pair" "$fft" "$spin" "$lmax" "$ntheta" "$fft"
          else
            run_case "fft-vs-even" "$pair" "$even" "$spin" "$lmax" "$ntheta" "$even"
          fi
        done
      done
    done
  fi
}

# Every pair contains both fresh-process measurements. Pair 2 reverses the
# process order to reduce systematic warm-cache and drift effects.
paired_grid odd-vs-even 720 721 1441 1442 1458
paired_grid odd-vs-even 1023 1024 2047 2048
paired_grid odd-vs-even 1279 1280 2559 2560
paired_grid odd-vs-even 2047 2048 4095 4096
