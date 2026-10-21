# RustSHT agent instructions

**Active plan (2026-10-21):** [Plan 15](plans/15-spin1-vector-gl-cc-f1-performance.md)
is **ACTIVE** on `opt/sht-performance-parity` in the same Luna session.
[Plan 11](plans/11-ducc-equivalent-scalar-analysis.md) is complete at
`a37f74d68631ab5fd373a4097640895e9f0c0c23`; [Plan 14](plans/14-small-serial-parallel-performance.md)
is complete at closeout commit `f2d9ce47fdf04946ef5cbe478a30b0fc38956f8e`.
Plan 14's full scalar GL sweep and four figures are published, its q8
sparse-tail validation and remaining stage excess are documented, and its
release/no-default checks pass. Plan 15 covers spin-1 vector GL → CC → F1;
do not expand it into scalar CC/F1, spins 2–4 or batch APIs. Plans 12/13
remain deferred.
Explicit current owner instructions take precedence;
other archived plans/reviews and benchmark reports are historical evidence.
Read the current active plan and production source before acting.

- **Scope/sequence:** Plan 14 is closed; no further scalar optimization is
  needed for its closeout. Follow Plan 15's native matched spin-1 E/B
  benchmark and GL → CC → F1 stages, preserving the grid-specific
  meridional analysis and adjoint semantics. Do not treat scalar
  optimization or spins 2–4 as part of Plan 15. Plans 12/13 remain
  future-only; no speculative global FFT rewrite, cache autotune, batch API
  or unrelated dependencies.
- **Numerics:** preserve the Rust API, independent `S/A/S*/A*` operators, GL/CC/F1, f32/f64, spin 0–4, correct scaling/phase/weights, subnormal and extreme-value safeguards, SIMD tails, serial/parallel behavior and checked fallbacks. Verify against pinned DUCC **and** analytic/adjoint tests; never relax tolerances for speed.
- **Implementation:** stable Rust, portable runtime Pulp SIMD and bounded caller-owned scratch. Neutral RustSHT source names, correct GPL-2.0-or-later and translated DUCC attribution. Archive demonstrably retired prototypes; do not retain layers of competing kernels.
- **Evidence:** compare **equivalent kernel boundaries and actual executed SIMD machine code** using matched native Rust/DUCC release builds. Isolate normal versus scaled work, hot-loop frequency, memory access, reductions, checks, loads/stores, compiler effects and full-transform overhead. A shorter assembly listing or lower stack frame is not proof of speed. Keep investigating the dominant residual when an isolated small change succeeds. No arbitrary truncation-specific or percentage acceptance target. Preserve historical evidence; keep raw disassembly and local host/process/PSI logs untracked, with concise reproducible findings committed.
- **Benchmark repetitions:** Nine single-call samples are insufficient for final short-kernel claims. For short operations, warm up and calibrate repeated invocations to ~20–100 ms per timed batch; collect **at least 30 timed batches across independent runs** per case. For long whole-transform calls above ~100 ms, individually timed calls over repeated interleaved processes are appropriate: report the actual count (e.g. 15), median, p10/p90 and spread, not a fabricated batch count. Pin q1 to one physical core, use matched physical-core sets at q8, retain first-call/warm distinctions and host-contended observations. Equivalent work/reset semantics are mandatory.
- **Workflow:** at most one Luna benchmark-only subagent, no Sol reviewer unless the owner changes this. Run formatting, relevant Clippy, release tests and no-default-features tests, with Python tests if bindings change.
- **Reviewable progress:** Implement and benchmark **one attributable change at a time**; make small buildable, numerically validated commits and push after each meaningful change, following the sync rule below. **Keep production scalar-analysis tiles at 256 reflected latitude pairs. Do not repeatedly rebench or recommend 64/128 tiles** as a workaround for the disabled-prefix route; previously measured comparisons are archived. A 512-tile experiment is optional only *after* the cutoff has been addressed and requires appropriate workspace-capacity changes. Packed reductions, direct coefficient accumulation and indexed-loop prototypes have been measured/rejected or retired; do not resurrect them without a new mechanism. The fixed-scratch q8 synthesis speedup is opt-in/unattributed, not promoted. Do not push known correctness regressions.
- **Mandatory sync before every push:** inspect worktree; safely commit/stash uncommitted work; `git fetch origin`; **rebase only unpublished local commits onto `origin/opt/sht-performance-parity`** when safe. Resolve conflicts without losing concurrent work; if unsafe, preserve work and report the conflict, never reset or force-push. After integrating remote work, reread this file, the currently active Plan 14 or Plan 15 and any new owner instructions, then rerun affected tests, push normally and verify remote HEAD. Never rewrite published commits, force-push or touch `main` without authorization.
