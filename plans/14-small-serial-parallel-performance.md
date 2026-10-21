# Follow-on plan 14 — Small-transform latency and large parallel SHT parity

**Status: ACTIVE (2026-10-21), following Plan 11 closeout
`a37f74d68631ab5fd373a4097640895e9f0c0c23`.** Plan 11 is complete and
retained as historical evidence. Work continues on the same Luna session
and `opt/sht-performance-parity` branch. The transition refreshed the
historical reduced-`mmax` claims below, and the initial Plan 14 rebaseline
from final Plan 11 source is complete. Continue the staged, evidence-driven
work from the current q1/q8 findings below. Plans 12/13 remain future-only.

**One-line goal:** Identify and reduce the dominant, measured sources of (1) small **serial** SHT latency and (2) residual large **eight-thread** SHT latency relative to a pinned native DUCC implementation, without sacrificing numerical correctness, portability, or memory safety.

## 1. Boundaries and relationship to existing plans

- This plan covers reusable-plan, single-field CPU spherical harmonic transforms, principally **f64 scalar Gauss–Legendre STANDARD analysis and synthesis**. Record both operations separately. Add a small spin-1 control set only after a shared hotspot is identified; spin-1 kernel redesign is a separate task unless directly justified.
- **The T2047/T2048 prefix discontinuity is already fixed by `0bc5f153`.**
  The fixed DUCC `2^-60` threshold is retained, and q1/q8,
  high-order/polar, cancellation, exceptional-value and adjoint checks
  passed. Do not reopen its removed global latitude-count gate or repeat
  64/128 tile sweeps. Keep **256 reflected-pair tiles** as the production
  baseline. Plan 11's already-parallelized q8 order preparation
  (`dfed3ff7`/`3b32f2de`) is also accepted source baseline; use the
  **final post-Plan-11 commit**, not historical comparisons, for Plan 14.
- **Defer cache-topology-driven table/recurrence selection to Plan 12.** The existing operation-specific scalar table selector, T64 synthesis recurrence policy, finite-input fallback, and memory cap are the baseline, not errors to be simplified away. This plan may measure their effects but must not implement Plan 12's hardware discovery or autotuning.
- **Defer shared-Legendre multi-field batch kernels and batch API changes to Plan 13.** A spin-1 pair of components is not an independent field batch.
- No general FFT replacement, new dependency, broad API redesign, default fast-math, changed numerical conventions, or unbounded allocation is authorized by this plan. A bounded FFT improvement is possible **only after matching stage attribution proves it is responsible for a material share of the remaining whole-SHT gap**.
- The goal is performance attributable to an implementation difference, **not** a predetermined speedup percentage or a claim that Rust language overhead is inherently slower.

## 2. Grounded starting evidence — not final results

Final Plan 11 source and closeout (commit
`a37f74d68631ab5fd373a4097640895e9f0c0c23`, 2026-10-21) establish:

- At small bands T1–T32, historical matched warm serial scalar transforms had Rust/DUCC geometric-mean ratios of approximately **3.2× for analysis and 3.1× for synthesis**. This is evidence of a latency problem; it does **not** establish that FFT, dispatch, or any one per-call step causes it. The scalar compact Legendre table is **not eligible below T63**, so the T1–T32 excess must not be blamed on a table/recurrence switch without other evidence.
- **Final reduced-order q8 values after output pruning:** T720 m180
  **1.728 vs 2.002 ms (0.863×)**, T2047 m511 **18.739 vs 17.064 ms
  (1.098×)**, and T3072 m768 **59.266 vs 58.253 ms (1.017×)** (Rust vs
  DUCC). The earlier 1.228×/1.122× values predate output pruning and are
  superseded. Full-order q8 analysis in the same checkpoint was T720
  **3.063 vs 2.937 ms (1.043×)**, T2047 **42.907 vs 42.563 ms (1.008×)**,
  and T3072 **124.428 vs 117.119 ms (1.062×)**. Reduced T2047/T3072 paired
  ratio ranges overlap unity; q1 reduced-order values were T720
  **11.196 vs 13.910 ms**, T2047 **122.010 vs 119.737 ms**, and T3072
  **380.152 vs 384.761 ms**. These final Plan 11 values replace the
  pre-pruning table above; Plan 14 must remeasure its selected cases under
  its own interleaved baseline protocol.
- The final-source stage profile found Rust's forward input scan/copy,
  FFT and recombination did not account for an excess over DUCC `map2leg`;
  Rust q8 recurrence plus separate preparation was comparable with DUCC's
  combined `leg2alm` stage. The remaining reduced-order q8 whole-call
  differences were +1.675 ms at T2047 and +1.013 ms at T3072, with paired
  ranges overlapping unity. The finite-input scan is part of the numerical
  contract and remains. Plan 14 should investigate its selected small-q1
  and large-q8 total/stage costs without repeating the closed frontend work.
- DUCC's author notes that the *DUCC* implementation has multiple parallel regions with microsecond-scale overhead, which hurts small **parallel** transforms. This explains why Rust can occasionally win small q8 cases; **it does not explain why Rust is slower for small q1**.
- RustSHT's scalar analysis constructs order-major longitude spectra.
  Independent per-order **preparation is already parallel** for recurrence
  plans using Rayon and disjoint output rows; do not propose to parallelize
  it again. Legendre jobs use `Mutex<VecDeque>` one-order scheduling. A
  T720 q8 profile measured about `42 us` aggregate dequeue lock/wait time,
  while per-thread work stayed balanced. A batch-pop candidate did not
  produce a repeatable full-transform gain and is rejected; see the current
  q8 analysis scheduling results below.
- DUCC's `sht.cc` performs `map2leg` and `leg2alm_internal` with its own geometry/packing and `execDynamic` scheduling; `sht_inner_loop.h` uses `fill_m2a`, `iter_to_ieee`, and `map2alm_kernel`. DUCC's output layout can differ by synthesis/analysis and thread count (`allocate_leg`). Compare actual data movement and SIMD useful work, not just surface API names.
- At plan start, a fixed-size aligned scratch candidate was experimental. Its initial 11.4–18.0% q8 synthesis screen was not sufficient to promote it. The later aligned/unaligned ablation and current-source results below isolated its layout effect; the default implementation now uses inline aligned scratch.

**Source checkpoints:** `PERFORMANCE_STATUS.md`; `reports/scalar-t64-recurrence-transition-2026-10-20.md`; `reports/scalar-analysis-cumulative-2026-10-20.md`; `reports/PERFORMANCE_FINDINGS.md`; DUCC issue 86, especially comments `6294359778` and `6294831374`. These are provenance, not fresh performance guarantees.

## 3. Benchmark contract and initial matrix

First, **reuse the existing matched native RustSHT/DUCC benchmark infrastructure**
and its recent evidence rather than starting a redundant full campaign.
Freeze final Plan 11 source commit `a37f74d68631ab5fd373a4097640895e9f0c0c23` on this branch and pin DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The preceding matched builds used Rust 1.99.1/LLVM 23.1.1 and AVX2/FMA on a Ryzen 9 5950X. Record actual toolchains, source SHAs, flags, CPU affinity, SIMD dispatch, and memory-layout contracts. DUCC's reference SHT objects used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; Rust used strict FP and Pulp V3. Do not silently conflate FP settings or timings from older versions. A separately matched GCC/Clang DUCC control may isolate compiler effects.

- **Small q1 latency screen:** T4, T8, T16, T32, T48, T63, T64, T127, plus a tightly targeted boundary case if necessary. Use f64 GL scalar analysis and synthesis, full `mmax`, one exact matched longitude length per T, and only a few odd-minimum versus even-smooth controls where FFT shape is at issue. Include T1 and T2 only as absolute overhead controls. A cold/first-call table and a steady-state table are separate.
- **Large throughput screen:** T720 and T2047 with post-Plan-11-safe geometry; optionally T1023 for scaling. At T720 compare full `mmax` and a reduced-order control (such as `mmax=180`); compare matched odd-minimum/even-smooth longitude sampling where relevant. Run q1 and q8; q4 in the same cache domain is an optional discriminator, not a mandatory broad thread sweep. Do not spend time repeating known pathological T2048 cutoff measurements.
- **Benchmark semantics:** identical map/coefficient formulas, geometry, spin, normalization, phase, latitude/longitude resolution and useful harmonic work. Preallocate output and scratch as each API allows. Exclude plan/workspace creation from steady-state latency; measure **first call and plan/workspace setup separately**. Include exact input/output identity checks and normalized errors. Avoid comparing Rust native calls to Python DUCC wrapper calls as if call boundaries match.
- **Timing:** warm up; for operations shorter than ~100 ms, calibrate repeated complete calls to ~20–100 ms per batch and collect **at least 30 timed batches over multiple independent, interleaved process runs**. For long calls, use repeated individually timed calls across independent interleaved runs, report count, median, p10/p90 and spread. Pin q1 to one physical core; use identical eight **physical** CPU cores and affinity for both libraries at q8. Verify selected Pulp V3 and DUCC V3 actually execute; do not infer ISA from a compiled symbol alone. Check host contention/PSI and frequency; preserve noisy observations and explain uncertainty.
- **Stage boundary rule:** measure comparable stages in each library, ideally using temporary minimally intrusive native hooks. In particular separate FFT, spectral packing/scaling/reordering, Legendre preparation, ordinary recurrence/accumulation, scaled work, output scatter, scheduler/worker startup and zeroing. Do not subtract incompatible phase timers and label the difference a CPU-cycle measurement. Report total whole-call latency alongside stage numbers.

## Current progress — direct q8 synthesis output (2026-10-21)

Source commit [`fe0590ac`](../reports/plan14-direct-spectrum-output-2026-10-21.md)
removes the order-major spectrum scratch and its separate parallel scatter
from scalar GL ordered synthesis. A dynamically scheduled order now writes
directly to its unique column in the pre-cleared ring-major spectrum. The
checked fallback clears partial writes before running. This changes neither
the recurrence nor the serial path.

Matched clean-source q8 runs at T720 (full and reduced `mmax`, odd/even
longitude sampling) and T2047 (`ntheta=2049`, `nphi=4096`, full `mmax`)
showed Rust latency reductions of **16–35%**. T2047 changed from **75.24 ms
(1.307x DUCC)** at parent `a6059cd` to **54.89 ms (0.950x)** averaged over
two candidate invocations. The maximum relative L2 error was `1.84e-13`;
the parallel and no-default-feature suites and fallback tests passed. Data,
per-run distributions, host observations and limitations are in the
[checkpoint report](../reports/plan14-direct-spectrum-output-2026-10-21.md).
The earlier dirty-worktree screen is excluded from this comparison.

This is a measured layout/scheduling improvement, not evidence that all of
the removed 20.35 ms at T2047 came from the transpose alone. The next work
completes a clean small-q1 baseline, then profiles the remaining q8
synthesis excess using this direct-output source.

## Current progress — small q1 baseline and T127 analysis gap (2026-10-21)

The clean-source q1 screen covers T4, T8, T16, T32, T48, T63, T64 and T127,
with full/quarter `mmax`, odd/even longitude shapes, analysis and synthesis.
Each case used 30 calibrated batches across three interleaved process pairs.
The T127 reduced-order case (`mmax=31`) is the largest selected q1 analysis
excess: 192.5 µs Rust versus 99.5 µs DUCC averaged across longitude shapes.
A warmed Rust phase probe measured 161.0 µs in recurrence/accumulation,
19.96 µs in FFT, 4.95 µs in order preparation and 2.49 µs in latitude
setup. Its route had 32 normal blocks, zero scaled blocks and 460,800
estimated active ring-degree terms. Timer-only DUCC hooks measured
34.34 µs in `map2leg` and 69.84 µs in `leg2alm_internal`; boundaries differ
because the latter includes DUCC ring packing and initialization.

Disassembly of the executed GCC DUCC and Rust Pulp V3 bodies shows four f64
lanes and packed FMA in both. The Rust two-pair vector body has 12 packed
FMA instructions with eight accumulation chains; the DUCC two-coefficient
body has 13 with the same eight accumulators and two recurrence states. No
hot-loop spills or important scalarized recurrence were observed. The nearly
2x reduced-order stage gap is therefore not explained by a missing SIMD path
or a spill visible in these loops. Keep the current recurrence unchanged
while isolating same-active-ring kernel work. Full results, exact revisions,
flags, error and contention indicators are in
[`plan14-small-q1-baseline-2026-10-21.md`](../reports/plan14-small-q1-baseline-2026-10-21.md).

No small-q1 source change is supported yet. The q8 synthesis phase profile
below now narrows the remaining large-transform lead; isolate that odd-length
inverse stage before returning to the T127 q1 kernel comparison.

## Current progress — T720 q8 synthesis stage profile (2026-10-21)

At T720, `mmax=180`, q8, exact `phi0=0.23` benchmark coefficients and GL
geometry, the clean whole-transform baseline is 1.723 ms Rust / 1.254 ms DUCC
for odd-minimum `nphi=361` (+0.469 ms), and 1.410 / 1.221 ms for even-smooth
`nphi=378` (+0.189 ms). The values are from the existing 30-batch
`fe0590ac` checkpoint, not the short phase probe.

Test-only Rust phase probes (three process runs, eight calls per run) and
timer-only hooks in a temporary pinned DUCC copy produced this diagnostic
breakdown:

| `nphi` | Rust geometry + clear | Rust ordered recurrence | Rust inverse FFT + writeback | DUCC `alm2leg` | DUCC `leg2map` |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 361 | 0.075 ms | 1.210 ms | 0.332 ms | 1.084 ms | 0.168 ms |
| 378 | 0.086 ms | 1.198 ms | 0.065 ms | 1.088 ms | 0.128 ms |

These are stage diagnostics, not calibrated Rust stage distributions. Rust
phase probes used short individual calls; the outer test-only Legendre timer
overlaps the ordered-recurrence timer and is excluded above. DUCC's timings
average 234–245 instrumented calls per process. The high-level boundaries also
differ: DUCC `leg2map` includes phase packing, real FFT and map stores, while
Rust reports its inverse row routine as one FFT/writeback phase. No stage
percentages should be interpreted as CPU-cycle shares.

The odd-length path is the largest measured shape-specific lead. Rust's
`Engine::Odd` reconstructs and transforms a full complex length-361 row;
DUCC's `leg2map` uses its real pocketfft path. For the even-smooth control Rust
uses its half-complex real path and the combined inverse stage is shorter than
DUCC's. This supports testing the odd real-transform work separately; it does
not yet prove that replacing the Rust FFT path is the best whole-call change.

Two spectrum-clear experiments were removed. Writing polar zeros separately
from each order produced bitwise-identical output but regressed a matched
same-host q8 A/B from 1.692 ms to 1.996 ms Rust, with DUCC at 1.266 ms; the
extra strided per-order stores outweighed skipping the clear. A row-contiguous
polar-tail initializer reduced the test-only clear phase from about 54 µs to
2 µs, but its full-transform follow-up ran with roughly 12% scheduler wait
and DUCC itself moved to 1.46 ms. That timing is inconclusive, so this variant
was also removed. Production retains the existing full clear.

## Current progress — odd-length inverse component profile (2026-10-21)

The test-only `plan14_odd_inverse_phase_attribution` probe decomposes the
actual Rust odd inverse path at T720, `mmax=180`, `ntheta=721`, `nphi=361`,
q8. It retains the existing implementation and separately times Hermitian
row packing, `rustfft` complex inverse execution, and real-output extraction.
The probe checks q1/q8 map agreement (`relative L2 < 4e-13`). Three pinned
processes each ran 30 batches of 16 full synthesis calls. Each batch lasted
about 26–27 ms. CPU affinity used the same eight physical cores within one
CCD; Rust release used `-C target-cpu=x86-64-v3` and runtime Pulp V3. Host CPU PSI `avg10` was
0.25% after the capture. Raw batch logs were retained locally and are not
committed.

| Process | Whole-call p10 / median / p90 (ms) | Inverse stage p10 / median / p90 (ms) | Max-worker pack / complex FFT / output (µs per call) |
| ---: | ---: | ---: | ---: |
| 1 | 1.621 / 1.706 / 1.787 | 0.374 / 0.423 / 0.481 | 34.0 / 300.5 / 7.5 |
| 2 | 1.613 / 1.653 / 1.711 | 0.362 / 0.385 / 0.426 | 34.4 / 299.9 / 7.5 |
| 3 | 1.612 / 1.630 / 1.665 | 0.361 / 0.370 / 0.394 | 34.4 / 300.3 / 7.6 |

The component counters aggregate worker CPU time, so their sums exceed wall
time. Across the three process medians, the summed worker costs per full call
were about 0.263 ms for packing, 2.236 ms for complex inverse arithmetic and
0.056 ms for output extraction. The slowest worker spent about 0.034, 0.300
and 0.008 ms in those components. The first three timer controls suggested
an 11–23 µs increase to the complete call, but a later short control reversed
the sign amid wider timing spread; do not subtract a fixed instrumentation
correction. Component values are diagnostic. The separate existing phase
probe measured a 0.370–0.423 ms median inverse stage in the primary runs.
This confirms that the complex FFT itself dominates the Rust odd inverse
stage, while packing and output stores are small. The earlier DUCC `leg2map`
timer was 0.168 ms, but its boundary also includes pocketfft and map output;
the remaining difference is not a direct FFT-only ratio.

A screening comparison of the existing standalone `rustsht-fft` real C2R
engine at N=361 (721 rows, one thread) measured 3.861 µs per transform versus
3.089 µs for the RustFFT odd-complex adapter. It is single-process screening
evidence, not a q8 production A/B. It provides no basis for integrating that
engine into `ShtPlan`, and no production FFT code changed.

Reproduction: run
`RUSTFLAGS='-C target-cpu=x86-64-v3' taskset -c "${RUSTSHT_Q8_CPUSET:?set eight physical CPU IDs}" cargo test --release --lib plan14_odd_inverse_phase_attribution -- --ignored --nocapture --test-threads=1`
three times; screen the standalone alternative with
`RUSTFLAGS='-C target-cpu=x86-64-v3' taskset -c "${RUSTSHT_Q1_CPUSET:?set one physical CPU ID}" cargo bench --manifest-path crates/rustsht-fft/Cargo.toml --bench rustfft -- bench-one c2r 361 721`
and `RUSTFLAGS='-C target-cpu=x86-64-v3' taskset -c "${RUSTSHT_Q1_CPUSET:?set one physical CPU ID}" cargo bench --manifest-path crates/rustsht-fft/Cargo.toml --bench rustfft -- bench-native-real 361 721`.

The source lead is now bounded: the odd full-complex inverse arithmetic is
the largest identified q8 synthesis substage, but the existing real-FFT
implementation is slower and the stage still accounts for only part of the
whole-call excess. Do not add a replacement FFT engine based on this profile.
The T720 q8 analysis profile below is complete; next compare the same-active-
ring T127 q1 analysis boundary.

## Current progress — calibrated q8 analysis profile (2026-10-21)

The final Plan 11 source at `5268ec4a` was compared with DUCC 0.42.0 at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Both cases used T720 GL,
`ntheta=721`, `nphi=1442`, `phi0=0.23`, q8, identical deterministic map
fixtures and three interleaved process pairs pinned to the same eight physical
cores. Each backend/case contributed 30 timed batches across three processes;
each batch targeted 25 ms; observed whole-call batch durations were
20.5–26.2 ms. Rust used release, strict FP,
`-C target-cpu=x86-64-v3` and runtime Pulp V3. DUCC used GCC 15 x86-64-v3
SHT objects with `-O3 -ffast-math -ffp-contract=fast`; its validation driver
remained strict. The CSV preserves exact fixture hashes, batch distributions
and phase distributions.

| `mmax` | Rust whole p10/median/p90 (ms) | DUCC whole p10/median/p90 (ms) | median Rust/DUCC | median excess | relative L2 error |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 720 | 2.960 / 3.009 / 3.109 | 2.835 / 2.854 / 2.881 | 1.054x | +0.155 ms | `4.66e-14` |
| 180 | 1.632 / 1.687 / 1.752 | 1.960 / 1.972 / 1.990 | 0.856x | -0.284 ms | `4.93e-14` |

For full `mmax`, Rust phase medians were 0.948 ms forward FFT, 0.015 ms
latitude setup, 0.252 ms order-spectrum preparation, 0.084 ms coefficient
initialization and 1.746 ms recurrence/accumulation. DUCC measured 1.221 ms
in `map2leg` and 1.589 ms in `leg2alm_internal`. Grouping Rust's front-end
phases gives about 1.215 ms, close to DUCC `map2leg`; the recurrence-side
Rust phases total about 1.831 ms, above DUCC's combined Legendre stage. This
is the remaining T720 full-order lead, but its phase boundaries are not exact
subtractions and it explains only the observed 0.155 ms whole-call excess
directionally.

At `mmax=180` with the same full longitude sampling, Rust measured 0.802 ms
forward FFT, 0.015 ms latitude setup, 0.069 ms order preparation, 0.027 ms
coefficient initialization and 0.774 ms recurrence/accumulation. DUCC measured
1.176 ms in `map2leg` and 0.757 ms in `leg2alm_internal`. The Rust front-end
group is about 0.886 ms while the recurrence-side group is about 0.801 ms;
the former advantage outweighs the small latter excess, consistent with the
0.284 ms whole-call win. The recurrence phase includes job scheduling and
worker waiting, so this profile does not isolate SIMD arithmetic from q8
scheduling.

Across the matched whole-call runs, CPU pressure-stall readings remained
negligible; observed per-process CPU-frequency medians ranged 3.89–4.42 GHz for Rust and
4.34–4.38 GHz for DUCC. Rust's third instrumented full-order phase run had a
wider tail, retained in the p90 and source data. DUCC stage hooks were enabled
only after calibration: the per-process stage call counts were exactly 90 for
full order and 130 for reduced order, matching their timed calls. Maximum
absolute output error was `1.04e-15` (full) and `1.11e-15` (reduced). No
production kernel change was justified by this profile. The reduced-order
front end is already faster than DUCC at this geometry; preserve it and
proceed to the matched T127 q1 same-active-ring comparison.

Reproduction: use
`scripts/benchmark_ducc_full_transform_parity.py` with
`--lmax 720 --mmax 720 --fixed-nphi 1442 --threads 8 --operation analysis`
and then `--mmax 180`, using three interleaved process pairs, 25 ms batches,
and the pinned DUCC revision above. Whole-call timings use the clean pinned
DUCC SHT object build; the temporary instrumented build is used only for
stages. The test-only Rust phase probe is
`plan14_analysis_phase_batch_profile`; run it three times per `mmax` with
`RUSTSHT_PROFILE_Q=8`, `RUSTSHT_PROFILE_LMAX=720`,
`RUSTSHT_PROFILE_MMAX=720` or `180`, `RUSTSHT_PROFILE_BATCHES=30`,
`RUSTSHT_PROFILE_BATCH_MS=30`, and `RUSTSHT_PROFILE_MAP` set to the matching
fixture. DUCC phase values use temporary `steady_clock` hooks around
`map2leg` and `leg2alm_internal`; they are stage diagnostics, not hardware
counters. The committed machine-readable summary is
[`summary.csv`](../reports/data/plan14-q8-analysis-t720-2026-10-21/summary.csv);
raw host and process logs remain local.

## Current progress — exact T720 q8 stage rebaseline (2026-10-21)

After Plan 11 closeout and the inline-scratch production commit, the exact
full-order even-smooth analysis case (`mmax=720`, `ntheta=721`,
`nphi=1458`) measured **2.479 ms Rust vs 2.124 ms DUCC** (1.167x,
`+0.355 ms`). The reduced-order odd-minimum synthesis case
(`mmax=180`, `nphi=361`) measured **1.638 vs 1.263 ms** (1.297x,
`+0.375 ms`). Each clean whole-call comparison used three interleaved
process pairs, 30 calibrated 25 ms batches per backend and q8 pinned to the
same eight physical cores. Rust used 1.99.1/LLVM 23.1.1, strict FP,
`-C target-cpu=x86-64-v3`, runtime Pulp V3; DUCC was pinned at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2, with production SHT
flags `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` and a strict
driver. Relative L2 errors were `4.63e-14` for analysis and `5.19e-14` for
synthesis. Per-case whole-call distributions are in
[`summary.csv`](../reports/data/plan14-q8-rebaseline-2026-10-21/summary.csv).

For full-order analysis, Rust's test-only phase batches put median order
spectrum preparation at `240.1 us` (p10/p90 `225.3/255.7 us`) and output
coefficient initialization at `81.8 us`; recurrence/accumulation was
`1.683 ms`, forward FFT `491.3 us`, and latitude setup `15.4 us`. Three
temporary DUCC timer-hook processes measured `map2leg` at `500/503/550 us`
and `leg2alm_internal` at `1.568/1.567/1.730 ms`. The third DUCC process
was slower across both stages and the whole call; keep it visible as a
contended observation. Stage boundaries differ, so these values rank work
but are not subtractable cycle shares. The clean whole-call medians remain
the performance comparison.

The exact Rust preparation transforms and stores every order/ring pair:
`721 x 721 = 519,841` complex entries. The recurrence consumer checks
`ring_order_limits` before each read and substitutes zero for inactive
north/south rings; DUCC `fill_m2a` packs only rings with `mlim >= m`. I
tested skipping inactive slots in two forms. A per-ring guarded form raised
phase preparation from `240.1` to `333.2 us` and the q8 whole call from
`2.479` to `2.596 ms` (same pinned DUCC source, but with a high 10% p90
runqueue-wait observation). Bounding the loop to a symmetric active interval
reduced preparation to `222.9 us`, but whole q8 latency was `2.483 ms`
versus `2.479 ms` at baseline; the distributions overlap and no end-to-end
gain is established. The q1 candidate was `16.128 ms`, but has no same-round
Rust baseline. Both variants matched DUCC at `4.63e-14` relative L2, and
NaN-sentinel/GL-cutoff checks passed. The candidate code was removed because
the stage saving did not produce a whole-call improvement. Measurements are
in [`candidate-ablation.csv`](../reports/data/plan14-inactive-ring-preparation-2026-10-21/candidate-ablation.csv).

The q8 synthesis scheduler probe counted 181 order jobs. In two
instrumented calls, mutex acquisition plus pop took `6.04/6.67 us` total;
the first call's workers processed estimated work from `8.93M` to `10.76M`
terms in `1.099–1.154 ms`.
That queue cost cannot explain the `0.375 ms` whole-call excess, so do not
replace the synthesis scheduler on this evidence. The analysis queue and
worker profile below supersedes the earlier shared-observer-lock probe. Raw
timer and process logs remain local under `/tmp`; compact reproducible values
are in
[`stage.csv`](../reports/data/plan14-q8-rebaseline-2026-10-21/stage.csv).

## Current progress — T720 q8 analysis queue profile (2026-10-21)

A test-only profile of the exact T720 full-order analysis fixture counted 721
order jobs per call. It times dequeue locking and each order's analysis work
in worker-local counters; it does not take the shared observation mutex used
by the first, provisional probe. In two independent 30-call runs, the sum of
lock/wait durations over all Rayon threads had p10/median/p90
`38.258/41.654/49.448 us` and `34.072/38.990/49.847 us` per call. This is
aggregate thread time, not a directly subtractable wall-time stage because
each worker's waiting is counted in the sum.

Across the eight Rayon threads, average estimated work ranged from `19.12M`
to `20.54M` ring-degree terms (max/min spreads of about 7.5% and 3.0%); order
execution time ranged `1.660–1.668 ms` and `1.645–1.650 ms` per thread in the
two runs, with 85–91 orders per thread. This suggests the dynamic queue is
balancing this case well. Unprofiled call medians were `2.464/2.588 ms`;
instrumented medians were `2.536/2.528 ms`. Their differing shifts and tail
ordering show that these probes cannot provide a calibrated production timing
or isolate timer overhead.

The queue has a measurable cost and is a candidate for one bounded
batch-dequeue experiment, but this profile does not establish its wall-time
contribution to the `+0.355 ms` Rust/DUCC whole-call excess. The experiment
below found no robust benefit, so production keeps one-order dequeue. The
ignored test is `plan14_analysis_order_queue_profile`; its compact
measurements are in
[`analysis-queue-profile.csv`](../reports/data/plan14-analysis-scheduler-2026-10-21/analysis-queue-profile.csv).

## Current progress — q8 analysis dequeue batching A/B (2026-10-21)

Two temporary queue variants were compared against the unchanged
single-order scheduler at T720, even-smooth GL analysis, `mmax=1/180/720`,
q1/q8. Each matrix used three interleaved Rust/DUCC process pairs and 30
calibrated 25 ms batches per backend/case on the same physical cores. Rust
was 1.99.1/LLVM 23.1.1 with strict FP and runtime Pulp V3; DUCC was pinned at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2, with the production
x86-64-v3 SHT flags. Deterministic inputs were identical; the maximum
relative L2 error was `6.12e-14`. The runs were separate source builds (each
interleaved Rust with DUCC), not an interleaved three-way source comparison.
Host runqueue-wait p90 was about 16.7% in all three matrices, so retain the
spread and treat small shifts as inconclusive.

| q8 policy | `mmax=1` Rust ms | `mmax=180` Rust ms | `mmax=720` Rust ms | Change vs one-pop (`m=1/180/720`) |
| --- | ---: | ---: | ---: | ---: |
| One order per pop | 0.0870 | 1.0067 | 2.4956 | reference |
| Fixed two-order pop | 0.1222 | 1.0289 | 2.4658 | +40.4% / +2.2% / −1.2% |
| Two-order while queue is deep; one-order tail | 0.0859 | 1.0043 | 2.5003 | −1.3% / −0.2% / +0.2% |

DUCC medians for the same q8 cases were respectively `0.1323/0.9359/2.1252
ms` in the one-pop matrix and `0.1320/0.9320/2.1376 ms` in the guided
matrix. Fixed two-order dequeue assigned both jobs in the `mmax=1` queue to
one worker, explaining that failure. The guided tail avoided this regression,
but neither full nor reduced order showed a repeatable whole-call gain; q1
changes were within noise and q1 bypasses the parallel queue. Both temporary
variants were removed. Keep the current scheduler and proceed to the current
source q8 synthesis odd-inverse stage, the largest selected absolute excess.

The complete q1/q8 rows, p10/p90, excess, errors and host summary are in
[`batch-ablation.csv`](../reports/data/plan14-analysis-scheduler-2026-10-21/batch-ablation.csv).

## Current progress — T127 q1 reduced-order dispatch correction (2026-10-21)

The earlier T127 phase profile compared the ordinary callback route against
DUCC, but its assembly section incorrectly treated the compiled fused symbol
as the executed production path. Source tracing showed
`FUSED_SCALAR_ANALYSIS_MIN_LMAX=255`; at T127, `mmax=31`, Rust used
`for_each_y_lm_paired_status_dispatched` and callback accumulation at every
degree. The matched 64-lane comparison then measured that actual route at
3.700 µs median versus 0.705 µs for DUCC. The checked fused route measured
1.499 µs versus 0.707 µs DUCC (2.12x at this isolated boundary).

Commit [`50c9866b`](https://github.com/astro-informatics/RustSHT/commit/50c9866b196d7979eb2f1f301fc9b3a5d07b6adf)
sets a type-specific eligibility threshold: f64 scalar analysis uses the
existing fused route from T127, while f32 remains at T255. T126/T127
production tests observed zero/32 fused blocks for the reduced-order plan.
The fused route retains its checked proof, scaling path, numerical
preflights, late range validation and fallback. No recurrence arithmetic,
table policy, spin-1 path or FFT changed.

On the same T127 GL `mmax=31`, `ntheta=128`, odd `nphi=63` whole-transform
fixture, Rust improved from 188.940 µs (1.818x DUCC) to 100.249 µs (0.964x
DUCC at 103.999 µs). Candidate relative L2 error was `8.77e-15`, maximum
absolute error `1.55e-15`. The paired-ratio p10/p90 was 0.934/0.993. The
candidate's median runqueue wait was 10.4% (p90 29.3%), so the small whole
call win over DUCC is approximate parity; the reduction from the previous
85 µs excess is the robust result. Isolated-kernel and full-call distributions,
flags, setup/cold timings, validation, and reproduction commands are in the
[T127 report](../reports/plan14-t127-fused-route-2026-10-21.md). The earlier
report's compiled-versus-executed correction is preserved in the
[initial q1 baseline](../reports/plan14-small-q1-baseline-2026-10-21.md).

This closes the reduced-order T127 q1 lead, not the full small-q1 study.
Continue with the remaining T1–T64 fixed-cost attribution, then re-rank the
largest whole-call q8 synthesis/analysis excess. The T720 odd-length inverse
FFT remains a diagnostic lead only: the tested standalone real inverse was
slower, so no production FFT change is supported.

## Current progress — T127 compact-table analysis SIMD (2026-10-21)

Commit `0484a876` adds a Pulp V3 path for the existing f64 scalar analysis
table. It packs two reflected complex latitude pairs per YMM and accumulates
adjacent even/odd degrees together. Other vector widths and unsupported
geometries retain the scalar implementation; analysis selection and
synthesis, spin-1, f32 and numerical fallbacks are unchanged.

At T127 full `mmax`, `ntheta=128`, odd `nphi=255`, q1 warmed Rust latency
fell from 398.496 to 376.600 µs (-5.5%), against DUCC 294.042 µs in the
candidate round (1.281x Rust/DUCC, +82.6 µs). Relative L2 was `8.52e-15`.
At q8, Rust was 221.254 µs before and 223.743 µs after, with overlapping
spreads and higher scheduler interference in the candidate round; no q8
change is established. Three interleaved process pairs and 30 calibrated
batches per backend were used. A table-versus-forced-recurrence route control
does not support changing the table selector. Full distributions, toolchain,
flags, dispatch and numerical checks are in the
[focused report](../reports/plan14-t127-table-analysis-simd-2026-10-21.md).

The T64 reduced-order recurrence lead and its checked fused-route candidate
are described next. T32 and T31 full/reduced-order analysis now use the
checked fused route from T32/T16, respectively. The T16 cutoff experiment
and its first-call caveat are recorded below; do not continue lowering the
analysis floor without new evidence. Return to the remaining q8 whole-call
excess from current-source controls.

## Current progress — T64 checked fused analysis route (2026-10-21)

Commit `7e51ae43` lowers the f64 scalar fused-analysis eligibility floor
from T127 to T64. The checked SIMD route, range proof, scaled transition,
late validation and fallback are unchanged. f32 remains at T255; the
scalar table selector, spin-1 and other operations are untouched.

Matched T63/T64 GL q1 analysis at full local geometry used odd-minimum and
even-smooth longitude counts, three interleaved Rust/DUCC process pairs and
30 calibrated batches per backend/case. At T63, Rust moved only +0.2–0.4%
within the observed spread. At T64 `mmax=16`, Rust improved 36.1% for
`nphi=33` (41.262 to 26.374 µs, 0.855x DUCC) and 35.2% for `nphi=36`
(42.912 to 27.793 µs, 1.051x DUCC). Relative errors were below `2.91e-15`.

The current-source stage profiles attribute 13.6–14.3 µs of the full-call
gain to recurrence/accumulation; FFT and preparation stages stayed stable.
The report and raw data are in
[the T64 checkpoint](../reports/plan14-t64-fused-analysis-cutoff-2026-10-21.md).

## Current progress — T32 checked fused analysis route (2026-10-21)

Commit `de4c6569` lowers only the f64 scalar analysis fused-route floor
from T64 to T32. The proof/cache, scaled transition, late range checks and
robust fallback are unchanged; f32 remains at T255 and table, spin-1 and
other operation policies are unchanged.

The matched q1 GL A/B used three interleaved Rust/DUCC process pairs and
30 calibrated batches per backend for T31/T32 full/reduced `mmax`, with
odd-minimum and even-smooth longitude counts. At T31, Rust changed by
`+0.35%` (`nphi=63`) and `-0.28%` (`nphi=64`) within the process spread.
At T32, all four cases improved: full-order `nphi=65/66` fell by
`18.1%/18.8%` and reduced-order `nphi=17/18` by `22.1%/22.0%`. The T32
full-order even case moved from 31.011 to 25.176 µs versus DUCC's 21.678
and 21.833 µs in the two rounds; its absolute excess narrowed from 9.333
to 3.343 µs. Maximum candidate relative L2 was `2.62e-15` and maximum
absolute error `2.67e-15`. Runqueue wait remained elevated (median roughly
12–16%, p90 up to 32%), but the baseline/candidate latency distributions
separated for every T32 case.

The calibrated phase probe for T32 full-order even sampling measured
recurrence/accumulation falling from 21.184 to 16.558 µs; FFT and other
preparation stayed effectively unchanged. The remaining low-band analysis
lead is T31 full-order even-smooth sampling: Rust/DUCC was 27.130/16.499 µs
before and 27.054/16.491 µs after T32 dispatch, a stable +10.6 µs excess.
That callback-route gap was subsequently tested with a lower T16 floor; see
the next section. Full raw batches and reproducibility details are in the
[T32 report and data](../reports/plan14-t32-fused-analysis-cutoff-2026-10-21.md).

## Current progress — T16 checked fused analysis route (2026-10-21)

Commit `830ecc63` lowers only the f64 fused scalar-analysis floor from T32
to T16. The route-selection test confirms T15 remains on the callback path
and T16/T31 use the checked fused path. f32 remains at T255; scaling,
preflight, proof/cache, fallback, tables, spin-1 and all other operators are
unchanged.

The native Rust/DUCC GL q1 A/B covers full-order T15/T16/T31 at odd-minimum
and even-smooth `nphi`, with three interleaved process pairs and 30 timed
75 ms-target batches per backend/case. Every batch lasted 42.47–71.94 ms.
T15 moved by only ±0.5%. T16 improved 8.8–9.4%, and T31 improved
20.3–22.3%. T31 even-smooth Rust/DUCC moved from 27.051/16.520 µs to
21.559/16.325 µs; the remaining excess is +5.234 µs. Rust's actual Pulp
backend was V3; DUCC was GCC 15.2, AVX2/FMA, built with its production
fast-math SHT flags. Host frequency medians matched at 4.691 GHz; q1
runqueue-wait medians were 5.4%/5.8%, p90 11.7%/14.2%.

The candidate maximum relative L2 error was `1.54e-15`; all full and
no-default release tests, numerical/adjoint checks, formatting and strict
Clippy passed. A 10-pair T15/T16 cold-follow-up found T16's first-call
median at 30.898 µs candidate versus 26.230 µs baseline, with overlapping
p10–p90 ranges and a 2.29 µs first-call shift in the T15 control. If the full
4.67 µs T16 shift is real, its 0.775 µs warm saving repays it after roughly
six further analyses. Keep this cold/warm distinction visible; the measured
steady-state gain supports the floor for reused plans but does not establish
a first-call improvement.

The report, binary hashes, raw batches and phase summaries are in the
[T16 cutoff checkpoint](../reports/plan14-t16-analysis-cutoff-2026-10-21.md).
Phase A's low-band cutoff follow-up is closed for now. The current q8
rebaseline and scratch-layout result are recorded below; do not start another
low-degree cutoff sweep.

## Current progress — inline aligned recurrence scratch (2026-10-21)

The fixed-scratch candidate was tested at source `b38d6696` against the
`Vec`-backed workspace, then promoted without a cfg selector in source commit
`8e49f7d5`. The seven recurrence/analysis buffers have a 256-element
capacity in both layouts: `recurrence_scratch_len()` is at least
`SCALAR_ORDER_BATCH_SIZE=256` and `LATITUDE_BLOCK_SIZE=64`. f64 scratch
payload remains 14,336 bytes per recurrence worker; the new layout removes
seven heap allocations and 168 bytes of `Vec` headers per worker. The
workspace stays caller-owned and bounded.

The matched T720 GL q8 whole-transform comparison used Rust 1.99.1/LLVM
23.1.1 with runtime Pulp V3 (four f64 lanes), and DUCC 0.42.0 at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` with GCC 15.2 x86-64-v3 SHT
flags `-O3 -ffast-math -ffp-contract=fast`. Full `mmax=720` and reduced
`mmax=180` each covered odd-minimum and even-smooth longitude sizes. Each
backend/case had three interleaved process pairs and 30 calibrated batches;
the target was 25 ms. One full-order candidate batch measured 19.5 ms, just
below the 20 ms lower bound; the remaining batches were within 20–100 ms.

| mmax | nphi | op | Rust Vec → inline (ms) | DUCC before-run → after-run (ms) | Rust/DUCC ratio before → after | Rust change |
| ---: | ---: | :-- | ---: | ---: | ---: | ---: |
| 720 | 1441 | A | 3.332 → 3.339 | 3.171 → 3.171 | 1.051 → 1.053 | +0.2% |
| 720 | 1441 | S | 4.173 → 4.093 | 3.887 → 3.889 | 1.073 → 1.053 | −1.9% |
| 720 | 1458 | A | 2.510 → 2.457 | 2.122 → 2.121 | 1.183 → 1.158 | −2.1% |
| 720 | 1458 | S | 3.100 → 3.018 | 2.695 → 2.731 | 1.150 → 1.105 | −2.7% |
| 180 | 361 | A | 1.239 → 1.212 | 0.949 → 0.972 | 1.305 → 1.247 | −2.2% |
| 180 | 361 | S | 1.711 → 1.660 | 1.258 → 1.254 | 1.361 → 1.324 | −3.0% |
| 180 | 378 | A | 1.005 → 1.003 | 0.923 → 0.921 | 1.089 → 1.090 | −0.2% |
| 180 | 378 | S | 1.424 → 1.342 | 1.220 → 1.219 | 1.167 → 1.101 | −5.8% |

The reduced even-smooth synthesis case was independently extended to 60
batches per source over six process runs. Rust Vec / inline p10–median–p90
was `1.3996/1.4127/1.4371` / `1.3254/1.3380/1.3664` ms; the corresponding
DUCC distributions were `1.2041/1.2138/1.2245` / `1.1976/1.2110/1.2279`
ms. Its median Rust/DUCC ratio fell from 1.164 to 1.105 and absolute excess
from 0.199 to 0.127 ms. A targeted 64-byte versus 16-byte alignment build
kept the contiguous inline layout: medians were 1.338 and 1.373 ms
respectively, versus 1.450 ms for `Vec` and 1.221 ms for DUCC. The 16-byte
variant's upper tail was affected by a 5.2% sampled runqueue wait, so the
independent 64-byte alignment contribution remains directional rather than
conclusive.

A six-pair q1 repeat (60 batches per source, 40 ms target) of full-order
even-smooth synthesis measured baseline / inline p10–median–p90 at
`20.174/21.960/38.457` / `20.191/21.984/22.506` ms; DUCC was
`18.713/18.893/19.889` ms. The median difference is 0.1%. The isolated
near-2× process moved from the inline build in the earlier run to the Vec
baseline in this repeat; all Rust q1 runs showed 5–7% sampled runqueue wait.
Treat q1 as neutral within host/process variability, not as a candidate
regression or a measured gain. Rust baseline and inline outputs were
bitwise identical in the repeated comparison; max relative L2 error versus
DUCC was `4.71e-14` for the q8 control and `4.41e-14` for q1.

The executed AVX2 V3 `ScalarOrderedSynthesis` symbol remained 12,824 bytes
and 2,435 decoded instructions in both builds. Its opcode sequence and
normalized non-address operands matched exactly (46 packed FMA instructions,
25 packed multiply/add instructions, 165 vector moves). The measured change
therefore comes from scratch placement/allocation and alignment, not a new
SIMD instruction schedule. Fixed arrays have the same capacity as the prior
vectors; process RSS was comparable. Release all-feature and no-default
feature tests, the aligned-layout/tail test, formatting and strict Clippy
passed. The compact matched data is in
[`summary.csv`](../reports/data/plan14-aligned-scratch-layout-2026-10-21/summary.csv);
raw process and host logs remain local under `/tmp`.

The largest remaining selected q8 excess is now reduced-order odd-minimum
synthesis (+0.407 ms); full-order even-smooth analysis is next (+0.335 ms).
Re-profile the first case's recurrence versus odd inverse stage after this
layout change, then continue with the largest measured absolute stage.

## Current progress — post-scheduler T720 synthesis rebaseline (2026-10-21)

After the analysis queue A/B, the current production source was remeasured at
T720, `mmax=180`, `ntheta=721`, q1/q8 with odd-minimum `nphi=361` and
even-smooth `nphi=378`. The same pinned DUCC 0.42.0 build and native
full-transform harness were used. Rust Pulp V3 and DUCC x86-64-v3 were
selected; each backend/case had three interleaved process pairs and 30
calibrated batches. The odd q8 case measured 1.682/1.252 ms Rust/DUCC
(1.344x, +0.430 ms); the even control was 1.363/1.221 ms (1.116x,
+0.141 ms). Relative L2 errors were `5.19e-14` and `4.71e-14`.

The q1 odd/even excesses were +2.220/+0.753 ms. Host runqueue wait reached
14–16% in the q8 comparison, so the exact q8 ratios remain directional. The
shape-specific odd inverse stage is still a measured lead, but the tested
standalone real inverse was slower than RustFFT's odd-complex adapter. No
production FFT replacement is supported by this evidence. Per-case timing
distributions are in the
[odd/even rebaseline data](../reports/data/plan14-q8-synthesis-odd-rebaseline-2026-10-21/summary.csv).

## Current progress — order-local f64 analysis output initialization (2026-10-21)

The prior T720 q8 phase profile measured a serial `alm.fill(0)` stage of
about 82 us at full `mmax` and 27 us at `mmax=180`. The recurrence owns
disjoint output slices by harmonic order, so the production change moves
that initialization into each order job immediately before accumulation.
This does not remove coefficient zeroing. The compact-table branch retains
whole-output initialization, and f32 keeps the same global initialization
because only f64 supports the paired scalar recurrence. No recurrence
arithmetic, scaling, fallback or output format changed.

The source A/B used T720 GL, `nphi=1458`, full `mmax=720` and reduced
`mmax=180`, q1/q8, deterministic full-triangle fixtures, and pinned DUCC
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Rust 1.99.1/LLVM 23.1.1 used
strict FP and `-C target-cpu=x86-64-v3` with runtime Pulp V3; DUCC GCC 15.2
used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` for SHT code.
Three process rounds rotated baseline Rust, DUCC and candidate Rust; each
backend had 30 calibrated 25 ms batches per case/thread.

| `mmax` | q | Rust before → after (ms) | DUCC (ms) | Rust/DUCC before → after | Candidate/baseline | Candidate relative L2 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 720 | 1 | 16.012 → 15.969 | 14.473 | 1.110 → 1.103 | 0.997 | `4.63e-14` |
| 720 | 8 | 2.578 → 2.388 | 2.128 | 1.209 → 1.124 | 0.926 | `4.63e-14` |
| 180 | 1 | 7.758 → 7.737 | 8.353 | 0.929 → 0.929 | 0.999 | `4.96e-14` |
| 180 | 8 | 1.236 → 1.195 | 1.248 | 0.989 → 0.961 | 0.967 | `4.96e-14` |

All three q8 full-order process-pair medians improved, reducing the median
Rust/DUCC excess by about 0.190 ms (7.4% Rust latency reduction). The
reduced-order q8 median fell 3.3%, but its per-pair effect was mixed and its
distributions overlap; treat that gain as directional. Full-order q8 Rust
p10–p90 was `2.490–2.887` ms before and `2.364–2.461` ms after; DUCC was
`2.117–2.146` ms. Reduced-order ranges overlap. q1 medians changed by less
than 0.3%, with 11–16% sampled runqueue wait, so q1 is neutral. First-call
and setup timings are included in the raw data; their process ranges overlap,
and no cold-start gain is claimed. The candidate and baseline outputs were
bitwise identical. Process-level host and frequency captures remain local;
the committed sanitized timing summary is
[`summary.csv`](../reports/data/plan14-analysis-order-local-init-2026-10-21/summary.csv).
The interleaved runner is
[`benchmark_plan14_analysis_source_ab.py`](../scripts/benchmark_plan14_analysis_source_ab.py).

Release parallel and no-default-feature suites passed, including f32,
exceptional-value, all-grid, spin and adjoint coverage. A focused q1/q8 test
also initializes the output coefficients to nonzero values and verifies
bitwise equality with a zero-initialized output. Formatting and strict
Clippy passed. The remaining selected q8 full-order analysis excess is
about 0.260 ms before the follow-up direct-input change below.

## Current progress — direct ring-major scalar analysis packing (2026-10-21)

The post-initialization phase profile at T720, `mmax=720`, `nphi=1458`, q8
measured the separate Rust order-spectrum preparation at a median `241.9 us`
(three processes: `240.8–242.1 us`). The recurrence/accumulation phase was
`1.698 ms`, forward FFT `472 us`, and latitude setup `15.3 us`. A temporary
DUCC 0.42.0 stage build at the same geometry measured `map2leg` at `511 us`
and `leg2alm_internal` at `1.579 ms`. These stage boundaries differ and the
Rust phase probe and DUCC stage runner used different deterministic finite
maps; they rank the work but are not subtractable cycle shares. Rust's
separate order-major pass was the clearest Rust-only cost to test.

The f64 GL fused-analysis route now reads the ring-major FFT spectra directly
while it packs reflected north/south pairs. It applies the same longitude
phase and per-ring weight/longitude scale in the same operation order as the
previous preparation routine. DUCC performs the analogous active-ring
preparation in `fill_m2a` inside `leg2alm_internal`. This avoids materializing
the full weighted order-major matrix and its later reads: T720 full `mmax`
has `721 x 721` complex values (`8,317,456` bytes, about `7.9 MiB`). The
existing workspace capacity remains unchanged. Compact-table, non-GL,
resampled and other-precision paths still use their prior preparation.
Product checks, scaled/ordinary transition, recurrence range proof and safe
fallback are unchanged. The direct-value test verifies bitwise equality with
the previous prepared row for every ring/order in its fixture.

The matched source comparison used baseline `f2249ac7` and candidate
`e51da649`, Rust 1.99.1 / LLVM 23.1.1 strict FP with Pulp V3, and pinned
DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` built with GCC 15.2 and
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` for SHT code. Both
screens used T720 GL, `ntheta=721`, `nphi=1458`, q1/q8, full `mmax=720` and
reduced `mmax=180`; each case had three interleaved process pairs and 30
calibrated batches per backend/thread count. The repeat screen is summarized
here; both it and the initial screen are retained in the
[machine-readable data](../reports/data/plan14-direct-analysis-ring-pack-2026-10-21/).

| `mmax` | q | Baseline Rust ms | Candidate Rust ms | DUCC ms | Candidate/baseline | Candidate/DUCC | Candidate excess ms |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 720 | 1 | 16.015 | 15.398 | 14.523 | 0.961 | 1.061 | +0.874 |
| 720 | 8 | 2.513 | 2.229 | 2.129 | 0.889 | 1.047 | +0.100 |
| 180 | 1 | 7.694 | 7.512 | 8.294 | 0.977 | 0.905 | −0.782 |
| 180 | 8 | 1.205 | 1.137 | 1.243 | 0.955 | 0.911 | −0.106 |

The candidate/baseline paired p10–p90 ratios were `0.948–0.972`,
`0.800–0.941`, `0.968–0.989`, and `0.850–0.988` in table order. A first
screen had full-order q8 ratio `0.945` with interval `0.894–1.092`; retain
that less favorable screen as well. In the repeat q8 run, candidate scheduler
wait was lower than baseline in each process pair, so the q8 reduction is
directional and its magnitude needs a quieter-host confirmation. q1 had
11–16% scheduler wait, but the full/reduced source change was consistent
across both screens. The candidate output was bitwise identical to baseline;
relative L2 versus DUCC was `4.63e-14` full-order and `4.96e-14` reduced.

The immediate full-order q8 excess fell from `+0.384 ms` to `+0.100 ms` in
the repeat screen. The order-major workspace is still allocated, but this
route no longer writes or rereads its full weighted matrix. Run the same
source comparison at T2047 q8 next to confirm the dataflow benefit at the
larger parallel geometry; keep the scheduler and odd FFT path unchanged.
Release parallel and no-default-feature tests, including all-grid/spin,
extreme-value, adjoint and f32 coverage, formatting and strict Clippy passed.

## 4. Phase A — Find and fix small serial fixed costs

**Hypothesis to test:** small T1–T32 transforms spend a significant fraction of wall time in repeated per-call setup, data conversion, buffer handling and/or independent row FFTs rather than in useful Legendre arithmetic. This is a hypothesis, not a proven diagnosis.

1. The small-q1 route-cutoff experiment is complete at T16. Retain its cold/warm tradeoff and proceed to the q8 rebaseline below; only reopen Phase A for a concrete newly measured cause.
2. Compare DUCC's `map2leg`/`leg2map`, `ringhelper`, `allocate_leg` and Legendre preparation with Rust's `src/fft.rs`, `src/transform.rs`, `src/plan.rs` and `src/workspace.rs`. DUCC's own per-call preparation can be **greater** than Rust's; measure it rather than assuming a plan advantage or deficit.
3. Distinguish **fixed per-call costs** from O(`ntheta*nphi`) row FFT costs and O(`lmax`-dependent) Legendre work by fitting latency against operation counts or using carefully chosen neighboring sizes. Identify the single largest excess in **absolute microseconds** across target sizes before editing production.
4. Test one reversible, source-justified candidate at a time: e.g. remove demonstrably redundant initialization, cache immutable geometry already owned by the plan, avoid unnecessary intermediate copies, or specialize an existing tiny-FFT path if measured. Any cache must be immutable/thread-safe and preserve first-use behavior. Do not bypass input validation, finite/range checks, real Nyquist handling, scaling, or adjoint invariants. No unsupported small-band hardcoded answers.
5. Rerun both directions across small sizes and a T127/T720 control. Keep a change only when it reproducibly improves whole-call latency without a relevant regression or material memory increase. If no general change helps, document the quantified floor and explicitly stop; do not stack speculative micro-optimizations.

**Plan 12 interface:** measure and report the separate T63/T64 and T127/T255 table-selection discontinuities, cold-build cost and warm-read cost, but leave any cache-aware threshold redesign to Plan 12. Current f64 scalar **synthesis** selects recurrence from T64; scalar **analysis** has its own selector. One global T64 cutoff has already been rejected.

## 5. Phase B — Attribute the remaining q8 large-transform excess

Do this **after** integrating the T2048 result, using the new baseline. Rank **excess wall milliseconds**, not percentage speedups of isolated microkernels. Analyze scalar **analysis and synthesis separately**. A 1.2–1.4× whole-transform ratio cannot be attributed to a mutex, memory traffic, FFT or SIMD solely because that difference is visible in source.

### B1. Parallel work distribution and per-call scheduling

- Rust analysis currently builds a per-call order queue of `(m, disjoint coefficient slice)` items guarded by `Mutex<VecDeque>`, then each Rayon worker pops one order at a time. DUCC dispatches order work through `execDynamic(..., chunksize=1)` using a lightweight atomic-index path; its FFT ring passes use their own scheduling policy. Compare queue creation, per-pop locking, Rayon region entry, worker utilization, time per `m`, and critical-path tail. Account for both libraries' **multiple parallel regions**, which affect small tasks differently from large tasks.
- First measure queue wait, number of order jobs, worker idle time and imbalance. The queue may not explain a many-millisecond q8 gap, and static contiguous division can degrade balance because low `m` contains more Legendre work. Do not replace a scheduler on appearance alone.
- If supported by measurements, make one isolated alternative: atomic-index jobs with safe disjoint-output ownership, weighted static/guided order ranges, or another bounded policy. Avoid a global mutex on a hot work-item path, but preserve workspace ownership, error propagation, deterministic output and no oversubscription. Benchmark q1 and q8 plus full/reduced `mmax` before retaining it.

### B2. Data layout, zeroing, packing and output movement

- Trace real bytes read/written, ownership and strides: original row-major map, row FFT half spectra, Rust's additional order-major spectra representation, compact reflected-ring SIMD inputs, coefficient accumulators and final scatter. Compare DUCC's actual `allocate_leg`, `fill_m2a`, order-local SIMD scratch and output layout for **the same operation**.
- Measure spectrum transpose/packing, weight/phase application, repeated `sin/cos`/polar-limit calculations, buffer clears and output scatter **individually**. At high T, the intermediate complex arrays are large enough to warrant bandwidth measurements, but do not call them bandwidth-bound without evidence.
- If a measured memory pass is material, try a blocked conversion, fuse compatible passes without altering arithmetic order, or reduce a redundant copy. Preserve SIMD-contiguous inputs, data races, cancellation accuracy, complex conjugation, and per-worker scratch limits. Compare whole-call and cache/NUMA-domain scaling; avoid making the single-thread path worse merely to simplify q8.

### B3. Longitude FFT and its parallel execution

- RustSHT currently uses `RealFftPlan` backed by RustFFT, with even-length half-complex packing/recombination and a full complex transform for odd length. DUCC's `ringhelper`/pocketfft implementation has different data movement and may pack different amounts of work per ring. The existence of these designs **does not prove** they dominate the residual.
- Compare equivalent forward/inverse **real row FFT** calls, packing, scaling, scratch and row scheduling at the exact SHT longitude lengths. Separate FFT arithmetic from reading/writing row buffers; include q1/q8, odd-minimum and even-smooth sizes, and full-SHT stage time.
- Only pursue FFT changes if the measured whole-SHT opportunity is substantial. Prefer a localized improvement in existing RustSHT's FFT integration before adding another FFT engine or dependency. Keep unrelated RustSHT-FFT performance work separate. Test R2C/C2R, Nyquist/DC edge values, odd/even length, f32/f64 and exceptional-input fallback.

### B4. SIMD arithmetic and recurrence organization

- On **the executing warm, proven-safe route**, compare matching Rust Pulp V3 and DUCC `map2alm_kernel`/`alm2map_kernel` useful degree/lane work and actual AVX2/FMA assembly. DUCC's scalar analysis kernel shares coefficient broadcasts over vector blocks, keeps four parity/real-imag accumulators, and reduces them at degree groups; Rust's fused recurrence and accumulation have their own coefficient, scratch and reduction lifetime. Identify an actual critical-path or memory-traffic difference before redesigning arithmetic.
- Distinguish ordinary recurrence from scaled-prefix/transition work, safety proof on the cold call, horizontal reductions, register spills, coefficient staging and final coefficient correction. Follow source to executed assembly; static instruction counts, frame size, or a simpler loop are insufficient. Use hardware counters only when permitted; if `perf_event_paranoid` prevents access, label cache/IPC theories unverified and use bounded software probes.
- Previously unsuccessful unchanged ideas are **not candidates**: three accumulator chains, naive multi-degree unroll, packed complex reduction, direct output accumulation, indexed loop syntax changes, global smaller analysis tiles and broad range-check rewrites. A **different** algorithm requires new evidence and an independently tested mechanism.
- Treat the fixed aligned-scratch q8 **synthesis** improvement as an experimental lead if Plan 11 did not resolve it. Isolate exact code-generation, alignment, false sharing, scratch-capacity and memory effects across scalar analysis/synthesis. Do not promote a config flag or allocate substantially more scratch based on one q8 speedup.

### B5. Parallel topology control

- On the Ryzen 5950X, eight physical cores can fit within one eight-core CCD/L3 domain. Repeat only the diagnostically useful q8 cases on the **same pinned cores** for both libraries. If necessary compare one-CCD q8 with a cross-CCD q8 arrangement, documenting placement, not treating disparate CPU affinity as a library speedup. Check whether Rayon pool reuse and DUCC pool reuse have matching startup contracts.
- Explicitly distinguish speedup relative to each library's own q1 from Rust/DUCC q8 ratio; an intrinsically faster q1 kernel can still scale poorly, and vice versa.

## 6. Phase C — Validate and close with a narrow implementation

For each accepted candidate:

1. Keep the change isolated and identify the specific source-level mechanism it removes. Validate **same-work, warm and first-call** correctness before performance claims.
2. Test full `S/A/S*/A*` scalar operations as applicable, spin 0–4 regressions, GL/CC/F1, f32/f64, odd/even longitude counts, polar/high-order edge cases, subnormals, near-zero cancellations, huge finite and nonfinite inputs, SIMD tails, cold-safe/unsafe cache transitions, thread counts and concurrent independent workspaces. Compare to pinned DUCC and analytic/adjoint identities; do not loosen existing tolerances.
3. Run release/feature and no-default-features Rust test suites, strict Clippy/formatting and Python bindings tests **only if touched**. Track stack, heap, peak RSS, per-worker scratch and construction cost. Stable Rust, portable runtime Pulp dispatch and bounded caller-owned scratch remain requirements.
4. Commit only validated, evidence-backed changes. Cleanly remove failed cfg paths, redundant temporary kernels and large raw assembly dumps from production. Preserve small machine-readable timing datasets and the decisive diagnostic summary with exact source/toolchain revisions and reproduction commands.
5. Summarize remaining q1/q8 latency by **absolute time in each stage**, measured Rust/DUCC ratios, 30-batch uncertainty or individual-call counts, accepted/rejected alternatives, accuracy, memory and next justified work. If measured parity is not achieved, report the isolated residual and a tested follow-up mechanism; do not declare success from a 2–5% local change.

## 7. Execution and branch rules

- **Active branch/session:** remain on `opt/sht-performance-parity`.
  Plan 11 closeout `a37f74d6` was pushed and verified. `AGENTS.md`,
  `plans/README.md`, Plan 11 and this plan mark Plan 11 complete and
  Plan 14 active. No branch or session change is needed.
- Rebaseline the target cases from the final Plan 11 commit and use the
  staged measurements below to select work. Do not rerun the closed
  reduced-`mmax` frontend, cutoff or tile experiments.
- **Rebaseline rather than duplicate finished work.** The T2048
  cutoff, fixed 256 tile, q8 order-row preparation, and any additional
  resolved Plan 11 frontend work are accepted baseline. Compare current
  whole-transform latencies and select the most significant remaining
  small-serial or parallel deficit. Revise stale hypotheses and
  remove inapplicable benchmark requests before running them.
- **Execute the staged Plan 14 milestones** (baseline/stage attribution,
  small serial latency, remaining parallel excess, whole-SHT validation,
  documentation and cleanup) guided by measured absolute time. The
  sequence is a roadmap, not permission to repeat experiments with
  settled results or chase fixed acceptance percentages.
- **Commit and push on `opt/sht-performance-parity` after meaningful
  tested milestones**, following the existing `AGENTS.md` rule:
  inspect worktree, `git fetch origin`, rebase **only unpublished**
  local commits when safe, reread changed instructions, retest
  affected paths, push normally and verify remote HEAD.
  Never reset away local/concurrent work, force-push, rewrite
  published commits or push known numerical failures.
- Do not start Plans 12/13, add a Sol reviewer or begin unrelated
  work unless the owner subsequently requests it. Keep raw host
  diagnostics and oversized disassembly local/untracked.
- Final deliverables: reproducible native baseline, stage-level excess-time table, source/assembly explanation, small serial and q8 results with noise bounds, numerical and memory checks, concise plan completion report, and exact remaining bottlenecks.

## References

- RustSHT source: `src/transform.rs`, `src/legendre/scalar_paired.rs`, `src/legendre/scalar_ordered.rs`, `src/legendre/scalar_table.rs`, `src/fft.rs`, `src/workspace.rs`.
- Current repo instructions: `AGENTS.md`, `plans/11-ducc-equivalent-scalar-analysis.md`, `PERFORMANCE_STATUS.md`.
- Existing deferred scopes: `plans/12-cache-informed-legendre-dispatch.md`, `plans/13-fused-batched-sht.md`.
- DUCC 0.42.0 pinned reference: `https://github.com/mreineck/ducc/tree/42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; relevant code: `src/ducc0/sht/sht.cc`, `src/ducc0/sht/sht_inner_loop.h`, `src/ducc0/infra/threading.cc`.
- Martin Reinecke comments: `https://github.com/mreineck/ducc/issues/86#issuecomment-6294831374`, `https://github.com/mreineck/ducc/issues/86#issuecomment-6162674657`.
