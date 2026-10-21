# Plan 15 GL spin-1 synthesis: parallel row FFT A/B

**Candidate:** `ac8e754791322c33d305be55631dbaa3403214aa`
**Before:** `4523b9278e1752b206e03f2d256827aaadd26fbf`
**DUCC:** 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`

This change routes each spin-1 synthesis output channel through the existing
row-parallel inverse-FFT helper when its established geometry and workspace
guards permit it. The FFT algorithm, recurrence and spectrum values are
unchanged. The q1 path remains serial. The odd longitude case is a control;
this does not alter the FFT planner or algorithm.

## Matched whole-transform screen

Native f64 STANDARD spin-1 GL synthesis, reused plans/workspaces, identical
fixture per implementation, q8 pinned to physical CPUs 0–7, q1 to CPU 0,
nonzero `phi0=0.23`. Rust used 1.99.1 / LLVM 23.1.1 with
`-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. DUCC used GCC 15.2.0, SHT
sources `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, and a strict
validation driver. Competitors were interleaved for three process pairs with
30 calibrated q8 samples per backend/case. q1 exceeded the batch threshold
and used 15 individually timed calls per backend.

| Workload | Rust before ms | Rust candidate ms | DUCC ms | Candidate/before paired ratios | Candidate/DUCC paired median | Candidate excess ms |
|---|---:|---:|---:|---|---:|---:|
| T720 full, even `nphi=1458`, q8 | 63.391 [51.529, 65.817] | 59.052 [50.602, 60.334] | 9.315 [8.977, 10.849] | 0.989, 0.933, 0.928 | 6.435× | +49.736 |
| T720 full, odd `nphi=1443`, q8 | 90.710 [85.344, 91.876] | 64.742 [57.948, 66.063] | 10.316 [9.125, 11.672] | 0.700, 0.720, 0.719 | 6.399× | +54.426 |
| T720 `mmax=180`, even `nphi=378`, q8 | 36.336 [35.540, 37.287] | 34.901 [33.038, 35.468] | 5.064 [4.208, 5.815] | 0.949, 0.971, 0.958 | 6.981× | +29.837 |

Brackets give p10–p90 over the collected calls. The full-even and reduced
q8 whole-call distributions overlap, and q8 runqueue wait ranged from 1.8%
to 35.1% across processes. Treat those magnitude estimates as provisional.
The full-odd improvement was consistent across all three pairs and its
before/after p10–p90 ranges did not overlap; this remains a control for the
same row-parallel route, not an odd-FFT optimization.

The q1 full-even control was unstable and is not credited as a gain: the
candidate's call distribution was bimodal (p10 103.2 ms, p90 172.8 ms),
while the baseline was 168.9–172.8 ms. The three paired candidate/baseline
ratios were 0.999, 0.614 and 0.601 despite identical code paths being serial
at q1. That regime shift is unresolved and requires a quieter repeat before
any q1 conclusion.

## Stage profile and correctness

A test-only T720 full-even q8 phase probe measured the two inverse row FFTs
and map writes at 7.85–8.00 ms before the change and 2.24–2.40 ms afterward
(medians 7.97 and 2.30 ms). The order scheduling/Legendre/accumulation stage
remained 54.0–54.9 ms before and 53.7–56.9 ms afterward. Thus the measured
remaining GL bottleneck is recurrence/scheduling, not the inverse rows.

Candidate outputs were bitwise equal to the before binary for all four
workloads. Repeat and output-reset checks passed. Maximum candidate-versus-DUCC
relative L2 was `5.23e-13`; maximum absolute difference was `4.50e-9`.

Validation passed: `cargo fmt --all -- --check`, strict Clippy,
`cargo test --release --features parallel --lib -- --test-threads=1`
(245 passed, 32 ignored), and `cargo test --release --no-default-features
--lib` (201 passed, 16 ignored). The default-parallel test run first hit an
existing scalar worker-probe assertion; that test passed alone and the full
serialized run passed.

## Interpretation and next work

The measured q8 output stage fell substantially while preserving both spin
channels and numerical results. The candidate still trails DUCC by roughly
6–7× on these q8 cases. The recurrence/scheduling stage is the dominant
remaining cost (about 54–57 ms of a roughly 60–65 ms call). The next GL task
is to characterize per-ring-group work and worker utilization, then compare
the current ring-group dataflow with DUCC's order-oriented active-ring
batches. Do not resume odd-FFT work.

## Reproduction

Build the before and candidate example binaries from the source SHAs above
with the same command and `RUSTFLAGS` recorded in the JSON. Reuse the DUCC
binary and build metadata in the Plan 15 baseline data. The deterministic
fixture generator, native worker argument builder and CPU-pinned timer are
`make_fixture`, `worker_command` in
`scripts/benchmark_ducc_spin1_native.py`, and `run_pinned` in
`scripts/benchmark_ducc_full_transform_parity.py`. For each q8 case the
runner used the interleaved backend orders
`baseline/candidate/DUCC`, `DUCC/baseline/candidate`, then
`candidate/DUCC/baseline`; q1 used the same order with individually timed
long calls. Case geometry, binary hashes, compiler flags, sample arrays and
per-process diagnostics are in the JSON dataset.

The machine-readable distributions, pair ratios, correctness records, binary
hashes, and host wait/frequency summaries are in
[`reports/data/plan15-spin1-gl-rowfft-2026-10-21/`](data/plan15-spin1-gl-rowfft-2026-10-21/).
