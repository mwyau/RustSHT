#!/usr/bin/env bash
set -euo pipefail

data_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="$(cd "$data_dir/../../.." && pwd)"
baseline="$repo_dir/target/release/examples/m1b_bench-before-scheduler"
candidate="$repo_dir/target/release/examples/m1b_bench-after-capacity-split"
log="$data_dir/paired-spin1-t1023-q5-repeat.txt"

exec >"$log" 2>&1
printf 'host_before='
uptime

run_case() {
    local variant="$1"
    local binary="$2"
    printf '\nvariant=%s spin=1 T1023 q5 cpus=0-4\n' "$variant"
    /usr/bin/time -f 'elapsed_s=%e max_rss_kib=%M' \
        taskset -c 0-4 "$binary" 1 1023 5 analysis GL STANDARD 1024 2049 1023 requested-only
}

for pair in 1 2 3 4 5; do
    printf '\npair=%s spin=1 T1023 q5\n' "$pair"
    if ((pair % 2 == 1)); then
        run_case baseline "$baseline"
        run_case candidate "$candidate"
    else
        run_case candidate "$candidate"
        run_case baseline "$baseline"
    fi
done

printf '\nhost_after='
uptime
