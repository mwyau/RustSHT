# Follow-on plan 14 — Small-transform latency and large parallel SHT parity

**Status: ACTIVE (2026-10-21), following Plan 11 closeout
`a37f74d68631ab5fd373a4097640895e9f0c0c23`.** Plan 11 is complete and
retained as historical evidence. Work continues on the same Luna session
and `opt/sht-performance-parity` branch. The transition refreshed the
historical reduced-`mmax` claims below; now rebaseline the Plan 14 target
cases from this final source and continue its staged evidence-driven work.
Plans 12/13 remain future-only.

**One-line goal:** Identify and reduce the dominant, measured sources of (1) small **serial** SHT latency and (2) residual large **eight-thread** SHT latency relative to a pinned native DUCC implementation, without sacrificing numerical correctness, portability, or memory safety.

## 1. Boundaries and relationship to existing plans

- This plan covers reusable-plan, single-field CPU spherical harmonic transforms, principally **f64 scalar Gauss–Legendre STANDARD analysis and synthesis**. Record both operations separately. Add a small spin-1 control set only after a shared hotspot is identified; spin-1 kernel redesign is a separate task unless directly justified.
- **The T2047/T2048 prefix discontinuity is already fixed by `0bc5f153`.**
  The fixed DUCC `2^-60` threshold is retained, and q1/q8,
  high-order/polar, cancellation, exceptional-value and adjoint checks
  passed. Do not reopen its removed global latitude-count gate or repeat
  64/128 tile sweeps. Keep **256 reflected-pair tiles** as the production
  baseline. Plan 11's already-parallelized q8 order preparation
  (`dfed3ff7`/`3b32f2de`) is also accepted source baseline; use the
  **final post-Plan-11 commit**, not historical comparisons, for Plan 14.
- **Defer cache-topology-driven table/recurrence selection to Plan 12.** The existing operation-specific scalar table selector, T64 synthesis recurrence policy, finite-input fallback, and memory cap are the baseline, not errors to be simplified away. This plan may measure their effects but must not implement Plan 12's hardware discovery or autotuning.
- **Defer shared-Legendre multi-field batch kernels and batch API changes to Plan 13.** A spin-1 pair of components is not an independent field batch.
- No general FFT replacement, new dependency, broad API redesign, default fast-math, changed numerical conventions, or unbounded allocation is authorized by this plan. A bounded FFT improvement is possible **only after matching stage attribution proves it is responsible for a material share of the remaining whole-SHT gap**.
- The goal is performance attributable to an implementation difference, **not** a predetermined speedup percentage or a claim that Rust language overhead is inherently slower.

## 2. Grounded starting evidence — not final results

Final Plan 11 source and closeout (commit
`a37f74d68631ab5fd373a4097640895e9f0c0c23`, 2026-10-21) establish:

- At small bands T1–T32, historical matched warm serial scalar transforms had Rust/DUCC geometric-mean ratios of approximately **3.2× for analysis and 3.1× for synthesis**. This is evidence of a latency problem; it does **not** establish that FFT, dispatch, or any one per-call step causes it. The scalar compact Legendre table is **not eligible below T63**, so the T1–T32 excess must not be blamed on a table/recurrence switch without other evidence.
- **Final reduced-order q8 values after output pruning:** T720 m180
  **1.728 vs 2.002 ms (0.863×)**, T2047 m511 **18.739 vs 17.064 ms
  (1.098×)**, and T3072 m768 **59.266 vs 58.253 ms (1.017×)** (Rust vs
  DUCC). The earlier 1.228×/1.122× values predate output pruning and are
  superseded. Full-order q8 analysis in the same checkpoint was T720
  **3.063 vs 2.937 ms (1.043×)**, T2047 **42.907 vs 42.563 ms (1.008×)**,
  and T3072 **124.428 vs 117.119 ms (1.062×)**. Reduced T2047/T3072 paired
  ratio ranges overlap unity; q1 reduced-order values were T720
  **11.196 vs 13.910 ms**, T2047 **122.010 vs 119.737 ms**, and T3072
  **380.152 vs 384.761 ms**. These final Plan 11 values replace the
  pre-pruning table above; Plan 14 must remeasure its selected cases under
  its own interleaved baseline protocol.
- The final-source stage profile found Rust's forward input scan/copy,
  FFT and recombination did not account for an excess over DUCC `map2leg`;
  Rust q8 recurrence plus separate preparation was comparable with DUCC's
  combined `leg2alm` stage. The remaining reduced-order q8 whole-call
  differences were +1.675 ms at T2047 and +1.013 ms at T3072, with paired
  ranges overlapping unity. The finite-input scan is part of the numerical
  contract and remains. Plan 14 should investigate its selected small-q1
  and large-q8 total/stage costs without repeating the closed frontend work.
- DUCC's author notes that the *DUCC* implementation has multiple parallel regions with microsecond-scale overhead, which hurts small **parallel** transforms. This explains why Rust can occasionally win small q8 cases; **it does not explain why Rust is slower for small q1**.
- RustSHT's scalar analysis constructs order-major longitude spectra.
  Independent per-order **preparation is already parallel** for recurrence
  plans using Rayon and disjoint output rows; do not propose to parallelize
  it again. Legendre jobs still use `Mutex<VecDeque>` dynamic
  scheduling; its cost is **not yet established as a meaningful q8
  bottleneck**. Investigate the execution profile, not the source
  appearance, before modifying scheduling.
- DUCC's `sht.cc` performs `map2leg` and `leg2alm_internal` with its own geometry/packing and `execDynamic` scheduling; `sht_inner_loop.h` uses `fill_m2a`, `iter_to_ieee`, and `map2alm_kernel`. DUCC's output layout can differ by synthesis/analysis and thread count (`allocate_leg`). Compare actual data movement and SIMD useful work, not just surface API names.
- Some historical q8 synthesis profiles localized excess in Legendre/order work rather than FFT, and an **experimental** fixed-size aligned scratch variant improved selected T720 q8 synthesis by 11.4–18.0% without a comparable consistent analysis improvement. Its mechanism and generality remain unproven. Do not promote it on that result alone.

**Source checkpoints:** `PERFORMANCE_STATUS.md`; `reports/scalar-t64-recurrence-transition-2026-10-20.md`; `reports/scalar-analysis-cumulative-2026-10-20.md`; `reports/PERFORMANCE_FINDINGS.md`; DUCC issue 86, especially comments `6294359778` and `6294831374`. These are provenance, not fresh performance guarantees.

## 3. Benchmark contract and initial matrix

First, **reuse the existing matched native RustSHT/DUCC benchmark infrastructure**
and its recent evidence rather than starting a redundant full campaign.
Freeze final Plan 11 source commit `a37f74d68631ab5fd373a4097640895e9f0c0c23` on this branch and pin DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The preceding matched builds used Rust 1.99.1/LLVM 23.1.1 and AVX2/FMA on a Ryzen 9 5950X. Record actual toolchains, source SHAs, flags, CPU affinity, SIMD dispatch, and memory-layout contracts. DUCC's reference SHT objects used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; Rust used strict FP and Pulp V3. Do not silently conflate FP settings or timings from older versions. A separately matched GCC/Clang DUCC control may isolate compiler effects.

- **Small q1 latency screen:** T4, T8, T16, T32, T48, T63, T64, T127, plus a tightly targeted boundary case if necessary. Use f64 GL scalar analysis and synthesis, full `mmax`, one exact matched longitude length per T, and only a few odd-minimum versus even-smooth controls where FFT shape is at issue. Include T1 and T2 only as absolute overhead controls. A cold/first-call table and a steady-state table are separate.
- **Large throughput screen:** T720 and T2047 with post-Plan-11-safe geometry; optionally T1023 for scaling. At T720 compare full `mmax` and a reduced-order control (such as `mmax=180`); compare matched odd-minimum/even-smooth longitude sampling where relevant. Run q1 and q8; q4 in the same cache domain is an optional discriminator, not a mandatory broad thread sweep. Do not spend time repeating known pathological T2048 cutoff measurements.
- **Benchmark semantics:** identical map/coefficient formulas, geometry, spin, normalization, phase, latitude/longitude resolution and useful harmonic work. Preallocate output and scratch as each API allows. Exclude plan/workspace creation from steady-state latency; measure **first call and plan/workspace setup separately**. Include exact input/output identity checks and normalized errors. Avoid comparing Rust native calls to Python DUCC wrapper calls as if call boundaries match.
- **Timing:** warm up; for operations shorter than ~100 ms, calibrate repeated complete calls to ~20–100 ms per batch and collect **at least 30 timed batches over multiple independent, interleaved process runs**. For long calls, use repeated individually timed calls across independent interleaved runs, report count, median, p10/p90 and spread. Pin q1 to one physical core; use identical eight **physical** CPU cores and affinity for both libraries at q8. Verify selected Pulp V3 and DUCC V3 actually execute; do not infer ISA from a compiled symbol alone. Check host contention/PSI and frequency; preserve noisy observations and explain uncertainty.
- **Stage boundary rule:** measure comparable stages in each library, ideally using temporary minimally intrusive native hooks. In particular separate FFT, spectral packing/scaling/reordering, Legendre preparation, ordinary recurrence/accumulation, scaled work, output scatter, scheduler/worker startup and zeroing. Do not subtract incompatible phase timers and label the difference a CPU-cycle measurement. Report total whole-call latency alongside stage numbers.

## 4. Phase A — Find and fix small serial fixed costs

**Hypothesis to test:** small T1–T32 transforms spend a significant fraction of wall time in repeated per-call setup, data conversion, buffer handling and/or independent row FFTs rather than in useful Legendre arithmetic. This is a hypothesis, not a proven diagnosis.

1. Build a microsecond-level, reusable-plan stage profile for q1 analysis and synthesis. Measure per-call validation, output clearing, grid `sin/cos`/order-limit preparation, phase and weight construction, spectrum reordering, table or recurrence dispatch, real FFT packing/recombination, recurrence coefficient preparation and numerical safety paths. Use repeated stage timing or controlled no-op/phase ablations so timer overhead does not dominate.
2. Compare DUCC's `map2leg`/`leg2map`, `ringhelper`, `allocate_leg` and Legendre preparation with Rust's `src/fft.rs`, `src/transform.rs`, `src/plan.rs` and `src/workspace.rs`. DUCC's own per-call preparation can be **greater** than Rust's; measure it rather than assuming a plan advantage or deficit.
3. Distinguish **fixed per-call costs** from O(`ntheta*nphi`) row FFT costs and O(`lmax`-dependent) Legendre work by fitting latency against operation counts or using carefully chosen neighboring sizes. Identify the single largest excess in **absolute microseconds** across target sizes before editing production.
4. Test one reversible, source-justified candidate at a time: e.g. remove demonstrably redundant initialization, cache immutable geometry already owned by the plan, avoid unnecessary intermediate copies, or specialize an existing tiny-FFT path if measured. Any cache must be immutable/thread-safe and preserve first-use behavior. Do not bypass input validation, finite/range checks, real Nyquist handling, scaling, or adjoint invariants. No unsupported small-band hardcoded answers.
5. Rerun both directions across small sizes and a T127/T720 control. Keep a change only when it reproducibly improves whole-call latency without a relevant regression or material memory increase. If no general change helps, document the quantified floor and explicitly stop; do not stack speculative micro-optimizations.

**Plan 12 interface:** measure and report the separate T63/T64 and T127/T255 table-selection discontinuities, cold-build cost and warm-read cost, but leave any cache-aware threshold redesign to Plan 12. Current f64 scalar **synthesis** selects recurrence from T64; scalar **analysis** has its own selector. One global T64 cutoff has already been rejected.

## 5. Phase B — Attribute the remaining q8 large-transform excess

Do this **after** integrating the T2048 result, using the new baseline. Rank **excess wall milliseconds**, not percentage speedups of isolated microkernels. Analyze scalar **analysis and synthesis separately**. A 1.2–1.4× whole-transform ratio cannot be attributed to a mutex, memory traffic, FFT or SIMD solely because that difference is visible in source.

### B1. Parallel work distribution and per-call scheduling

- Rust analysis currently builds a per-call order queue of `(m, disjoint coefficient slice)` items guarded by `Mutex<VecDeque>`, then each Rayon worker pops one order at a time. DUCC dispatches order work through `execDynamic(..., chunksize=1)` using a lightweight atomic-index path; its FFT ring passes use their own scheduling policy. Compare queue creation, per-pop locking, Rayon region entry, worker utilization, time per `m`, and critical-path tail. Account for both libraries' **multiple parallel regions**, which affect small tasks differently from large tasks.
- First measure queue wait, number of order jobs, worker idle time and imbalance. The queue may not explain a many-millisecond q8 gap, and static contiguous division can degrade balance because low `m` contains more Legendre work. Do not replace a scheduler on appearance alone.
- If supported by measurements, make one isolated alternative: atomic-index jobs with safe disjoint-output ownership, weighted static/guided order ranges, or another bounded policy. Avoid a global mutex on a hot work-item path, but preserve workspace ownership, error propagation, deterministic output and no oversubscription. Benchmark q1 and q8 plus full/reduced `mmax` before retaining it.

### B2. Data layout, zeroing, packing and output movement

- Trace real bytes read/written, ownership and strides: original row-major map, row FFT half spectra, Rust's additional order-major spectra representation, compact reflected-ring SIMD inputs, coefficient accumulators and final scatter. Compare DUCC's actual `allocate_leg`, `fill_m2a`, order-local SIMD scratch and output layout for **the same operation**.
- Measure spectrum transpose/packing, weight/phase application, repeated `sin/cos`/polar-limit calculations, buffer clears and output scatter **individually**. At high T, the intermediate complex arrays are large enough to warrant bandwidth measurements, but do not call them bandwidth-bound without evidence.
- If a measured memory pass is material, try a blocked conversion, fuse compatible passes without altering arithmetic order, or reduce a redundant copy. Preserve SIMD-contiguous inputs, data races, cancellation accuracy, complex conjugation, and per-worker scratch limits. Compare whole-call and cache/NUMA-domain scaling; avoid making the single-thread path worse merely to simplify q8.

### B3. Longitude FFT and its parallel execution

- RustSHT currently uses `RealFftPlan` backed by RustFFT, with even-length half-complex packing/recombination and a full complex transform for odd length. DUCC's `ringhelper`/pocketfft implementation has different data movement and may pack different amounts of work per ring. The existence of these designs **does not prove** they dominate the residual.
- Compare equivalent forward/inverse **real row FFT** calls, packing, scaling, scratch and row scheduling at the exact SHT longitude lengths. Separate FFT arithmetic from reading/writing row buffers; include q1/q8, odd-minimum and even-smooth sizes, and full-SHT stage time.
- Only pursue FFT changes if the measured whole-SHT opportunity is substantial. Prefer a localized improvement in existing RustSHT's FFT integration before adding another FFT engine or dependency. Keep unrelated RustSHT-FFT performance work separate. Test R2C/C2R, Nyquist/DC edge values, odd/even length, f32/f64 and exceptional-input fallback.

### B4. SIMD arithmetic and recurrence organization

- On **the executing warm, proven-safe route**, compare matching Rust Pulp V3 and DUCC `map2alm_kernel`/`alm2map_kernel` useful degree/lane work and actual AVX2/FMA assembly. DUCC's scalar analysis kernel shares coefficient broadcasts over vector blocks, keeps four parity/real-imag accumulators, and reduces them at degree groups; Rust's fused recurrence and accumulation have their own coefficient, scratch and reduction lifetime. Identify an actual critical-path or memory-traffic difference before redesigning arithmetic.
- Distinguish ordinary recurrence from scaled-prefix/transition work, safety proof on the cold call, horizontal reductions, register spills, coefficient staging and final coefficient correction. Follow source to executed assembly; static instruction counts, frame size, or a simpler loop are insufficient. Use hardware counters only when permitted; if `perf_event_paranoid` prevents access, label cache/IPC theories unverified and use bounded software probes.
- Previously unsuccessful unchanged ideas are **not candidates**: three accumulator chains, naive multi-degree unroll, packed complex reduction, direct output accumulation, indexed loop syntax changes, global smaller analysis tiles and broad range-check rewrites. A **different** algorithm requires new evidence and an independently tested mechanism.
- Treat the fixed aligned-scratch q8 **synthesis** improvement as an experimental lead if Plan 11 did not resolve it. Isolate exact code-generation, alignment, false sharing, scratch-capacity and memory effects across scalar analysis/synthesis. Do not promote a config flag or allocate substantially more scratch based on one q8 speedup.

### B5. Parallel topology control

- On the Ryzen 5950X, eight physical cores can fit within one eight-core CCD/L3 domain. Repeat only the diagnostically useful q8 cases on the **same pinned cores** for both libraries. If necessary compare one-CCD q8 with a cross-CCD q8 arrangement, documenting placement, not treating disparate CPU affinity as a library speedup. Check whether Rayon pool reuse and DUCC pool reuse have matching startup contracts.
- Explicitly distinguish speedup relative to each library's own q1 from Rust/DUCC q8 ratio; an intrinsically faster q1 kernel can still scale poorly, and vice versa.

## 6. Phase C — Validate and close with a narrow implementation

For each accepted candidate:

1. Keep the change isolated and identify the specific source-level mechanism it removes. Validate **same-work, warm and first-call** correctness before performance claims.
2. Test full `S/A/S*/A*` scalar operations as applicable, spin 0–4 regressions, GL/CC/F1, f32/f64, odd/even longitude counts, polar/high-order edge cases, subnormals, near-zero cancellations, huge finite and nonfinite inputs, SIMD tails, cold-safe/unsafe cache transitions, thread counts and concurrent independent workspaces. Compare to pinned DUCC and analytic/adjoint identities; do not loosen existing tolerances.
3. Run release/feature and no-default-features Rust test suites, strict Clippy/formatting and Python bindings tests **only if touched**. Track stack, heap, peak RSS, per-worker scratch and construction cost. Stable Rust, portable runtime Pulp dispatch and bounded caller-owned scratch remain requirements.
4. Commit only validated, evidence-backed changes. Cleanly remove failed cfg paths, redundant temporary kernels and large raw assembly dumps from production. Preserve small machine-readable timing datasets and the decisive diagnostic summary with exact source/toolchain revisions and reproduction commands.
5. Summarize remaining q1/q8 latency by **absolute time in each stage**, measured Rust/DUCC ratios, 30-batch uncertainty or individual-call counts, accepted/rejected alternatives, accuracy, memory and next justified work. If measured parity is not achieved, report the isolated residual and a tested follow-up mechanism; do not declare success from a 2–5% local change.

## 7. Execution and branch rules

- **Active branch/session:** remain on `opt/sht-performance-parity`.
  Plan 11 closeout `a37f74d6` was pushed and verified. `AGENTS.md`,
  `plans/README.md`, Plan 11 and this plan mark Plan 11 complete and
  Plan 14 active. No branch or session change is needed.
- Rebaseline the target cases from the final Plan 11 commit and use the
  staged measurements below to select work. Do not rerun the closed
  reduced-`mmax` frontend, cutoff or tile experiments.
- **Rebaseline rather than duplicate finished work.** The T2048
  cutoff, fixed 256 tile, q8 order-row preparation, and any additional
  resolved Plan 11 frontend work are accepted baseline. Compare current
  whole-transform latencies and select the most significant remaining
  small-serial or parallel deficit. Revise stale hypotheses and
  remove inapplicable benchmark requests before running them.
- **Execute the staged Plan 14 milestones** (baseline/stage attribution,
  small serial latency, remaining parallel excess, whole-SHT validation,
  documentation and cleanup) guided by measured absolute time. The
  sequence is a roadmap, not permission to repeat experiments with
  settled results or chase fixed acceptance percentages.
- **Commit and push on `opt/sht-performance-parity` after meaningful
  tested milestones**, following the existing `AGENTS.md` rule:
  inspect worktree, `git fetch origin`, rebase **only unpublished**
  local commits when safe, reread changed instructions, retest
  affected paths, push normally and verify remote HEAD.
  Never reset away local/concurrent work, force-push, rewrite
  published commits or push known numerical failures.
- Do not start Plans 12/13, add a Sol reviewer or begin unrelated
  work unless the owner subsequently requests it. Keep raw host
  diagnostics and oversized disassembly local/untracked.
- Final deliverables: reproducible native baseline, stage-level excess-time table, source/assembly explanation, small serial and q8 results with noise bounds, numerical and memory checks, concise plan completion report, and exact remaining bottlenecks.

## References

- RustSHT source: `src/transform.rs`, `src/legendre/scalar_paired.rs`, `src/legendre/scalar_ordered.rs`, `src/legendre/scalar_table.rs`, `src/fft.rs`, `src/workspace.rs`.
- Current repo instructions: `AGENTS.md`, `plans/11-ducc-equivalent-scalar-analysis.md`, `PERFORMANCE_STATUS.md`.
- Existing deferred scopes: `plans/12-cache-informed-legendre-dispatch.md`, `plans/13-fused-batched-sht.md`.
- DUCC 0.42.0 pinned reference: `https://github.com/mreineck/ducc/tree/42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; relevant code: `src/ducc0/sht/sht.cc`, `src/ducc0/sht/sht_inner_loop.h`, `src/ducc0/infra/threading.cc`.
- Martin Reinecke comments: `https://github.com/mreineck/ducc/issues/86#issuecomment-6294831374`, `https://github.com/mreineck/ducc/issues/86#issuecomment-6162674657`.
