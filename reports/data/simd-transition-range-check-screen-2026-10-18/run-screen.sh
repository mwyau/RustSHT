#!/bin/sh
set -eu

# Run once at ff7c91d4 for baseline, then after applying candidate.patch with
# git apply --unidiff-zero for the candidate.
cargo +1.99.0 run --release --example m1b_bench -- \
  0 2047 1 synthesis GL STANDARD 2048 4097 2047 requested-only
cargo +1.99.0 run --release --example m1b_bench -- \
  0 1279 8 synthesis GL STANDARD 1280 2561 1279 requested-only
