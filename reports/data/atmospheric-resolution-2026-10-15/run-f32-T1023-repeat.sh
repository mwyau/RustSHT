#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"
out_dir=reports/data/atmospheric-resolution-2026-10-15/native/f32/repeat-CPU worker-placement
bin=target/release/examples/m1_f32_atmospheric_bench
mkdir -p "$out_dir"
sha256sum "$bin" > "$out_dir/binary.sha256"
git rev-parse HEAD > "$out_dir/source-commit.txt"

for spin in 0 1; do
  label="T1023-nphi2048-spin${spin}-f32-q1q8"
  taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} /usr/bin/time \
    -f 'wall_seconds=%e peak_rss_kib=%M user_seconds=%U system_seconds=%S' \
    -o "$out_dir/${label}.time" \
    "$bin" "$spin" 1023 2048 8 \
    > "$out_dir/${label}.log" 2>&1
done
