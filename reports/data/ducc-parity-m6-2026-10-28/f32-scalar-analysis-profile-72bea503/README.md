# Native-f32 GL T720 scalar-analysis profile

**Status:** current-source diagnosis; no production implementation changed. Scalar
analysis and spin-1 cells remain **GAP**. The accepted scalar-synthesis packet
leaf remains in production; its later scaled-to-normal suffix trials were
rejected and block that direction on this source's f32 exponent range.

## Source and method

- RustSHT source: 72bea50383f57bed6f54f48a3f360f01b180d5c5 on dev.
  For scalar-analysis timings, the 15-pair full-mmax parent is
  2b531d73049fdbc6e1b94b2132bfb9cb3476d319; the intervening changes only
  touched scalar synthesis and documentation. The isolated q1 Callgrind source
  worktree was 72bea503.
- Pinned DUCC 0.42.0:
  42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b. Its f32 interface uses f64
  internally for the Legendre stage.
- Host: AMD Ryzen 9 5950X; rustc 1.99.1 / LLVM 23.1.1; GCC 15.2.0; x86-64-v3,
  AVX2/FMA. Rust selected Pulp V3. Full and reduced fixtures use GL T720,
  ntheta=721, phi0=0.23, with nphi=1458/mmax=720 and
  nphi=378/mmax=180.
- Callgrind 3.26.0 recorded Ir only, with cache and branch simulation disabled.
  Each Rust and DUCC q1 worker warmed and reset the output before exactly one
  profiled analysis call. The Rust and DUCC binaries used equivalent
  x86-64-v3 settings. These are instruction references, not cycles or elapsed
  time.
- Whole-transform warm timings use matched Rust/DUCC workers. Full mmax has
  15 interleaved q1/q8 pairs; reduced mmax has three q1/q8 pairs and is a
  baseline screen, not an acceptance campaign. All workers reported finite,
  repeat-consistent, reset-consistent output.
- A temporary test-only route/worker census used existing workspace probes
  from an isolated source worktree. Its worker times include probe overhead and
  are used only to compare workers within each run. The exact temporary test
  patch, raw counters, profiles, disassembly, binaries and fixture hashes are
  archived in this directory.

## Current whole-transform results

| GL T720 native-f32 scalar analysis | Rust ms | DUCC ms | Rust/DUCC | Pairs | Max rel-L2 vs DUCC | Max abs error | Status |
|---|---:|---:|---:|---:|---:|---:|---|
| Full q1 | 545.836 | 14.017 | 38.94x | 15 | 1.829e-5 | 4.295e-7 | **GAP** |
| Full q8 | 72.382 | 2.015 | 35.93x | 15 | 1.829e-5 | 4.295e-7 | **GAP** |
| Reduced q1 | 193.015 | 5.781 | 33.38x | 3 | 2.205e-5 | 9.057e-7 | **GAP** |
| Reduced q8 | 26.056 | 0.876 | 29.89x | 3 | 2.205e-5 | 9.057e-7 | **GAP** |

The reduced q1/q8 figures are a fresh exact-source screen at 72bea503.
Full-mmax timings are the 15-pair parent screen at 2b531d73; scalar
analysis source is unchanged at 72bea503.

## Equivalent work, fallback and scaling

The existing per-order observations count useful **degree × reflected-latitude
pair** updates, after GL polar-order cutoffs:

| mmax | Useful pair updates | Active order × latitude pairs | Rust Ir | DUCC Ir | Rust Ir/update | DUCC Ir/update |
|---:|---:|---:|---:|---:|---:|---:|
| 720 | 161,199,393 | 387,265 | 6,170,457,791 | 214,206,155 | 38.28 | 1.33 |
| 180 | 81,210,177 | 128,497 | 2,377,721,728 | 86,529,402 | 29.28 | 1.07 |

DUCC's map2alm_kernel accounts for 51.9% of full and 64.6% of reduced
whole-worker Ir. It vectorizes across four reflected latitude pairs and
advances two degrees per inner-loop iteration. The Rust q1 profiles instead
show multiply_complex_scaled_f32 called 14.21M times full and 40.69M times
reduced. In the full profile that helper accounts for 36.78% of Ir; its
normal-factor component scaler accounts for 30.06% exclusive self Ir, with
__powisf2 at 6.71%. In the reduced profile, the paired analysis callback
and accumulator tree accounts for 88.03%; the scaled complex helper is 51.39%
inclusive. Parent/child percentages overlap and must not be added.

The temporary route census reports 161.199M/81.210M useful updates at q1 and
the same work at q8. Of the callback latitude blocks, full had 423 blocks that
saw only normal-range degrees and 716 that crossed normal and scaled-range
degrees; reduced had 186 and 176 respectively. There were no scaled-only
blocks. These are **range-status blocks**, not counts of fused blocks: the f32
ShtScalar implementation has no override for the fused scalar-analysis
block or its vector accumulator, so this callback work remains on the scalar
lane accumulator.

At q8, analysis scheduling does not explain the gap. All eight workers were
used. Full had 721 order jobs (89–91 per worker), 19.93–20.41M estimated
updates per worker, and 68.62–68.64 ms accumulated leaf time in the
instrumented run. Reduced had 181 jobs (22–23 per worker), 9.86–10.35M
updates and 23.18–24.19 ms per worker. Instrumented elapsed values are
diagnostic, but the work and job counts show no large idle-worker or
long-tail imbalance.

## Executed code

The Rust multiply_complex_scaled_f32 assembly has an exponent-zero pair of
scalar multiplies, but nonzero-exponent inputs enter repeated exponent and
normal-range branches, scalar component repair, and subnormal/overflow
fallbacks. The map2alm_kernel AVX2 loop uses YMM double-FMA for recurrence
and complex accumulation, keeps multiple independent vector sums, then
horizontally reduces once per degree pair. The compact SIMD suffix is the
main emitted-code contrast; Ir/update confirms the difference in executed
work without treating instruction counts as time.

The strongest next scalar-analysis hypothesis is one bounded native-f32
accumulation leaf for blocks that prove normal recurrence state, safe finite
spectrum products and bounded sums, retaining the existing scaled scalar
callback for every declined block. This needs contribution-level coverage
counts before implementation; the current block census is insufficient to
predict its gain. Keep the f64 analysis visitor unchanged. After that screen,
profile the measured spin-1 synthesis/analysis routes rather than reopening the
rejected scalar-synthesis suffix handoff or returning to f64 gaps.

M6 remains active. No performance parity is claimed.
