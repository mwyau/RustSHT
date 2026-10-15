# Fused scalar recurrence and accumulation for analysis

## Kernel change

The f64 scalar paired-analysis path now advances the Legendre recurrence and accumulates each degree pair in the same SIMD dispatch. The previous path dispatched the recurrence, called a visitor for each pair, and dispatched the reducer separately. The fused kernel writes a latitude block's coefficient contributions into existing worker scratch and commits them only if the complete block remains in the normal numeric range. If the range check or recurrence requires scaling, it discards the staged values and runs the established scaled paired recurrence. The change adds no workspace allocation; f32 retains its prior path. The fused path is selected at `lmax >= 255`. Before this gate, forced-recurrence T127 q8 analysis regressed by a median 7.47%. The post-gate T127 q8 control showed no repeatable change. The gate avoids applying the fused kernel where its dispatch and setup cost outweigh the saved work.

## Native performance

The baseline is commit `b6f2415`; the accepted candidate is `7c79dd6`. The benchmark used `examples/m1b_bench.rs` with f64 scalar STANDARD analysis, GL, `phi0=0.37`, full `mmax`, `ntheta=lmax+1`, and `nphi=2*lmax+3`. Each process warmed the transform and recorded five timed analyses using a reused plan and workspace. Three alternating baseline/candidate process pairs ran at q1 (one CPU core) and q8 (8 pinned CPU cores). Values are medians of process medians. Paired change is the median of each pair's candidate/baseline percentage; the range spans the three paired changes. The host was an x86-64 CPU running Linux. Builds used Rust `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`, release `opt-level=3`, 16 codegen units, no LTO, portable Pulp runtime dispatch (AVX2/FMA on this CPU), and no `target-cpu=native`. The baseline executable was built at `b6f2415` in `<local-home-path> the candidate executable is `target/release/examples/m1b_bench` at `7c79dd6`. These commands reproduce the T255/T511 recurrence acceptance cells; use the baseline executable in place of the candidate path for paired baseline runs. T127 controls use the same arguments with `127 128 257 127`; T255 uses `255 256 513 255`, and T511 uses `511 512 1025 511`. ```sh
RUSTSHT_LEGENDRE_REUSE_ESTIMATE=-,0 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 0 255 1 analysis GL STANDARD 256 513 255 requested-only
RUSTSHT_LEGENDRE_REUSE_ESTIMATE=-,0 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 0 255 8 analysis GL STANDARD 256 513 255 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 0 511 1 analysis GL STANDARD 512 1025 511 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 0 511 8 analysis GL STANDARD 512 1025 511 requested-only
```

| Bandlimit and path | Threads | Baseline | Candidate | Paired change | Paired range |
| --- | ---: | ---: | ---: | ---: | ---: |
| T127 automatic compact-table control | 1 | 0.612 ms | 0.613 ms | +0.49% | +0.16% to +0.66% |
| T127 automatic compact-table control | 8 | 0.318 ms | 0.296 ms | -8.84% | -14.78% to -1.00% |
| T127 forced recurrence, before gate | 1 | 0.714 ms | 0.703 ms | -1.54% | -2.39% to -1.54% |
| T127 forced recurrence, before gate | 8 | 0.380 ms | 0.403 ms | +7.47% | +2.03% to +12.63% |
| T127 forced recurrence, gated candidate | 8 | 0.386 ms | 0.389 ms | -0.78% | -5.35% to +4.19% |
| T255 forced recurrence | 1 | 3.890 ms | 3.542 ms | -8.95% | -9.59% to -8.67% |
| T255 forced recurrence | 8 | 1.399 ms | 1.384 ms | -3.69% | -4.46% to +2.29% |
| T511 automatic recurrence | 1 | 22.585 ms | 19.878 ms | -12.21% | -12.23% to -11.30% |
| T511 automatic recurrence | 8 | 6.930 ms | 6.541 ms | -5.51% | -6.34% to -5.30% |

The T127 automatic results use the existing compact-table path and are a selector control, not evidence for fused recurrence performance. Forced recurrence used `RUSTSHT_LEGENDRE_REUSE_ESTIMATE=-,0`; T511 selects recurrence automatically. T255 q1 and both T511 thread counts improved in all three pairs. T255 q8 had a favorable median but one pair crossed zero, so the q8 result is not conclusive. The measurements support the fused path for the tested f64 scalar GL analysis cases; they do not establish gains for other grids, precisions, spins, bandlimits, or operations. Raw samples and process logs, including setup timing, are in [`reports/data/fused-scalar-analysis-2026-10-15`](data/fused-scalar-analysis-2026-10-15/). A separate peak-RSS spot check is recorded in that directory; it is descriptive and was not part of the paired timing acceptance. The separate T511 process check measured peak RSS of `28,396/40,224 KiB` for baseline q1/q8 and `28,352/41,388 KiB` for candidate q1/q8. This was one process per case; the q8 candidate used `1.14 MiB` more peak RSS in this sample, while q1 was effectively unchanged. The kernel uses existing worker scratch, and this single-process check does not establish a repeatable memory change. The measurements used `/usr/bin/time` around the same `requested-only` benchmark command, pinned to one CPU core or 8 pinned CPU cores:

```sh
/usr/bin/time -f 'max_rss_kib=%M' taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" <baseline-or-candidate-binary> 0 511 1 analysis GL STANDARD 512 1025 511 requested-only
/usr/bin/time -f 'max_rss_kib=%M' taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" <baseline-or-candidate-binary> 0 511 8 analysis GL STANDARD 512 1025 511 requested-only
```

## Numerical validation

The fused-block unit test compares the new recurrence and accumulation with the previous paired recurrence over SIMD lanes and a tail, and confirms that scaled sectoral inputs decline the fast path. A T511 GL test uses `a(511,511)=1` and `phi0=0.37`: five middle-ring synthesis samples match stored DUCC 0.41.0 references within `5e-11`; the complete map matches the independently evaluated sectoral formula within `5e-10`; analysis recovers the packed coefficient vector within `2e-7`. The T255 scalar analysis test also matches DUCC 0.41.0 coefficients. The DUCC source revision is `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Validation passed: `cargo fmt --all -- --check`; workspace Clippy with all targets/features and warnings denied; `cargo test --workspace --all-features` (171 Rust tests and 10 f32 tests passed; 9 ignored); and `cargo test -p rustsht --no-default-features` (140 Rust tests and 10 f32 tests passed; 7 ignored). The opt-in M3 T511/T1023/T2047 all-grid test remains ignored and was not run in this study. No Python bindings changed, so `uv run pytest python/tests` was not rerun.
