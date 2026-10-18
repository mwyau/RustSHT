# Scaled-mantissa no-op normalization screen

## Outcome

The one-pair screen is **inconclusive**. The no-op candidate's RustSHT series
passed the fixed stability limits and was 5.00% lower in median latency than
the unmodified scaled-prefix accumulator. Its DUCC series failed the
predeclared first-three/final-two drift limit (6.228% versus 5%). The v1
protocol requires every backend series to pass. No retry, T1023 follow-up,
q8 test, or regression matrix was run. The candidate remains isolated and is
not accepted for production.

This was a comparison against the scaled-prefix accumulator, not against the
clean production baseline. It does not replace the earlier six-pair clean-
baseline q1 measurements.

## Frozen source and configuration

The [screen protocol](scaled-mantissa-noop-screen-v1-protocol.md) was fixed
before collection. The baseline extension SHA-256 is
`4a389758f7a070c3ffa94e7e7ce23561b863e313a53677ca94d2aaa3e7434df9`; the
candidate extension SHA-256 is
`9739c6f96ffab888ed60785b3ce703e37e198c0726bc8906c86368f47c21d001`. The
candidate's exact source delta is preserved in
[`scaled-mantissa-noop-screen-v1.patch`](scaled-mantissa-noop-screen-v1.patch),
SHA-256
`a2f9d6adfe22ff8dbe9b49adfb46391c5fb11158373e6a054de981e48f728237`. It
applies to the protocol's frozen accumulator source. Before the patch,
`scalar.rs` was SHA-256
`388841b78579b75f96b62bac4ffb0684001cbf3be176b7a0dfb687999ca46442` and
`synthesis_fast.rs` was SHA-256
`e877d0bfab25fe3c643337526eb1d918f00e5374748192ce32408bcc7c6d3613`.
After the patch, their hashes are respectively
`cfe5f7ee31e75f70445fdca118f154cd39961ed5b0751f593e94c26794283a0a` and
`04aadcae00ab04feb18704e3f2d50687141ed088d32c7ce932222c4d186c308b`.

The workload was GL f64 scalar `STANDARD` synthesis, full `mmax`,
`phi0=0.37`, T2047 (`2048x4096`), one worker, plan scope. Each fresh process
ran one warm-up and five timed complete transformations for RustSHT and
DUCC; the baseline process ran first. DUCC was version 0.42.0 from source
revision `80388904b8cfc32400ba84af8a6a45aabfe60f86`, using the frozen
extension in the protocol. The benchmark used Python 3.11.15, NumPy 2.4.6,
Rust/Cargo 1.99.0, and release settings on Ubuntu 26.04.1, x86-64 with an
AMD Ryzen 9 5950X, AVX2/FMA, 16 physical cores, 32 logical CPUs, and 96 GiB
RAM.

## Complete-transform timings

All values are milliseconds. The drift column compares the median of the
first three timed calls with the median of the final two.

| Backend | Baseline samples | Median | CV | Drift | Candidate samples | Median | CV | Drift |
|---|---|---:|---:|---:|---|---:|---:|---:|
| RustSHT | 1352.500, 1355.530, 1354.577, 1358.297, 1353.470 | 1354.577 | 0.147% | 0.096% | 1286.743, 1289.767, 1285.176, 1287.488, 1286.857 | 1286.857 | 0.116% | 0.033% |
| DUCC | 415.379, 433.199, 475.881, 436.009, 428.302 | 433.199 | 4.645% | 0.241% | 463.054, 444.866, 465.363, 433.359, 437.340 | 444.866 | 2.927% | 6.228% |

The Rust candidate/baseline median ratio was `0.950007`, meeting the
predeclared Rust-only screen threshold of `0.98`. That result cannot advance
because the candidate DUCC drift failed. Its raw DUCC median is not used to
update the valid DUCC performance gap. The latest valid six-pair T2047 q1
comparison remains the accumulator result at 3.666x DUCC latency; its T511
control and complete regression matrix remain unresolved.

Both RustSHT outputs were compared with DUCC. Each had relative-L2 error
`3.015e-13`, RMS error `1.426e-14`, and maximum absolute error `1.110e-12`,
within the fixed tolerances. The helper's lane-by-lane test matched the
existing scaled normalizer bitwise, including normal, zero, extreme finite,
subnormal, and varied-exponent cases. The complete all-features workspace
suite passed on the candidate source: 203 library tests passed, 14 were
ignored, and the other workspace groups passed (4, 11, and 1 tests). The
format check passed. Clippy did not complete: rustc could not write metadata
because the temporary filesystem was full; this was an environment failure,
not a reported lint.

Peak resident memory was 611.4 MiB for the baseline and 611.8 MiB for the
candidate. The additional observed peak was 0.4 MiB.

## Validity and interpretation

Preflight and postflight CPU-pressure checks passed the 2.00% gate. A
during-run observation entered the warning band. Process and worker affinity
matched the declared one-worker placement; no concurrent build or benchmark
was observed. CPU frequency varied during the pair, and its effect is
unresolved.
No samples were removed. Under the fixed protocol the DUCC drift failure
ends this variant's screen; the existing numbers remain diagnostic evidence.

The measured prefix remains the next kernel bottleneck. The T2047 occupancy
profile found nonzero exponents in 49.08% of prefix lane-visits, and 98.69%
of those visits did not change exponent. The no-op helper is a focused
response to that evidence, and its Rust timing signal is promising but not
validated by the required backend stability gate. Do not infer a SIMD
throughput gain: the earlier tile comparison measured similar suffix
throughput for 128- and 256-ring tiles, with the larger tile spending more
time in the scaled prefix. Further performance claims require a new,
prospectively valid experiment with a genuinely new hypothesis; this screen
does not authorize another run of the same candidate.
