#!/usr/bin/env bash
set -euo pipefail

data_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="$(cd "$data_dir/../../.." && pwd)"
baseline="$repo_dir/target/release/examples/m1b_bench-before-scheduler"
candidate="$repo_dir/target/release/examples/m1b_bench-after-exact-capacity-split"
log="$data_dir/paired-q6-exact-capacity-split.txt"

exec >"$log" 2>&1
printf 'host_before='
uptime

run_case() {
    local variant="$1"
    local binary="$2"
    local spin="$3"
    local lmax="$4"
    local ntheta=$((lmax + 1))
    local nphi=$((2 * lmax + 3))
    printf '\nvariant=%s spin=%s T%s q6 worker-sized CPU affinity\n' "$variant" "$spin" "$lmax"
    /usr/bin/time -f 'elapsed_s=%e max_rss_kib=%M' \
        taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} "$binary" "$spin" "$lmax" 6 analysis GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only
}

for spin in 0 1; do
    for lmax in 511 1023; do
        for pair in 1 2 3 4 5; do
            printf '\npair=%s spin=%s T%s q6\n' "$pair" "$spin" "$lmax"
            if ((pair % 2 == 1)); then
                run_case baseline "$baseline" "$spin" "$lmax"
                run_case candidate "$candidate" "$spin" "$lmax"
            else
                run_case candidate "$candidate" "$spin" "$lmax"
                run_case baseline "$baseline" "$spin" "$lmax"
            fi
        done
    done
done

printf '\nhost_after='
uptime
