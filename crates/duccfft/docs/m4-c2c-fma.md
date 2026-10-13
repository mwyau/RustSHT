# M4 f64 C2C FMA experiment

## Change and measurement

Commit `9ea5337f62f4628c6c34b19673046c56b74973d2` adds AVX2+FMA variants of
the f64 complex pass loops. Runtime dispatch selects them only when both AVX2
and FMA are available. Other x86 CPUs retain the existing AVX2 path, and
non-x86 targets retain the portable path. The FMA kernel combines the complex
multiply's multiply/add-sub sequence with `_mm256_fmaddsub_pd`. It does not
change the factorization, pass schedule, scaling, or workspace. The direct
length-1,024 f64 DFT regression checks both directions after this change.

The baseline is `418ec1bb5c7253a9e2dca617d67e4126a74e7be3`, immediately before
the FMA change. The identical benchmark source is Git blob
`3bdbb3d7f441816bf2f2475e45a9f5eb92664cdb` (SHA-256
`6a85531e042175af96e9bcdc07368c15908e46212b7d441d68c57cfb7b9b4cba`). Both
builds use `crates/duccfft/Cargo.lock` (SHA-256
`e61d5628d09dc7c10e54e15cb5f3aeb5dd5c28392b44692ed153784ad8ca21ba`), Rust
1.99.0 (`b940084d7`, LLVM 23.1.1), and Cargo's default `bench` profile at
`opt-level=3`. `RUSTFLAGS`, LTO, and target CPU flags were unset. The measured
binary SHA-256 hashes are baseline
`32e9dd95adce0672853ce387d6b61595622f2cc26881ecb19e923a13300df897` and
candidate `507c80e991eb227978a6ee8fa235f174d99cf27583b76aa28882ad1c19045efa`.

The host was an AMD Ryzen 9 5950X with AVX2 and FMA. Processes were pinned to
logical CPU 30, on a different physical core from the active Legendre process.
The recorded one-minute load averages around the sweeps were 4.51–5.27, so the
measurements are provisional. Twelve baseline/candidate process pairs per
length alternated execution order. Each process warmed the benchmark once and
reported the median of seven operations. Plan and workspace setup used 20
repetitions outside the timed operation. Inputs and reusable plans/workspaces
were the same in both builds.

`batch_count=32` is 32 sequential public calls using one plan and workspace;
it is not `forward_batch` or `inverse_batch`. Each table time is the median of
12 per-process medians in microseconds. The gain is the median of the 12 paired
ratios `(baseline / FMA) - 1`; positive values favor FMA. The final benchmark
checksum is the harness's first-output-value checksum, not a full-array hash.
It matched exactly in all 672 paired records. The independent DFT tests compare
the full length-1,024 output. Raw samples, including R2C/C2R controls and setup
measurements, are in
[`m4-c2c-fma-process-samples.csv`](../benches/results/m4-c2c-fma-process-samples.csv).

| n | C2C forward, one call (baseline → FMA; gain; pairs) | Forward, sequential 32 | C2C inverse, one call | Inverse, sequential 32 |
|---:|---:|---:|---:|---:|
| 64 | 0.235 → 0.231 (+2.04%; 6/12) | 0.215 → 0.206 (+4.18%; 8/12) | 0.310 → 0.311 (−0.32%; 4/12) | 0.290 → 0.289 (+0.27%; 7/12) |
| 128 | 0.431 → 0.421 (+0.00%; 5/12) | 0.403 → 0.402 (+0.12%; 6/12) | 0.611 → 0.601 (+1.66%; 8/12) | 0.589 → 0.586 (+0.51%; 8/12) |
| 256 | 0.771 → 0.751 (+2.01%; 8/12) | 0.752 → 0.746 (+1.36%; 8/12) | 1.132 → 1.122 (+0.89%; 8/12) | 1.109 → 1.099 (+0.93%; 7/12) |
| 512 | 1.653 → 1.639 (+0.64%; 7/12) | 1.674 → 1.656 (+0.61%; 7/12) | 2.410 → 2.380 (+0.63%; 6/12) | 2.391 → 2.392 (−0.42%; 5/12) |
| 1,024 | 3.938 → 3.873 (+1.41%; 9/12) | 3.892 → 3.843 (+1.57%; 10/12) | 5.426 → 5.360 (+1.30%; 9/12) | 5.401 → 5.355 (+0.92%; 8/12) |
| 2,048 | 11.757 → 11.737 (+0.62%; 6/12) | 8.273 → 8.179 (+0.13%; 6/12) | 14.713 → 14.512 (+0.20%; 6/12) | 14.505 → 14.506 (−1.04%; 4/12) |
| 4,096 | 26.986 → 27.703 (−2.06%; 1/12) | 19.689 → 20.162 (−2.05%; 4/12) | 32.917 → 33.544 (−1.85%; 3/12) | 32.786 → 33.395 (−1.55%; 4/12) |

The geometric mean of the 84 paired forward ratios favors FMA by 1.12% for one
call and 0.66% for sequential batches. The inverse geometric mean is 0.11%
slower for one call and 0.16% faster for sequential batches. The 4,096-point
case regresses by 1.55–2.06% in all four measurements. These changes are too
small and length-dependent to close the existing C2C gap to RustFFT.

Setup medians use only `c2c_forward` records with `batch_count=1`:

| n | Plan, baseline → FMA | Workspace, baseline → FMA | Scratch bytes |
|---:|---:|---:|---:|
| 64 | 1.747 → 1.559 µs | 0.161 → 0.161 µs | 1,280 |
| 128 | 1.847 → 1.849 µs | 0.077 → 0.078 µs | 2,304 |
| 256 | 3.520 → 3.524 µs | 0.088 → 0.088 µs | 4,352 |
| 512 | 6.846 → 6.882 µs | 0.122 → 0.120 µs | 8,448 |
| 1,024 | 14.206 → 14.154 µs | 0.187 → 0.186 µs | 16,640 |
| 2,048 | 27.181 → 27.377 µs | 0.318 → 0.321 µs | 33,024 |
| 4,096 | 54.523 → 54.897 µs | 0.583 → 0.588 µs | 65,792 |

## Generated code and decision

The candidate binary contains `vfmaddsub` instructions in the FMA pass path.
The optimized benchmark binary's text grew from 1,621,217 to 1,626,313 bytes.
The baseline `f64_kernel::pass` has a 200-byte stack frame. The FMA pass has
the same frame size; its code size is 2,696 bytes, compared with 2,571 bytes
for the baseline pass. This confirms FMA code generation but does not explain
the whole-operation regression at length 4,096.

The candidate passes
`cargo test --manifest-path crates/duccfft/Cargo.toml --all-features` (5 unit,
21 complex, 26 real tests) and
`cargo clippy --manifest-path crates/duccfft/Cargo.toml --all-targets --all-features -- -D warnings`.
The current evidence supports retaining the experiment only until the R22
read-only review assesses the numerical path, runtime dispatch, and inconsistent
timing results. The benchmark used a DUCCFFT-only harness; no fresh RustFFT or
direct C++ DUCC timings were collected for this source-to-source experiment.
