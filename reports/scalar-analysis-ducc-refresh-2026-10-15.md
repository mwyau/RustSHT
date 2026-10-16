# f64 scalar T511 analysis timing against DUCC

Five alternating process pairs measured f64 GL T511 STANDARD analysis with a
reused native RustSHT plan/workspace and DUCC's Python `analysis_2d` call.
The paired median RustSHT/DUCC ratio was `1.65x` at q1 and `2.17x` at q8.
Every pair favored DUCC, although each thread-count screen had one timing
outlier. The q1 DUCC outlier increased its process range to `12.264–15.586
ms`; the q8 RustSHT outlier increased its range to `3.570–4.475 ms`.

| Threads | RustSHT median (range) | DUCC median (range) | Paired ratio median (range) |
| ---: | ---: | ---: | ---: |
| 1 | `20.488 ms` (`20.296–20.541`) | `12.287 ms` (`12.264–15.586`) | `1.65x` (`1.30–1.67x`) |
| 8 | `3.635 ms` (`3.570–4.475`) | `1.675 ms` (`1.656–1.697`) | `2.17x` (`2.14–2.64x`) |

The operation used GL `512x1025`, `mmax=511`, `phi0=0.37`, and deterministic
full-band map input. DUCC was `ducc0==0.41.0`, source revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`; Python was `3.11.15` with NumPy
`2.4.6`. RustSHT used source commit
`7ac443a40b034d5c52526d23ef30f6cef4b13d19`, Rust `1.99.0`, LLVM `23.1.1`,
portable Pulp runtime dispatch, and no `target-cpu=native`.

q1 was pinned to CPU 0; q8 was pinned to CPUs 8–15. The host was an AMD Ryzen
9 5950X running Linux `7.0.14-20-pve`. Load average decreased from `5.04`
before the screen to `4.04` afterward. The CPUs were not isolated, and the
Rust native and DUCC Python timing boundaries differ. A prior Python-plan
comparison recorded DUCC at `6.470/1.074 ms` for q1/q8
([scalar normalization report](scaled-lane-normalization-2026-10-14.md));
the new DUCC medians are higher. This screen records a current observation and
does not replace the earlier table.

Raw samples, process timings, load snapshots, commands and binary hashes are
in [`the benchmark data`](data/scalar-analysis-ducc-refresh-2026-10-15/).
No Rust source changed during the screen. This timing study did not rerun
numerical tests; existing scalar analytic, DUCC, adjoint and high-bandlimit
tests remain the correctness evidence.
