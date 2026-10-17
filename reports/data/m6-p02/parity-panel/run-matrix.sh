#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
report_dir="$repo_dir/reports/data/m6-p02/parity-panel"
runner="$report_dir/run-one.sh"
baseline="$repo_dir/target/m6-p01-baseline-bench"
candidate=${RUSTSHT_TMP_DIR:-${TMPDIR:-$(python3 -c 'import tempfile;print(tempfile.gettempdir())')}}/rustsht-p02-panel-target/release/examples/m1b_bench

for workload in '1023 0' '511 0' '511 1'; do
    read -r lmax spin <<< "$workload"
    for pair in 1 2 3 4 5 6; do
        if (( pair % 2 == 1 )); then
            bash "$runner" "$baseline" "$lmax" "$spin" "$pair" baseline
            bash "$runner" "$candidate" "$lmax" "$spin" "$pair" candidate
        else
            bash "$runner" "$candidate" "$lmax" "$spin" "$pair" candidate
            bash "$runner" "$baseline" "$lmax" "$spin" "$pair" baseline
        fi
    done
done
