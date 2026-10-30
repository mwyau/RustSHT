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
The full scalar-analysis suite was not rerun on this variant. Independent
high-band accuracy is OPEN: the preceding adaptive-only result was about
`4.9081e-5` relative L2 against binary128, and this prefix variant has no new
independent high-band result or declared science tolerance.

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
