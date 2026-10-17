#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
lmax=${1:?lmax required}
spin=${2:?spin required}
pair=${3:?pair number required}
baseline="$repo_dir/target/m6-p01-baseline-bench"
candidate="$repo_dir/reports/data/m6-p02/precomputed-visitor/candidate-bench"
runner="$repo_dir/reports/data/m6-p02/precomputed-visitor/run-one.sh"

run_baseline() { bash "$runner" "$baseline" "$lmax" "$spin" "$pair" baseline; }
run_candidate() { bash "$runner" "$candidate" "$lmax" "$spin" "$pair" candidate; }

if (( pair % 2 == 1 )); then
    run_baseline
    run_candidate
else
    run_candidate
    run_baseline
fi
