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
    local ntheta=$((lmax + 1))
    local nphi=$((2 * lmax + 3))
    local stem="t${lmax}-q8-repeat-${pair}-${source}"
    taskset -c 0-7 /usr/bin/time -f 'peak_rss_kib=%M' \
        -o "$report_dir/${stem}.time" \
        "$binary" 1 "$lmax" 8 analysis GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only \
        > "$report_dir/${stem}.log"
}

{
    printf 'Five additional alternating q8 process pairs; f64 spin-1 GL STANDARD analysis at T511 and T1023.\n'
    printf 'Each process runs one warmup and five timed calls; taskset pins CPUs 0-7.\n'
    printf 'utc_start=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'loadavg_start=%s\n' "$(cat /proc/loadavg)"
    printf 'baseline_commit=541ac1b\n'
    printf 'baseline_binary_sha256=%s\n' "$(sha256sum "$baseline_binary" | cut -d' ' -f1)"
    printf 'candidate_binary_sha256=%s\n' "$(sha256sum "$candidate_binary" | cut -d' ' -f1)"
} > "$report_dir/q8-repeat.txt"

for lmax in 511 1023; do
    for pair in 6 7 8 9 10; do
        if (( pair % 2 == 1 )); then
            write_one "$baseline_binary" baseline "$pair" "$lmax"
            write_one "$candidate_binary" candidate "$pair" "$lmax"
        else
            write_one "$candidate_binary" candidate "$pair" "$lmax"
            write_one "$baseline_binary" baseline "$pair" "$lmax"
        fi
    done
done

{
    printf 'loadavg_end=%s\n' "$(cat /proc/loadavg)"
    printf 'utc_end=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
} >> "$report_dir/q8-repeat.txt"
