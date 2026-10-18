# f64 T1023 scalar analysis timing follow-up

Five alternating fresh-process pairs measured scalar GL STANDARD analysis at
T1023. The paired RustSHT/DUCC ratios were `1.32x` at q1 and `1.67x` at q8. The q8 ratio is lower than the provisional `2.13x` from the earlier screen;
both RustSHT and DUCC timings were lower in this run.

| Threads | RustSHT median (range) | DUCC median (range) | Paired ratio median (range) |
| ---: | ---: | ---: | ---: |
| q1 | `158.026 ms` (`157.455–158.762`) | `119.376 ms` (`119.041–120.929`) | `1.32x` (`1.31–1.33x`) |
| q8 | `26.460 ms` (`25.919–28.158`) | `15.864 ms` (`15.765–16.029`) | `1.67x` (`1.62–1.77x`) |

The grid was `1024x2049`, with `mmax=1023`, `phi0=0.37`, and the same
deterministic full-band map for each process. RustSHT used a reused native
plan and workspace. DUCC used the Python `analysis_2d` call. The package was
`ducc0==0.41.0`, source revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`; Python was `3.11.15` with NumPy
`2.4.6`. RustSHT used rustc `1.99.0`, LLVM `23.1.1`, release defaults and
portable Pulp runtime dispatch, without `target-cpu=native`. Its source was
unchanged from commit `7ac443a40b034d5c52526d23ef30f6cef4b13d19`; the binary
SHA256 was
`690fc10f9abf1c6a5b396e0d7ff27647daf8e51d90b8620537645123efe7f1c8`.

q1 was pinned to one CPU core and q8 to 8 pinned CPU cores on an x86-64 CPU with host topology omitted, running Linux. Load-average telemetry was retained only in local diagnostics. A preflight review found unrelated CPU activity that could migrate across cores;
these remain shared-host observations rather than isolated-core measurements. The report data directory retains paired numerical summaries and binary provenance;
raw host captures remain local and untracked. The timing screen did not change Rust source or rerun numerical tests. Existing
analytic, DUCC, adjoint and high-bandlimit tests remain the correctness
evidence. Before the timing refresh, the current-source phase diagnostic
measured `24.190 ms` in scalar recurrence/accumulation at q6 and `7.017 ms` in
the forward row FFT. Further scalar tuning should focus on the recurrence
path, with a paired complete-transform comparison before accepting a change.
