#!/usr/bin/env bash
set -euo pipefail

repo_dir=/home/albert/RustSHT
report_dir="$repo_dir/reports/data/m6-c03/ordinary-controls"
baseline="$repo_dir/target/m6-c02-revised-bench"
candidate="$repo_dir/target/m6-c02-candidate-worktree/target/release/examples/m1b_bench"
preflight_log="$report_dir/preflight-processes.log"

preflight() {
    local run_id="$1"
    {
        printf '\n--- run=%s affinity=0 ---\n' "$run_id"
        date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
        uptime
        cat /proc/pressure/cpu
        ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 20
        printf 'matching compute processes:\n'
        ps -eo pid,pcpu,stat,comm,args | awk '$4 == "rustc" || $4 == "cargo" || $4 == "cc1" || $4 == "cc1plus" || $4 == "make" || $4 == "cmake" || $4 == "ninja" || $4 ~ /^m6-c02/ || $4 ~ /^time_SHT/ || $4 ~ /^fftw/ || $4 ~ /^fft/ || $4 ~ /^shtns/ || $4 ~ /^ducc/ {print}'
    } >> "$preflight_log"
    local blockers
    blockers=$(ps -eo pid,pcpu,comm --no-headers | awk -v self="$BASHPID" -v parent="$PPID" '$1 != self && $1 != parent && $3 !~ /^(ps|awk)$/ && ($3 == "rustc" || $3 == "cargo" || $3 == "cc1" || $3 == "cc1plus" || $3 == "make" || $3 == "cmake" || $3 == "ninja" || $3 ~ /^m6-c02/ || $3 ~ /^time_SHT/ || $3 ~ /^fftw/ || $3 ~ /^fft/ || $3 ~ /^shtns/ || $3 ~ /^ducc/ || $2 + 0 >= 8) {print}')
    if [[ -n "$blockers" ]]; then
        {
            printf 'PREFLIGHT_REFUSED run=%s\n' "$run_id"
            printf '%s\n' "$blockers"
        } | tee -a "$preflight_log" >&2
        return 75
    fi
}

run_one() {
    local cell="$1" pair="$2" flavor="$3" spin="$4" lmax="$5"
    local binary="$baseline"
    if [[ "$flavor" == candidate ]]; then binary="$candidate"; fi
    local ntheta=$((lmax + 1))
    local nphi=$((2 * (lmax + 1)))
    local stem="$cell-pair${pair}-${flavor}"
    if [[ -f "$report_dir/$stem.stdout" && -f "$report_dir/$stem.time" ]]; then
        printf 'SKIP_COMPLETE run=%s\n' "$stem" >> "$preflight_log"
        return 0
    fi
    if [[ -e "$report_dir/$stem.stdout" || -e "$report_dir/$stem.time" ]]; then
        printf 'INCOMPLETE_ARTIFACT run=%s\n' "$stem" >> "$preflight_log"
        return 2
    fi
    preflight "$stem"
    printf '%s\n' "command: taskset -c 0 /usr/bin/time -f 'elapsed_s=%e peak_rss_kib=%M' -o $report_dir/$stem.time $binary $spin $lmax 1 both GL STANDARD $ntheta $nphi $lmax requested-only" >> "$preflight_log"
    taskset -c 0 /usr/bin/time -f 'elapsed_s=%e peak_rss_kib=%M' -o "$report_dir/$stem.time" \
        "$binary" "$spin" "$lmax" 1 both GL STANDARD "$ntheta" "$nphi" "$lmax" requested-only \
        > "$report_dir/$stem.stdout" 2>&1
}

if [[ ! -f "$report_dir/binary-sha256.txt" ]]; then
    {
        printf 'baseline C02-only SHA256: '
        sha256sum "$baseline"
        printf 'candidate C02+C03 SHA256: '
        sha256sum "$candidate"
        printf 'order: odd pairs baseline then candidate; even pairs candidate then baseline\n'
        printf 'first_run_utc=%s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
    } > "$report_dir/binary-sha256.txt"
else
    printf 'resume_utc=%s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" >> "$report_dir/binary-sha256.txt"
fi

for spec in 'spin1 1 511' 'spin2 2 511'; do
    read -r label spin lmax <<< "$spec"
    cell="$label-t$lmax-q1"
    for pair in 1 2 3 4 5 6; do
        if (( pair % 2 == 1 )); then
            run_one "$cell" "$pair" baseline "$spin" "$lmax"
            run_one "$cell" "$pair" candidate "$spin" "$lmax"
        else
            run_one "$cell" "$pair" candidate "$spin" "$lmax"
            run_one "$cell" "$pair" baseline "$spin" "$lmax"
        fi
    done
done
printf 'completed_utc=%s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" >> "$report_dir/binary-sha256.txt"
