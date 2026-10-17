#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
pair_runner="$repo_dir/reports/data/m6-p01/link-order/causal-ab/run-pair.sh"
summary="$repo_dir/reports/data/m6-p01/link-order/causal-ab/summarize.py"

for workload in "511 1" "1023 0"; do
    read -r lmax spin <<< "$workload"
    for pair in 1 2 3 4 5 6; do
        bash "$pair_runner" "$lmax" "$spin" "$pair"
    done
done

python3 "$summary"
