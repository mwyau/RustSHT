# M6 Rader and Bluestein workspace buffer sharing

Date: 2026-10-15. Baseline: M5 commit `9cbd354f2e3e2f1debef023cca68b86900904cfe`.

## Decision

Share the Bluestein convolution buffer with the 241-point Rader pass used by
the N=723 real plan. Keep the owned Rader convolution buffer for the 257-point
pass used by N=257.

The initial all-radix sharing candidate saved workspace bytes at both lengths,
but repeated paired runs showed an N=257 C2R slowdown. Moving the shared slice
out of the group loop and bundling the buffer and FFT workspace into one
execution context did not remove that slowdown. The final implementation
therefore shares only the radix-241 buffer. The normal Rader and unsafe-input
Bluestein routes use the shared allocation sequentially. The nested complex
FFT workspace remains separate, and transform execution adds no allocation.

## Correctness and ownership

The N=723 Rader pass writes all 240 complex entries before running its nested
forward FFT. Each group overwrites those entries before reuse. Bluestein uses
the same parent workspace buffer only after the Rader route has returned. The
radix-257 route continues to use its owned 256-element convolution buffer.
Rader safety checks and fallback selection are unchanged.

The route diagnostic records N=257 as Rader-257 with a 256-element
convolution and a length-525 Bluestein fallback; N=723 uses Rader-241 with a
240-element convolution and a length-512 fallback. See
[`m5-current-head-route-diagnostics.txt`](../benches/results/m5-current-head-route-diagnostics.txt).

`rader_and_fallback_reuse_workspace_buffer_across_routes` compares forward
results and independent inverse spectra with direct DFT references at N=257
and N=723. It reuses each workspace across ordinary transforms, a validation
error, high-amplitude fallback execution, and later ordinary transforms. The
concurrency test also covers N=257 and N=723 with a separate workspace per
worker. Existing route tests exercise sparse, cancellation, broad-dynamic-range
and nonfinite fallback inputs.

The read-only Sol High review found no current path that reaches the empty
owned convolution vector for radix 241. It confirmed that the shared buffer is
fully written before use, nested FFT scratch remains separate, fallback reuse
is sequential, the arithmetic and scaling match the owned-buffer path, and the
workspace byte summaries reproduce from the raw captures. It found no
actionable issue.

## Workspace and resident memory

Workspace payload counts initialized elements and excludes allocator metadata.
At eight independent workspaces, the saved payload scales linearly.

| Real length | Route | M5 bytes per workspace | M6 bytes per workspace | Change |
|---:|---|---:|---:|---:|
| 241 | Bluestein control | 35,120 | 35,120 | 0 |
| 257 | Rader-257 plus Bluestein fallback | 46,032 | 46,032 | 0 |
| 723 | Rader-241 plus Bluestein fallback | 57,376 | 53,536 | -3,840 |

At N=723, eight workspaces use 428,288 bytes instead of 459,008 bytes, a
30,720-byte reduction. The memory probe also records Linux `VmHWM` for six
alternating process pairs at 1, 2, 4 and 8 workspaces. At N=723 the median
high-water mark was 1,984 KiB versus 1,998 KiB for one workspace and 2,374 KiB
versus 2,424 KiB for eight. The ranges overlap, so these measurements do not
establish a process-resident-memory reduction at this scale. The initialized
workspace-payload reduction is exact.

Raw and summarized measurements:

- [`m6-final-workspace-memory-six-pairs.jsonl`](../benches/results/m6-final-workspace-memory-six-pairs.jsonl)
- [`m6-final-workspace-memory-summary.csv`](../benches/results/m6-final-workspace-memory-summary.csv)

## Paired latency

The exact-target campaign uses six alternating process pairs pinned to logical
CPU 30. Each process reports the median of seven warmed public-operation
samples. Rows match the SHT workloads: 120 at N=241, 128 at N=257 and 361 at
N=723. Ratios are `M5 time / M6 time`; values above 1 indicate that M6 is
faster. The intervals are 95% bootstrap intervals over the six paired ratios.

| N | Operation | Rows | M6 median (µs) | M5 median (µs) | M5/M6 ratio | Paired range | 95% interval |
|---:|---|---:|---:|---:|---:|---:|---:|
| 241 | R2C | 120 | 10.859 | 10.818 | 0.997 | 0.994–1.001 | 0.994–1.000 |
| 241 | C2R | 120 | 10.970 | 10.993 | 1.003 | 0.999–1.009 | 0.999–1.007 |
| 257 | R2C | 128 | 2.127 | 2.155 | 1.010 | 0.988–1.037 | 0.994–1.033 |
| 257 | C2R | 128 | 2.409 | 2.478 | 1.022 | 1.007–1.042 | 1.012–1.039 |
| 723 | R2C | 361 | 11.055 | 11.597 | 1.051 | 1.011–1.062 | 1.026–1.059 |
| 723 | C2R | 361 | 11.731 | 12.940 | 1.103 | 1.084–1.118 | 1.087–1.112 |

The N=723 improvement is present in all six pairs for both directions. N=257
shows no material regression in the final candidate; its R2C interval includes
parity. N=241 is a Bluestein control and remains within 1% of M5. Plan and
workspace construction timings are included in the summary CSV; they show no
material plan-construction change.

- [`m6-final-real-target-summary.csv`](../benches/results/m6-final-real-target-summary.csv)
- [`N=241 raw pairs`](../benches/results/m6-final-real-241-six-pairs.jsonl)
- [`N=257 raw pairs`](../benches/results/m6-final-real-257-six-pairs.jsonl)
- [`N=723 raw pairs`](../benches/results/m6-final-real-723-six-pairs.jsonl)

The full standard matrix has 328 matched cells across 41 lengths, C2C forward
and inverse, R2C and C2R, and row counts 1 and 32 (four rows above N=10,000).
The N=723 row-32 cells improve 5.9% R2C and 11.7% C2R by paired median. The
matrix also contains noisy unrelated controls. R2C N=19 at 32 rows has a
0.934 ratio with a 0.763–1.012 interval; a focused six-pair follow-up remains
inconclusive at 0.944 with a 0.760–1.213 interval. No kernel change at N=19
explains that spread, so this result is recorded as provisional and is not
used to select the M6 route.

- [`m6-final-standard-summary.csv`](../benches/results/m6-final-standard-summary.csv)
- [`m6-final-standard-six-pairs.jsonl`](../benches/results/m6-final-standard-six-pairs.jsonl)
- [`m6-final-control-19-six-pairs.jsonl`](../benches/results/m6-final-control-19-six-pairs.jsonl)

## Rejected all-radix sharing

The first candidate shared Rader convolution scratch for both radices. Its
N=257 C2R median ratio was 0.880 (M5/M6), with a 0.844–0.943 95% interval;
M6 was about 13.7% slower. Moving the slice outside the per-group loop still
gave a 0.872 ratio and a 0.845–0.902 interval. Bundling the buffer and FFT
workspace into one context produced a noisier 0.798 ratio with a 0.625–1.092
interval. Those candidates were not retained. Their raw captures are preserved
as [`M6`](../benches/results/m6-rader-workspace-real-257-six-pairs.jsonl),
[`M6b`](../benches/results/m6b-rader-workspace-real-257-six-pairs.jsonl), and
[`M6c`](../benches/results/m6c-rader-workspace-real-257-six-pairs.jsonl).

## Measurement environment and scope

The runs used an AMD Ryzen 9 5950X under Linux x86-64, Rust 1.99.0
(`rustc 1.99.0`, LLVM 23.1.1), the release bench profile, a portable compiler
target, and runtime AVX2 dispatch. Both binaries use the same RustSHT-FFT
benchmark harness. The M5 benchmark executable SHA-256 is
`62e2e0bda8a791c3af015185c3b2096ba177caf6621d447ce104b5156c9d4e1f`; the
selected M6 executable SHA-256 is
`4fd81a13a3564d47fbf7c982b78e53d74aa1ecd9ef8c9ed1ef50e64600393192`.
Per-process order, load average, commands and hashes are recorded in the raw
JSONL captures.

## N=723 cross-library comparison

The final M6 candidate was compared with RustFFT 6.4.1, DUCC revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf` in portable and native builds,
system FFTW 3.3.10, and Admiral revision
`ce67bc00ff9e6de96bf5b75f7a789581105ddb58`. Each backend ran the public real
transform at N=723 with 361 sequential rows and one execution thread. Twelve
alternating process pairs were collected per backend and direction; each
process reports the median of seven warmed samples. Ratios are
`reference time / M6 time`, so values above one favor M6. Intervals are 95%
bootstrap intervals over the paired ratios.

| Reference | Operation | M6 (µs) | Reference (µs) | Reference/M6 | 95% interval |
|---|---|---:|---:|---:|---:|
| RustFFT 6.4.1 | R2C | 11.127 | 4.705 | 0.424 | 0.417–0.428 |
| RustFFT 6.4.1 | C2R | 11.769 | 4.744 | 0.403 | 0.402–0.404 |
| DUCC portable | R2C | 11.111 | 16.596 | 1.488 | 1.483–1.507 |
| DUCC portable | C2R | 11.738 | 16.806 | 1.424 | 1.416–1.436 |
| DUCC native | R2C | 11.121 | 11.655 | 1.048 | 1.041–1.052 |
| DUCC native | C2R | 11.862 | 11.609 | 0.983 | 0.968–0.987 |
| FFTW system 3.3.10 | R2C | 11.138 | 8.875 | 0.794 | 0.793–0.798 |
| FFTW system 3.3.10 | C2R | 11.754 | 8.990 | 0.765 | 0.760–0.766 |
| Admiral | R2C | 11.103 | 10.618 | 0.954 | 0.942–0.969 |
| Admiral | C2R | 11.756 | 10.655 | 0.903 | 0.894–0.913 |

M6 is faster than portable DUCC in both directions and close to native DUCC:
the R2C median favors M6 by 4.8%, while the C2R median favors native DUCC by
1.7%. M6 remains slower than RustFFT, FFTW and Admiral at this length. The
native-DUCC R2C paired range is wider than its bootstrap interval
(`0.966–1.121`), so its small median advantage should be treated as
load-sensitive. The broader cross-library matrix for lengths unaffected by
this workspace change remains in the [M5 comparison report](m5-current-head-cross-library.md).

The 12-pair raw captures and summary are
[`RustFFT R2C`](../benches/results/m6-final-crosslib-n723-rustfft-6.4.1-r2c-12-pairs.jsonl),
[`RustFFT C2R`](../benches/results/m6-final-crosslib-n723-rustfft-6.4.1-c2r-12-pairs.jsonl),
[`DUCC portable R2C`](../benches/results/m6-final-crosslib-n723-ducc-c1ed01b8-portable-r2c-12-pairs.jsonl),
[`DUCC portable C2R`](../benches/results/m6-final-crosslib-n723-ducc-c1ed01b8-portable-c2r-12-pairs.jsonl),
[`DUCC native R2C`](../benches/results/m6-final-crosslib-n723-ducc-c1ed01b8-native-r2c-12-pairs.jsonl),
[`DUCC native C2R`](../benches/results/m6-final-crosslib-n723-ducc-c1ed01b8-native-c2r-12-pairs.jsonl),
[`FFTW`](../benches/results/m6-final-crosslib-n723-fftw-system-3.3.10-12-pairs.jsonl),
[`Admiral`](../benches/results/m6-final-crosslib-n723-admiral-ce67bc00-12-pairs.jsonl), and
the [summary CSV](../benches/results/m6-final-crosslib-n723-summary.csv).
The merged captures preserve both original six-pair rounds and identify their
source files and pair numbers. The processes recorded one-minute load averages
from 0.71 to 2.76 on the 32-logical-CPU host.

## Full-output comparison

At N=723, the final M6 candidate was also compared across all C2C, R2C and C2R
output records against RustFFT, portable and native DUCC, FFTW and Admiral.
Each comparison covered 2,531 complex output records (5,062 scalar
components) at `atol=rtol=2e-11`. Maximum scaled differences were `6.523e-14`
against RustFFT, `4.508e-14` against portable DUCC, `4.574e-14` against native
DUCC, `5.546e-14` against FFTW and `7.422e-14` against Admiral. The comparison
commands and full results are in
[`m6-final-n723-output-comparison.txt`](../benches/results/m6-final-n723-output-comparison.txt).

The M6-to-M5 timing comparison establishes the source-relative change; this
focused cross-library campaign checks the final N=723 candidate. M6 changes
only the real radix-241 route, so the M5 cross-library table remains the
measured context for other lengths. These standalone FFT results do not
establish a whole-SHT speedup.
