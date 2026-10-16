# Scalar f32 recurrence normalization skip at T1279

## Result

The normal-range normalization skip previously enabled only at T1023 also
improved complete scalar f32 transforms at T1279. Three alternating process
pairs on GL `1280×2560`, full `mmax=1279`, and `phi0=0.37` favored the candidate
for synthesis and analysis at both one and eight threads. The selector remains
limited to the measured T1023 and T1279 cases; T720 and T2047 retain the
established path.

| Operation | Threads | Baseline median | Candidate median | Paired change median | Paired range |
| --- | ---: | ---: | ---: | ---: | ---: |
| Synthesis | 1 | 7,452.405 ms | 7,132.825 ms | −4.13% | −5.22% to −4.08% |
| Synthesis | 8 | 2,699.080 ms | 2,662.328 ms | −1.36% | −1.88% to −0.64% |
| Analysis | 1 | 3,700.838 ms | 3,463.289 ms | −6.50% | −6.58% to −5.97% |
| Analysis | 8 | 682.756 ms | 660.584 ms | −3.42% | −3.89% to −2.45% |

Each process constructed and reused one plan and q8 workspace. It measured a
first call and five warm samples for each scalar STANDARD operation at q1 and
q8. The final selector uses exact matches for T1023 and T1279. Its disabled-path
T720 control used GL `721×1458`; median changes ranged from −0.74% to −2.68%
across q1/q8 operations, and every pair favored the candidate. A previous
two-entry array-search selector regressed T720 by 1.69%–4.07%; that form was
rejected and its raw data are retained. The final T1279 peak RSS medians were
79,592 KiB for baseline and 79,768 KiB for candidate, a 176 KiB increase.

## Numerical checks

The complete T1279 scalar synthesis map (3,276,800 float32 values) and analysis
vector (819,840 complex64 coefficients) were bitwise identical between the
fallback and candidate. A matched DUCC 0.41.0 comparison used source revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`, GL `1280×2560`, full T1279,
`phi0=0.37`, and one thread. Full-array relative-L2 differences were
`3.77e-5` for synthesis and `2.05e-5` for analysis. The analysis reference
used the captured baseline synthesis map as its input, so both implementations
were checked with identical map samples. RustSHT has not set a T1279 f32
acceptance threshold; these results characterize accuracy without defining one.

The opt-in T1023 scalar and spin-1 DUCC test also passed with the expanded
selector. Formatting, Clippy, workspace all-feature tests, and no-default tests
passed. The first all-feature run reached doc tests after 188 unit and 11 f32
tests passed, then failed because the shared `/tmp` tmpfs was full. The rerun
with `TMPDIR=target/tmp` passed. Python tests were not run because the bindings
did not change.

## Measurement record

The baseline was commit `1bc1403b7551f62d70f0e7305e3bf5c8e535993d`; at T1279
its compile-time gate selected the established normalizer. The final candidate
kept the T1023 path and added an exact T1279 match. Release benchmark binaries were built with
`cargo build --release --example m1_f32_atmospheric_bench --features parallel`
and used the default release profile without global native CPU flags. Their
SHA-256 hashes were:

- Baseline: `785db7f0e599df71f8788d93d1422eb2c17df51635e60ba087537f396831c237`
- Rejected array-search candidate: `5f97eeb730cd1a14810b8b490f1e100ebfa4b13f77f6a29f427a43102f4f8176`
- Final exact-match candidate: `7811fccf53f914389a689fb6b87f7ca775321fa4d9a5e7b3c6a33a4719ddb2a5`

The host was an AMD Ryzen 9 5950X with AVX2/FMA and no AVX-512. Rust was
`1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`. The alternating process script
pinned both q1 and q8 work to CPUs 0–7; the initial load average was
`0.63, 0.93, 2.00`. A mid-run process check showed the benchmark as the only
high-CPU process. Complete T1279 arrays were captured from the same recurrence
specialization selected by the final exact-match gate; the output comparison
binaries have separate hashes in the data README. The initial T1279 summary
uses the rejected array-search gate and is retained as negative evidence. Final
exact-match timings are in `paired-T1279-match/` and
`paired-T720-match-control/`. Raw process logs, elapsed time and RSS, complete
output arrays, hashes, comparison scripts, and validation logs are in
[`reports/data/f32-scalar-normalization-skip-T1279-2026-10-16`](data/f32-scalar-normalization-skip-T1279-2026-10-16/).
