#!/usr/bin/env bash
set -euo pipefail

data_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="$(cd "$data_dir/../../.." && pwd)"
baseline="$repo_dir/target/release/examples/m1b_bench-before-scheduler"
candidate="$repo_dir/target/release/examples/m1b_bench-after-exact-capacity-split"
log="$data_dir/paired-t511-q5-exact-repeat.txt"

exec >"$log" 2>&1
printf 'host_before='
uptime

run_case() {
    local variant="$1"
    local binary="$2"
    local spin="$3"
    printf '\nvariant=%s spin=%s T511 q5 cpus=0-4\n' "$variant" "$spin"
    /usr/bin/time -f 'elapsed_s=%e max_rss_kib=%M' \
        taskset -c 0-4 "$binary" "$spin" 511 5 analysis GL STANDARD 512 1025 511 requested-only
}

for spin in 0 1; do
    for pair in 1 2 3 4 5; do
        printf '\npair=%s spin=%s T511 q5\n' "$pair" "$spin"
        if ((pair % 2 == 1)); then
            run_case baseline "$baseline" "$spin"
            run_case candidate "$candidate" "$spin"
        else
            run_case candidate "$candidate" "$spin"
            run_case baseline "$baseline" "$spin"
        fi
    done
done

printf '\nhost_after='
uptime
