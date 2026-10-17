#!/usr/bin/env bash
set -euo pipefail

runner=$(git rev-parse --show-toplevel)/reports/data/m6-p01/link-order/acceptance/run-pair.sh
for cell in '511 0 1' '511 1 1' '1023 0 1' '2047 0 1' '1023 0 8' '2047 0 8'; do
    read -r lmax spin threads <<< "$cell"
    for pair in 1 2 3 4 5 6; do
        bash "$runner" "$lmax" "$spin" "$threads" "$pair"
    done
done
