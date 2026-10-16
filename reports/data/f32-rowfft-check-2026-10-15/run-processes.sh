#!/usr/bin/env bash
set -euo pipefail

binary=$1

for run in 1 2 3 4 5; do
  printf 'PROCESS run=%s binary=%s\n' "$run" "$binary"
  /usr/bin/time -f 'max_rss_kib=%M' taskset -c 0-7 "$binary"
done
