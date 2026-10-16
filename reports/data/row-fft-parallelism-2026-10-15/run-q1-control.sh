#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
data_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
baseline_binary="$repo_dir/target/rowfft-baseline-target/release/examples/m1b_bench"
candidate_binary="$repo_dir/target/rowfft-candidate-target/release/examples/m1b_bench"

exec >"$data_dir/q1-control.log" 2>&1
exec 9>"$repo_dir/target/rustsht-benchmark.lock"
if ! flock -n 9; then
    printf 'benchmark lock is busy\n'
    exit 2
fi

printf 'started_utc='
date -u '+%Y-%m-%d %H:%M:%S UTC'
printf 'host_before='
uptime
printf 'top_processes_before:\n'
ps -eo pid,etime,%cpu,comm,args --sort=-%cpu | head -10
printf 'affinity=pinned; rustc='
rustc --version
printf 'cpu='
lscpu | sed -n 's/^Model name:[[:space:]]*//p' | head -1

run_case() {
    local variant="$1"
    local binary="$2"
    local spin="$3"
    local lmax="$4"
    local ntheta=$((lmax + 1))
    local nphi=$((2 * lmax + 3))
    printf '\nvariant=%s spin=%s T%s q1 cpu=${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}\n' "$variant" "$spin" "$lmax"
    /usr/bin/time -f 'elapsed_s=%e max_rss_kib=%M' \
        taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} "$binary" "$spin" "$lmax" 1 analysis GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only
}

for spin in 0 1; do
    for lmax in 511 1023; do
        for pair in 1 2 3 4 5; do
            printf '\npair=%s spin=%s T%s q1\n' "$pair" "$spin" "$lmax"
            if ((pair % 2 == 1)); then
                run_case baseline "$baseline_binary" "$spin" "$lmax"
                run_case candidate "$candidate_binary" "$spin" "$lmax"
            else
                run_case candidate "$candidate_binary" "$spin" "$lmax"
                run_case baseline "$baseline_binary" "$spin" "$lmax"
            fi
        done
    done
done

printf '\nhost_after='
uptime
printf 'top_processes_after:\n'
ps -eo pid,etime,%cpu,comm,args --sort=-%cpu | head -10
