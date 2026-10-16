#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"
out_dir=reports/data/atmospheric-resolution-2026-10-15/python-ducc-high/repeat-after-build
mkdir -p "$out_dir" target/rustsht-python-tmp
sha256sum python/rustsht/_rustsht.abi3.so > "$out_dir/extension.sha256"
git rev-parse HEAD > "$out_dir/source-commit.txt"

for spin in 0 1; do
  label="T2047-nphi4096-spin${spin}-q1"
  TMPDIR="$PWD/target/rustsht-python-tmp" taskset -c 8 env \
    RAYON_NUM_THREADS=1 RUSTSHT_PERF_RAW=1 \
    /usr/bin/time \
    -f 'wall_seconds=%e peak_rss_kib=%M user_seconds=%U system_seconds=%S' \
    -o "$out_dir/${label}.time" \
    uv run --locked python examples/python_perf_baseline.py \
    2047 "$spin" 1 GL 2 plan both both STANDARD 4096 2048 \
    > "$out_dir/${label}.log" 2>&1
done
