# Plan 14: reuse reflected spectra in T127 scalar analysis

**Baseline:** RustSHT `2af92383`; **candidate:** the table-loop change in
this source-and-evidence commit. **DUCC:** 0.42.0 at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Rust 1.99.1 / LLVM 23.1.1,
Pulp 0.22.3, release with strict FP and `-C target-cpu=x86-64-v3`; runtime
dispatch reported Pulp V3 (four f64 lanes, AVX2/FMA). DUCC used GCC 15.2,
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` for SHT sources and a
strict benchmark driver.

## Change

For scalar GL analysis through the compact basis table, each order's
north/south spectrum sums and differences are invariant across degree pairs.
The previous V3 accumulator repeated those additions and the reflected-lane
rotation for each pair. The new loop computes the even and odd reflected
spectra once per order, stores them in the existing per-worker scalar
coefficient buffers, and reuses them for all degree pairs. It adds no table
allocation or workspace growth. Table selection, synthesis, f32, spin-1,
exceptional-input checks, and the scalar fallback are unchanged.

## Matched whole-transform results

The native harness exercised T127 GL, full `mmax=127`, `ntheta=128`,
`phi0=0.23`, at odd-minimum `nphi=255` and even `nphi=256`. Rust reused its
plan and workspace; both libraries reused deterministic inputs and outputs,
and validation ran outside timed batches. Each source/case used three
interleaved process pairs, 30 calibrated batches per backend across those
pairs, and approximately 25 ms per batch. T127 odd q1 also has a second
baseline source round. `comparison.csv` contains the paired source summaries.

| `nphi` | threads | Rust before → after (µs) | DUCC before → after (µs) | Rust/DUCC before → after | Rust change | relative L2 |
|---:|---:|---:|---:|---:|---:|---:|
| 255 | q1 | 374.762 → 282.961 | 295.646 → 299.806 | 1.268× → 0.944× | −24.5% | `8.52e-15` |
| 256 | q1 | 338.830 → 248.491 | 261.587 → 261.479 | 1.295× → 0.950× | −26.7% | `8.46e-15` |
| 255 | q8 | 226.204 → 206.118 | 81.268 → 79.699 | 2.783× → 2.586× | −8.9% | `8.52e-15` |
| 256 | q8 | 187.847 → 172.909 | 77.584 → 73.532 | 2.421× → 2.352× | −8.0% | `8.46e-15` |

The q1 table path now runs near DUCC for both longitude shapes; paired
process variation still crosses unity for the odd case, so the result is
near parity rather than a robust claim of beating DUCC. The q8 reduction is
smaller and the run showed scheduling interference; the remaining q8 gap is
still substantial. First-call latency did not increase in the odd q1
comparison (4.23 ms baseline, 4.16 ms candidate); table construction remains
outside the warmed operation.

## Numerical checks and next work

The native comparison covered the complete coefficient triangle and
validated reset consistency, finite zero/subnormal/extreme-finite outputs,
selected basis modes, roundtrip and both adjoint identities. Maximum absolute
Rust/DUCC error was `1.18e-15`. The table-versus-recurrence test covered the
four scalar operators at exact and over-resolved grids; extreme table inputs
still take the safe recurrence route. Release all-feature tests passed (236
unit, 4 extreme-scaling, 11 f32 and 1 workspace tests); no-default-feature
tests passed (197 unit, 4 extreme-scaling, 11 f32 and 1 workspace tests).
Strict Clippy and formatting passed.

Reproduce with RustSHT's native full-transform harness, pinned DUCC source,
and `RUSTSHT_FULL_TRANSFORM_RUSTFLAGS='-C target-cpu=x86-64-v3
-Aunexpected_cfgs'`. Select T127, `mmax=127`, `ntheta=128`, `nphi=255` or
256, analysis, and q1/q8; use separate output directories for the baseline
and candidate commits. The next Plan 14 target is the remaining large-q8
whole-transform excess, beginning with a current-source stage rerank; do not
reopen settled prefix or tile experiments.
