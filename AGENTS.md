# RustSHT agent instructions

**Owner-authorized sequential active work (2026-10-21):** Finish
[Plan 11](plans/11-ducc-equivalent-scalar-analysis.md) **first**, then
continue automatically into [Plan 14](plans/14-small-serial-parallel-performance.md)
**in the same Luna session and on the same `opt/sht-performance-parity`
branch**. No new branch, new session or additional approval is required.
The T2048 latitude-dependent prefix discontinuity is **already fixed**
at `0bc5f153`; the q8 order-preparation improvement is already
implemented at `dfed3ff7`/`3b32f2de`. Plan 11's remaining
task is narrow reduced-`mmax` frontend attribution and closeout.
Only after committing/pushing that closeout should Luna
mark Plan 14 active and begin its next stage.
Read the current active plan and production source before acting.
After Plan 11 closeout, update Plan 11, Plan 14, `plans/README.md` and
this file to record Plan 11 **COMPLETE**, Plan 14 **ACTIVE** and the
closeout SHA. Rebaseline Plan 14 from the final code rather than redoing
already completed measurements. Explicit current owner instructions take
precedence; other archived plans/reviews and benchmark reports are
historical evidence, not competing goals.

- **Scope and order:** Complete Plan 11's **f64 scalar GL STANDARD reduced-`mmax` forward-analysis frontend** study first: distinguish finite-input scan, forward row FFT and spectrum/weight packing versus pinned DUCC at T2047 m511 / T3072 m768; then validate, document, clean up and push. Do not repeat resolved T2048 cutoff, q8 order-row preparation, or 64/128 tile campaigns. **Once Plan 11 is closed**, proceed directly to Plan 14's separately specified small q1 and large q8 performance scope using a refreshed current-source baseline. Plans 12/13 remain future-only; no speculative global FFT rewrite, cache autotune, batch API or unrelated new dependencies.
- **Numerics:** preserve the Rust API, independent `S/A/S*/A*` operators, GL/CC/F1, f32/f64, spin 0–4, correct scaling/phase/weights, subnormal and extreme-value safeguards, SIMD tails, serial/parallel behavior and checked fallbacks. Verify against pinned DUCC **and** analytic/adjoint tests; never relax tolerances for speed.
- **Implementation:** stable Rust, portable runtime Pulp SIMD and bounded caller-owned scratch. Neutral RustSHT source names, correct GPL-2.0-or-later and translated DUCC attribution. Archive demonstrably retired prototypes; do not retain layers of competing kernels.
- **Evidence:** compare **equivalent kernel boundaries and actual executed SIMD machine code** using matched native Rust/DUCC release builds. Isolate normal versus scaled work, hot-loop frequency, memory access, reductions, checks, loads/stores, compiler effects and full-transform overhead. A shorter assembly listing or lower stack frame is not proof of speed. Keep investigating the dominant residual when an isolated small change succeeds. No arbitrary truncation-specific or percentage acceptance target. Preserve historical evidence; keep raw disassembly and local host/process/PSI logs untracked, with concise reproducible findings committed.
- **Benchmark repetitions:** Nine single-call samples are insufficient for final short-kernel claims. For short operations, warm up and calibrate repeated invocations to ~20–100 ms per timed batch; collect **at least 30 timed batches across independent runs** per case. For long whole-transform calls above ~100 ms, individually timed calls over repeated interleaved processes are appropriate: report the actual count (e.g. 15), median, p10/p90 and spread, not a fabricated batch count. Pin q1 to one physical core, use matched physical-core sets at q8, retain first-call/warm distinctions and host-contended observations. Equivalent work/reset semantics are mandatory.
- **Workflow:** at most one Luna benchmark-only subagent, no Sol reviewer unless the owner changes this. Run formatting, relevant Clippy, release tests and no-default-features tests, with Python tests if bindings change.
- **Reviewable progress:** Implement and benchmark **one attributable change at a time**; make small buildable, numerically validated commits and push after each meaningful change, following the sync rule below. **Keep production scalar-analysis tiles at 256 reflected latitude pairs. Do not repeatedly rebench or recommend 64/128 tiles** as a workaround for the disabled-prefix route; previously measured comparisons are archived. A 512-tile experiment is optional only *after* the cutoff has been addressed and requires appropriate workspace-capacity changes. Packed reductions, direct coefficient accumulation and indexed-loop prototypes have been measured/rejected or retired; do not resurrect them without a new mechanism. The fixed-scratch q8 synthesis speedup is opt-in/unattributed, not promoted. Do not push known correctness regressions.
- **Mandatory sync before every push:** inspect worktree; safely commit/stash uncommitted work; `git fetch origin`; **rebase only unpublished local commits onto `origin/opt/sht-performance-parity`** when safe. Resolve conflicts without losing concurrent work; if unsafe, preserve work and report the conflict, never reset or force-push. After integrating remote work, reread this file, the **currently active** Plan 11 or Plan 14 and any new owner instructions, then rerun affected tests, push normally and verify remote HEAD. **The Plan 11 → Plan 14 transition stays on this branch.** Never rewrite published commits, force-push or touch `main` without authorization.
