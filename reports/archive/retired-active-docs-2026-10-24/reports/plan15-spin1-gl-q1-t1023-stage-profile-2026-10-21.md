# Plan 15 GL q1 synthesis stage profile at T1023

**Purpose:** close the q1 GL synthesis attribution step before moving to CC.
This is a focused profile of the largest q1 GL spin-1 synthesis excess in the
published native baseline; it does not change production code.

## Matched whole-transform result

The case is f64 spin-1 STANDARD synthesis, `lmax=mmax=1023`, exact GL
`ntheta=1024`, `nphi=2048`, `phi0=0.23`, q1 pinned to one physical core (the
same core for both backends). Rust source was
`a218c4a49005b0e5bbb9dadf080cd0d6b231c32d`; DUCC 0.42.0 was pinned to
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Both used the existing matched
native benchmark and deterministic fixtures. Three interleaved process pairs
each measured five individual long calls per backend.

| Backend | Warm median | p10–p90 | Samples | First-call median |
|---|---:|---:|---:|---:|
| RustSHT | 261.848 ms | 261.018–263.682 ms | 15 | 271.277 ms |
| DUCC | 120.307 ms | 118.980–121.846 ms | 15 | 121.961 ms |

The aggregate ratio is `2.177x`, with paired process-median ratios
`2.152x, 2.183x, 2.183x`; excess is `141.541 ms`. RustSHT relative L2 error
was `3.903e-13` and maximum absolute error was `5.528e-9`. The q1 runqueue
wait medians were 1.73% for RustSHT and 0.05% for DUCC, while median sampled
frequencies were 4.73 and 4.68 GHz. This small host asymmetry does not explain
the observed latency gap. The earlier Plan 15 baseline at source `549bd2f0`
was 260.559 vs 121.636 ms (+138.923 ms), so the newer source did not introduce
a material regression in this case.

## Stage attribution

The Rust test-only phase probe used three warmups and eight measured calls.
Its phase medians were 2.431 ms for latitude/spectrum setup, 315.621 ms for
order scheduling plus Legendre recurrence/spectral accumulation, and
10.004 ms for inverse FFT/writeback. The instrumented phase values are not
additive to or directly comparable with the independent production whole-call
timings; the phase probe includes test instrumentation. They establish that
the large Rust cost is in the order/recurrence stage, not the inverse FFT.

Temporary per-route counters subdivided the broad fallback bucket (worker-sum
medians):

| Route or work | Median | Calls / blocks |
|---|---:|---:|
| Paired fused recurrence and accumulation | 88.379 ms | 9,488 successful blocks |
| Staged transition recurrence and accumulation | 162.808 ms | 2,910 calls |
| Polar split transition | 15.376 ms | 130 calls |
| Final generic degree callback | 5.364 ms | 130 calls |
| Paired coefficient preparation | 16.325 ms | — |
| Order factors and finite scans | 3.402 / 1.588 ms | — |

There were 10,596 pair-eligible blocks and 3,040 outer fallback blocks. The
staged transition count is 2,910 because 130 polar blocks take the split path.
All 2,910 staged calls completed the recurrence and accumulated their spectra;
the generic degree callback was confined to the 130 polar-split blocks. Thus
the large combined fallback timer mostly measures useful staged recurrence,
not replay of degree work already completed. The paired fused path declined or
was ineligible on the remaining blocks. DUCC's direct `alm2leg` samples had a
105.872 ms median (45 samples); its separate `leg2map` median was 12.389 ms
(90 samples). These direct-stage timings identify the corresponding DUCC
work but are kept separate from the whole-call comparison.

The source difference to investigate is the cost of Rust's order-oriented
block transitions and staged scaled-to-ordinary recurrence versus DUCC's
active-ring packing and shared order traversal. This profile does not support
removing the checked transition or callback. No GL production change is
justified by this attribution alone; the remaining GL q1 excess is recorded
for the integrated Plan 15 assessment while work proceeds to CC's distinct
meridional path.

The temporary counters were test-only and have been removed. Sanitized timing
distributions, route counts, numerical comparisons and host summaries are in
[`stage-profile.json`](data/plan15-spin1-gl-q1-t1023-stage-profile-2026-10-21/stage-profile.json).
The whole-transform input and command are reproducible with the existing
`scripts/benchmark_ducc_spin1_native.py` runner. The stage commands used the
existing ignored Rust phase test with `RUSTSHT_PROFILE_LMAX=1023`,
`RUSTSHT_PROFILE_THREADS=1`, eight repetitions, single-core affinity, and a temporary
test-only counter around the staged-transition and polar/generic fallback
boundaries. DUCC stages called the pinned library's `alm2leg` and `leg2map`
directly. Full compiler flags and benchmark-driver hashes remain in the
published native baseline metadata.
