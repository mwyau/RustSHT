#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
report_dir="$repo_dir/reports/data/m6-p01/shared-step-redesign"
runner="$report_dir/run-one.sh"
old_p01="$repo_dir/target/m6-p01-candidate-bench"
shared_step=${RUSTSHT_TMP_DIR:-${TMPDIR:-$(python3 -c 'import tempfile;print(tempfile.gettempdir())')}}/rustsht-p01-redesign-target/release/examples/m1b_bench

check_preflight() {
    local psi
    psi=$(awk '$1 == "some" { for (i = 2; i <= NF; i++) if ($i ~ /^avg10=/) { sub("avg10=", "", $i); print $i } }' /proc/pressure/cpu)
    awk -v psi="$psi" 'BEGIN { exit !(psi < 8.0) }' || {
        printf 'Refusing next process at CPU PSI some avg10=%s%%\n' "$psi" >&2
        exit 75
    }
}

for pair in 1 2 3 4 5 6; do
    if (( pair % 2 == 1 )); then
        check_preflight
        bash "$runner" "$old_p01" 511 1 "$pair" p01-default
        check_preflight
        bash "$runner" "$shared_step" 511 1 "$pair" p01-shared-step
    else
        check_preflight
        bash "$runner" "$shared_step" 511 1 "$pair" p01-shared-step
        check_preflight
        bash "$runner" "$old_p01" 511 1 "$pair" p01-default
    fi
done
