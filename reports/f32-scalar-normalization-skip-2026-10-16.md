# A guarded scalar f32 path reduces T1023 transform time by 1–4%

At GL T1023, a guarded scalar f32 recurrence reduced synthesis and analysis
time in all three paired process comparisons. The median reductions were 4.39%
and 4.49% with one thread, and 2.95% and 1.10% with eight threads. The
optimization is enabled only when `lmax == 1023`; a T720 control did not
regress.

## Method

The candidate skips repeated lane-by-lane normalization when all recurrence
pairs are within the existing normal range and their binary exponents are zero. After a normal pair, a Pulp SIMD check detects whether any lane needs scaling. The existing normalizer runs whenever the check finds an out-of-range lane or
the preceding pair required scaling. The selector uses a const-generic path so
other truncations retain the original per-degree normalizer without a runtime
gate in that loop. Scalar f32 truncations other than T1023 use the established
path; spin-1 and f64 recurrence code is unchanged. The benchmark compares binaries built from baseline commit
`2aea96bdb23b24b762dc9c83102773cd51d12a37` and the candidate source on
`opt/sht-kernel-performance`. Both use Rust `1.99.0 (b940084d7 2026-09-28)`,
LLVM `23.1.1`, the release profile, all features, and portable Pulp runtime
dispatch. The CPU is an x86-64 CPU with AVX2 and FMA and no AVX-512. Each fresh process was 8 pinned CPU cores. The host is shared and the cgroup
has no CPU quota. The benchmark measures native plan methods with a reused plan
and workspace, one warm-up, and five samples per operation. It records plan
construction, workspace allocation, elapsed time, and peak process RSS. The
timing includes no Python wrapper. The T1023 grid is GL `1024×2048`, full `mmax=1023`, and `phi0=0.37`. The T720
control is GL `721×1458`, full `mmax=720`, and the same longitude phase. Both
use deterministic f32 maps and complex64 coefficients. Three alternating
baseline/candidate process pairs were collected at each truncation. The
baseline and candidate binary hashes and raw logs are in the
[`benchmark data`](data/f32-scalar-normalization-skip-2026-10-16/).

## T1023 performance

Times are medians across the three baseline or candidate processes. Changes
are candidate relative to its paired baseline; negative values mean shorter
elapsed time.

| Operation | Threads | Baseline (ms) | Candidate (ms) | Median paired change | Paired range |
| --- | ---: | ---: | ---: | ---: | ---: |
| Synthesis | 1 | 3,927.244 | 3,752.387 | −4.39% | −4.76% to −4.32% |
| Synthesis | 8 | 674.379 | 658.349 | −2.95% | −3.80% to −1.67% |
| Analysis | 1 | 1,952.932 | 1,865.168 | −4.49% | −4.50% to −4.44% |
| Analysis | 8 | 361.217 | 357.578 | −1.10% | −1.72% to −0.88% |

All three pairs favored the candidate for each operation and thread count. The
median peak process RSS was 51,828 KiB for the baseline and 51,560 KiB for the
candidate. Workspace allocation and plan construction remained below 8 ms;
the recurrence adds no workspace storage.

## T720 control and rejected selector

The final candidate disables the skip at T720. Its paired changes were
−1.04%/−0.36% for synthesis at one/eight threads and −0.36%/−0.23% for analysis. Each range favored the candidate, but these small control changes do not
measure a benefit from the skip path. An intermediate build used a per-degree runtime gate to disable the skip below
T1023. That build regressed T720 q8 synthesis by 3.20% and q8 analysis by
2.63% across three pairs. The gate was moved outside the recurrence and
specialized at compile time. The final candidate uses the original normalizer
for T720. Both intermediate and final measurements remain in the data
directory; only the final binary supports the reported T1023 result.

## Numerical comparison

The release test compares scalar and spin-1 STANDARD synthesis and analysis
with `ducc0==0.41.0`, source revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. It uses the T1023 GL grid above and
one thread. The candidate passed the selected-value, sum, and norm checks in
the test. Six complete RustSHT output arrays are bitwise identical to the
baseline: scalar synthesis and analysis, plus both spin-1 synthesis and
analysis channels. Full-array relative L2 errors against DUCC are `4.87e-5` for scalar synthesis,
`1.67e-5` for scalar analysis, `3.84e-5` and `4.19e-5` for spin-1 synthesis,
and `2.07e-5` and `2.56e-5` for spin-1 analysis. RustSHT has no established
T1023 f32 acceptance threshold, so these values characterize this comparison
without setting one. The SIMD range-check unit test covers vector lanes and a
scalar tail, zero values, the `SMALL` and `BIG` boundaries, values outside
those boundaries, NaNs, infinity, and nonzero exponents.

## Validation

The following checks passed with the final source:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`: 188 unit tests and 11 f32 tests
  passed; 13 unit tests and one f32 test were ignored.
- `cargo test -p rustsht --no-default-features`: 150 unit tests and 11 f32
  tests passed; seven unit tests and one f32 test were ignored.
- The opt-in T1023 f32 comparison with DUCC 0.41.0 passed. Python tests were not run because the Python bindings did not change. The exact
commands, test logs, binary hashes, pair summaries, and six-array comparisons
are under [`reports/data/f32-scalar-normalization-skip-2026-10-16`](data/f32-scalar-normalization-skip-2026-10-16/).
