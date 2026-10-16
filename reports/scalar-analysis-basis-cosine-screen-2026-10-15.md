# Scalar analysis basis-cosine reuse screen

## Candidate

The candidate rewrote opposite-parity products from
`(spectrum * basis) * cos(theta)` to `spectrum * (basis * cos(theta))`. It
computed `basis * cos(theta)` once for each SIMD lane and reused it for the real
and imaginary components. The recurrence, cutoff, fallback and workspace
behavior were unchanged. This is an algebraic reassociation and can change
rounding in the last few bits.

## Method

Five alternating release-process pairs compared the candidate with baseline
commit `87f8991`. Each process ran `examples/m1b_bench.rs` for f64 scalar GL
STANDARD analysis at T511 (`512 x 1025`, full `mmax`), using a reused plan and
workspace, one warmup, and five timed calls. q1 processes were pinned to CPU 0;
q8 processes were pinned to CPUs 0–7. The host was an AMD Ryzen 9 5950X with
32 logical CPUs. Rust was `1.99.0` with LLVM `23.1.1`; release builds used
portable Pulp runtime dispatch and AVX2/FMA without `target-cpu=native`.

Load averages were `1.57, 2.97, 4.21` at the start and `1.69, 2.97, 4.20` at
the end. These are shared-host observations. The raw file retains all inner
samples, affinity, load averages, process times and binary hashes:
[`paired-process-output.txt`](data/scalar-analysis-basis-cosine-screen-2026-10-15/paired-process-output.txt).
The candidate patch is retained in the same directory.

## Results

Each table entry is a process median. Paired change is the median percent
change within the five baseline/candidate pairs; negative values favor the
candidate.

| Threads | Baseline median | Candidate median | Median paired change | Paired change range |
| ---: | ---: | ---: | ---: | ---: |
| q1 | 20.027 ms | 20.206 ms | +0.94% | +0.54% to +1.00% |
| q8 | 3.606 ms | 3.666 ms | -1.39% | -5.40% to +40.99% |

All five q1 pairs were slower with the candidate. q8 had four faster pairs and
one `+40.99%` slow pair, leaving a broad range that crosses zero. The candidate
did not demonstrate a repeatable whole-transform improvement and was removed.
The change added no workspace storage. Process peak RSS was not collected.

## Validation

Focused paired-analysis tests passed (7 tests), fused scalar analysis block
tests passed (2 tests), and the release T511 GL scalar analysis test passed
against pinned DUCC samples and the analytic sectoral field. The workspace
suite and Clippy were not run with this rejected prototype. No source change
from the candidate remains in the working tree.
