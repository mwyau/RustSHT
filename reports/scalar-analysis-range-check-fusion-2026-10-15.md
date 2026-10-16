# Scalar analysis range checks now run with the recurrence update

The fused f64 scalar analysis block checks each newly computed recurrence
value before using it in the next degree. This removes the separate pass that
previously reread both recurrence buffers after every update. The recurrence
equations and their accumulation order are unchanged. If any lane is outside
the normal range, the block declines and the caller discards its staged
coefficients and reruns the established scaled path.

## Paired RustSHT comparison

Five alternating release-process pairs compared candidate source with commit
`740105b`. Each process ran the reused-plan/workspace f64 scalar GL STANDARD
analysis benchmark, one warmup and five timed calls, at full `mmax` for T511
(`512 x 1025`) and T1023 (`1024 x 2049`). q1 used one CPU core and q8 used 8 pinned CPU cores. The shared x86-64 CPU host allowed 32 pinned CPU cores; no CPU quota applied. Rust was `1.99.0`, LLVM `23.1.1`, with portable Pulp runtime dispatch (AVX2/FMA
on this host), and no `target-cpu=native`. Negative paired changes favor the candidate. The value is the median of the
five within-pair percentage changes; ranges show the minimum and maximum pair.

| Grid | Threads | Baseline median | Candidate median | Paired change median (range) |
| --- | ---: | ---: | ---: | ---: |
| T511 | q1 | 19.916 ms | 18.720 ms | -6.16% (-6.45% to -5.57%) |
| T511 | q8 | 3.521 ms | 3.437 ms | -3.05% (-5.53% to +12.82%) |
| T1023 | q1 | 154.911 ms | 146.907 ms | -5.10% (-5.52% to -4.17%) |
| T1023 | q8 | 25.589 ms | 24.910 ms | -3.21% (-17.58% to +5.28%) |

Every q1 pair favored the candidate at both sizes. The q8 medians were lower,
but both paired ranges cross zero, so the measurements do not establish a q8
gain. At T1023, median peak process RSS was 102,364/102,224 KiB at q1 and
145,864/145,496 KiB at q8 (baseline/candidate). The code adds no workspace
storage. The raw per-call samples, process logs, hashes, affinity and load
records are in [`the paired data`](data/scalar-analysis-range-check-fusion-2026-10-15/).

## Current T1023 comparison with DUCC

Five alternating native RustSHT/Python DUCC process pairs measured the same
full-band T1023 GL STANDARD analysis. RustSHT used the native reused-plan
benchmark; DUCC used `ducc0==0.41.0` through `analysis_2d`, from source revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. q1 was pinned to one CPU core and q8 to
8 pinned CPU cores. Each native process ran one warmup and five timed calls. DUCC's
inner benchmark reports two run medians per process. These are shared-host
measurements, not isolated-core results.

| Threads | RustSHT median (range) | DUCC median (range) | Paired ratio median (range) |
| ---: | ---: | ---: | ---: |
| q1 | 147.459 ms (146.890–148.590) | 118.630 ms (116.984–121.217) | 1.25x (1.21–1.26x) |
| q8 | 24.405 ms (24.077–24.691) | 15.697 ms (15.603–17.052) | 1.56x (1.43–1.58x) |

The comparison describes the candidate's current gap at this grid and setup;
it does not isolate the change's effect on the RustSHT/DUCC ratio. Raw logs,
process resource records, commands, compiler and CPU details are in
[`the DUCC comparison data`](data/scalar-analysis-range-check-fusion-2026-10-15/ducc-comparison/).

## Validation

The focused late-range-failure test exercises both vector lanes and a scalar
tail after earlier coefficients have been staged. The caller adds staged
coefficients only when the block succeeds, so a declined block cannot leak
partial results. The release high-bandlimit scalar check passes for T511,
T1023 and T2047 on GL, CC and F1 against DUCC and closed-form values. ```text
cargo test --workspace --all-features
  186 unit tests passed; 12 ignored
  11 f32 integration tests passed; doctests passed (0 tests)

cargo test -p rustsht --no-default-features
  148 unit tests passed; 7 ignored
  11 f32 integration tests passed; doctests passed (0 tests)

cargo clippy --workspace --all-targets --all-features -- -D warnings
  passed

cargo fmt --all -- --check
  passed
```

Python bindings did not change, so Python tests were not run. The paired
measurements cover f64 scalar GL STANDARD analysis only; spin-1 performance,
other grids and thread counts remain unmeasured for this change.
