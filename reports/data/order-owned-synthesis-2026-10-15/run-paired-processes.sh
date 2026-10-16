#!/usr/bin/env bash
set -euo pipefail

baseline=$1
candidate=$2

for degree in 511 1023; do
  for threads in 1 8; do
    if [[ $threads == 1 ]]; then
      cpus=0
    else
      cpus=0-7
    fi

    for pair in 1 2 3 4 5; do
      if ((pair % 2 == 1)); then
        order=(baseline candidate)
      else
        order=(candidate baseline)
      fi
      for label in "${order[@]}"; do
        if [[ $label == baseline ]]; then
          binary=$baseline
        else
          binary=$candidate
        fi
        printf 'PAIR degree=%s threads=%s pair=%s binary=%s\n' "$degree" "$threads" "$pair" "$label"
        RUSTSHT_LEGENDRE_REUSE_ESTIMATE=-,0 taskset -c "$cpus" "$binary" \
          0 "$degree" "$threads" synthesis GL STANDARD "$((degree + 1))" \
          "$((2 * degree + 3))" "$degree" requested-only
      done
    done
  done
done
