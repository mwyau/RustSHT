#!/usr/bin/env bash
set -euo pipefail
repo_dir=$(git rev-parse --show-toplevel)
report_dir="$repo_dir/reports/data/m6-c02/paired"
preflight_log="$report_dir/isolation-preflight.log"

preflight() {
    local run_id="$1"
    {
        printf '\n--- run=%s affinity=pinned ---\n' "$run_id"
        date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
        uptime
        ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 20
        printf 'matching compute processes:\n'
        ps -eo pid,pcpu,stat,comm,args | awk '$4 == "rustc" || $4 == "cargo" || $4 == "cc1" || $4 == "cc1plus" || $4 == "make" || $4 == "cmake" || $4 == "ninja" || $4 ~ /^m6-c02/ || $4 ~ /^time_SHT/ || $4 ~ /^fftw/ || $4 ~ /^fft/ || $4 ~ /^shtns/ || $4 ~ /^ducc/ {print}'
    } >> "$preflight_log"
    local blockers
    blockers=$(ps -eo pid,pcpu,comm --no-headers | awk -v self="$BASHPID" -v parent="$PPID" '$1 != self && $1 != parent && ($3 == "rustc" || $3 == "cargo" || $3 == "cc1" || $3 == "cc1plus" || $3 == "make" || $3 == "cmake" || $3 == "ninja" || $3 ~ /^m6-c02/ || $3 ~ /^time_SHT/ || $3 ~ /^fftw/ || $3 ~ /^fft/ || $3 ~ /^shtns/ || $3 ~ /^ducc/ || $2 + 0 >= 30) {print}')
    if [[ -n "$blockers" ]]; then
        {
            printf 'CONTENTION_FLAG run=%s\n' "$run_id"
            printf '%s\n' "$blockers"
        } | tee -a "$preflight_log" >&2
        return 75
    fi
}

run_one() {
    local cell="$1" triple="$2" flavor="$3" spin="$4" lmax="$5"
    local binary="$repo_dir/target/m6-c02-$flavor-bench"
    local ntheta=$((lmax + 1))
    local nphi=$((2 * (lmax + 1)))
    local stem="$cell-triple${triple}-${flavor}"
    preflight "$stem"
    printf '%s\n' "command: taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} /usr/bin/time -f 'elapsed_s=%e peak_rss_kib=%M' -o $report_dir/$stem.time $binary $spin $lmax 1 both GL STANDARD $ntheta $nphi $lmax requested-only" >> "$preflight_log"
    taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} /usr/bin/time -f 'elapsed_s=%e peak_rss_kib=%M' -o "$report_dir/$stem.time" \
        "$binary" "$spin" "$lmax" 1 both GL STANDARD "$ntheta" "$nphi" "$lmax" requested-only \
        > "$report_dir/$stem.stdout" 2>&1
}

for spec in 'scalar-t255-q1 0 255' 'spin1-t511-q1 1 511'; do
    read -r cell spin lmax <<< "$spec"
    run_one "$cell" 1 baseline "$spin" "$lmax"
    run_one "$cell" 1 candidate "$spin" "$lmax"
    run_one "$cell" 1 no-guard "$spin" "$lmax"
    run_one "$cell" 2 candidate "$spin" "$lmax"
    run_one "$cell" 2 no-guard "$spin" "$lmax"
    run_one "$cell" 2 baseline "$spin" "$lmax"
    run_one "$cell" 3 no-guard "$spin" "$lmax"
    run_one "$cell" 3 baseline "$spin" "$lmax"
    run_one "$cell" 3 candidate "$spin" "$lmax"
done
date -u '+completed_utc=%Y-%m-%dT%H:%M:%SZ' >> "$preflight_log"
