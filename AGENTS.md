# RustSHT agent instructions

## Start here — short context only

1. Read this file and [PERFORMANCE_STATUS.md](PERFORMANCE_STATUS.md). Read [plans/05-validation-and-milestones.md](plans/05-validation-and-milestones.md) only for the **current DUCC-first implementation roadmap**.
2. Inspect the actual source files for the task. Consult [plans/02-numerics-and-grids.md](plans/02-numerics-and-grids.md), [plans/06-ducc-contract-crosswalk.md](plans/06-ducc-contract-crosswalk.md), and [plans/00-scope-and-decisions.md](plans/00-scope-and-decisions.md) **selectively** for specific numerical or API questions.
3. Do **not** bulk-read `reports/`, archived milestones, old reviews, or benchmark samples. The [historical validation ledger](plans/05-validation-and-milestones-history.md), [historical performance snapshot](reports/performance-status-pre-ducc-port-2026-10-18.md), and `ASTRA_INTEGRATED_REVIEW.md` preserve provenance, **not active instructions**.
4. Current explicit session/owner instructions take precedence over older project plans and reviews. Historical report benchmark choices are not optimization targets.

## Active engineering priority

**Port DUCC's actual numerical execution algorithms into native Rust first**, then optimize the resulting implementation. A scalar f64 GL order-major synthesis implementation (currently `src/legendre/scalar_ducc.rs`, to be renamed as directed in the naming/retirement plan) now includes SIMD and parallel scheduling; validate, refine and generalize it rather than restarting it: active and reflected ring batching, scaled initialization and DUCC's negligible-prefix criterion, parity reconstruction, transformed coefficient preparation, unrolled SIMD recurrence/accumulation, and efficient order scheduling. Compare equivalent kernel work with pinned DUCC, preserving RustSHT's numerically robust fallback for extreme values. Then tackle scalar analysis, spin-1 synthesis/analysis, applicable adjoints, f32, and supported grids. Keep the public API stable.

Production-code naming and retirement are governed by [plans/08-kernel-naming-and-retirement.md](plans/08-kernel-naming-and-retirement.md): use algorithmic names (no competitor branding for RustSHT types/files), archive proven inferior/dead variants outside `src/`, and consolidate to **one minimal validated safety fallback per operation/regime where required**. Do not retain unnecessary fallback layers; retain uniquely required coverage until its replacement passes tests. Historical code remains available in backups/Git/reports. Consult the concise status before rediscovering work.

Implement and test meaningful source changes before launching major profiling or benchmark campaigns. No arbitrary T511/T1023/T2047 optimization focus; choose varied, realistic and irregular bandlimits, reduced `mmax`, grid sizes and worker counts. Those historic cases remain regression coverage only. Do not install exact-bandlimit tuning selectors without a measured, explainable regime boundary.

## Numerical and implementation contract

- Independent GPL-2.0-or-later Rust library. Use **Albert M. W. Yau** for new contributions and preserve Martin Reinecke/DUCC notices, GPL obligations, and pinned source revisions when directly translating source. Do not copy DUCC's GPL-3.0-or-later Python tests verbatim.
- Stable **Rust 1.99.0** as the current baseline, Rust 2024. Test newer stable toolchains separately if they plausibly improve SIMD/codegen; change the pin only after numerical and performance validation. Portable runtime SIMD selection via `pulp` with scalar fallback; use Rayon for optional parallelism. No mandatory `target-cpu=native`, nightly, or unvalidated ISA; use raw intrinsics only for demonstrated gains.
- Keep RustFFT as the root crate's sole FFT backend, with the existing private even/odd real-FFT adapters. The separate `crates/rustsht-fft` research crate is not an integrated backend; do not delete it or work on it without an explicit task. Avoid new dependencies and unrelated refactors.
- Rust core owns validated `GridSpec`, `HarmonicSpec`, `ShtPlan`, caller-owned `ShtWorkspace` and transform kernels. Reuse bounded scratch; no hidden shared mutable state. Python bindings live in `python/` and do not belong in the numerical core.
- Preserve normalization, longitude phase and Nyquist conventions; scalar/spin signs, scaled recurrence, polar cutoffs and reflection parity; `m=0` reality and `l<spin` restrictions; GL/CC/F1 rules; independent **S, A, S*, A***; STANDARD/GRAD_ONLY/DERIV1 mode contracts; f32/f64; existing spin 2–4 functionality. Refer to the linked numerical specification for details.
- Guard extreme finite coefficients, subnormals, overflow, unsafe conversions, and fallback output staging. DUCC-like insignificant-term skipping requires a justified numerical error budget; do not trade away existing safety silently.
- Compare with a pinned DUCC revision **and** independent analytic/adjoint checks; roundtrips alone are insufficient. Test odd/even `nphi`, partial `mmax`, nonzero `phi0`, polar rings, SIMD tails and serial/parallel builds. Declare tested dimensions and errors rather than claiming universal parity.

## Workflow

- Work on the existing task branch; for this campaign, `opt/sht-performance-parity`. Check status, `git fetch origin`, and branch ancestry before pushing. Preserve concurrent work. Fast-forward when possible; if diverged, do not force-push/reset/rebase without authorization.
- Prioritize integrated algorithms and correctness. Use lightweight screens during implementation; reserve matched DUCC/SHTns comparisons and statistical gates for **major integrated milestones**. Record valid and invalid results accurately. A noisy benchmark is not a reason to stop safe implementation.
- Keep `PERFORMANCE_STATUS.md` short. Update after substantive milestones, not every micro-experiment. Retain every historical report, artifact and source patch; do not rewrite history or add one-off GitHub Actions jobs.
- Validate changes with `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and relevant `cargo test --workspace --all-features` / `cargo test -p rustsht --no-default-features`; run `uv run pytest python/tests` for binding changes. Run extended cases selectively and report skipped checks.
- For this campaign, use **at most one Luna benchmark-only subagent**; no Sol reviewer or additional subagents unless the owner explicitly changes this constraint.
- Commit and push meaningful validated milestones to the authorized feature branch; do not update `main` or force-push without authorization.

## Benchmark evidence and artifact hygiene

Commit reproducible methods, numerical errors, aggregate RSS, software/CPU specifications (CPU model, ISA, cores/RAM), source hashes, elapsed-time samples, bottleneck summaries and validated conclusions. Keep exact transient CPU pressure/PSI, frequency, affinity/CPU-ID or thread-placement captures, PIDs, process/command listings, local paths, usernames/hostnames, environment dumps, raw `/proc`, GNU time, build/test stdout/stderr and machine-state diagnostics **local and untracked**. Do not ignore entire scientific file extensions or delete historical evidence. Review `git diff --cached --name-status` and `git diff --cached`; use `git config core.hooksPath .githooks` and `python3 scripts/check_report_artifacts.py` before committing report evidence.
