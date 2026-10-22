# Plan 17 — f32 spin normalizer SIMD precheck

This report records a matched full-transform comparison of the production
scalar lane normalizer against a Pulp SIMD range precheck. It is one step in
Plan 17 Milestone B, not a DUCC parity claim.

## Implementation and source

The spin recurrence calls `normalize_pair_lanes` twice per degree. The
existing normalizer walks each latitude lane and leaves both recurrence values
and its exponent unchanged when the pair is zero or its largest magnitude is
within the inclusive `[SMALL, BIG]` range. The new precheck tests that same
condition using sign-stripped IEEE-754 magnitude bits in packed u32 lanes. If
every lane is already in the no-op range, it skips the scalar walk and its
stores. Any subnormal, too-small nonzero value, oversized value, or nonfinite
bit pattern declines the fast check and calls the original normalizer. The
scaling thresholds, exponent arithmetic and exceptional-value fallback are
unchanged.

The benchmark baseline is RustSHT `4c84b4f049c6ec35355d87d7588ccc5ec227ab13`;
the production implementation and tests are `9dea41434fe9c222ec4cc815629a48499dab91df`.
The DUCC reference is 0.42.0 at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

On the Ryzen 9 5950X, the Rust 1.99.1 release binary reported Pulp V3 with
AVX2/FMA and eight f32 lanes. The production no-cfg binary was rebuilt and
smoke-compared after the change; it selected the same V3 path and returned
bitwise-identical Rust outputs. In the production V3 `ForEachSpin32`
specialization, the precheck is inlined: no separate precheck symbol remains.
The optimized loop uses packed YMM `vpand`, `vpmaxud`, `vpcmpeqd`, `vpminud`,
`vpor`, and `vtestps` operations, with scalar handling for the tail. The
original scalar normalizer remains a separate fallback.

An initial out-of-line version of the Pulp precheck was rejected. Its V3
function had a 512-byte stack frame and out-of-line AVX wrapper calls, and the
T255 q1 analysis screen took about 2.6× the baseline. Adding
`#[inline(always)]` removed that call boundary; the measurements below use
the inlined implementation. This rejected variant is not included in the
candidate timing distribution.

## Matched benchmark

The native drivers used the same deterministic f32 spin-1 GL fixtures,
`phi0=0.23`, full `mmax`, `ntheta=lmax+1`, and even smooth `nphi`. Plans and
workspaces were reused; output reset and repeat checks ran inside each worker.
The measured operation includes the complete transform. Setup, first call,
warmup and steady-state calls are recorded separately. q1 used physical CPU 0;
q8 used physical CPUs 0–7, all within L3 domain 0. DUCC SHT code used GCC 15.2
with `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; its validation
driver was compiled separately with strict finite-value checks. Rust used
`rustc 1.99.1` / LLVM 23.1.1, release defaults and runtime Pulp dispatch.

Each T255 case has 30 calibrated batches per implementation (three
interleaved process pairs, ten batches per process). Each T720 whole transform
exceeds 100 ms and has 15 individually timed calls per implementation (three
process pairs, five calls per process). Timings are milliseconds; p10/p90 are
computed from the underlying samples. The paired candidate/baseline ratio is
the median of the three per-process-pair median ratios.

| T | Operation | Threads | Baseline Rust ms (p10–p90) | Candidate Rust ms (p10–p90) | Rust saved ms | Paired candidate / baseline | DUCC ms (p10–p90) | Candidate / DUCC | Candidate vs DUCC relative L2 |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 255 | Analysis | 1 | 67.711 (67.547–68.094) | 58.249 (58.099–58.510) | 9.462 | 0.860 | 2.757 (2.744–2.776) | 21.22× | 6.54e-6 |
| 255 | Analysis | 8 | 8.953 (8.932–9.001) | 7.724 (7.689–7.747) | 1.229 | 0.862 | 0.453 (0.452–0.457) | 17.12× | 6.54e-6 |
| 255 | Synthesis | 1 | 84.358 (82.997–85.744) | 74.637 (74.381–75.135) | 9.721 | 0.889 | 4.076 (4.051–4.094) | 18.39× | 3.71e-5 |
| 255 | Synthesis | 8 | 15.831 (15.422–16.459) | 14.855 (14.649–15.159) | 0.975 | 0.944 | 0.576 (0.573–0.580) | 26.18× | 3.71e-5 |
| 720 | Analysis | 1 | 1233.912 (1229.826–1239.473) | 1026.729 (1024.125–1032.852) | 207.183 | 0.831 | 28.436 (28.342–28.666) | 36.06× | 1.57e-5 |
| 720 | Analysis | 8 | 163.000 (162.704–163.444) | 134.612 (134.475–134.761) | 28.388 | 0.826 | 4.075 (4.048–4.099) | 33.19× | 1.57e-5 |
| 720 | Synthesis | 1 | 1653.570 (1645.746–1717.681) | 1493.379 (1457.636–1499.109) | 160.191 | 0.895 | 40.247 (39.848–41.212) | 37.20× | 1.43e-4 |
| 720 | Synthesis | 8 | 449.976 (447.471–464.789) | 414.745 (408.824–423.602) | 35.232 | 0.922 | 5.592 (5.562–5.631) | 73.84× | 1.43e-4 |

The candidate reduced warmed Rust latency in all eight cases. Improvements
range from 5.6% (T255 q8 synthesis) to 17.4% (T720 q8 analysis); absolute
median savings range from 0.976 ms to 207.183 ms. Rust baseline and candidate
outputs were bitwise identical in every process pair. Candidate/DUCC relative
L2 errors range from `6.54e-6` to `1.43e-4`; these are the existing f32
cross-library differences, not a change introduced by the candidate. The
remaining candidate/DUCC ratios are 17.1–73.8×, so recurrence/dataflow work
remains the dominant Plan 17 gap.

First-call and setup measurements are preserved per process in the JSON;
the candidate did not add plan-time setup. For example, T720 q1 analysis
first-call medians changed from 1231.959 to 1024.063 ms while plan setup
remained 3.277/3.471 ms. T720 q1 synthesis had a wider process spread; each
paired process median still favored the candidate. The post-run host check
showed no full CPU stall; small amounts of scheduler pressure had accumulated
over the longer averaging window.

## Correctness and checks

The focused precheck test passed on `Arch::Scalar` and runtime dispatch. It
checks positive/negative zero, inclusive `SMALL`/`BIG` boundaries, values just
outside the range, the smallest f32 subnormal, `f32::MAX`, a SIMD tail, and
that NaN/Inf are never accepted as safe. For unsafe finite lanes it compares
the resulting values bit-for-bit and exponents against the original scalar
normalizer.

- `cargo fmt --all --check`: passed.
- Strict Clippy, parallel and no-default-features: passed (`-D warnings`, with
  the repository's existing undeclared `rustsht_disable_paired_odd_fft` cfg
  warning explicitly allowed).
- Release parallel suite, serialized test execution: 270 passed, 36 ignored;
  integration suites added 16 passes and one ignored test. The ordinary
  concurrent test runner twice hit the unrelated scheduler assertion in
  `scalar_ordered_parallel_sparse_tails_reuse_odd_and_even_latitude_workspaces`;
  that test passed in isolation and in the serialized full suite.
- Release no-default-features suite: 220 passed, 17 ignored; integration
  suites added 16 passes and one ignored test.

## T1023 q8 high-bandlimit control

After publishing the T255/T720 result, the same baseline/candidate pair was
checked at full-order GL T1023 (`1024 × 2048`, spin 1, `phi0=0.23`) for q8
analysis and synthesis. This was a bounded control, not a new matrix: three
fresh interleaved process pairs, five individually timed calls per Rust
process, and ten calibrated ~25 ms DUCC batches per process. Both Rust builds
executed Pulp V3 with eight f32 lanes on CPUs 0–7 in one L3 domain. The baseline
was `4c84b4f049c6ec35355d87d7588ccc5ec227ab13`; the candidate was production
commit `9dea41434fe9c222ec4cc815629a48499dab91df`; DUCC remained pinned to
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

| Operation | Baseline Rust ms, p10–p90 | Candidate Rust ms, p10–p90 (DUCC pairing) | Paired candidate / baseline | DUCC ms, p10–p90 | Candidate / DUCC | Absolute excess over DUCC | Relative L2 vs DUCC |
|---|---:|---:|---:|---:|---:|---:|---:|
| Analysis q8 | 457.918 (454.919–465.068) | 375.485 (372.677–377.833) | 0.822 | 9.611 (9.466–10.476) | 39.18× | 365.9 ms | 2.35e-5 |
| Synthesis q8 | 753.555 (727.167–1535.428) | 674.696 (667.211–702.114) | 0.889 | 14.056 (13.940–14.624) | 47.95× | 660.6 ms | 2.01e-4 |

Rust baseline and candidate outputs were bitwise identical. The precheck
therefore retains its T255/T720 gain at T1023: 17.8% for analysis and 11.1%
for synthesis by paired process medians. The baseline synthesis distribution
contains two calls above 2 seconds in one process pair, followed by three
calls near 0.75 seconds. CPU pressure rose during the roughly two-minute
screen, so that baseline spread is host-contended; the separate candidate vs
DUCC comparisons were much tighter. Treat the high-bandlimit speedup as
directional, while the large remaining DUCC gap is clear. First-call and
setup measurements are in the JSON and cold/warm CSV.

The full-transform comparison confirms that optimizing the normalizer is no
longer enough: the q8 gap is about 38–39× for analysis and 48× for synthesis.
Next, attribute recurrence arithmetic, visitor/accumulation, and scheduling
costs on the executed f32 spin path before changing its degree-loop dataflow.
The original q1/q8 T255/T720 comparisons remain the primary measured
precheck result.

The [T1023 JSON](data/plan17-f32-spin-normalizer-simd-precheck-2026-10-22/t1023-q8/results.json),
[timed samples](data/plan17-f32-spin-normalizer-simd-precheck-2026-10-22/t1023-q8/samples.csv),
and [first/warm/setup timings](data/plan17-f32-spin-normalizer-simd-precheck-2026-10-22/t1023-q8/cold-warm.csv)
preserve the matched runs and their binary hashes.

## Reproduction and data

Build the pinned baseline at commit `4c84b4f` and the candidate at `9dea4143`
with the same Rust 1.99.1 toolchain and release profile. Build the DUCC worker
from the pinned source and compiler flags above, then run:

```sh
python3 scripts/benchmark_plan17_spin_recurrence_inline.py \
  --baseline-rust build/baseline/ducc_f32_full_transform_parity \
  --candidate-rust target/release/examples/ducc_f32_full_transform_parity \
  --ducc build/ducc-f32-plan17/ducc_f32_full_transform_parity \
  --outdir benchmark-output/plan17-normalizer-precheck \
  --lmax 255 720 --threads 1 8 --operations analysis synthesis \
  --process-pairs 3 --extra-t255-q8-synthesis-pairs 0 --target-ms 25 \
  --q1-cpu 0 --q8-cpus 0,1,2,3,4,5,6,7 \
  --baseline-commit 4c84b4f049c6ec35355d87d7588ccc5ec227ab13 \
  --candidate-description 'inlined Pulp SIMD range precheck before existing scalar normalizer'
```

The [JSON](data/plan17-f32-spin-normalizer-simd-precheck-2026-10-22/results.json)
contains source/toolchain/binary hashes, full per-case timing arrays, first and
warmup timings and output errors. The [summary CSV](data/plan17-f32-spin-normalizer-simd-precheck-2026-10-22/summary.csv)
reports medians and p10/p90; the [sample CSV](data/plan17-f32-spin-normalizer-simd-precheck-2026-10-22/samples.csv)
preserves each timed batch/call.

The T1023 control uses the same command and options with these substitutions:

```sh
python3 scripts/benchmark_plan17_spin_recurrence_inline.py \
  --baseline-rust build/t1023-baseline/release/examples/ducc_f32_full_transform_parity \
  --candidate-rust build/t1023-candidate/release/examples/ducc_f32_full_transform_parity \
  --ducc build/ducc-f32-plan17/ducc_f32_full_transform_parity \
  --outdir benchmark-output/plan17-normalizer-precheck-t1023-q8 \
  --lmax 1023 --threads 8 --operations analysis synthesis \
  --process-pairs 3 --extra-t255-q8-synthesis-pairs 0 --target-ms 25 \
  --q8-cpus 0,1,2,3,4,5,6,7 \
  --baseline-commit 4c84b4f049c6ec35355d87d7588ccc5ec227ab13 \
  --candidate-description 'inlined Pulp SIMD range precheck before existing scalar normalizer'
```

The precheck is retained in production in commit `9dea4143`. The next Plan 17
work should separate recurrence arithmetic from its coefficient visitor and
accumulation costs, then address the largest measured component before
broadening this change into other spins or operations.
