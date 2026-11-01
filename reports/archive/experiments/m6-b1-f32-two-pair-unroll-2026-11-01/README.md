# B1 native-f32 ordinary two-pair traversal

## Result

The f32 specialization now enables the existing shared two-pair ordinary
analysis traversal. The dispatch remains guarded by the cached safe-range and
ordinary-state checks, so scaled and unchecked blocks continue through the
existing path. This amortizes the ordinary recurrence loop over adjacent
coefficient pairs and reuses each loaded latitude vector for both pair
accumulations.

Fifteen paired GL T720 runs show supported whole-call gains in all four
f32 cells. Outputs match the exact parent bit-for-bit. The remaining f32/DUCC
gaps are 2.20–2.85x; f64/DUCC remains 1.226–1.310x and its V3 kernel bytes are
unchanged. This is an optimization step, not parity.

## Provenance and environment

- Parent source: `715eb98924fc7c8e9fd278630f8a4bb60e084c5b`.
- Candidate diff: one f32 precision constant in
  [`candidate.patch.gz`](candidate.patch.gz); decompressed patch SHA-256
  `dc250011e534d792268038740970ae89a5662a8fc7973095cf797116c8ac0fae`.
- DUCC: pinned 0.42.0 revision
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host: AMD Ryzen 9 5950X; rustc 1.99.1; `-C target-cpu=x86-64-v3`;
  Pulp V3 AVX2/FMA. Exact affinity, fixture hashes and binary hashes are in
  [`screen15.json`](screen15.json).
- Candidate/parent outputs were bitwise equal for every pair. The f64 V3
  analysis function is byte-identical between parent and candidate.

## Fifteen-pair screen

Times are paired medians in milliseconds. Candidate/parent and candidate/DUCC
ratios are geometric paired means with 95% t intervals on log ratios.

| Cell | Parent f32 | Candidate f32 | DUCC f32 | Candidate/parent | Candidate/DUCC |
|---|---:|---:|---:|---:|---:|
| Full q1 | 42.3010 | 39.8244 | 13.9595 | 0.942 [0.935, 0.948] PASS | 2.853 [2.835, 2.870] GAP |
| Full q8 | 5.7423 | 5.4115 | 2.0486 | 0.950 [0.941, 0.958] PASS | 2.646 [2.623, 2.670] GAP |
| Reduced q1 | 15.0707 | 13.7251 | 5.8600 | 0.917 [0.898, 0.935] PASS | 2.365 [2.312, 2.420] GAP |
| Reduced q8 | 2.1186 | 1.9428 | 0.8894 | 0.917 [0.901, 0.933] PASS | 2.200 [2.175, 2.224] GAP |

The source-specific rows, paired values, execution dispatch, and binary and
fixture SHA-256 values are machine-readable in `screen15.json`. Reproduce the
source change with `gzip -dc candidate.patch.gz | git apply`. The runner is
[`screen-runner.py`](screen-runner.py); it expects parent, candidate and DUCC
binaries plus q1/q8 affinity through environment variables.

## Profile and generated code

Current matched phase probes put B1 Legendre recurrence plus accumulation at
93.5–93.9% of f32 q1/q8 time and 82.4–83.0% of f64 q1/q8 time. The FFT takes
the remainder. Hardware `perf stat`/`perf record` events are unavailable on
this host (`perf_event_paranoid=4`), so those phase timings and Callgrind
instruction counts do not establish hardware cycle attribution.

The f32 V3 typed function grows from 19,820 to 22,532 bytes (+13.7%). The
ordinary two-pair loop carries eight YMM accumulators, issues two recurrence
updates per pass, and has no vector stack spills in the inspected hot loop.
The f64 V3 function remains 7,236 bytes with identical instruction bytes.
Representative captured instructions and their hashes are in
[`assembly.txt`](assembly.txt). The larger f32 body is a code-size cost; no
instruction-cache penalty was measurable with hardware counters disabled.

The best-supported avoidable cost is per-pair work in the ordinary f32
traversal: the measured route is ordinary for 80.0% of full-order and 86.7%
of reduced-order pair invocations, and processing adjacent pairs together
reduces whole-call time by 5.0–8.3%. This supports the loop-amortization
hypothesis but does not isolate its cycle share. The f64 deficit and the
remaining f32 gap are still concentrated in the Legendre phase.

## Validation

The focused scalar-paired analysis tests, T720 f32 analytic/adjoint/reset
test, full parallel Rust release tests, strict all-target/all-feature
Clippy, both Python test suites, and formatting passed on the candidate.
The protected f64 T1023 synthesis control from the immediately preceding B1
candidate remains applicable: this change only enables the already-tested
f32 analysis specialization; f64 kernel bytes are identical.

## Next

Keep the f32 two-pair traversal. Profile the remaining f64 Legendre gap and
separate recurrence, ordinary/scaled accumulation, normalization and
exceptional handling before selecting another change. Do not infer cycles
from Callgrind or expand the f32 kernel without a measured hypothesis.
