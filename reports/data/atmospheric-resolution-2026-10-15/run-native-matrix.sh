#!/usr/bin/env bash
set -euo pipefail
repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"
out_dir=reports/data/atmospheric-resolution-2026-10-15/native
bin=target/release/examples/m1b_bench
if [[ ! -x "$bin" ]]; then
  echo "missing benchmark binary: $bin" >&2
  exit 2
fi
sha256sum "$bin" > "$out_dir/binary.sha256"
git rev-parse HEAD > "$out_dir/source-commit.txt"
while IFS=, read -r lmax ntheta mmax nphi sampling scope; do
  [[ "$lmax" == "lmax" ]] && continue
  for spin in 0 1; do
    label="T${lmax}-nphi${nphi}-spin${spin}-q1q8"
    log="$out_dir/${label}.log"
    tim="$out_dir/${label}.time"
    if [[ -s "$log" && -s "$tim" ]]; then
      continue
    fi
    echo "case=$label ntheta=$ntheta mmax=$mmax sampling=$sampling scope=$scope"
    taskset -c 8-15 /usr/bin/time -f 'wall_seconds=%e peak_rss_kib=%M user_seconds=%U system_seconds=%S' -o "$tim" \
      "$bin" "$spin" "$lmax" 8 both GL STANDARD "$ntheta" "$nphi" "$mmax" > "$log" 2>&1
  done
done < reports/data/atmospheric-resolution-2026-10-15/grid-matrix.csv
