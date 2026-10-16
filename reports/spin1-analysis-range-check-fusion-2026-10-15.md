# Fusing spin-1 range checks improves q1 analysis; q8 remains near neutral

The f64 fused Q/U scalar-conversion kernel now checks each newly computed
recurrence value before using it at the next degree. This removes the separate
range-check pass that reread both recurrence buffers after each update. A
failure declines the fused block; the caller commits staged coefficients only
on success and otherwise reruns the established scaled recurrence. The
recurrence and channel-accumulation order are unchanged. f32 and workspace
storage are unchanged.

## Paired RustSHT results

Five alternating release-process pairs compared the candidate with commit
`541ac1b` at T511 and T1023, using f64 spin-1 GL STANDARD analysis and the
reused-plan/workspace native benchmark. Each process ran one warmup and five
timed calls at full `mmax`. q1 was pinned to one CPU core; q8 used 8 pinned CPU cores. Since the
first q8 ranges crossed zero, five additional q8 pairs measured both sizes. The shared x86-64 CPU host allowed 32 pinned CPU cores and had no CPU quota; the
CPUs were not isolated. Rust was `1.99.0`, LLVM `23.1.1`, and Pulp used
portable runtime dispatch (AVX2/FMA on this host) without `target-cpu=native`. Negative paired changes favor the candidate. q1 entries summarize five pairs;
q8 entries summarize ten pairs. Process medians and paired changes are listed
separately.

| Grid | Threads | Baseline median | Candidate median | Paired change median (range) | Candidate faster |
| --- | ---: | ---: | ---: | ---: | ---: |
| T511 | q1 | 32.232 ms | 31.216 ms | -3.24% (-3.55% to -1.70%) | 5/5 |
| T511 | q8 | 5.952 ms | 5.908 ms | -0.11% (-2.31% to +4.53%) | 5/10 |
| T1023 | q1 | 251.721 ms | 245.349 ms | -2.60% (-3.34% to -1.74%) | 5/5 |
| T1023 | q8 | 43.121 ms | 42.632 ms | -0.88% (-16.49% to +1.65%) | 7/10 |

The q1 improvement repeated at both sizes. The pooled q8 result is near
neutral at T511 and slightly faster at T1023, with ranges that still cross
zero. The process peak RSS medians are within 52 KiB of baseline across these
cases; no workspace arrays were added. Initial and repeated samples are in
[`paired-results.tsv`](data/spin1-analysis-range-check-fusion-screen-2026-10-15/paired-results.tsv)
and [`q8-results.tsv`](data/spin1-analysis-range-check-fusion-screen-2026-10-15/q8-results.tsv).

## Current T1023 comparison with DUCC

Five alternating native RustSHT/Python DUCC process pairs measured full-band
T1023 GL STANDARD spin-1 analysis. RustSHT used the native reused-plan
benchmark; DUCC used `analysis_2d` through `ducc0==0.41.0`, source revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. q1 used one CPU core and q8 used 8 pinned CPU cores for both implementations. These are shared-host results, not isolated
core measurements.

| Threads | RustSHT median (range) | DUCC median (range) | Paired ratio median (range) |
| ---: | ---: | ---: | ---: |
| q1 | 242.153 ms (241.337–242.790) | 258.569 ms (256.199–298.459) | 0.94x (0.81–0.95x) |
| q8 | 42.237 ms (41.869–42.589) | 37.015 ms (36.900–38.270) | 1.14x (1.10–1.15x) |

One DUCC q1 process contained a much slower inner run, widening the q1 ratio
range. The q8 comparison is narrower. The ratio describes the candidate's
current timing against this DUCC run; it does not isolate the code change's
effect on the ratio. Raw commands, logs, process resource data, versions and
binary hash are in
[`the DUCC comparison data`](data/spin1-analysis-range-check-fusion-screen-2026-10-15/ducc-comparison/).

## Validation

The focused fused two-channel tests passed, including a regression where an
out-of-range value occurs in the first vector, a later vector, or the scalar
tail after coefficients have been staged. The caller does not commit those
staged values when the block declines. ```text
cargo test --workspace --all-features
  187 unit tests passed; 12 ignored
  11 f32 integration tests passed; doctests passed (0 tests)

cargo test -p rustsht --no-default-features
  149 unit tests passed; 7 ignored
  11 f32 integration tests passed; doctests passed (0 tests)

cargo clippy --workspace --all-targets --all-features -- -D warnings
  passed

cargo fmt --all -- --check
  passed

cargo test --release -p rustsht m3_spin_t511_all_grids_matches_ducc_and_roundtrips -- --ignored --nocapture
  passed for spins 1 and 2 on GL, CC and F1

cargo test --release -p rustsht m3_spin_t1023_exact_analysis_matches_ducc -- --ignored --nocapture
  passed for spins 1 and 2 on GL, CC and F1
```

The pinned DUCC fixtures use `ducc0==0.41.0` and source revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Python bindings did not change,
so `uv run pytest python/tests` was not run.
