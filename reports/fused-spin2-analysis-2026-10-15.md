# Fused spin-2 scalar-conversion analysis

## Kernel change

The f64 GL spin-2 analysis path converts Q and U Fourier spectra to two scalar coefficient rows. Before this change, each latitude block dispatched the paired Legendre recurrence, called an accumulation visitor for each degree pair, and dispatched the two-channel reducer from that visitor. The candidate advances the paired recurrence and reduces both channels within one SIMD dispatch. It stages each block's two coefficient rows in existing `fast_spin_first` and `fast_spin_second` worker scratch, then commits them only when the complete block remains in the normal range. If either channel's Fourier products are unsafe, or recurrence lanes need binary scaling, the candidate uses the existing paired scaled path. It adds no plan or workspace storage and leaves f32 on the prior path. The fused path is only considered inside the existing f64 spin-2 scalar-conversion analysis selection: exact GL analysis at `lmax >= 500`, with the existing `sin(theta) >= 0.01` fast latitude range. Cutoff-inactive lanes remain zeroed, and the general recurrence handles polar rings. No public API or DUCC code changed.

## Native performance

The baseline executable was copied from source commit `f86b559`; the candidate includes the fused analysis code. Both used `examples/m1b_bench.rs`, f64 spin-2 STANDARD analysis, full `mmax`, `phi0=0.37`, and reused plans and workspaces. The tested exact GL shapes were T511 `512x1025` and T720 `721x1443`. Each process warmed the transform and measured five analyses. Three alternating baseline/candidate pairs ran at q1 pinned to one CPU core and q8 8 pinned CPU cores. Values are medians of process medians; paired changes use each pair's candidate/baseline percentage. The host was an x86-64 CPU under Linux. Builds used Rust `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`, release `opt-level=3`, 16 codegen units, no LTO, portable Pulp runtime dispatch (AVX2/FMA on this CPU), and no `target-cpu=native`.

| Bandlimit | Threads | Baseline | Candidate | Paired change | Paired range |
| --- | ---: | ---: | ---: | ---: | ---: |
| T511 | 1 | 127.305 ms | 117.930 ms | -7.36% | -7.48% to -6.66% |
| T511 | 8 | 39.417 ms | 38.508 ms | -2.59% | -3.39% to -1.31% |
| T720 | 1 | 298.693 ms | 271.846 ms | -8.99% | -9.46% to -7.82% |
| T720 | 8 | 83.770 ms | 80.577 ms | -3.32% | -3.98% to -2.96% |

All three pairs favored the fused path at both sizes and thread counts. The q1 changes were 7–9%; q8 changes were 2.6–3.3%. The result applies to the tested f64 GL spin-2 STANDARD analysis path at T511 and T720. The exact commands used the candidate executable shown below; the baseline runs substituted `target/m1b_bench-before-spin2-analysis-fusion`, copied from `f86b559` before editing:

```sh
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 2 511 1 analysis GL STANDARD 512 1025 511 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 2 511 8 analysis GL STANDARD 512 1025 511 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 2 720 1 analysis GL STANDARD 721 1443 720 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 2 720 8 analysis GL STANDARD 721 1443 720 requested-only
```

Structured timing samples and the RSS spot check are in [`reports/data/spin2-fused-analysis-2026-10-15`](data/spin2-fused-analysis-2026-10-15/). One T720 process per case measured peak RSS of `51,524/52,396 KiB` at baseline q1/q8 and `52,744/53,220 KiB` at candidate q1/q8. These single-process values are descriptive; they do not establish a repeatable memory change. The candidate reuses existing scratch.

## Numerical validation

The new fused-block test compares both channel outputs with the existing paired recurrence and accumulation across 17 SIMD lanes and a tail. A scaled-sectoral case confirms that the fast block declines to the fallback. A high-level T511 test analyzes a `1e300` Q map with a unit U map and compares both outputs with the general spin recurrence; it verifies the unsafe Q products bypass fusion while preserving finite results. `cargo test --release -p rustsht --all-features m3_spin_t511_all_grids_matches_ducc_and_roundtrips -- --ignored --test-threads=1` passed. The test compares spin-1/2 operations against stored DUCC 0.41.0 references and roundtrips on GL, CC and F1; the fused path is selected for the GL spin-2 analysis case. The pinned DUCC source revision is `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Validation passed: `cargo fmt --all -- --check`; workspace Clippy with all targets/features and warnings denied; `cargo test --workspace --all-features` (174 Rust tests and 10 f32 tests passed; 9 ignored); and `cargo test -p rustsht --no-default-features` (143 Rust tests and 10 f32 tests passed; 7 ignored). The focused two-channel SIMD tests and unsafe-channel fallback test passed, as did the release T511 spin-1/2 GL/CC/F1 DUCC and roundtrip test. The opt-in T1023 spin-2 tests were not run for this change. No Python bindings changed, so Python tests were not run.
