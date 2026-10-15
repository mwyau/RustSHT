# M2 native real radix-11 pass

Date: 2026-10-15. This change adds a fixed radix-11 pass to the native real
transform planner. It reuses the existing complex radix-11 butterfly for
interior complex lanes and uses fixed real formulas for the real lane and
terminal pass. The planner retains the general-prime path for standalone
length 11 because paired measurements showed a regression for that case.

## Kernel and plan storage

The real butterfly combines the five symmetric sample pairs. Their sums feed
the DC and cosine terms; their differences feed the sine terms. Interior lanes
with nonzero frequency components call the existing complex radix-11
butterfly, with the existing twiddle and packed-spectrum layout. The terminal
inverse reads the six-bin real spectrum and reconstructs the eleven samples.
These equations follow DUCC's `fft1d_impl.h` at revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`.

For mixed lengths containing radix 11, the planner omits the generic-prime
root vector and 121-byte phase-index table. Their logical payload is 297 bytes
per pass for f64 and 209 bytes per pass for f32. Lengths 121, 242, 363, and 726
have two such passes; lengths 22, 1023, and 1441 have one. Length 11 retains
the general-prime tables. Workspace sizes are unchanged, and the fixed pass
adds no execution allocation.

The route test checks terminal layout `(N=22, l1=2, ido=1)` and interior
layouts at 121, 242, 363, 726, 1023, and 1441. It also checks that the
standalone length-11 plan and controls 513, 1025, 1443, and 1452 do not select
the native real specialization. Length 1452 uses the existing complexified
length-726 path.

## Numerical validation

The f32 and f64 real tests compare forward output and independently supplied
inverse spectra with direct DFTs at lengths 11, 22, 121, 242, 363, 726, 1023,
and 1441. The length-22 terminal inverse case includes cancellation, imaginary
DC and Nyquist values, minimum normal values, minimum subnormals, and maximum
finite DC values. These cases exercise the fixed terminal pass.

The pinned DUCC comparison covered C2C forward/inverse and R2C/C2R at the
eight test lengths plus controls 513, 1025, 1443, and 1452. It compared 29,345
output records (58,690 scalar components); maximum absolute error was
`1.705e-13`, and maximum scaled error was `1.137e-13`. DUCC used revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`, g++ 15.2.0 with
`-std=c++20 -O3 -DNDEBUG -march=native -pthread`, and `vectorize=off`.
The full build and comparison details are in
[`m2-radix11-ducc-comparison.txt`](../benches/results/m2-radix11-ducc-comparison.txt).

## Warm timings

The baseline is M1 commit `15023973f1880775b28bac5ffd9fb18a7ff49c76`; the
candidate adds the fixed pass. Both use Rust 1.99.0's optimized bench profile
without `target-cpu=native`, on an AMD Ryzen 9 5950X, one thread, pinned to
logical CPU 30. Eight alternating process pairs measured one-row and 32-row
R2C/C2R calls. Each call timing is the median of one warm-up and seven timed
samples; speedups are paired baseline/candidate ratios, with 95% paired
bootstrap intervals. The acceptance gate required each mixed target and
operation to exceed 1.10x median speedup and 1.05x interval lower bound. Every
target passed.

| Length | R2C speedup, rows 1 / 32 | C2R speedup, rows 1 / 32 |
| ---: | ---: | ---: |
| 121 | 2.000x / 2.108x | 2.303x / 2.365x |
| 242 | 2.023x / 2.042x | 2.293x / 2.324x |
| 363 | 2.121x / 2.146x | 2.313x / 2.368x |
| 726 | 2.030x / 2.034x | 2.289x / 2.304x |
| 1023 | 1.735x / 1.741x | 1.826x / 1.837x |
| 1441 | 1.300x / 1.293x | 1.345x / 1.338x |

Across these 24 target cells, the smallest paired-bootstrap lower bound was
1.291x for R2C and 1.327x for C2R. The N=11, N=22, and control medians were
all at least 0.95x. The fallback matters at N=11: the initial fixed route had
a 0.891x C2R median at 32 rows (95% interval 0.738–0.938). In the final
matrix, the general N=11 route and controls 513, 1025, 1443, and 1452 remained
within the configured 5% median regression limit.

The paired CSV includes elapsed time, setup time, checksums, and workspace
bytes for each operation, length, and row count. The per-process captures and
configuration are in [`benches/results`](../benches/results/):

- `m2-radix11-process-paired.jsonl` and `m2-radix11-summary.csv` contain the
  final eight-pair warm matrix.
- `m2-radix11-pre-fallback-process-paired.jsonl` and
  `m2-radix11-pre-fallback-summary.csv` record the fixed N=11 measurements
  used to choose the generic fallback.
- `m2-radix11-n1441-process-paired.jsonl` and
  `m2-radix11-n1441-summary.csv` contain a 12-pair repeat.

## First-call measurements and limits

The first-call harness constructs a plan, workspace, and input before timing
the first public operation for each plan. In the multi-length set, later
plans run in processes that have already executed transforms. The 12-pair
isolated N=1441 run measured R2C at 1.161x first-call speedup
(95% interval 1.012–1.421) and 1.185x for the following warm calls
(1.044–1.251). C2R measured 1.316x first-call speedup (1.199–1.431) and
1.354x warm speedup (1.259–1.483). In the full matrix, N=1441 R2C's
first-call interval included 1.0x (0.910x median; 0.874–1.215). N=22 R2C
measured a 0.877x first-call median (0.760–0.889). These results do not
establish a general first-call non-regression result.

First-call captures and summaries are in `m2-radix11-first-call-*` and
`m2-radix11-n1441-first-call-*`; `summarize_first_call.py` reproduces the
paired medians and bootstrap intervals. The M1 baseline binary includes the same
benchmark-only first-call-set addition as the candidate. Their executable
SHA-256 values are recorded in the JSON configurations. These measurements
cover standalone f64 FFTs on one CPU; they do not establish f32 or end-to-end
SHT performance.

## Checks

- `cargo fmt --all -- --check` passed in the root workspace and standalone FFT
  crate.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  passed in both workspaces.
- `cargo test --workspace --all-features` passed in the root workspace and
  standalone FFT crate, in debug and release. The standalone crate ran 17 unit,
  2 prototype, 28 complex, and 35 real tests in each profile.
- The final DUCC comparison passed at `atol=rtol=2e-11`.
- `git diff --check` passed before commit.
