# Native-f32 T720 scaled-prefix handoff screen — 2026-10-28

**Decision:** reject the isolated suffix handoff as a production change. It executes a large safe suffix but does not show a consistent whole-transform gain across full/reduced mmax and q1/q8. A separate exponent-field-adjustment census found only 2 fully safe packets at full mmax and none at reduced mmax; do not implement this packet route or use lane masks to expand it. Production remains exact source `061a0467c506ae1711d00b952dd000c48d5e9cab`.

## Provenance and protocol

- Parent: RustSHT `061a0467c506ae1711d00b952dd000c48d5e9cab`; branch `dev`, matched against pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Candidate: isolated source patch only, SHA-256 `310ebd0c084c17c5fc49e85427aedf2269b8795fc1406533aa0609435105dbdc`. It added an early-stop control to the shared paired visitor and a transactional f32 accumulator transition to the existing typed fused kernel. It was never applied to production. The shared visitor touch is one reason not to carry this candidate forward unchanged.
- Host: AMD Ryzen 9 5950X; q1 and q8 used fixed one-core and eight-core affinity, respectively. Rust 1.99.1 / LLVM 23.1.1; DUCC GCC 15.2.0. Both used x86-64-v3/AVX2/FMA. Rust release flags: `-C target-cpu=x86-64-v3`; DUCC sources: `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; DUCC driver disabled fast-math. q1 and q8 runs used fixed single-core and eight-core affinity masks, respectively; host-local CPU IDs are kept with local raw captures.
- Fixtures: GL, `lmax=720`, `ntheta=721`, `phi0=0.23`; full `mmax=720`, `nphi=1458`; reduced `mmax=180`, `nphi=378`. Exact input hashes are in [provenance.json](provenance.json).
- Three interleaved fresh-process pairs per cell; each worker checked finite output, repeat consistency and workspace-reset consistency. Whole-transform measurements are uninstrumented. Separate one-call stage timers report ring-group recurrence/spectral-accumulation duration; q8 worker sums are not wall time.

## Whole-transform screen

Times are medians of the per-process warm batch medians; Rust/DUCC and candidate/parent are paired within each of the three interleaved pairs. Ratios are medians of the paired ratios. No cell justified a 15-pair extension.

| GL T720 f32 scalar synthesis | Parent ms | Candidate ms | DUCC ms | Candidate/parent | Parent/DUCC | Candidate/DUCC |
|---|---:|---:|---:|---:|---:|---:|
| Full, q1 | 728.544 | 716.543 | 19.976 | 0.9807 | 36.48× | 35.87× |
| Full, q8 | 117.736 | 113.332 | 2.821 | 0.9626 | 41.99× | 40.55× |
| Reduced, q1 | 216.951 | 215.479 | 8.550 | 0.9932 | 25.48× | 25.16× |
| Reduced, q8 | 59.764 | 60.548 | 1.289 | 1.0021 | 46.77× | 46.96× |

The full q1/q8 cells moved modestly, reduced q1 was near neutral, and reduced q8 was slightly slower. This is not a repeatable matrix improvement. The summarized three-pair timings are in [paired-summary.json](paired-summary.json). The raw worker records remain local and untracked.

The one-call Legendre-group timer is a diagnostic, not a timing acceptance sample. q1 measures one ring group; q8 shows the summed duration of each worker's groups, with the maximum worker sum approximating the recurrence-stage critical path before the remaining transform tail:

| Cell | Backend | Group duration sum (ms) | Maximum worker group sum (ms) | First whole transform (ms) |
|---|---|---:|---:|---:|
| Full q1 | Parent | 721.968 | 721.968 | 723.705 |
| Full q1 | Candidate | 725.408 | 725.408 | 727.186 |
| Full q8 | Parent | 732.570 | 111.375 | 115.065 |
| Full q8 | Candidate | 757.891 | 115.232 | 117.355 |
| Reduced q1 | Parent | 218.776 | 218.776 | 219.351 |
| Reduced q1 | Candidate | 216.382 | 216.382 | 216.782 |
| Reduced q8 | Parent | 225.832 | 65.315 | 65.466 |
| Reduced q8 | Candidate | 206.060 | 57.308 | 57.545 |

These timer samples show that the suffix candidate did not consistently reduce the Legendre-stage critical path; the whole-transform screen remains the decision gate. Full per-worker rows are in [stage-screen.csv](stage-screen.csv).

## Route counts and transition coverage

At exact parent `061a0467`, the active mathematical work is 80,696,637 degree–latitude contributions for full mmax and 40,637,387 for reduced mmax. The route census separates initially fused contributions, fallback visits, mixed/polar contributions, successful checked packets, unsafe packet retries, and partial SIMD tails in the compressed per-order logs under [raw](raw/).

The suffix candidate successfully handed off 31,115,584 full-q1, 30,799,296 full-q8, 13,806,400 reduced-q1 and 13,726,240 reduced-q8 contributions. It declined zero candidate suffix blocks after its transactional checks. That is substantial route coverage, but the uninstrumented whole-transform results above do not support promotion. The exact-parent packet route totals are summarized in [route-summary.csv](route-summary.csv).

The next bounded diagnostic tested exponent-field adjustment on each currently unscaleable full SIMD packet. For every lane it required normal coefficient, recurrence basis and product; bitwise equality to the existing scaled helper; a normal adjusted result; and finite normal-or-zero staged sums. A packet was counted only if **all** lanes passed, so this did not assume mixed-lane masking.

| Cell | Unscaleable packets | Exact lanes | Safe lanes | Fully safe packets |
|---|---:|---:|---:|---:|
| Full q1 | 446,244 | 364,030 | 294,829 | 2 |
| Full q8 | 624,474 | 462,717 | 375,835 | 2 |
| Reduced q1 | 160,361 | 212,049 | 193,683 | 0 |
| Reduced q8 | 199,189 | 238,999 | 216,979 | 0 |

Safe individual lanes are dispersed among packets that fail another lane's exactness or normal-range conditions. Processing those lanes separately would reintroduce the rejected mixed-lane route. The full-packet exponent-adjustment idea is therefore rejected before implementation.

## Instructions and emitted code

A matched one-call Callgrind profile at the exact parent measured 7,389,581,004 Rust Ir q1 and 6,928,340,193 q8, versus 260,814,851 and 260,387,345 DUCC Ir. These are whole-worker instruction counts, **not cycles or elapsed time**. q1 Rust source attribution puts 4,422M Ir inclusive under `multiply_complex_scaled_f32` (59.84% of the program); its two `multiply_component_scaled_with_normal_factor_f32` calls each account for about 1,657M inclusive Ir (22.43%). Nested inclusive totals must not be added. The sanitized counts are in [callgrind-summary.csv](callgrind-summary.csv); raw profiles and annotations remain local and untracked.

In the pinned DUCC binary, `calc_alm2map` advances the scaled prefix using `iter_to_ieee`, corrects the residual scale once, then calls `alm2map_kernel`. The executed AVX2 kernel uses four double lanes and an unrolled recurrence/accumulation body with packed FMA instructions, unit-stride recurrence and output updates, and no calls in its hot loop. See [DUCC prefix disassembly](assembly/ducc-scaled-prefix-transition.txt) and [kernel disassembly](assembly/ducc-alm2map-kernel.txt). Rust's executed scaled-component helper contains a scalar `vmulss`, exponent-bit extraction and checks, integer range branches, sign/mantissa reconstruction, and a large exceptional split/scale path; see [Rust helper disassembly](assembly/rust-scaled-component-helper.txt). This matches the measured cost of keeping millions of scaled contributions in the scalar callback. The source-level loop work is mathematically comparable, but a DUCC per-update counter was not collected in this screen; the Ir totals must not be presented as a per-update performance ratio.

## Numerical decision and next work

Candidate outputs were repeat- and reset-consistent and finite in all 36 paired workers. Candidate/parent relative L2 was `1.62e-7`–`1.81e-7` with maximum absolute difference `1.79e-7`–`2.38e-7`. Candidate/DUCC relative L2 remained `1.7234e-4` full and `2.7255e-4` reduced, with maximum absolute errors `9.6441e-4` and `1.5479e-3`, matching the parent's DUCC error. Per-cell values are in [correctness-errors.csv](correctness-errors.csv). The isolated focused synthesis tests passed 3/3. Because the initial screen was not compelling and the candidate changed accumulation order, analytic/adjoint/extreme and protected-f64 promotion gates were not run.

**Next:** measure a diagnostic f32-I/O/f64-internal Legendre route at the same T720 shapes to determine whether DUCC's wider exponent range and packed f64 kernel account for most of the remaining cost. Keep it isolated; do not change production precision. Instrument actual DUCC and Rust useful pair updates before claiming equivalent-work-normalized instruction rates. No production source changed in this screen; all four f32 Rust/DUCC cells remain **GAP**.
