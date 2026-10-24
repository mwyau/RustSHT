# Independent f64 RustSHT/DUCC algorithm review — 2026-10-22

Inspected RustSHT `7b1629d4f45b6a74cb4ee0e3a53c67f4faa51643` on
`opt/sht-performance-parity` and the actual, clean DUCC 0.42.0 checkout at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. This report follows the owner's
later steering to prioritize f64 spin-1 algorithms, then scalar CC and GL
table policy. No production numerical code was changed. The measurements
below concern f64. The f32 program remains paused.

**The main finding is an execution/dataflow difference, with substantial
checked fallback work.** Both libraries convert ordinary large-band spin-1
work into scalar harmonics. DUCC combines that conversion with north/south
reflection, independent parity sums, and a scaled-prefix/ordinary-kernel
transition. Rust's current T720/T1023 spin-1 path shares recurrence across
two components but evaluates the hemispheres separately, and spends a large
worker-time sum in checked scalar fallbacks. Merely separating parity sums
does not remove the dominant arithmetic; the previously rejected experiment
already demonstrates its register and recurrence costs.

The scalar CC sweep independently establishes large synthesis gaps in its
saved cells. Its final high-degree shard is incomplete. Scalar CC analysis
also has a serial resampling stage that spin-1's parallel resampler does not
share. These conclusions must remain separate.

Evidence is labeled **verified** for source inspection or direct validation,
**measured** for observed timing distributions, and **hypothesis** for an
optimization whose benefit remains untested. Supporting CSVs, numerical
controls, route counts, diagnostic patches and reproduction instructions are
in [the review data directory](data/f64-ducc-independent-review-2026-10-22/README.md).

**DUCC's executed spin-1 algorithm — verified**

The reference is the pinned
[sht.cc](https://github.com/mreineck/ducc/blob/42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b/src/ducc0/sht/sht.cc)
and
[sht_inner_loop.h](https://github.com/mreineck/ducc/blob/42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b/src/ducc0/sht/sht_inner_loop.h),
read locally for the paths identified below.

`alm2leg` selects its accelerated branch only for `lmax >= 500`, `STANDARD`,
and spin 1 or 2 (`sht.cc:649`). For spin 1, `make_ringdata` first pairs
reflected rings and sorts them by sine (`:190`); records with
`abs(sin(theta)) >= 1e-4` enter the scalar-converted path. Remaining records
use direct spin recurrence. The conversion is `spin1to0` (`:473`), through
degree `lmax+1`. Scalar normalization follows conversion. A dynamic queue
claims one order at a time; `gen.prepare(m)` is shared, but
`inner_loop_a2m` is called **twice**, once per converted component (`:705–717`).
The two scalar calls have separate recurrence state.

`fill_a2m` packs active reflected pairs into batches of at most 256 northern
latitudes, pads incomplete SIMD vectors with valid geometry, initializes
scalar recurrence, and zeros four parity/component sums
(`sht_inner_loop.h:1383`). `calc_alm2map` (`:542`) runs `iter_to_ieee` (`:426`),
handles intermediate scaled values with correction factors, then calls the
ordinary `alm2map_kernel` (`:459`). Scalar `sht_ftol` is `2^-60`; negative
scale epochs can produce zero correction factors. This is a numerical
approximation policy, not permission to remove Rust's extreme-value guards.
`extract_a2m` (`:1425`) combines even and cosine-weighted odd sums only after
the degree loop, reconstructs both hemispheres, and applies `1/sin(theta)`.
The degree tiling branch uses `lstep=4096`; it is not active at T720/T1023.

`leg2alm_internal` has the corresponding large-band split (`sht.cc:851`):
two scalar analyses precede scalar normalization and `spin0to1` (`:382`).
The remaining polar records use direct spin analysis, with their results
added to the ordinary contribution. `leg2map` (`:1008`) claims eight rings
at a time and processes both output components. Exact CC synthesis does not
call `resample_to_prepared_CC`: it synthesizes its requested latitude grid
directly. Exact CC analysis does call that resampler (`:1729–1789`).

A local counter build verified actual execution, rather than inferring it
from the presence of source code. Each row below is one public f64 spin-1
synthesis call; "pairs" includes an equatorial singleton when present.

| CC case | Physical rings in converted path | Physical rings in direct path | Scalar `inner_loop_a2m` calls | Scalar `calc_alm2map` calls | Direct-spin `calc_alm2map_spin` calls |
|---|---:|---:|---:|---:|---:|
| T499, full, q1 | 0 | 501 | 0 | 0 | 948 |
| T500, full, q1 | 500 | 2 | 1,002 | 1,002 | 102 |
| T511, full, q1 | 511 | 2 | 1,024 | 1,024 | 102 |
| T720, full, q1 or q8 | 720 | 2 | 1,442 | 2,276 | 102 |
| T720, m180, q8 | 720 | 2 | 362 | 724 | 102 |
| T1023, full, q8 | 1,023 | 2 | 2,048 | 3,698 | 102 |
| T1023, m255, q8 | 1,023 | 2 | 512 | 1,024 | 102 |

At T511 GL, all 512 rings take the converted path and no direct-spin
`calc_alm2map_spin` call executes. On these CC grids the direct path is the
polar pair; records excluded by the order limit do not run a recurrence.
All scalar `calc_alm2map` calls in the accelerated rows eventually reached
the ordinary scalar kernel. Counters do not measure time spent getting there.

Independent whole-map comparisons across the transition used the same
deterministic E/B fixtures in Rust and an uninstrumented pinned DUCC object.
Relative L2 errors were `9.90e-14`, `1.63e-13`, and `1.08e-13` at
T499/T500/T511 CC q1. The counter build agreed with clean DUCC to at most
`6.98e-16` relative L2. No timing conclusion is drawn from this diagnostic
build or from comparing different bandlimits.

**Current source crosswalk — verified**

| Work | DUCC 0.42.0 | Current RustSHT | Repeated work / implication |
|---|---|---|---|
| E/B conversion | `sht.cc::spin1to0:473`, once per order and call | [`spin_fast.rs:23`](../src/legendre/spin_fast.rs#L23), [`prepare_spin1_scalar_coefficients:1560`](../src/spin_transform.rs#L1560) | Same coefficient algebra and extra scalar degree. q8 cache is rectangular: 16,657,984 bytes at T720 full and 33,587,200 at T1023 full; DUCC uses worker-local rows. Conversion/cache preparation already has separate timing. |
| Large-band selection | `alm2leg:649`, STANDARD, T>=500, sine>=1e-4 | [`synthesize_spin_ring_group_orders:1851`](../src/spin_transform.rs#L1851), f64 scalar conversion from T15, sine>=1e-4, checked finite-order eligibility | Rust already implements the central scalar-conversion idea. DERIV1/GRAD_ONLY and unsafe converted orders require separate treatment. |
| Ordinary SIMD recurrence | `fill_a2m:1383`, `calc_alm2map:542`, `alm2map_kernel:459` | [`FusedPreparedSynthesisPairBlock:5182`](../src/legendre/scalar_paired.rs#L5182) | DUCC: two component passes over reflected pairs. Rust: one shared recurrence over every physical ring, four combined complex-component real sums. |
| Reflection/parity | `make_ringdata:190`, `extract_a2m:1425` | Main large-band path [`synthesize_spin_ring_group_mode_orders:1998`](../src/spin_transform.rs#L1998) processes both hemispheres independently | Rust's existing hemisphere kernel is gated to full-order GL T319–511, q4–q8 ([`spin_transform.rs:60`](../src/spin_transform.rs#L60)). It does not supply symmetry at T720/T1023 CC. |
| Paired coefficient preparation | `inner_loop_a2m:1532`, once per component/order | [`synthesize_spin1_scalar_block_to_order:2460`](../src/spin_transform.rs#L2460), twice per eligible latitude block | Current order queue removed repeated factor setup across ring groups. Prepared parity rows are still rebuilt for blocks; Plan 16 C measured that caching alone was ineffective. |
| Scale/near-polar handling | `mypow:347`, `iter_to_ieee:426`, `calc_alm2map:542` | Dual-channel kernel requires sine>=0.01 and normal initialization ([`:2454`](../src/spin_transform.rs#L2454)); checked transition/scalar visitor at [`:2665`](../src/spin_transform.rs#L2665) | The 1e-4 conversion split matches DUCC, but the ordinary dual-channel eligibility is narrower. Its fallback is usually scalar-converted recurrence, not direct spin. Direct spin remains for the polar segments and unsafe original coefficients. |
| Checked scalar fallback | Scaled vector recurrence, zero correction factors, ordinary transition | [`StagedSynthesisPairTransition:953`](../src/legendre/scalar_paired.rs#L953), then [`scalar.rs`](../src/legendre/scalar.rs) visitor when declined | Rust checks coefficient combinations, scaled products and sums for each lane/pair; failure may retry an unpaired, per-degree scaled traversal. A fallback block count alone omits the attempted work. |
| Current q8 traversal | One-order dynamic queue | [`synthesize_spin1_order_major_queue:1647`](../src/spin_transform.rs#L1647), eight-order atomic tiles | Both already schedule by order at T720/T1023 eligible CC. Rust clears two latitude-length scratch rows and scatters 32 bytes/ring/order into ring-major output, in addition to the kernel's row stores. |
| Spin-1 analysis | Two scalar passes over reflected pairs, then `spin0to1` | CC uses [`analyze_spin_scalar_fast_order:4290`](../src/spin_transform.rs#L4290); GL has a separate reflected-pair route | CC's prepared grid does not enter the GL-only pair route. The fused two-channel analysis kernel is q1-only; q8 uses a per-degree paired visitor and normalization checks. This is a specific recurrence/dataflow hypothesis for the 51.7 ms stage. |
| CC analysis resampling | `resample_to_prepared_CC:1168`, dynamic 64-order chunks | [`meridional.rs:350`](../src/meridional.rs#L350), bounded parallel order tiles for spin-1 | Accepted T1023 spin-1 stage evidence is comparable, 16.2 vs 17.1 ms. Preserve it. Scalar analysis instead calls the serial method at [`transform.rs:2200`](../src/transform.rs#L2200). |

**Parity arithmetic and assembly — verified mechanism; benefit still a hypothesis**

The executed Rust V3 ordinary dual-channel loop has four f64 lanes, four
cosine/coefficient FMAs, four accumulation FMAs, and two recurrence FMAs per
interior degree pair. The inspected native spin-1 binary SHA-256 is
`e112b7d0bf23a7e0d78f2658f27e7abd7442a37629d1e9ef7de8fc76ffc6c1f0`;
its loop at `0x1c4f20–0x1c4fb2` contains those ten packed FMAs. Cosine squared
is hoisted before that loop. There are no YMM stack operands in this loop.
Scalar tail paths are separate. No claim about all fallback code follows
from inspecting this ordinary loop.

For the same four northern rings and their four southern counterparts:

| Ordinary design | Approximate vector FMAs per degree pair, two output components |
|---|---:|
| Current Rust: shared recurrence, two physical hemisphere vectors | 20 |
| DUCC: two scalar component streams, one reflected vector | 12 |
| Plan 16 B parity-separated two-channel candidate, still both hemispheres | 20, plus final combination |
| Plan 16 B one-channel candidate, still both hemispheres | 24, plus final combination |

These counts ignore the last-pair recurrence omission, tails, checks, seed
work and memory effects. They are work estimates, not speed predictions.
Separating `sum(lambda*(even+cos*odd))` into separate even/odd sums exchanges
four coefficient-combination FMAs for four extra accumulation FMAs per pair.
It also doubles the live accumulator count. This explains why "move cosine
outside" alone was insufficient: the two-channel candidate spilled, and
the one-channel variant added recurrence work and regressed q1 synthesis by
about 23% in [Plan 16 B](plan16-spin1-parity-separated-ab-2026-10-22.md).

DUCC broadcasts coefficients outside its vector-block loop; Rust reloads
eight coefficient components plus two recurrence factors, approximately
80 bytes per vector/pair before cache reuse. Rust keeps its recurrence state
and sums in registers. DUCC instead loads/stores state arrays; its AVX2 loop
unrolls two degree pairs. These are real tradeoffs. Plan 15's loop reversal
lost to the added state traffic; neither that reversal nor the rejected
parity-only kernel should be repeated unchanged.

A **new hypothesis** is a bounded adapter using the existing scalar ordered
engine for each converted component over active reflected pairs, including
its checked scaled handling. Two component recurrence passes over half as
many physical rings avoid the recurrence duplication of Plan 16 B's
all-ring one-channel experiment. This also differs from the older coarse
hemisphere prototype: order ownership, global active-pair packing and
once-per-order setup are necessary, rather than pairing only fully ordinary
blocks. It remains an experiment requiring numerical and whole-call A/B
evidence, not a recommendation to broaden a production guard immediately.

**What currently costs time in spin-1 — measured diagnostics**

The accepted uninstrumented comparisons remain the performance baseline:
T720 CC q8 synthesis about 2.6–2.8x DUCC, T1023 about 2.0x full / 2.6x
reduced, and T1023 full analysis about 2.16x. See
[Plan 16 D](plan16-spin1-order-major-dataflow-2026-10-21.md) and
[Plan 16 E](plan16-spin1-t1023-cache-extension-2026-10-22.md).
The only subsequent `spin_transform.rs` production edit is a type-gated f32
group-grain rule; the f64 numerical paths inspected here retain those
algorithms. Historical f32 ratios contribute no evidence to this review.

Five-call, current-source diagnostic profiles confirmed the following
spin-1 routes. These heavily instrumented **worker sums overlap across q8
workers** and are not full-call latency claims or exact time fractions.

| CC synthesis | Successful fused blocks | Scalar fallback blocks | Fused attempt/accumulation worker sum | Checked fallback worker sum |
|---|---:|---:|---:|---:|
| T720 full q1 | 5,027 | 1,588 | 25.8 ms | 45.6 ms |
| T720 full q8 | 5,027 | 1,588 | 35.7 ms | 61.1 ms |
| T720 m180 q8 | 1,736 | 305 | 15.9 ms | 21.5 ms |
| T1023 full q8 | 9,485 | 3,035 | 87.8 ms | 205.0 ms |
| T1023 m255 q8 | 3,414 | 586 | 44.5 ms | 65.8 ms |

All four q8 cases allocated eight order scratch workers and used eight Rayon
workers. Full T1023's first diagnostic call had only 1.10x slowest/fastest
worker task time. Its remaining residual is not explained by the four-worker
underfill found separately in scalar CC synthesis. Per-block timers perturb
short work; the robust conclusion is that checked scalar fallback deserves
priority alongside ordinary-loop symmetry, rather than another coefficient
cache or FFT change. The next frequency probe should distinguish successful
scaled transition, early/late transition decline, and final scalar visitor,
weighted by degree/lane updates. That separates wasted retries from useful
scaled work without resurrecting Plan 15 compaction.

For analysis, the accepted T1023 profile already measures 51.7 ms in
recurrence versus 16.2 ms in Rust resampling and 17.1 ms in DUCC resampling.
Source inspection identifies CC's all-ring visitor and q8 fusion gate as
specific differences. Earlier q8 fusion trials did not establish a gain;
do not flip the gate without a new mechanism such as reflected-pair packing
and a matched current-source A/B.

**Scalar CC harness and coverage verdict — verified, conditional acceptance**

The new optional GRID argument is passed to both native workers. It retains
the legacy omitted-GRID GL protocol. CC fixtures use both poles,
`ntheta=lmax+2`, north-to-south latitude order and common `phi0=0.23`.
DUCC explicitly requires that latitude count in `analysis_2d`; Rust enforces
the same exact-analysis limit. CC's resampling is essential: ordinary CC
grid weights alone do not implement exact bandlimited analysis at this count.

The compact m-major coefficient body, DUCC's leading `mmax` padding and
`mstart[m]+l` indexing agree. m=0 coefficients are real. Both backends use
the same fixture bytes, longitude count, phase, scalar spin, and analysis /
synthesis operation. In analysis, both divide longitude spectra by `nphi`
before the same reflected, weighted meridional construction. The reflection
sign is `(-1)^m`, self-reflected endpoints receive half factors, and the
prepared-grid normalization is `0.5/(nfull_out*nfull_in)` when first
resampling is needed. Both use `good_size_complex(lmax+1)+1` prepared rings.

Timing boundaries are comparable **public warmed transforms**: Rust reuses
its plan/workspace, while DUCC's public API performs internal setup each
call. Setup and first calls are separately reported; this does not establish
an equivalent bare-kernel benchmark. q1 CPU 0 and q8 CPUs 0–7 are matched
physical cores in one CCD. Native flags and executable hashes are consistent
across all saved shards. Rust is strict FP with explicit Pulp FMA; DUCC's
SHT objects use GCC `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`,
with a strict driver. Backend process order alternates.

Severity-classified review issues:

- **High, coverage:** the final `high-2049-3072` runner and its workers were
  absent on inspection; its log stops at case 41/48 and it has no saved
  `results.json`. Its preceding log lines do not recover the in-memory
  numerical records. Saved data cover 13 shards, 618 geometries and 2,472
  operation/thread groups, every T0–256 and samples through T2048. No exit
  reason was established. The final 48 groups must remain unmeasured in
  machine-readable coverage until recovered or rerun.
- **High, coverage:** `make_cases` uses `max(1,lmax//4)` rather than the
  directive's `floor(lmax/4)`. Six T1–T3 m=0 geometries / 24 groups are
  absent; 16 saved groups at T2/T3 m=1 are extra relative to that directive.
  An independent manifest requires 632 geometries / 2,528 groups. Of the
  2,472 saved groups, 2,456 match it; 72 requested groups remain absent.
  The runner's self-generated coverage audit cannot discover this mismatch.
- **High, validation reporting:** the harness records finite random-output
  errors, roundtrip/basis errors, and adjoint residuals without enforcing
  their numerical tolerances. It mainly fails on nonfinite outputs, missing
  data and reset inconsistency. "Zero recorded failures" therefore does
  not prove that all numerical acceptance gates passed. A finite wrong
  answer can enter a row labeled complete. Fix the acceptance/reporting
  distinction and check saved values against established tolerances.
- **Medium, recovery:** native results are accumulated until shard completion.
  A long shard interruption loses its already completed in-memory records.
  Persist each completed pair/group atomically and resume by configuration
  identity, with frozen binary/hash checks.
- **Low, telemetry:** the worker's `row_fft_parallel_active` expression still
  excludes full-order T63 synthesis, although production removed that table
  exception and CC never used the GL table. The label is false for some
  executed parallel FFT cells. The literal four-lane field is also a host
  assumption; the actual V3 architecture is verified on this host.

The saved 7,416 random comparisons and 1,236 validation records are finite,
exact-zero checks hold, and all repeated-output reset flags hold. Maximum
random relative L2 is `3.187e-12`; maximum basis map cross-library relative
L2 is `2.020e-11`. These are distinct metrics. The largest recorded Rust
synthesis-adjoint relative-to-dot residual is `1.784e-11`. Rechecking its
T214 fixture with long-double dot products gave about `2.1e-11` relative
to the dot, but `2.0e-17` relative to the sum of norm products: the dot is
ill-conditioned by about `1.04e6`. This example does not establish an
operator defect; report both residual definitions rather than relaxing a
tolerance or interpreting cancellation as a performance regression.

Targeted frozen-worker analytic controls passed ten configurations / forty
backend-operation records: T0 both longitude parities, the missing m=0
low-degree shapes, altered phases, real and imaginary m=1 coefficients,
zonal and sectoral fields, and T255 q1/q8 odd/even controls. Legacy GL and
explicit GL produced exactly identical outputs in both workers. Existing
CC closed-form, DUCC-reference and adjoint tests also passed. These controls
support the adapter's numerical conventions; they do not close the missing
performance manifest or validate every extreme-value case anew.

Most short measurements have 30 timed batches, but not all meet the duration
window: 132,058 of 144,790 short-process batches lie within 20–100 ms
(91.2%). Actual per-backend counts include 15/20/25/30 because the long-call
decision is made independently in each backend process. Some published
summary code reports only Rust's count. Use actual backend counts and
retain mixed-route cases. Process pairs and batches are clustered samples;
zipping separately timed batches does not turn them into simultaneous
paired observations. Host waiting and wide process spreads remain visible
in the original records; no outlier was removed here.

**Scalar CC performance — measured saved cells, not a completed sweep**

The following are even-smooth full-order warmed medians from the saved
native f64 shards. Both analysis and synthesis have the same spectral
triangle in Rust and DUCC.

| Case | q | Rust / DUCC analysis ms | Ratio | Rust / DUCC synthesis ms | Ratio |
|---|---:|---:|---:|---:|---:|
| T255 | 1 | 4.026 / 3.854 | 1.04 | 6.957 / 1.458 | 4.77 |
| T255 | 8 | 2.308 / 1.303 | 1.77 | 3.128 / 0.299 | 10.45 |
| T720 | 1 | 60.146 / 59.120 | 1.02 | 90.315 / 20.435 | 4.42 |
| T720 | 8 | 34.557 / 12.292 | 2.81 | 47.916 / 2.951 | 16.24 |
| T1023 | 8 | 54.990 / 18.572 | 2.96 | 136.614 / 8.156 | 16.75 |
| T2048 | 8 | 449.600 / 123.008 | 3.66 | 1,414.172 / 65.689 | 21.53 |

These medians retain host interference. For example, T1023 full q8 synthesis
Rust process medians were 147.4/132.1/130.7 ms and DUCC's 7.37/7.95/8.86 ms.
This variability cannot plausibly account for the entire observed gap.

The matched GL control at T720 full q8 measured Rust / DUCC synthesis
3.397 / 3.070 ms, versus CC's 47.916 / 2.951 ms. GL uses 721 rings and CC
722; this is a diagnostic comparison, not identical geometry. Rust selects
the scalar ordered synthesis engine only for GL
([`transform.rs:1135`](../src/transform.rs#L1135)); CC uses the general
ring-group route. Source and the measured CC/GL separation identify this
dispatch/dataflow difference as substantive. CC has no eligible compact
Legendre table; the GL 20 MiB cap cannot explain this selection.

Bounded T720 scalar phase diagnostics, with two independent Rust processes
and 60 observations per cell, and three DUCC processes / 30 calibrated
batches, give this crosswalk in milliseconds:

| Scalar full CC stage | Rust q1 | DUCC q1 | Rust q8 | DUCC q8 |
|---|---:|---:|---:|---:|
| Analysis longitude FFT | 3.26 | 3.92 | 0.49 | 0.66 |
| Analysis resampling / latitude setup | 25.56 | 44.64 | 26.50 | 8.35 |
| Analysis recurrence / accumulation | 31.48 | 11.25 | 4.39 | 2.11 |
| Synthesis recurrence / spectrum | 83.17 | 17.09 | 38.07 | 2.32 |

Rust's resampling phase includes spectrum normalization and latitude setup;
DUCC's corresponding interval brackets `resample_to_prepared_CC`.
Recurrence intervals also contain different preparation work. These local
stage probes are attribution evidence, not a replacement matched A/B campaign;
backend captures were not interleaved with one another. CPUs were 8 / 8–15
on the other CCD. The first Rust probes overlapped another project's smoke
process on the host; its processes subsequently exited. Later profiles
retained observed variability. No direct kernel speed factor is inferred
from these unequal stage boundaries.

The scalar-specific serial resampler is demonstrably the q8 analysis floor
at T720. q1 public-call parity masks opposite stage differences: Rust's
resampling is faster, its recurrence slower. Route counters found 672 normal
seed blocks and 472 checked scaled blocks, with zero prefix-enabled blocks,
at full order; quarter order had 232 / 130. Both operations selected
recurrence at q1 and q8. Prefix advancement is explicitly disabled on CC
analysis, independently of the obsolete large-latitude cutoff.

Scalar q8 synthesis has a separate scheduling defect: T720 uses twelve
64-ring groups with `with_min_len(ceil(12/8))`; Rayon executes four coarse
tasks of three groups. The counter probe observed four actual workers at
T255/T720/T1023. T720 full had 5,493 successful fused blocks and 1,220
visitor fallbacks; none took the T>=1023 scalar transition. Changing the
**task grain** is a small decisive next A/B; it does not require changing
the production 256-pair analysis tile. Even perfect scheduling will leave
the missing ordered/symmetric/scale-transition algorithm to address.

**The GL 20 MiB cutoff — verified dispatch explanation, provisional policy**

The existing study covers T0–256 but its five 1 ms batches are exploratory.
Focused independent 30-batch runs reproduce the nonmonotonic q1 table /
recurrence ratios. The source explains a major part of their structure:
[`F64TableAnalysisGroup::with_simd:588`](../src/legendre/scalar_table.rs#L588)
selects the cached AVX2 two-complex-lane accumulator only when
`ntheta % 4 == 0`. Other latitude counts call `analyze_group_scalar`, which
rebuilds north/south sums inside each degree pair. The frozen binary's scalar
accumulator has no YMM arithmetic; this is an actual scalar fallback, not a
claim based on source naming. T127/T159/T191 qualify for the vector path;
their neighbors generally do not.

| Full-order even q1 analysis | Payload MiB | Table / recurrence, two focused processes | Build cost, representative |
|---|---:|---:|---:|
| T96 | 1.78 | 0.931 / 0.930 | 1.70 ms |
| T104 | 2.25 | 1.018 / 1.023 | 1.78 ms |
| T126 | 3.97 | 1.165 / 1.159 | 3.78 ms |
| T127 | 4.03 | 0.733 / 0.733 | 3.11 ms |
| T128 | 4.16 | 1.091 / 1.091 | 2.42 ms |
| T159 | 7.86 | 0.770 / 0.768 | 5.50 ms |
| T191 | 13.57 | 0.862 / 0.858 | 9.36 ms |
| T223 | 21.53 | 1.218 / 1.183 | 14.66 ms |

Thus neither cache capacity nor a single monotonic byte crossover explains
the data. The earlier report's statement that focused repeats "support"
20 MiB overstates the evidence for optimality. It remains a conservative
resource guard. It admits reproducibly slower table cells below the cap,
and focused data do not resolve T217/T218, near-full eligible order, or
independent cold/reuse workloads. The 34 MiB allocation ceiling is another
distinct guard, not the 20 MiB analysis selector.

Warm savings also do not imply first-use savings. Using build cost divided
by warm saving alone gives illustrative amortization counts of about 12
calls at T63, 122 at T96, 34 at T127 and 85 at T191. These omit independent
cold first-use effects. The existing test runs table first, recurrence
second, sharing plan/workspace and a resident table. Its alternating warm
route measurements are useful; its first calls are sequence-coupled and its
resident-table recurrence differs from a production plan that never builds
one. Preserve the cap pending a defensible rule. Do not replace it with a
new degree exception or a different unexplained byte count.

The next policy study should stratify by actual vector/scalar table
execution. Reuse the existing full-order T104/T126–128/T159/T191 repeats;
first add T192 and T215–220, particularly T217/T218, on the same current
source. Add `mmax=lmax-lmax/4` near-full order at T127/T128/T191/T192/T217/T218
with both longitude shapes. Use independent route processes, calibrated
>=30 batches across runs, and q1 analysis as the primary case. Existing
q8/synthesis evidence supplies controls; refresh selected T127/T218 controls
if policy or shared implementation changes. For a rule that survives these
warm comparisons, measure fresh-plan total costs at reuse 1/2/4/8/16/32/64,
including a recurrence-only plan with no resident table. Hold out additional
neighbors in each dispatch class for acceptance rather than rerunning the
entire T0–256 screen. Reduced
quarter-order forced routes are not evidence for production eligibility,
which requires at least 75% of full order. Generalizing table SIMD tails
could change the crossover; that would be a separate attributable kernel
experiment, not a reason to fit a policy to the current sawtooth.

**Constrained handoff for Luna**

The performance opportunities are ranked below in the owner's priority
order. Ranking reflects the current evidence and scope; it is not a speedup
prediction. Harness acceptance and missing coverage remain prerequisites
for publishing a complete CC campaign.

| Rank | Opportunity | Established evidence | Smallest decisive next experiment |
|---|---|---|---|
| 1 | Spin-1 checked fallback and reflected-pair scalar execution | Large fallback worker sums; DUCC's reflected two-stream execution verified | Untimed weighted transition/visitor counts, then one converted order over reflected pairs with scaled and late-decline controls; only then T720/T1023 full/reduced q1/q8 A/B |
| 2 | Scalar CC synthesis dispatch and scheduling | Ordered engine bypassed; four q8 workers observed; large native saved-cell gaps | Change only task grain for a T720 full q1/q8 control; separately validate one-order ordered-engine adaptation before whole-call timing |
| 3 | Scalar CC analysis serial resampling | About 26.5 ms at q8 versus 4.4 ms recurrence in the T720 diagnostic | Reuse existing bounded worker buffers/resampler in one isolated candidate; compare T720 full/reduced q1/q8 stages and whole call |
| 4 | Spin-1 CC analysis recurrence organization | Existing 51.7 ms recurrence stage; all-ring visitor and q8 fusion gate verified | One-order reflected analysis/adjoint proof with prepared CC geometry, followed by a matched T1023 q1/q8 control; preserve resampling |
| 5 | GL table dispatch and amortization | Vector eligibility explains strong sawtooth; cold/reuse crossover unresolved | Complete the focused dispatch-stratified cases above and independent fresh-plan reuse tests before selecting policy |

1. Preserve the frozen CC binaries and thirteen saved shards. Fix manifest
   semantics and numerical acceptance reporting, add completed-group
   persistence, and reconcile the final shard on a quiescent host. Add the
   missing low m=0 cells without discarding valid existing measurements.
   Publish the requested sweep/plots only after every item is accounted for.
2. For the owner's next f64 spin-1 implementation investigation, first split
   fallback frequency into scaled transition success, declined retry, final
   scalar visitor, and true direct-spin work. Count weighted lane/degree
   updates outside timing runs. Keep T720/T1023 full/reduced and q1/q8 as
   controls; do not allocate another transform-wide coefficient cache.
3. Test one private, checked adapter to the existing scalar ordered engine
   for converted spin-1 rows over reflected pairs. The new mechanism must
   include active-pair packing and scaled/ordinary transitions. Preserve the
   polar/original-coefficient fallback and discard partial order output on
   decline. A one-order numerical/assembly probe should precede a calibrated
   whole-call A/B. Repeating all-ring parity separation or loop reversal
   does not test this mechanism.
4. For scalar CC, separately A/B one-group task grain with existing ring
   groups. Then evaluate an adapter to the existing ordered scalar engine;
   validate CC reflection, poles, singleton equator, phase, subnormal/extreme
   inputs and late fallback before considering production eligibility.
   Preserve analysis tiles at 256 reflected pairs.
5. Scalar analysis may reuse the existing bounded parallel CC resampler;
   its scalar workspace currently does not prepare spin-1's worker buffers.
   This is supported by new scalar timing, and does not justify rewriting
   the accepted spin-1 resampler or changing any FFT engine. After that
   floor is addressed, investigate CC analysis's checked scaled recurrence
   and missing reflected/fused organization, with coefficient/range safety
   intact. Do not blindly enable an insignificant-prefix flag.
6. Retain the GL baseline selector while completing the dispatch-stratified,
   cold/reuse study above. A production policy change needs held-out
   whole-call wins and numerical validation; cross-architecture behavior is
   still unproven on this Ryzen-only evidence.

All proposed production changes require a separate attributable commit and
matched native validation. The review does not reopen the rejected Plan 15
loop reversal, Plan 16 parity-only kernels, analysis tile experiments, f32
optimization, spins 2–4, or FFT backend work.

Validation of the reviewed production tree: formatting passed; release
workspace all-feature tests passed (270 library tests, 4 extreme-scaling,
11 f32 and 1 workspace-compatibility test; opt-in tests remain ignored).
Workspace no-default tests also passed, but Python's dependency unifies the
parallel feature there. Package-only no-default release tests passed with
220 library tests, 4 extreme-scaling, 11 f32 and 1 compatibility test;
17 library and 1 f32 opt-in tests remained ignored. Strict all-target,
all-feature workspace Clippy passed with warnings denied.
No Python implementation or binding was changed. Temporary diagnostic
projects, raw assembly, fixtures and host/process captures remain local;
the committed artifacts contain concise numerical and timing evidence.
