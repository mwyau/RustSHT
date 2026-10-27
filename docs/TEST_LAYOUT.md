# RustSHT test layout — one Rust module convention

**Owner decision: 2026-10-27 02:19 UTC.** This is the standing repository rule for **all internal Rust unit tests**. This policy is about code organization only; it does not authorize weakening/removing tests or modifying numerical behavior.

## One layout, no inline unit-test bodies

- Every production module with unit tests declares only **`#[cfg(test)] mod tests;`** in its implementation module. **No inline `mod tests { ... }` bodies, no inline `mod f32_tests { ... }` bodies, no unit-test functions embedded in implementation files.** The external module owns test bodies and fixtures.
- The unit-test module file is always the module's natural **`tests.rs` in the directory named after that module**:
  - `src/evaluation.rs` → `src/evaluation/tests.rs`;
  - `src/grid/geodesic.rs` → `src/grid/geodesic/tests.rs`;
  - `src/legendre/pointwise.rs` → `src/legendre/pointwise/tests.rs`;
  - `src/legendre/scalar.rs` → `src/legendre/scalar/tests.rs`;
  - `src/legendre/spin.rs` → `src/legendre/spin/tests.rs`;
  - `src/legendre/scalar_paired/analysis.rs` → `src/legendre/scalar_paired/analysis/tests.rs`;
  - `src/transform/mod.rs` → `src/transform/tests.rs`;
  - `src/transform/spin.rs` → `src/transform/spin/tests.rs`;
  - `src/transform/spin/hemisphere.rs` → `src/transform/spin/hemisphere/tests.rs`.
- This is Rust's normal child-module resolution: both `foo.rs` and `foo/mod.rs` can declare `#[cfg(test)] mod tests;` and place the body in `foo/tests.rs`. **Do not convert existing `foo.rs` modules to `foo/mod.rs` merely to accommodate tests.** No custom `#[path = "..."]` declarations or `foo_tests.rs` siblings for normal unit tests.
- If a unit-test suite is too large for a useful single file, its **one entrypoint remains `foo/tests.rs`**, which may declare test-only child modules `mod synthesis;`, `mod analysis;` loaded from `foo/tests/synthesis.rs`, etc. This is still the *same pattern*, not a second top-level layout. For small modules, just `foo/tests.rs` is sufficient. Only split for useful domain grouping, not an arbitrary line limit.
- The root Cargo **`tests/*.rs`** directory is reserved for **integration tests of public API behavior**, including `tests/f32.rs`, `tests/extreme_fft_scaling.rs`, and `tests/workspace_compatibility.rs`. Keep those as independent integration crates; do not move them into private module unit-test trees. Python tests retain their Python tooling conventions. Nonstandard test-only *benchmark harnesses* remain separately identifiable as benchmarks; when reorganizing one, preserve its invocation, cfg gating and provenance.

## Migration, after current spin numerical consolidation

This is **not a request to interrupt the ongoing `spin_f32.rs` algorithm retirement**. Finish the native-spin numerical refactor first; then perform a separate mechanical test-layout change, without concurrent mathematical/route/performance changes.

1. Move the existing `src/evaluation_tests.rs`, `src/grid/geodesic_tests.rs`, `src/legendre/pointwise_tests.rs` to their respective `<module>/tests.rs` locations and replace their `#[path]` declarations with `#[cfg(test)] mod tests;`.
2. Move all ordinary/f32 inline unit tests in `src/legendre/scalar.rs` to `src/legendre/scalar/tests.rs`; similarly migrate inline tests in `spin`, `scalar_paired`, `scalar_paired/analysis`, `scalar_paired/synthesis`, `numeric`, and other implementation modules as applicable. Preserve related test-private imports, helper scope and feature/cfg conditions. Where there were `tests` and `f32_tests`, fold them under the one `tests` module with logical subgroups if necessary; update any test-filter selectors/harness references affected by the module path.
3. Keep `src/transform/tests.rs`, `src/transform/spin/tests.rs` and `src/transform/spin/hemisphere/tests.rs` where they already match the convention. Splitting large suites into nested test-only files is optional and should not be coupled to scalar/spin math changes.
4. Move test-only benchmark fixtures that use custom `#[path]` relative imports into the correct test-only module subtree **only after** checking harness entrypoints and import paths. Do not treat performance data/report artifacts as Rust unit tests. No silent benchmark removal.
5. Keep **every** existing test's assertions, fixtures, numerical tolerances, coverage, feature gates and ignore status. Compare test enumeration/counts before and after `cargo test --locked --all-features --lib -- --list` (and integration tests where relevant), run representative focused suites and `cargo check`/`cargo clippy`/`cargo fmt`; review module/test name changes for workflow scripts. Do not run expensive timing campaigns for pure source relocation.
6. Commit this test-layout migration separately from the numerical changes, including a short inventory of moved paths and the tests run. Use the existing bounded `/tmp` experiment isolation and save/verify/cleanup policy if a build is needed. Do not use Sol.

## Code pattern

In `src/legendre/scalar.rs`:

```rust
#[cfg(test)]
mod tests;
```

Test assertions and helpers live in `src/legendre/scalar/tests.rs`:

```rust
use super::*;

#[test]
fn scalar_recurrence_matches_reference() {
    // existing unchanged correctness assertions
}
```

**Acceptance:** all internal Rust unit tests use `<module>/tests.rs` as their canonical entry; no inline implementation-file test bodies, no sibling `*_tests.rs`, no custom test `#[path]` except documented non-unit-test interoperability, and no lost test coverage.
