# f32 spin-1 analysis SIMD overflow fallback

## Finding and change

The SIMD eligibility checks accepted normal finite Fourier inputs and recurrence bases but did not prevent a SIMD lane's running sum from overflowing. With 16 active lanes alternating `+f32::MAX` and `-f32::MAX`, `plus == minus == 0.5` and `alpha_norm == 1`, scalar accumulation cancels adjacent values to zero. AVX2 accumulates same-sign values in each lane across vector blocks, producing nonfinite partial sums. The accumulator now checks its four reduced output components after SIMD execution. It returns finite results directly; if any component is infinite or NaN, it recomputes the coefficient in the scalar accumulation order. The normal path does not perform another lane scan, allocate workspace, or change the SIMD reduction. The regression test exercises the private helper boundary; a public-transform reproducer has not been established.

## Validation

The focused regression, all-feature workspace suite and no-default-feature Rust suite pass. The T511 f32 spin-1 GL analysis integration test compares selected coefficients with `ducc0==0.41.0` and passes in both suites. ```text
cargo test -p rustsht --all-features f32_spin_analysis_falls_back_when_simd_lane_partials_can_overflow -- --nocapture
  1 passed

TMPDIR=<temporary-path> cargo test --workspace --all-features
  185 unit tests passed; 12 ignored
  11 f32 integration tests passed
  doctests passed (0 tests)

TMPDIR=<temporary-path> cargo test -p rustsht --no-default-features
  147 unit tests passed; 7 ignored
  11 f32 integration tests passed
  doctests passed (0 tests)

TMPDIR=<temporary-path> cargo clippy --workspace --all-targets --all-features -- -D warnings
  passed

cargo fmt --all -- --check
  passed
```

The temporary-directory override was required because `/tmp` was a full tmpfs. `/var/tmp` had available space. No files were removed from `/tmp`.

## Performance screen

Five alternating release-process pairs compared the candidate with baseline commit `a763ad84aaf60a14933bba2b58fea2876c9684d8`. Each process ran the existing `examples/m4_f32_bench.rs`: spin-1 f32, GL T511 (`512 x 1025`), one warmup and five measured samples for q1 and q8 synthesis and analysis. RustSHT used rustc `1.99.0`, LLVM `23.1.1`, release defaults and runtime AVX2 dispatch on an x86-64 CPU. The binaries used portable build flags, without `target-cpu=native`.

| Operation | Threads | Baseline median | Candidate median | Median paired change | Paired change range |
| --- | ---: | ---: | ---: | ---: | ---: |
| Analysis | 1 | 845.843 ms | 827.700 ms | -2.30% | -2.61% to -1.05% |
| Analysis | 8 | 163.146 ms | 158.221 ms | -2.25% | -4.52% to +2.78% |
| Synthesis control | 1 | 1004.082 ms | 1004.271 ms | +0.02% | -0.77% to +0.16% |
| Synthesis control | 8 | 149.624 ms | 149.976 ms | +0.58% | -1.96% to +2.16% |

The process affinity allowed 32 pinned CPU cores. Load-average telemetry was retained only in local diagnostics. These shared-host samples show no repeatable regression; the analysis differences are not claimed as a speedup. RustSHT estimated 12.13 MiB for core numeric arrays in both binaries. Process peak RSS was not collected. This screen did not time DUCC. Load-average telemetry was retained only in local diagnostics. The candidate binary hash was `deaa2f1218ebe6f8ffc2b490b2fcb2c406bec7d3ad499b83e8a01776d8780935`; the baseline binary hash was `5b216a77aa03fa694e39f79e103f448ddfbac6461a4910b89ac73ebd12dc57d9`.
