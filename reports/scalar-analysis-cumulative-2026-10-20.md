# Scalar-analysis experimental results — current evidence index

**Disposition (2026-10-21):** The preceding 40k-character cumulative
experiment log is now archived
[verbatim as a historical snapshot](archive/experiments/scalar-analysis-cumulative-2026-10-20-snapshot.md).
This concise report keeps the **measured current conclusions** and links
to the original machine-readable files; previous candidate proposals and
“next” directions in the archive are retired. **Current instructions are
[Plan 11](../plans/11-ducc-equivalent-scalar-analysis.md), not this report.**

## T2047/T2048 cutoff and pair-tile comparison (2026-10-20)

Fixed native f64 GL STANDARD **analysis** geometry:
`mmax=2047`, `nphi=4096`; Ryzen 5950X; Rust 1.99.1
strict x86-64-v3/Pulp V3; DUCC `42ce8ab5` with GCC 15.2
native V3 `-O3 -ffast-math -ffp-contract=fast`.
Data: [cutoff study README and summary CSV](data/scalar-analysis-cumulative-2026-10-20/t2048-cutoff-current/).

| Geometry | Threads | DUCC | Rust tile 256 / 128 / 64 | Paired ratio tile 256 / 128 / 64 |
| --- | ---: | ---: | ---: | ---: |
| T2047, 2048 latitudes | q1 | 248.645 ms | 251.498 / 261.872 / 282.107 ms | 1.013 / 1.050 / 1.125 |
| T2047, 2048 latitudes | q8 | 45.793 ms | 62.201 / 63.074 / 67.679 ms | 1.371 / 1.388 / 1.468 |
| T2047, 2049 latitudes | q1 | 249.445 ms | 828.435 / 718.965 / 696.002 ms | 3.326 / 2.888 / 2.781 |
| T2047, 2049 latitudes | q8 | 45.928 ms | 143.732 / 129.008 / 122.973 ms | 2.926 / 2.741 / 2.647 |
| T2048, 2049 latitudes | q1 | 250.061 ms | 832.444 / 716.284 / 693.156 ms | 3.328 / 2.857 / 2.774 |
| T2048, 2049 latitudes | q8 | 42.401 ms | 133.959 / 121.908 / 115.758 ms | 3.198 / 2.899 / 2.752 |

Paired ratios come from the interleaved per-process measurements;
they need not equal a ratio of individually rounded medians.
Three interleaved rounds gave 15 whole-call measurements per long
backend/tile case and 30 calibrated short-case q8 batches.
Maximum relative L2 error `1.18e-13`; all worker reset checks passed.
The timing data include host contention diagnostics, especially at q8.

A single additional latitude changes Rust's prefix eligibility
and raises whole-call latency dramatically; fixing `lmax` while
changing only `ntheta` establishes that adding harmonic degree
2048 is **not** the cause. Route counts at tile 256 change from
2,432 prefix-enabled blocks (`ntheta=2048`) to 2,434
checked-scaled blocks (`ntheta=2049`). RustSHT's *extra*
`ntheta * 2^-60 <= 8*EPSILON` global gate is absent from
DUCC's `iter_to_ieee`. At the time of this baseline, removing that gate had
**not yet been validated** as a safe production change. The follow-up below
records the controlled validation and supersedes that status; independent
input-range, scale, polar/subnormal and fallback checks remain in place.

**Tile conclusion:** 256 is the **retained production default**;
smaller tiles only recover a fraction of the disabled-prefix
slowdown and remain much slower than DUCC. The existing 64/128
comparisons are historical diagnostics, **not a request to run them
again**. The original cutoff note deferred a 512 trial until after the
prefix issue; it remains optional only if a separate cache/setup hypothesis
justifies expanding and validating fixed scratch capacity.

## T2048 fixed-`sht_ftol` follow-up (2026-10-20)

Commit `0bc5f153` removes only RustSHT's global condition
`latitude_count * ANALYSIS_FTOL <= 8 * f64::EPSILON`. The threshold remains
exactly `2^-60`; finite-value, exponent, recurrence-range, coefficient
product and transactional fallback checks remain unchanged. Production tile
size remains 256 reflected latitude pairs.

Pinned DUCC 0.42 uses `sht_ftol=2^-60` in `init_lambda` and `iter_to_ieee`.
`rescale` advances lanes according to their scaled recurrence state, and
`calc_map2alm` handles mixed scaled/ordinary lanes before ordinary SIMD
accumulation. It has no latitude-count multiplier. Rust's removed gate was a
stricter error budget for the worst-case coherent sum of omitted prefix
contributions, not a finite-value or exponent-safety invariant. The
validated Rust route retains its independent safety checks and decline
fallbacks. The cases below found no need for the extra bound; this is
empirical coverage, not a claim of bitwise identity for every possible input.

The matched before/after full-transform matrix uses Rust 1.99.1, Pulp V3,
DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2 with
DUCC SHT flags `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, and a
strict validation driver. Baseline source was `86f43486`; candidate source
was `0bc5f153`. Geometry is f64 GL STANDARD analysis with `mmax=2047`,
`nphi=4096`, three interleaved process rounds, 15 whole-call observations
per q1 backend/tile and 30 calibrated q8 batches. Ratios below pair each
Rust result with that phase's DUCC samples; small DUCC shifts between phases
are measurement variation.

| Geometry | q | Rust before → after (ms) | Rust/DUCC before → after | Rust latency change |
|---|---:|---:|---:|---:|
| T2047, 2048 latitudes | 1 | 251.50 → 254.59 | 1.011× → 1.033× | +1.2% |
| T2047, 2048 latitudes | 8 | 62.20 → 58.60 | 1.358× → 1.384× | −5.8% |
| T2047, 2049 latitudes | 1 | 828.43 → 254.64 | 3.321× → 1.036× | −69.3% |
| T2047, 2049 latitudes | 8 | 143.73 → 57.31 | 3.129× → 1.361× | −60.1% |
| T2048, 2049 latitudes | 1 | 832.44 → 255.40 | 3.329× → 1.029× | −69.3% |
| T2048, 2049 latitudes | 8 | 133.96 → 58.32 | 3.159× → 1.383× | −56.5% |

The one-latitude performance discontinuity is removed. On the already-fast
T2047/2048-latitude control, q1 is within its observed spread; the q8 median
was lower in this run. The remaining q8 whole-transform excess at 2049
latitudes is about 1.36–1.38×, down from 3.13–3.16×.

After fixing the prefix route, the one permitted tile diagnostic still
favors 256 at the 2049-latitude shapes:

| Geometry | q | Tile 256 | Tile 128 | Tile 64 |
|---|---:|---:|---:|---:|
| T2047, 2049 latitudes | 1 | **254.64 ms** | 263.54 ms | 280.66 ms |
| T2047, 2049 latitudes | 8 | **57.31 ms** | 59.18 ms | 61.86 ms |
| T2048, 2049 latitudes | 1 | **255.40 ms** | 263.57 ms | 282.08 ms |
| T2048, 2049 latitudes | 8 | **58.32 ms** | 58.56 ms | 60.89 ms |

Tiles 64/128 are diagnostic only; no dispatch change is justified, and no
further tile sweep is planned. Route counters at tile 256 now report
2,432/2,434/2,434/5,780 prefix-scaled blocks and zero checked-scaled blocks
for T2047/2048-latitude, T2047/2049-latitude, T2048/2049-latitude and
T3072/3073-latitude cases, respectively. Initialization and recurrence
decline counters are zero in all four.

The T3072 full-order GL check used `mmax=3072`, `ntheta=3073`, `nphi=6174`.
Its one-process, five-call q1 screen measured analysis at 775.8 ms Rust vs
735.1 ms DUCC (1.056×), and synthesis at 1121.4 vs 1152.6 ms (0.973×); these
timings are directional rather than a full benchmark campaign. Random
analysis/synthesis relative L2 errors were `1.96e-13`/`2.11e-13`, with
absolute maxima `1.44e-15`/`1.07e-8`. Rust adjoint residuals were
`1.19e-15` for synthesis and `1.10e-14` for analysis. Zero outputs matched
exactly; extreme and subnormal cases remained finite, with subnormal outputs
matching DUCC exactly. The `(3072,0)` and `(3072,3072)` basis maps differed
from DUCC by relative L2 `1.26e-11` and `2.24e-15`. Antisymmetric north/south
maps at T2047, T2048 and T3072 produced exact zero for every even-degree
`m=0` coefficient in both libraries; maximum Rust/DUCC relative L2 errors
were `1.80e-13`, `1.16e-13` and `1.89e-13`.

Compact before/after timings and errors are in
[`t2048-ducc-fixed-ftol/summary.csv`](data/scalar-analysis-cumulative-2026-10-20/t2048-ducc-fixed-ftol/summary.csv)
and [`validation-summary.json`](data/scalar-analysis-cumulative-2026-10-20/t2048-ducc-fixed-ftol/validation-summary.json).
The T2047/T2048 q1/q8 full-transform checks passed finite-output, reset,
zero, extreme, subnormal, basis, roundtrip and adjoint checks; maximum random
relative L2 error was `1.18e-13` for analysis and `1.28e-13` for synthesis.
Release tests passed (230); no-default-feature release tests passed (192);
formatting, all-target/all-feature Clippy and the T2047/T2048/T3072 route test
passed. The subsequent q8 phase profile is recorded below; do not repeat the
cutoff or tile experiment.

## Current q8 stage profile and whole-transform comparison

Current source is `6948ecdb`; Rust 1.99.1/LLVM 23.1.1, strict Rust
`-C target-cpu=x86-64-v3` with runtime Pulp V3 (four f64 lanes), compared with
DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. DUCC's SHT translation
unit used GCC 15.2 and `-O3 -ffast-math -ffp-contract=fast
-march=x86-64-v3`; its validation driver remained strict. Rust's phase probe
used the real production analysis path, reusable workspace, one warm call and
five timed whole calls per case. These five-call phase medians identify where
time is spent but are not the calibrated whole-transform estimates below.

| lmax / threads | Rust FFT | Rust order-spectrum preparation | Rust recurrence + accumulation | Rust whole call |
|---|---:|---:|---:|---:|
| T720 q1 | 6.949 ms | 0.813 ms | 12.445 ms | 20.316 ms |
| T720 q8 | 0.893 ms | 0.781 ms | 1.735 ms | 3.544 ms |
| T2047 q1 | 24.256 ms | 26.443 ms | 206.799 ms | 259.718 ms |
| T2047 q8 | 5.525 ms | 23.523 ms | 28.256 ms | 59.616 ms |
| T3072 q1 | 64.189 ms | 38.744 ms | 659.959 ms | 766.162 ms |
| T3072 q8 | 17.215 ms | 40.515 ms | 91.285 ms | 152.412 ms |

For the direct DUCC decomposition, `map2leg` covers Fourier extraction and
ring weighting; `leg2alm` covers its order-oriented ring packing and Legendre
recurrence/accumulation. The boundaries do not exactly match Rust's separate
order-spectrum preparation pass.

| lmax / threads | DUCC `map2leg` | DUCC `leg2alm` |
|---|---:|---:|
| T720 q1 | 8.617 ms | 10.821 ms |
| T720 q8 | 1.218 ms | 1.600 ms |
| T2047 q1 | 28.433 ms | 190.158 ms |
| T2047 q8 | 5.165 ms | 28.329 ms |
| T3072 q1 | 71.745 ms | 606.329 ms |
| T3072 q8 | 13.066 ms | 85.937 ms |

DUCC numbers are medians from calibrated direct-stage batches (30 batches for
short stages; 15 individually timed calls when a stage exceeded 100 ms), one
process per case. Rust's five-call phase probe is less robust and some T3072
q8 phase samples show host-frequency variation; interpret this table as stage
attribution, not a final kernel parity claim. Rust `Legendre recurrence and
accumulation` includes dynamic order scheduling at q8. DUCC `leg2alm` includes
its order scheduling and `fill_m2a` packing.

Matched production-dispatch full-transform analysis measurements provide the
stronger comparison:

| Geometry | Threads | Rust ms | DUCC ms | Rust/DUCC | Excess ms | Relative L2 error |
|---|---:|---:|---:|---:|---:|---:|
| T720, mmax 720, nphi 1442 | q1 | 20.332 | 19.959 | 1.019x | 0.373 | `4.66e-14` |
| T720, mmax 720, nphi 1442 | q8 | 3.597 | 2.915 | 1.234x | 0.682 | `4.66e-14` |
| T2047, mmax 2047, nphi 4096 | q1 | 254.644 | 245.730 | 1.036x | 8.914 | `1.18e-13` |
| T2047, mmax 2047, nphi 4096 | q8 | 57.310 | 42.121 | 1.361x | 15.190 | `1.18e-13` |
| T3072, mmax 3072, nphi 6174 | q1 | 768.713 | 736.621 | 1.044x | 32.092 | `1.96e-13` |
| T3072, mmax 3072, nphi 6174 | q8 | 144.337 | 117.933 | 1.224x | 26.404 | `1.96e-13` |

T720 used three interleaved process pairs and 30 calibrated batches per
backend/case. T2047 used three process rounds, 15 long q1 calls and 30 q8
batches. T3072 used three interleaved process pairs with five individually
timed long calls per process and backend; one repeat screen had occasional
outliers despite low process wait fractions, so treat its spread as host
variability. The q8 excess is real at T2047/T3072, but recurrence alone does
not explain it: T2047's phase recurrence is 28.256 ms against DUCC's 28.329 ms,
and T3072's is 91.285 against 85.937 ms. Meanwhile Rust's order-spectrum
preparation remains nearly serial from q1 to q8 (26.443 to 23.523 ms at
T2047; 38.744 to 40.515 ms at T3072). DUCC performs eligible-ring packing
inside dynamically scheduled order work. This identifies the largest
source-level candidate as parallelizing Rust's independent order-spectrum
rows; it does not predict that the whole q8 gap will disappear. The first
implementation comparison will leave table-path product checks and all
recurrence/safety behavior unchanged.

T720 whole-call source, distributions and cold/warm data are in
[`t720-ab-current`](data/scalar-analysis-cumulative-2026-10-20/t720-ab-current/);
T2047 distributions and validation are in
[`t2048-ducc-fixed-ftol`](data/scalar-analysis-cumulative-2026-10-20/t2048-ducc-fixed-ftol/).
T3072 timing samples and host diagnostics were kept locally because they
contain process/host detail.

## q8 independent-order preparation candidate (2026-10-20)

Source baseline was `6948ecdb`; the implementation is `dfed3ff7` and the
inlinable preparation context cleanup is `3b32f2de`. For recurrence plans,
the candidate prepares disjoint order-major spectrum rows using the existing
Rayon pool and workspace capacity. Each row retains the same phase, weight,
and product operation order. Compact-table safety preparation remains
serial. The Legendre recurrence, fallback checks, 256-pair tile, and fixed
`2^-60` prefix threshold are unchanged. At q1, where the pool capacity is
one, preparation remains serial.

The focused phase probe measured order preparation at approximately
0.25/6.4/15.4 ms for T720/T2047/T3072 q8, compared with 0.781/23.523/40.515
ms before the change. This probe used one warm call and five timed calls and
is stage attribution, not the primary performance result.

The current-source whole-transform screen used Rust `3b32f2de`, the same
Rust 1.99.1/Pulp V3 strict build and DUCC 0.42.0/GCC 15.2 flags described
above, deterministic full-triangle inputs, reusable plans/workspaces, three
interleaved Rust/DUCC process pairs, and q8 workers pinned to the same eight
physical cores. Each backend/case collected 30 calibrated approximately
25 ms batches, except T3072 full-mmax whose operations exceeded 100 ms and
were measured as five whole calls per process (15 samples). Output comparison
was outside timing. Reported ratio is the ratio of medians; paired batch
ratios and p10/p90 are also retained in the summary data.

| Case | Rust ms [p10,p90] | DUCC ms [p10,p90] | Median ratio | Paired ratio median [p10,p90] | Excess ms | Max absolute error | Relative L2 |
|---|---:|---:|---:|---:|---:|---:|---:|
| T720, m720, nphi1442 | 3.081 [2.966,3.226] | 2.888 [2.840,3.102] | 1.067x | 1.053 [0.996,1.114] | +0.193 | 1.04e-15 | 4.66e-14 |
| T720, m180, nphi1442 | 1.848 [1.801,1.953] | 2.016 [1.970,2.171] | 0.917x | 0.927 [0.829,0.985] | -0.168 | 1.11e-15 | 4.93e-14 |
| T2047, m2047, nphi4096, ntheta2049 | 42.655 [41.639,46.416] | 42.515 [41.199,51.367] | 1.003x | 1.012 [0.828,1.100] | +0.140 | 1.38e-15 | 1.18e-13 |
| T2047, m511, nphi4096, ntheta2049 | 21.399 [20.975,23.236] | 17.425 [16.593,18.992] | 1.228x | 1.248 [1.139,1.332] | +3.974 | 1.11e-15 | 1.26e-13 |
| T3072, m3072, nphi6174 | 122.835 [122.379,130.428] | 117.828 [115.233,118.998] | 1.042x | 1.060 [1.029,1.114] | +5.007 | 1.44e-15 | 1.96e-13 |
| T3072, m768, nphi6174 | 64.306 [63.197,65.840] | 57.315 [55.693,59.070] | 1.122x | 1.125 [1.097,1.161] | +6.991 | 1.39e-15 | 2.08e-13 |

Against the pre-change whole-transform q8 screen on full `mmax`, Rust latency
fell from 3.597 to 3.081 ms at T720 (-14.4%), 57.310 to 42.655 ms at T2047
(-25.6%), and 144.337 to 122.835 ms at T3072 (-14.9%). The corresponding
Rust/DUCC ratios moved from 1.234x/1.361x/1.224x to 1.067x/1.003x/1.042x.
The T2047 median distributions overlap broadly; the DUCC first process pair
was an outlier (50.7 ms versus about 42 ms in the later pairs), so the
near-equal median is not evidence of stable Rust superiority. The full-order
q8 excess is now small relative to measurement spread at T2047 and about
5 ms at T3072. The clearest remaining excess is in reduced-order cases:
about 4.0 ms at T2047 m511 and 7.0 ms at T3072 m768.

For reduced `mmax`, a direct stage screen found Rust recurrence/accumulation
near DUCC `leg2alm` (T2047 m511: approximately 13.43/13.37 ms; T3072 m768:
44.1/42.85 ms). Rust's forward FFT plus order-spectrum preparation was
approximately 7.15 ms at T2047 m511 and 19–23 ms at T3072 m768, while DUCC
`map2leg` was approximately 3.50 and 9.32 ms. Those stage boundaries differ:
DUCC `map2leg` combines FFT extraction and ring weighting, so this identifies
the front end as the next investigation rather than assigning its entire
excess to one exact substage. Rust currently computes and retains all
`nphi/2+1` real-FFT bins per ring even when only orders through `mmax` enter
the scalar recurrence. Do not start a wholesale FFT rewrite; first separate
finite-input scanning, FFT execution, and output/weight packing costs at the
production call boundary.

Host conditions were not fully quiet. CPU frequency varied, and one T720
Rust process recorded 474 involuntary context switches; the distributions
show that variation. Numerical comparisons remained stable: across the six
cases relative L2 error was at most `2.08e-13` and maximum absolute
difference at most `1.44e-15`. Exact per-batch summaries are in
[`order-prep-q8-candidate/summary.csv`](data/scalar-analysis-cumulative-2026-10-20/order-prep-q8-candidate/summary.csv).

## Reduced-order real-FFT output pruning (2026-10-20)

Commit `41c7223f` adds an internal `forward_truncated` path for scalar GL
analysis. Rust still scans every map sample for finiteness/headroom and runs
the complete real-input FFT; it now recombines and stores only modes
`0..=mmax`, which are the only Fourier orders consumed by scalar GL analysis.
The existing workspace row stride and all other grid/spin FFT calls retain
their full spectra. This is a bounded output-pruning change, not an FFT
algorithm rewrite. The full and truncated paths produce bitwise-identical
Rust coefficients for every q8 case compared here.

The before build was `3b32f2de`; the candidate was `41c7223f`. Both used Rust
1.99.1/Pulp V3 strict x86-64-v3 and the same pinned DUCC 0.42.0/GCC 15.2
build. The six geometries, deterministic full-triangle fixtures, output
layout, reused plans/workspaces, 25 ms target batches and three interleaved
process pairs were identical. q8 used the same eight physical cores. Cases
under 100 ms collected 30 batches per backend; T3072 full-mmax used 15 whole
calls per backend. Output errors were computed after timing. The baseline and
candidate screens were separate runs, each interleaved against DUCC, so the
Rust before/after delta is subject to between-run host variation.

| q8 case | Rust before ms [p10,p90] | Rust after ms [p10,p90] | Rust change | DUCC after ms | After Rust/DUCC | After excess | Paired ratio [p10,p90] |
|---|---:|---:|---:|---:|---:|---:|---:|
| T720, m720 | 3.081 [2.966,3.226] | 3.063 [2.988,3.248] | -0.6% | 2.937 | 1.043x | +0.127 ms | 1.052 [1.010,1.123] |
| T720, m180 | 1.848 [1.801,1.953] | 1.728 [1.668,1.836] | -6.5% | 2.002 | 0.863x | -0.274 ms | 0.864 [0.820,0.919] |
| T2047, m2047 | 42.655 [41.639,46.416] | 42.907 [42.284,49.223] | +0.6% | 42.563 | 1.008x | +0.344 ms | 1.023 [0.964,1.135] |
| T2047, m511 | 21.399 [20.975,23.236] | 18.739 [18.271,20.185] | -12.4% | 17.064 | 1.098x | +1.675 ms | 1.099 [0.981,1.180] |
| T3072, m3072 | 122.835 [122.379,130.428] | 124.428 [123.016,125.892] | +1.3% | 117.119 | 1.062x | +7.309 ms | 1.064 [1.026,1.092] |
| T3072, m768 | 64.306 [63.197,65.840] | 59.266 [58.633,59.999] | -7.8% | 58.253 | 1.017x | +1.013 ms | 1.019 [0.895,1.042] |

The reduced-order benefit matches the dataflow change: T2047 m511 previously
materialized 2,049 bins per ring but uses 512; T3072 m768 materialized 3,088
but uses 769. The candidate reduced q8 excess by 2.30 ms at T2047 and 5.98 ms
at T3072. Full-order shifts are within the observed spread, including a
small T3072 increase, so there is no measured full-order gain. The remaining
reduced-order medians are 1.10x at T2047 m511 and 1.02x at T3072 m768; their
paired ranges are broad and should not be interpreted as exact parity.

The q1 candidate screen found median Rust/DUCC ratios 1.003x/0.805x at T720
full/reduced `mmax`, 1.013x/1.019x at T2047, and 1.028x/0.988x at T3072.
There is no q1 before/after baseline from the same source pair, so these are
checks against current DUCC only. One q1 process had elevated scheduler wait;
distributions and host counters are retained in the local raw result.
Across candidate q1/q8 comparisons, maximum relative L2 error remained
`2.08e-13` and maximum absolute error `1.44e-15`; q8 Rust coefficients were
bitwise identical before and after pruning.

Reproduction: build with
`RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' cargo build --release --features parallel --example ducc_full_transform_parity`,
then run the repository's `scripts/benchmark_ducc_full_transform_parity.py`
for each `--lmax`/`--mmax`/fixed-geometry row in the table, pinned to the
same DUCC revision and output directory. That harness uses the same native
workers, calibrated-batch method, affinity helper, fixture generator and
post-timing output comparison. The compact q1/q8 results are in
[`truncated-output-candidate/summary.csv`](data/scalar-analysis-cumulative-2026-10-20/truncated-output-candidate/summary.csv).

## Current T720 production/candidate A/B (2026-10-20)

Source `b25c011d`; identical f64 GL scalar A/S,
`mmax=180/720`, odd-minimum and even-smooth longitude shapes,
q1/q8. Three interleaved process pairs, 30 timed batches per
backend/shape. Compared production, fixed aligned scratch,
packed complex reduction, and their combination against pinned
DUCC native V3. Maximum relative L2 error `5.20e-14`.
[Full results, including first calls and uncertainty](data/scalar-analysis-cumulative-2026-10-20/t720-ab-current/).

- Fixed scratch alone changed **analysis** latency by approximately
  -2.8% to +0.2% across T720; it had an independently confirmed
  **q8 synthesis** improvement of **11.4–18.0%** over four cases
  (five interleaved process pairs, 50 batches/case, low runqueue wait).
  q1 synthesis was near-neutral. This is still an **opt-in candidate**;
  its mechanism, total workspace/memory cost and generality are unresolved.
- Packed complex reduction improved the isolated kernel by a little,
  but full-call analysis was only -2.0% to +0.4% alone, inconsistent;
  **removed**.
- Direct coefficient accumulation caused large T720 analysis
  regressions in an earlier cumulative experiment; **removed**.
- Safe indexed loop reduced instructions without a verified
  whole-transform performance advantage; **removed**.
- Sectoral seed-prefix and scaled-epoch/preflight reuse are in
  current production and common to the matched builds; their
  *individual* benefits were not measured in that A/B.

A combined earlier candidate build with retired features gave
large regression on several T720 analysis shapes. **Do not conflate
that obsolete build with current production or resurrect its flags.**
Review the full archived [experiment narrative](archive/experiments/scalar-analysis-cumulative-2026-10-20-snapshot.md)
only when a specific source or provenance question requires it.

## Further historical experiments and provenance

- [Archived exact 2026-10-20 cumulative log](archive/experiments/scalar-analysis-cumulative-2026-10-20-snapshot.md):
  earlier kernel, reduction, range, direct-output, epoch,
  setup and route-profiling experiments, including rejected
  implementations and their source/benchmark conditions.
- [Post-upgrade GCC/Clang baseline](archive/experiments/ducc-upgrade-baseline-2026-10-20.md)
  and [historical full-bandlimit sweep](archive/experiments/ducc-full-transform-parity-2026-10-20.md)
  use separately specified source/flag versions; do not merge
  their values into a current-source trend without matched reruns.
- [Current factual checkpoint](../PERFORMANCE_STATUS.md),
  [active Plan 11](../plans/11-ducc-equivalent-scalar-analysis.md).

No archived experiment or older "next" paragraph is authorization
to retry a known rejected tile/reduction experiment. The prefix cutoff
investigation is complete; the next target is the measured q8 reduced-order
analysis front end described above.

## Plan 11 reduced-mmax frontend closeout (2026-10-21)

The final frontend study uses the production output-pruning candidate at
`1fddce4a75b0a0c3ba5ee5825d2cbaa50a42b0ed`, Rust 1.99.1 / LLVM 23.1.1,
strict `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'`, runtime
Pulp V3, and DUCC 0.42.0 at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. DUCC SHT sources used GCC
15.2 with `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the
benchmark driver used `-fno-fast-math`. Both ran on the Ryzen 9 5950X.
Single-thread screens were pinned to physical CPU 0; q8 screens used
physical CPUs 0–7. CPU pressure stayed low on average, though two q8
T3072 phase samples were visibly slower than the other three.

For Rust, five warmed production-path calls per case were profiled. Temporary
test-only timers separated the existing full-row finite/headroom scan and
FFT-input copy, the half-length complex FFT, and output recombination. The
reported q8 substage time is the maximum worker-local accumulation, while
the forward-FFT phase is wall time; these values are diagnostic, not
calibrated kernel benchmarks. The counters were removed after collection.
The DUCC native stage harness collected 30 calibrated batches per stage,
with per-batch repetitions selected to reach about 25 ms. DUCC `map2leg`
includes ring weighting and row FFT; `leg2alm` includes `fill_m2a` packing
and recurrence. These stage boundaries are similar but not identical.

| Shape / threads | Rust input scan + copy | Rust complex FFT | Rust recombination | Rust forward wall | Rust order prep | Rust recurrence | DUCC `map2leg` | DUCC `leg2alm` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| T2047 m511 q1 | 6.03 ms | 7.25 ms | 2.84 ms | 16.32 ms | 5.21 ms | 98.62 ms | 25.60 ms | 94.37 ms |
| T2047 m511 q8 | 1.48 ms | 1.03 ms | 0.49 ms | 3.05 ms | 1.35 ms | 13.78 ms | 3.59 ms | 13.50 ms |
| T3072 m768 q1 | 13.25 ms | 25.88 ms | 6.37 ms | 45.76 ms | 8.64 ms | 318.38 ms | 64.90 ms | 306.05 ms |
| T3072 m768 q8 | 3.01 ms | 3.85 ms | 1.02 ms | 7.99 ms | 4.56 ms | 45.23 ms | 9.45 ms | 43.03 ms |

The full input pass remains necessary: every longitude sample participates
in the row FFT, and Rust must retain its finite/headroom validation. It is
already fused with the copy into FFT workspace. The candidate still performs
the complete FFT, but recombines only 512 of 2,049 bins at T2047 m511 and 769
of 3,088 bins at T3072 m768. Its forward wall stage is below DUCC `map2leg`
in all four diagnostic cases, though DUCC folds ring weighting into that
stage. This does not support the forward FFT or safe input scan as the
remaining dominant source of whole-transform excess.

Order-spectrum preparation is a separate Rust pass that applies the
ring/order factors and creates order-major data; DUCC performs its related
packing within `leg2alm`. The stage probes therefore cannot be subtracted
as if they were identical CPU counters. Their combined serial timings are
close at T2047; the T3072 q8 samples include contention-sensitive recurrence
outliers. The calibrated full-transform data remain the comparison of
record: q8 T2047 m511 is 18.739 ms Rust vs 17.064 ms DUCC (+1.675 ms,
paired ratio p10–p90 0.981–1.180); q8 T3072 m768 is 59.266 vs 58.253 ms
(+1.013 ms, paired ratio p10–p90 0.895–1.042). q1 T2047 is 122.010 vs
119.737 ms, while q1 T3072 Rust is faster at 380.152 vs 384.761 ms. The
q8 residual is small and the paired ranges overlap; this study found no
additional frontend change with a demonstrated material whole-transform
benefit. Do not remove the finite scan or launch an FFT rewrite on this
evidence.

The existing output pruning is numerically exact relative to the prior
Rust output in all measured q8 cases. Against pinned DUCC the maximum
relative L2 error is `2.08e-13` and the maximum absolute error is
`1.44e-15`; release parallel and no-default-feature suites, including
FFT exceptional-value, analytic and adjoint tests, passed. Detailed medians
and whole-transform uncertainty are in
[`reduced-frontend-attribution-2026-10-21.csv`](data/scalar-analysis-cumulative-2026-10-20/reduced-frontend-attribution-2026-10-21.csv).
Reproduce the Rust broad phases with the ignored
`scalar_analysis_phase_profile_selected` test and the environment variables
`RUSTSHT_PROFILE_Q`, `RUSTSHT_PROFILE_LMAX` and `RUSTSHT_PROFILE_MMAX`;
run the DUCC stage profile with `(lmax, ntheta, nphi, threads, mmax)` for
the rows above. The matched whole-transform harness and commands remain
listed in the preceding output-pruning section.

**Plan 11 is ready to close.** Keep `forward_truncated` as the validated
reduced-order path. Defer further frontend redesign: the remaining q8
whole-call differences overlap the measured spread, and the separated
profile does not isolate a safe additional optimization with sufficient
impact. Continue with the Plan 14 small-serial and large-parallel baseline
from this final code.
