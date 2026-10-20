# Active numerical implementation and validation roadmap

**Status:** Updated 2026-10-18 after the matched RustSHT–DUCC stage profile and scalar-prefix fix `41b309a`. Active development branch: `opt/sht-performance-parity`. The objective remains faithful **native Rust implementation of DUCC's complete numerical execution algorithms**, with comparable accuracy and performance across supported SHT operations. **DUCC is an external numerical reference, not a name for RustSHT modules/types.**

**Current highest priority (2026-10-18 evening):** [Plan 11 — DUCC-equivalent scalar-analysis execution](../../../plans/11-ducc-equivalent-scalar-analysis.md). The latest profile found a ~2× full-analysis deficit from recurrence/accumulation; 474 initially scaled blocks were real work, **not degree-loop replay**. Focus on matched kernel-only normal/scaled profiling and an integrated scaled-to-ordinary transition before more unrelated spin-1 work. Existing P0/P1 scalar-synthesis checkpoints below are completed historical work, not a standing benchmark queue; Plan 11 governs the active P2 implementation.

**Three-paper literature audit (reference only):** [plan 10](10-three-paper-method-audit.md) classifies methods from libsharp (2013), Ishioka (2018), and general SHT (2023) against current source. Continue P2/P3; defer multi-map fusion, autotuning, new grids, MPI and NUFFT until separately justified.

**Start here:** [AGENTS.md](../../../AGENTS.md) → [PERFORMANCE_STATUS.md](../performance-status-2026-10-19-pre-cleanup.md) → this roadmap. Read the [measured stage profile](../../matched-rust-ducc-stage-profile-2026-10-18.md) and [profiling procedure](09-matched-ducc-rust-profiling.md) only when profiling or changing the measured hotspots. Do not bulk-read the [historical ledger](05-validation-and-milestones-history.md), [old performance snapshot](../../performance-status-pre-ducc-port-2026-10-18.md), or archived reviews.

## Verified checkpoint and interpretation

- RustSHT `c5f1021` and pinned DUCC (`ducc0 0.41.0`, source `8038890`) were profiled at **f64 scalar GL T720 full mmax, 721×1442 (even longitude)** and **T300 mmax150, 301×603 (odd longitude)**, q1/q8, with matched AVX2/FMA x86-64-v3 optimization settings, numeric checks and five-sample medians.
- **Before the rescaling fix**, T720 q1 stage times were Rust/DUCC: **init/normalization 1.79/0.51 ms**, **scaled prefix 8.29/0.09 ms**, **ordinary recurrence 13.66/12.90 ms**. The timing boundaries are approximate, not identical: Rust's scaled-prefix bucket includes some scaled accumulation. T300 had the same qualitative pattern. Ordinary SIMD arithmetic itself was not the primary measured q1 excess; output/FFT was not the T720 bottleneck.
- Commit **`41b309a`** introduced an early-return fast path for empty SIMD rescaling and conditional scale-epoch reduction. In a small optimized screen, **T720 q1 went 31.098 → 26.741 ms (~14% faster), DUCC 26.001 ms (1.03×)**; **T300 q1 3.523 → 3.094 ms (~12% faster), DUCC 2.302 ms (1.34×)**. This is **workload-specific**, not general q1 parity. The **post-fix component-level prefix cost has not been measured**.
- **q8 remains behind and variable:** post-fix T720 6.343 versus DUCC 3.917 ms (1.62×); T300 0.898 versus 0.467 ms (1.92×). Five-sample q8 ranges overlap previous Rust measurements, so no definite improvement from the latest rescaling change is established. Earlier q8 stage data locate the excess inside the Legendre/order phase; a ~0.221 ms spectrum scatter at T720 is not the whole explanation.
- The earlier 21-cell synthesis matrix (Rust `6b356cb`, different timing/build conditions and more shapes) showed larger slowdowns. **Do not merge its ratios with the matched current stage screen or declare general parity.** Hardware `perf` events are unavailable (`perf_event_paranoid=4`); phase timers and static disassembly are presently the available evidence.
- Production scalar f64 GL synthesis uses **`src/legendre/scalar_ordered.rs`**, including SIMD and dynamic harmonic-order scheduling; the tile-128 accelerator is archived under `backups/retired-kernels/`. Scalar/spin analysis now has dynamic one-order scheduling, but its recurrence and performance still need work. All existing supported operations remain contractually required.

## Next implementation priorities (in order)

### P0 — One *post-fix* hotspot verification, then implement (short)

On **current source**, repeat a **bounded T720 q1 and T300 q1** equivalent-stage screen. Record initial powers, empty/nonempty prefix rescale, scaled accumulation, ordinary recurrence, FFT/writeback, and end-to-end medians. Use boundary timers, not per-degree timing. Count iterations/active lanes only if inexpensive and profiling-only. Record whether each candidate actually takes the ordered production route and whether any fallback fires.

**Decision:** If the remaining prefix/initialization overhead is materially above DUCC, implement **one** small source-grounded change to eliminate unnecessary per-vector masks, exponent reductions, initialization or duplicated work, and validate it. If the prefix gap has already shrunk substantially, **stop scalar q1 micro-tuning** and proceed. Preserve scaled numerical safeguards and source provenance. Do not spend a day trying to reproduce inaccessible hardware counters; do not change host `perf_event_paranoid` settings.

### P1 — Explain and reduce the q8 Legendre/order gap

Compare current Rust/DUCC **T720 q8** first, **T300 q8** second, at matching build flags and outputs; interleave short repetitions and distinguish jitter from a repeatable structural deficit. Separate dynamic order queue, per-worker recurrence busy time, order-major scratch/scatter, memory traffic and inverse FFT. Check worker counts, useful work by order, task granularity and actual fallbacks. Compare q1/q8 *stages* rather than whole-transform ratio alone.

Implement only the **largest measured** q8 deficiency. Do not blindly redo rejected recursive order scheduling, change to unbounded buffers, or rewrite RustFFT when FFT is already competitive in the profiled T720 case. Correct q8 output/exception handling and portable scalar execution are mandatory.

### P2 — Implement DUCC-equivalent scalar analysis execution (active)

Follow [Plan 11](../../../plans/11-ducc-equivalent-scalar-analysis.md), not the superseded initial stage-profiling task. Current analysis profiling shows normal recurrence/accumulation is the main excess and that all 474 T720 fast-route declines occur at initialization, with **no degree-loop replay**. Implement an efficient scaled-to-ordinary execution path and bounded register-pressure ordinary accumulation using DUCC's true data flow as the reference; reject any change that fails numeric/regime checks or worsens equivalent-work timings. Dynamic order scheduling, active-ring compaction, SIMD sectoral initialization and cosine preweighting already exist. Keep `A`, `S*`, and `A*` distinct.

### P3 — Spin-1 synthesis/analysis and shared execution

Bring the validated order/latitude SIMD machinery to spin-1 synthesis and analysis **where mathematically valid**, respecting E/B coupling, spin-1 recurrences, polar safeguards, derivative/GRAD_ONLY behavior, and existing adjoints. Reuse shared scheduling/scratch where justified instead of copying duplicate scalar code. Run compact representative q1/q8 DUCC/analytic/adjoint correctness and performance screens; avoid a prolonged broad benchmark campaign before substantive source changes.

### P4 — f32, grids and clean production code

Extend validated implementation paths to efficient native f32 and appropriate CC/F1 and spin-2 regimes without weakening distinct GL/CC/F1 contracts; preserve spin 3–4. Maintain algorithm-based Rust names and the [retirement rules](08-kernel-naming-and-retirement.md). **Aim for one optimized production implementation and at most one small robust generic fallback per necessary operation/regime**, not layers of superseded tiles, caches and experimental dispatches. Archive **only proven dead/dominated** code in `backups/retired-kernels/`, with source history and rationale. Do not remove a uniquely required operator or numerical edge case.

## Numerical, performance and operational gates

- Every substantive change: `cargo fmt --all -- --check`, relevant Clippy, all-features and no-default-features focused tests; broader suites periodically; Python tests if bindings affected. Validate pinned DUCC **and independent analytic/adjoint** checks. Do not relax tolerances to gain speed.
- Include a varied smoke screen (not just T511/T1023/T2047): odd/even longitude, irregular degree, reduced `mmax`, nonzero `phi0`, polar rings, SIMD tails, extreme/subnormal/very large finite coefficients, q1/q8, scalar and parallel builds. Compare the same numerical operation and matched equivalent work, including differing phase/output conversions. Preserve supported S/A/S*/A* identities, f32/f64 and spins 0–4.
- Performance: optimized **release** builds on both sides; explicitly record Rust/C++ compiler versions, CPU target/ISA, optimization flags, thread counts, plan/workspace lifecycle, fallback route and numerical errors. Separate timings with profiling hooks from uninstrumented total latency. Report q8 wall-clock separately from cumulative thread CPU time. Accept only repeatable changes; do not misstate five-sample results as comprehensive parity.
- Use at most **one Luna benchmarking-only subagent**, no Sol reviewer. Main agent implements tested code; do not leave the session blocked by host load or unavailable `perf`.
- Work only on `opt/sht-performance-parity`. **Before every push** inspect status and `git fetch origin`; fast-forward when behind, reconcile unpublished local commits by safe rebase where appropriate or merge safely; rerun relevant checks, push normally and verify remote HEAD. Never force push, reset away concurrent work, or touch `main` without authorization.
- Update [PERFORMANCE_STATUS.md](../performance-status-2026-10-19-pre-cleanup.md) **only after meaningful implementation milestones**. Keep it short; retain the [matched stage report](../../matched-rust-ducc-stage-profile-2026-10-18.md), historical reports, archived source and GPL attribution.
