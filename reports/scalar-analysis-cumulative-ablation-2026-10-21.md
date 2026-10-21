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

## Step 4 — recurrence range-check handling

No new range-check implementation was needed: the current production path
already contains the checked-first proof cache from `aeb123d6`. It keeps the
initialization and input-product checks active, requires a checked fused
traversal to complete before setting a block's proof bit, and invalidates the
bits when the plan, runtime SIMD backend, latitude count, or insignificant
prefix mode changes. On a cached block the ordinary vector-state scan is
omitted; scalar tails and input-product safeguards remain checked. A declined
block is not cached, and output remains in `fast_scalar_first` until the
transactional block has succeeded.

The earlier same-source cache A/B measured T720 full-order analysis at
26.405 → 23.291 ms q1 (11.8% lower). Its q8 process-pair medians were
5.138–5.287 ms without the cache and 4.556–4.772 ms with it; the q8 spread
includes host outliers. The matched kernel experiment attributes 7.388 µs of
the checked 256-lane direct-kernel time to the post-group scan. These results
identify a real validation cost and show the current safe route already
removes repeated scans after proof. The packed-reduction candidate's focused
release regression test passed with the cache enabled, including reuse after
zero, subnormal, extreme-finite and adjoint inputs. Final combined timing will
still measure interaction with the reduction candidate and all three tiles.

## Step 5 — output staging and accumulation

The direct-output experiment already exists in published history at
`376992ce` and was removed at `3ad85539` after four matched T720 full/reduced
q1/q8 cases measured 1.44–1.71× the staged route. Its direct route required
an additional whole-latitude product-bound scan before the already checked
input-preparation pass, then performed read/modify/write accumulation into
the user coefficient span. The record does not isolate which cost dominated.
Do not restore that implementation unchanged. A new candidate fused the
global product bound into the existing input-preparation pass, then selected
direct output only for checked, cached recurrence blocks whose inputs passed
that whole-latitude bound. Unsafe inputs and unproven blocks still use the
transactional staged path. This removes the previously duplicated input scan,
but the direct-output layout remains an experimental compile-time candidate
(`rustsht_scalar_analysis_fused_direct_preflight`), never the default.

Matched full-transform comparison used Rust 1.99.1 / LLVM 23.1.1 and the
release Rust example with Pulp V3 AVX2/FMA and strict Rust FP. DUCC was pinned
to `80388904b8cfc32400ba84af8a6a45aabfe60f86` and GCC 15.2.0; the SHT kernel
used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, with strict driver
validation. Both Rust variants included Step 3's packed reduction; only the
candidate enabled fused-preflight direct output. Each row has three
interleaved Rust/DUCC process pairs and 30 calibrated batches per backend.
Timings are median (p10–p90), milliseconds; the source variants ran in
separate blocks, so process pairs are interleaved with DUCC, not with each
other.

| T720 case | Staged Rust | DUCC | Staged/DUCC | Direct Rust | DUCC | Direct/DUCC | Direct/staged | Relative L2 vs DUCC (staged/direct) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| full q1 | 15.424 (15.339–15.516) | 14.489 (14.404–14.610) | 1.065× | 40.210 (39.886–41.237) | 14.579 (14.449–14.817) | 2.758× | 2.607× | `4.63e-14` / `4.66e-14` |
| full q8 | 2.265 (2.227–2.416) | 2.164 (2.124–2.352) | 1.047× | 5.488 (5.428–5.802) | 2.147 (2.118–2.209) | 2.556× | 2.423× | `4.63e-14` / `4.66e-14` |
| `mmax=180` q1 | 6.158 (6.091–6.233) | 5.966 (5.882–6.134) | 1.032× | 16.547 (16.369–17.159) | 5.951 (5.870–6.623) | 2.781× | 2.687× | `4.91e-14` / `4.98e-14` |
| `mmax=180` q8 | 0.935 (0.911–1.039) | 0.980 (0.921–1.008) | 0.955× | 2.355 (2.284–2.558) | 0.953 (0.919–1.003) | 2.472× | 2.518× | `4.91e-14` / `4.98e-14` |

## Step 6 — sectoral initialization review

No new source change was justified. Rust's vectorized ordinary and scaled
sectoral initializers are already present from `cbb1ea6b`:
`InitializeNormalAnalysisSectoral` evaluates integer powers over four f64 V3
lanes and verifies the result range, while `InitializeScaledAnalysisSectoral`
reuses completed ordinary vectors and normalizes the remaining vectors and
scalar tail with lane-specific scale epochs. This corresponds to DUCC
`fill_m2a` followed by `init_lambda`/`mypow`. The current T720 scalar analysis
route is above `SIMD_SECTORAL_INIT_MIN_LMAX=511` and uses the vector route.
Below that gate, the established scalar scaled-power initializer remains
active; the T64–T256 policy checkpoint measured recurrence whole calls at or
near DUCC after its table and row-FFT dispatch changes, so there is no current
evidence that lowering this gate would improve whole-call latency.

Existing tests compare the high-order vector result with the scaled scalar
reference, cover V3 lane tails, and verify reuse of a completed ordinary
prefix when later vectors require scaling. DUCC chooses its quick versus
scaled power route from `powlimit` before evaluating each vector. Rust may
compute an ordinary power for the first vector that declines before
continuing through the scaled initializer, but this bounded extra work has
not been isolated as a material cost. Retain the current initialization and
safety checks; do not add another initializer candidate without new stage
evidence.

## Steps 7–10 — existing scaled execution, packing and factors

Source review found these requested pieces already implemented; no separate
candidate was added.

| DUCC routine/dataflow | RustSHT production counterpart | Remaining difference |
|---|---|---|
| `fill_a2m` active ring selection and parity spectra | `first_active_symmetric_pair` and `prepare_symmetric_analysis_pair_block` in `src/transform.rs` | Rust binary-searches the first active reflected pair, then packs mixed polar blocks into caller-owned SoA scratch; DUCC fills fixed vector groups by scanning `ringdata`. Both preserve the equatorial singleton and use north/south even/odd sums. |
| `init_lambda` / `mypow` | `initialize_normal_analysis_sectoral_lanes`, `InitializeNormalAnalysisSectoral`, and `InitializeScaledAnalysisSectoral` | Rust keeps its scalar scaled initializer below the measured 511-degree vectorization gate and adds explicit checks/tail handling. |
| `iter_to_ieee` / `rescale` | `prepare_analysis_scale_corrections`, `advance_and_rescale_analysis_pair`, and `rescale_scaled_analysis_pair` | The ordinary SIMD vectors carry the below-limit frontier through the two-pair traversal. Rust also checks exponent bounds, finite values and representable corrections before promotion/fallback. |
| `calc_map2alm` mixed route and transition | `FusedScaledScalarAnalysisPair` and `promote_analysis_pair_to_ordinary` | Same scaled-to-ordinary structure; Rust validates the complete block before entering the ordinary kernel and retains exponent-aware fallback. |
| `map2alm_kernel` | `FusedScalarAnalysisPair` | Both process two recurrence pairs in a lane traversal and accumulate the even/odd complex spectra; Rust retains its checked state contract and optional packed reduction. |
| `Ylmgen::prepare` epsilon, alpha and recurrence coefficients | `T::fill_epsilon`, `T::fill_paired_scalar_factors` / `fill_factors`, then `finalize_paired_scalar_analysis_coefficients` | Both prepare the per-order factors once before the ring-block loop and convert accumulated coefficients once after all blocks. Rust stores these arrays in reusable worker workspace. |

DUCC's scaled loop applies its threshold to the second state after each
two-pair recurrence group. Rust's `advance_and_rescale_analysis_pair` does the
same grouping and carries the epoch frontier instead of rescanning it. The
extra Rust checks are tied to the existing extreme/subnormal safety contract;
removing them would require a distinct proof and measured benefit. Active
packing tests cover reflected polar masks and the equatorial singleton, while
the scaled-state tests cover SIMD tails and mixed epochs. The 256-pair tile
remains the production default; no tile-policy change follows from this code
review.

Reproduce the two matched case sets with separate result directories and
target directories so each binary keeps its own cfg set:

```sh
STAGED_FLAGS='-C target-cpu=x86-64-v3 --cfg rustsht_packed_complex_pair_reduction -Aunexpected_cfgs'
DIRECT_FLAGS="$STAGED_FLAGS --cfg rustsht_scalar_analysis_fused_direct_preflight"
CARGO_TARGET_DIR="${STAGED_TARGET:?}" RUSTSHT_FULL_TRANSFORM_RUSTFLAGS="$STAGED_FLAGS" \
  python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src "${DUCC_SRC:?}" \
  --ducc-revision 80388904b8cfc32400ba84af8a6a45aabfe60f86 \
  --lmax 720 --full-mmax-only --longitude-mode even_smooth --operation analysis \
  --threads 1 --threads 8 --results-dir "${STAGED_FULL_RESULTS:?}" --no-report
CARGO_TARGET_DIR="${STAGED_TARGET:?}" RUSTSHT_FULL_TRANSFORM_RUSTFLAGS="$STAGED_FLAGS" \
  python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src "${DUCC_SRC:?}" \
  --ducc-revision 80388904b8cfc32400ba84af8a6a45aabfe60f86 \
  --lmax 720 --mmax 180 --longitude-mode even_smooth --operation analysis \
  --threads 1 --threads 8 --results-dir "${STAGED_REDUCED_RESULTS:?}" --no-report
CARGO_TARGET_DIR="${DIRECT_TARGET:?}" RUSTSHT_FULL_TRANSFORM_RUSTFLAGS="$DIRECT_FLAGS" \
  python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src "${DUCC_SRC:?}" \
  --ducc-revision 80388904b8cfc32400ba84af8a6a45aabfe60f86 \
  --lmax 720 --full-mmax-only --longitude-mode even_smooth --operation analysis \
  --threads 1 --threads 8 --results-dir "${DIRECT_FULL_RESULTS:?}" --no-report
CARGO_TARGET_DIR="${DIRECT_TARGET:?}" RUSTSHT_FULL_TRANSFORM_RUSTFLAGS="$DIRECT_FLAGS" \
  python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src "${DUCC_SRC:?}" \
  --ducc-revision 80388904b8cfc32400ba84af8a6a45aabfe60f86 \
  --lmax 720 --mmax 180 --longitude-mode even_smooth --operation analysis \
  --threads 1 --threads 8 --results-dir "${DIRECT_REDUCED_RESULTS:?}" --no-report
```

The candidate is 2.42–2.69× slower than staged Rust across these four
workloads despite removal of the separate bound scan. This is a clear
whole-transform regression with tight Rust timing distributions and similar
frequency/runqueue telemetry between builds; the source does not yet isolate
the cause beyond the direct per-pair read/modify/write output flow. Keep it
only for the required cumulative interaction/ablation evaluation. The
candidate's first call uses staging to establish the range proof, while warm
calls use direct output, so the native harness correctly records
`reset_consistent=false` when comparing cold and warm outputs. Warm candidate
repeats are bitwise stable. The staged-versus-direct coefficient difference
was `2.5974e-16` relative L2 (`2.5324e-17` maximum absolute); DUCC comparisons
also passed their existing numerical limits. Release exceptional-input and
adjoint tests exercise the staged fallback. Do not promote this route.

## Validation and reproducibility

- Release library suite with packed-reduction and fused-preflight direct cfgs:
  250 passed, 34 ignored. No-default-features release library suite: 202
  passed, 17 ignored.
- Strict all-target/all-feature Clippy with both candidate cfgs: passed;
  formatting and `git diff --check`: passed.
- Coefficient max-scaled difference was `1.97e-16`; direct-kernel output
  relative L2 was at most `1.63e-13`, and max absolute error at most
  `2.56e-11`. T720 full-transform relative L2 was at most `4.91e-14`.
- Benchmark source/result logs and annotated disassembly remain in local
  scratch and are intentionally not committed. DUCC's static assembly was
  generated from the pinned checkout listed above.

Reproduce the direct kernel comparison with the pinned DUCC checkout:

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3 --cfg rustsht_packed_complex_pair_reduction --cfg rustsht_scalar_analysis_fused_direct_preflight -Aunexpected_cfgs' \
  cargo test --release --lib scalar_analysis_pair_range_cache_matches_checked_recurrence_and_is_plan_scoped \
  -- --nocapture --test-threads=1

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
| 4. Recurrence range-check handling | Complete in production since `aeb123d6`; checked-first cached proof skips only repeated ordinary vector scans. |
| 5. Output staging and accumulation | Fused-preflight direct-output candidate implemented and validated, but 2.42–2.69× slower; retain only for final cumulative ablation. |
| 6. Sectoral initialization | Reviewed: existing ordinary/scaled SIMD implementation already covers the production high-bandlimit route; no new change justified. |
| 7. Scaled prefix/rescaling | Reviewed: existing SIMD threshold/frontier handling matches DUCC's two-pair transition and preserves checked scaling. |
| 8. Scaled-to-ordinary transition | Reviewed: guarded correction/promotion and transactional fallback are already integrated. |
| 9. Reflected active-ring packing | Reviewed: binary-searched first-active pair plus bounded SoA compaction is already integrated. |
| 10. Coefficient/factor preparation | Reviewed: epsilon and recurrence factors are prepared once per order and reused across ring blocks, matching DUCC `Ylmgen::prepare`. |
| Final combined evaluation | Pending: original `ce710` baseline, cumulative 64/128/256, DUCC, then one-change-at-a-time ablations. |

The scalar analysis objective remains active until this final evaluation and
the required function crosswalk are complete.
