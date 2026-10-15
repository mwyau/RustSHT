#!/usr/bin/env bash
set -euo pipefail

data_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="$(cd "$data_dir/../../.." && pwd)"
baseline="$repo_dir/target/release/examples/m1b_bench-before-scheduler"
candidate="$repo_dir/target/release/examples/m1b_bench-after-capacity-split"
log="$data_dir/paired-process-logs.txt"

exec >"$log" 2>&1
printf 'host_before='
uptime

run_case() {
    local variant="$1"
    local binary="$2"
    local spin="$3"
    local lmax="$4"
    local threads="$5"
    local cpus="$6"
    local ntheta=$((lmax + 1))
    local nphi=$((2 * lmax + 3))

    printf '\nvariant=%s spin=%s T%s q%s cpus=%s\n' "$variant" "$spin" "$lmax" "$threads" "$cpus"
    /usr/bin/time -f 'elapsed_s=%e max_rss_kib=%M' \
        taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" "$binary" "$spin" "$lmax" "$threads" analysis GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only
}

for spin in 0 1; do
    for lmax in 511 1023; do
        for threads in 3 5; do
            if ((threads == 3)); then
                cpus=${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}
            else
                cpus=${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}
            fi
            for pair in 1 2 3 4 5; do
                printf '\npair=%s spin=%s T%s q%s\n' "$pair" "$spin" "$lmax" "$threads"
                if ((pair % 2 == 1)); then
                    run_case baseline "$baseline" "$spin" "$lmax" "$threads" "$cpus"
                    run_case candidate "$candidate" "$spin" "$lmax" "$threads" "$cpus"
                else
                    run_case candidate "$candidate" "$spin" "$lmax" "$threads" "$cpus"
                    run_case baseline "$baseline" "$spin" "$lmax" "$threads" "$cpus"
                fi
            done
        done
    done
done

printf '\nhost_after='
uptime
