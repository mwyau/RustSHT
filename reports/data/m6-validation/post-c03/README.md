# Post-C03 validation results

These commands ran on 2026-10-17 after C03 and before the final P01 link-layout
screen. The worktree included uncommitted P01 and P02 changes and M1 route
instrumentation. The results apply to that combined worktree and do not accept
either performance candidate.

| Check | Command | Result |
| --- | --- | --- |
| Clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed |
| All features | `cargo test --workspace --all-features` | Passed: 197 unit, 4 extreme FFT integration, 11 f32, 1 workspace compatibility |
| No default features | `cargo test --workspace --no-default-features` | Passed with the same test counts |
| Release | `cargo test --workspace --all-features --release` | Passed with the same Rust test counts |
| Python bindings | `uv run pytest python/tests` | Passed: 20 tests |

Raw command output remains local and untracked; this table retains the validation results.

Each Rust suite ignored 14 unit tests and one opt-in f32 test. Formatting was
checked with `cargo fmt --all -- --check`; that command passed without output. No additional tests were manually skipped.
