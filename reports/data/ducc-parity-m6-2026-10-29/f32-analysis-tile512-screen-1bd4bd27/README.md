# Shared scalar-analysis tile 512 screen

## Decision

Do not promote the tile-width change. The isolated direct-spectrum f32-I/O / f64-recurrence route gained about 1% over tile 256, but three of four GL T720 cells remain slower than native-f32 DUCC. Only reduced q8 is a performance PASS, and that route remains diagnostic pending independent accuracy coverage. Keep the production tile at 256.

## Provenance and protocol

- Exact production source parent: `1bd4bd274cacda38f87cc741c2d4f20342107b8f`. Current `dev` descendants through `979de36e` contain report-only changes to the source modules used here.
- Candidate source: parent plus the test-only direct-spectrum harness and `tile512-only.patch`; the cumulative uncompressed candidate patch SHA-256 is `dbf4d42cb2d9e18b3f64eb77d56381f6a393e0ab98372eac85f7058e8813ebad`. The production delta widens `SCALAR_ANALYSIS_PAIR_TILE_SIZE` from 256 to 512 and sizes its reusable analysis vector buffer accordingly. It does not alter recurrence equations.
- Pinned DUCC: 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, native-f32 API for scalar analysis.
- Rust: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, flags `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. DUCC: GCC 15.2, `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` (driver compiled with `-fno-fast-math`).
- Exact GL T720 inputs and SHA-256 values are in `f32-fixture-sha256.json`; protected GL T1023 inputs are in `f64-t1023-fixture-sha256.json`. Rust and DUCC binaries, including SHA-256, are in `binary-hashes.txt`.
- Host: AMD Ryzen 9 5950X, AVX2/FMA. The direct-route screen ran 15 interleaved parent/candidate/DUCC comparisons per cell. Each Rust worker reused one plan/workspace, warmed twice, then timed seven calls; DUCC used a matching warmed worker with a 25 ms batch. Setup was outside timed regions. q1 was pinned to CPU 0; q8 to CPUs 0–7. The separately compiled protected f64 control ran three interleaved triplets, with q1 pinned to CPU 0 and q8 to CPUs 0,2,4,6,8,10,12,14. Raw pairs and the exact runners are retained here.

## Direct-spectrum route, GL T720 f32 scalar analysis

Times are median milliseconds. C/P and C/D are paired candidate/parent and candidate/DUCC median ratios with paired-bootstrap 95% percentile intervals (50,000 resamples, seed 9062029).

| Cell | Parent | Candidate | DUCC f32 | C/P [95% CI] | C/D [95% CI] | Status |
|---|---:|---:|---:|---:|---:|---|
| Reduced q1 | 6.008 | 5.955 | 5.754 | 0.988 [0.980, 0.998] | 1.033 [1.030, 1.038] | GAP |
| Reduced q8 | 0.877 | 0.874 | 0.893 | 0.990 [0.975, 1.006] | 0.974 [0.968, 0.992] | PASS, diagnostic route only |
| Full q1 | 14.667 | 14.534 | 14.119 | 0.991 [0.988, 0.996] | 1.029 [1.025, 1.033] | GAP |
| Full q8 | 2.130 | 2.111 | 2.055 | 0.991 [0.950, 1.011] | 1.027 [1.017, 1.048] | GAP |

The candidate output was bitwise equal to its parent in every pair; all outputs were finite and poisoned-workspace reset checks passed. Candidate-vs-DUCC relative-L2 / maximum-absolute errors were `1.319e-7 / 1.397e-9` reduced and `1.432e-7 / 6.985e-10` full. This is cross-implementation agreement, not an independent accuracy reference.

## Adjacent controls

The native-f32 scalar-analysis controls had only three pairs each and are screening evidence, not acceptance intervals. Candidate/parent medians were 1.003 reduced q1, 0.988 reduced q8, 0.982 full q1, and 1.001 full q8; each result was bitwise equal to parent. The apparent 1.8% full-q1 gain is not enough to justify a global workspace increase from one three-pair screen. Native-f32 candidate/DUCC remained about 24x, 21x, 32x, and 29x for reduced q1/q8 and full q1/q8.

For protected f64 GL T1023 spin-1 synthesis, the three-pair candidate/parent paired medians were 1.005 q1 (95% interval 0.995–1.011) and 1.010 q8 (0.981–1.062). Outputs were bitwise equal to parent, finite, and passed repeat/reset checks. These short, noisy controls show no established regression and do not certify the tile change for production. Candidate/DUCC medians were 0.744 q1 and 0.824 q8.

## Mechanism, memory, and validation limits

The candidate stage profile used 30 warm calls per case and is diagnostic, not paired acceptance timing. FFT / promotion / recurrence-and-gather / writeback medians in milliseconds were: reduced q1 `0.632 / 0 / 5.302 / 0.034`; reduced q8 `0.109 / 0 / 0.772 / 0.039`; full q1 `2.587 / 0 / 11.919 / 0.081`; full q8 `0.377 / 0 / 1.658 / 0.085`. Recurrence and gather consumed about 78–89% of the whole call. Candidate peak RSS was 27,268 / 33,896 / 76,800 / 87,616 KiB in the same cell order. A new allocation count was not collected for this screen; the previous direct-route probe found one small q8 warm-call allocation shared with promoted-spectrum and native routes, not a direct-route-only allocation.

`cargo fmt --all -- --check` passed for both isolated source trees. The paired harness checked finite outputs, bitwise parent equality, repeat calls and poisoned-workspace reset. No independent analytic, adjoint, exceptional-value, or broader-bandlimit suite was run on this non-promoted tile candidate. No production behavior or precision policy changed.

## Recoverable artifacts

- `tile512-only.patch.gz`: production tile-size delta.
- `direct-route-256-test-harness.patch.gz`: diagnostic f32-FFT / f64-recurrence direct-spectrum harness.
- `tile512-cumulative.patch.gz`: complete isolated candidate patch.
- `tile512-direct-route-paired.json.gz`, `tile512-native-controls.json.gz`, `protected-f64-results.json.gz`: timing and validation records.
- `stage-profiles.json`, fixture manifests, runner snapshots, and `binary-hashes.txt`: stage, input, runner, and binary provenance.
- `SHA256SUMS.txt`: package integrity manifest.
