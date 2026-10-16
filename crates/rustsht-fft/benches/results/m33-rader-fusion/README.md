# M33 real-prime investigation and final M32 source evidence

The accepted source remains M32 at `234ab2d30b8d542457398e2f7be17e7e7b9aa25e`. M33 safety-scan fusion improved the selected real rows but regressed C2C forward N=146 and N=722, so the candidate source was removed. The final report is [`docs/m33-final-performance-report.md`](../../../docs/m33-final-performance-report.md).

## Accepted-source captures

The `m32-accepted-*-six-pairs-config.json` and `.jsonl` files contain fresh six-pair comparisons for the accepted M32 binary against RustFFT 6.4.1, native and portable DUCC at `c1ed01b8fda69d2a378190f0915f1a19762dddbf`, FFTW 3.3.10, and Admiral `ce67bc00`. Each capture covers the established 34-cell C2C and real matrix. [`capture-integrity.txt`](capture-integrity.txt) records successful process exit codes and all 34 result rows in every capture. The accepted binary SHA-256 is `a1036e527f31637d8f0fd9212d1401c2d568155cd1e8dc0faff785aa05333a8c`.

[`m32-accepted-source-cross-library-summary.csv`](m32-accepted-source-cross-library-summary.csv) combines the five complete matrices. [`m32-accepted-route-memory-diagnostics.txt`](m32-accepted-route-memory-diagnostics.txt) records planner routes and retained-plan/workspace bytes. Numerical comparisons against DUCC, FFTW, and Admiral are in:

- [`m32-accepted-native-ducc-numerical-comparison.txt`](m32-accepted-native-ducc-numerical-comparison.txt)
- [`m32-accepted-portable-ducc-numerical-comparison.txt`](m32-accepted-portable-ducc-numerical-comparison.txt)

## M33 v3 candidate captures

`m33-rader-only-pack-v3-34-cell-six-pairs-*` records the final candidate comparison against the accepted M32 baseline: six alternating process pairs, seven warmed samples, one transform thread, and one CPU core. The full matrix is in [`m33-rader-only-pack-v3-34-cell-six-pairs-summary.csv`](m33-rader-only-pack-v3-34-cell-six-pairs-summary.csv). Earlier v1 and v2 screens are preserved in their original files. The five `m33-{rustfft,ducc-native,ducc-portable,fftw,admiral}-six-pairs-*` sets compare the rejected v3 candidate across the same 34-cell workload. These are labeled candidate diagnostics, not final accepted-source performance. Their reference summaries identify RustFFT 6.4.1, native and portable DUCC, FFTW 3.3.10, and Admiral `ce67bc00`. The independent reviewer recommended rejecting v3 after confirming repeatable 6.5% C2C forward slowdown at N=146 and 9.1% at N=722. Its source diff hash is `8845bead2e211dffc709f2cb13c908f4332754329bec60b45b10d59f666f7ee0`; no candidate source changes remain in the branch.

## Profiles and validation


The M33 candidate’s first-call diagnostic is [`m33-first-call-diagnostic.txt`](m33-first-call-diagnostic.txt). It records one non-paired observation per binary and separates first-call values from seven warmed samples. The validation commands and result are summarized in [`final-validation-summary.txt`](final-validation-summary.txt). Final accepted-source checks are recorded in [`final-fmt-check.txt`](final-fmt-check.txt), [`final-workspace-clippy.txt`](final-workspace-clippy.txt), [`final-validation-summary.txt`](final-validation-summary.txt), and [`final-validation-summary.txt`](final-validation-summary.txt). The debug and release suites each passed 111 tests.
