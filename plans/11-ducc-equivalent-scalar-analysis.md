# Active plan 11 — Scalar f64 GL analysis parity with DUCC

**Status: COMPLETE (2026-10-21); closeout commit
[`a37f74d6`](../reports/scalar-analysis-cumulative-2026-10-20.md#plan-11-reduced-mmax-frontend-closeout-2026-10-21).**
The reduced-`mmax` frontend was attributed, validated and closed on
`opt/sht-performance-parity`. Plan 14 is now the active successor. Read
[AGENTS.md](../AGENTS.md), the current code and
[PERFORMANCE_STATUS.md](../PERFORMANCE_STATUS.md) before changes.
The [full preceding Plan 11](../reports/archive/plans/11-ducc-equivalent-scalar-analysis-pre-refresh-2026-10-21.md)
is preserved as a historical snapshot, **not** a second implementation queue.

## Objective and boundaries

Reduce **measured, matched, whole-transform** RustSHT-versus-DUCC scalar
f64 Gauss–Legendre STANDARD **analysis** latency without numerical,
memory-safety, API, or cold-start regressions. Identify and remedy the
dominant remaining bottleneck rather than declaring success after isolated
small kernel changes. The earlier generalized “about 2× gap” was a
historical characterization, **not a universal current multiplier or
acceptance threshold**: at T2047, ntheta=2048, q1 is essentially at DUCC
parity, while the one-latitude cutoff causes a severe regression.

Keep current GL/CC/F1, scalar/spin, f32/f64, S/A/S*/A* and derivative
behavior intact. Spin-1 tuning, FFT redesign, new batch kernels, cache
autotune, new dependencies and general API changes are outside this task.

**Pinned comparison:** Rust 1.99.1 / LLVM 23.1.1, Pulp V3 AVX2/FMA,
`-C target-cpu=x86-64-v3` with strict Rust FP; DUCC 0.42.0 commit
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2 native
SHT compiled `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`,
strict validation driver. Report actual flags and code generation. Do not
alter numerical settings merely to equalize source-language flags.
Historical Rust 1.99.0 / DUCC `80388904` measurements remain separately
identified.

## Established results — use these, do not rerun finished campaigns

1. **The T2047/T2048 prefix discontinuity is resolved in `0bc5f153`.**
   With `mmax=2047`, `nphi=4096`, and tile 256, the prior T2047/2048-latitude
   baseline was 251.5 ms Rust vs 248.6 ms DUCC (q1), while changing only
   latitude count to 2049 raised it to 828.4 vs 249.4 ms. Removing only the
   latitude-count gate gives 254.6 vs 245.7 ms at T2047/2049 (q1), 57.3 vs
   42.1 ms (q8), 255.4 vs 248.3 ms at T2048/2049 (q1), and 58.3 vs 42.2 ms
   (q8). The former 3.1–3.3x slowdown is gone; the remaining q8 excess is
   1.36–1.38x. The route count changes from 2,434 checked-scaled blocks to
   2,434 prefix-enabled blocks with zero declines. See the
   [follow-up report](../reports/scalar-analysis-cumulative-2026-10-20.md),
   [after data](../reports/data/scalar-analysis-cumulative-2026-10-20/t2048-ducc-fixed-ftol/)
   and [before data](../reports/data/scalar-analysis-cumulative-2026-10-20/t2048-cutoff-current/).
2. **The extra gate was a conservative aggregate-error budget, not a
   finite/exponent safety invariant.** It bounded the worst-case coherent
   sum of insignificant-prefix omissions. DUCC's `init_lambda`,
   `iter_to_ieee`, `rescale` and `calc_map2alm` use fixed
   `sht_ftol=2^-60` without a latitude-count multiplier. The Rust change
   preserves independent finite/range checks, scale/exponent validation and
   safe fallback. T2047/T2048 q1/q8 and T3072 full-order cases passed DUCC,
   polar/high-order basis, zero, subnormal, extreme-finite, antisymmetric
   cancellation and adjoint checks. Even-degree zonal cancellation
   coefficients were exactly zero in both libraries. This is extensive
   empirical coverage, not a claim of bitwise identity for all inputs.
3. **The 256-tile policy remains settled after fixing the prefix route.**
   At T2047/2049, tile 256 beat 128 and 64 at q1 (254.6 / 263.5 / 280.7 ms)
   and q8 (57.3 / 59.2 / 61.9 ms); T2048/2049 showed the same ordering.
   **Keep production tile 256.** The one post-fix 64/128 comparison was
   diagnostic; do not repeat the tile sweep or promote smaller tiles. An
   isolated 512 experiment may follow only for a separate, evidence-based
   setup/cache hypothesis that justifies increasing workspace capacity.
4. **T720 A/B closed several experiments.** Packed-complex reduction,
   indexed SIMD loop and direct-output accumulation candidates were
   removed after absent/inconsistent full-transform gains or severe
   regressions. Do **not** restart unchanged ablations. Fixed-size,
   aligned SIMD scratch remains **experimental**: matched T720 q8
   synthesis improved **11.4–18.0%** over four tested geometries;
   analysis was near-neutral (roughly -2.8% to +0.2% alone), q1
   synthesis near-neutral. Why an analysis scratch-layout build flag
   affects synthesis is **unresolved**; evaluate mechanism, workspace
   memory and broader shapes before promotion.
5. **Small recurrence-loop changes do not explain the whole gap.**
   The corrected warmed T720 `m=1` kernel is competitive with DUCC
   under one matched GCC build, but compiler-dependent kernel
   comparisons were affected by contention and different code
   generation. Do not mix reports or treat kernel-only wins as
   full-transform wins.
6. **T64 Legendre selection already decided:** f64 scalar synthesis
   chooses recurrence starting at T64, while analysis and spin-1
   retain their own table policies. One universal T64 cutoff
   regressed q1 analysis. Defer cache-informed dispatch to Plan 12.
7. **q8 order preparation is parallelized and measured** in
   `dfed3ff7`/`3b32f2de`. The fixed 256-pair recurrence and the table-route
   safety preflight are unchanged. At full `mmax`, current Rust/DUCC median
   q8 ratios are 1.067x at T720, 1.003x at T2047 and 1.042x at T3072;
   T2047 and T3072 Rust whole-call latency fell 25.6% and 14.9% from the
   pre-change screen. Reduced-`mmax` ratios are 1.228x at T2047 m511 and
   1.122x at T3072 m768. Stage profiles show their recurrence near DUCC and
   locate the next investigation before recurrence. Details and distributions
   are in the [cumulative report](../reports/scalar-analysis-cumulative-2026-10-20.md#q8-independent-order-preparation-candidate-2026-10-20).
8. **Reduced-`mmax` FFT output pruning is validated** in current source
   commit `1fddce4a` (source-equivalent pre-rebase hash `41c7223f`).
   The full real FFT and safe input scan remain; only recombination/stores of
   bins above `mmax` are skipped for scalar GL analysis. q8 T2047 m511
   improved 12.4% (21.399 to 18.739 ms), and T3072 m768 improved 7.8%
   (64.306 to 59.266 ms). T720 reduced mmax improved 6.5%; full-order changes
   were within the observed spread. q8 output coefficients were bitwise
   identical before/after and q1 shows no broad regression. See the
   [comparison](../reports/scalar-analysis-cumulative-2026-10-20.md#reduced-order-real-fft-output-pruning-2026-10-20).

## Immediate work — priority order

### A. Prefix-route experiment — complete

The source and benchmark milestones are complete at `0bc5f153`. The
production path uses fixed `sht_ftol=2^-60` for every nonempty latitude
geometry. The T2047/T2048 fixed-grid q1/q8 matrix and the T3072 full-order q1
check passed with tile 256; T3072 records 5,780 prefix-scaled blocks, zero
checked-scaled blocks and zero initialization/recurrence declines. Numerical
errors, distributions and edge-case outcomes are in the report and data links
above. Do not repeat the cutoff experiment or broaden it into another tile
campaign.

### B. Reduced-order q8 front end — complete

The recurrence-plan order-preparation and truncated-spectrum candidates are
complete; do not repeat their comparisons. The remaining reduced-`mmax`
production-path input finite-check/copy, row FFT, output recombination and
spectrum/weight packing were profiled at T2047 m511 and T3072 m768. DUCC
`map2leg` combines FFT and ring weighting, while its `leg2alm` includes
packing and recurrence. Preserve robust input checks and the 256-pair tile.
The profile did not justify a wholesale FFT rewrite.

The final Rust split used temporary test-only timers at the current V3 build:
q1/q8 input scan and complex-workspace copy, complex FFT, output recombination,
order-spectrum preparation and Legendre recurrence. Its forward wall stage
was below DUCC `map2leg` in both geometries at both thread counts. Rust's full
finite/headroom scan is required and already fused with FFT packing. DUCC's
`leg2alm` includes order packing plus recurrence, while Rust reports those as
separate phases; stage timings are diagnostic rather than subtractable
hardware counters. After output pruning, the calibrated q8 whole-transform
excess is +1.675 ms at T2047 m511 and +1.013 ms at T3072 m768; paired ratio
ranges overlap unity. The q1 T3072 candidate is faster than DUCC in the
matched whole-transform screen. No further frontend change has demonstrated
a material benefit, so the finite scan remains and no FFT rewrite is proposed.
See the [closeout profile and interpretation](../reports/scalar-analysis-cumulative-2026-10-20.md#plan-11-reduced-mmax-frontend-closeout-2026-10-21)
and [stage/whole-call data](../reports/data/scalar-analysis-cumulative-2026-10-20/reduced-frontend-attribution-2026-10-21.csv).

### C. Reduced-order frontend closeout and handoff — complete

The final frontend attribution, validation, cleanup and closeout are
recorded in the
[closeout report](../reports/scalar-analysis-cumulative-2026-10-20.md#plan-11-reduced-mmax-frontend-closeout-2026-10-21)
and machine-readable
[stage/whole-call data](../reports/data/scalar-analysis-cumulative-2026-10-20/reduced-frontend-attribution-2026-10-21.csv).
The validated scalar GL analysis FFT-output pruning is in source commit
`1fddce4a`; full FFT input checking remains. No further front-end change
was supported by the measured whole-transform results. The distinct Plan 11
closeout was committed and pushed as
`a37f74d68631ab5fd373a4097640895e9f0c0c23`.

The authorized same-session, same-branch handoff is complete: Plan 14 is
ACTIVE, and Plans 12/13 remain deferred. Plan 14's initial rebaseline must
use the final Plan 11 source at the closeout commit, revise stale
reduced-`mmax` claims, and proceed with its small-q1 and large-q8 evidence
stages. Do not create `opt/sht-small-serial-parallel-parity` or switch to
`main`.

## Tile definition and future 512 experiment (not active)

The scalar pair recurrence loops over **reflected north/south
latitude-pair positions** for each harmonic order `m`. A 256 tile
covers at most 256 pairs (up to 512 physical latitudes); T2047/
ntheta=2048 has 1,024 pairs and up to four tiles per fully active
order. Tile size is not an FFT length, number of harmonic degrees,
number of thread workers or future batch dimension. Larger tiles
reduce initialization, repeated coefficient updates and block-count
overheads but enlarge recurrence, spectrum and compacted-index
scratch; neither cache occupancy nor speed changes monotonically.
DUCC's `nv0=256/VLEN` is **256 latitude entries**, not
necessarily an equivalent reflected-pair unit.

A 512 candidate exceeds current fixed scratch
`ANALYSIS_VECTOR_BUFFER_CAPACITY=SCALAR_ORDER_BATCH_SIZE=256`
in `src/workspace.rs`; it requires coordinated, safe
allocation/capacity changes, range-cache indexing and tail checks.
**Do not use this issue as motivation for recurring 64/128 sweeps.**

## Benchmark and repository rules

- Benchmark only comparable actual executed paths, pinned source and
  compiler/ISA flags, equivalent geometry, useful harmonic work,
  first-call versus warmed calls and real threaded execution. Interleave
  competitors; for short operations calibrate ~20–100 ms repeated
  batches and at least 30 samples across independent processes.
  Long operations may use individually timed whole calls with
  explicitly stated sample counts. Keep host load and q8 noise
  visible; never manufacture a speedup under contention.
- Run relevant release tests, no-default-feature tests, strict
  Clippy/formatting, DUCC comparisons and adjoint/numerical tests.
  Do not push a known numerical regression.
- **Before each push:** inspect worktree; fetch `origin`, rebase
  **unpublished** commits onto `origin/opt/sht-performance-parity`
  only when safe; reread this plan, AGENTS and owner instructions after
  integrating newer remote changes; retest affected paths, push
  normally and verify remote HEAD. Never force-push, delete work,
  overwrite a concurrent change or update `main` without approval.
  At most one Luna benchmark-only subagent; no Sol reviewer
  unless expressly authorized. Keep raw host/process logs and
  disassembly local unless reduced reproducible evidence is needed.

## Follow-on, deferred and historical references

- **Active successor:**
  [Plan 14 — small serial and large parallel performance](14-small-serial-parallel-performance.md),
  on the same session and branch; the Plan 11 closeout SHA is recorded above.
- [Plan 12: offline-calibrated cache-informed Legendre dispatch](12-cache-informed-legendre-dispatch.md) — **future only**; one-time
  hardware cache query, optional diagnostic pointer-chase, no
  startup transform timing.
- [Plan 13: true fused batch transforms](13-fused-batched-sht.md)
  — **future only**; internal blocked B=1/2/4/8/16/32 study,
  distinct from independent calls; batch-aware selection later.
- [Numerical specification](02-numerics-and-grids.md),
  [DUCC contracts](06-ducc-contract-crosswalk.md) and
  [license/provenance](01-ducc-provenance-and-license.md)
  — references, not work queues.
- [Current status](../PERFORMANCE_STATUS.md),
  [recent evidence index](../reports/README.md),
  [archived previous Plan 11](../reports/archive/plans/11-ducc-equivalent-scalar-analysis-pre-refresh-2026-10-21.md)
  and [historical 80-report catalog](../reports/PERFORMANCE_FINDINGS.md).
