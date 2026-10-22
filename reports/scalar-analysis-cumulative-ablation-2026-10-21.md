# Cumulative scalar-analysis candidate and ablation record

**Status: COMPLETE (2026-10-21).** The final combined candidate and required
ablation study are complete. Production remains on the known-good staged
output path with the checked range cache and fixed 256-pair tile. This
owner-directed continuation uses the existing
`opt/sht-performance-parity` branch. It does not change the public API or the
production default.

## Sources and measurement baseline

- Current-source reference: RustSHT `ff0fe71676b9a3084ec17e0cd7c8ccbd3ddad6ae`.
- Original requested starting checkpoint: `ce710d3622e3ca37783e0666b8bb800426fd72e0`.
  Its T720 whole-transform analysis and 64/128/256-lane kernel baselines are
  now remeasured below. The source comparison is historical-to-current
  evidence, not attribution to an individual change.
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
establish a production gain. The candidate was retained only through the
final interaction/ablation evaluation below and is not enabled by default.

### Current-source remeasurement at `ef238290`

The previous Step 3 measurements were rechecked after the range-cache and
direct-preflight candidates were integrated. Default and packed builds used
the same current source and differed only by
`rustsht_packed_complex_pair_reduction`; full-transform and direct-kernel
summaries are in the [current-source A/B data](data/scalar-analysis-cumulative-ablation-2026-10-21/current-source-packed-ab/README.md).
The packed candidate adds no heap workspace or scratch arrays. Measured
median process RSS was within 0.4 MiB between builds; the largest observed
case difference was 1.7 MiB, within the process-to-process RSS range.

The isolated range-proof-cached fused kernel includes Rust's final staged
coefficient commit; DUCC uses the matching `map2alm_kernel` output work. These
latencies include the vector recurrence, reduction and coefficient write,
while the benchmark separately reports the checked variant and reset cost.

| Active lanes | Default Rust µs (p10–p90) | Packed Rust µs (p10–p90) | DUCC µs, default / packed build | Paired Rust/DUCC ratio, default → packed | Packed/default Rust |
|---:|---:|---:|---:|---:|---:|
| 64 | 4.876 (4.831–5.014) | 4.655 (4.620–4.861) | 4.107 / 4.109 | 1.186 → 1.134 | 0.955 |
| 128 | 8.934 (8.848–9.151) | 8.838 (8.801–8.903) | 8.010 / 8.152 | 1.116 → 1.084 | 0.977 |
| 256 | 17.888 (17.769–18.156) | 17.261 (17.186–17.365) | 15.798 / 15.790 | 1.130 → 1.092 | 0.965 |

The kernel medians improved 4.5%, 1.1% and 3.5% at 64, 128 and 256 lanes.
The direct route therefore narrows the per-order kernel excess, with the
largest absolute remaining excess at 256 lanes (1.449 µs, versus 2.059 µs
for default). The V3 specialization still uses four f64 lanes, and this
reducer change does not alter the recurrence or allocate additional memory.
The isolated kernel runs had low measured runqueue wait (median about 0.04%,
maximum below 0.1%) and 4.617 GHz median frequency; the 64-lane process spread
was wider than at 128/256 lanes.

| T720 GL analysis | q | Default Rust ms (p10–p90) | Packed Rust ms (p10–p90) | Rust packed/default | Rust/DUCC ratio, default → packed | Relative L2 |
|---|---:|---:|---:|---:|---:|---:|
| `mmax=1` | 1 | 0.09221 (0.09114–0.09316) | 0.09071 (0.09017–0.09200) | 0.984 | 0.684 → 0.659 | `6.12e-14` |
| `mmax=1` | 8 | 0.07756 (0.07581–0.08000) | 0.07676 (0.07475–0.07929) | 0.990 | 0.584 → 0.585 | `6.12e-14` |
| `mmax=180` | 1 | 6.268 (6.232–6.303) | 6.159 (6.105–6.284) | 0.983 | 1.062 → 1.041 | `4.91e-14` |
| `mmax=180` | 8 | 0.936 (0.922–0.971) | 0.927 (0.911–0.976) | 0.990 | 1.009 → 1.005 | `4.91e-14` |
| full `mmax=720` | 1 | 15.683 (15.482–16.033) | 15.474 (15.349–15.843) | 0.987 | 1.077 → 1.060 | `4.63e-14` |
| full `mmax=720` | 8 | 2.251 (2.228–2.379) | 2.236 (2.205–2.395) | 0.993 | 1.058 → 1.043 | `4.63e-14` |

All six whole-transform changes are below 1.7% and their p10–p90 intervals
overlap. q8 runqueue-wait p90 was 15.6–17.9%, so these results do not
establish an end-to-end speedup. First-call/setup timing and per-case RSS are
retained in the CSV. All three process pairs completed for every case with no
numerical or reset failures; maximum relative L2 was `6.12e-14`. Direct-kernel
coefficient error was `1.97e-16` scaled, with output relative L2 at most
`1.63e-13` and maximum absolute error `2.56e-11`.

This remeasurement supports keeping the packed reducer in the cumulative
candidate, not enabling it by default. It is a small isolated improvement,
and the whole-transform effect remains unresolved under q8 host variation.

### Original `ce710` whole-transform baseline

The requested original-source A baseline was remeasured with Rust 1.99.1,
strict x86-64-v3 Rust flags, the same native Rust/DUCC full-call harness, and
pinned DUCC 803. The `ce710` checkout required a temporary toolchain override
from its checked-in Rust 1.99.0 pin. The native worker and Python runner were
copied from the source-equivalent harness at `ff0fe716`; their hashes and executable hashes
are recorded in the [machine-readable run](data/scalar-analysis-cumulative-ablation-2026-10-21/ce710-full-transform/summary.json).
The old checkout had only test-module benchmark instrumentation in addition
to the `ce710` production sources; that instrumentation was not linked into
the native example. Three interleaved process pairs collected ten calibrated
20–100 ms batches per process and backend. q1 used CPU 0; q8 used CPUs 0–7 on
one L3 domain. All six T720 GL even-smooth analysis cases (`mmax` 1, 180 and
720; q1/q8) completed, with 18 numerical/identity checks and no failures.

The q1/q8 `mmax=1` case is retained as a useful low-work control. The table
below compares full and quarter-order configurations against the existing
`ff0fe` current-source measurements above; DUCC medians come from each
source's matched process pairs. The old-to-current source delta includes all
intervening implementation changes and does not isolate a single
optimization.

| T720 GL analysis | `ce710` Rust ms (p10–p90) | `ce710` DUCC ms | `ce710` ratio | `ff0fe` Rust ms | `ff0fe` DUCC ms | `ff0fe` ratio | Rust latency change | Relative L2 (`ce710` / `ff0fe`) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| full, q1 | 21.321 (21.217–21.458) | 14.416 | 1.479× | 15.612 | 14.479 | 1.078× | −26.8% | `4.63e-14` / `4.63e-14` |
| full, q8 | 3.721 (3.687–3.787) | 2.118 | 1.757× | 2.256 | 2.132 | 1.058× | −39.4% | `4.63e-14` / `4.63e-14` |
| `mmax=180`, q1 | 8.886 (8.862–9.121) | 5.920 | 1.501× | 6.276 | 5.906 | 1.063× | −29.4% | `4.91e-14` / `4.91e-14` |
| `mmax=180`, q8 | 1.461 (1.448–1.479) | 0.921 | 1.587× | 0.932 | 0.938 | 0.994× | −36.2% | `4.91e-14` / `4.91e-14` |

The original-source direct-kernel baseline also shows the accumulated source
improvement at the selected `m=1` order. These reset-subtracted kernel times
use the same 64/128/256 active-lane work and pinned DUCC kernel; compare them
with the `ff0fe` kernel baseline in Step 3, not with a whole-transform row.

| Active lanes | `ce710` Rust kernel µs (p10–p90) | DUCC kernel µs (p10–p90) | Ratio | `ff0fe` Rust kernel µs | `ce710` → `ff0fe` Rust change |
|---:|---:|---:|---:|---:|---:|
| 64 | 7.502 (7.464–7.662) | 4.087 (4.069–4.141) | 1.832× | 4.765 | −36.5% |
| 128 | 13.034 (12.973–13.451) | 8.029 (7.967–8.095) | 1.620× | 8.712 | −33.2% |
| 256 | 24.601 (24.533–24.743) | 15.751 (15.715–15.852) | 1.561× | 16.835 | −31.6% |

Paired ratios and sanitized distributions are in the [kernel summary](data/scalar-analysis-cumulative-ablation-2026-10-21/ce710-kernel/summary.json); raw batch samples remain local.

For the extra `mmax=1` control, `ce710` measured 0.130 ms Rust / 0.136 ms
DUCC at q1 and 0.099 / 0.137 ms at q8; relative L2 was `6.12e-14`. Median
runqueue wait across sampled workers was 0.64% (p90 17.0%), with occasional
involuntary-switch outliers. The distributions for the four full/reduced
cases support the aggregate source improvement; small changes between
neighboring builds remain host-sensitive. Setup and first-call observations,
fixture hashes and p10/median/p90 rows are retained in the sanitized summary.
Raw process batches remain local. Compact rows are in [`results.csv`](data/scalar-analysis-cumulative-ablation-2026-10-21/ce710-full-transform/results.csv),
and reproduction details are in the [data README](data/scalar-analysis-cumulative-ablation-2026-10-21/ce710-full-transform/README.md).

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
zero, subnormal, extreme-finite and adjoint inputs. The final combined timing
below measures interaction with the reduction candidate and all three tiles.

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
but the direct-output layout remained an experimental compile-time candidate
(`rustsht_scalar_analysis_fused_direct_preflight`), never the default. The
final cumulative run below rejects it for latency and reset behavior.

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

The candidate was 2.42–2.69× slower than staged Rust across these four
workloads despite removal of the separate bound scan. This is a clear
whole-transform regression; the source does not yet isolate the cause beyond
the direct per-pair read/modify/write output flow. It was retained only for
the required cumulative interaction/ablation evaluation. The
candidate's first call uses staging to establish the range proof, while warm
calls use direct output, so the native harness correctly records
`reset_consistent=false` when comparing cold and warm outputs. Warm candidate
repeats are bitwise stable. The staged-versus-direct coefficient difference
was `2.5974e-16` relative L2 (`2.5324e-17` maximum absolute); DUCC comparisons
also passed their existing numerical limits. Release exceptional-input and
adjoint tests exercise the staged fallback. Do not promote this route.

## Step 2 — fixed SIMD workspace field order

The cfg `rustsht_scalar_analysis_hot_scratch_order` moves the rarely used
`next` recurrence array after the four spectrum arrays in the fixed
`RecurrenceWorkspace`; it changes no array capacity, alignment, number of
allocations, or copying. The benchmark-only kernel fixture uses the same
ordering. The A/B otherwise keeps the packed complex reducer enabled in both
builds. Sanitized results and reproduction details are in the
[scratch-order summary](data/scalar-analysis-cumulative-ablation-2026-10-21/hot-scratch-order/README.md).

The matched q1 kernel comparison found no repeatable difference. For 64,
128, and 256 lanes, baseline/candidate medians were 4.615/4.621, 8.630/8.592,
and 16.813/16.824 microseconds; process-median ranges overlapped. DUCC was
4.279/4.262, 8.240/8.207, and 16.150/16.067 microseconds in the corresponding
source-build sets. The production V3 recurrence loop had identical instruction
bytes between builds, so this field reorder did not alter its generated hot
loop.

The full T720 q1/q8 repeats likewise provide no supported improvement. For
full order, Rust warm medians changed from 15.323 to 15.519 ms at q1 and 2.336
to 2.453 ms at q8; at `mmax=180` they changed from 6.160 to 6.140 ms and
0.919 to 0.956 ms. Their timing distributions overlap, and q8 runqueue-wait
p90 was about 15% in both repeats. The first process pair was excluded because
baseline q8 processes had 11–12% runqueue wait while candidate processes had
less than 1%. Keep the layout cfg only for the requested cumulative interaction
test; do not enable it in production based on these measurements.

Both repeat sets completed all six cases with no benchmark failures, stable
reset semantics, finite zero/subnormal/extreme outputs, and basis/roundtrip
checks. The maximum Rust/DUCC relative L2 across case comparisons was
`6.12e-14`. Release library tests passed with packed-reducer and layout cfgs
(250 passed, 34 ignored); no-default-features release tests passed (202 passed,
17 ignored), as did strict Clippy and formatting.

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

## Final cumulative evaluation and disposition

The final A/B used current source `2b8d31893d84ffcdc3dc8f76f69581323ae65aa6`
and pinned DUCC `80388904b8cfc32400ba84af8a6a45aabfe60f86`. Rust was 1.99.1
(LLVM 23.1.1), Pulp 0.22.3, strict FP and runtime V3; DUCC used GCC 15.2 with
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` and a strict benchmark
driver. T720 GL analysis used deterministic even-smooth fixtures at `mmax=1`
(low-work control), 180 and 720, q1/q8. Each build used three independent
process pairs and ten calibrated ~25 ms batches per process/backend. Rust and
DUCC were interleaved within each source build, but candidate builds ran in
separate blocks; small cross-build deltas are screening evidence, not tightly
paired attribution. q1 used CPU 0 and q8 CPUs 0–7 on one L3 domain. The
current-default host runqueue wait was 0.24% median / 15.5% p90; frequency
ranged 1.75–4.74 GHz (median 3.97 GHz), so marginal q8 shifts are
inconclusive.

The sanitized per-case table and build manifest are in
[`summary.csv`](data/scalar-analysis-cumulative-ablation-2026-10-21/final-combined/summary.csv)
and [`variants.csv`](data/scalar-analysis-cumulative-ablation-2026-10-21/final-combined/variants.csv).
Raw process JSON and host samples remain local. All variants used the same
fixtures, output buffers, transform boundary and numerical checks.

| Variant | `mmax=180` q1 / q8 Rust ms (Rust/DUCC) | full `mmax=720` q1 / q8 Rust ms (Rust/DUCC) | Conclusion |
|---|---:|---:|---|
| Current default | 6.331 (1.053×) / 0.925 (1.002×) | 15.634 (1.074×) / 2.279 (1.028×) | Known-good reference |
| Packed reducer only | 6.491 (1.037×) / 1.078 (1.050×) | 16.097 (1.083×) / 2.513 (1.130×) | Isolated kernel gain did not carry to this whole-call screen |
| Scratch layout only | 6.399 (1.069×) / 1.073 (1.065×) | 16.043 (1.081×) / 2.639 (1.068×) | No supported gain; generated loop bytes were identical |
| Packed + layout, tile 256 | 6.469 (1.033×) / 0.933 (0.991×) | 15.911 (1.058×) / 2.262 (1.057×) | Mixed, small changes; ranges overlap and no production gain established |
| Packed + layout, tile 128 | 6.643 (1.040×) / 1.089 (1.085×) | 16.160 (1.089×) / 2.706 (1.096×) | Slower than tile 256 |
| Packed + layout, tile 64 | 6.654 (1.104×) / 0.992 (1.050×) | 16.554 (1.115×) / 2.429 (1.122×) | Slower than tile 256 |
| Range cache disabled | 8.044 (1.338×) / 1.165 (1.254×) | 19.713 (1.280×) / 3.183 (1.270×) | Keep the checked-first range cache |
| Direct output only | 17.029 (2.686×) / 2.745 (2.557×) | 42.735 (2.732×) / 5.571 (2.595×) | Reject: 2.44–2.97× baseline latency and cold/warm reset mismatch |
| Packed + layout + direct, tile 256 | 17.104 (2.808×) / 2.643 (2.525×) | 41.560 (2.804×) / 6.419 (2.580×) | Reject for the same direct-output regression/reset mismatch |

The low-work `mmax=1` controls remain in the CSV; their q8 shifts varied
substantially and do not override the full/reduced-work results. All safe
variants completed 36 backend samples across six configurations with finite
outputs and stable reset behavior. The two direct-output variants failed
reset consistency in all three process runs for `mmax=180` and full `mmax`,
q1/q8. Their numerical relative-L2 errors stayed around `5e-14`; numerical
agreement does not excuse the output-contract failure. No tolerance or
exceptional-value safeguard was changed.

The `ce710` versus `ff0fe` comparison remains historical-to-current evidence,
not attribution to one change: T720 full analysis improved 26.8% q1 and 39.4%
q8, while `mmax=180` improved 29.4% and 36.2%. Isolated 64/128/256-lane
kernels improved 31.6–36.5% versus `ce710`; current baseline kernels remain
1.07–1.18× DUCC. The sanitized historical rows above do not assign that net
gain to any single candidate.

**Production decision:** retire the packed complex reducer, scratch-field
reorder, 64/128 tile selectors and fused direct-output path. Keep the ordinary
real/imag SIMD reduction, original workspace field order, fixed 256 reflected
pair tile, checked range-proof cache, staged output and robust fallbacks. The
known-good source is the no-cfg `current-default` build above. It measured
about 1.05–1.08× DUCC at q1 and 1.00–1.03× at q8 for T720 `mmax=180`/full
analysis; q8 tails limit small-delta conclusions.

The scalar-analysis cumulative-ablation objective is closed. The next
implementation study is Plan 16, beginning with its executed-kernel crosswalk
before changing spin-1 recurrence code.
