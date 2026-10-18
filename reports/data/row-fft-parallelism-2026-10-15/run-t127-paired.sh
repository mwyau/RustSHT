#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
data_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
baseline_binary="$repo_dir/target/rowfft-baseline-target/release/examples/m1b_bench"
candidate_binary="$repo_dir/target/rowfft-candidate-target/release/examples/m1b_bench"

exec >"$data_dir/t127-paired.log" 2>&1
exec 9>"$repo_dir/target/rustsht-benchmark.lock"
if ! flock -n 9; then
    printf 'benchmark lock is busy\n'
    exit 2
fi

printf 'started_utc='
date -u '+%Y-%m-%d %H:%M:%S UTC'
printf 'host_before='
uptime
ps -eo pid,etime,%cpu,comm,args --sort=-%cpu | head -10
printf '6 pinned CPU cores; rustc='
rustc --version

run_case() {
    local variant="$1"
    local binary="$2"
    local spin="$3"
    local lmax=127
    local ntheta=128
    local nphi=257
    printf '\nvariant=%s spin=%s T127 q6 worker-sized CPU affinity\n' "$variant" "$spin"
    /usr/bin/time -f 'elapsed_s=%e max_rss_kib=%M' \
        taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} "$binary" "$spin" "$lmax" 6 analysis GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only
}

for spin in 0 1; do
    for pair in 1 2 3 4 5; do
        printf '\npair=%s spin=%s T127 q6\n' "$pair" "$spin"
        if ((pair % 2 == 1)); then
            run_case baseline "$baseline_binary" "$spin"
            run_case candidate "$candidate_binary" "$spin"
        else
            run_case candidate "$candidate_binary" "$spin"
            run_case baseline "$baseline_binary" "$spin"
        fi
    done
done

printf '\nhost_after='
uptime
