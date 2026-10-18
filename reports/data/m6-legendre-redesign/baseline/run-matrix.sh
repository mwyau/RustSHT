#!/usr/bin/env bash
set -euo pipefail
script=$(git rev-parse --show-toplevel)/reports/data/m6-legendre-redesign/baseline/run-one.sh
for entry in 511:1 1023:1 2047:1 511:2 1023:2 2047:2 511:3 1023:3 2047:3; do
    lmax=${entry%%:*}
    run_id="r${entry##*:}"
    "$script" "$lmax" "$run_id"
done
