# DUCC scalar-kernel performance gap: implementation checklist

**Scope:** Native RustSHT DUCC-style scalar f64 GL synthesis, as of `43bec370` (2026-10-18). This is a **focused implementation brief**, not a standing benchmarking protocol. Current live priorities remain in [05](05-validation-and-milestones.md) and [status](../PERFORMANCE_STATUS.md).

**Working rule:** Check current branch HEAD and implementation before acting. If the agent has since implemented an item, validate or skip it. Do not redo completed work. Keep the old tile/paired/scaled Rust kernel as a correctness reference and robust fallback. No new subagents except at most one Luna benchmark-only agent; no Sol reviewer.

## 1. What the existing evidence actually establishes

The first single-thread DUCC-style synthesis candidate (`b484a8c7`) already implements order-major active/reflected-ring batching, transformed even/odd coefficients, DUCC-like scaled prefix skipping, SIMD ordinary recurrence/accumulation, and Fourier output reconstruction.

Its **initial** directional medians (f64 GL, one worker) were:

| Workload | Rust DUCC-style Legendre | DUCC `alm2leg` | Rust whole synthesis | DUCC `synthesis_2d` |
| --- | ---: | ---: | ---: | ---: |
| T255, 256×513, full mmax | 1.803 ms | 1.466 ms | 2.494 ms | 1.885 ms |
| T300, 301×603, full mmax | 2.658 ms | 1.957 ms | 4.091 ms | 3.779 ms |

The Rust Legendre call **also applies longitude phase and writes row-major real-FFT spectra**; DUCC's `alm2leg` writes unphased Legendre values. The full DUCC public call recreates geometry/transform state, whereas Rust reuses its plan. Thus these are **not equal-work kernel comparisons or proof of a specific bottleneck**. No host-pressure gate ran. Numerical differences against the prior Rust route were <3e-13 relative L2 in tested shapes.

**Later implementation, do not redo:** `9b1651f` added a monotone per-order active polar-ring boundary. `43bec37` added a parallel order-range route with bounded order-major scratch and a tiled transpose/scatter, preserving old-path fallback. Five local T300 GL 301×603 **reduced mmax=150** release samples measured q1/q8 full-call medians ~3.33/2.13 ms (1.56× q8 speedup). This is a **different workload from the full-mmax T300 comparison above** and has **no matched DUCC q8 result**. The q8 implementation uses a recursive weighted order split, **not DUCC's `execDynamic` scheduler**. Do not call q8 parity established.

Evidence: [initial kernel screen](../reports/scalar-ducc-kernel-screen-2026-10-18.md), [current Rust kernel](../src/legendre/scalar_ordered.rs), [Rust driver](../src/transform.rs), [status](../PERFORMANCE_STATUS.md).

## 2. Source-defined differences and actionable implementations

Compare pinned DUCC `80388904b8cfc32400ba84af8a6a45aabfe60f86` (`src/ducc0/sht/sht_inner_loop.h`, `src/ducc0/sht/sht.cc`) with the current Rust source. The installed DUCC 0.41.0 corresponds to a separate pinned revision; the earlier source comparison found namespace-level differences in relevant SHT routines, but recheck if changing reference binaries.

### P1 — Compute power thresholds once per order, then vectorize sectoral initialization

- **Confirmed source difference at this brief's source revision:** DUCC `YlmBase` maintains `powlimit`, and `init_lambda` calls vector `mypow` for an entire SIMD ring vector. In the then-current Rust `scalar_ducc.rs`, `synthesize_batch` looped over individual lanes and invoked `mypow(sin_theta, m)`. Rust `mypow` computed `exp(-400 * ln(2) / m)` at every invocation for `m>0`, although the threshold depends only on the order.
- **First implement:** Obtain/cache the threshold once per order or per reusable plan, with correct `m=0`, large-order, exact-zero and finite-value treatment. Remove repeated `exp` work from the latitude loop. Keep existing safe arithmetic/fallback.
- **Then implement:** Port DUCC's vector `mypow`/sectoral `init_lambda` logic using `pulp` masks, including the unscaled fast case and scaled case, without scalar per-lane setup for normal SIMD blocks.
- **Check:** Crossvalidate normalized recurrence state and full spectra against the current Rust route and pinned DUCC for polar, nonpolar, scalar-tail and multiple `m`; include underflow/overflow/subnormal fallback cases. Measure full Legendre-stage effects, not only timing `exp`.

### P2 — Vectorize scaling and significance transitions

- **Confirmed source difference:** DUCC's `Tvnormalize`, `rescale` and `getCorfac` use vector masks and vector scaling state. Rust currently has per-lane `normalize_lane`, `rescale_batch`, `set_correction_factors` and scalar scale checks in `iter_to_ieee`/scaled accumulation; its normal-range recurrence already uses `pulp` SIMD.
- **Implement:** Represent the hot-path exponent/scale and correction factors as SIMD vectors where practical, use masked update/normalization, and avoid repeatedly scanning all lanes in scalar form. Preserve checked overflow behavior and the numerical significance criteria, especially at transition boundaries.
- **Do not:** Replace DUCC's `2^-60` significance threshold with an arbitrary larger cutoff, or silently weaken RustSHT's robust extreme-value guarantees. Retain a safe scalar fallback for uncommon unsafe inputs.
- **Check:** Exact route-entry/exit behavior and full-result error; include mixed active lanes, near-polar degrees, SIMD tails and exceptional finite values.

### P3 — Make the ordinary recurrence's generated machine code genuinely comparable

- **Confirmed:** Current Rust `ordinary_two_pair_step` and `recurrence_two_pair_step` use Pulp SIMD and FMA; recorded AVX2 assembly shows packed YMM/FMA. A one-pair tail is still an out-of-line call. DUCC's `alm2map_kernel` explicitly unrolls four degree steps (two paired steps) for AVX2 and, on wider SIMD, additional work.
- **Evaluate and implement:** Compare actual optimized inner-loop assembly and counts of vector loads/stores, spills, branches, broadcasts and FMA instructions. Aim for per-degree coefficient reuse, register-resident vector state and minimal repeated slice/view preparation. Hoist loop-invariant vector broadcasts. Try a tightly fused Rust SIMD loop if current helper decomposition yields unnecessary stores.
- **Tail:** DUCC pads incomplete ring batches to a whole vector (reusing the last ring), allowing vector execution for final lanes. Rust currently handles SIMD tails through scalar suffix loops. Implement equivalent padded SIMD tails only if safe and beneficial, with no writes for padded lanes.
- **Avoid:** Raw intrinsics solely on speculation or an assumption that Pulp is slow. Preserve stable Rust, runtime dispatch and correctness. Do not launch a general FFT replacement project.

### P4 — Align scratch/output layout and avoid redundant full-array writes

- **Confirmed:** DUCC `fill_a2m`/`extract_a2m` operate on fixed 256-ring batches and reflected north/south outputs. Rust now implements 256-ring batches and monotone polar skip; its q1 route writes directly into row-major `spectra`, while q8 writes intermediate order-major spectra and transposes to row-major on success.
- **Measure and improve:** Quantify active-ring packing, order setup, output phase/reconstruction and full-array transpose independently. Compare layout choices with equivalent end-to-end work. If a q8 order-major temporary is necessary, optimize its blocked scatter and reuse memory. Avoid allocating another copy of the full Fourier spectrum per worker or committing partial output before a failed route is identified.
- **Do not blindly recreate** previously rejected whole-transform transpose/order-owned prototypes; the present DUCC-style kernel is architecturally different, but full transpose cost still counts.

### P5 — Complete parallel order scheduling, then test q8 against DUCC

- **Already implemented in `43bec37`:** Parallel DUCC-style order ranges with Rayon, weighted recursive split, order-major workspace scratch and a bounded tile scatter. Preserve it.
- **Still different:** DUCC uses `execDynamic(nm, nthreads, 1, ...)` for harmonic orders. Rust's current static/recursive weighted split is not a faithful port of dynamic order scheduling and may suffer load imbalance at higher truncation/reduced mmax.
- **Next:** Test q1/q4/q8 for representative irregular and atmospheric workloads; collect simple per-worker elapsed work and transpose cost **only if** a scaling bottleneck appears. If imbalanced, implement a safe dynamic order queue or appropriately sized disjoint tiles without shared mutable recurrence scratch or false sharing, and show net performance including conversion.
- **Guard:** Worker-count variation, reduced `mmax`, oversampled rings, odd/even longitude, extreme-value fallback, and no partially committed spectra when a worker fails. Do not assume q8 benefit from the small T300 screen extends to high resolutions.

### P6 — Remaining DUCC structure only when relevant

- DUCC's `inner_loop_a2m` switches to degree panels at `lmax+1-m > lstep` (`lstep=4096`). That does **not** constrain tested T255–T2047 workloads; do not implement high-degree panel machinery ahead of more consequential items. Revisit for larger validated bandlimits.
- DUCC may avoid extra work by its memory layout, vectorized ring normalization and special-case geometry. Inspect exact code before attributing latency; avoid generic claims that “C++ is faster than Rust.”
- For odd `nphi`, Rust's root real-FFT adapter uses the full-complex fallback. Diagnose the **FFT difference** separately from the Legendre gap; compare even and odd FFT-friendly lengths. A slow FFT should not be misdiagnosed as a recurrence issue.

## 3. Test protocol without returning to benchmark paralysis

Implementation-first sequence:

1. Re-read **only** current `scalar_ordered.rs`, `transform.rs`, current `PERFORMANCE_STATUS.md`, and the relevant pinned DUCC functions. Inspect the latest commits before editing; skip already completed work.
2. Choose **one coherent change at a time** (P1 is the first candidate), implement it in production source, check numerics/portable fallback, and commit a substantive working increment.
3. Use a small, varied correctness screen: one small/medium case with a SIMD tail, one irregular middle bandlimit, and one high-resolution case. Include reduced `mmax`, odd/even `nphi`, nonzero `phi0`, polarized-ring extremes, and q1/q8 at the integrated parallel milestone. Maintain existing regression coverage for T511/T1023/T2047, but never privilege them as performance targets.
4. For a **major working integrated revision only**, delegate bounded matched benchmarking to at most one Luna benchmark-only subagent. Compare (a) old Rust route, (b) current DUCC-style Rust route, (c) reference DUCC; clearly distinguish Legendre-stage calls, phase/output conversion, plan construction and full synthesis. Report medians, noise limitations, numerical error, memory and regressions. Do not conflate unlike-for-like samples or change gates retroactively.
5. If no meaningful benefit, report why briefly and continue with the next source-supported change. If a host is noisy, do not stop otherwise safe code development.
6. After scalar synthesis is functionally and numerically stable with a significantly smaller measured gap, reuse the architecture for scalar analysis and spin-1 synthesis/analysis, then native f32 where mathematically appropriate; preserve GL/CC/F1 and adjoint conventions.

## 4. Git and reporting rules

Work on `opt/sht-performance-parity`. Before **every push**: inspect working tree, `git fetch origin`, compare local and remote history; fast-forward if behind, reconcile diverged *unpublished* local commits by safe rebase when appropriate or non-destructive merge, rerun relevant checks, then push normally and verify remote HEAD. Do not force-push, reset away work, or overwrite other sessions' changes. If conflicts cannot be safely resolved, preserve work and report the blocker.

Keep the milestone ledger, reports and archived work. Do **not** expand `AGENTS.md` or the short active milestone file with experiment-by-experiment details. Link this note when relevant. Update `PERFORMANCE_STATUS.md` only at substantive milestones.

**Success criterion:** Verified, portable native Rust scalar execution implementing DUCC's actual algorithms; numerical fallbacks intact; smaller measured equal-work Legendre and complete-transform gap across varied grids/bandlimits, plus efficient q8 scaling. Do not label it “DUCC parity” before matched evidence exists.
