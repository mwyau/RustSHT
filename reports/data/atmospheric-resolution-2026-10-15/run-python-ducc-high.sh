#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"
out_dir=reports/data/atmospheric-resolution-2026-10-15/python-ducc-high
mkdir -p "$out_dir" target/rustsht-python-tmp
sha256sum python/rustsht/_rustsht.abi3.so > "$out_dir/extension.sha256"
git rev-parse HEAD > "$out_dir/source-commit.txt"
uv run --locked python --version > "$out_dir/python-version.txt"
uv run --locked python -c 'import numpy, ducc0; print("numpy", numpy.__version__); print("ducc0", ducc0.__version__)' > "$out_dir/package-versions.txt"

for item in "720 1458" "1023 2048" "1279 2560" "2047 4096"; do
  read -r lmax nphi <<< "$item"
  ntheta=$((lmax + 1))
  for spin in 0 1; do
    for threads in 1 8; do
      label="T${lmax}-nphi${nphi}-spin${spin}-q${threads}"
      if (( threads == 1 )); then
        cpus=8
      else
        cpus=0-7
      fi
      TMPDIR="$PWD/target/rustsht-python-tmp" taskset -c "$cpus" env \
        RAYON_NUM_THREADS="$threads" RUSTSHT_PERF_RAW=1 \
        /usr/bin/time \
        -f 'wall_seconds=%e peak_rss_kib=%M user_seconds=%U system_seconds=%S' \
        -o "$out_dir/${label}.time" \
        uv run --locked python examples/python_perf_baseline.py \
        "$lmax" "$spin" "$threads" GL 2 plan both both STANDARD "$nphi" "$ntheta" \
        > "$out_dir/${label}.log" 2>&1
    done
  done
done
