# f32 spin-1 analysis SIMD accumulation screen

The f32 spin-analysis visitor now has a guarded Pulp accumulation path. It vectorizes E/B accumulation across latitude lanes when every lane is active for the order, Fourier inputs and basis values are normal, and both recurrence exponent arrays are zero. Scaled, inactive, zero, subnormal, and otherwise exceptional blocks keep the scalar path. The kernel uses existing workspace arrays and handles a scalar SIMD tail. Two five-pair alternating process screens at f32 GL T511 show a repeatable analysis improvement: about 14% at q1 and 9–10% at q8. The full-transform screen also shows q1 synthesis 2–3% slower in the candidate executable. Combined synthesis-plus-analysis time improved 5.8% at q1; the q8 combined change was small and its paired ranges crossed zero. The SIMD path remains because analysis improves in every pair in both screens, while the report records the synthesis movement and q8 uncertainty.

## Method

The benchmark used `examples/m4_f32_bench.rs`, with reused plan/workspace, spin 1, GL `512x1025`, T511, full `mmax`, `phi0=0.37`, deterministic coefficients, and the harness's deterministic map inputs. Each fresh process warmed each operation once, then reported the median of five calls for q1 and q8 synthesis and analysis. Five baseline/candidate process pairs ran in alternating order per screen, affinity sized for the requested worker count, under `target/rustsht-benchmark.lock`. `/usr/bin/time` recorded process elapsed time and peak RSS. The baseline binary was built from commit `1e69eb3`; the candidate included the uncommitted patch saved in the data directory. Both used Rust `1.99.0` / LLVM `23.1.1`, release optimization, portable Pulp dispatch, and the host's AVX2/FMA path. The host was an x86-64 CPU with Linux. Load-average telemetry was retained only in local diagnostics. The host was shared and not idle. Local interference review found no unrelated CPU-intensive workload before the screens.

## Results

Times are medians across five process medians. Paired change is the median candidate/baseline percentage across the five alternating pairs; ranges show the minimum and maximum paired changes. Negative values favor the candidate.

| Screen | Operation | Baseline (ms) | Candidate (ms) | Paired change | Paired range |
| --- | --- | ---: | ---: | ---: | ---: |
| First | Spin-1 analysis q1 | 1072.337 | 919.450 | -14.40% | -14.49% to -13.52% |
| First | Spin-1 analysis q8 | 182.314 | 167.455 | -8.58% | -9.01% to -3.27% |
| First | Spin-1 synthesis q1 | 1048.288 | 1082.590 | +2.64% | +2.21% to +5.40% |
| First | Spin-1 synthesis q8 | 149.204 | 161.926 | +5.38% | -3.22% to +16.88% |
| First | Synthesis + analysis q1 | 2117.711 | 2002.220 | -5.88% | -6.05% to -4.24% |
| First | Synthesis + analysis q8 | 333.237 | 329.821 | -2.57% | -6.06% to +5.54% |
| Repeat | Spin-1 analysis q1 | 1030.065 | 884.650 | -14.13% | -15.98% to -13.77% |
| Repeat | Spin-1 analysis q8 | 182.533 | 164.946 | -10.08% | -21.74% to -3.93% |
| Repeat | Spin-1 synthesis q1 | 1012.600 | 1038.612 | +2.23% | +1.87% to +3.53% |
| Repeat | Spin-1 synthesis q8 | 154.821 | 169.819 | +3.22% | -2.77% to +14.65% |
| Repeat | Synthesis + analysis q1 | 2040.993 | 1920.713 | -5.82% | -7.11% to -5.27% |
| Repeat | Synthesis + analysis q8 | 337.354 | 335.514 | -3.02% | -13.18% to +4.53% |

Median process peak RSS was 14,744 KiB baseline and 14,764 KiB candidate in the first screen; the repeat medians were 14,948 KiB and 15,020 KiB. Estimated core numeric arrays remain 12.13 MiB. The implementation adds no workspace arrays. The screen does not include a new DUCC wall-time run. The prior `ducc0==0.41.0` T511 timings remain context only and are not divided into these RustSHT measurements. A new f32 Rust test compares selected E/B coefficients at low, middle, and maximum degree against DUCC 0.41.0, source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, using GL `512x1025` and `phi0=0.37`.

## Validation

The accumulation tests compare `Arch::new()` and `Arch::Scalar` against a lane-wise reference for 64- and 61-lane blocks, E-only and E/B output, SIMD tails, scaled exponents, inactive cutoff lanes, and zero-valued Fourier input. The T511 DUCC check passes in debug and release mode. The all-feature workspace run passed 184 Rust unit tests and 11 f32 integration tests; doctests passed separately using workspace-local scratch after temporary storage was exhausted. The no-default-features run passed 146 Rust unit tests and 11 f32 integration tests. Formatting and workspace Clippy with warnings denied passed. Python tests were not run because bindings did not change. Structured timing samples, parser, binary hashes, and the candidate source patch are in [`the data directory`](data/f32-spin1-analysis-simd-2026-10-15/).
