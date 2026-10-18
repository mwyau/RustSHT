#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
data_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
baseline_binary=${RUSTSHT_TMP_DIR:-${TMPDIR:-$(python3 -c 'import tempfile;print(tempfile.gettempdir())')}}/rustsht-m1b-baseline-5ccd807
candidate_binary="$repo_dir/target/release/examples/m1b_bench"

for binary in "$baseline_binary" "$candidate_binary"; do
    if [[ ! -x "$binary" ]]; then
        printf 'benchmark binary is missing: %s\n' "$binary" >&2
        exit 1
    fi
done

exec >"$data_dir/paired-process-logs.txt" 2>&1
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
printf 'affinity=q1 one CPU core; q8 8 pinned CPU cores\n'
printf 'rustc='
rustc --version --verbose
printf 'cpu='
lscpu | sed -n 's/^Model name:[[:space:]]*//p' | head -1
printf 'baseline_source_commit=5ccd8072807b381dda3edbad4c62b2e48ae45a7a\n'
printf 'baseline_binary_sha256='
sha256sum "$baseline_binary"
printf 'candidate_source=uncommitted scalar analysis recurrence/accumulation fusion based on 5ccd807\n'
printf 'candidate_binary_sha256='
sha256sum "$candidate_binary"

run_case() {
    local variant="$1"
    local binary="$2"
    local lmax="$3"
    local threads="$4"
    local ntheta=$((lmax + 1))
    local nphi=$((2 * lmax + 3))
    local cpus=${RUSTSHT_CPUSET_Q1:?set RUSTSHT_CPUSET_Q1}
    if ((threads > 1)); then
        cpus=${RUSTSHT_CPUSET_Q8:?set RUSTSHT_CPUSET_Q8}
    fi
    printf '\nvariant=%s spin=0 T%s q%s\n' "$variant" "$lmax" "$threads"
    /usr/bin/time -f 'process_elapsed_s=%e peak_rss_kib=%M' \
        taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" "$binary" 0 "$lmax" "$threads" analysis GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only
}

for lmax in 511 1023; do
    for threads in 1 8; do
        for pair in 1 2 3 4 5; do
            printf '\npair=%s spin=0 T%s q%s\n' "$pair" "$lmax" "$threads"
            if ((pair % 2 == 1)); then
                run_case baseline "$baseline_binary" "$lmax" "$threads"
                run_case candidate "$candidate_binary" "$lmax" "$threads"
            else
                run_case candidate "$candidate_binary" "$lmax" "$threads"
                run_case baseline "$baseline_binary" "$lmax" "$threads"
            fi
        done
    done
done

printf '\nhost_after='
uptime
printf 'top_processes_after:\n'
ps -eo pid,etime,%cpu,comm,args --sort=-%cpu | head -10
