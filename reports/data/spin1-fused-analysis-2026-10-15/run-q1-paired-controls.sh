#!/usr/bin/env bash
set -euo pipefail

data_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="$(cd "$data_dir/../../.." && pwd)"
baseline="$repo_dir/target/release/examples/m1b_bench-before-spin1-fusion"
candidate="$repo_dir/target/release/examples/m1b_bench-after-spin1-fusion-q1"
log="$data_dir/paired-q1-controls.txt"

exec >"$log" 2>&1
printf 'host_before='
uptime

run_case() {
    local variant="$1"
    local binary="$2"
    local lmax="$3"
    local ntheta=$((lmax + 1))
    local nphi=$((2 * lmax + 3))

    printf '\nvariant=%s T%s q1 cpus=0\n' "$variant" "$lmax"
    /usr/bin/time -f 'elapsed_s=%e max_rss_kib=%M' \
        taskset -c 0 "$binary" 1 "$lmax" 1 analysis GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only
}

for lmax in 63 127 511 1023; do
    for pair in 1 2 3 4 5; do
        printf '\npair=%s T%s q1\n' "$pair" "$lmax"
        if ((pair % 2 == 1)); then
            run_case baseline "$baseline" "$lmax"
            run_case candidate "$candidate" "$lmax"
        else
            run_case candidate "$candidate" "$lmax"
            run_case baseline "$baseline" "$lmax"
        fi
    done
done

printf '\nhost_after='
uptime
