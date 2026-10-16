#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"
out_dir=reports/data/atmospheric-resolution-2026-10-15/native/paired-longitudes
bin=target/release/examples/m1b_bench

for pair in 3 4; do
  if [[ "$pair" == 3 ]]; then
    first=4095
    second=4096
  else
    first=4096
    second=4095
  fi
  for nphi in "$first" "$second"; do
    label="odd-vs-even-pair${pair}-T2047-nphi${nphi}-spin1"
    taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET},8-13,15 /usr/bin/time \
      -f 'wall_seconds=%e peak_rss_kib=%M user_seconds=%U system_seconds=%S' \
      -o "$out_dir/${label}.time" \
      "$bin" 1 2047 8 both GL STANDARD 2048 "$nphi" 2047 \
      > "$out_dir/${label}.log" 2>&1
  done
done
