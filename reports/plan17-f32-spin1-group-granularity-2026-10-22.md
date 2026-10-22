# Plan 17 f32 spin-1 synthesis group granularity (2026-10-22)

## Result

The static f32 spin-1 synthesis route grouped adjacent 64-row ring groups into
coarser Rayon tasks using `with_min_len(ceil(group_count / workers))`. A
benchmark-only `with_min_len(1)` candidate improved the measured q8 transforms
when there were more ring groups than workers. Production now uses one ring
group per stealable task only for f32 spin-1 synthesis with multiple workers
and `group_count > worker_count`. This leaves q1 unchanged and leaves cases
with no surplus groups on the old chunk size.

The current production binary was compared with the `9dea4143` baseline on
T720 GL q8 synthesis. Paired process-median candidate/baseline ratios were
`0.691`, `0.582`, and `0.740` (median `0.691`, about 31% lower latency).
There was visible host/timing variation: baseline process medians were
482–512 ms and candidate medians 298–356 ms. An earlier candidate screen was
also favorable in all three process pairs (`0.642–0.655`). At T1023 GL q8,
the same grouping candidate measured `0.890`, `0.894`, and `0.929` in the
three pairs, with exact Rust output equality. At T511, the existing policy
already selected `min_len=1` because there were eight ring groups for eight
workers; the A/B was neutral overall (median `1.006`, with one contended
baseline pair). These results support a work-count condition rather than a
bandlimit cutoff.

The current q8 implementation remains far from pinned DUCC in these f32
spin-1 synthesis cases: about `47–55x` at T720 and `56–59x` at T1023. The
scheduling correction improves Rust's utilization but does not close the
numerical-kernel gap. The next Plan 17 source target remains coefficient
visitor/spectral accumulation, which dominates recurrence arithmetic in the
test-only component sample.

## Route and implementation

`LATITUDE_BLOCK_SIZE` is 64. The observed exact GL ring-group counts were 8 at
T511, 12 at T720, and 16 at T1023. With q8, the old minimum chunk was 1, 2,
and 2 groups respectively. The production rule sets the minimum to one only
for f32 spin-1 synthesis when q is greater than one and the number of groups
exceeds q. Other spins, scalar transforms, analysis, f64, q1, and workloads
without surplus groups keep their previous task grain. The change neither
changes the recurrence nor allocates additional scratch.

For q1, the exploratory forced-`min_len=1` run did not show a reliable effect
(paired ratios `0.965`, `0.990`, `0.947`); production explicitly keeps the
previous serial grain. At T511 q8 the rule is also inactive because the
existing computed grain is already one.

## Numerical and phase evidence

All Rust candidate outputs were finite, repeat/reset consistent, and bitwise
identical to the baseline (`relative_l2=0`, `max_abs=0`). Pulp selected V3
(AVX2/FMA), with eight f32 lanes. Candidate-versus-DUCC relative L2 errors
were `1.426e-4` at T720, `5.881e-5` at T511, and `2.008e-4` at T1023. DUCC
uses a different public f32 numerical path; these values are cross-library
comparisons, not changes caused by task scheduling.

A temporary, test-only sampled component timer on T720 found q8 synthesis
worker-time estimates of about 1.25 s for coefficient visitor work, 96 ms
for recurrence steps, and 159 ms for normalization. The profile sampled one
degree per 64-degree segment and expands each sample by its segment length;
these are aggregate estimated worker times, not transform wall time. The
timer perturbs execution and its q8 samples varied substantially, so it is
used only to rank the next source target. Linux perf counters were unavailable
under the host's `perf_event_paranoid=4` setting; no system setting was
changed.

The focused Rust f32 integration suite passed in both default/parallel and
no-default-features builds: 11 passed, 0 failed, 1 ignored in each. The
ignored test is the opt-in T1023 comparison against an older DUCC fixture.
`cargo fmt --all --check` and strict all-target/all-feature Clippy also passed.

## Reproduction and provenance

- Rust baseline source: `9dea41434fe9c222ec4cc815629a48499dab91df`.
- Candidate source base: `656a952c7944d12562b6eb8f505d45bfc028441c` plus the
  production scheduling change in this report.
- Pinned DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust: 1.99.1, LLVM 23.1.1; release defaults, runtime Pulp V3 dispatch.
- DUCC SHT: `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; strict
  native driver.
- Host: AMD Ryzen 9 5950X. q1 was pinned to one physical core; q8 was pinned
  to eight physical cores within one L3 domain. The repository's artifact
  policy omits exact host CPU indices.
- Long operations used three interleaved process pairs and five individually
  timed Rust calls per process. Plans/workspaces were reused; first-call,
  warmup, output finite, repeat, and reset fields remain in the source JSON.

The concise distributions and binary hashes are in
[`data/plan17-f32-spin1-group-granularity-2026-10-22/`](data/plan17-f32-spin1-group-granularity-2026-10-22/).
The reusable benchmark entry point is
[`scripts/benchmark_plan17_spin_recurrence_inline.py`](../scripts/benchmark_plan17_spin_recurrence_inline.py).
