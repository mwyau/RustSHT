# Scalar order-owned synthesis experiment (M4, 2026-10-15)

The scalar order-owned synthesis prototype did not improve whole-transform time and was removed. Across five paired process runs, the T511 q8 candidate was 32.65% slower at the median; T1023 q8 was 1.87% slower. Both q1 controls also ran slower. The results do not meet the M4 performance gate.

## Method

The candidate parallelized scalar f64 Gauss–Legendre synthesis by harmonic order for exact grids at T511 through T1023. Each worker prepared recurrence and coefficient factors for its order range, wrote into an order-major view of the existing `workspace.spin_spectra` allocation, and transposed those values into the Fourier-row layout. Other shapes and q1 execution used the ring-owned path. The baseline was RustSHT commit `8ccc0d5` (`perf: parallelize analysis row FFTs`). Both executables used `cargo build --release --example m1b_bench` with rustc `1.99.0` and LLVM `23.1.1`. Baseline SHA-256: `4b28381e93b65567cf5f1761a0f70c29f4c737d05bf54641c5fe54f2708de717`. Candidate SHA-256: `e5c72dc27a28c6c784735e176dc1a07ae3035920c35d985eb547cfd671ca0687`. The benchmark measured f64 GL STANDARD scalar synthesis with `ntheta=lmax+1`, `nphi=2*lmax+3`, full `mmax`, and `phi0=0.37`. Each process performed one warmup and five timed calls. Five baseline/candidate process pairs alternated execution order for q1 and q8 at T511 and T1023. q1 processes used one CPU core; q8 processes used 8 pinned CPU cores. The host-wide benchmark lock was held. `RUSTSHT_LEGENDRE_REUSE_ESTIMATE=-,0` disabled reuse-estimate effects. The host was an x86-64 CPU with host topology omitted. The build used Pulp runtime dispatch; the benchmark did not log the selected ISA. Load-average telemetry was retained only in local diagnostics. Process snapshots showed no unrelated CPU-heavy job. The nonzero load makes these screening results provisional; the paired direction was consistent across all five pairs in every case.

## Results

Each baseline and candidate value is the median of five process medians. Paired change is the median candidate-to-baseline ratio; the range shows the five individual paired changes.

| Degree | Threads | Baseline (ms) | Candidate (ms) | Candidate change |
| --- | ---: | ---: | ---: | ---: |
| T511 | q1 | 42.852 | 43.847 | +2.91% (+0.56% to +3.21%) |
| T511 | q8 | 11.901 | 15.788 | +32.65% (+31.43% to +33.97%) |
| T1023 | q1 | 326.079 | 335.247 | +2.07% (+1.54% to +4.12%) |
| T1023 | q8 | 107.469 | 109.390 | +1.87% (+0.81% to +2.15%) |

The q8 measurements showed no benefit at either tested degree. Equal order counts also give workers unequal recurrence work because an order `m` includes `lmax-m+1` degrees. That imbalance may explain part of the q8 result; the benchmark did not time recurrence and layout conversion separately. The prototype reused existing workspace storage and added no workspace array. Peak process RSS was not measured. The timing benchmark compared the candidate with RustSHT `8ccc0d5`, not with DUCC.

## Numerical checks

The release comparison covered T511, T767, and T1023, q8 or q5, odd `nphi=2*lmax+3`, even Nyquist `nphi=2*lmax`, and nonzero `phi0`. Sparse coefficients included `m=0`, `m=lmax`, worker-boundary orders, and degree endpoints. Candidate maps matched q1 maps within `5e-12`; all samples were finite. The q5 T767 case exercised an uneven order split. The T1023 scaled fallback test compared order-major and ring-major destinations for subnormal and maximum finite coefficients. These candidate-specific checks compared with the same RustSHT implementation at q1; they were not independent DUCC comparisons. The workspace suite also passed its existing DUCC 0.41.0, analytic-field, and adjoint checks. Formatting and Clippy passed. `cargo test --workspace --all-features` passed 183 Rust tests and 10 f32 integration tests, with 12 ignored. `cargo test -p rustsht --no-default-features` passed 144 Rust tests and 10 f32 integration tests, with 7 ignored. The prototype and its candidate-specific tests were removed after the timing comparison. Raw process output and the paired runner are in [`data/order-owned-synthesis-2026-10-15/`](data/order-owned-synthesis-2026-10-15/).
