# Native-f32 scalar-analysis subnormal-prefix screen

This bounded candidate adds a conservative omission check for a prefix whose
total scaled contribution is strictly below half the least f32 subnormal. It
uses the shared native-f32 scalar-analysis recurrence and SIMD path. The base
is `053c5668079fa89f37438a488b2545b5ceab1e69`; the candidate source patch
SHA-256 is `645951812d6dc995c3921d16f32af4091f5d67ed46c2259fce897a0f934320c5`.
The no-skip control patch SHA-256 is
`adc46fb915fddd07f788347f9467879f7fd0301e54c575280abfa09d3d4a8693`.

The screen used three interleaved pairs on Ryzen 9 5950X, GL T720, full M720
and reduced M180, q1/q8 pinned to CPUs 0 and 0–7. Rust used
`-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; DUCC was native f32 at pinned
revision `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The f64 worker was
rebuilt from the candidate source and widened the same f32 fixture values only
for the diagnostic comparison. Both screens use identical fixture hashes.

| Cell | f32 ms | matched f64 ms | f32/f64 | DUCC ms | f32/DUCC |
|---|---:|---:|---:|---:|---:|
| Full q1 | 51.296 | 20.261 | 2.544x | 14.109 | 3.636x |
| Full q8 | 6.971 | 2.864 | 2.420x | 2.072 | 3.359x |
| Reduced q1 | 18.773 | 8.453 | 2.224x | 5.762 | 3.247x |
| Reduced q8 | 2.623 | 1.241 | 2.122x | 0.903 | 2.913x |

All four cells miss the interim 2.0x f64 target. Against the same-source
adaptive candidate with prefix omission disabled, the candidate saves 5.4–6.3%
per cell. The full q1 random fixture output was byte-identical between enabled
and disabled prefix variants across 520,562 f32 values. This is preservation
evidence, not independent numerical accuracy acceptance. Relative L2 versus
DUCC on those random fixtures was `1.839e-5` full and `2.220e-5` reduced.
The archived JSON keeps all pair timings, error metrics, fixture hashes and
binary hashes; temporary output-file paths and transient host telemetry are
omitted because those files were deleted after comparison and the live host
readings are not part of the reproducible measurement.

Formatting, release library check, the strict prefix-bound unit test, and the
T720 scalar-analysis analytic/adjoint/workspace-reset integration test passed.
The full scalar-analysis suite was not rerun on this variant. At the time of
the timing screen, independent high-band accuracy was still open. The exact
candidate has since received a binary128 and Fourier-stage follow-up below;
no science tolerance is declared.

The candidate remains isolated and is a bounded GAP. The contribution bound
does not remove enough scale-sensitive recurrence and accumulation work to
reach the timing gate. Exact paired measurements, fixture hashes, binary
hashes, and validation details are in the two JSON files and
`SHA256SUMS`. `candidate.patch.gz` and `no-skip-control.patch.gz` are
recoverable against the recorded base.

To replay, apply the patch in a clean worktree at the recorded base, build the
native-f32 worker and packaged matched-f64 worker with the recorded Rust flags,
and set `RUSTSHT_B1_WORKTREE`, `RUSTSHT_B1_BUILD`, `RUSTSHT_B1_DUCC`,
`RUSTSHT_B1_SCREEN_RUST`, and `RUSTSHT_B1_SCREEN_OUTDIR` before running
`python3 run_screen.py`.

## Independent accuracy and Fourier-stage follow-up

The exact prefix candidate was checked against the independent GCC
binary128/libquadmath reference used by the [cross-family diagnostic
package](../native-f32-cross-family-accuracy-2026-10-30/README.md). Metrics,
source and binary hashes, and fixture/output hashes are in
[`accuracy-independent-binary128.json`](accuracy-independent-binary128.json).

For GL T720 `mmax=16`, all output degrees for orders 0 through 16 were
compared on deterministic scalar, Q and U maps. Relative L2 is `3.6553657e-5`
for scalar analysis, `3.8152882e-5` for spin E and `3.2030944e-5` for spin B;
matched f64 controls are approximately `6.5e-14`. Replacing the native-f32
Fourier input with direct binary128 Fourier sums changes these relative-L2
errors by only `9.02e-10`, `5.29e-10` and `8.06e-10`; forward-FFT residuals
are about `1.1e-7`. This places the dominant discrepancy after Fourier
extraction on these fixtures, but does not separate recurrence from
coefficient accumulation.

On the full `mmax=720` grid, the selected 11 orders and degrees 600 through
720 yield relative L2 `4.9080809e-5` for scalar analysis, `4.8309102e-5` for
spin E and `5.3312544e-5` for spin B. q1 and q8 analysis coefficients and
comparison tables are byte-identical. These checks use the exact timing
candidate but cover selected orders only at full mmax. They are not
full-order accuracy acceptance, and no scientific tolerance was supplied.
Existing analytic/adjoint/workspace tests remain separate validation; the
full scalar-analysis suite and exceptional-magnitude coverage have not been
rerun on this variant.

To reproduce, apply `candidate.patch.gz` at the recorded base, then apply the
test-only spectrum observer and copy `phasec_accuracy_dump.rs` and
`phasec_fullorder_accuracy_dump.rs` from the cross-family package. Build with
`RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'`, using
`RAYON_NUM_THREADS=1` or `8`. Run the mmax16 example and spectrum capture into
one directory, copy the example's map/geometry/output files alongside the
captured spectra, then run the mmax16 binary128 reference. Run the full-order
example and reference separately for selected high-band outputs. Reference
source and binary hashes are recorded in the JSON. Full raw CSVs and generated
arrays remain outside the repository.
