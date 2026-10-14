# Spin-1 paired coefficient preparation experiment

## Investigation plan

The existing q8 GL spin-1 STANDARD synthesis cache computes the two scalar
coefficient streams once per order. Each latitude group then copies those
rows into its worker scratch and applies `prepare_paired_scalar_synthesis_coefficients`
to both channels before the fused paired recurrence. At T511 this preparation
is repeated across 16 groups. This may duplicate coefficient arithmetic and
row traffic after the earlier scalar-conversion cache.

The initial hypothesis was that each latitude group repeats paired recurrence
coefficient preparation for both scalar-conversion channels. Five-process
native baselines at T511/T720 q8 and T511 Callgrind attribution were collected
before production edits. Pair preparation contributes only `670,656` self
instruction references (`0.0083%` of the instrumented benchmark), so a
second persistent cache for those values is not justified. This candidate is
rejected without a source change.

The next hypothesis is memory traffic: the current scalar cache copies two
complete coefficient rows into each group's worker scratch. Source dimensions
imply up to 128.2 MiB copied per T511 q8 transform and 190.7 MiB at T720.
A temporary fast path read cached rows directly for fully active, ordinary
paired blocks. Polar, cutoff, table, and scaled blocks kept the copy-and-fallback
path. The prototype added no persistent memory or coefficient arithmetic.

## Baseline

The release baseline is source commit `726750e`, Rust `1.99.0`, `opt-level=3`,
16 codegen units, LTO off and portable Pulp AVX2/FMA dispatch on an AMD Ryzen 9
5950X under Linux `7.0.14-20-pve`. It used exact GL f64 grids, full `mmax`,
`phi0=0.37`, reused plan/workspace, one warm-up and five timed transforms per
process. Five fresh q8 processes gave T511 median `19.119 ms` (range
`18.269–20.259 ms`) and T720 median `47.162 ms` (range `46.342–48.210 ms`).
Three q1 process medians were `57.241 ms` at T511 and `133.151 ms` at T720;
the q1 route does not build or read the parallel coefficient cache. Exact
samples are in [`baseline-process-samples.txt`](data/spin1-paired-coefficient-preparation-2026-10-14/baseline-process-samples.txt).

Callgrind 3.26.0 with release debug symbols counted `8,116,852,263`
instruction references across five q8 T511 samples. The AVX2
`FusedPreparedSynthesisPairBlock` accounted for `1,457,064,728` self
references (`17.95%`); paired coefficient preparation accounted for only
`670,656` (`0.0083%`). This rejects precomputing the paired coefficients as a
separate cache. The profile is not an elapsed-time estimate and does not
measure hardware cache traffic. The estimated `128.2/190.7 MiB` row-copy
traffic at T511/T720 is a separate hypothesis to test with whole-transform
timings.

## Candidate measurements

The final candidate moved the `lmax >= 640` decision out of the latitude-block
loop. Below T640 it copied cached rows into worker scratch once per order. At
T640 and above it passed cached rows directly to eligible blocks, copying once
only when a block used the general fallback. The measured direct-read cases
were q8 T720 GL spin-1 STANDARD synthesis; q8 T511 exercised the copy path as
a control. Each process used one warm-up, five timed transforms, and a reused
plan and workspace. Ten q8 process pairs at T720 and five at T511 alternated
which binary ran first. Runs were pinned to CPUs 0–7.

| Workload | Baseline process-median median (range) | Candidate median (range) | Paired candidate change | Candidate faster |
| --- | ---: | ---: | ---: | ---: |
| T720 q8, direct reads eligible | `46.453 ms` (`44.603–46.887`) | `46.072 ms` (`44.694–47.366`) | `-0.42%` median (`-3.96%` to `+6.19%`) | `6/10` |
| T511 q8, direct reads disabled | `20.797 ms` (`20.459–21.201`) | `20.980 ms` (`18.993–22.127`) | `+1.65%` median (`-10.41%` to `+8.15%`) | `2/5` |

The T720 whole-transform change is within process variability. The T511 control
also shows no repeatable shared-path effect. Earlier non-alternating screens
had a larger apparent T720 gain, but the alternating runs did not reproduce
it. The source changes were reverted; the compact cached-row copy path remains
the production implementation. The direct-read prototype is rejected because
it did not establish a reproducible workload-level gain.

The estimated `128.2 MiB` at T511 and `190.7 MiB` at T720 count two complex
rows copied for each order and latitude group. They describe source-level copy
volume, not measured DRAM traffic. Direct reads added no retained table or
workspace memory. The paired coefficient preparation remains a poor cache
target: its Callgrind self references were `0.0083%` of the T511 q8 benchmark.

The direct block helper produced bitwise-identical Fourier coefficients to the
existing worker-copy path for the tested T720 mid-latitude block. The cached
and uncached T127 transform comparison had maximum component errors
`5.463e-19` and `9.216e-19`. The release T511 spin-1/2 DUCC and round-trip test
passed on GL, CC, and F1; spin-1 GL synthesis had maximum sample error
`4.885e-15`, and analysis had maximum coefficient error `4.970e-18`. A prior
T720 Python-plan comparison using the same direct block arithmetic had
relative-L2 error `9.123e-14` against installed `ducc0==0.41.0`; it measured
Python-facing throughput, not the native before/after change. That comparison
gave RustSHT/DUCC medians of `139.969/53.662 ms` at q1 and `51.609/7.121 ms`
at q8.

The CPU was an AMD Ryzen 9 5950X under Linux `7.0.14-20-pve`; Rust was
`1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`, release `opt-level=3`, 16
codegen units, LTO off, portable Pulp AVX2/FMA dispatch, and no
`target-cpu=native`. The native command was:

```text
cargo build --release --example m1b_bench
taskset -c 0-7 target/release/examples/m1b_bench 1 720 8 synthesis GL STANDARD 721 1443 720 requested-only
taskset -c 0-7 target/release/examples/m1b_bench 1 511 8 synthesis GL STANDARD 512 1025 511 requested-only
```

The same commands ran against the baseline executable built from `726750e`.
Raw alternating process medians and within-process samples are in
[`final-alternating-process-samples.txt`](data/spin1-paired-coefficient-preparation-2026-10-14/final-alternating-process-samples.txt).
The earlier baseline samples and Callgrind attribution are in the same data
directory. Hardware counters were unavailable (`perf_event_paranoid=4`), so no
instruction-to-cycle claim is made.
