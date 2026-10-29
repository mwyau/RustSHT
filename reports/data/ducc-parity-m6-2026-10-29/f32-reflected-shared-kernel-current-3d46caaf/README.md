# Current-source reflected f32 scalar-synthesis replay

## Decision

Reject this candidate for production. The archived patch is based on
`49295a89`; it applies with a three-way reconciliation to exact current parent
`3d46caaf89a8fcd3796fe43c49aa8ef82e481cad`. It preserves the accepted shared
`FusedSynthesisBlock`, paired recurrence visitor, and
`F32SynthesisAccumulator`; it adds no maintained recurrence implementation.

The candidate's normal-input f32 T720 reflected route is preempted by the
already accepted reusable f32-I/O/f64-internal scalar-synthesis backend. A
temporary public-path route census reported `[0, 1, 0, 0]` for each full and
reduced q1/q8 case: one scalar recurrence in the f64 child and no native f32
reflected recurrence. Therefore this candidate did not accelerate the normal
public f32 T720 calls. Its whole-call candidate/parent results are neutral to
slightly slower, and it caused repeatable adjacent scalar-analysis and
protected f64 synthesis slowdowns. No production code was changed.

The existing public f32 scalar-synthesis route itself is already competitive:
the current parent was `0.781–0.928x` native-f32 DUCC across these four cells,
with conversion and writeback included. These PASS cells are evidence for the
existing f64-internal implementation, not a gain from this reflected patch.

## Provenance and build

- Rust source parent: `3d46caaf89a8fcd3796fe43c49aa8ef82e481cad`.
- Archived candidate patch, applied with `git apply --3way`: SHA-256
  `f62628d42c21db6bada73ee6b0d0ad5001b67e319ed5370c7b77fc2e01b3af31`.
  It is retained as `candidate.patch.gz`; the source patch is also preserved at
  `reports/data/ducc-parity-m6-2026-10-28/f32-reflected-shared-kernel-49295a89/candidate.patch.gz`.
- Route-census test patch: `route-counter-test.patch.gz`; it is temporary test
  instrumentation and is not part of the candidate production patch.
- Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Compiler: rustc 1.99.1 / LLVM 23.1.1; GCC 15.2.0. Rust used
  `--locked --release --features parallel` and
  `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. DUCC SHT sources used
  `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the driver used
  `-fno-fast-math`.
- Host: AMD Ryzen 9 5950X with AVX2/FMA. q1 pinned to CPU 0; q8 pinned to
  CPUs `0,2,4,6,8,10,12,14`.
- Each timed worker used a 25 ms warm-call target. Setup and reusable plan and
  workspace creation were outside timed batches. The tested T720 cells use
  `ntheta=721`, `phi0=0.23`, full `mmax=720,nphi=1458`, reduced
  `mmax=180,nphi=378`.
- Binary SHA-256 values, per-case input fixture hashes, all interleaved pair
  medians, and accuracy/reset checks are in `paired-results.json`.

## T720 scalar synthesis

Each cell has 15 interleaved parent/candidate/native-f32-DUCC comparisons.
Times are milliseconds; candidate/parent and candidate/DUCC are paired
medians. The 95% intervals are bootstrap intervals over paired ratios.

| GL T720 scalar synthesis | Parent / candidate / DUCC ms | Candidate/parent [95% CI] | Candidate/DUCC | Result |
|---|---:|---:|---:|---|
| Full q1 | 15.665 / 15.762 / 18.997 | 1.0074 [1.0035, 1.0119] | 0.8305 | No candidate gain; existing public route PASS |
| Reduced q1 | 6.458 / 6.508 / 8.323 | 1.0068 [1.0040, 1.0097] | 0.7810 | No candidate gain; existing public route PASS |
| Full q8 | 2.871 / 2.810 / 2.953 | 1.0002 [0.8555, 1.0328] | 0.9278 | Inconclusive candidate delta; existing public route PASS |
| Reduced q8 | 1.114 / 1.123 / 1.279 | 1.0056 [0.9831, 1.0432] | 0.8727 | Inconclusive candidate delta; existing public route PASS |

Candidate and parent synthesis outputs were bitwise equal in every pair. All
workers passed finite-output, repeat-consistency, and poisoned-output reset
checks. The wide q8 intervals are retained as inconclusive; they do not
establish a candidate improvement.

## Adjacent controls

The scalar-analysis candidate/parent regression repeated at q1 and q8 in both
truncations. The q1 cells combine 3 initial pairs and 15 extension pairs; q8
uses 15 pairs. The spin-1 synthesis q1/q8 cells combine the initial 3 pairs
with 15 additional pairs. Spin-1 analysis remained a three-pair screen because
all observed pairs were faster and no regression signal appeared. T255 q1 and
T1023 q8 scalar synthesis were three-pair scope checks.

| Control | Pairs | Parent / candidate / DUCC ms | Candidate/parent [95% CI] | Status |
|---|---:|---:|---:|---|
| Scalar analysis full q1 | 18 | 447.951 / 452.844 / 13.951 | 1.0109 [1.0087, 1.0126] | Repeatable regression |
| Scalar analysis full q8 | 15 | 59.424 / 60.138 / 2.307 | 1.0120 [1.0082, 1.0131] | Repeatable regression |
| Scalar analysis reduced q1 | 18 | 138.062 / 139.514 / 5.723 | 1.0107 [1.0095, 1.0120] | Repeatable regression |
| Scalar analysis reduced q8 | 15 | 19.154 / 19.251 / 1.128 | 1.0117 [1.0051, 1.0158] | Repeatable regression |
| Spin-1 analysis full q1 | 3 | 1037.025 / 1023.036 / 28.702 | 0.9865 | No regression in screen |
| Spin-1 analysis full q8 | 3 | 137.657 / 135.585 / 4.389 | 0.9875 | No regression in screen |
| Spin-1 synthesis full q1 | 18 | 1463.365 / 1490.810 / 40.163 | 1.0174 [1.0117, 1.0213] | Repeatable regression |
| Spin-1 synthesis full q8 | 18 | 269.774 / 273.055 / 5.944 | 1.0108 [1.0010, 1.0219] | Small regression |
| Scalar synthesis T255 full q1 | 3 | 37.826 / 37.733 / 1.444 | 0.9972 | Scope screen only |
| Scalar synthesis T1023 full q8 | 3 | 404.823 / 382.900 / 7.129 | 0.9458 | Scope screen only |

The T255/T1023 results do not establish DUCC parity (`26.15x` and `53.44x`
candidate/DUCC), nor do three pairs justify generalizing the patch outside
T720.

## Protected f64 GL T1023 spin-1 synthesis

q1 combines 3 initial pairs plus 12 extension pairs. q8 combines 15 initial
pairs plus 15 additional pairs to address variable timings. Candidate and
parent outputs were bitwise equal; all repeat/reset checks and independent
finite error metrics passed.

| Threads | Pairs | Parent / candidate / DUCC ms | Candidate/parent [95% CI] | Candidate/DUCC |
|---|---:|---:|---:|---:|
| q1 | 15 | 88.148 / 89.057 / 119.536 | 1.0109 [1.0051, 1.0140] | 0.7446 |
| q8 | 30 | 16.529 / 17.481 / 20.258 | 1.0630 [1.0288, 1.1011] | 0.8486 |

This is a repeatable protected-path regression, especially q8. The q8 result
rules out promotion even though the candidate remains faster than DUCC on this
control.

## Correctness checks run

- `cargo fmt --check --all` passed.
- The test-only public route census passed for all four T720 scalar-synthesis
  cells and observed only the existing f64 child recurrence on regular data.
- `cargo test --locked --release --features parallel --test f32 f32_t720_ -- --nocapture`
  passed (4 tests): analytic scalar modes, scalar adjoints, scalar-analysis
  analytic coefficients, sectoral spin-1 behavior, and f32 exceptional/reset
  cases.
- `cargo test --locked --release --features parallel --lib scalar_paired::synthesis::tests`
  passed (5 tests), including scaled/subnormal/overflow packet checks and SIMD
  tails.
- Every whole-call worker passed finite, repeat, and reset checks. The
  candidate and parent f32 scalar-synthesis outputs were bitwise equal;
  maximum candidate-vs-DUCC relative-L2 error was below `3.90e-8`. Protected
  f64 outputs were bitwise equal to parent and max relative-L2 versus DUCC was
  `2.61e-13`.

## Reproduction and next experiment

The `controls-*.py` files and DUCC f64 worker source are retained with the
paired results. The f32 fixtures are deterministically generated by
`scripts/benchmark_ducc_f32_native.py::make_fixture`; their SHA-256 values are
in `paired-results.json`. The f64 fixture hashes use
`scripts/benchmark_ducc_spin1_native.py::make_fixture`.

The next bounded Stage C hypothesis is to keep RustSHT's native-f32 longitude
FFT and feed its spectrum to the existing shared f64 scalar-analysis
recurrence. Pinned DUCC's `map2leg<T>` creates `complex<T>` spectra, while
`map2alm_kernel` accumulates with `native_simd<double>` and `complex<double>`.
The Stage B full scalar-analysis adapter used f64 FFT plus f64 recurrence and
was `1.052–1.179x` DUCC; testing a f32 FFT/f64 recurrence boundary removes the
known whole-map promotion and tests the same precision split DUCC uses. Profile
that whole route and measure all four full/reduced q1/q8 T720 cells before any
production change.
