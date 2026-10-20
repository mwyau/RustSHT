# RustSHT session handoff — 2026-10-14

## Purpose and current state

This handoff captures the RustSHT optimization session through the adaptive
Legendre strategy work and its caller-provided reuse-estimate follow-up. The
session goal was to improve production atmospheric scalar and horizontal
vector spherical harmonic transforms, especially spin 0 and spin 1, while
preserving numerical behavior and reducing the performance gap to DUCC0. The repository is on `main`, with work published to `origin/main`. The last
source commit before this closeout documentation was
`5d3ed6b3fd3d8e7d2e3c248d525e91e1e515db62` (`Record Legendre reuse estimate
results`). Its code and plans were clean and matched `origin/main`. The final
closeout adds the profile note, this handoff, the profile samples and a short
entry in `plans/05-validation-and-milestones.md`; the final commit SHA is
reported in the session closeout. No numerical source modification from the
unfinished last experiment is retained. The long-term DUCC-parity goal remains open. This document closes the current
work session administratively; it does not claim performance parity or that
all follow-up work is complete.

## Priorities and constraints

- Spin 0 and spin 1 are the primary atmospheric-science operations. Spin 2 is
  supported and should be optimized after 0/1. Existing spin-3/4 APIs and
  regressions remain supported; no spin 5+ work is authorized.
- Use stable Rust 1.99.0 for current development and tests. Keep portable Pulp
  runtime dispatch; do not ship `target-cpu=native` binaries.
- Keep plans immutable and execution scratch in independently mutable caller
  workspaces. Rayon workers must not share mutable scratch.
- Preserve GL, CC and F1 conventions; f32 and f64; STANDARD, GRAD_ONLY,
  DERIV1 where supported; separate synthesis/analysis operators and adjoints;
  polar scaling; high-band stability; and existing public APIs except the
  separately documented additive reuse-estimate API.
- RustFFT remains the FFT engine. FFT engine and parallel-FFT optimization
  were explicitly deferred in the recent optimization phases.
- DUCC numerical reference: installed `ducc0==0.41.0`, source revision
  `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. DUCC source-build tests found no
  repeatable performance advantage over its installed wheel sufficient to
  explain RustSHT's remaining gap.
- Commit verified changes directly to `main`, push to `origin/main`, and do
  not use commit prefixes `ci:` or `perf(python):`.

## Architecture and implementation status

The numerical Rust core implements scalar and spin transforms on GL, CC and
F1, using DUCC-derived recurrences and grid/resampling methods. Its production
approach is recurrence-first with bounded specializations. Scalar Legendre
tables store a compact triangular north hemisphere and reconstruct south
values by parity. Tables are lazy on the relevant paths, subject to safe
coefficient and geometry checks, and capped at 34 MiB. Larger dense matrices
were not made part of production. The f64 scalar paired recurrence includes DUCC's transformed paired-degree
coefficients, scaled recurrence, normal-range fast paths, polar order cutoffs,
and active-ring compaction. Scalar analysis pairs mirrored rings and applies
the correct even/odd parity and per-ring quadrature. Spin-1 STANDARD operations
use scalar coefficient conversion where measurements support it; general
spin recurrence remains for poles, unsupported modes and fallbacks. The
spin-1 GL analysis path processes mirrored rings with independent cutoffs. A
multithreaded synthesis path prepares scalar-conversion coefficient streams
once per order in worker scratch and reuses them across groups. Recent optimization work includes:

- Fused scalar synthesis recurrence and accumulation in SIMD registers for
  eligible normal-range blocks, with scaled fallback elsewhere.
- Paired scalar synthesis and spin-1 synthesis improvements, including
  staged spin coefficient factorization and a measured q4/q8 hemispheric
  spin-1 path on selected T319/T511 cases.
- Spin-1 scalar-conversion analysis with fused Q/U SIMD reduction. The
  accepted normal-range kernel shares basis and cosine loads, while unsafe
  products use established scaled fallbacks.
- Scalar paired analysis SIMD accumulation, mixed exponent-zero/scaled lane
  handling, extreme finite product guards, lane-normalization shortcuts and
  a two-vector/two-chain AVX2 reduction.
- Weighted analysis order splitting, analysis polar cutoff handling, and
  active-ring compaction for eligible scalar/spin paths.
- Safe borrowed NumPy input handling and existing Python plan/in-place APIs;
  binding changes remain secondary to native recurrence costs.

## Adaptive Legendre strategy selection

The recent selector milestone compared recurrence and compact north-hemisphere
tables using repeated current-source process-paired measurements. It did not
add a hidden autotuner. Automatic gates are deterministic and operation
specific, account for the executing Rayon pool, preserve the 34 MiB bound,
and remain conservative where process ranges or first-call costs are
uncertain. An additive public `LegendreReuseEstimate` constructor allows applications
with known STANDARD table-eligible reuse to estimate coefficient-to-map and
map-to-coefficient operation counts independently. `ShtPlan::new` remains
unchanged. Estimates include the first call. Spin-1 adjoint synthesis cannot
use the analysis table and therefore cannot be counted in that direction. The table remains lazy, and the active pool is checked at execution time. Reuse estimates only select measured break-even cells; no extrapolation is
made to unmeasured combinations. A positive estimate may use a table already
initialized by another eligible operation, while `Some(0)` keeps recurrence. The four measured examples were:

| Workload | Recurrence first/warm | Table first/warm | Measured break-even | Selected threshold |
|---|---:|---:|---:|---:|
| Scalar T127 exact GL synthesis, q8 | 0.468/0.370 ms | 4.344/0.324 ms | 86 calls | 90 |
| Scalar T85 atmospheric GL synthesis, q8 | 0.162/0.165 ms | 2.069/0.136 ms | 67 calls | 70 |
| Spin-1 T159 exact GL synthesis, q1 | 5.456/5.064 ms | 12.611/4.824 ms | 31 calls | 32 |
| Spin-1 T159 atmospheric GL analysis, q4 | 2.495/2.529 ms | 17.551/1.928 ms | 27 calls | 30 |

These results are exact to the documented grid dimensions in
[`legendre-reuse-estimate-2026-10-14.md`](../../legendre-reuse-estimate-2026-10-14.md). Payload estimates ranged from 1.83 to 15.92 MiB. RSS was not measured in that
experiment. Extensive earlier studies found large tables can make high-band
multithread analysis slower despite one-thread synthesis gains; do not raise
the budget based on isolated kernels. Sol High performed a strictly read-only review of the selector architecture
after implementation. It identified operation-direction naming, a missing
first-call `+1` in the break-even formula, a spin-1 adjoint eligibility case,
and incomplete docs for an already-initialized shared table. These were
corrected and covered by a regression. The reviewer confirmed the issues
were resolved. No numerical tolerance changed.

## Performance evidence and remaining gap

Representative latest recorded f64 GL Python-plan comparisons at T511:

| Operation | Threads | RustSHT | DUCC0 | Ratio | Relative-L2 error |
|---|---:|---:|---:|---:|---:|
| Scalar analysis | 1 | 23.058 ms | 6.470 ms | 3.57x | `3.484e-14` |
| Scalar analysis | 8 | 7.344 ms | 1.074 ms | 6.84x | `3.484e-14` |
| Spin-1 STANDARD analysis | 1 | 35.623 ms | 12.784 ms | 2.79x | `3.740e-14` |
| Spin-1 STANDARD analysis | 8 | 12.210 ms | 2.004 ms | 6.09x | `3.740e-14` |

These are Python plan-method timings and include Python/binding and per-call
workspace costs; native reused-workspace results are reported separately in
the detailed reports. The input-copy fraction was previously measured as
small relative to these gaps. At T720, native scalar synthesis/analysis were
reported around `103.395/61.274 ms` q1 and `27.905/19.521 ms` q8 in the
pre-latest profile; spin-1 was `125.544/101.040 ms` q1 and `41.646/33.532 ms`
q8. Check current raw samples before making new claims. The repeated profiling conclusion is that Legendre recurrence and coefficient
accumulation dominate absolute excess runtime. RustFFT/packing is a smaller
share in measured cases. `perf stat`/`perf record` could not access hardware
time. A local optimized DUCC source build with native compiler flags had no
repeatable advantage over the installed wheel in the tested paired cases;
compiler configuration alone does not explain the gap.

## Experiments accepted or rejected

Accepted changes are in the repository history and milestone notes. Important
rejections and their detailed raw measurements include:

- Scalar north/south paired synthesis was 10.4% slower q1 and 6.2% slower q8
  at T511; repeated factor preparation and scatter erased work saved.
- Direct fused spin-1 recurrence was 5–12x slower than scalar conversion in
  tested STANDARD synthesis. Scalar conversion remains preferred, with the
  general recurrence near poles and for unsupported operations.
- Spin-1 order-major Fourier transpose slowed warm q8 analysis by about
  3.4–3.6% and added 8–15.9 MiB scratch.
- Extra analysis order leaves did not improve weighted scheduling; the T511
  q8 candidate was 3.19% slower at median.
- Third SIMD reduction chain increased register pressure and slowed q1
  analysis by 6.8–7.2%; two chains remain.
- FMA lowered simulated instruction references but slowed T511 spin-1 q8
  analysis by 2.84%; it was reverted.
- Various callback fusion, coefficient-cache, ring-scale-hoist, batch,
  table-threshold, and compiler-flag experiments were kept only where paired
  whole-operation measurements supported them. Negative findings and exact
  conditions are indexed in `plans/05-validation-and-milestones.md` and the
  corresponding `reports/` files.
- Torch-harmonics PR #322 uses precomputed basis/projection tensors. The
  T720 float64 dense payload estimate is about 2.79 GiB for scalar and 5.59
  GiB for two spin/vector rows. Rust's T720 triangular north-hemisphere scalar
  table estimate is 716.87 MiB, still above the production table budget; no
  Torch package was installed for direct timing or allocation measurements. SHTns research was used only for transferable ideas such as runtime
operation-specific algorithm selection. Hidden plan-time calibration and
implicit autotuning were rejected as a fit for the Rust reusable-plan API. The SHTns-inspired field-batching, FFT-friendly grid selection, hemisphere
pairing, and direct spin recurrence studies are documented; do not repeat
them without new workload evidence.

## Numerical validation status

The latest accepted selector commit was validated with:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`: 160 Rust tests and 10 f32 tests
  passed; 9 ignored.
- `cargo test -p rustsht --no-default-features`: 132 Rust tests and 10 f32
  tests passed; 7 ignored.
- `uv run pytest python/tests`: 20 passed. High-band checks recorded in preceding reports include scalar T511/T1023/T2047
DUCC and analytic checks on GL/CC/F1, spin-1/2 T511 all-grid transform and
adjoint checks, spin exact analysis through T1023, sparse spin-1–4 T2047
synthesis, f32 checks, extreme finite reducer fallback, polar-order cutoff
regressions and one/multithread equivalence. Spin-2 polar handling was fixed
in `a739902`; the high-order regression remains. These historical checks are
not newly rerun by the profile report in this commit.

## Final investigation before closeout

The last measurement-only investigation profiled f64 scalar and spin-1 T511
recorded `3,248,451,599` instruction references; the paired recurrence
vectorizer was 37.91%, normal analysis accumulation 15.52%, and RustFFT column
butterflies 3.16% of the reported attribution. The scalar reused-workspace
native baseline at T511 was `22.788 ms` q1 and `6.959 ms` q8; T127 was
`0.612/0.334 ms`. All raw process medians and the command are in
[`native-process-samples.txt`](../../data/current-scalar-analysis-profile-2026-10-14/native-process-samples.txt). The proposed next kernel would fuse the scalar paired recurrence and analysis
accumulation in a Pulp dispatch, stage output coefficients per block, and
commit them only if the full block stays normal-range; otherwise it would
discard staged values and run the existing scaled fallback. This avoids
partial coefficient updates if late recurrence scaling is needed. The
hypothesis is supported by the recurrence scratch writes and separate
per-degree reducer in generated AVX2 code, but the kernel was not implemented
or tested. Only temporary method declarations were started and then removed. There is no uncommitted production source change from this experiment. The profile and implementation conditions are in
[`current-scalar-analysis-kernel-profile-2026-10-14.md`](../../current-scalar-analysis-kernel-profile-2026-10-14.md).

## Next steps, in priority order

1. Resume the overarching spin-0/spin-1 DUCC performance goal. The unfinished
   scalar-analysis fused-kernel hypothesis is a candidate, not a commitment;
   implement it only if the staged-output/fallback design stays bounded and
   the q1/q8 whole-transform measurements are favorable.
2. Compare current f64 GL scalar and spin-1 STANDARD synthesis/analysis at
   atmospheric and exact dimensions, beginning T127/T511 and checking T720 or
   T1023 where practical. Use same-process paired measurements and raw sample
   retention. Match native reused workspace and Python-plan boundaries
   explicitly.
3. After scalar analysis, profile the largest remaining spin-1 synthesis or
   analysis gap. Preserve the selected compact-table policy and the accepted
   fused spin-1 synthesis work; do not re-run rejected fusion/FMA/normalization
   prototypes without new evidence.
4. Extend accepted shared-path checks across CC/F1, f32, GRAD_ONLY, DERIV1,
   adjoints, mixed cutoff blocks, extreme finite inputs and spins 3/4
   regressions as appropriate.
5. Keep FFT optimization deferred unless a future profile establishes a
   material absolute excess and a separately authorized phase.
6. Continue recording performance evidence in `plans/05` and concise
   experiment-specific reports. Use Sol High only for a major numerical or
   architectural milestone, strictly read-only.

## Reproduction and repository references

Current profile commands:

```text
cargo build --release --example m1b_bench
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 0 511 1 analysis GL STANDARD 512 1025 511 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 0 511 8 analysis GL STANDARD 512 1025 511 requested-only
callgrind_annotate --inclusive=yes --show=Ir <temporary-path>
```

Choose an available CPU set sized for the requested worker count on the current host.
Record the worker count and actual placement in local benchmark diagnostics. Benchmark examples are
`examples/m1b_bench.rs` and `examples/python_perf_baseline.py`. Important
design and validation references are:

- `plans/00-scope-and-decisions.md` — scope, priority and selector decisions.
- `plans/02-numerics-and-grids.md` — numerical contracts.
- `plans/03-api-and-python.md` — Rust/Python API decisions.
- `plans/05-validation-and-milestones.md` — accepted and rejected evidence.
- `plans/06-ducc-contract-crosswalk.md` — DUCC operation mapping.
- `reports/legendre-reuse-estimate-2026-10-14.md` — final selector experiment.
- `reports/spin1-dual-channel-analysis-2026-10-14.md` — spin-1 Q/U reducer.
- `reports/spin1-staged-synthesis-factorization-2026-10-14.md` — spin-1
  synthesis improvement.
- `reports/legendre-precomputation-2026-10-13.md` — table storage and
  precomputation memory/performance.
- `reports/current-scalar-analysis-kernel-profile-2026-10-14.md` — unfinished
  current-source scalar analysis experiment.
