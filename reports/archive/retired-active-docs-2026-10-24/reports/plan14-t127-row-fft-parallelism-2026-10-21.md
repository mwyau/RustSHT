# Plan 14: enable row FFT parallelism at T127 longitude sizes

**Baseline:** RustSHT `3e0734da` (including reflected-spectrum reuse).
**Candidate:** this report's source-and-evidence commit, changing only the
row-FFT geometry predicate from `nphi >= 257` to `nphi >= 255` while keeping
`ntheta >= 128`. **DUCC:** 0.42.0,
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Measured bottleneck and change

The matched T127 q8 table-analysis case was still 2.3–2.6x slower than DUCC
after the prior recurrence improvement. Test-only phase timing showed that
Rust used its serial row-FFT path at both `nphi=255` and `256`: the forward
FFT phase took 107.5 and 79.4 microseconds, respectively. At q1, the same
phases were about 80 microseconds. Table order-spectrum preparation also
remains serial at roughly 41–43 microseconds.

The existing parallel row-FFT implementation was already available, with
per-worker scratch prepared by the workspace. Extending its geometry gate to
`nphi >= 255` activates it for T127's odd-minimum 255 and smooth-even 256
longitude counts. q1 remains on the serial path because execution still
requires more than one worker. The threshold change affects only row
scheduling; each row retains the same FFT operation and arithmetic. It adds
no API and uses the existing bounded FFT worker workspaces.

The q8 forward-FFT phase then measured 24.4 microseconds at `nphi=255` and
18.8 microseconds at `nphi=256`. Table accumulation remained a separate
phase. A focused dispatch test covers the latitude and longitude boundary.

## Matched whole-transform results

Rust 1.99.1 / LLVM 23.1.1 / Pulp 0.22.3 used strict floating point and
`-C target-cpu=x86-64-v3`; runtime dispatch was Pulp V3 (AVX2/FMA). DUCC
SHT sources used GCC 15.2 with
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; its validation driver
used strict floating point. Both builds used the existing native full
transform harness, T127 GL, `ntheta=128`, full `mmax=127`, identical
deterministic inputs and output reset semantics.

The baseline and first candidate pass each used three interleaved process
pairs and 30 calibrated ~25 ms batches per backend and case. A second
candidate q8 pass used the same three-pair protocol after host runqueue wait
had fallen. The table uses the baseline and second candidate medians; the
first candidate pass produced similar q8 reductions. Ranges are batch p10–
p90. The repeated candidate DUCC run is listed as well because the source
rounds were interleaved separately.

| `nphi` | operation | Rust q8 before → after (µs, p10–p90) | DUCC q8 before → repeat (µs, p10–p90) | Rust/DUCC | excess before → after (µs) | Rust change | candidate relative L2 |
|---:|---|---:|---:|---:|---:|---:|---:|
| 255 | analysis | 209.55 (203.63–219.07) → 119.70 (117.34–128.72) | 79.88 (79.38–80.65) → 80.17 (79.14–83.02) | 2.623× → 1.493× | 129.67 → 39.53 | −42.9% | `8.52e-15` |
| 255 | synthesis | 221.18 (218.78–224.64) → 107.15 (105.61–110.40) | 75.97 (75.43–77.19) → 77.14 (76.18–82.08) | 2.911× → 1.389× | 145.21 → 30.01 | −51.5% | `6.89e-15` |
| 256 | analysis | 170.92 (169.53–173.30) → 106.99 (105.85–108.52) | 74.07 (73.42–75.29) → 74.11 (71.34–75.97) | 2.308× → 1.444× | 96.85 → 32.89 | −37.4% | `8.46e-15` |
| 256 | synthesis | 112.94 (110.82–116.61) → 82.80 (82.31–85.00) | 72.75 (71.12–73.84) → 73.35 (72.59–74.81) | 1.552× → 1.129× | 40.19 → 9.44 | −26.7% | `6.95e-15` |

The first candidate pass showed the same result: q8 Rust fell 36.8–51.8%
across these cases. Its DUCC medians were stable. q1 stayed serial and its
before/after medians differed by 0.2–1.9%, within observed process
variation; its T256 synthesis p90 included a host outlier. Full-transform
output validation covered all coefficients and maps. Maximum absolute error
was `1.18e-15` for analysis and `3.41e-12` for synthesis.

## Host and setup observations

The host was an AMD Ryzen 9 5950X. q1 used one physical core and q8 used
physical cores 0–7. Median runqueue wait was 2.28% in the baseline, 2.66%
in the first candidate, and 1.89% in the repeat. The first two runs had p90
wait near 26.9%, while the quieter repeat had p90 8.1%. The candidate repeat
confirms the large reduction under lower interference, though the machine
was not completely isolated.

q8 plan/workspace setup remained about 0.50–0.63 ms. Enabling row
parallelism makes the already-supported worker FFT workspaces available at
these sizes; first-call timings remained around 4.2 ms for analysis. There
was no meaningful q1 regression or numerical change.

## Validation and next work

The T127 native Rust/DUCC comparison passed for both operations, odd and
even longitude counts, q1 and q8. Relative L2 was at most `8.53e-15`. The
focused row-FFT predicate test, all-feature and no-default release suites,
strict Clippy, and formatting passed. No numerical tolerances or fallback
rules changed.

This closes the selected T127 row-FFT scheduling opportunity; odd q8
analysis still has about 40 microseconds excess, and table order-spectrum
preparation remains serial. Return to the larger Plan 14 q8 target: T720
odd-length synthesis, whose prior measured whole-transform excess was about
0.407 ms. The existing C2R replacement candidate was slower and remains
rejected; attribute a new candidate before changing the production FFT.

Reproduce with the pinned source and the script
`scripts/benchmark_ducc_full_transform_parity.py`, using T127, `mmax=127`,
`ntheta=128`, both `odd_minimum` and `even_smooth` modes, q1/q8, and separate
baseline/candidate result directories. The committed
[`comparison.csv`](data/plan14-row-fft-parallelism-2026-10-21/comparison.csv)
contains per-case medians, p10/p90, errors, setup and first-call values for
the baseline, first candidate pass, and quieter q8 repeat.
