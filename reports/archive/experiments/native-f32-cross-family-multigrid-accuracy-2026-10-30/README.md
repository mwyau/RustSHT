# Native-f32 B1–B3 accuracy characterization, T720

## Scope and provenance

This package extends independent numerical checks for the shared native-f32
scalar analysis (B1), spin-1 synthesis (B2) and spin-1 analysis (B3) routes.
It covers full `mmax=720`, all valid coefficient orders/degrees, GL/CC/F1
geometries, representative high-degree modes, tiny and large finite values,
adjoint identities, FFT-stage substitutions and q1/q8 repeatability. It does
not change production code or existing test tolerances.

The tested Rust `src/` tree is commit
`dc1aa4f65109fc93fef8a0595998df464a70a639`; current `dev` at
`9a117693b38ded0ce89bc75b30b34bdb3afa0b54` has the same `src/` tree. DUCC
0.42.0 is pinned at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, but this is
an accuracy package, not a new DUCC timing comparison. Rust test capture is
isolated in `harness/analysis-and-prepared-capture.patch.gz`; all Rust
transforms use native f32 or f64 inputs, recurrence, spectrum and FFT. The
independent reference uses GCC `__float128`/`libquadmath`. Build, source,
fixture and output hashes are recorded in `results.json` and `SHA256SUMS`.

GL fixtures use T720, `nphi=1458`, `phi0=0`, seeds 11/23/47 for scalar/Q/U
maps, and all valid `m=0..720,l=m..720`. CC uses 722 latitude rings and F1
721. For CC/F1 analysis, the reference consumes the actual post-resampling
Fourier arrays and prepared latitude geometry, so the recurrence/accumulation
stage is compared independently of meridional resampling. A separate float64
FFT computes the spectrum-stage error. B2's independent map reference starts
from the exact f32 E/B inputs and checks the complete synthesized map.

The prepared-grid spin reference includes the finite analytic spin-1
`m=1` pole limit. An initial reference variant incorrectly omitted that term
and made the matched f64 control appear `0.45–0.52%` wrong at `m=1`; a
one-order replay showed that including the limit restores f64 agreement to
about `7e-14`. The committed reference source uses the corrected endpoint
formula. This was a diagnostic-reference correction, not a production change.

## Full-array binary128 results

Values below are relative L2 over every valid coefficient, compared with the
binary128 projection of the exact captured f32 or f64 Fourier input. For CC/F1,
that input is the plan's prepared spectrum; the reference uses exact captured
prepared geometry. B3 E and B errors are listed separately.

| Family / grid | f32 vs binary128 | matched f64 vs binary128 | f32 spectrum substitution sensitivity |
|---|---:|---:|---:|
| B1 scalar analysis, GL | `1.86479e-5` | `5.19191e-14` | `1.33782e-7` |
| B1 scalar analysis, CC prepared | `1.84502e-5` | `5.40076e-14` | `2.32268e-7` |
| B1 scalar analysis, F1 prepared | `1.80995e-5` | `5.36230e-14` | `2.32696e-7` |
| B3 spin analysis E, GL | `1.84249e-5` | `5.24759e-14` | `1.33963e-7` |
| B3 spin analysis B, GL | `1.86401e-5` | `5.22753e-14` | `1.33672e-7` |
| B3 spin analysis E, CC prepared | `1.74279e-5` | `5.33676e-14` | `2.32478e-7` |
| B3 spin analysis B, CC prepared | `1.76923e-5` | `5.33482e-14` | `2.31924e-7` |
| B3 spin analysis E, F1 prepared | `1.75490e-5` | `5.35602e-14` | `2.32719e-7` |
| B3 spin analysis B, F1 prepared | `1.75796e-5` | `5.33853e-14` | `2.32948e-7` |

The f32-versus-f64 spectrum substitution moves each binary128 projection by
`1.34–2.33e-7` relative L2. Across these normal-scale full-array cases, the
total coefficient error remains about `1.7–1.9e-5`; the FFT input accounts
for only a small fraction of that error. The remaining error is after the
longitude FFT, in the Legendre evaluation and accumulation path. These
measurements do not yet separate recurrence-state error from accumulation
rounding.

The f32 forward FFT differs from an independent float64 FFT by `1.40–1.41e-7`
relative L2 on normal-scale GL, CC and F1 fixtures; f64 controls are about
`6e-16`. The captured B1 and B3 f32 spectra and output coefficient arrays are
byte-identical between q1 and q8 on each grid. Selected order/degrees and exact
hashes are in the generated summaries and `results.json`.

For direct raw-grid projections of the random CC/F1 map fixtures, both f32 and
f64 differ from a direct weighted sum by about `0.38–0.43%`. Those inputs are
not bandlimited, and the Rust API applies meridional resampling; this direct
quadrature comparison is not a precision error measurement. The prepared-grid
comparison above isolates the post-resampling transform stage instead.

## B2 full maps, modes and magnitude range

The high-band B2 fixture sets deterministic coefficients for every order and
`l=600..720`, with exact f32 inputs widened only for the matched f64 control.
All map values and Fourier orders are included.

| Grid / route | Q f32 map rel L2 | U f32 map rel L2 | matched f64 map rel L2 | f32 inverse-FFT residual |
|---|---:|---:|---:|---:|
| GL q1 | `1.46456e-4` | `1.38105e-4` | `1.33–1.36e-13` | `1.42e-7` |
| CC q1 | `6.17821e-5` | `6.75315e-5` | `7.52–7.66e-14` | `1.42e-7` |
| CC q8 | `1.43558e-4` | `1.38737e-4` | `8.53–8.55e-14` | `1.42e-7` |
| F1 q1/q8 (identical f32 map) | `6.50491e-5` | `7.40059e-5` | `1.80–2.21e-13` | `1.42e-7` |

The GL map envelope-scaled maxima are `3.312e-3` Q and `2.652e-3` U. The
full-map error is already present in the pre-FFT spectrum. Channel preparation
is much smaller in the sampled cases: its relative L2 is `1.6e-8–6.3e-8`;
at `m=1`, ring 0, the prepared-channel absolute error is `2.86e-4` Q and
`1.22e-4` U, while the downstream binary128 spectrum errors are `2.41e-2`
and `2.52e-2`. Recurrence and accumulation remain grouped in that downstream
measurement.

Isolated F1 modes `(l=600,m=1,E.re)`, `(720,1,E.im)`, `(650,5,B.re)`,
`(720,16,B.im)`, `(720,719,E.re)` and `(720,720,E.re)` have map relative-L2
errors from `1.13e-5` to `1.94e-3`; the largest is Q for `(720,1,E.im)`. The
CC `(720,1,E.re)` error is `1.127e-3` Q and `1.685e-3` U. F1 q1/q8 outputs
are byte-identical; the selected CC q1/q8 outputs are not. Each route was
compared separately to the same binary128 reference. The known reduced-order
B2 q1/q8 arithmetic route from `dc1aa4f` is not retraced here.

The F1 `(l=720,m=720,E.re)` input was tested at coefficient bit patterns
`0x00000001` (minimum subnormal), `0x00000200` (`2^-140`), `0x00800000`
(minimum normal), and `0x7b800000` (`2^120`). At the two smallest scales,
relative errors range from `0.035` to `1` because expected map values are zero
or subnormal; maximum absolute map error is at most `8.01e-44`. At minimum
normal and `2^120`, relative L2 is `1.16e-5–1.63e-5`. The large case has no
nonfinite output. F1 f32 q1/q8 outputs match for all tested scales. Complete
per-mode, scale and q1/q8 summaries are in `generated/b2-*-fullmap-q1q8.json`
and `results.json`.

## Exceptional analysis scales and adjoint identities

B1/B3 GL maps were scaled by `0x00000200` and `0x7b800000` (`2^-140` and
`2^120`). At the low scale, over 99.9% of f32 map components are subnormal.
The f32 analysis relative-L2 error is about `1.01`, but maximum absolute
coefficient error is only `6.1–6.6e-45`, below the operation-count absolute
roundoff screen below. At `2^120`, all outputs remain finite; B1/B3 relative
L2 errors are about `1.85e-5`, with f32 Fourier magnitudes below the maximum
finite f32 value. q1/q8 B1/B3 spectra and outputs are byte-identical at both
scales. The f32 FFT relative error rises to `5.27e-2` at the low scale, while
the absolute FFT error remains subnormal; relative error is not meaningful
for that case.

High-band adjoint identities were checked on GL, CC and F1 at q1/q8. The
maximum condition-scaled f32 residual is `5.02e-9` for B1 scalar analysis,
`2.64e-7` for B2 synthesis, and `1.79e-7` for B3 spin analysis. These check
adjoint self-consistency, not closeness to an independent expected transform.

## Proposed numerical screen

For T720, use f32 unit roundoff `u=2^-24`, `L=720` recurrence steps and
`Ntheta=721` rings. A pessimistic operation count for the shared recurrence
and complex accumulation is `K=4L+8Ntheta=8648`; its first-order rounding
bound is `gamma_K=Ku/(1-Ku)=5.157268e-4`. For subnormal outputs, the crude
absolute bound `K*2^-149=1.211843e-41` is more useful than relative error.
This is a proposed screening budget for normal, moderately conditioned
outputs, not a theorem for a recurrence with unmeasured state conditioning and
not an application-approved science tolerance. No test tolerance was changed.

B1/B3 normal full-array relative-L2 errors are below this screen, and their
maximum envelope-scaled errors are `4.5e-6–1.83e-5`. B2 full-field relative-L2
errors are also below it, but B2 high-degree single-mode errors reach
`1.94e-3`, and its GL envelope-scaled maxima reach `3.31e-3`; those exceed the
screen. Thus these data do not justify accepting all three families. B2 has a
demonstrated high-degree accuracy gap; recurrence-state versus accumulation
error remains to be separated for B1–B3. The independent application
requirement still needs an owner decision before final numerical acceptance.

## Reproduction and retained evidence

From a detached worktree at the source SHA above, apply the compressed
test-only capture patch, copy `phasec_fullarray_accuracy_dump.rs` into
`examples/`, and run it with `PHASEC_GRID_KIND=gl|cc|f1`,
`PHASEC_MAP_SCALE_BITS=00000200|7b800000`, and `RAYON_NUM_THREADS=1|8`.
Then run the ignored capture test
`phasec_capture_fullarray_analysis_spectra` with `PHASEC_FULLARRAY_DIR` set
to the fixture directory. Compile
`harness/fullarray_analysis_reference_multigrid.cpp` with
`g++ -std=c++20 -O2 -march=x86-64-v3 -lquadmath`; use `prepared` mode for
CC/F1. The helper `fullarray_fft_stage_multigrid.py` compares captured
spectra with float64 FFT output. `phasec_highband_adjoint_identity.rs` captures
the independent adjoint-identity inputs/results. B2 uses the full-map
reference and comparator in the adjacent B2 package.

Raw map and spectrum arrays remain outside Git. This package retains the
deterministic dump source, test-only patch, binary128/FFT reference sources,
compact machine-readable results, bounded logs and checksums. No B1/B2/B3
family is marked numerical PASS.
