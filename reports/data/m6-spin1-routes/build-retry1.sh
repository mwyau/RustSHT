#!/usr/bin/env bash
set -euo pipefail
repo_dir=$(git rev-parse --show-toplevel)
source_dir="$repo_dir/target/m6-spin1-routes/source"
target_dir="$repo_dir/target/m6-spin1-routes/cargo-target"
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" RUSTSHT_CPUSET} /usr/bin/time -v -o "$repo_dir/reports/data/m6-spin1-routes/build-retry1.time" \
    env CARGO_TARGET_DIR="$target_dir" \
    cargo +1.99.0 test --release --manifest-path "$source_dir/Cargo.toml" \
        -p rustsht --lib m6_spin1_synthesis_routes --no-run
