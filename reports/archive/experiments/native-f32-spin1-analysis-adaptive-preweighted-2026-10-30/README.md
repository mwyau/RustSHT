# Native-f32 spin-1 analysis adaptive/preweighted composition screen

This isolated candidate combines the measured adaptive reflected-ring schedule
(32-pair scale-sensitive prefix, 256-pair suffix) with the existing
scale-epoch channel-preweighting experiment. Its base is
`053c5668079fa89f37438a488b2545b5ceab1e69`, whose `src/` tree matches the
adaptive-only screen base `00f9770741a6e3e84646217d51374e35515daaa1`. The
combined source diff SHA-256 is
`d83f1442fa7cc7b2c27880df12b352cb974394b71962e83aa21bde1ce0979ab9`; its
recoverable compressed patch is `candidate.patch.gz`.

The screen used three interleaved pairs on Ryzen 9 5950X, GL T720, full M720
and reduced M180, q1/q8 pinned to CPUs 0 and 0–7. Rust used
`-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; the matched f64 worker was built
from this exact candidate source and used widened copies of the same f32
fixtures only for the diagnostic comparison. DUCC was native f32 at pinned
revision `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

| Cell | f32 ms | matched f64 ms | f32/f64 | DUCC ms | f32/DUCC |
|---|---:|---:|---:|---:|---:|
| Full q1 | 143.254 | 37.919 | 3.758x | 29.371 | 4.833x |
| Full q8 | 19.225 | 6.694 | 2.893x | 4.771 | 4.030x |
| Reduced q1 | 46.930 | 15.002 | 3.118x | 11.871 | 3.938x |
| Reduced q8 | 6.431 | 2.239 | 2.873x | 1.754 | 3.660x |

All four cells miss the interim 2.0x f64 target. This composition was slower
than the separately screened adaptive-only candidate on both full-grid cells
and approximately even on reduced cells; the comparison is exploratory across
separate three-pair runs, not a paired candidate-versus-parent claim. This
does not justify more tile-size tuning or a 15-pair screen. The bounded result
leaves scale-sensitive recurrence/accumulation work as the B3 blocker. Random
fixture relative L2 versus DUCC was `1.847e-5` full and `2.171e-5` reduced;
independent accuracy was pending at the time of the timing screen. The
follow-up below records an independent binary128 comparison for this exact
candidate.

Validation passed: format check, release library check, strict library
Clippy, 51 nonignored `spin1_` library tests, the dedicated scaled-seed replay,
and 14 f32 integration tests. Five spin unit tests and one f32 integration
test were ignored by their existing opt-in annotations. No production code was
promoted. Exact pair records, fixture and worker hashes are in
`screen-three-pair.json`; `SHA256SUMS` covers the package artifacts.

## Independent accuracy and Fourier-stage follow-up

The exact combined source candidate was checked against an independently
implemented GCC binary128/libquadmath reference. Machine-readable metrics,
source/binary identities and fixture/output hashes are in
[`accuracy-independent-binary128.json`](accuracy-independent-binary128.json).
The reference code and test-only spectrum observer are reused from the
[cross-family diagnostic package](../native-f32-cross-family-accuracy-2026-10-30/README.md).

On GL T720 with `mmax=16`, `nphi=34`, the reference covers every degree for
orders 0 through 16 on deterministic scalar, Q and U maps. Spin-analysis
relative L2 errors are `3.8157436e-5` for E and `3.2032681e-5` for B; matched
f64 controls are about `6.5e-14`. Replacing the actual f32 Fourier input with
the direct binary128 Fourier projection changes the E/B relative-L2 errors by
only `5.33e-10` / `8.18e-10`; forward-FFT relative residuals are `1.09e-7` for
Q and `1.11e-7` for U. The FFT is therefore not the main error source on this
fixture. This comparison does not separate recurrence error from coefficient
accumulation error.

A separate full-`mmax=720` GL T720 check covers 11 selected orders and degrees
600 through 720 (1,092 coefficients per result). Relative L2 is `4.8306702e-5`
for E and `5.3298096e-5` for B; q1 and q8 outputs and comparison files are
byte-identical. These are selected-mode results rather than full-order
acceptance. No scientific tolerance was supplied, so the candidate remains
numerically unaccepted. Existing focused spin-analysis and f32 integration
tests passed on the isolated source; this evidence does not replace
application-tolerance, exceptional-magnitude or complete full-order
validation.

To reproduce from a clean worktree, apply `candidate.patch.gz`, then apply the
test-only observer and copy the two Rust harnesses from the cross-family
package. Build/run with `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'`
and `RAYON_NUM_THREADS=1` or `8`; use the mmax16 harness with output directory
`spectra-q1`/`spectra-q8` and the full-order harness with
`accuracy-q1`/`accuracy-q8`. Build the matching C++ references with
`c++ -std=c++20 -O2 -march=x86-64-v3 <reference.cpp> -lquadmath`, then run each
binary on its corresponding output directory. All inputs and comparison
tables are deterministic; their hashes are recorded in the JSON. Full raw
CSV traces and generated map/coefficient arrays remain outside the repository.
