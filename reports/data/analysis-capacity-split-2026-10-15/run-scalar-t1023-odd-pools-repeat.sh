#!/usr/bin/env bash
set -euo pipefail

data_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="$(cd "$data_dir/../../.." && pwd)"
baseline="$repo_dir/target/release/examples/m1b_bench-before-scheduler"
candidate="$repo_dir/target/release/examples/m1b_bench-after-capacity-split"
log="$data_dir/paired-scalar-t1023-odd-pools-repeat.txt"

exec >"$log" 2>&1
printf 'host_before='
uptime

run_case() {
    local variant="$1"
    local binary="$2"
    local threads="$3"
    local cpus="$4"
    printf '\nvariant=%s spin=0 T1023 q%s\n' "$variant" "$threads"
    /usr/bin/time -f 'elapsed_s=%e max_rss_kib=%M' \
        taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" "$binary" 0 1023 "$threads" analysis GL STANDARD 1024 2049 1023 requested-only
}

for threads in 3 5; do
    if ((threads == 3)); then
        cpus=${RUSTSHT_CPUSET_Q3:?set RUSTSHT_CPUSET_Q3}
    else
        cpus=${RUSTSHT_CPUSET_Q5:?set RUSTSHT_CPUSET_Q5}
    fi
    for pair in 1 2 3 4 5; do
        printf '\npair=%s spin=0 T1023 q%s\n' "$pair" "$threads"
        if ((pair % 2 == 1)); then
            run_case baseline "$baseline" "$threads" "$cpus"
            run_case candidate "$candidate" "$threads" "$cpus"
        else
            run_case candidate "$candidate" "$threads" "$cpus"
            run_case baseline "$baseline" "$threads" "$cpus"
        fi
    done
done

printf '\nhost_after='
uptime
