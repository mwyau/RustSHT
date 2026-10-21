# Plan 14 — scalar-analysis workspace-base control

## Conclusion

The one-base dispatch candidate does not improve the warmed f64 scalar
analysis kernel consistently. At 64 and 128 active rings it is about 1.3%
and 0.9% faster than the current slice-based dispatch; at 256 rings it is
about 0.9% slower. This is not a production optimization. The production
`RecurrenceWorkspace` already has the contiguous aligned storage that made
the initial fragmented-buffer comparison look faster.

The corrected same-layout 256-lane Rust/DUCC ratio is `1.057x` for the
isolated kernel plus coefficient commit (`16.835` versus `15.914 µs`, paired
excess `0.910 µs`). These data supersede the earlier comparison of DUCC
against separately allocated Rust scratch vectors as a production-layout
estimate. They do not represent a complete transform.

## Method

Source baseline: `e9010c80faa8f964732ec8f8e4a96a712bbcb7fe`, plus the
test-only controls in this closeout. Rust 1.99.1 / LLVM 23.1.1 used release
mode, strict floating point, `-C target-cpu=x86-64-v3`, and runtime Pulp V3
AVX2/FMA. The host was an AMD Ryzen 9 5950X (16 cores / 32 logical CPUs).
Pinned DUCC was `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, built with GCC
15.2.0. The kernel used
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the driver used
`-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3`.

The operation was `m=1`, `lmax=720`, with 64, 128 or 256 active latitude
rings, fixed initial recurrence state, reset per operation, and raw paired
coefficient output. Rust times include staged-to-output coefficient commit;
DUCC accumulates directly into the corresponding output. The Python harness
calibrated each backend/variant and collected 60 batches across ten
interleaved process pairs per lane count. Each batch targeted 20–100 ms.
Rust checked and unchecked routes using independent `Vec`s were retained as
controls. The `fields` route uses the current slice-based dispatch with
workspace arrays in the exact same aligned contiguous storage as the
one-base `workspace` route. Numerical checks were outside timed batches.

## Timings

Values are median µs/op and batch p10–p90. `fields/DUCC` ratios and excesses
are paired across the ten process medians.

| Lanes | Rust current `fields` | Rust one-base `workspace` | DUCC | Current/DUCC | Current excess | Current/workspace |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 64 | 4.824 (4.796–4.963) | 4.757 (4.737–4.886) | 4.148 (4.103–4.298) | 1.162x | +0.674 µs | 1.013x |
| 128 | 8.852 (8.788–9.181) | 8.768 (8.709–8.885) | 8.077 (8.015–8.262) | 1.095x | +0.765 µs | 1.009x |
| 256 | 16.835 (16.716–17.020) | 16.969 (16.855–17.390) | 15.914 (15.760–16.722) | 1.057x | +0.910 µs | 0.991x |

The current `fields` route was 1.0%, 1.3% and 6.5% faster than the
fragmented-vector unchecked control at 64, 128 and 256 lanes, respectively.
That storage advantage is already present in production and is not credited
to the one-base candidate. Full per-variant medians, p10/p90 values, and
paired process ratios are in the
[machine-readable summary](data/plan14-workspace-base-control-2026-10-21/).

## Code and numerical checks

The executed Rust route printed `Arch::V3` at runtime. Both route bodies
execute four-f64 YMM lanes and the same two-degree recurrence group with ten
packed FMA instructions: two recurrence updates and eight real/imaginary
spectrum accumulations. The inner vector loop contains no YMM stack spills.
The one-base wrapper changes setup and inlining but leaves the recurrence
and accumulation instruction work intact.

Rust `fields` and `workspace` outputs matched exactly. Against DUCC, the
coefficient maximum scaled difference was `1.97e-16`; output relative L2
errors were `6.67e-14`, `1.63e-13` and `5.93e-14` for 64/128/256 lanes, with
maximum absolute errors `5.01e-12`, `1.90e-11` and `2.56e-11`.

The host showed low contention: median and maximum observed runqueue wait
were `0.038%` and `0.082%`. Frequency telemetry was collected, but included
idle/transition readings and was not used to normalize timings. No samples
were filtered. The full process log remains local and is intentionally not
committed.

Reproduce with:

```sh
DUCC_SRC="$DUCC_SRC_DIR" DUCC_GIT="$DUCC_SRC_DIR" CXX=g++ \
  python3 scripts/benchmark_scalar_analysis_kernel.py
```

## Disposition and next work

Do not integrate the workspace-base dispatch. Keep the current production
slice dispatch and contiguous workspace. The next Plan 14 task is the
remaining small-q1 fixed-cost attribution across T1–T64, followed by ranking
the largest absolute per-call excess before changing production code.
