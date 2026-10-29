# T720 scalar analysis with f32 FFT and shared f64 recurrence

## Decision

The isolated route is a substantial improvement over native f32 scalar
analysis, but remains above pinned native-f32 DUCC in all four GL T720 cells.
It is **not promoted**. The three-pair screen measured candidate/DUCC ratios
from `1.057x` to `1.248x`; the most consequential miss is full-mmax q8 at
`1.235x` median. Stage timing shows the shared f64 recurrence and accumulation
remain the largest stage. f32 spectrum promotion and output narrowing do not
account for enough time to close the q8 full gap.

This is an ordinary finite-fixture lower bound, not a production-ready
precision policy. The test-only route does not implement guarded decline to
the native-f32 path or validate all f32 exceptional semantics. It changes no
production source on `dev`.

## Provenance and method

- Exact source parent: `64133b570865712b26a2ed8bf1b1d5ea9061a6d3`.
- Candidate source patch SHA-256:
  `4cce560395a8499cda2d2e579ab0cb8f24adc455da33882546fe76a4d08dd692`.
  The patch adds test-only hooks and a precomputed-spectrum diagnostic; the
  f64 analysis recurrence remains the existing shared implementation.
- Pinned DUCC 0.42.0:
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust: rustc 1.99.1, LLVM 23.1.1; `cargo test --release --features
  parallel`. Rust release optimization is `opt-level=3`; no `RUSTFLAGS` or
  fixed `target-cpu` was supplied. Runtime Pulp dispatch selected AVX2/V3.
- DUCC: GCC 15.2.0; `-O3 -ffast-math -ffp-contract=fast
  -march=x86-64-v3`; the driver alone also specifies `-fno-fast-math`.
- Host: AMD Ryzen 9 5950X. q1 affinity `[0]`; q8 affinity `[0,1,2,3,4,5,6,7]`.
- Inputs: deterministic splitmix f32 fixtures, GL `ntheta=721`, `phi0=0.23`.
  Reduced uses `nphi=378, mmax=180`; full uses `nphi=1458, mmax=720`.
  Map fixture hashes are recorded in [`fixtures.json`](fixtures.json).
- Each cell used three interleaved native-Rust / candidate / DUCC processes in
  rotating order. Rust maps and DUCC's native-f32 map were the same f32 files.
  Each process constructed its reusable plan/workspace before timing. Rust
  candidate timing includes the f32 FFT, promotion of requested complex
  spectra to f64, shared f64 analysis recurrence/accumulation, and Complex64
  to Complex32 output writeback. DUCC timing uses its native f32
  `analysis_2d` API. Timing samples and raw worker summaries are in
  [`paired-results.json`](paired-results.json) and
  [`raw-paired-screen.json`](raw-paired-screen.json).

## Whole-call screen

Times are milliseconds. Ratios are paired candidate/DUCC medians; ranges show
the three individual paired ratios and are descriptive, not confidence
intervals.

| GL T720 scalar analysis | Native f32 ms | f32 FFT + f64 recurrence ms | DUCC f32 ms | Candidate/DUCC | Native/DUCC | Status |
|---|---:|---:|---:|---:|---:|---|
| Reduced q1 | 143.606 | 6.191 | 5.747 | `1.077x` (`1.057–1.080`) | `25.03x` | **GAP** |
| Reduced q8 | 19.390 | 0.984 | 0.887 | `1.107x` (`1.106–1.122`) | `21.86x` | **GAP** |
| Full q1 | 469.226 | 15.329 | 14.077 | `1.091x` (`1.084–1.096`) | `33.43x` | **GAP** |
| Full q8 | 61.464 | 2.524 | 2.051 | `1.235x` (`1.220–1.248`) | `29.96x` | **GAP** |

All workers returned finite outputs and passed repeated-call and poisoned
spectrum-reset checks. Candidate coefficients differed from DUCC by relative
L2 `1.32e-7` reduced and `1.43e-7` full; maximum absolute differences were
`1.40e-9` and `6.99e-10`. These fixture comparisons do not replace analytic,
adjoint, extreme-value, or independent-reference validation.

The p3 candidate and DUCC output binaries are retained in
[`outputs/`](outputs/). Their hashes are in `SHA256SUMS.txt`.

## Stage and memory profile

The diagnostic stage timings below were collected separately from the
uninstrumented paired screen. Values are medians across 7 or 9 calls and need
not sum exactly to the whole-call measurement.

| Cell | Whole call ms | f32 FFT ms | Spectrum promotion ms | Shared f64 recurrence ms | Output narrowing ms |
|---|---:|---:|---:|---:|---:|
| Reduced q1 | 6.136 | 0.665 | 0.062 | 5.372 | 0.034 |
| Reduced q8 | 1.004 | 0.116 | 0.081 | 0.768 | 0.037 |
| Full q1 | 15.317 | 2.741 | 0.291 | 12.152 | 0.088 |
| Full q8 | 2.539 | 0.401 | 0.314 | 1.728 | 0.089 |

The recurrence stage is the largest cost in every cell. Full q8 promotion plus
writeback takes about `0.403 ms`, below the `0.483 ms` candidate-to-DUCC whole
call gap. Even eliminating both completely cannot close that cell by itself.
The per-process peak RSS medians were 26,780 / 22,052 KiB candidate/native
reduced q1, 31,568 / 25,324 KiB reduced q8, 76,404 / 59,272 KiB full q1, and
84,108 / 64,348 KiB full q8. These are process high-water marks, not exact
workspace allocation totals. The f64 plan/workspace and coefficient scratch
are reusable and allocated before timing. Source inspection shows the q8
dynamic order scheduler constructs one `VecDeque` per call, with 4,344 bytes
of order-entry payload at reduced mmax and 17,304 bytes at full mmax; this
experiment did not use a heap-allocation interposer to count allocator calls.

## Callgrind and executed code

One full-q1 candidate test run recorded `1,362,138,019` Ir across three
transform calls plus setup; the DUCC worker recorded `870,410,854` Ir across
four calls plus setup. These totals have different call counts and harness
setup, are instruction references rather than cycles, and are not used as a
whole-call ratio. Rust's call tree attributes about `493.8M` Ir to
`fused_scalar_analysis_pair_block_impl<true,true>`; the executed
`FusedScaledScalarAnalysisPair<true,true>` SIMD leaf accounts for `220.1M`
Ir in that profile. DUCC's `map2alm_kernel` accounts for `223.9M` Ir in its
profile. Inclusive rows overlap and must not be added.

Callgrind and the V3 disassembly show the shared f64 scaled-prefix path is
active in this route. Source inspection finds a checked degree-pair prefix
that normalizes and tests recurrence state before handing an ordinary suffix
to `FusedScalarAnalysisPair`. The Rust AVX2 leaf is a 20,496-byte routine with
a 0x248-byte stack frame; DUCC's executed AVX2 `map2alm_kernel` is a compact
736-byte routine with an unrolled four-latitude-vector recurrence, eight
YMM FMA accumulators, and horizontal reductions after the inner loop. This
points to recurrence/control work as the remaining gap, not f32-to-f64
spectrum conversion alone. The saved assembly and compressed Callgrind data
are in [`assembly/`](assembly/) and [`callgrind/`](callgrind/).

## Reproduction and next action

The exact test-only source patch is [`candidate.patch.gz`](candidate.patch.gz).
The compiled test binary also included an earlier test-only spin-profile
module from the same isolated worktree; its module declaration and source are
archived in [`build-overlay/`](build-overlay/) and are included in the binary
hash provenance. The benchmark worker snapshot is
[`benchmark/scalar-f32fft-screen.py`](benchmark/scalar-f32fft-screen.py).
The test command is:

```sh
cargo test --release --features parallel --lib \
  transform::tests::m6_profile_scalar_analysis_f32_fft_f64_recurrence \
  -- --exact --nocapture
```

Set `M6_SCALAR_ROUTE` to `native`, `f32_fft_f64_recurrence`, or
`f32_fft_f64_recurrence_profile`; the remaining required fixture and shape
variables are documented in the archived harness patch and paired runner
snapshot.

**NEXT:** do not promote this precision boundary. Continue at exact parent
`64133b570865712b26a2ed8bf1b1d5ea9061a6d3` with the Stage D f32 spin-1
synthesis hypothesis: a dual-channel reflected accumulation leaf through the
existing hemisphere scheduler, with route counters proving q1/q8 activation.
Keep the native-f32 checked fallback and protected f64 T1023 synthesis
control. M6 remains open.
