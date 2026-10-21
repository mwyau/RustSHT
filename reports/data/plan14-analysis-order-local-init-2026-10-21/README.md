# T720 order-local coefficient initialization A/B

The process-level timing capture is retained locally. `summary.csv` contains
the committed sanitized results: per-backend medians and p10/p90, plus paired
ratios. The baseline source is
`1630176a64f4a63be38eb7ee8624732fd874bc1d`; the candidate source is in the
commit containing this dataset. DUCC is pinned at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

Build baseline and candidate binaries with Rust 1.99.1,
`RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'`, and
`cargo build --release --features parallel --example ducc_full_transform_parity`.
Build the DUCC benchmark binary from the pinned checkout using the existing
`reports/data/ducc-full-transform-parity-2026-10-20` CMake project. Its SHT
objects use `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the
benchmark driver remains strict FP.

Run from the repository root after setting the binary and source variables:

```sh
python3 scripts/benchmark_plan14_order_local_init.py \
  --baseline-binary "$BASELINE_BIN" \
  --candidate-binary "$CANDIDATE_BIN" \
  --ducc-binary "$DUCC_BIN" \
  --ducc-src "$DUCC_SRC" \
  --baseline-revision 1630176a64f4a63be38eb7ee8624732fd874bc1d \
  --candidate-revision "$CANDIDATE_REVISION" \
  --results-dir results/plan14-order-local-init
```

The runner uses deterministic full-triangle fixtures at T720, `mmax=720/180`,
`nphi=1458`, q1/q8; three interleaved process rounds; 30 calibrated 25 ms
batches per backend/case/thread; and the same physical CPU sets for each
backend. It validates outputs outside the timed region and records reset
consistency, numerical errors, setup/first-call timing, CPU frequency and
scheduler wait.
