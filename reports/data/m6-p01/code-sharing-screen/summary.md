# P01 shared fused-projection code-size screen (rejected)

The P01-only source contains two SIMD projection bodies: the existing fused
scalar route and the checked scalar suffix after a scaled prefix. The
spin-1 diagnosis found unchanged normalized instructions but a repeatable
final-link layout effect, while P01 added about 25 KiB of `.text`. This screen
replaced the duplicated bodies with one out-of-line SIMD projection helper
that retains the suffix normal-range checks. The helper did not reduce the release executable's text size. Its `.text` was
`0x214795` bytes, compared with `0x2141b5` for default-link P01,
`0x2141a5` for ordered-link P01, and `0x20df05` for the pre-P01 baseline. The change therefore increased text by 1,504 bytes over default P01 and
remained 26,768 bytes above baseline. It failed the code-size screen and was
not benchmarked; this result does not claim a performance effect or solve the
T511 spin-1 control. Formatting passed. The release Rust library suite passed 197 tests; 14
diagnostic/high-bandlimit tests were ignored by the existing suite. The full
isolated source file is retained in `source/scalar_paired.rs`; the active
worktree's P01 source was not changed. The executable was built with Rust
1.99.0 using the saved P01-only source snapshot and isolated `CARGO_TARGET_DIR`. No production change was accepted. The T511 spin-1 regression remains
correlated with final-executable layout; its instruction-level mechanism is
still unresolved. See the main M6 report for the paired performance evidence.
