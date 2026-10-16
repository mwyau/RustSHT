#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"
out_dir=reports/data/atmospheric-resolution-2026-10-15/native/q16
bin=target/release/examples/m1b_bench
mkdir -p "$out_dir"
sha256sum "$bin" > "$out_dir/binary.sha256"
git rev-parse HEAD > "$out_dir/run-tree-commit.txt"
printf '%s\n' 2756ba4a1d9bcf6b7b1a1293a185e98e8e6c74b2 > "$out_dir/binary-source-revision.txt"

for item in "1023 2048" "1279 2560"; do
  read -r lmax nphi <<< "$item"
  ntheta=$((lmax + 1))
  for spin in 0 1; do
    label="T${lmax}-nphi${nphi}-spin${spin}-q1q16"
    taskset -c 0-15 /usr/bin/time \
      -f 'wall_seconds=%e peak_rss_kib=%M user_seconds=%U system_seconds=%S' \
      -o "$out_dir/${label}.time" \
      "$bin" "$spin" "$lmax" 16 both GL STANDARD "$ntheta" "$nphi" "$lmax" \
      > "$out_dir/${label}.log" 2>&1
  done
done
