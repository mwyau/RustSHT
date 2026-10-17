#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
report_dir="$repo_dir/reports/data/m6-m2-general-visitor"
runner="$report_dir/run-one.sh"
baseline="$repo_dir/target/m6-m2-general-visitor-baseline-bench"
candidate="$repo_dir/target/m6-m2-general-visitor-candidate-bench"

run_cell() {
    local lmax=$1
    local spin=$2
    local pair=$3
    if (( pair % 2 == 1 )); then
        bash "$runner" "$baseline" "$lmax" "$spin" "$pair" baseline
        bash "$runner" "$candidate" "$lmax" "$spin" "$pair" candidate
    else
        bash "$runner" "$candidate" "$lmax" "$spin" "$pair" candidate
        bash "$runner" "$baseline" "$lmax" "$spin" "$pair" baseline
    fi
}

for pair in 1 2 3 4 5 6; do
    run_cell 511 0 "$pair"
    run_cell 511 1 "$pair"
    run_cell 1023 0 "$pair"
done

python3 "$report_dir/summarize.py"
