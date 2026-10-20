# Retained-vector reduction probe

This benchmark-only patch was based on RustSHT `60490446`. It added an
accumulation-only specialization that kept the eight SIMD accumulator chains,
passed them through `std::hint::black_box`, and omitted horizontal reductions.
The timed run also used pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`
with the existing calibrated kernel harness.

To reproduce the attempted run, apply `retained-vector-probe.patch` to
`60490446` with `git apply --unidiff-zero`, then run:

```sh
DUCC_SRC=/path/to/pinned/ducc DUCC_GIT=/path/to/ducc-checkout \
RUSTSHT_KERNEL_BENCH_RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
python3 scripts/benchmark_scalar_analysis_kernel.py
```

The candidate preserved recurrence states, but the generated code materialized
the eight SIMD accumulators as stack stores at each two-pair group. The timing
therefore includes extra traffic absent from the full kernel. The full-minus-
candidate deltas changed sign with lane count; they do not estimate reduction
cost and were not used to select a production implementation. See the
[cumulative results](../../scalar-analysis-cumulative-2026-10-20.md).
