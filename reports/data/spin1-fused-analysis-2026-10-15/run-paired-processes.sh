#!/usr/bin/env bash
set -euo pipefail

data_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="$(cd "$data_dir/../../.." && pwd)"
baseline="$repo_dir/target/release/examples/m1b_bench-before-spin1-fusion"
candidate="$repo_dir/target/release/examples/m1b_bench-after-spin1-fusion"
log="$data_dir/paired-process-logs.txt"

exec >"$log" 2>&1
printf 'host_before='
uptime

run_case() {
    local variant="$1"
    local binary="$2"
    local lmax="$3"
    local threads="$4"
    local cpus="$5"
    local ntheta=$((lmax + 1))
    local nphi=$((2 * lmax + 3))

    printf '\nvariant=%s T%s q%s cpus=%s\n' "$variant" "$lmax" "$threads" "$cpus"
    /usr/bin/time -f 'elapsed_s=%e max_rss_kib=%M' \
        taskset -c "$cpus" "$binary" 1 "$lmax" "$threads" analysis GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only
}

for lmax in 511 1023; do
    for threads in 1 8 16; do
        case "$threads" in
            1) cpus=0 ;;
            8) cpus=0-7 ;;
            16) cpus=0-15 ;;
        esac
        for pair in 1 2 3 4 5; do
            printf '\npair=%s T%s q%s\n' "$pair" "$lmax" "$threads"
            if ((pair % 2 == 1)); then
                run_case baseline "$baseline" "$lmax" "$threads" "$cpus"
                run_case candidate "$candidate" "$lmax" "$threads" "$cpus"
            else
                run_case candidate "$candidate" "$lmax" "$threads" "$cpus"
                run_case baseline "$baseline" "$lmax" "$threads" "$cpus"
            fi
        done
    done
done

printf '\nhost_after='
uptime
