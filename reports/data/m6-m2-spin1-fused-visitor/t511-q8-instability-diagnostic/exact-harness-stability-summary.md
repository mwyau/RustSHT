# Exact-harness baseline stability validation

Diagnostic-only baseline stability check; not candidate acceptance evidence. Four separate fresh processes ran in the requested order q8, q1, q1, q8 with the archived `backend=both` harness unchanged. The extension hash was verified as `a8641456e1fcdf8d37ce3dd583dd1b7cf224306ca792d38c2b75f3bfcac63847`; the harness hash was `72cd1986a220573c1a445bd6a6aa96b9df272a6bff1dc833843ee3f274c77739`. The harness reported Python 3.11.15, NumPy 2.4.6, DUCC 0.41.0, CPU x86-64 CPU, GL STANDARD f64 scalar T511, grid 512x1024, full mmax, phi0=0.37, scope `into`, operation synthesis, `runs=1`. `backend=both` ran its Rust-vs-DUCC synthesis check, then Rust synthesis `time_runs` performed one untimed warm-up and five timed calls. The raw rows below are the five timed Rust calls. CV is calculated as in the historical audit: sample standard deviation divided by the five-sample median. First-three and final-two fields are the medians of those subsets.

| Process | Raw Rust synthesis samples (ms) | CV | First 3 median | Final 2 median |
| --- | --- | ---: | ---: | ---: |
| q8-process01 | 24.938, 20.991, 14.161, 9.736, 9.289 | 48.96% | 20.991 | 9.512 |
| q1-process02 | 39.646, 39.516, 39.475, 39.645, 39.560 | 0.19% | 39.516 | 39.603 |
| q1-process03 | 39.787, 39.963, 39.534, 39.745, 39.827 | 0.39% | 39.787 | 39.786 |
| q8-process04 | 21.038, 15.110, 17.596, 9.379, 9.433 | 33.84% | 17.596 | 9.406 |


`/usr/bin/time -v` gives whole-process context-switch totals only. Because the archived harness was run unchanged, it exposes no per-call involuntary context-switch deltas; those cannot be derived from the five elapsed-time values. the q8 process diagnostics differed, yet both show an early-to-late decline. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. This indicates the q8 timing instability is present on baseline under the archived method and makes q8 candidate comparisons inconclusive. It does not identify the cause or establish a revised warm-up/acceptance protocol. The prior selected audit samples were not modified; all process timing series and numerical comparisons are summarized here; raw process and host captures remain local. Rust-vs-DUCC error was identical on each run: `max_abs=3.482e-14`, `rms=7.845e-16`, `rel_l2=6.632e-14`. DUCC timing was recorded separately by the harness.
