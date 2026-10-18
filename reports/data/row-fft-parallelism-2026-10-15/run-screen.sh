#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
data_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
baseline_src="$repo_dir/target/rowfft-baseline-src"
baseline_target="$repo_dir/target/rowfft-baseline-target"
candidate_target="$repo_dir/target/rowfft-candidate-target"
baseline_binary="$baseline_target/release/examples/m1b_bench"
candidate_binary="$candidate_target/release/examples/m1b_bench"
temporary_directory="${RUSTSHT_TMP_DIR:-${TMPDIR:-$(python3 -c 'import tempfile;print(tempfile.gettempdir())')}}/rustsht-rowfft-tmp"

if [[ -e "$baseline_src" ]]; then
    printf 'baseline worktree path already exists: %s\n' "$baseline_src" >&2
    exit 1
fi

mkdir -p "$temporary_directory"
exec >"$data_dir/screen.log" 2>&1
exec 9>"$repo_dir/target/rustsht-benchmark.lock"
if ! flock -n 9; then
    printf 'benchmark lock is busy\n'
    exit 2
fi

cleanup() {
    git -C "$repo_dir" worktree remove --force "$baseline_src"
}
trap cleanup EXIT

printf 'started_utc='
date -u '+%Y-%m-%d %H:%M:%S UTC'
printf 'host_before='
uptime
printf 'top_processes_before:\n'
ps -eo pid,etime,%cpu,comm,args --sort=-%cpu | head -10
printf 'baseline_commit=4e8e8b4a55688f855815f6b0b8e9fec11b647674\n'
printf 'candidate_commit=uncommitted changes based on 4e8e8b4\n'
printf 'rustc='
rustc --version
printf 'cpu='
lscpu | sed -n 's/^Model name:[[:space:]]*//p' | head -1
printf 'affinity='
taskset -pc "$$"

git -C "$repo_dir" worktree add --detach "$baseline_src" 4e8e8b4a55688f855815f6b0b8e9fec11b647674
CARGO_TARGET_DIR="$baseline_target" TMPDIR="$temporary_directory" cargo build \
    --manifest-path "$baseline_src/Cargo.toml" \
    --package rustsht --release --example m1b_bench
CARGO_TARGET_DIR="$candidate_target" TMPDIR="$temporary_directory" cargo build \
    --manifest-path "$repo_dir/Cargo.toml" \
    --package rustsht --release --example m1b_bench

run_case() {
    local variant="$1"
    local binary="$2"
    local spin="$3"
    local lmax="$4"
    local ntheta=$((lmax + 1))
    local nphi=$((2 * lmax + 3))
    printf '\nvariant=%s spin=%s T%s q6 worker-sized CPU affinity\n' "$variant" "$spin" "$lmax"
    /usr/bin/time -f 'elapsed_s=%e max_rss_kib=%M' \
        taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} "$binary" "$spin" "$lmax" 6 analysis GL STANDARD \
        "$ntheta" "$nphi" "$lmax" requested-only
}

for spin in 0 1; do
    for lmax in 511 1023; do
        printf '\nscreen_pair spin=%s T%s q6\n' "$spin" "$lmax"
        run_case baseline "$baseline_binary" "$spin" "$lmax"
        run_case candidate "$candidate_binary" "$spin" "$lmax"
    done
done

printf '\nhost_after='
uptime
