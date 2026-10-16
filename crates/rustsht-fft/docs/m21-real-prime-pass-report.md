# M21 real general-prime pass reuse

Date: 2026-10-16. Baseline: M20 commit
`343e5adc08e44cc5472032138dfb263bb34b4786`. The M21 benchmark executable
SHA-256 is
`b88e1fa3b418ea1fdba8343dd53101cd2a09dc4b228ba066fa282bae52f89ec8`.

## Method

The forward general-prime pass applied a branch twiddle again for every
positive output frequency. The inverse pass re-read and reconstructed the same
Hermitian input frequencies for every output sample. Both passes now cache
those values in a stack array for each inner-frequency lane and reuse them
across the remaining output loop. The radix equations, twiddle order,
normalization and fallback paths are unchanged. The plan and workspace layouts
do not change. The candidate uses the general-prime routes for N=1,443 (`radix=13, ido=37`)
and N=4,097 (`radix=17, ido=241`). Route selection is recorded in the
[M19 route trace](../benches/results/m19-current-head-cross-library/m19-route-memory-diagnostics.txt). The N=4,097 Rader-241 convolution and its Bluestein fallback are unchanged. The M20 and M21 executables used Rust 1.99.0, the optimized bench profile,
portable target flags, runtime SIMD dispatch and one transform thread. Each
process was pinned to logical one CPU core on an x86-64 CPU. The target
comparison used six alternating process pairs and seven warmed samples per
operation. It included all SHT-shaped C2C and real lengths with their scalar
and spin-1 row counts. Load-average telemetry was retained only in local diagnostics. The regression-control comparison
used the same process and sampling method for one and 32 rows. The host policy
observed that day was `amd-pstate-epp`, with the performance governor and EPP
and a CPU-frequency telemetry omitted range. This is host context, not a per-sample frequency
measurement. The table reports median microseconds per real transform. Ratios are M20 time
divided by M21 time, so values above one favor M21. Intervals are 95%
bootstrap intervals over the six paired process ratios.

| Operation | N | Rows | M20 (µs) | M21 (µs) | M20/M21 (95% interval) |
| --- | ---: | ---: | ---: | ---: | ---: |
| R2C | 257 | 128 | 2.230 | 2.141 | 1.048 (1.032–1.062) |
| C2R | 257 | 128 | 2.465 | 2.386 | 1.037 (1.007–1.046) |
| R2C | 723 | 361 | 11.011 | 10.961 | 1.008 (0.984–1.017) |
| C2R | 723 | 361 | 11.836 | 11.800 | 1.005 (1.000–1.028) |
| R2C | 1,025 | 512 | 6.052 | 6.002 | 1.010 (0.992–1.022) |
| C2R | 1,025 | 512 | 7.281 | 7.252 | 1.003 (0.998–1.009) |
| R2C | 1,443 | 721 | 18.130 | 14.798 | 1.218 (1.214–1.235) |
| C2R | 1,443 | 721 | 22.030 | 17.975 | 1.227 (1.221–1.233) |
| R2C | 2,049 | 1,024 | 32.255 | 31.707 | 1.017 (1.010–1.024) |
| C2R | 2,049 | 1,024 | 34.318 | 35.248 | 0.972 (0.966–0.982) |
| R2C | 4,097 | 2,048 | 101.759 | 89.904 | 1.128 (1.121–1.143) |
| C2R | 4,097 | 2,048 | 116.938 | 101.355 | 1.153 (1.137–1.166) |

N=1,443 improved by 21.8–22.7% in both directions. N=4,097 improved by
12.8% for R2C and 15.3% for C2R. N=723 remained at parity. N=2,049 R2C
improved by 1.7%; C2R was 2.9% slower. The paired M21/M20 elapsed-time ratio
was 1.029 (95% interval 1.018–1.035). The median remains inside the 5%
regression limit used in the M7 Rader acceptance test; the result is reported
as a control regression. A separate six-pair N=2,049 run measured R2C at 1.1% faster and C2R at 4.0%
slower. The paired M21/M20 elapsed-time ratio for C2R was 1.040 (95% interval
1.021–1.056), corresponding to a 2.1–5.6% slowdown. The interval includes
slowdowns above 5%; the target matrix and the isolated follow-up therefore
agree on the direction of this control result, while the follow-up leaves its
upper bound uncertain. N=2,049 workspace size was 159,856 bytes in both builds. All 34 target
operation/length workspace counts matched. Plan and workspace construction
medians were within 1.5% for the affected sizes; the candidate adds no retained
plan payload or caller-owned scratch. At N=4,097, plan construction measured
93.63 µs for M20 and 93.67 µs for M21; workspace construction measured 1.82
and 1.82 µs, respectively. The separate control matrix covered C2C N=682, smooth real N=1,024 and real
N=241 at one and 32 rows. Median slowdowns were at most 1.4% in these cells;
the largest was single-row R2C at N=1,024. N=682 C2C inverse at 32 rows was
0.9% slower. These measurements do not show a change in the complex routes. The complete target matrix measured N=257 R2C and C2R 3.7–4.8% faster.

## Numerical checks

The release-mode `rader_4097_matches_independent_real_transform_references`
test passed against direct DFTs and RustFFT 6.4.1. The
`real_general_prime_and_bluestein_paths_match_independent_dfts` test passed
after adding N=1,443 to its f64 cases. The f32
`f32_large_smooth_prime_bluestein_and_sht_lengths_match_f64_references` test
also passed; its direct f64 reference cases include N=1,443 and N=4,097. These checks cover the tested inputs and lengths; M22 measures the f32
performance effect separately. `cargo fmt --all -- --check` and
`cargo clippy --all-targets --all-features -- -D warnings` passed. The full
all-features test suite and broader release numerical matrix are scheduled for
M23 closeout.

## Supporting data

- [Six-pair SHT-target capture](../benches/results/m21-prime-pass-reuse/m21-real-prime-pass-six-pairs.jsonl),
  [summary](../benches/results/m21-prime-pass-reuse/m21-real-prime-pass-summary.csv)
  and [configuration](../benches/results/m21-prime-pass-reuse/m21-real-prime-pass-config.json).
- [Six-pair regression controls](../benches/results/m21-prime-pass-reuse/m21-regression-controls-six-pairs.jsonl),
  [summary](../benches/results/m21-prime-pass-reuse/m21-regression-controls-summary.csv)
  and [configuration](../benches/results/m21-prime-pass-reuse/m21-regression-controls-config.json).
- [Six-pair N=2,049 confirmation](../benches/results/m21-prime-pass-reuse/m21-n2049-control-six-pairs.jsonl),
  [summary](../benches/results/m21-prime-pass-reuse/m21-n2049-control-summary.csv)
  and [configuration](../benches/results/m21-prime-pass-reuse/m21-n2049-control-config.json).
