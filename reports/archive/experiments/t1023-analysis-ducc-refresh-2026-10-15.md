# f64 T1023 scalar and spin-1 analysis timing against DUCC

Five alternating process pairs measured f64 GL STANDARD analysis at T1023 for
scalar and spin 1. The spin-1 timings were close to DUCC: RustSHT/DUCC paired
median ratios were `0.97x` at q1 and `1.14x` at q8. Scalar analysis measured
`1.32x` at q1 and `2.13x` at q8. RustSHT used a reused native plan and
workspace; DUCC used the Python `analysis_2d` call, so the ratios describe
these two call boundaries.

| Spin | Threads | RustSHT median (range) | DUCC median (range) | Paired ratio median (range) |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 1 | `164.117 ms` (`157.845–168.894`) | `124.705 ms` (`120.401–126.301`) | `1.32x` (`1.30–1.34x`) |
| 0 | 8 | `42.436 ms` (`36.886–44.203`) | `19.895 ms` (`18.764–21.696`) | `2.13x` (`1.84–2.25x`) |
| 1 | 1 | `250.939 ms` (`248.170–254.406`) | `259.588 ms` (`257.958–263.361`) | `0.97x` (`0.95–0.98x`) |
| 1 | 8 | `42.404 ms` (`42.181–43.107`) | `37.214 ms` (`36.940–37.443`) | `1.14x` (`1.13–1.16x`) |

The grid was `1024x2049`, with `mmax=1023`, `phi0=0.37`, and deterministic
full-band map input. DUCC was `ducc0==0.41.0`, source revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`; Python was `3.11.15` with NumPy
`2.4.6`. RustSHT source was commit
`7ac443a40b034d5c52526d23ef30f6cef4b13d19`, Rust `1.99.0`, LLVM `23.1.1`,
portable Pulp runtime dispatch, and no `target-cpu=native`.

q1 was pinned to one CPU core; q8 was 8 pinned CPU cores on an x86-64 CPU
running Linux. The CPUs were not isolated, and no CPU quota
applied. Load-average telemetry was retained only in local diagnostics. The scalar values are
provisional under that load. The spin-1 screen ran after load fell from `3.53`
to `2.87`; its pair ranges were narrower. Structured timing samples, binary hashes and reproduction commands are
in [`the benchmark data`](../../data/sht-t1023-ducc-refresh-2026-10-15). Existing
scalar and spin-1 analytic, DUCC, adjoint and high-bandlimit tests remain the
correctness evidence; this timing study did not rerun numerical tests.
