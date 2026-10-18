# Kernel naming and production-code retirement

**Active instruction — 2026-10-18.** Applies to `opt/sht-performance-parity`. This is a bounded, implementation-oriented cleanup, **not** permission to delete required numerical behavior. Consult [AGENTS.md](../AGENTS.md) and [the current roadmap](05-validation-and-milestones.md). Inspect the **latest remote source and commits** before touching anything; this branch is being updated concurrently.

## Decisions

1. **Use algorithmic Rust names, not competitor names.** DUCC is a numerical reference and GPL-attributed source, **not** RustSHT's internal subsystem identity. Rename production files, structs, traits, helpers, dispatch functions, constants and private tests containing `ducc` when naming *our implementation*. The immediate target is `src/legendre/scalar_ducc.rs` → an algorithmic name such as `src/legendre/scalar_ordered.rs`, `ScalarDuccScratch` → `ScalarOrderedScratch`, `SCALAR_DUCC_BATCH_SIZE` → `SCALAR_ORDER_BATCH_SIZE`, and `synthesize_scalar_ducc_*` → `synthesize_scalar_ordered_*`. Prefer descriptive, coherent names over mechanical blind replacement. Match actual module semantics; `ordered` is an example, not a mandatory suffix.
2. **Preserve honest provenance and interoperability names.** Do **not** scrub DUCC from copyright/licensing, source-port attributions, pinned upstream comparisons, dependency names, external API names, reference fixtures, or benchmark/report text where it identifies the real library. Rename private test identifiers and outputs if they unnecessarily brand RustSHT's own kernels as DUCC, but keep reference attribution. Do not alter public Rust/Python APIs unnecessarily.
3. **Production `src/` should contain live, maintainable, necessary implementations only.** Retire abandoned experiments, uncalled alternatives, duplicated numerical kernels that are comprehensively dominated, and stale scratch/dispatch complexity when evidence proves their removal is safe. Do not keep every older candidate as a permanent production fallback.
4. **Minimal fallback policy:** Target one **small, genuinely robust generic safety fallback per mathematical operation/numerical regime where necessary**, plus any unavoidable portable scalar execution. A safe fallback must handle extreme finite/subnormal coefficients, overflow/underflow, unusual geometry, SIMD-ineligible cases and failed fast-path preflight **as appropriate to the supported operation**. Do not stack the old table, tile, paired, visitor and experimental implementations as interchangeable fallbacks merely because they once existed. If an old path still uniquely implements required f32, CC/F1, spin/adjoint or extreme-value behavior, it is **not dead**; keep it until equivalent tested coverage exists. Efficient small-size specializations may remain if a measured regime justifies them. Avoid excessive defensiveness and duplicate layers.
5. **Move only proven inferior or now-unused code to `backups/retired-kernels/` outside `src/`.** Preserve source text and GPL headers, with `README.md` mapping former path/symbols to reason, evidence, replacement, source commit and date. Use descriptive files, optionally `.rs.txt` for standalone uncompiled archived fragments. It must not be included in Cargo modules, builds, codegen, or production dispatch. Do not move a large shared source module wholesale if it still provides live functionality; extract/archive only retired portions and leave required code in a lean module. If equivalent snapshots already exist in tracked archival reports, reference them in the index rather than unnecessarily duplicating huge blobs, but keep an accessible backup for each intentionally retired unique implementation.
6. **Removal requires evidence.** Classify every candidate as (A) demonstrably slower across comparable representative valid workloads with no justified niche; (B) unreachable/dead after source and feature/test call-site inspection; (C) superseded by a fully validated replacement; or (D) still needed/unknown. Only retire A/B/C; keep D until resolved. One noisy single-shape screen, missing benchmark, or a rejection in an unrelated algorithm is not proof. Compare equivalent work, setup, memory and numerical coverage. Scope the cleanup: do not repeat a multi-day benchmark campaign to retire code.

## Concrete audit and cleanup sequence

### Phase 1 — Inventory, not wholesale deletion

Trace production call sites and dispatch precedence in:

- `src/legendre/scalar_ducc.rs` (current new order-major kernel; **retain its live implementation while renaming**)
- `src/legendre/synthesis_fast.rs` (old tile-128 implementation)
- `src/legendre/scalar_paired.rs`, `scalar.rs`, `scalar_f32.rs`, `scalar_table.rs`
- `src/legendre/mod.rs`, `src/transform.rs`, `src/spin_transform.rs`
- `src/workspace.rs`, `src/plan.rs` and relevant Rust/Python tests

Check `#[cfg]` variants, no-default-features, runtimes, PyO3 interfaces and high-bandlimit/odd-longitude/reduced-mmax paths. Identify which modules implement shared analysis/adjoints or spin behavior and must remain. Do not assume `scalar_paired.rs` or `synthesis_fast.rs` is wholly obsolete just because one synthesized fast path is quicker.

### Phase 2 — Rename first, in a coherent commit

Rename the new **live** scalar source to a neutral algorithm-based name. Update module declarations, trait methods, constants, tests, examples, private logging and documentation pointers. Keep DUCC source attribution and comparative report names where historical. Run `rg -n -i 'ducc' src python` and inspect remaining matches; actual external reference names and license text are allowed, internal algorithm branding is not. Compile/test both serial and parallel. Commit and push this rename separately so source history remains readable.

### Phase 3 — Consolidate to one dependable path and small fallback

Establish a support matrix by operation, dtype, grid, mode, spin, longitude parity, `mmax`, thread count and exceptional inputs. Where the ordered kernel covers an operation with correct fast and exceptional behavior, eliminate obsolete fallback chains, unnecessary coefficient caches, unused workspace fields, stale dispatch selectors and duplicate kernels. If exceptional paths are not safely covered, **first provide one minimal generic robust fallback** (reuse the simplest proven one if feasible), then retire redundant older alternatives. Avoid duplicating the whole optimized pipeline for a rare numerical corner case. Remove obsolete temporary tests but retain meaningful contract and regression tests.

Preserve working CC/F1, f32, spin 1–4, S/A/S*/A*, GRAD_ONLY/DERIV1, all supported numerical constraints, Python integration and portable builds. Do not turn a currently passing combination into an unsupported one merely to reduce source size. Do not “clean” the separate `crates/rustsht-fft` research crate.

### Phase 4 — Backup, verify, and move on

Create `backups/retired-kernels/README.md` with a short audit table:

`Old path/symbol | Status | Evidence | Replacement/remaining fallback | Archived file or Git revision`

Move the confirmed retired source **outside `src/`**, remove stale references, and avoid needless duplicated backup versions. Full preserved historical `reports/` artifacts remain untouched.

Run `cargo fmt --all -- --check`, Clippy all targets/features, `cargo test --workspace --all-features`, no-default-features build/test and the relevant pinned DUCC/analytic tests; Python tests where interfaces or binding-sensitive internals change. Use a small representative smoke panel to detect **large performance regressions**, including q1/q4/q8, irregular moderate/high bandlimits, reduced `mmax`, odd/even longitudes, and robust-fallback inputs. Do not weaken tolerances. Document any unavailable optional checks.

Complete the rename/cleanup in bounded meaningful commits. Then **return to the DUCC-derived algorithms and performance parity work**, prioritizing scalar analysis and spin-1 extension after necessary synthesis validation. Keep `PERFORMANCE_STATUS.md` short: summarize source architecture and exceptions, not a detailed cleanup diary.

## Git synchronization and ownership

Before **every push**, `git status`, `git fetch origin`, inspect branch ancestry and local/remote differences; fast-forward local if behind, or safely rebase **unpublished** commits/merge concurrent changes when appropriate, resolving conflicts without discarding work. Run relevant tests after reconciliation. Push normally and verify remote HEAD. No force push, destructive reset, or modification to `main`.

Only one Luna benchmark-only subagent is allowed; no Sol reviewer. The main Luna Max agent owns source refactoring, numerical testing and the implemented result.

**Priority:** one efficient production route, one minimal validated exceptional-case fallback when necessary, no accumulation of dead/slower kernels in `src/`, and attribution kept intact.
