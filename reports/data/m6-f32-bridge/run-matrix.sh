#!/usr/bin/env bash
set -euo pipefail

repo_dir=${1:?worktree path required}
binary=${2:?benchmark binary required}
output_root=${3:?output root required}
script_dir=$(cd "$(dirname "$0")" && pwd)
status_file="$repo_dir/reports/data/m6-f32-bridge/raw/matrix-status.txt"

: > "$status_file"
for lmax in 127 511 1023; do
    for spin in 0 1; do
        for threads in 1 8; do
            for operation in synthesis analysis; do
                stem="t${lmax}-s${spin}-q${threads}-${operation}"
                if grep -q 'Exit status: 0' \
                    "$repo_dir/reports/data/m6-f32-bridge/raw/$stem.time" 2>/dev/null; then
                    printf 'T%s spin=%s q=%s %s exit=0 reused-existing\n' \
                        "$lmax" "$spin" "$threads" "$operation" \
                        >> "$status_file"
                    continue
                fi
                set +e
                bash "$script_dir/run-one.sh" \
                    "$repo_dir" "$binary" "$output_root" \
                    "$lmax" "$spin" "$threads" "$operation"
                status=$?
                set -e
                printf 'T%s spin=%s q=%s %s exit=%s\n' \
                    "$lmax" "$spin" "$threads" "$operation" "$status" \
                    >> "$status_file"
                if [[ "$status" -ne 0 && "$status" -ne 75 ]]; then
                    exit "$status"
                fi
            done
        done
    done
done
