# Cumulative scalar-analysis candidate and ablation record

**Status:** in progress; the final combined candidate and ablation study remain
required. This owner-directed continuation uses the existing
`opt/sht-performance-parity` branch. It does not change the public API or the
production default.

## Sources and measurement baseline

- Current-source reference: RustSHT `ff0fe71676b9a3084ec17e0cd7c8ccbd3ddad6ae`.
- Original requested starting checkpoint: `ce710d3622e3ca37783e0666b8bb800426fd72e0`.
  It has **not** yet been remeasured in this continuation; do not treat the
  current-source reference as the required final A baseline.
- DUCC kernel reference: `80388904b8cfc32400ba84af8a6a45aabfe60f86`.
- Rust 1.99.1 / LLVM 23.1.1, Pulp runtime V3 (AVX2/FMA), strict Rust FP,
  `-C target-cpu=x86-64-v3`. DUCC kernel: GCC 15.2,
  `-O3 -DNDEBUG -ffast-math -ffp-contract=fast -march=x86-64-v3`; driver
  compiled separately with `-fno-fast-math`.
- Host: Ryzen 9 5950X; direct q1 kernels pinned to one physical core. The
  kernel harness used 10 interleaved process pairs and 6 calibrated 20–100 ms batches per
  process and backend. T720 full-transform runs used three interleaved
  process pairs and 30 calibrated batches per backend and case.

The current production source already has the scalar-analysis 64/128/256
pair-tile selector (default 256) and fixed-capacity SIMD workspace arrays.
Those changes predate this continuation. Final evaluation must still compare
the cumulative implementation at all three tile sizes, and must include the
original `ce710` baseline.

## Step 3 — packed complex reduction candidate

The candidate follows DUCC's `vhsum_cmplx_special` data layout in
`src/ducc0/sht/sht_inner_loop.h`: interleave real/imaginary lanes and reduce
complex pairs. The Rust path is compile-time gated by
`rustsht_packed_complex_pair_reduction`; the default build retains separate
real and imaginary reductions. The V3-only specialization has a portable Pulp
fallback for other vector widths.

### Reduction-only and direct-kernel results

Reduction-only measured two complex outputs per iteration: baseline Rust calls
two separate real/imaginary reductions; the candidate calls the packed Pulp
reducer twice. Three pinned process invocations collected 30 calibrated
batches per variant. Every batch lasted 25.9–46.8 ms.

| Rust reduction path | p10 / median / p90 (ns per two complex outputs) |
|---|---:|
| Separate real/imaginary reductions | 1.310 / 1.320 / 1.330 |
| Packed complex reduction | 1.275 / 1.280 / 1.286 |

The packed reducer is 3.0% faster in this isolated screen, a small change
that is consistent with the modest complete-kernel result. The initial
un-pinned screen was discarded: it ignored the returned checksum and some
timed batches fell below 20 ms, allowing optimizer elimination and producing
misleading latency. The final harness consumes results outside each timed
batch, makes each iteration's SIMD inputs opaque, pins one physical core,
and produced 60 20–100 ms batches across three process runs.

| Active lanes | Full kernel baseline → packed (µs; p10–p90) | Recurrence only baseline → packed (µs) | Accumulation + reduction baseline → packed (µs) | Kernel + commit Rust/DUCC ratio baseline → packed |
|---:|---:|---:|---:|---:|
| 64 | 4.725 (4.704–4.764) → 4.575 (4.529–4.703) | 2.654 → 2.677* | 2.074 → 1.899 | 1.164 → 1.120 |
| 128 | 8.767 (8.709–8.815) → 8.687 (8.630–8.898) | 5.157 → 5.198 | 3.596 → 3.496 | 1.090 → 1.071 |
| 256 | 17.753 (17.675–17.917) → 17.162 (17.025–18.077) | 10.190 → 10.338 | 7.559 → 6.853 | 1.068 → 1.059 |

`*` The 64-lane baseline recurrence-only process spread was 24.1%, so that
component is unstable. The packed code does not alter recurrence; small
component differences there are measurement variation. Kernel-plus-commit
ratios use per-run DUCC measurements; lane medians were Rust/DUCC 4.765/4.087
µs → 4.617/4.131 µs, 8.712/7.988 → 8.592/8.030 µs, and 16.835/15.766 →
16.777/15.830 µs, respectively.

### Whole T720 analysis

All rows use the same T720 GL fixtures, operation and source except for the
compile-time reducer. Each Rust/DUCC phase used three process pairs and 30
timed batches per backend. Latencies are median (p10–p90), in milliseconds.

| `mmax` | q | Rust baseline | Rust packed | DUCC baseline → packed run | Rust/DUCC ratio baseline → packed | Packed/baseline | Relative L2 |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 720 | 1 | 15.612 (15.448–15.965) | 15.672 (15.426–16.604) | 14.479 → 14.415 | 1.078 → 1.087 | 1.004 | `4.63e-14` |
| 720 | 8 | 2.256 (2.212–2.445) | 2.227 (2.198–2.261) | 2.132 → 2.124 | 1.058 → 1.048 | 0.987 | `4.63e-14` |
| 180 | 1 | 6.276 (6.226–6.581) | 6.223 (6.173–6.387) | 5.906 → 5.872 | 1.063 → 1.060 | 0.991 | `4.91e-14` |
| 180 | 8 | 0.932 (0.914–0.992) | 0.927 (0.912–0.989) | 0.938 → 0.924 | 0.994 → 1.003 | 0.995 | `4.91e-14` |

Whole-transform shifts are small and overlap the observed ranges; they do not
establish a production gain. The candidate remains in the cumulative build
only because the required final interaction/ablation evaluation is still
pending. It is not enabled by default.

### Generated code

The exact V3 `FusedScalarAnalysisPair<true, false>` optimized symbol changed
from 6,113 to 5,648 bytes (static instruction count 1,269 to 1,177). Static
counts changed as follows: packed `vfmadd*pd` 46→58, scalar `vfmadd*sd`
66→42, `vaddpd` 20→24, `vshufpd` 17→9, and `vunpckl/hpd` 0→8. Neither symbol
has YMM stack references. These are static whole-symbol counts, not dynamic
instruction frequencies. The candidate's change in packed/scalar FMA mix and
register allocation is a plausible explanation for the whole-kernel change;
the slower reduction-only result shows that the reduction instruction
sequence alone is not the mechanism.

Pinned DUCC's V3 `map2alm_kernel` uses four f64 latitude lanes, keeps its
recurrence and coefficient accumulators in YMM registers, and reduces after
the lane traversal. Its `vhsum_cmplx_special` uses `vhaddpd`,
`vpermute2f128`, and packed addition before coefficient updates. Both DUCC
and Rust symbols have no observed YMM stack spills. Keep this as code-shape
evidence, not a runtime attribution.

## Validation and reproducibility

- Candidate-enabled release library suite: 250 passed, 34 ignored.
- Candidate-enabled no-default-features release library suite: 202 passed,
  17 ignored.
- Strict all-target/all-feature Clippy with the candidate cfg: passed;
  formatting and `git diff --check`: passed.
- Coefficient max-scaled difference was `1.97e-16`; direct-kernel output
  relative L2 was at most `1.63e-13`, and max absolute error at most
  `2.56e-11`. T720 full-transform relative L2 was at most `4.91e-14`.
- Benchmark source/result logs and annotated disassembly remain in local
  scratch and are intentionally not committed. DUCC's static assembly was
  generated from the pinned checkout listed above.

Reproduce the direct kernel comparison with the pinned DUCC checkout:

```sh
RUSTSHT_KERNEL_BENCH_RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  python3 scripts/benchmark_scalar_analysis_kernel.py
RUSTSHT_KERNEL_BENCH_RUSTFLAGS='-C target-cpu=x86-64-v3 --cfg rustsht_packed_complex_pair_reduction -Aunexpected_cfgs' \
  python3 scripts/benchmark_scalar_analysis_kernel.py
```

Run the reduction-only 30-batch comparison three times, pinning the same
physical CPU and preserving each invocation's output:

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3 --cfg rustsht_packed_complex_pair_reduction -Aunexpected_cfgs' \
  taskset -c "${Q1_CPU:?set Q1_CPU to an allowed physical core}" \
  cargo test --release --lib benchmark_scalar_analysis_complex_reduction \
  -- --ignored --nocapture --test-threads=1
```

## Remaining experiment sequence

| Step | State |
|---|---|
| 0. Current-source baseline | Recorded at `ff0fe`; remeasure original `ce710` for final A/B. |
| 1. 64/128/256 pair tiles | Selector and 256 initial setting exist; compare all three in final cumulative evaluation. |
| 2. Fixed SIMD workspace layout | Existing source layout retained; include in final ablation. |
| 3. Packed complex reduction | Implemented and validated behind cfg; isolated reducer is 3% faster, whole-transform effect remains inconclusive; retain only for cumulative evaluation. |
| 4. Recurrence range-check handling | Next source investigation; preserve finite/range safety and checked fallback. |
| 5–10. Staging, initialization, prefix, transition, packing and conversion | Pending review against current source; skip anything already implemented. |
| Final combined evaluation | Pending: original `ce710` baseline, cumulative 64/128/256, DUCC, then one-change-at-a-time ablations. |

The scalar analysis objective remains active until this final evaluation and
the required function crosswalk are complete.
