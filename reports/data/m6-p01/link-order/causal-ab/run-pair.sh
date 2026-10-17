#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
lmax=${1:?lmax required}
spin=${2:?spin required}
pair=${3:?pair number required}
baseline="$repo_dir/target/m6-p01-candidate-bench"
ordered="$repo_dir/target/m6-p01-link-order-bench"
runner="$repo_dir/reports/data/m6-p01/link-order/causal-ab/run-one.sh"

run_baseline() {
    bash "$runner" "$baseline" "$lmax" "$spin" "$pair" baseline
}
run_ordered() {
    bash "$runner" "$ordered" "$lmax" "$spin" "$pair" ordered
}

if (( pair % 2 == 1 )); then
    run_baseline
    run_ordered
else
    run_ordered
    run_baseline
fi
