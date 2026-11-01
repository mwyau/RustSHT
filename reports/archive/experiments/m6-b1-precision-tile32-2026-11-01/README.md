# B1 native-f32 tile 32 screen

## Result

This candidate changes only the scalar-analysis latitude-pair tile selected for
f32: 256 to 32. The f64 tile remains 256. The recurrence and its f32/f64 SIMD
kernels are unchanged. The source change is isolated in
[`candidate.patch.gz`](candidate.patch.gz), based on `ded15d0aeaf8d20ef0961320468b07b600eb88c4`.

Fifteen interleaved warm whole-transform pairs show a statistically supported
f32 speedup over that exact parent in all four GL T720 cells. Calls include
order scheduling, Legendre recurrence and accumulation, FFT, and map/coefficient
handling. Every cell remains slower than pinned DUCC f32.

| Cell | Parent ms | Candidate ms | DUCC f32 ms | Candidate/parent, 95% CI | Candidate/DUCC | Status |
|---|---:|---:|---:|---:|---:|---|
| Full q1 | 57.726 | 43.772 | 13.959 | 0.7608 [0.7556, 0.7661] | 3.1311 | improvement PASS; DUCC GAP |
| Full q8 | 7.773 | 5.969 | 2.036 | 0.7654 [0.7570, 0.7738] | 2.9291 | improvement PASS; DUCC GAP |
| Reduced q1 | 18.595 | 15.501 | 5.733 | 0.8370 [0.8317, 0.8424] | 2.7153 | improvement PASS; DUCC GAP |
| Reduced q8 | 2.593 | 2.168 | 0.885 | 0.8417 [0.8298, 0.8537] | 2.4506 | improvement PASS; DUCC GAP |

Ratios are geometric means of 15 paired process-median ratios; intervals are
95% t intervals on log ratios. Rust used `-C target-cpu=x86-64-v3`; runtime
dispatch was Pulp V3 AVX2/FMA with eight f32 lanes. The host was an AMD Ryzen 9
5950X. q1 used one pinned physical core and q8 used eight distinct pinned
physical cores. The deterministic f32 GL fixtures and all binary hashes are in
[`screen-15pair.json`](screen-15pair.json). Pinned DUCC is
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; its f32 worker hash is recorded
there.

Candidate outputs differ from the parent only through the changed block and
reduction grouping: maximum absolute differences are `2.24e-8` full and
`4.47e-8` reduced, with relative L2 at most `9.10e-7` and `1.11e-6`.
Candidate/DUCC relative L2 is `1.8391e-5` full and `2.2204e-5` reduced.
Repeat and workspace-reset checks pass. No tolerance changed.

## Measured bottleneck and hypothesis

The matched B1 phase probe at `410c9ec2594007a1e081c949767fc5f7c28d2654`
placed recurrence plus accumulation at 94–97% of f32 q1/q8 call time and
82–91% of f64 time; FFT was 3–6% f32 and 8–18% f64. The matched full-q1
Callgrind profile at the later `acd7fe5` source plus the f32 classification
candidate counted 1.179B Rust f32 instructions, 320.34M Rust f64 instructions,
214.19M DUCC f32 instructions, and 219.96M DUCC f64 instructions. Rust's
`analyze_symmetric_order_group` accounts for 96.55% of f32 and 83.8% of f64
instructions inclusively. These are dynamic instructions, not cycles.

At source `410c9ec`, non-overlapping f32 Callgrind regions were 20.9M normal
accumulation, 539.1M scaled accumulation including factor preparation, 49.8M
recurrence advancement, and 538.8M range check plus normalization (92.7% of
total together). Factor preparation (409.6M) is inside scaled accumulation;
normalization (232.2M) is inside range check plus normalization. Do not add
those nested counts again. Hardware `perf stat` and `perf record` are blocked
by `perf_event_paranoid=4`.

The test-only typed-route probe on the exact parent and candidate shows the
tile change's effect:

| Geometry | Parent tile / attempts | Parent normal / scaled | Candidate tile / attempts | Candidate normal / scaled |
|---|---:|---:|---:|---:|
| Full T720 | 256 / 1,139 | 111,909 / 125,629 (52.9% scaled) | 32 / 6,437 | 1,067,695 / 266,685 (20.0% scaled) |
| Reduced T720 | 256 / 362 | 61,843 / 52,459 (45.9% scaled) | 32 / 2,119 | 581,344 / 89,429 (13.3% scaled) |

These counters count coefficient-pair accumulation invocations, not lane
updates. Smaller blocks increase block and pair invocations, but each scaled
invocation covers at most 32 rather than 256 latitude lanes. The reduced
scaled share and the whole-call gain support the hypothesis that mixed-scale
256-lane groups make too much of the recurrence use expensive scaled
accumulation. The probe uses zero spectra to isolate recurrence routing; the
whole-transform benchmark uses nonzero deterministic maps and coefficients.

The generated f32 V3 typed block contains YMM AVX2/FMA operations, including
`vfmadd231ps`, and is eight-lane. Its source was not changed by this candidate.
The f64 executable `.text` hash is identical between parent and candidate
(`a85a78adf6a88f2fd3b00a9b3945a061ac2a2af3a7e8f1e4c931a3622a3612a2`), so the
f64 implementation remains unchanged. Its existing matching-DUCC T720 baseline
is still a 1.23–1.31x GAP. The separate protected T1023 f64 spin-1 synthesis
three-pair control is bitwise exact; its small timing differences are
INCONCLUSIVE, and the f64 spin-1 executable `.text` hash is identical.

## Execution crosswalk and microbenchmark context

- `ShtPlan<T>::analyze_symmetric_order_group` in `src/transform/scalar.rs`
  selects `T::ANALYSIS_PAIR_TILE_SIZE` for each latitude-pair block. The f32
  specialization is 32; f64 inherits the 256-lane maximum.
- `ShtScalar::ANALYSIS_PAIR_TILE_SIZE` is declared in `src/legendre/mod.rs`;
  workspace range-cache sizing follows the same precision-specific value in
  `src/workspace.rs`.
- `fused_scalar_analysis_pair_block_preweighted_typed` in
  `src/legendre/scalar_paired/analysis.rs` remains the shared native arithmetic
  kernel. AVX2 uses eight f32 lanes; this candidate changes scheduling, not the
  recurrence, FMA order, SIMD width or exceptional-lane rules.

The preceding B1 direct factor-helper microbenchmark at the hard-factor parent
measured a `0.625 [0.602, 0.648]` paired ratio for the fused one-pass helper
versus its two-pass predecessor (nine interleaved pairs, 20,000 fills per
sample). That isolates factor-preparation overhead, not the tile change. For
tile 32, the isolated evidence is the route-count probe above; the four
nonzero-fixture 15-pair whole-call measurements establish its actual runtime
effect. The helper microbenchmark details and reproducible test-only probe are
in the [preceding classification package](../m6-b1-hard-factor-classification-fusion-2026-11-01/README.md).

## Validation and provenance

- Rust module tests: 38 B1 paired-analysis tests passed, 8 manual diagnostics
  ignored; 18 `scalar_analysis` library tests passed, 8 ignored.
- T720 f32 analytic, adjoint, SIMD tail, and workspace-reset integration test
  passed at q1/q8 and full/reduced order.
- Extreme scaling integration: 4 passed.
- Full workspace Rust suite: 329 passed, 39 ignored; extreme scaling 4 passed;
  f32 integration 14 passed, 1 ignored; workspace compatibility 1 passed.
- `cargo fmt --all -- --check` and strict all-target/all-feature Clippy passed.
- Python API tests: 20 passed after installing the candidate worktree's locked
  environment with `uv sync --python 3.11`.
- Protected f64 T1023 spin-1 synthesis: three pairs per q. Parent/candidate
  output is bitwise equal; q1 ratio `1.0080 [0.9910, 1.0252]`, q8
  `1.0146 [0.9997, 1.0297]`; both are INCONCLUSIVE, with no regression
  established. Candidate/parent `.text` hashes match.
- The previous parent CI run `41294797600` passed. The candidate CI run is
  checked after its commit is pushed.

The 15-pair runner is [`screen-runner.py`](screen-runner.py). The compact
route counts, protected f64 control, and selected disassembly facts are in
[`routes.json`](routes.json), [`protected-f64-t1023.json`](protected-f64-t1023.json),
and [`disassembly-summary.txt`](disassembly-summary.txt). `SHA256SUMS` covers
the evidence artifacts and this report; it excludes the checksum index itself.

## NEXT

Keep tile 32 for native f32 B1. Continue profiling the remaining scaled
accumulation and normalization cost under this schedule, then isolate a new
candidate against the exact updated parent. Keep f64 unchanged and compare
both precisions with their matching DUCC workers.
