# Active plan 11 — Scalar f64 GL analysis parity with DUCC

**Status: ACTIVE. Updated 2026-10-21 (UTC).** Only this plan authorizes
current optimization on `opt/sht-performance-parity`. Read
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

1. **T2047 abrupt cutoff is primarily a route change, not added
   harmonic-degree cost.** With `mmax=2047`, `nphi=4096` fixed,
   T2047/ntheta=2048 is 251.5 ms Rust vs 248.6 ms DUCC (q1)
   and 62.2 vs 45.8 ms (q8), using **256 reflected-pair tiles**.
   Changing only ntheta to 2049 is 828.4 vs 249.4 ms (q1) and
   143.7 vs 45.9 ms (q8). At fixed ntheta=2049, T2048 adds
   essentially no cost over T2047. Measured routes change from
   2,432 prefix-enabled blocks at 2048 latitudes to 2,434
   checked-scaled blocks at 2049. This is a demonstrated association
   with the dominant slowdown; verify causality with a one-change
   controlled ablation, not an assumption about CPU cache or tiles.
   [Report](../reports/scalar-analysis-cumulative-2026-10-20.md)
   · [original measured data](../reports/data/scalar-analysis-cumulative-2026-10-20/t2048-cutoff-current/).
2. **Rust-only global prefix gate.** The current code requires
   `latitude_count * ANALYSIS_FTOL <= 8 * f64::EPSILON`, with
   `ANALYSIS_FTOL=2^-60`, which passes at 2048 and fails at
   2049. Pinned DUCC `iter_to_ieee`/`rescale` uses
   `sht_ftol=2^-60` and does **not** impose the global latitude-count
   gate. The gate is an additional RustSHT accuracy policy, not a
   stated requirement of DUCC. Whether it protects a distinct,
   *needed* invariant still requires direct proof and adversarial tests;
   do not declare removing it safe solely from matching DUCC.
3. **256-tile policy is settled for now.** In the fast-prefix
   T2047/2048-latitude configuration 256 beats 128 and 64 at q1
   (251.5 / 261.9 / 282.1 ms) and q8
   (62.2 / 63.1 / 67.7 ms). In the *abnormally slow*
   prefix-disabled 2049-latitude route, 64 recovers only about
   14–17% and remains ~2.65–2.79× slower than DUCC. **Do not
   repeatedly benchmark, restore, recommend, or globally select 64/128
   tiles** to mask the prefix issue. Keep production 256; retain
   past smaller-tile data as diagnostic history. An isolated **512**
   experiment may follow *after* fixing the cutoff if a specific
   performance hypothesis warrants it, not as a prerequisite.
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

## Immediate work — priority order

### A. Resolve the T2048 insignificant-prefix discontinuity

1. Compare the **executed** Rust scale-epoch initialization,
   prefix-skipping, mantissa validity, `rescale` and transition-to-
   ordinary path against pinned DUCC's `init_lambda`,
   `iter_to_ieee`, `calc_map2alm` and `map2alm_kernel`.
   Identify any actual invariant supplied by RustSHT's **global**
   eight-ULP gate beyond existing finite/range/fallback checks.
2. In a controlled experimental build, remove **only the extra
   latitude-count gate**, retaining `sht_ftol=2^-60`, all scale
   transitions, exponent safety, checked finite/nonfinite behavior
   and transactional output handling. Do not simultaneously change
   the tile, FFT, compiler, weights or fallback. If it is unsafe,
   isolate a counterexample and derive the *smallest* principled
   alternative (for example a more precise omission bound or
   selective guard); do not invent a new blanket cutoff.
3. Establish correctness against DUCC, analytic coefficients,
   independent adjoints, weighted cancellation/near-zero modes,
   high-order polar tails, subnormal/extreme-finite/nonfinite
   inputs, SIMD tails, repeated workspaces and first-call behavior.
   Report both **absolute** and relative errors, and assess
   coefficient-wise cancellation. Do not silently relax tolerances
   or eliminate necessary safety checks.
4. Run identical full f64 GL `mmax=2047, nphi=4096` T2047/
   ntheta=2048, T2047/ntheta=2049 and T2048/ntheta=2049,
   q1/q8 on **the unchanged 256 tile**. Collect before/after
   time and prefix/checked-scaled/ordinary/fallback route counts
   with comparable warm/cold conditions. Check representative
   T720 and T3072 to prevent shifting the discontinuity elsewhere.
   Compare to pinned DUCC and preserve all timing distributions.

### B. Profile remaining q8 and validated synthesis candidate

Only after A produces an evidence-based result, profile the
remaining q8 gap at representative T720/T2047/T3072, separating
Fourier, packing/weights, ordinary Legendre work, scaled-prefix
work, scheduling, memory pressure and output. Prioritize the
**largest measured** residual. q8 must use the same physical
core/affinity conventions as its reference.

The optional fixed-scratch candidate may be examined independently,
using source/assembly/worker-layout inspection, peak workspace and
stack/heap/RSS data, plus controlled q1/q8 A/S runs. Promotion
requires a reproduced whole-transform advantage across relevant
shapes, satisfactory memory cost and all correctness tests. It
must not distract from A or trigger another accumulation of cfgs.

### C. Close out the active task

Document measured before/after results and any residual cause with
reproducible commands and data. Remove failed production experiments
and unnecessary test-only flags; preserve evidence in the archive.
If parity cannot be reached, document the actual bottleneck and a
specific *tested* next design, not a generic “still 2× slower.”

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

## Deferred and historical references

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
