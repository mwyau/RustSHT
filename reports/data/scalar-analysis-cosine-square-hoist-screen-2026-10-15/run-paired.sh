#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "usage: $0 BASELINE_M1B_BINARY CANDIDATE_M1B_BINARY" >&2
    exit 2
fi

baseline_binary="$1"
candidate_binary="$2"
report_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="$(cd "$report_dir/../../.." && pwd)"
exec 9>"$repo_dir/target/rustsht-benchmark.lock"
flock -x 9

write_one() {
    local binary="$1"
    local source="$2"
    local pair="$3"
    local lmax="$4"
    local threads="$5"
    local ntheta=$((lmax + 1))
    local nphi=$((2 * lmax + 3))
    local affinity=0
    if [[ "$threads" == "8" ]]; then
        affinity=0-7
    fi
    local stem="t${lmax}-q${threads}-${pair}-${source}"
    taskset -c "$affinity" /usr/bin/time -f 'peak_rss_kib=%M' \
        -o "$report_dir/${stem}.time" \
        "$binary" 0 "$lmax" "$threads" analysis GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only \
        > "$report_dir/${stem}.log"
}

{
    printf 'Five alternating release-process pairs; scalar f64 GL STANDARD analysis.\n'
    printf 'Each process runs one warmup and five timed calls.\n'
    printf 'utc_start=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'loadavg_start=%s\n' "$(cat /proc/loadavg)"
    printf 'affinity_allowed=%s\n' "$(taskset -pc $$)"
    printf 'baseline_commit=387ef37\n'
    printf 'baseline_binary_sha256=%s\n' "$(sha256sum "$baseline_binary" | cut -d' ' -f1)"
    printf 'candidate_binary_sha256=%s\n' "$(sha256sum "$candidate_binary" | cut -d' ' -f1)"
} > "$report_dir/paired-process-output.txt"

for lmax in 511 1023; do
    for threads in 1 8; do
        for pair in 1 2 3 4 5; do
            if (( pair % 2 == 1 )); then
                write_one "$baseline_binary" baseline "$pair" "$lmax" "$threads"
                write_one "$candidate_binary" candidate "$pair" "$lmax" "$threads"
            else
                write_one "$candidate_binary" candidate "$pair" "$lmax" "$threads"
                write_one "$baseline_binary" baseline "$pair" "$lmax" "$threads"
            fi
        done
    done
done

{
    printf 'loadavg_end=%s\n' "$(cat /proc/loadavg)"
    printf 'utc_end=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
} >> "$report_dir/paired-process-output.txt"
