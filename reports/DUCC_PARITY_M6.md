# M6 — current-head DUCC parity triage

## 2026-10-28 block exponent-correction suffix transition rejected at exact parent 746c7e60

An isolated synthesis-only candidate restored both recurrence states from their shared f32 exponent at a block transition, then used the shared fused SIMD suffix. Three interleaved warm parent/candidate/pinned-DUCC pairs per GL T720 full/reduced q1/q8 cell show a clear slowdown in all twelve candidate comparisons. The candidate patch was not promoted.

| Cell | Parent / candidate / DUCC ms | Candidate/parent | Parent/DUCC | Candidate/DUCC | Status |
|---|---:|---:|---:|---:|---|
| Full q1 | 645.744 / 701.308 / 19.774 | 1.082 | 32.60× | 35.49× | **GAP; reject** |
| Full q8 | 96.136 / 108.665 / 2.662 | 1.155 | 36.12× | 40.83× | **GAP; reject** |
| Reduced q1 | 191.771 / 222.484 / 8.771 | 1.160 | 21.84× | 25.36× | **GAP; reject** |
| Reduced q8 | 50.782 / 64.536 / 1.205 | 1.270 | 42.15× | 53.55× | **GAP; reject** |

The separate feature-instrumented Legendre-stage screen also regressed in each cell: 1.071×/1.253× full and 1.142×/1.269× reduced q1/q8. It found 33.598M/33.048M full and 13.695M/13.736M reduced useful suffix contributions completed at q1/q8. Another 445/940 full and 159/303 reduced blocks declined; 98/131/93/93 blocks found no handoff; all 98 partial tails bypassed the transition. Prefix work was 8.134M/7.328M full and 3.282M/2.395M reduced q1/q8. Recurrence-state declines were zero; later coefficient/product/output checks caused declines. Declined and no-handoff blocks discarded prefix work and replayed through the original fallback.

The route coverage did not pay for repeated staged-sum scans at each candidate degree, per-degree suffix range checks, and block replay. Candidate/parent relative L2 was 1.57e-7–1.78e-7, maximum absolute difference 1.79e-7–2.38e-7. Candidate/DUCC error matched the parent: relative L2 1.7234e-4 full / 2.7255e-4 reduced. Focused release f32 integration, scalar recurrence, and paired synthesis tests passed; timed workers passed finite/repeat/reset checks. Adjacent timing extensions and a candidate f64 control were not run after the conclusive four-cell regression. No tolerance was weakened and no production code changed.

Pinned DUCC's init_lambda/iter_to_ieee/calc_alm2map path keeps double recurrence state plus per-lane scale, corrects the residual scale once after the prefix, then enters a four-degree/two-pair AVX2 double-FMA kernel. The candidate reused Rust's shared native-f32 fused kernel, but its normal/product/sum checks and transactional replay did not preserve DUCC's low-check bulk suffix cost. AVX2 disassembly and source-level path comparison are in the [screen report](data/ducc-parity-m6-2026-10-28/f32-scaled-prefix-suffix-746c7e60/README.md).

A matched candidate-specific dynamic instruction count was not obtained: perf events are blocked by host perf_event_paranoid=4, and the one-transform Callgrind attempt was stopped after more than 77 seconds without a complete profile. No candidate equivalent-work-normalized Ir result is claimed for that suffix trial. The production parent 746c7e60 Rust CI run passed. The current-source native-f32 spin-1 synthesis and analysis profiles follow; keep scalar synthesis on the scorecard and M6 open.

## 2026-10-28 current-source f32 spin-1 q1 profile at `0f821099`

The current exact-source Callgrind screen measured full GL T720 native-f32
spin-1 synthesis at 22,433,360,982 Rust Ir versus 520,826,788 pinned-DUCC Ir
(43.07x). Spin-1 analysis measured 15,752,365,732 versus 430,971,670 Ir
(36.55x). These are instructions for one warmed transform, not cycles or
elapsed timing. The synthesis counts normalize to 139.06 Rust and 3.23 DUCC
Ir per 161,323,854 logical degree-latitude output contributions. Rust's
test-only route census recorded 4.504B recurrence lane updates and 322.648M
scaled-basis calls, two per useful contribution.

The executed Rust synthesis callback has 7.136B self Ir; its
`scaled_basis_value_f32` child has 8.364B inclusive Ir over 322.648M calls.
These overlap and are not additive. The emitted callback assembly confirms
two indirect scaling calls per active lane followed by scalar Q/U accumulation.
DUCC's high-Lmax `synthesis<float>` path converts spin-1 coefficients into two
scalar channels, processes 361 north/equator ring records for the 721-ring GL
grid, and reflects the results. Its Callgrind run executes `alm2map_kernel`
(122.213M Ir) and does not execute `calc_alm2map_spin` or
`alm2map_spin_kernel`. Its emitted scalar kernel uses four AVX2 double lanes
and fused multiply-add recurrence/accumulation. Rust's current f32 policy
leaves this scalar paired-spin route disabled.

The old three-pair, code-equivalent timing screen at `b3289762` measured
spin-1 synthesis at 37.654x/47.938x full q1/q8 and 35.864x/40.444x reduced
q1/q8. These elapsed values are reported separately from the Callgrind
profile; they are not a fresh 15-pair acceptance campaign at `0f821099`.
The narrow f32 dispatch screen at parent `092fe72f` is recorded below. See the
[profile report](data/ducc-parity-m6-2026-10-28/f32-spin1-profile-0f821099/README.md)
for exact flags, hashes, raw profiles, and disassembly. Scalar synthesis stays
a major GAP and M6 remains open.

## 2026-10-28 f32 spin-1 separate paired-scalar dispatch screen — not promoted

The isolated one-gate candidate allowed native-f32 spin 1 to enter the
existing fast-scalar eligibility and separate-channel paired-scalar path. It
did not alter the recurrence or globally enable paired-spin support. Exact
parent was `092fe72f322bd482dac73b020785b096772bc901`; pinned DUCC was
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

Three interleaved GL T720 parent/candidate/DUCC pairs ran per full/reduced
q1/q8 cell. The reduced-q8 result initially suggested a 10.3% gain, so that
cell received 15 more pairs. Timings and hashes are in the [screen report](data/ducc-parity-m6-2026-10-28/f32-spin1-paired-scalar-screen-092fe72f/README.md).

| Cell | Parent / candidate / DUCC median ms | Candidate/parent paired median | Candidate/DUCC paired median | Status |
|---|---:|---:|---:|---|
| Full q1 | 1484.907 / 1493.662 / 40.528 | 1.006 | 37.10x | **GAP** |
| Full q8 | 271.436 / 268.161 / 5.563 | 0.983 | 48.53x | **GAP** |
| Reduced q1 | 615.820 / 613.619 / 17.682 | 0.995 | 34.64x | **GAP** |
| Reduced q8, initial 3 | 123.724 / 101.717 / 2.459 | 0.897 | 41.36x | **INCONCLUSIVE** |
| Reduced q8, 15-pair extension | 122.540 / 102.338 / 2.487 | 1.006 `[0.823, 1.036]` | 41.17x | **INCONCLUSIVE** |

Across the initial matrix, parent/candidate output was bitwise equal and all
workers passed finite, repeat, and reset checks. The 15-pair reduced-q8
candidate/parent interval spans parity; only 6/15 pairs were faster, with
paired ratios from 0.796 to 1.244. Separate parent/candidate medians are
strongly dispersed, so the paired ratio controls the decision. Candidate/DUCC
error was `1.41495e-4` relative L2 and `9.13277e-4` max absolute on the
reduced shape. No analytic/adjoint/extreme-value or protected f64 control was
run after the targeted screen failed to confirm a gain.

No candidate-specific route-success counter, Legendre-stage timing, or
equivalent-work instruction count was collected. Therefore this result does
not establish that more work reached the fused scalar path. **Decision:** do
not promote; no whole-transform gain is established. The exact-current-source
scalar-synthesis baseline and stage profile follow.

## 2026-10-28 f32 spin-1 paired-scalar adapter census and initial screen — rejected

A test-only route census at exact current parent `8472668b` confirmed why the
one-gate candidate above did not establish a route change: f32 spin-1 work had
zero fast-scalar scratch and inherited the default prepared-block `Ok(false)`.
Thus the earlier one-gate timings were not evidence for paired-scalar kernel
execution. A new isolated adapter provisioned the existing f32 worker scratch
and called the shared native-f32 prepared paired-scalar synthesis kernel. It
reused the scaled visitor fallback and did not add another recurrence
implementation. Full route counts and the recoverable candidate are in the
[route census and screen report](data/ducc-parity-m6-2026-10-28/f32-spin1-shared-scalar-route-8472668b/README.md).

The route census counts paired recurrence lane updates only within blocks
entering the adapter. Full/reduced fused-versus-fallback paired lane updates
were 55.53M/106.38M and 38.42M/39.69M at q1; at q8 they were 56.11M/93.22M
and 38.42M/39.69M. q8 additionally declined 1,214 outer full-mmax blocks to
the general-spin path (150 reduced); these are not expanded into lane-update
counts. These are route counts, not whole-transform instruction or timing
estimates.

Three interleaved fresh-process whole-transform pairs ran per cell on GL T720
spin-1 synthesis, parent `8472668b`, candidate based on that parent, and pinned
DUCC `42ce8ab5`. Parent/candidate/DUCC median ms were `1528.307/786.247/40.824`
full q1, `613.044/389.330/17.695` reduced q1, `270.121/274.927/5.571` full
q8, and `125.068/177.267/2.498` reduced q8. Candidate/parent paired medians
were `0.515`, `0.633`, `1.018`, and `1.417`; all q1 pairs favored the
candidate, while full q8 favored it once and reduced q8 zero times. Candidate
versus parent relative-L2 differences were `1.70e-4` full and `2.53e-4`
reduced; candidate versus DUCC relative-L2 errors were `1.96e-4` and
`2.11e-4`. All workers passed finite, repeated-call and workspace-reset
checks, but the outputs were not bitwise identical.

**Decision:** reject the unconditional adapter. The q1 gains were large, but
the reduced-q8 regression is decisive and the full-q8 result is neutral to
slower. No 15-pair or extended correctness screen was warranted for this
unconditional change, and no candidate-specific Legendre-stage timing was
collected. Independent analytic-mode, adjoint, exceptional-value and f64
controls remain unrun. No production code changed. Next, refresh the GL T720
f32 spin-1 analysis baseline at current source (the earlier Callgrind was at
`0f821099`, before the accepted scalar-analysis leaf), then resume the planned
reflected-ring f32 scalar-synthesis hypothesis. M6 remains open.

## 2026-10-28 exact-current native-f32 scalar-synthesis baseline — all cells GAP

Production scalar-synthesis source is `092fe72f322bd482dac73b020785b096772bc901`;
latest `dev` head `023ba6f3` changes documentation only. Pinned DUCC is
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The GL T720 warm whole-transform
screen used three interleaved parent/DUCC pairs per cell, identical fixture
geometry and x86-64-v3 AVX2/FMA settings. Paired parent/DUCC ratios are medians
of the three per-pair ratios; these are a current baseline screen, not an
acceptance campaign for a candidate.

| Cell | Rust parent ms | DUCC ms | Paired parent/DUCC | Max Rust/DUCC rel-L2 / abs error | Status |
|---|---:|---:|---:|---:|---|
| Full q1 | 642.768 | 20.015 | 32.08× | `1.72343e-4` / `9.64403e-4` | **GAP** |
| Full q8 | 97.791 | 2.657 | 36.94× | `1.72343e-4` / `9.64403e-4` | **GAP** |
| Reduced q1 | 191.050 | 8.389 | 22.77× | `2.72546e-4` / `1.54787e-3` | **GAP** |
| Reduced q8 | 51.287 | 1.205 | 42.43× | `2.72546e-4` / `1.54787e-3` | **GAP** |

An isolated test-only phase probe measured 30 warmed calls per cell. Median
Legendre recurrence and spectral accumulation consumed `99.68–99.82%` of the
whole call; inverse FFT and writeback consumed `0.076–1.593 ms`. The current
source-equivalent census counts 161,199,393/81,210,177 Rust useful
degree-latitude terms for full/reduced mmax versus 80,729,837/40,662,194 DUCC
northern-ring terms, with reflected outputs. This is about 2× the recurrence
visit count, not a cycle estimate. q8 used 23 ring-group jobs; counter-built
worker contribution totals spanned 3.12M–56.47M full and 3.09M–18.21M reduced,
but those worker nanoseconds are instrumentation-contaminated and are not
reliable makespan measurements. q1's 22.77–32.08× gap confirms the dominant
disparity is not scheduling alone.

The earlier route probe on the byte-identical synthesis source counted
1,744/4,337 initial all-normal fused blocks and 3,657/7,112 non-polar fallback
blocks for full q1/q8; reduced had 922/2,036 fused and 972/1,849 fallback
blocks. That probe did not count polar-excluded or mixed blocks. A later
guarded transition screen at `746c7e60` did measure the scaled-prefix route;
the production paired-synthesis and scalar-transform source blobs are exactly
the same at `746c7e60` and current source `092fe72f`:

| GL T720 f32 synthesis | Completed blocks | Declined blocks | No transition | Partial tails | Prefix terms | Guarded fused suffix terms |
|---|---:|---:|---:|---:|---:|---:|
| Full q1 | 3,114 | 445 | 98 | 98 | 8.134M | 33.598M |
| Full q8 | 6,041 | 940 | 131 | 98 | 7.328M | 33.048M |
| Reduced q1 | 720 | 159 | 93 | 98 | 3.282M | 13.695M |
| Reduced q8 | 1,453 | 303 | 93 | 98 | 2.395M | 13.736M |

Transition degree min/median/max was 5/399/720 and 5/392/720 full q1/q8,
5/130/715 and 5/132/715 reduced q1/q8. The recurrence-state guards did not
decline after transition; the 159–940 block declines were from later
coefficient, product or accumulated-output checks. Blocks that declined or
found no transition discarded prefix work and replayed through the original
fallback. The candidate regressed whole-transform latency 8.2%, 15.5%, 16.0%
and 27.0% full q1/q8 and reduced q1/q8; its measured Legendre stage regressed
7.1%, 25.3%, 14.2% and 26.9%. These are guarded candidate counts, not
production route counts. The exact patch, paired timings, tests and DUCC AVX2
body are in the [suffix-screen evidence](data/ducc-parity-m6-2026-10-28/f32-scaled-prefix-suffix-746c7e60/README.md).

DUCC's `init_lambda`/`iter_to_ieee` keeps double recurrence states plus a
per-lane scale through the prefix; `calc_alm2map` applies the residual scale
once and hands the suffix to a four-degree/two-pair AVX2 double-FMA loop. Rust
already emits packed f32 YMM FMA in the ordinary kernel, but the guarded route
added coefficient/product/output checks and transactional replay around that
kernel. Real-state replay found exponents from -160 to -320 where restoring a
nonzero recurrence state in f32 produced zero while scaled coefficient
products remained representable. Keep the original scaled fallback; do not
repeat the block-check/replay suffix route unchanged.

The separate f32-I/O/f64-internal diagnostic found Rust's existing f64
reflected order-major route at 0.746–0.779× DUCC, while forcing the same f64
build through all-ring paired recurrence cost 3.714–13.442× DUCC. This
quantifies a large dataflow cost beyond f32 exponent range. After this
documented suffix blocker, continue with remaining native-f32 scalar/spin
families, then return to reflected-ring scalar synthesis. Keep scalar analysis
and f64 dispatch unchanged. Current-source AVX2 disassembly shows Rust packed
f32 YMM FMA; DUCC's executed loop uses packed f64 YMM FMA and an unrolled
degree suffix. Callgrind on the source-equivalent q1 parent/DUCC profile
measured 6.610B vs 260.8M Ir (41.01 vs 1.62 Ir per required output
degree-latitude slot); Ir are instructions, not elapsed time. Source, hashes,
per-pair timings, phase data, and reproduction notes are in the [baseline
evidence](data/ducc-parity-m6-2026-10-28/f32-scalar-synthesis-baseline-092fe72f/README.md).
All four cells remain **GAP** and M6 remains open.

## 2026-10-28 native-f32 scalar-analysis callback SIMD accepted at parent `4770e1c7`

A narrow f32-only accumulation leaf now handles callback contributions only
when the shared recurrence reports normal range, all exponents are zero, and
a conservative spectrum/product bound passes. Declined, scaled, subnormal,
nonfinite, oversized and unsupported inputs retain the existing scalar scaled
callback. The shared f64 analysis override is unchanged. Exact parent was
`4770e1c7504d084a979a62ca0d8ef57e9fd293f8`; candidate patch SHA-256 is recorded
in the linked evidence report.

Fifteen interleaved paired whole-transform comparisons per GL T720 f32 scalar-
analysis cell show repeatable gains. Candidate/parent intervals are paired
percentile bootstraps; all four cells remain **GAP**:

| Cell | Parent / candidate / DUCC ms | Candidate/parent [95% CI] | Candidate/DUCC | Max candidate-parent rel-L2 | Status |
|---|---:|---:|---:|---:|---|
| Full q1 | 542.702 / 455.413 / 14.013 | 0.839 [0.811, 0.841] | 32.44× | 7.368e-7 | **GAP** |
| Full q8 | 71.905 / 60.078 / 2.246 | 0.834 [0.831, 0.840] | 26.73× | 7.368e-7 | **GAP** |
| Reduced q1 | 188.596 / 140.455 / 5.752 | 0.743 [0.742, 0.748] | 24.37× | 8.554e-7 | **GAP** |
| Reduced q8 | 25.572 / 19.145 / 1.042 | 0.750 [0.746, 0.765] | 19.06× | 8.554e-7 | **GAP** |

The test-only contribution census found 58.829M/80.730M full and
32.554M/40.662M reduced useful paired degree-latitude terms safe for the
ordinary f32 accumulator (72.9%/80.0%). Every normal-range term passed; scaled
terms remained on the fallback. q8 job/work counts were balanced. The same
one-first-transform Callgrind boundary measured 6.174B→4.847B Rust Ir full and
2.379B→1.640B reduced; normalized to 161.199M/81.210M useful updates, this is
38.30→30.07 and 29.30→20.19 Ir/update. Pinned DUCC measured 1.38/1.13
Ir/update on that boundary. These are instruction counts, not elapsed time.
The inspected AVX2 path uses eight f32 lanes and separate multiply/add
operations, then a horizontal reduction. Full route counts, raw profiles and
disassembly are retained in the [candidate report](data/ducc-parity-m6-2026-10-28/f32-scalar-analysis-simd-4770e1c7/README.md).

The operation-order change is small but non-bitwise: maximum parent-relative
L2/absolute error is `8.554e-7` / `4.471e-8`; max candidate/DUCC relative-L2
is `2.205e-5`. The explicit T720 analytic mode test measures relative-L2
`4.321e-5`–`4.323e-5` and max absolute error below `4.65e-6`, under its
`1e-4` / `2e-5` bounds. Adjoint, poison/reset, 19-lane tail, f32 extreme-value
and paired-analysis tests pass; no tolerance was weakened. Strict all-target,
all-feature Clippy passes.

Protected GL T1023 f64 spin-1 synthesis is bitwise identical to parent. q1 is
neutral across three pairs. q8 has fifteen pairs, candidate/parent median
`1.002×` and bootstrap interval `[0.980, 1.047]`; the variable q8 timing does
not establish a meaningful regression. Adjacent full-mmax f32 spin-1 analysis
is also bitwise identical and about 4% faster in the three-pair q1/q8 screen;
spin synthesis is neutral/noisy and is not counted as a gain.

This is a material scalar-analysis improvement, not parity. Native-f32 scalar
synthesis remains a major GAP. The later exact-parent 746c7e60 block-transition
screen completed safe nonzero-exponent suffixes but regressed every cell; its
current parent ratios are 32.60×/36.12× full and 21.84×/42.15× reduced q1/q8.
Continue with f32 spin-1 synthesis/analysis profiling, then revisit the remaining
f64 gaps. M6 stays open.

## 2026-10-28 current native-f32 scalar-analysis profile at `72bea503`

After the corrected scalar-synthesis suffix candidate was rejected as a
transition blocker, profiling moved to the other native-f32 families as
directed. Current source `72bea50383f57bed6f54f48a3f360f01b180d5c5` is
documentation-only beyond scalar-analysis code at exact timing parent
`2b531d73049fdbc6e1b94b2132bfb9cb3476d319`. Pinned DUCC 0.42.0 remains
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

| GL T720 f32 scalar analysis | Rust ms | DUCC ms | Rust/DUCC | Pairs | Max rel-L2 | Max abs | Status |
|---|---:|---:|---:|---:|---:|---:|---|
| Full q1 | 545.836 | 14.017 | 38.94× | 15 | 1.829e-5 | 4.295e-7 | **GAP** |
| Full q8 | 72.382 | 2.015 | 35.93× | 15 | 1.829e-5 | 4.295e-7 | **GAP** |
| Reduced q1 | 193.015 | 5.781 | 33.38× | 3 | 2.205e-5 | 9.057e-7 | **GAP** |
| Reduced q8 | 26.056 | 0.876 | 29.89× | 3 | 2.205e-5 | 9.057e-7 | **GAP** |

The reduced screen is baseline-only and does not certify a candidate. All
workers passed finite, repeat and workspace-reset checks. q1 Callgrind records
6.170B/2.378B Rust Ir versus 214.2M/86.5M DUCC Ir full/reduced. Normalized to
161.199M/81.210M useful degree × reflected-latitude pair updates, that is
38.28/29.28 Rust Ir and 1.33/1.07 DUCC Ir per update. Rust's
`multiply_complex_scaled_f32` is called 14.21M full and 40.69M reduced times;
it accounts for 36.78% full and 51.39% reduced inclusive Ir. DUCC's AVX2
`map2alm_kernel` accounts for 51.9%/64.6% of DUCC Ir and fuses recurrence with
two-degree, four-pair-lane accumulation. The Rust f32 `ShtScalar` default
still accumulates the callback lane-by-lane; its f64 override is vectorized.
The exact disassembly, nested callgrind attribution, call counts and
source-derived worker data are in the [profile report](data/ducc-parity-m6-2026-10-28/f32-scalar-analysis-profile-72bea503/README.md).

The existing test-only per-order census observed all eight q8 workers. Full
used 721 order jobs (89–91 per worker) with similar per-worker elapsed leaf
time; reduced used 181 jobs (22–23 per worker), also balanced. This makes
scheduling an unlikely leading cause of the scalar-analysis gap. The next
bounded hypothesis is an f32-native safe accumulation leaf with the current
scaled scalar fallback retained; count safe callback contributions before
coding. Keep the f64 analysis visitor unchanged. Then profile the f32 spin-1
families. M6 remains active.

The existing three-pair spin-1 baseline at source `b3289762` is code-equivalent
for these cells: `git diff b3289762 72bea503 -- src` contains only scalar
synthesis and its tests. Its full/reduced q1/q8 ratios are 35.21×/32.69× and
30.74×/28.09× for spin-1 analysis, and 37.65×/47.94× and 35.86×/40.44× for
spin-1 synthesis. These remain baseline GAPs, not a fresh exact-HEAD timing
certification. The paired runs and error checks are in the [all-family source
baseline](data/ducc-parity-m6-2026-10-28/f32-all-family-current-source/README.md).

## 2026-10-28 packetized native-f32 suffix screen at `2b531d73` — not promoted

Exact parent `2b531d73049fdbc6e1b94b2132bfb9cb3476d319` was fetched from
`origin/dev` before the screen. Against pinned DUCC 0.42.0
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, an isolated packet route reused
the shared f32 recurrence, accumulator, and fused SIMD kernel. It partitions
declined 32/64-ring blocks into SIMD-width packets, accumulates the scaled
prefix through the existing callback, stages a checked suffix transactionally,
and falls back to the untouched recurrence state on decline.

Fifteen interleaved warm whole-transform pairs ran per GL T720 f32 scalar
synthesis cell. Ratios and intervals are paired candidate/parent results.

| Cell | Parent / candidate / DUCC median ms | Candidate/parent [95% CI] | Candidate/DUCC | Faster pairs | Status |
|---|---:|---:|---:|---:|---|
| Full q1 | 680.737 / 643.320 / 20.397 | 0.94630 [0.94251, 0.95308] | 31.59× | 14/15 | **GAP** |
| Full q8 | 100.357 / 96.367 / 2.873 | 0.95932 [0.94338, 0.97653] | 33.32× | 15/15 | **GAP** |
| Reduced q1 | 202.371 / 184.452 / 8.894 | 0.91721 [0.91458, 0.91986] | 20.88× | 15/15 | **GAP** |
| Reduced q8 | 53.510 / 53.061 / 1.324 | 0.98741 [0.93760, 1.00305] | 40.93× | 11/15 | **GAP** |

The q1 gains are repeatable (5.4% full, 8.3% reduced); full q8 improved 4.1%.
Reduced q8 is inconclusive and effectively neutral. A separate eight-sample
test-only phase profile measured the Legendre stage independently: full q1
improved 652.589→613.512 ms, while reduced q1 was highly variable and q8
stage medians were neutral/noisy. The inverse FFT remained below 1.71 ms.
Those stage samples are diagnostic; whole-transform pairs determine the screen.

Candidate route counts per fallback packet were, in full/reduced q1/q8 order:
initial packet fusion `15.246M / 6.213M / 6.626M / 2.699M` terms; scaled
callback prefix `16.703M / 21.693M / 5.285M / 6.479M`; fused suffix
`59.633M / 61.195M / 26.483M / 27.159M`. All suffix attempts completed with
zero decline/replay. However, the corrected state contained **zero nonzero
exponent lanes** in all four cells. Removing the `normal_range` guard in a
separate route-only probe did not change any packet or term count. Thus the
candidate reduces scalar callback work, but its safe fused suffix still waits
for exponent zero; it does not reproduce DUCC's early scaled-prefix handoff.

The pinned DUCC path initializes a double recurrence plus per-lane scale,
advances a scaled prefix in `iter_to_ieee`, handles any remaining scaled
contributions in `calc_alm2map`, then applies the per-lane correction to both
states once and enters a four-degree (two recurrence-pair) AVX2 double-FMA suffix. Rust's
checked f32 fallback emits YMM FMA but also per-degree coefficient,
contribution, and recurrence checks. Its unprofiled V3 fallback symbol is 14,720 bytes
(3,321 static instructions, 358 static branches, including fallback and tail
code); DUCC's extracted `alm2map_kernel` is 738 bytes. These are static code
shapes, not dynamic work, cycles, or elapsed-time estimates. The existing
[f32-I/O/f64-internal diagnostic](data/ducc-parity-m6-2026-10-28/f32-internal-double-current-source/README.md)
shows that DUCC's reflected-ring reuse is another major factor; that source
predates this exact parent and is used only as algorithmic context.

Candidate numerical validation passed 23 focused f32 library tests and 12
f32 integration tests, including T720 analytic modes, scalar adjoint,
workspace reset, polar decline, overflow, subnormal, nonfinite, mixed exponent,
and partial-vector cases. All 60 parent/candidate pair cases (120 Rust outputs) were finite and
repeat/reset consistent. Candidate-parent relative L2 was
`1.71e-7`–`1.90e-7` (max absolute at most `2.39e-7`); candidate/DUCC relative
L2 remained `1.7234e-4` full / `2.7255e-4` reduced. Adjacent f32 controls were
run after the initial screen. Across 15 paired comparisons, scalar analysis
regressed 3.89% q1 (95% interval `[1.03697,1.04042]`) and 3.95% q8
(`[1.03518,1.04559]`); only 1/15 q1 and 0/15 q8 pairs were faster. Three-pair
spin-1 analysis and synthesis controls were approximately neutral. The
protected f64 T1023 spin-1 timing was not run after this clear adjacent f32
regression. No source code was promoted. See the [screen report](data/ducc-parity-m6-2026-10-28/f32-corrected-transition-2b531d73/README.md)
for exact pairs, route counters, stage samples, hashes, assembly and patches.

**Decision at that checkpoint:** reject both isolated transition patches.
Native-f32 range prevented a nonzero-exponent handoff on these fixtures; the
route-only probe found no additional safe states. The adjacent scalar-analysis
regression also fails the no-regression gate. The subsequent current-source
scalar-analysis profile above supersedes the reflected-ring measurement as the
immediate task. M6 remains active; all four synthesis cells remain **GAP**.


## Historical 2026-10-28 native-f32 prefix-to-fused-suffix follow-up at earlier source `49295a89` — not promoted

At exact source parent `49295a89f2d13d8b02c2cbfa8566a8c5c21c6a80`
(`src/` is identical to `ef11ce0a`) against pinned DUCC 0.42.0
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, an isolated candidate reused the
shared f32 recurrence and fused kernel after all recurrence lanes had exponent
zero. It moved 33–38% of active degree–latitude terms into the fused suffix,
but kept per-pair range/contribution checks and did not correct nonzero
residual exponents once. Fifteen interleaved warm whole-transform pairs per
cell measured:

| GL T720 f32 scalar synthesis | Parent ms | Candidate ms | DUCC ms | Candidate/parent | Candidate/DUCC | Status |
|---|---:|---:|---:|---:|---:|---|
| Full q1 | 677.754 | 649.485 | 19.907 | 0.9570 | 32.64× | **GAP** |
| Full q8 | 100.800 | 96.577 | 2.795 | 0.9576 | 34.58× | **GAP** |
| Reduced q1 | 201.600 | 189.859 | 8.714 | 0.9426 | 21.72× | **GAP** |
| Reduced q8 | 53.024 | 53.688 | 1.273 | 1.0142 | 42.09× | **GAP** |

Reduced q8 was slower in 12/15 pairs; the 95% bootstrap interval for its
paired candidate/parent ratio was `[1.0043, 1.0187]`. The other three cells
improved 4.3–5.7%, which is insufficient to offset the repeatable reduced-q8
regression or count as a material algorithmic step. Suffix failures were all
late recurrence-range declines; discarded attempts replayed only 0.09–0.19M
pair-lane contributions per cell. The f32 Rust/DUCC gap remains 21.72–42.09×.
The candidate was not promoted and adjacent/f64 controls were not run.

At that earlier source, the work census recorded about 2× Rust recurrence work
versus DUCC's reflected northern-hemisphere route. The reflected-ring
hypothesis was later screened separately; the active next step is recorded at
the top of this ledger. Full route counts, suffix-length bins, stage samples,
numerical errors, assembly and exact candidate patch are in the [screen report](data/ducc-parity-m6-2026-10-28/f32-prefix-suffix-transition-49295a89/README.md).
M6 remains open.

## 2026-10-28 native-f32 T720 all-family baseline at `b3289762`

At exact Rust source `b3289762c7ba019523e4908b6172109d6975bfca` (production
transform source unchanged from `54b166f2`) against pinned DUCC
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, three alternating matched pairs
per cell measured GL T720 native-f32 scalar synthesis/analysis and spin-1
analysis/synthesis, full/reduced mmax at q1/q8. The four scalar-synthesis
Rust/DUCC ratios are `36.737×`/`40.512×` full and `25.258×`/`45.888×`
reduced q1/q8; all remain **GAP**. Adjacent scalar analysis is `29.321–38.755×`,
spin-1 analysis `28.088–35.210×`, and spin-1 synthesis `35.864–47.938×`.
All 96 worker processes passed finite, repeat and workspace-reset checks.
T720 scalar synthesis max Rust-vs-DUCC relative-L2 / absolute errors are
`1.723e-4 / 9.644e-4` full and `2.725e-4 / 1.548e-3` reduced. These three-pair
values are a current baseline, not an independent numerical acceptance suite
or a parent/candidate screen. The exact methods, paired values, fixture and
binary hashes are in the [16-cell report](data/ducc-parity-m6-2026-10-28/f32-all-family-current-source/README.md).

This is the pre-candidate family baseline. Native-f32 exponent folding can
destroy nonzero recurrence states; the f32-I/O/f64-internal reflected route is
near DUCC, while the all-ring paired recurrence does about twice the useful
recurrence updates. The accepted fallback-vectorization result below changes
only scalar synthesis. Do not restore the rejected 513-line f32 reflected
recurrence candidate; it regressed protected reduced scalar-analysis q8 by
4.3%.

## 2026-10-28 accepted native-f32 scaled-fallback packet SIMD

The candidate patch on exact parent `33c2faba2f9be2f890b27543bd3f887c715f7a25`
vectorizes coefficient/product/scaled accumulation for checked full f32
fallback packets in the shared paired scalar-synthesis visitor. It retains the
existing recurrence, exact scalar exceptional route, subnormal scaler, and
partial-vector handling. The change does not add a recurrence copy or alter
f64 analysis. Against pinned DUCC 0.42.0
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, the current GL T720 Rust/DUCC
ratios are:

| f32 scalar synthesis | Parent ms | Candidate ms | DUCC ms | Candidate/parent [95% CI] | Candidate/DUCC | Status |
|---|---:|---:|---:|---:|---:|---|
| Full q1 | 731.604 | 672.131 | 19.719 | 0.92124 [0.91203, 0.92996] | 34.14× | **GAP** |
| Full q8 | 118.428 | 103.919 | 3.410 | 0.87294 [0.82945, 0.89726] | 30.65× | **GAP** |
| Reduced q1 | 218.893 | 198.619 | 8.575 | 0.90868 [0.90233, 0.92115] | 23.11× | **GAP** |
| Reduced q8 | 59.042 | 54.896 | 1.592 | 0.94219 [0.90064, 0.98313] | 35.47× | **GAP** |

Fifteen interleaved exact-parent pairs per cell were faster in all four cases
(13/15 for reduced q8). Separate Legendre-stage medians improved by 7.4–17.8%.
The f32 scaled-component helper's source-attributed Callgrind call count fell
30.7% full and 37.0% reduced; whole-call q1 Ir fell 10.51%/10.99%. The
candidate remains far from DUCC and does not implement a recurrence suffix.
Independent T720 analytic relative-L2/max-absolute errors were
`1.36819e-7/2.98023e-7` reduced and `1.51022e-7/3.57628e-7` full; scalar
adjoint relative errors were `1.06850e-6`/`8.07212e-6`. All 60 primary
parent/candidate outputs were bitwise equal. Protected f64 GL T1023 spin-1
synthesis remained bitwise equal and had no meaningful regression. Adjacent
f32 full spin-1 analysis q1 measured a small repeatable 1.27% slowdown
(`1.01274`, 95% interval `[1.00860,1.01694]`); other extended q8 controls
were inconclusive and are disclosed in the report.

The unguarded scaled-prefix-to-fused-SIMD-suffix candidate remains rejected:
real-state replay showed nonzero recurrence values underflowing to zero at
one-time exponent correction while later scaled contributions remained
nonzero. A corrected route guard completed suffixes only when every lane's
exponent stayed zero through the end. That route count covers 37.62M/33.88M
full and 16.70M/15.07M reduced paired-accumulation lane slots at q1/q8
(41.10–46.62% of active slots); it has not been timed or numerically accepted.
This route census preceded the guarded handoff screen above. The accepted
packet leaf remains an interim gain, not DUCC parity. M6 remains open; test
synthesis-only reflected-ring reuse next, using the shared recurrence and
leaving analysis/f64 routing unchanged.

## 2026-10-28 guarded zero-exponent suffix screen — rejected

The exact parent was current source `ef11ce0a1043d8acb049ce9178513e4cde058789`
(production transform code from `acc625d7`); pinned DUCC was
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Three interleaved warm
parent/candidate/DUCC pairs ran per GL T720 f32 scalar-synthesis cell.
Candidate/parent medians were `1.0274`, `1.0327`, `1.0401`, and `0.9448` for
full/reduced q1/q8. Full q1, full q8, and reduced q1 regressed in all three
pairs; reduced q8 favored the candidate in two of three and remains
inconclusive. The parent/candidate Rust/DUCC ratios in this same screen were
`34.44×`/`19.90×` and `23.59×`/`39.89×` for full/reduced q1/q8; candidate
ratios were `35.32×`/`21.02×` and `24.57×`/`39.27×`. No 15-pair extension was
warranted.

The seven-call Legendre-stage medians changed by `+3.76%`, `+5.32%`, `+2.22%`,
and `-5.98%` in the same cell order. Full-q1 Callgrind measured
`6,610,413,955` parent versus `6,925,618,870` candidate Ir (`+4.77%`), or
`41.01` versus `42.96` Ir per required degree-latitude slot. The pinned DUCC
same-shape reference is `260,801,202` Ir (`1.62` per slot); it is from the
earlier matched DUCC profile and was not reprofiled in this candidate screen.
Ir are instruction references, not cycles. The candidate's AVX2 V3 fused
symbol grew from 1,772 to 4,879 bytes and it added a 10,012-byte packet
fallback symbol. Both paths use YMM FMA; code size and Ir confirm that the
candidate added instruction work, but do not isolate the cost of each added
branch or traversal.

Candidate-parent maximum relative-L2 stayed below `1.2e-7` and maximum
absolute difference below `2.1e-7`; all workers were finite, repeat
consistent, and reset consistent. Candidate/DUCC relative-L2 remained
`1.7234e-4` full and `2.7255e-4` reduced. The isolated candidate passed seven
paired-synthesis module tests and the f32 integration suite (12 passed, 1
ignored), including analytic, adjoint, polar, and reset cases. The broad
exceptional matrix and protected f64 timing controls were skipped after the
primary screen rejected promotion. No production code changed.

See the [screen report](data/ducc-parity-m6-2026-10-28/f32-zero-exponent-suffix-screen/README.md)
for exact paired data, stage samples, focused Callgrind, emitted assembly,
numerical errors, and the rejected patch. It did not instrument completed
suffix/decline counts for this candidate; the earlier `37.62M`/`33.88M` full
and `16.70M`/`15.07M` reduced safe-suffix slot counts remain route opportunity,
not candidate-executed work.

**Next measured hypothesis:** use GL reflection to form both hemispheres from
the northern recurrence. Prior equivalent-work counts show DUCC performs about
half Rust's useful recurrence updates. Prototype a synthesis-only path using
the shared native-f32 recurrence, leaving scalar analysis and f64 dispatch
untouched; do not restore the prior duplicated reflected candidate, which
regressed reduced scalar-analysis q8.

## 2026-10-28 exact scalar-fallback route census

At repository HEAD `bb53b86dc4ed47a9fb2cc01fd06b4e5d8626a7f6` (transform
source unchanged from `54b166f2c0698f1c70e88f8679d12fa4376704fe`), a feature-only
instrumented census isolated the existing full-vector scalar fallback for GL
T720 f32 synthesis. Full/reduced q1/q8 contained 3.730M/5.203M and
1.338M/1.661M paired fallback slots. Every fallback packet had normal
coefficient, basis and product lanes; all but two full-mmax packets scaled at
least one result below the normal exponent range, and no packet overflowed.
Only 24.9K/35.1K full and 13.0K/16.1K reduced q1/q8 fallback slots passed a
deferred-scale segment safety screen (0.67–0.97%); a normal exponent-adjust
packet covered only two full-mmax packets and none reduced. Do not implement
either low-coverage route. The broad stable-exponent count includes work
already accepted by the existing packet SIMD path, so it is not a measure of
removable fallback work. q8 still ran all eight workers across 23 jobs, with
paired work distributed 4.69–20.40M slots per worker full and 1.54–10.97M
reduced; the instrumented worker times are not scheduling measurements. See
the [census and counter definitions](data/ducc-parity-m6-2026-10-28/f32-fallback-transition-census/README.md).

At this census source, before the accepted candidate above, the matched
Rust/DUCC ratios were `36.737×`/`40.512×` full and `25.258×`/`45.888×`
reduced q1/q8. The earlier q1 profile attributed 2.726B self Ir to the
scaled-component helper; those are instruction references, not elapsed time.
This census supplies route-distribution evidence for the candidate above and
is not itself a candidate timing result.

## 2026-10-28 f32-I/O / f64-internal diagnostic

At repository source `096216465ea35ab747ce977d5b65cf27dc4681f1` (transform implementation unchanged from `54b166f2c0698f1c70e88f8679d12fa4376704fe`), a three-pair native-ISA screen reproduced the earlier f32 baseline within run variation. Whole-transform Rust/DUCC ratios were `36.802×`/`40.487×` full and `25.376×`/`46.062×` reduced q1/q8. The f32-I/O/f64-internal default route measured `0.779×`/`0.769×` full and `0.746×`/`0.755×` reduced; forcing the same f64 build through the all-ring paired route cost `4.680×`/`12.260×` full and `3.714×`/`13.442×` reduced. This is diagnostic only; no production precision policy changed. The f64 reflected order-major route's Legendre stage was `0.731–0.839×` DUCC, while the f32 paired stage was `26.6–43.7×`. Instrumented work counts show Rust's all-ring paired route performs about twice DUCC's recurrence updates; DUCC forms both reflected outputs from northern-ring work. Reduced q8 also showed `6.13×` instrumented worker-time skew, but q1 and the near-DUCC f64 reflected route show scheduling is not the primary gap. The reflected-ring f32 candidate was later screened and rejected because reduced scalar-analysis q8 regressed 4.3%; its implementation must not be repeated unchanged. The exact scalar-fallback census above shows normal-range packet handoff covers little of the true scaled fallback. See the [complete diagnostic report](data/ducc-parity-m6-2026-10-28/f32-internal-double-current-source/README.md) and [route census](data/ducc-parity-m6-2026-10-28/f32-fallback-transition-census/README.md).

**Historical shared native-f32/f64 fused scalar-synthesis milestone: complete.** The shared typed recurrence and fused accumulation path, exponent-zero fast paths, native-f32 SIMD leaves, and accepted 32-ring schedule are implemented and validated. Source `54b166f2c0698f1c70e88f8679d12fa4376704fe` is the historical pre-packet-leaf implementation; its full/reduced GL T720 scalar-synthesis ratios were `36.591×`/`40.257×` full q1/q8 and `25.432×`/`45.090×` reduced q1/q8. All four cells were **GAP** at that source. The earlier unguarded exact-zero suffix attempt was rejected after real-state replay showed nonzero recurrence values underflowing to zero while later scaled contributions remained nonzero. A corrected route census bounded possible safe suffix work, but the current-source guarded handoff was later timed and rejected in the section above. See the [real-state replay](data/ducc-parity-m6-2026-10-28/f32-exact-zero-operand-suffix-rejection/README.md). This file remains the single current M6 ledger; dated source rows below are historical evidence.

**Exact-zero transition census — handoff rejected:** on GL T720, the unguarded route reported 29,188/28,135 full and 7,950/7,570 reduced q1/q8 completed packets and `88.77M`/`86.18M` full and `37.95M`/`35.89M` reduced suffix slots. Real-state replay proved some were unsafe nonzero-to-zero rescalings. With that underflow rejected, the safe route completed 28,269/26,647 full and 7,589/7,164 reduced q1/q8 packets, covering `37.62M`/`33.88M` and `16.70M`/`15.07M` paired-lane accumulation slots, respectively; every completed packet had exponent zero. The unguarded q1 candidate had relative L2 error `0.430165`/`0.376613` and max absolute error `0.716657`/`0.611738`. No candidate timings were collected. The [transition report](data/ducc-parity-m6-2026-10-28/f32-exact-zero-operand-suffix-rejection/README.md) contains exact states, per-worker counts, and patch provenance.

**Previously accepted scheduling change — 32-ring f32 GL scalar synthesis:** against exact parent `9dc45160`, the q8 candidate/parent paired medians are `0.7808` full and `0.6420` reduced (15/15 faster pairs; bootstrap 95% intervals `[0.7331,0.8321]` and `[0.6216,0.6607]`). q1 medians are `0.9826`/`0.9840` from three pairs; q1 does not execute parallel tasks. All eight workers were already active at 64 rings; the split changes job count 12→23 and reduces max/min worker group-time skew 3.24→1.70 full and 6.56→3.49 reduced. Workspace payload grows 738 KB; q8 workspace construction adds about 3.4 ms full and 26 μs reduced, outside warm transform timing. Adjacent f32 and protected f64 controls show no meaningful regression. The [complete screen](data/ducc-parity-m6-2026-10-27/f32-32-ring-schedule-2026-10-27/README.md) contains exact revisions, paired timings, allocation, numerical checks, route counters, ring costs and worker diagnostics.

**Historical native-f32 scaled-contribution packet leaf:** candidate `ec5d56ca` added checked f32 SIMD arithmetic for safe packets. That change was followed by the accepted packet-vector leaf `acc625d7`, whose exact-parent performance and controls are documented above. The [earlier packet-leaf report](data/ducc-parity-m6-2026-10-27/f32-scaled-packet-leaf-2026-10-27/README.md) retains its source-specific timings, errors, controls and AVX2 code.

**Historical scaled-prefix handoffs:** earlier block-wide and unguarded packet candidates failed their recorded source-specific screens. Real-state replay showed the unguarded one-time correction could underflow nonzero recurrence states to zero. A stricter route guard found safe all-zero-exponent suffix opportunity of 37.62M/33.88M full and 16.70M/15.07M reduced paired slots at q1/q8; the current-source guarded candidate was then screened and rejected in the section above. Do not repeat those candidates unchanged. See the [packet suffix report](data/ducc-parity-m6-2026-10-28/f32-packet-suffix-candidate/README.md), [real-state replay report](data/ducc-parity-m6-2026-10-28/f32-exact-zero-operand-suffix-rejection/README.md), and [current-source rejection report](data/ducc-parity-m6-2026-10-28/f32-zero-exponent-suffix-screen/README.md). DUCC uses reflected GL rings; the f32-I/O/f64-internal route remains diagnostic only. All current scalar-synthesis cells remain **GAP**.

## Reflected-ring f32 synthesis candidate — rejected

The isolated candidate reused the shared paired recurrence state to synthesize
the northern GL hemisphere and reconstruct the southern rings from even/odd
degree sums. It preserved the existing general route for other shapes and kept
the polar/scaled fallback. On the exact post-layout source, three paired
whole-transform runs per cell measured candidate/parent medians `0.6662` full
q1, `0.5697` full q8, `0.7087` reduced q1, and `0.3874` reduced q8. The
candidate remained 16.8–23.8× pinned DUCC. These synthesis gains do not pass the
protected-cell gate: reduced GL T720 f32 scalar analysis q8 regressed 4.3% in
15/15 paired runs, with a 95% bootstrap interval of `[1.0384,1.0455]` for
candidate/parent. Parent and candidate analysis outputs were bitwise identical.
The code-layout split did not eliminate the regression, and available evidence
does not establish its cause. **Do not promote this candidate.**

Formatting, workspace/all-target/all-feature checking, warning-denied Clippy,
and the focused T720 f32/f64-reference, analytic-mode, adjoint, and workspace
reset integration test passed. The candidate was rejected before completing
the broad exceptional-input matrix. Exact timings, outputs, hashes, patch and
validation limits are in the [candidate report](data/ducc-parity-m6-2026-10-28/f32-reflected-ring-candidate/README.md).

The current-source f32-I/O/f64-internal diagnostic and real-state replay have
since been completed. The fallback census confirms that almost all counted
scalar packets reach the subnormal/zero exponent region; neither deferred
aggregation nor normal exponent adjustment has useful route coverage. After
this documented blocker, measure the other current-source native-f32
scalar/spin cells, then pursue reflected-ring synthesis with a design that
does not reproduce the protected scalar-analysis regression. Do not restore
the rejected candidate unchanged.

**Earlier rejected screens:** the global task-grain candidate hurt protected f64 T1023 spin-1 synthesis; the transactional normal-suffix handoff hurt protected f64 T720 spin-1 analysis; the mixed-lane and per-vector retries did not deliver a stable whole-transform gain. Their source-specific evidence remains linked in the historical sections below.



**Most recent broad current-head baseline:** RustSHT `98adf1e6b5334021bfe6ae9abb618c6be167a822` on `dev`; pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Host: AMD Ryzen 9 5950X; rustc/Cargo 1.99.1, LLVM 23.1.1; GCC 15.2.0. The detailed [22-cell report](data/ducc-parity-m6-2026-10-27/current-head-rebaseline-98adf1e6/README.md), [machine-readable results](data/ducc-parity-m6-2026-10-27/current-head-rebaseline-98adf1e6/current-head-rebaseline.json), and [per-cell ledger](data/ducc-parity-m6-2026-10-27.csv) record that source's timings, pair ratios, hashes, errors, and worker diagnostics.

**Latest accepted source slice:** the shared f32/f64 recurrence at `a30537bdcfa0fc5273985abfd592998325a608de` plus the f32-only exponent-zero caller fast path screened below. The shared recurrence's exact parent is `972bacdaef57388509738d1c2cdebbb7973b89be`. Its original focused GL T720 scalar-synthesis screen measured q1 candidate/DUCC at 47.66×/34.36× and q8 at 140.90×/117.24× across full/reduced mmax; the [fused-synthesis report](data/ducc-parity-m6-2026-10-27/f32-fused-synthesis-2026-10-27/README.md) preserves that evidence. This is not a current full-matrix baseline.

## Native-f32 exponent-zero caller fast path — 2026-10-27

The paired scalar-synthesis caller now handles exponent-zero lanes with the same two native-f32 multiplies used inside `multiply_complex_scaled_f32`, avoiding the helper call for the common case. Against exact parent `a30537bd`, three alternating GL T720 full-mmax scalar-synthesis triplets per q gave paired median candidate/parent ratios `0.974301` q1 and `0.986955` q8; all six candidate runs were faster and Rust outputs were bitwise identical. Candidate/DUCC was `45.854×` q1 and `137.242×` q8, so the remaining f32 gap is substantial. Scalar tests passed (168 passed, 16 ignored), f32 integration passed (11 passed, 1 ignored), and all worker finite/repeat/reset checks passed. See the [screen report and preserved patch](data/ducc-parity-m6-2026-10-27/f32-expzero-caller-fastpath-2026-10-27/README.md).

## GL T720 f32 scalar-synthesis q8 Callgrind attribution — 2026-10-27

One warmed transform was profiled at current source `a30537bd` against exact parent `972bacda` and pinned DUCC `42ce8ab5`. The current whole-call profile collected **8,465,515,501 Ir**, versus **9,609,850,325** for the parent and **252,805,383** for DUCC. This is an 11.91% instruction-count reduction from the parent, not a cycle or elapsed-time measurement. The separate whole-transform timing screen recorded candidate/parent `0.9846` and candidate/DUCC `140.90×` for this q8 cell; parity remains far off. Output stayed finite; candidate/parent relative L2 was `3.9400e-7`, and candidate/DUCC was `1.72343e-4`. The [evidence folder](data/ducc-parity-m6-2026-10-27/f32-callgrind-q8-2026-10-27/README.md) preserves compressed valid profiles, temporary instrumentation patch, disassembly, source/fixture/output hashes, and Valgrind warning details.

`multiply_complex_scaled_f32` accounts for 39.15% inclusive Ir, while the paired fallback traversal is about 40.78% inclusive, including nested calls; do not add these nested counts. The all-normal fused SIMD block executes but admits only fully normal blocks. A separate trace at parent `972bacda` saw 1,744/5,499 blocks eligible. The mixed-lane mask follow-up was screened and rejected; the result is recorded below.

## Mixed-lane f32 scalar-synthesis fusion screen — rejected

Three alternating fresh-process parent/candidate/DUCC triplets per q ran for GL T720 full-mmax scalar synthesis. The q1 paired median candidate/parent ratio was `0.993786` (range `0.980393–1.003015`), a mixed near-parity result. At q8 it was `1.019974` (`1.016261–1.025878`), with all three candidate pairs 1.63–2.59% slower; candidate/DUCC was `46.878×` q1 and `142.984×` q8. The candidate matched parent within relative L2 `1.240014e-7`, and all workers passed finite, repeat, and reset checks. The focused mixed-lane test, scalar suite (169 passed, 16 ignored), f32 integration suite (11 passed, 1 ignored), formatting, and diff checks passed. The extra branch and fused pass were not retained. See the [screen evidence](data/ducc-parity-m6-2026-10-27/mixed-lane-fused-synthesis-screen-2026-10-27/README.md).

## Native-f32 vector-group fusion screen — rejected, 2026-10-27

At exact current parent `3189fe8e`, this candidate retried the existing fused synthesis kernel one native f32 vector at a time and kept exceptional groups on the scaled callback. Three alternating parent/candidate/DUCC triplets per GL T720 spin-0 synthesis cell covered full/reduced mmax at q1/q8. Median candidate/parent ratios were `0.99044` full q1, `0.99864` full q8, `0.98358` reduced q1, and `1.00002` reduced q8. The narrow q1 gains (0.1–1.6%) did not carry to q8; the path added a substantial f32-specific branch, so it was removed. No 15-pair screen was warranted. All 36 workers passed finite, repeat, and reset checks; candidate/parent relative L2 was at most `1.18e-7`, and candidate/DUCC error matched the parent to the shown precision. Focused mixed-scale/scalar/f32 tests, format, diff checks, and warning-denied library Clippy passed. See the [screen report and exact rejected patch](data/ducc-parity-m6-2026-10-27/f32-vector-fusion-screen-2026-10-27/README.md), [triplet data](data/ducc-parity-m6-2026-10-27/f32-vector-fusion-screen-2026-10-27/triplets.json), and [provenance](data/ducc-parity-m6-2026-10-27/f32-vector-fusion-screen-2026-10-27/provenance.json).

## Native-f32 analysis current-source rebaseline — 2026-10-27

Eight GL T720 scalar and spin-1 analysis cells were remeasured against exact parent `8c4dba98`, using production source `3189fe8e` and pinned DUCC 0.42.0. Three parent/current/DUCC triplets per cell showed bitwise-equal current/parent output in all 24 comparisons. The scalar-analysis route was 9.6–15.1% faster; spin-1 analysis was near parity, with median changes from 0.4–1.5% slower. All 72 workers passed finite, repeat, and reset checks. Current/DUCC remains a 27.38×–38.50× gap for scalar analysis and 28.81×–36.57× for spin-1 analysis. See the [cell report and results](data/ducc-parity-m6-2026-10-27/f32-analysis-rebaseline-2026-10-27/README.md) and [provenance](data/ducc-parity-m6-2026-10-27/f32-analysis-rebaseline-2026-10-27/provenance.json).

This fresh baseline covers six f64 spin-1 controls and all 16 native-f32 GL T720 scalar/spin-1 analysis/synthesis full/reduced q1/q8 cells. Three interleaved fresh-process pairs ran per cell; the ranges below are descriptive triage ranges, not confidence intervals. All 22 cells passed finite, repeat, and reset checks. The existing f32 integration suite passed: 11 passed, 1 ignored, 0 failed. No production code changed in this baseline.

## Selected source-dated f64 cells

These representative f64 rows belong to their recorded earlier source screens;
they are not a full remeasurement of source `9dc45160`. The scoped f32
candidate's f64 scalar T720 and protected spin-1 T1023 synthesis controls are
the current affected-path measurements, linked in the experiment report above.

| Workload | Rust ms | DUCC ms | Rust/DUCC | Three-pair range | Status | Max relative L2 |
|---|---:|---:|---:|---:|---|---:|
| GL T720 full spin-1 analysis q1 | 36.569 | 29.234 | 1.249× | 1.234–1.271× | **GAP** | 4.76e-14 |
| GL T720 full spin-1 analysis q8 | 5.372 | 4.260 | 1.298× | 1.220–1.307× | **GAP** | 4.76e-14 |
| GL T1023 full spin-1 synthesis q1 | 87.990 | 121.330 | 0.734× | 0.721–0.745× | **PASS** | 2.60e-13 |
| GL T1023 full spin-1 synthesis q8 | 14.080 | 19.775 | 0.715× | 0.696–0.716× | **PASS** | 2.60e-13 |
| CC T720 reduced m180 spin-1 synthesis q1 | 30.010 | 18.644 | 1.608× | 1.600–1.704× | **GAP** | 3.62e-13 |
| GL T720 reduced m180 over-resolved synthesis q8 | 4.255 | 4.677 | 0.911× | 0.902–0.912× | **PASS** | 2.27e-13 |

The T720 analysis target remains open: all three q1 and q8 pair ratios were above parity. Rust q1 scheduler wait was 6.4–6.7% (DUCC below 0.1%); q8 Rust wait was 0.4–1.1%. T1023 synthesis retains its speed advantage. CC reduced synthesis is a separate gap with elevated q1 Rust wait around 9.8–10.3%.

## Latest native-f32 result

The current-source GL T720 f32 scalar-synthesis cells are `ec5d56ca` in the machine-readable [per-cell ledger](data/ducc-parity-m6-2026-10-27.csv): candidate/DUCC is `36.37×`/`40.07×` at full `mmax=720` q1/q8 and `25.11×`/`45.32×` at reduced `mmax=180` q1/q8. Candidate/parent is `0.9783`/`0.9754` full and `0.9685`/`0.9901` reduced q1/q8; reduced q8 remains inconclusive. The [packet-leaf report](data/ducc-parity-m6-2026-10-27/f32-scaled-packet-leaf-2026-10-27/README.md) records matched timings, correctness, route bounds, assembly and protected controls; the [32-ring report](data/ducc-parity-m6-2026-10-27/f32-32-ring-schedule-2026-10-27/README.md) records worker scheduling, ring costs, task overhead and workspace allocation. All four candidate/DUCC cells remain **GAP**.

The scoped candidate also screens adjacent full-mmax f32 scalar analysis and spin-1 analysis/synthesis at q1/q8, plus f64 T720 scalar and protected T1023 spin-1 synthesis. All parent/candidate outputs were bitwise equal; those controls are in the same report. The broader 16-cell f32 matrix still contains source-dated gaps from the `98adf1e6` baseline; no full/reduced spin-1 synthesis parity is claimed. T720 error measurements are recorded, but no new tolerance was set.

The existing Phase B GL T720 analysis profile remains applicable: the diff from source `26941cf4` to `98adf1e6` contains no `src/`, `Cargo.toml`, or `Cargo.lock` changes. Its q1/q8 Callgrind totals were 1.4899×/1.4990× DUCC Ir; the reflected fused recurrence and accumulation dominate. This is instruction attribution, not cycles or a timing claim. See the [profile](#phase-b-profile--current-source-feb46e5f).

## Current analysis-path candidate screen — 2026-10-27

The cached-proof post-recurrence scan candidate (ledger B44) was not promoted. Candidate and parent outputs matched byte-for-byte; the focused spin-1 test suite passed (49 passed, 4 ignored). GL T720 full spin-1 analysis paired C/P was `0.98331 [0.97886,0.99329]` q1 and `1.01836 [0.97894,1.09460]` q8 on the alternate core set. On the first set the medians were `0.99202` q1 and `0.94441` q8, with wide uncertainty and elevated q8 scheduler wait. The direction changed across core sets, so no stable q1/q8 gain is established. The current 1.249×/1.298× DUCC analysis gaps remain open; production source is unchanged. See the [screen report](data/ducc-parity-m6-2026-10-27/analysis-range-proof-scan-2026-10-27/README.md), [machine-readable results](data/ducc-parity-m6-2026-10-27/analysis-range-proof-scan-2026-10-27/screen-results.json), and [candidate patch](data/ducc-parity-m6-2026-10-27/analysis-range-proof-scan-2026-10-27/candidate.patch.gz).

## Historical first screen at source `26941cf4`

The section below preserves the initial M6 baseline and the subsequent experiment history. Its performance rows are historical and do not replace the source-specific per-cell entries in the current M6 ledger above.

**Source:** `26941cf45564a9637b72b0eca87ff880472c3505` on `dev`<br>
**DUCC:** 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`<br>
**Host:** AMD Ryzen 9 5950X, Linux x86-64; rustc/Cargo 1.99.1, LLVM 23.1.1; GCC 15.2.0.

This first M6 screen covers 14 selected warm whole-transform cells: GL T720 spin-1 analysis, protected T1023 synthesis controls, two reduced synthesis routes, and native-f32 GL T720 scalar/spin-1 analysis and synthesis. The rest of the agreed matrix remains unmeasured. No production algorithm changed during this screen.

### Method

Both Rust workers were built in release mode with `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; the f32 worker reported Pulp V3 and eight f32 lanes. DUCC numerical sources used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; both C++ drivers used `-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3`. The pinned DUCC tree was clean at the exact source SHA. Binary and fixture SHA-256 values are recorded in the [per-cell CSV](data/ducc-parity-m6-2026-10-27.csv).

Each cell used three interleaved fresh-process pairs with shared deterministic fixtures, matching geometry, normalization, precision, and output dimensions. Timing covers warm calls using reusable Rust plans/workspaces and direct DUCC APIs; setup, fixture I/O, and output checks are outside the timed batches. The f64 worker targeted 25 ms and the f32 worker 10 ms per batch; each used ten batches for calls below 100 ms and five otherwise. q1 and q8 are thread counts. Workers pinned q1 to CPU 0 and q8 to eight distinct physical cores. f64 q8 used CPUs 0–7; f32 q8 used 0,2,4,6,8,10,12,14.

Three pairs give a triage range, not a confidence interval. The ledger retains each pair's Rust and DUCC medians, ratio, correctness data, fixture hashes, binary hashes, scheduler wait, and frequency samples. f64 comparisons passed finite, repeat, and reset checks. f32 comparisons passed finite, repeat, reset, and output-length checks; the f32 harness records DUCC-relative errors without enforcing a tolerance. Its broader independent-reference, adjoint, and exceptional-value gate remains open.

### Historical f64 cells

| Workload | Rust ms | DUCC ms | Rust/DUCC | Three-pair range | Status | Max relative L2 |
|---|---:|---:|---:|---:|---|---:|
| GL T720 full spin-1 analysis q1 | 36.724 | 29.173 | 1.263× | 1.244–1.264× | **GAP** | 4.76e-14 |
| GL T720 full spin-1 analysis q8 | 5.285 | 4.244 | 1.236× | 1.232–1.249× | **GAP** | 4.76e-14 |
| GL T1023 full spin-1 synthesis q1 | 88.272 | 119.637 | 0.737× | 0.736–0.740× | **PASS** | 2.60e-13 |
| GL T1023 full spin-1 synthesis q8 | 14.072 | 19.737 | 0.719× | 0.704–0.720× | **PASS** | 2.60e-13 |
| CC T720 reduced m180 spin-1 synthesis q1 | 30.006 | 18.593 | 1.616× | 1.591–1.658× | **GAP** | 3.62e-13 |
| GL T720 reduced m180 over-resolved synthesis q8, ntheta=1441 | 4.255 | 4.647 | 0.916× | 0.913–0.920× | **PASS** | 2.27e-13 |

The T720 analysis gap is present at both thread counts. Rust q1 scheduler wait was 6.5–6.9% across its three analysis runs, while DUCC wait was low; all paired ratios exceeded 1.23. The CC reduced q1 Rust runs also had about 10% wait, and all three ratios remained above 1.59. The q8 analysis runs had low wait. The T1023 synthesis controls preserve the established speed advantage.

### Historical native-f32 cells

All cells use GL T720 full `mmax`, `ntheta=721`, `nphi=1458`.

| Transform | q | Rust ms | DUCC ms | Rust/DUCC | Three-pair range | Max relative L2 | Status |
|---|---:|---:|---:|---:|---:|---:|---|
| Scalar analysis | 1 | 591.691 | 13.922 | 42.515× | 42.398–42.606× | 1.83e-5 | **GAP** |
| Scalar analysis | 8 | 77.904 | 3.434 | 22.139× | 17.287–27.916× | 1.83e-5 | **GAP** |
| Scalar synthesis | 1 | 873.293 | 19.420 | 44.881× | 44.424–45.501× | 1.72e-4 | **GAP** |
| Scalar synthesis | 8 | 339.733 | 3.748 | 91.134× | 65.084–118.232× | 1.72e-4 | **GAP** |
| Spin-1 analysis | 1 | 1084.978 | 28.811 | 37.530× | 37.402–37.740× | 1.57e-5 | **GAP** |
| Spin-1 analysis | 8 | 141.391 | 4.420 | 32.144× | 29.616–32.463× | 1.57e-5 | **GAP** |
| Spin-1 synthesis | 1 | 1535.415 | 40.581 | 37.715× | 36.713–38.063× | 1.43e-4 | **GAP** |
| Spin-1 synthesis | 8 | 279.188 | 6.063 | 46.628× | 41.908–46.691× | 1.43e-4 | **GAP** |

The q1 f32 runs had low scheduler wait. q8 scalar analysis/synthesis saw median wait of about 14.6%/10.5%; spin-1 q8 wait was lower but variable. Those conditions widen the q8 ratios, while every observed f32 ratio remains at least 17× DUCC. The timing classification is a clear **GAP**; the dedicated f32 numerical acceptance gate remains pending.

## Phase B profile — current source `feb46e5f`

The release worker uses the same GL T720/full-mmax geometry and seeded finite maps as the M6 harness. Source files in the profile worktree match production source at `26941cf4`; only test-gated work counters were added for the separate work-count helper. Rust used rustc/Cargo 1.99.1, LLVM 23.1.1, release debug symbols, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, and Pulp V3. Pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` used GCC 15.2 with `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; its driver added `-fno-fast-math -ffp-contract=fast`. Valgrind/Callgrind was 3.26.0. Each worker warmed three times and then measured one transform between Callgrind START/STOP controls; setup and output I/O were outside the measured region. Callgrind reported a `brk segment overflow` warning but completed all four profiles. `perf` was unavailable at `perf_event_paranoid=4`; system settings were left unchanged.

| q | Rust whole-call Ir | DUCC whole-call Ir | Rust/DUCC Ir | Rust fused pair kernel inclusive Ir | DUCC `calc_map2alm` inclusive Ir |
|---:|---:|---:|---:|---:|---:|
| 1 | 639,742,202 | 429,390,692 | 1.4899× | 260,221,946 | 227,226,660 |
| 8 | 643,881,097 | 429,534,467 | 1.4990× | 260,221,946 | 227,226,660 |

These are instruction counts, not cycles. Inclusive rows overlap their callees and must not be summed. Rust's `analyze_spin_scalar_reflected_order` stage is 534,633,890 Ir at q1 and 534,626,783 at q8 (about 83% of each whole call); it calls the fused pair-block route 3,579 times. The hot `FusedScalarAnalysisPair<true,false,false,true,true>` specialization is called 2,971 times. Its inclusive Pulp vectorization cost is 260.22M Ir, while direct source attribution is 106.61M Ir. The separate checked-scaled path runs 577 `<false>` calls and 22 `<true>` calls; its `<false>` vectorization source attribution is 32,682,944 Ir (5.1% of whole-call Ir). Do not add these nested inclusive and source-attributed values.

The executed AVX2 recurrence body at Rust PC `0x1d3212` ran 3,996,655 vector iterations at q1 and q8. Each iteration has 12 `vfmadd*` operations, six `vmovupd` loads, two vector stores and loop control; no vector spills appear in that loop. The separate U-channel accumulation body at `0x1d3ab5` ran 1,849,862 iterations, with eight `vfmadd*` operations and about ten vector loads; it also has no loop-body spills. DUCC `map2alm_kernel` at `0x67cab0` ran 9,934,482 vector iterations in each profile. Its paired-degree body has 12 FMA instructions, memory operands in the FMAs, recurrence stores, and loop control. The different batch/lane layouts make iteration totals a trace of executed work, not a standalone speed comparison.

Thread-safe test-only geometry counters gave the same q1/q8 totals: 387,673 active ring/order pairs; 81,083,602 reflected pair-lane degree states; 162,167,204 Q/U component-channel accumulations; 3,579 fused blocks; zero generic fallback or decline; and 14,316 safety scans over 813,340 lane inputs. The separately derived DUCC recurrence work is 9,934,482 vector iterations × 4 AVX2 lanes × 2 degree-state updates = 79,475,856 executed ringdata degree-state updates. Rust's pair-lane state formula and DUCC's ringdata update formula are close in cardinality, but their accumulation definitions differ; they are reported separately rather than treated as identical arithmetic. The geometry helper used deterministic random finite maps, while the Callgrind worker used seed `1416973757`; route totals matched, but the helper is not timing evidence.

The Rust forward FFT is 34,545,994 Ir and spin conversion is 11,455,960 Ir. DUCC's real FFT is 60,264,064 Ir, `spin0to1` is 16,407,759 Ir, and `fill_m2a` is 13,912,862 Ir. These stages are not the leading Rust/DUCC instruction-count excess. Rust/DUCC worker binaries were SHA-256 `59ed4481bec7fbed4e339f0562f457e9b9965d802c35df26d6ec735856136f48` / `f48064f89dae19e932f22ae92f880f8edc399d14b56ce6af17e4d977fd7c2d7f`; the profiled map inputs were `e3379278cb0dfc9a9f4d5f2c8ece40995f965bd727dcb4ce884b85fc62d7abba` / `dc3a132282b074d16cea41cf7305e64e796a48beac398221776ee8c7ad74d5cc`. Callgrind profile SHA-256 values, in q1 Rust/DUCC then q8 Rust/DUCC order, are `092a91da4a4a8bad9e8d17f0b79a27d975083566f1988230f299379921e220e`, `4110614f0e0602ac8b80ec8c23cfcddff8ba016c840be2b1e63f72abd09e86f2`, `052e0fafaec957a96ab03404349ee4eab60a6eee7a9b92dae045004d74f26fec`, and `933285ef080ef42e1a46ae845ee186ce64d498c936fd8c111d9c1425e4c4708d`. Compact values and definitions are in the [profile CSV](data/ducc-parity-m6-2026-10-27-profile.csv).

### Ranked hypotheses and selected experiment

| Rank | Exact route and evidence | Removable fraction and confidence | Decision |
|---:|---|---|---|
| 1 | Rust fused pair vector loop (`scalar_paired/analysis.rs`, hot PC `0x1d3212`) vs DUCC `map2alm_kernel` (`sht_inner_loop.h`, `0x67cab0`). Rust spends 36,024,856 Ir in `core::iter::zip` attribution inside the fused specialization (5.6% of whole-call Ir); the executed AVX2 loop has six explicit vector loads and two stores around the 12 FMAs, while DUCC uses memory operands in its FMA sequence. Their source-defined pair-lane work counts are similar. | At most 5.6% whole-call Ir is attributed to zip, and only part may be loop machinery; likely 0–5.6% Ir, low-to-medium confidence for elapsed time. | **Screened and removed:** the 15-pair indexed-loop attempt below had no supported q1/q8 speedup; keep the checked iterator unless new codegen evidence changes the trade-off. |
| 2 | Checked-scaled `FusedCheckedScaledAnalysisPairChannels` prefix (`0x1d4ba0` family), 599 calls across false/true specializations; false specialization has 32.68M exclusive source-attributed Ir. | Exclusive source attribution is 5.1% of whole-call Ir; inclusive path cost is larger but overlaps children. The route is polar/subnormal safety work, so practical removable fraction is lower; medium-low confidence. | **Screened and removed:** prefix/update fusion was slower in all three q1 and q8 triplets (paired median C/P 1.046406/1.130237); retain checked fallback and range behavior. |
| 3 | Reflected-order wrapper, preparation and safety scans. Four per-block safety passes total 14,316 scans / 813,340 lane inputs; Rust FFT and spin conversion are already below DUCC's corresponding Ir totals. | The safety scans are about 1–2% of the whole call; low expected return. | Defer; avoid broad scan fusion and preserve transactional fallback. |

This is a bounded causal profile, not a parity claim: the earlier measured q1/q8 Rust/DUCC runtime ratios were 1.263×/1.236× in the triage ledger. The indexed-loop trial below does not establish a performance improvement or parity.

### q1 source-line attribution follow-up — 2026-10-27

A fresh Rust-only Callgrind run used the exact current production source from base commit `1b875a6bb95d5c0a4ca3cc23d82185d9c1d3990b`; the relevant production files match the profiled-source commit `26941cf45564a9637b72b0eca87ff880472c3505`. The saved zero-context instrumentation patch is `ef6991cc0f736971642f0c7f34cdf28a13287a813bdf0a0d199cc8ee826d3c92`. GL T720/full-mmax spin-1 analysis ran at q1 with `ntheta=721`, `nphi=1458`, `phi0=0.23`, rustc/Cargo 1.99.1, LLVM 23.1.1, release debug symbols, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, Pulp V3, and Callgrind 3.26.0. Deterministic Python `random.Random(1416973757)` inputs are hashed in the linked CSV. The profiled call produced bitwise-identical output to the initial call and two warm calls. Callgrind collected `639,748,794` Ir, within 0.002% of the prior q1 profile. It emitted the known `brk segment overflow` warning but exited successfully. This is source attribution, not elapsed-time evidence or a DUCC comparison.

The `analyze_spin_order_group` call accounts for `534,888,388` inclusive Ir (83.61% of the whole call). The call site for `try_fused_scalar_analysis_pair_shared_recurrence` has `408,167,451` inclusive Ir across 3,579 calls (63.80%); the hot `FusedScalarAnalysisPair<true,false,false,true,true>` Pulp vectorizer has `106,607,678` directly attributed Ir (16.66%). The `store_analysis_pair` source assignments account for `19,383,051` Ir total (3.03%), and sectoral initialization accounts for `10,210,528` inclusive Ir (1.60%). These rows overlap and must not be summed. The source attribution does not show a larger low-risk setup target after the three no-gain screens. No candidate was selected from this profile; the M6 gap remains open. Next work should target a concrete reduction in the core fused recurrence/accumulation instruction count, with exact-parent q1/q8 measurements.

See the [source-line attribution report](data/ducc-parity-m6-2026-10-27/source-line-attribution-q1-2026-10-27.md) and [machine-readable counts](data/ducc-parity-m6-2026-10-27/source-line-attribution-q1-2026-10-27.csv).

## Phase C indexed-loop screen — candidate removed

The experiment changed only the `ROTATE_SEQUENTIAL_STATES` SIMD-vector loop in `fused_analysis_two_pair_steps_with_simd`: it replaced the nested zipped iterators with an integer index and `get_unchecked(_mut)` after the existing entry checks establish equal vector-slice lengths. The recurrence expressions and update order stayed unchanged; the non-rotating loop was untouched. Candidate patch SHA-256 is `1c5a867b641dec243b3754d03a69d8964123d675e8ce3f1ad43609e7f588cce0`; exact parent source was `1af82adb5190264e89586219c50d61e30b223822`. The patch was reverted after measurement and is preserved with the runner and full results in the [experiment evidence folder](data/ducc-parity-m6-2026-10-27/README.md).

Parent, candidate, and pinned DUCC were built in separate scratch targets. Rust used Cargo/rustc 1.99.1, LLVM 23.1.1, release mode, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; the measured worker uses runtime Pulp dispatch, which it does not print. DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` used GCC 15.2, numerical-source `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, and a driver with `-fno-fast-math -ffp-contract=fast`. The case was GL, spin 1 analysis, T720/full-mmax, `ntheta=721`, `nphi=1458`, `phi0=0.23`, seeded finite inputs (seed `1416973757`; map hashes match the Phase B worker). Each process warmed and checked repeat/reset, then recorded ten warm batches targeting 25 ms. Fifteen balanced interleaved parent/candidate/DUCC triplets ran at each of q1 and q8 on the same physical-core affinity used by the checked benchmark harness.

| q | Parent / candidate / DUCC median of process medians (ms) | Paired C/P ratio, median [bootstrap 95% CI] | Faster C/P pairs | Parent/DUCC; candidate/DUCC paired-ratio medians |
|---:|---:|---:|---:|---:|
| 1 | 36.153 / 35.892 / 29.304 | 0.993778 [0.991391, 1.001167] | 11/15 | 1.241058; 1.225888 |
| 8 | 5.293 / 5.307 / 4.224 | 1.001789 [0.999099, 1.011079] | 6/15 | 1.254247; 1.255433 |

Candidate and parent outputs were bitwise identical in all 30 triplets; all three backends were finite and passed repeat/reset checks. Parent and candidate both had maximum DUCC relative-L2 error `4.760590e-14` and maximum absolute error `1.295838e-15`. Timing uncertainty includes substantial q1 runqueue wait (parent/candidate medians about 6.7%/6.8%) and q8 frequency samples reaching 1.746 GHz; no sample was dropped. The q1 interval includes 1.0, q8 is neutral-to-slightly-slower, and the q8 pair spread is wide. The 5.6% zip source attribution from Phase B was only an upper bound; no candidate-specific whole-call Ir reduction was measured. **Decision:** remove the unsafe indexed variant; the screen does not support a material or repeatable f64 gain, while retaining unchecked accesses would add maintenance and proof burden. This is not a parity result.

Candidate validation passed `cargo test --locked --all-features --lib legendre::scalar_paired::analysis` (34 passed, 8 ignored), `cargo test --locked --all-features --lib spin1` (49 passed, 4 ignored), and `cargo fmt --all -- --check`. These focused cases include paired recurrence references, GL/CC/F1 fallback paths, poisoned workspace reuse, q1/q8 recurrence checks, and spin-1 adjoint identities. The full workspace suite and warning-denied Clippy were not run for this rejected variant. Per-triplet process-median timings, summarized wait/frequency data, binary/input/output hashes, exact runner provenance and recoverable source diff are linked from the [evidence folder](data/ducc-parity-m6-2026-10-27/README.md).


## Phase C follow-up — shared-cosine safety-scan candidate removed

The candidate patch combines Q/U input and post-compaction safety scans while reusing cosine validation; checked-scaled recurrence, range caching, and fallback routing are unchanged. It was applied in an isolated worktree based on d3aeb208222d4142217f4c59b83f712b0945d4a6, whose production source matches 469f175a905df34b4514a9f21db391a424adbe15. The patch SHA-256 is c71b5e36e5c953dbfbfcafdc389226ab045d242292cd17dafb76318093757071. The candidate was not applied to production.

Three balanced parent/candidate/DUCC triplets ran at each thread count on GL T720 full-mmax spin-1 analysis. The case used ntheta=721, nphi=1458, phi0=0.23, seed 1416973757, pinned DUCC 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b, rustc 1.99.1/LLVM 23.1.1, and GCC 15.2.0. Each worker warmed and recorded ten 25 ms target batches. Paired median C/P ratios were 0.999166 q1 and 0.998194 q8 (2/3 candidate-faster pairs for each). Median parent/candidate/DUCC process times were 36.411/36.380/29.463 ms q1 and 5.340/5.230/4.235 ms q8. Parent/candidate outputs were bitwise equal in all six triplets; outputs were finite, and repeat/reset checks passed. Both Rust versions had max DUCC relative-L2 error 4.760590e-14 and max absolute error 1.295838e-15.

The screen does not support a repeatable gain. q1 parent/candidate wait was 6.5–6.9%; at q8, one parent sample had 2.46% wait versus 0.55% for its candidate, while the other two pairs were near parity. Candidate/DUCC paired-ratio medians were 1.239101 q1 and 1.234792 q8. The production M6 gap remains open. Candidate focused tests passed before timing (paired analysis 35 passed/8 ignored; spin-1 49 passed/4 ignored; format and diff checks); no expanded suite or warning-denied Clippy was run for this rejected variant.

**Decision:** keep the checked path and do not extend this candidate. Per-pair batch samples, diagnostics, hashes, corrected runner provenance, and the recoverable patch are linked in the [evidence index](data/ducc-parity-m6-2026-10-27/README.md). The [screen report](data/ducc-parity-m6-2026-10-27/pair-safety-screen-2026-10-27.md) is the concise result record.

## Phase C follow-up — reflected active-lane mask screened and removed

The candidate records active reflected-latitude lanes during the staging pass and reuses a 64-bit mask during block compaction, removing the second pass's north/south ring-limit reloads and predicate checks. The exact parent and candidate base are `914e68fbdf4385b3a7e74304e2bcdc77cfd6e4c1`; candidate patch SHA-256 is `a2a399159a825272db5b04cad6d3e41fdd0fe93f7a478cf13edd4dceb47ffa04`. It changes no recurrence arithmetic, checks, or fallback contract and was not applied to production.

Three interleaved parent/candidate/DUCC triplets ran at each q on GL T720 full-mmax spin-1 analysis with the M6 fixture and pinned DUCC 0.42.0. Paired median C/P was `0.992072` q1 (2/3 candidate-faster pairs) and `1.008278` q8 (0/3 candidate-faster pairs). Median process times were 36.438/36.418/29.075 ms q1 and 5.236/5.293/4.201 ms q8. Parent/candidate outputs were bitwise identical in all six triplets; all finite, repeat/reset checks passed, and maximum DUCC relative-L2 error was `4.760590e-14` for both. q1 had 6.6–6.9% runqueue wait; q8 frequency samples included 1.746 GHz readings. The small q1 change did not repeat at q8, so the candidate was removed without extending the screen. Focused validation passed (49 spin tests, 4 ignored; format and diff checks). The [screen report](data/ducc-parity-m6-2026-10-27/active-mask-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md) contain the full results and exact patch. M6 parity remains open; next, inspect a distinct profiled cost.

## Phase C follow-up — paired preweight setup fusion screened and removed

The candidate replaced four stack-array copies, separate Q and U opposite-channel preweight/safety traversals, and the `cos²` traversal with one per-lane setup loop in `try_fused_scalar_analysis_pair_shared_recurrence`. Weighted values and recurrence coefficients remain local; unsafe products still decline before staged output commit. Exact parent/candidate base was `85fd4998827619887c6d1642330ceb614b014935`; measured patch SHA-256 was `a232afd292df1b9acc822fc93f301f42e34afcaa2e4e83b05cc88def12a9097c`.

Three interleaved parent/candidate/DUCC triplets ran at q1/q8 on GL T720 full-mmax spin-1 analysis. Paired median C/P was `1.001462` q1 (1/3 candidate-faster) and `0.998948` q8 (2/3 candidate-faster). Median process times were 36.177/36.068/29.063 ms q1 and 5.248/5.217/4.230 ms q8. Outputs were bitwise identical and finite; repeat/reset passed; maximum DUCC relative-L2 error was `4.760590e-14` for both Rust versions. q1 had 6.0–6.9% wait, q8 had 0.6–1.2% wait and frequency readings at 1.746 GHz. This near-neutral split does not support a gain or material regression, so the candidate was removed without extending the screen. Focused paired-analysis and spin suites passed (34 + 49 tests; 12 ignored total), along with format/diff checks. The [screen report](data/ducc-parity-m6-2026-10-27/preweight-fusion-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md) contain the full results. M6 parity remains open.

## Phase C follow-up — checked-scaled prefix/update fusion screened and removed

The candidate fused the checked-scaled pair-channel vector accumulation with recurrence advancement in `FusedCheckedMixedAnalysisPairChannelsStep`. Scaled exception handling, scalar-tail work, and decline/fallback behavior remained in place. Exact parent was `67eb85151d0e91cca0fcc3bc6b89372f67e64dcd`; the candidate patch SHA-256 was `aa01403775a9113be444d693c659d825f673875f53e7a87b275c36734b95bed7`.

Three interleaved parent/candidate/DUCC triplets ran per q on GL T720 full-mmax f64 spin-1 analysis. Median process times (parent/candidate/DUCC) were 36.164/37.916/29.139 ms q1 and 5.391/5.963/4.288 ms q8. Median paired candidate/parent ratios were `1.046406` q1 (3/3 candidate-slower) and `1.130237` q8 (3/3 candidate-slower). q1 scheduler wait was balanced between parent and candidate; one q8 candidate process had 3.50% runqueue wait. Treat the three-pair q8 direction cautiously. The candidate remained 1.294× and 1.372× DUCC by median paired ratio, so this screen does not close the M6 gap.

Candidate and parent outputs were bitwise identical in all six triplets, finite, and repeat/reset consistent. Candidate versus DUCC relative-L2 error was `4.760590e-14` and maximum absolute error `1.295838e-15`. Focused candidate validation passed (paired-analysis 34 passed/8 ignored; spin-1 49 passed/4 ignored; format check). The candidate was not applied to production and was not extended. Exact timings, host diagnostics, hashes, and the recoverable patch are in the [screen report](data/ducc-parity-m6-2026-10-27/checked-prefix-fusion-screen-2026-10-27.md) and [evidence folder](data/ducc-parity-m6-2026-10-27/README.md). The subsequent two-vector pipeline screen is recorded below; neither candidate was applied to production.

## Phase C follow-up — two-vector recurrence pipeline screened and removed

The candidate interleaved two independent SIMD-latitude vectors through the same two-degree recurrence. It preserved each output accumulator's vector order and the exact per-lane arithmetic; only the hot paired-state path was specialized. Exact parent was `30a17ee82154d761ccbbb40a256e27a97d3c2946`; patch SHA-256 was `f786de45bb2e09854146ad1259324e89619b4278b0fdb423351802f8e69f18a6`.

Three interleaved parent/candidate/DUCC triplets ran per q on GL T720 full-mmax f64 spin-1 analysis. Median process times (parent/candidate/DUCC) were 36.299/39.869/29.307 ms q1 and 5.266/5.679/4.216 ms q8. Median paired candidate/parent ratios were `1.105232` q1 (3/3 candidate-slower) and `1.077169` q8 (3/3 candidate-slower). q1 parent/candidate scheduler wait was similar at 6.4–6.9%; q8 wait was low in five workers, with one candidate at 1.42%. The candidate remained 1.356× and 1.342× DUCC by median paired ratio; this did not improve the measured cell.

Parent/candidate outputs were bitwise identical in all six triplets, finite, and repeat/reset consistent. Candidate versus DUCC relative-L2 error was `4.760590e-14` and maximum absolute error `1.295838e-15`. Focused paired-analysis tests (34 passed/8 ignored), spin-1 tests (49 passed/4 ignored), and format check passed. The candidate was not applied to production and was not extended. Exact timings, summary diagnostics, hashes, and recoverable source patch are in the [screen report](data/ducc-parity-m6-2026-10-27/two-vector-pipeline-screen-2026-10-27.md) and [evidence folder](data/ducc-parity-m6-2026-10-27/README.md).

## CC T720 reduced synthesis q1 route profile — 2026-10-27

The current q1 M6 cell is CC `lmax=720`, `mmax=180`, `ntheta=722`, `nphi=378`, spin 1: Rust 30.006 ms versus pinned DUCC 18.593 ms (1.616×; three-pair range 1.591–1.658×). The Rust runs had about 10% scheduler wait, so this remains a gap rather than a quiet-host performance estimate.

The existing ignored `profile_spin1_standard_synthesis_phases` helper was run at exact source `f2f813fd51c58b52a5991e666c92d83b3286bf7f` with release `x86-64-v3`, one Rayon thread, `phi0=0.23`, three warmups and three profiled calls. It generated deterministic coefficients internally; this is route evidence, not a matched benchmark or an independent numerical check. The targeted helper passed. The test binary SHA-256 is `6bde446901d25c045c09f3e050e68b565144436e0487d7b2bb2f05b358143ba7`; `src/transform/spin/synthesis.rs` SHA-256 is `8ef50c59d134f8c816793d8567198ff4120919503f9c243a8984ce6d13fef4e1`.

| Repetition | Total ms | Latitude/setup ms | Order recurrence and spectral accumulation ms | Two inverse FFTs and map writeback ms |
|---:|---:|---:|---:|---:|
| 0 | 28.191 | 0.068 | 27.048 | 1.055 |
| 1 | 29.344 | 0.066 | 28.218 | 1.029 |
| 2 | 28.039 | 0.074 | 26.967 | 0.968 |

The internal worker counters report 1,784 pair-eligible blocks and 1,736 ordinary fused-kernel successes per call. There are 305 fallback entries: 191 transition successes and 114 per-degree blocks; those 114 also report successful polar splits. Worker-sum timers put the fused recurrence/accumulation at 11.60–12.07 ms and scaled/per-degree fallback at 10.50–11.19 ms. They are nested diagnostics and should not be added to the phase timings. Order-factor fill, paired coefficient preparation, and finite scans are each smaller in these counters. No scalar coefficient cache or order-major scratch is allocated on this serial route.

The order recurrence and accumulation phase dominates this profile; inverse longitude FFTs and writeback are about 1 ms together. This does not support CC resampling or FFT tuning as the next cause. It points to a new synthesis recurrence/dataflow hypothesis. Do not retry the reduced reflected scalar component route unchanged: its earlier reduced-order T720 version failed an existing test, and its full-order CC q8 screen had no repeatable gain. The compact per-call counters, command, toolchain and limits are in the [route-profile report](data/ducc-parity-m6-2026-10-27/cc-t720-reduced-q1-route-profile-2026-10-27.md) and [CSV](data/ducc-parity-m6-2026-10-27/cc-t720-reduced-q1-route-profile.csv).

## Phase C follow-up — forced native-spin analysis route screened and removed

The candidate disabled the GL scalar fast-range selection so GL T720 full-mmax spin-1 analysis used the existing direct native-spin recurrence. The exact parent and candidate base were `f70d9845edf6bba617b9d034b1f5e1d628c429e0`; candidate patch SHA-256 is `8e0d496aac44c14b368d239586d75486c0728a7ed2e00f4adc47ad1385b5d072`. The candidate changed no production files and kept every numerical, workspace, reset, and extreme-input assertion; the route-specific cache expectation was adjusted only in the isolated test worktree.

Three interleaved parent/candidate/DUCC triplets ran per q. Median parent/candidate/DUCC times were **35.946/927.170/29.122 ms q1** and **5.285/121.786/4.227 ms q8**. The direct route was about 25.8×/23.0× slower than the current route. Candidate/DUCC relative-L2 error was `5.9393e-14`, maximum absolute error `1.7168e-15`; finite and repeat/reset checks passed. Focused spin-1 validation passed (49 passed, 4 ignored), plus formatting and diff checks. The benchmark harness's inherited final bitwise-parent condition caused its exit code 2 because this route is not bitwise-identical; all 18 backend workers completed and their numeric checks are recorded. The direct route is rejected for this cell. Exact timings, output hashes, and the recoverable patch are in the [screen report](data/ducc-parity-m6-2026-10-27/native-spin-route-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md). Cleanup removed only its exact owned scratch and two worktrees; available space increased by 941,285,376 bytes and free inodes by 10,408.

## Phase C follow-up — shared phase-scale and FMA rotation screened and removed

The candidate combines the per-ring scale with the shared phase factor for Q and U, then applies explicit FMA rotation in `analyze_spin_scalar_reflected_order`. Exact parent was `f70d9845edf6bba617b9d034b1f5e1d628c429e0`; patch SHA-256 is `062113589c586100665fd4b60776fe1de4609ece9f29e29638a4743a9aaba139`. It affects only spectrum phase/scale arithmetic; recurrence, input safety checks, transactional fallback, and coefficient staging are unchanged. Candidate validation passed 49 spin-1 tests (4 ignored), format check, and diff check.

Fifteen balanced parent/candidate/DUCC triplets per q ran on GL T720 full-mmax spin-1 analysis. Median parent/candidate/DUCC process times were **36.325/36.109/29.309 ms q1** and **5.342/5.330/4.259 ms q8**. Paired candidate/parent medians were **0.995288 [0.984092, 1.007003] q1** and **0.998563 [0.976639, 1.024761] q8**; the intervals include parity and only 11/15 q1 and 8/15 q8 pairs favored the candidate. Candidate-parent relative-L2 error was `1.07e-15`, max absolute error `4.17e-17`; candidate/DUCC relative-L2 remained `4.76e-14`, max absolute `1.30e-15`. q1 wait was elevated but balanced; q8 variation was wide. **Decision:** reject as inconclusive and do not promote the extra f64 arithmetic path. Rust remains about 1.23× DUCC on this cell. Full per-triplet diagnostics and patch are in the [screen report](data/ducc-parity-m6-2026-10-27/phase-scale-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md).

The next measured gap remains CC T720 reduced-mmax spin-1 synthesis q1. Its route profile shows recurrence and accumulation dominate, not the inverse FFTs/writeback; next source work should target a distinct synthesis dataflow mechanism and preserve the measured analysis controls. M6 remains active.


## Phase C follow-up — fuse inverse-sine scaling into paired synthesis SIMD; rejected

The isolated candidate wrote per-lane inverse-sine values into the prepared kernel's no-longer-used `previous` scratch, scaled all four Q/U recurrence sums in Pulp SIMD, and removed the fused branch's later per-ring spectrum-scale loop. The recurrence and FMA order were unchanged. Exact parent was `f70d9845edf6bba617b9d034b1f5e1d628c429e0`; candidate patch SHA-256 is `e0725761add848c53cf4032d0b68f3865aa6141703b9758cda5e338d78e4c6fe`. It was never applied to production.

Three balanced parent/candidate/pinned-DUCC triplets per q ran on exact CC T720 reduced-mmax spin-1 synthesis (`mmax=180`, `ntheta=722`, `nphi=378`, `phi0=0.23`). Median parent/candidate/DUCC process times were 30.623/31.429/20.664 ms q1 and 5.704/5.479/3.016 ms q8. Paired C/P medians were `1.026317` q1 (all three slower, range 1.024277–1.034464) and `0.977610` q8 (2/3 faster, range 0.877852–1.002711). q1 parent/candidate median wait was 9.75%/9.84%; q8 was noisy, with a candidate-first outlier. The screen does not establish a new DUCC ratio.

Parent and candidate outputs were bitwise equal in all six triplets; all 18 workers were finite and passed repeat/reset. Candidate-parent relative-L2 and max absolute error were zero; candidate-DUCC max relative-L2 was `3.619e-13` and max absolute error `3.025e-9`. Focused candidate validation passed the spin-1 suite (49 passed, 4 ignored), format and diff checks. **Decision:** reject the SIMD post-scale; the q1 slowdown is repeated and q8 does not establish a compensating gain. Keep the current source. Full measurements, diagnostics, hashes, and cleanup are in the [screen report](data/ducc-parity-m6-2026-10-27/simd-scale-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md).

M6 remains active. Next: test whether the prepared pair kernel can write directly to its cleared per-order Q/U slots, removing the current scratch-to-spectrum read/add step without changing recurrence arithmetic. Preserve the CC q1 fallback and GL f64 synthesis controls.


## Phase C follow-up — direct writes to cleared paired-synthesis spectra; rejected

The isolated candidate replaced the prepared pair kernel's Q/U `Complex64 +=` with direct stores of newly computed sums. Its caller clears spectrum storage before each transform and owns each order/ring slot once. The store keeps `0.0 + sum` to preserve signed-zero behavior. Exact parent was `f70d9845edf6bba617b9d034b1f5e1d628c429e0`; patch SHA-256 is `de5863798bf82495d2a235c77ea4d9e7e75682019e26c57b8e5383d2d888a037`. It was not applied to production.

Fifteen balanced parent/candidate/DUCC triplets per q ran on CC T720 reduced-mmax spin-1 synthesis (`mmax=180`, `ntheta=722`, `nphi=378`, `phi0=0.23`). Paired C/P medians were `0.997470` q1 (95% bootstrap interval `[0.992019,1.002035]`, 9/15 faster) and `1.008259` q8 (`[0.982173,1.010737]`, 5/15 faster). Median parent/candidate/DUCC process times were 27.823/27.796/18.559 ms q1 and 4.426/4.436/2.657 ms q8. Parent and candidate were bitwise equal in all 30 triplets; all 90 workers passed finite, repeat, and reset checks. Candidate-parent error was zero; candidate/DUCC maximum relative-L2 was `3.619e-13`, maximum absolute error `3.025e-9`. The small q1 lead is within the paired interval and does not repeat at q8. **Decision:** reject; no whole-transform gain is established. The exact data, runner, patch, and cleanup record are in the [screen report](data/ducc-parity-m6-2026-10-27/direct-store-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md).

M6 remains open. The prepared-kernel indexed-read candidate has been screened and rejected after a 0.12% q1 whole-call Ir reduction and q8 slowdowns in both controls. Next inspect the checked transition/fallback path using the [CC source attribution](data/ducc-parity-m6-2026-10-27/cc-t720-reduced-q1-source-attribution-2026-10-27.md), while retaining the CC q1/q8 and GL T1023 f64 synthesis controls.


## CC T720 reduced synthesis q1 source attribution — 2026-10-27

The exact parent `f70d9845edf6bba617b9d034b1f5e1d628c429e0` was profiled with Callgrind around one warmed CC T720 reduced-mmax spin-1 synthesis call. The source file `src/transform/spin/synthesis.rs` matches the SHA in the existing route profile. The run collected 613,272,869 Ir; the prepared fused pair path was 47.26% inclusive, with 10.06% directly attributed to the nested iterator zip implementation in its Pulp V3 vectorizer. Checked transition, polar-safe-lane, and per-degree fallback routes also have material inclusive counts. Inclusive counts overlap and are not elapsed-time estimates. No candidate or production change was made. The next hypothesis is indexed pair reads in `FusedPreparedSynthesisPairBlock::with_simd`, screened at CC q1/q8 and the GL T1023 f64 synthesis control. Full method, source hashes, raw profile and cleanup are in the [focused report](data/ducc-parity-m6-2026-10-27/cc-t720-reduced-q1-source-attribution-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md).


## Prepared paired-synthesis indexed traversal — rejected

The candidate changed only `FusedPreparedSynthesisPairBlock::with_simd`, replacing nested zip/enumerate traversal with ordered index reads for both SIMD vectors and the scalar tail. Parent `f70d9845` and candidate outputs were bitwise equal. Focused tests passed (49, 4 ignored); all 12 CC T720 reduced / GL T1023 full-mmax q1/q8 triplets passed finite, repeat, and reset checks. Candidate/DUCC maximum relative-L2 was `3.619e-13` on CC and `2.602e-13` on GL. Matched CC q1 Callgrind fell from 613,507,756 to 612,782,575 Ir, only 0.1182%; the candidate moved much of the zip source count into bounds/comparison code. q8 paired median C/P was `1.014293` on CC (0/3 faster) and `1.011036` on GL (1/3 faster). GL q1 had a small unconfirmed lead; all timing screens had n=3 and no confidence intervals. **Decision:** reject; no supported whole-transform gain, and the q8 results favor the parent. Full triplets, profiles, patch, hashes, and scratch cleanup are in the [screen report](data/ducc-parity-m6-2026-10-27/indexed-pair-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md).


## Prepared synthesis safety-check cache — screened and rejected

The isolated candidate memoized the prepared-pair coefficient range check across adjacent latitude blocks in one prepared run. It computes the result lazily only after the ordinary fused route declines, leaving successful fused blocks unchanged; the low-level transition API retains its direct check. Exact parent was `f70d9845edf6bba617b9d034b1f5e1d628c429e0`; candidate patch SHA-256 was `36553db57b80244ac39325f8049097fa5d9c405b8a9449412c8c3b6380f3c341`. It was never applied to production.

Three interleaved parent/candidate/DUCC triplets per q ran for CC T720 reduced-mmax synthesis and GL T1023 full-mmax synthesis. Parent/candidate outputs were bitwise identical across all 12 triplets; all 36 workers passed finite, repeat, and reset checks. Focused transition tests and the release all-features spin-1 suite passed (49 passed, 4 ignored). Candidate/DUCC maximum relative-L2 and maximum absolute error were `3.619e-13` / `3.025e-9` for CC and `2.602e-13` / `5.410e-9` for GL.

| Case | q | Median C/P | Median parent / candidate / DUCC time (ms) | Parent/candidate wait |
|---|---:|---:|---:|---:|
| CC T720 reduced synthesis | 1 | 1.001844 | 27.833 / 27.884 / 18.621 | 10.35% / 10.30% |
| CC T720 reduced synthesis | 8 | 1.000910 | 4.406 / 4.414 / 2.648 | 1.25% / 0.47% |
| GL T1023 full synthesis | 1 | 0.991833 | 88.352 / 87.818 / 119.909 | 2.09% / 2.07% |
| GL T1023 full synthesis | 8 | 1.000487 | 14.164 / 14.291 / 19.579 | 0.72% / 0.72% |

**Decision:** reject the extra cache plumbing. CC results were effectively neutral, and the small GL q1 lead did not repeat at q8; the q1 CC workers also had about 10% scheduler wait. These three-triplet screens have no confidence intervals. Full provenance, source hashes, triplet details, and cleanup are in the [screen report](data/ducc-parity-m6-2026-10-27/prepared-safety-cache-screen-2026-10-27.md) and [evidence index](data/ducc-parity-m6-2026-10-27/README.md).

**Historical checkpoint at source `102675a1`:** the four GL T720 f32 scalar-synthesis cells were all **GAP**, and the isolated route census found a potential suffix opportunity. The current source and candidate decision are the sections at the top of this ledger. M6 remains open; after the current f32 scalar-synthesis work reaches a material improvement or documented blocker, continue through f32 scalar analysis and spin-1 synthesis/analysis before resuming smaller f64 gaps. Keep per-cell statuses and exact source revisions in this ledger and the [CSV](data/ducc-parity-m6-2026-10-27.csv).
