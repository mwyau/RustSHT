#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
lmax=${1:?lmax required}
spin=${2:?spin required}
threads=${3:?thread count required}
pair=${4:?pair number required}
baseline="$repo_dir/target/m6-p01-baseline-bench"
wrapper="$repo_dir/target/m6-p01-codegen-wrapper-outline-bench"
runner="$repo_dir/reports/data/m6-p01/codegen-wrapper-outline/paired/run-one.sh"

run_baseline() {
    bash "$runner" "$baseline" "$lmax" "$spin" "$threads" "$pair" baseline
}
run_wrapper() {
    bash "$runner" "$wrapper" "$lmax" "$spin" "$threads" "$pair" wrapper
}

if (( pair % 2 == 1 )); then
    run_baseline
    run_wrapper
else
    run_wrapper
    run_baseline
fi
