# Native-f32 GL T720 current-source all-family baseline

**Status:** baseline only; no production source changed. The f32 scalar-synthesis
cells remain **GAP** against pinned DUCC. This screen confirms that the broad
native-f32 disparity also appears in scalar analysis and spin-1 analysis and
synthesis. It does not compare a candidate to an exact source parent and is not
an acceptance campaign.

## Source and method

- RustSHT source: `b3289762c7ba019523e4908b6172109d6975bfca` on `dev`; the
  production transform sources match `54b166f2c0698f1c70e88f8679d12fa4376704fe`.
- Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host: AMD Ryzen 9 5950X; rustc 1.99.1 / LLVM 23.1.1; GCC 15.2.0.
- Rust release used `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; Pulp retained
  runtime dispatch and selected V3 with eight f32 lanes. DUCC used
  `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, with the driver built
  using `-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3`.
- GL T720, `ntheta=721`, even longitude sampling, `phi0=0.23`; full
  `mmax=720,nphi=1458` and reduced `mmax=180,nphi=378`. The screen covered
  scalar (`spin=0`) synthesis and analysis, and spin-1 synthesis and analysis,
  at q1 and q8.
- Three alternating fresh-process Rust/DUCC pairs per cell used the same
  fixtures, reusable Rust plan/workspace, and warm-call boundary. q1 was pinned
  to CPU 3; q8 to CPUs 8–15. Rust timed batches used five samples when one call
  exceeded 100 ms, otherwise ten calibrated batches. Timings include the
  public transform call and exclude process startup/setup.
- All 96 worker processes passed finite-output, repeat-consistency, and
  workspace-reset checks. Error columns report maximum per-pair Rust-vs-DUCC
  relative L2 and maximum absolute error. Independent analytic, adjoint and
  exceptional-value acceptance checks were not part of this baseline.

The exact result archive, CSV, fixture hashes, binary hashes, and harness hash
are in this directory. The Rust and DUCC executable SHA-256 values and evidence
hashes are recorded in `provenance.json`.

## Whole-transform timing

Medians over the three paired process comparisons. Lower is faster.

| Workload | q | Rust ms | DUCC ms | Rust/DUCC | Max rel-L2 | Max abs error | Status |
|---|---:|---:|---:|---:|---:|---:|---|
| Scalar synthesis, full | 1 | 729.692 | 19.819 | 36.737× | 1.723e-4 | 9.644e-4 | GAP |
| Scalar synthesis, full | 8 | 110.256 | 2.797 | 40.512× | 1.723e-4 | 9.644e-4 | GAP |
| Scalar synthesis, reduced | 1 | 217.799 | 8.646 | 25.258× | 2.725e-4 | 1.548e-3 | GAP |
| Scalar synthesis, reduced | 8 | 57.912 | 1.261 | 45.888× | 2.725e-4 | 1.548e-3 | GAP |
| Scalar analysis, full | 1 | 577.776 | 15.056 | 38.755× | 1.829e-5 | 4.295e-7 | GAP |
| Scalar analysis, full | 8 | 75.228 | 2.158 | 35.045× | 1.829e-5 | 4.295e-7 | GAP |
| Spin-1 analysis, full | 1 | 1109.673 | 31.420 | 35.210× | 1.569e-5 | 2.231e-7 | GAP |
| Spin-1 analysis, full | 8 | 142.164 | 4.360 | 32.691× | 1.569e-5 | 2.231e-7 | GAP |
| Spin-1 synthesis, full | 1 | 1595.214 | 42.627 | 37.654× | 1.426e-4 | 1.172e-3 | GAP |
| Spin-1 synthesis, full | 8 | 276.636 | 5.765 | 47.938× | 1.426e-4 | 1.172e-3 | GAP |
| Scalar analysis, reduced | 1 | 205.135 | 6.231 | 32.776× | 2.205e-5 | 9.057e-7 | GAP |
| Scalar analysis, reduced | 8 | 27.223 | 0.941 | 29.321× | 2.205e-5 | 9.057e-7 | GAP |
| Spin-1 analysis, reduced | 1 | 397.808 | 12.924 | 30.740× | 1.172e-5 | 3.504e-7 | GAP |
| Spin-1 analysis, reduced | 8 | 51.490 | 1.819 | 28.088× | 1.172e-5 | 3.504e-7 | GAP |
| Spin-1 synthesis, reduced | 1 | 657.734 | 18.363 | 35.864× | 1.415e-4 | 9.133e-4 | GAP |
| Spin-1 synthesis, reduced | 8 | 105.409 | 2.594 | 40.444× | 1.415e-4 | 9.133e-4 | GAP |

The fresh scalar-synthesis ratios reproduce the earlier source-specific
baseline within timing variation. q8 remains far from DUCC even though the
accepted 32-ring scheduler uses all workers; earlier worker diagnostics show
that q8 imbalance is larger for reduced mmax but cannot explain the q1 gap.

## Decision and next work

Keep production unchanged. The scaled-to-normal suffix handoff remains blocked
by actual nonzero f32 recurrence states becoming zero when their exponents are
folded; a same-source f32-I/O/f64-internal diagnostic reaches 0.75–0.78× DUCC
on the reflected order-major route. A reflected-ring f32 synthesis screen showed
large gains but added a separate 513-line f32 recurrence module and regressed
reduced scalar-analysis q8 by 4.3%; do not restore it. The next synthesis
hypothesis must retain DUCC's reflected work reduction while reusing the
existing paired recurrence and typed accumulation architecture, and must keep
the analysis dispatch/source layout isolated enough to test the prior control
regression directly.
