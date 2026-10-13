# Real forward radix-two AVX2 measurements

The real forward multipass now has an AVX2 interior for the radix-two stage
used by lengths whose factorization has an odd power of two. It processes two
adjacent complex values per f64 vector and four per f32 vector. The scalar loop
handles the stage endpoints and vector tail. Runtime AVX2 detection selects
the kernel for `ido >= 16`; other CPUs use the scalar equations. The kernel
adds no workspace or scratch allocation. The pass follows DUCC's `rfftp2` ordering and stage twiddle layout from pinned
revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The mirrored output branch
is stored in reverse complex-pair order so that both output branches are
written contiguously. Tests compare the vector body and tail with the scalar
equations for both precisions. The crate's real transform tests exercise the
public API at the selected lengths and compare against independent DFTs.

## f64 paired comparison

Baseline source was `ab2bf32`; candidate source was `5e05008`. Two six-pair
runs used the existing `rustfft.rs` benchmark. The first ran all smooth lengths
in one process; the second ran each length separately. Pair order alternated,
all processes were pinned to logical one CPU core on an x86-64 CPU, and plans
were warmed before seven timed samples. Timings measure the public R2C or
C2R call with reusable workspaces. The batch column is per transform for a
public call over 32 rows. R2C inputs use the harness's deterministic real
sequence; C2R uses its deterministic Hermitian spectrum. The table reports each run separately to expose host variation. Report Run 1
maps to `run2` in the raw CSV, and Run 2 maps to `run3`. Each timing cell is
the median of six process-level medians in microseconds, baseline to candidate. The parenthesized gain is the median of the six matched
`baseline/candidate - 1` gains; the count gives pairs with a positive gain. Timing medians and paired gains are aggregated separately. Positive gains mean
lower candidate time.

| N | Radix-two stage `ido` | Run 1 R2C single | Run 1 R2C batch 32 | Run 2 R2C single | Run 2 R2C batch 32 |
|---:|---:|---:|---:|---:|---:|
| 64 | — | 0.1905 → 0.1905 (0.00%; 1/6) | 0.1602 → 0.1647 (−2.94%; 0/6) | 0.1905 → 0.1900 (+0.26%; 3/6) | 0.1598 → 0.1611 (−0.59%; 2/6) |
| 128 | 64 | 0.4060 → 0.3610 (+11.08%; 6/6) | 0.3782 → 0.3402 (+11.49%; 6/6) | 0.4010 → 0.3610 (+10.94%; 4/6) | 0.3821 → 0.3412 (+11.72%; 4/6) |
| 256 | — | 0.7010 → 0.7120 (−1.48%; 1/6) | 0.6749 → 0.6940 (−2.94%; 0/6) | 0.8620 → 0.8665 (+0.58%; 3/6) | 0.7570 → 0.6875 (+5.94%; 4/6) |
| 512 | 256 | 1.6230 → 1.4575 (+11.36%; 6/6) | 1.6062 → 1.4421 (+11.15%; 6/6) | 1.6630 → 1.4720 (+13.15%; 6/6) | 1.6179 → 1.4304 (+12.85%; 6/6) |
| 1,024 | — | 2.9760 → 3.0660 (−3.29%; 0/6) | 2.9543 → 3.0572 (−3.19%; 0/6) | 3.0560 → 3.0810 (−0.65%; 1/6) | 2.9467 → 3.0398 (−2.96%; 0/6) |
| 2,048 | 1,024 | 7.0780 → 6.5470 (+7.95%; 6/6) | 7.1043 → 6.5666 (+8.11%; 6/6) | 7.1890 → 6.5625 (+9.39%; 6/6) | 7.1551 → 6.5483 (+9.03%; 6/6) |
| 4,096 | — | 13.5400 → 13.9215 (−3.35%; 0/6) | 13.6802 → 14.0687 (−3.08%; 0/6) | 13.3205 → 13.7310 (−3.24%; 0/6) | 13.4673 → 13.9038 (−3.14%; 0/6) |

Both runs show repeatable R2C gains at 128, 512, and 2,048, the lengths that
execute this radix-two stage. At 512 and 2,048 every pair favored the
candidate. The 128-point single-transform result has more pair variation in
the second run, while its batch result remains close to an 11% gain. The
no-radix-two controls do not show a repeatable gain; R2C at 1,024 and 4,096
was 0.6–3.4% slower in these samples. C2R was not changed by this kernel and
serves as a control; its paired changes were mixed and mostly within 3.1%. The control regressions remain open for follow-up. The Legendre workload was
active on the host, so these measurements are provisional. The f64 raw file contains 672 rows: 12 process pairs across seven lengths,
forward and inverse operations, and single and batch-32 calls. It records
process order, timing, workspace size, and checksums. Checksums and workspace
sizes match between baseline and candidate. The candidate adds one optional
kernel function pointer to the immutable real multipass plan; execution
workspace and scratch sizes are unchanged.

## f32 paired comparison

The existing `f32_real_simd.rs` harness compares the public operations with a
RustFFT 6.4.1 half-length complex adapter. It checks the final R2C and C2R
outputs against that adapter with relative-L2 and L2-scaled maximum-error
limits of `2e-5`. Twelve baseline/candidate process pairs were pinned to the
same CPU; each process used seven alternating RustFFT/DUCCFFT samples. The
table shows median times in microseconds per transform, the displayed
DUCCFFT/RustFFT ratio before and after this change, the median paired gain,
and candidate-favored pair count. Each timing cell is the median of 12
process-level medians, with seven in-process samples per process. Each paired
gain is the median of `baseline/candidate - 1` across the 12 matching process
pairs. Displayed DUCCFFT/RustFFT ratios divide the corresponding unrounded
DUCCFFT and RustFFT timing medians.

| N | Radix-two stage `ido` | R2C single: DUCCFFT baseline → candidate | RustFFT ratio baseline → candidate | Paired gain | R2C batch 32: DUCCFFT baseline → candidate | RustFFT ratio baseline → candidate | Paired gain |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 64 | — | 0.151 → 0.152 | 1.298× → 1.323× | −1.12% (1/12) | 0.151 → 0.154 | 1.294× → 1.339× | −1.66% (0/12) |
| 128 | 64 | 0.359 → 0.309 | 1.624× → 1.410× | +16.24% (12/12) | 0.359 → 0.312 | 1.621× → 1.414× | +15.20% (12/12) |
| 256 | — | 0.613 → 0.618 | 1.388× → 1.418× | −0.57% (1/12) | 0.609 → 0.617 | 1.379× → 1.406× | −1.09% (1/12) |
| 512 | 256 | 1.481 → 1.258 | 1.671× → 1.434× | +17.68% (12/12) | 1.483 → 1.263 | 1.673× → 1.430× | +17.54% (12/12) |
| 1,024 | — | 2.531 → 2.542 | 1.382× → 1.391× | −0.54% (1/12) | 2.535 → 2.552 | 1.379× → 1.390× | −0.72% (3/12) |
| 2,048 | 1,024 | 6.167 → 5.264 | 1.632× → 1.415× | +16.99% (12/12) | 6.189 → 5.296 | 1.628× → 1.408× | +17.12% (12/12) |
| 4,096 | — | 10.907 → 10.968 | 1.408× → 1.432× | −0.48% (3/12) | 10.963 → 11.038 | 1.402× → 1.432× | −0.75% (2/12) |

The f32 results show gains of 15–18% at all three radix-two lengths, with all
12 pairs favoring the candidate. The no-radix-two R2C controls are within
1.7% of baseline. C2R remains an unchanged control. The benchmark completed
all output comparisons; the broader f32 direct-DFT error metrics and extreme
value cases remain in [`m4-real-multipass.md`](m4-real-multipass.md).

## Build and machine-code details

Both precision builds used Rust 1.99.0 (`b940084d7`, LLVM 23.1.1), Cargo
1.99.0, the default optimized `bench` profile (`-C opt-level=3`,
`-C embed-bitcode=no`, `-C strip=debuginfo`), RustFFT 6.4.1, no custom
`RUSTFLAGS`, no LTO, and no `target-cpu` setting. AVX2 was selected at runtime. The f64 harness source SHA-256 is
`6a85531e042175af96e9bcdc07368c15908e46212b7d441d68c57cfb7b9b4cba`; the f32
harness source SHA-256 is
`76ea65520ac18ab604087c9011c81f61b5f54b597dbfb3b45ad7cf9320c2b665`. The
crate lockfile SHA-256 is
`e61d5628d09dc7c10e54e15cb5f3aeb5dd5c28392b44692ed153784ad8ca21ba`. The Rust benchmark binary SHA-256 values are `6a71ca04abbbec9e6ca85d8c3649870be01a232b41d94d112f9bc81a857849a6`
and `59d1fff792235d376b116f7a88241e80f9c7bfabaec51041f74247119f9b6275`
for f64 baseline and candidate, and
`6ab9b21688bb108b93785de4a646ca716c64162919d2f52351996dada8fd95df` and
`8a263ab39c38e6682f60def2e1d0a58bd49538546775187aec31a395d6f5d2f5` for f32. Release disassembly of both forward kernels contains packed AVX2 operations. The f64 body uses YMM `vmovupd`, `vmulpd`, `vaddpd`, `vsubpd`, `vblendpd`, and
`vpermpd`; the f32 body uses `vmovups`, `vmulps`, `vaddps`, `vsubps`,
`vblendps`, and packed shuffles. Each vector loop has one back-edge branch,
with no helper calls or stack-local spills. Scalar tails remain outside the
vector loop. The disassembly confirms packed operations but does not explain
the small no-radix-two timing regressions. The previous f64 cross-library tables in
[`m4-real-multipass.md`](m4-real-multipass.md) and
[`m4-real-radix2-avx2.md`](m4-real-radix2-avx2.md) predate this forward kernel. This report does not present them as candidate measurements. The f32 table
includes its RustFFT adapter control; a new direct C++ DUCC comparison was
not run for this incremental kernel. Raw paired samples are in
[`m4-real-radix2-forward-avx2-paired-process-samples.csv`](../benches/results/m4-real-radix2-forward-avx2-paired-process-samples.csv)
and
[`m4-real-radix2-forward-avx2-f32-paired-process-samples.csv`](../benches/results/m4-real-radix2-forward-avx2-f32-paired-process-samples.csv).
