#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"
out_dir=reports/data/atmospheric-resolution-2026-10-15/batch-T720
bin=target/release/examples/m5_batch_bench
sha256sum "$bin" > "$out_dir/binary.sha256"
git rev-parse HEAD > "$out_dir/source-commit.txt"

for spin in 0 1; do
  for operation in synthesis analysis; do
    for fields in 8 32; do
      label="spin${spin}-${operation}-fields${fields}-sequential"
      taskset -c 0-7 /usr/bin/time \
        -f 'wall_seconds=%e peak_rss_kib=%M user_seconds=%U system_seconds=%S' \
        -o "$out_dir/${label}.time" \
        "$bin" "$spin" 720 8 "$operation" "$fields" sequential 721 1458 720 5 \
        > "$out_dir/${label}.log" 2>&1
    done
  done
done
