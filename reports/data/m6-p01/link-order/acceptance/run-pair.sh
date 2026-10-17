#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
lmax=${1:?lmax required}
spin=${2:?spin required}
threads=${3:?thread count required}
pair=${4:?pair number required}
baseline="$repo_dir/target/m6-p01-baseline-bench"
ordered="$repo_dir/target/m6-p01-link-order-bench"
runner="$repo_dir/reports/data/m6-p01/link-order/acceptance/run-one.sh"

run_baseline() {
    bash "$runner" "$baseline" "$lmax" "$spin" "$threads" "$pair" baseline
}
run_ordered() {
    bash "$runner" "$ordered" "$lmax" "$spin" "$threads" "$pair" ordered
}

if (( pair % 2 == 1 )); then
    run_baseline
    run_ordered
else
    run_ordered
    run_baseline
fi
