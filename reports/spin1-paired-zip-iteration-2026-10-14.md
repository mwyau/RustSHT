# Spin-1 paired synthesis coefficient iteration

## Change and hypothesis

The fused spin-1 synthesis loop indexed six degree-pair coefficient rows in
each iteration. AVX2 disassembly showed six repeated slice bounds checks in
the vector loop. This change slices all six rows once to their common
`pair_count`, then iterates them together. The vector blocks and scalar tail
use cloned iterators over the same bounded range. The recurrence, coefficient
arithmetic, accumulation order, and final-pair guard are unchanged. The code
uses borrowed slices and adds no workspace allocation or retained memory. Baseline source was `6066fa2` on `main`; the candidate was the corresponding
uncommitted source diff, later committed with this report. The source change
removes per-row indexed accesses from the degree loop. Generated AVX2 code no
longer contains six per-array bounds branches in that loop; the iterator's
degree bound and existing final-pair recurrence guard remain. This was a
machine-code improvement with a directly matched whole-transform measurement,
not an inference from instruction count alone.

## Native before/after measurements

The benchmark used `examples/m1b_bench`, f64 GL spin-1 STANDARD synthesis,
full `mmax`, `phi0=0.37`, exact GL dimensions, a reused plan and workspace,
one warm-up and five timed transforms per process. Five fresh-process pairs
per case alternated baseline/candidate order. q1 used one CPU core; q8 used 8 pinned CPU cores. The table reports the median of the five process medians and the median
paired candidate reduction. Raw process medians and all within-process samples
are in [`process-samples.txt`](data/spin1-paired-zip-iteration-2026-10-14/process-samples.txt).

| Grid | Threads | Baseline | Candidate | Paired reduction | Faster pairs |
| --- | ---: | ---: | ---: | ---: | ---: |
| T511 (`512x1025`) | 1 | 57.625 ms | 51.152 ms | 11.04% | 5/5 |
| T511 (`512x1025`) | 8 | 21.006 ms | 19.222 ms | 7.64% | 5/5 |
| T720 (`721x1443`) | 1 | 132.456 ms | 116.433 ms | 12.06% | 5/5 |
| T720 (`721x1443`) | 8 | 44.910 ms | 44.461 ms | 2.95% | 4/5 |

T511 q8 paired changes ranged from `-4.54%` to `18.68%`; T720 q8 ranged
from `-1.00%` to `4.51%`. The q1 improvements were consistent and substantial. The T720 q8 result is modest and noisy; it is not presented as a reliable
multicore gain.

baseline and `6,399,845,770` for the candidate (`-9.49%`) in a q1 T511 process
that included setup and five timed synthesis calls. This is a simulated
instruction count, not cycles or elapsed time. Both runs completed with a

## DUCC comparison and numerical checks

The rebuilt Python extension was compared with installed `ducc0==0.41.0`
(source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`). These are
candidate Python-plan timings, not the native before/after measurements above. Inputs, GL geometry, `phi0`, and coefficients matched.

| Grid | Threads | RustSHT plan | DUCC0 | RustSHT/DUCC0 | Max abs / RMS / relative L2 error |
| --- | ---: | ---: | ---: | ---: | ---: |
| T511 | 1 | 52.706 ms | 18.692 ms | 2.82x | `3.257e-14 / 8.152e-16 / 6.697e-14` |
| T511 | 8 | 18.441 ms | 2.546 ms | 7.24x | `3.257e-14 / 8.152e-16 / 6.697e-14` |
| T720 | 1 | 123.215 ms | 52.270 ms | 2.36x | `7.446e-14 / 1.564e-15 / 9.123e-14` |
| T720 | 8 | 48.020 ms | 6.329 ms | 7.59x | `7.446e-14 / 1.564e-15 / 9.123e-14` |

DUCC remains faster, especially at eight threads. The zip change improves
RustSHT's spin-1 synthesis kernel but does not close the broader parallel
execution gap. Native and Python-facing samples and comparison details are in
[`ducc-python-results.txt`](data/spin1-paired-zip-iteration-2026-10-14/ducc-python-results.txt). Validation passed for the focused scalar-versus-general spin-1 synthesis
cases; T511 spin-1/2 DUCC and round-trip tests on GL, CC and F1; T2047 sparse
spin-1 through spin-4 synthesis on GL, CC and F1; spin adjoint identities;
and spin-3/4 analytic and adjoint regressions. T511 spin-1 DUCC synthesis
maximum absolute sample errors were `4.885e-15` (GL), `4.441e-15` (CC), and
`4.219e-15` (F1); analysis coefficient errors were at most `4.970e-18`. T2047 spin-1 GL DUCC middle-ring error was `2.137e-13`, and analytic map
error was `1.344e-11`. The complete checks passed: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo test --workspace --all-features` (144 Rust tests and 10 f32 tests; 8
ignored), `cargo test -p rustsht --no-default-features` (128 Rust tests and 10
f32 tests; 7 ignored), and `uv run pytest python/tests` (16 passed).

## Environment and reproducibility

CPU: x86-64 CPU. OS: Linux. Rust: `1.99.0`
(`b940084d7`, 2026-09-28); LLVM `23.1.1`. Release configuration:
`opt-level=3`, 16 codegen units, LTO off, Pulp portable runtime dispatch
(AVX2/FMA on this host), no `target-cpu=native`. Python: 3.11.15, NumPy
2.4.6, DUCC0 0.41.0. Full commands and all process samples are preserved in
the data directory. The native commands were:

```text
cargo build --release --example m1b_bench
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 511 1 synthesis GL STANDARD 512 1025 511 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 511 8 synthesis GL STANDARD 512 1025 511 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 720 1 synthesis GL STANDARD 721 1443 720 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 720 8 synthesis GL STANDARD 721 1443 720 requested-only
```

The baseline executable was built from `6066fa2`; the candidate executable
was built from the source change accompanying this report. Each command ran
in five alternating fresh-process pairs. The Python comparisons used
`examples/python_perf_baseline.py` with reused plan methods and matching
thread pinning:

```text
RAYON_NUM_THREADS=1 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" uv run python examples/python_perf_baseline.py 511 1 1 GL 2 plan synthesis both STANDARD
RAYON_NUM_THREADS=8 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" uv run python examples/python_perf_baseline.py 511 1 8 GL 2 plan synthesis both STANDARD
RAYON_NUM_THREADS=1 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" uv run python examples/python_perf_baseline.py 720 1 1 GL 2 plan synthesis both STANDARD
RAYON_NUM_THREADS=8 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" uv run python examples/python_perf_baseline.py 720 1 8 GL 2 plan synthesis both STANDARD
```

## Independent review and decision

The read-only Sol High reviewer examined the candidate diff, recurrence and
final-pair behavior, slice alignment, scalar tails, caller buffer lengths,
workspace ownership, and the matched benchmark evidence. It found no
correctness or maintainability blocker. Its review confirmed that all six
equal-length slices enumerate the same `0..pair_count` sequence, that cloned
iterators restart for each vector block and scalar tail, and that accumulation
and guarded recurrence updates preserve order. The review also advised
characterizing T720 q8 as modest/noisy, as done above. The optimization is
retained based on the repeatable q1 whole-transform reduction and supporting
T511 q8 results; the review made no source changes.
