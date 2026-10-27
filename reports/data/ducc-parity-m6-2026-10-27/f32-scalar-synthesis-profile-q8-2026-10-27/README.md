# GL T720 native-f32 scalar synthesis q8 profile — 2026-10-27

## Result

Callgrind counted **13,863,893,209 Ir** in one GL T720 full-mmax native-f32 scalar synthesis call at q8. The measured call used the same `alm0` fixture as the current-head M6 baseline; its output SHA-256 matches the baseline Rust output. The baseline whole-transform timing remains 373.030 ms Rust versus 2.660 ms pinned DUCC, or **139.613×**. Those timings come from the existing rebaseline and are not Callgrind timings.

The largest useful attribution is in the paired f32 recurrence and accumulation: the `synthesize_paired_scalar_order_prepared_f32` callback and the `multiply_complex_scaled_f32` / per-component scale helpers account for several billion self-attributed instructions. The report JSON contains the selected self-attribution rows and the inlining/build provenance. The inclusive `WorkerThread::wait_until_cold` row covers nested Rayon task work; it does not mean the transform spent that share idle.

This is a Rust-only instruction profile. No DUCC instruction profile or new numerical comparison was run. Existing baseline errors for this cell are max relative L2 `1.72342e-4` and max absolute `9.64404e-4`; there is no T720-specific acceptance threshold.

## Method and provenance

- Production source: exact RustSHT `98adf1e6b5334021bfe6ae9abb618c6be167a822`; production source files were unchanged.
- Pinned DUCC reference in the same-source baseline: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0).
- Build: Cargo/rustc 1.99.1, LLVM 23.1.1, `--locked --release --features parallel`, release debug info enabled for source attribution; no target CPU override. Pulp's V3 SIMD implementation is present in the profile.
- Callgrind 3.26.0 counted `Ir` only, with cache and branch simulation disabled. Instrumentation started immediately before the measured `synthesis_into` and stopped after it. The q8 call ran with a fixed eight-CPU affinity set; identifiers are omitted.
- The worker completed one initial call, two bitwise repeat checks, and a poisoned-output reset check before profiling. The profiled output was finite and bitwise equal to the unprofiled output.
- The exact scratch-only [Rust harness](callgrind-harness.rs) and [C client-request shim](callgrind-shim.c) are retained. The original 97,741-byte raw profile was summarized into [`profile-summary.json`](profile-summary.json) and removed with the experiment scratch.

## Profile-directed next step

Screen a native-f32 fast path in `multiply_complex_scaled_f32` for the common zero-exponent case. Keep the existing scaled/exceptional route for cases that do not satisfy the fast-path proof, and require the exact f32 scalar reference tests plus matched q1/q8 whole-transform measurements before promotion. The f64 route is outside this change.

## Cleanup

The report, summary, and exact harness/shim were saved and verified before removing the experiment-owned target, Cargo home, fixture, worker binaries, raw profiles, and temporary worktree. Cleanup increased `/tmp` free space by 468,942,848 bytes and free inodes by 5,652; no profiler or benchmark process remained.
