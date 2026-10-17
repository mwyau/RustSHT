#!/usr/bin/env bash
set -euo pipefail

cd ${RUSTSHT_TMP_DIR:-${TMPDIR:-$(python3 -c 'import tempfile;print(tempfile.gettempdir())')}}/rustsht-m6-p02-panel
export CARGO_TARGET_DIR=${RUSTSHT_TMP_DIR:-${TMPDIR:-$(python3 -c 'import tempfile;print(tempfile.gettempdir())')}}/rustsht-p02-panel-target
export TMPDIR=${RUSTSHT_TMP_DIR:-${TMPDIR:-$(python3 -c 'import tempfile;print(tempfile.gettempdir())')}}/rustsht-p02-panel-tmp

cargo fmt --all -- --check
git diff --check
cargo test --release -p rustsht --features parallel --lib fused_paired_synthesis -- --nocapture
cargo test --release -p rustsht --features parallel --lib synthesis_matches_ducc_0410 -- --nocapture
