#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
lmax=${1:?lmax required}
spin=${2:?spin required}
pair=${3:?pair number required}
p01="$repo_dir/target/m6-p01-candidate-bench"
p02="$repo_dir/reports/data/m6-p02/precomputed-visitor/candidate-bench"
runner="$repo_dir/reports/data/m6-p02/precomputed-visitor/incremental/run-one.sh"

run_p01() { bash "$runner" "$p01" "$lmax" "$spin" "$pair" p01; }
run_p02() { bash "$runner" "$p02" "$lmax" "$spin" "$pair" p02; }

if (( pair % 2 == 1 )); then
    run_p01
    run_p02
else
    run_p02
    run_p01
fi
