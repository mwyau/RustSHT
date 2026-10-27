# Shared spin-analysis SIMD accumulation: focused f64 screen

**Date:** 2026-10-27 UTC  
**Candidate parent:** `9af7f3b03e1089a769b99efc21f14e29deebf56a`  
**Candidate source diff SHA-256:** `d88aca51d913a9ed3b308f029f5386a2244ea6d05cbce2d42bd687e780230675`  
**DUCC:** pinned 0.42.0 source `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`  
**Host/toolchain:** AMD Ryzen 9 5950X 16-Core Processor; rustc 1.99.1 (58cf8f9c3 2026-10-13); Rust `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.

## Change screened

The candidate replaces the separate f64 and f32 spin-analysis accumulation loops with one shared E/B accumulation formula, SIMD loop, scalar tail loop and reduction. Typed Pulp operations remain in f32/f64 precision leaves. It retains the f32 normal-or-zero input policy and the scalar fallback when SIMD partial sums become nonfinite; f64 retains its normal-only vector guard and existing fallback routing.

## Cases and measurements

The case was GL T720, full mmax, spin 1, analysis. Each process used the same fixture and 25 ms target batches; parent, candidate and pinned DUCC ran in alternating fresh-process order. Every parent/candidate output was bitwise identical and finite; worker repeat/reset checks passed.

| Threads | Pairs | Parent median (ms) | Candidate median (ms) | DUCC median (ms) | Paired candidate/parent | Interpretation |
|---:|---:|---:|---:|---:|---|---|
| 1 | 3 | 36.3283 | 36.2333 | 29.4524 | 0.997383 (0.990981–1.005483) | Wait-affected; secondary only. |
| 8, initial | 3 | 5.2802 | 5.5473 | 4.2536 | 1.027480 (1.008158–1.050577) | Candidate wait/frequency imbalance; retained as descriptive data. |
| 8, extended | 15 | 5.3365 | 5.3304 | 4.2725 | 0.995689 (0.803619–1.061615) | Wide host-noise range; time-inconclusive. |

The initial q8 screen had higher candidate scheduler wait in all three pairs and one candidate frequency median at 1.746 GHz. The 15-pair extension was mixed in both directions: its paired median is 0.995689, with a wide 0.803619–1.061615 range. A sensitivity subset with parent/candidate wait at most 2% and both median frequencies at least 3 GHz contains 10 pairs; its median is 0.992854, range 0.940339–1.030676. These data do not establish a repeatable material f64 slowdown or a runtime gain.

RustSHT remains slower than DUCC in this cell: extended q8 medians are about 1.248× DUCC for the candidate and 1.249× for the parent. The source change does not materially change that gap; no DUCC parity claim is made.

## Decision and validation

Keep the shared accumulator and typed vector-operation leaves. The focused screen found no repeatable material f64 regression; f64 correctness and parent bitwise output are preserved. No f32 timing claim is made.

On the candidate, the full focused spin suite passed before a final trait-bound-only cleanup (95 passed, 0 failed, 11 ignored). After that cleanup, the three f32 accumulation-policy tests and both f32/f64 SIMD-to-scalar accumulation/tail tests passed. `cargo check --locked --workspace --all-targets --all-features`, warning-denied workspace Clippy, `cargo fmt --all`, and `git diff --check` passed on the final candidate source.

## Provenance and limits

The fixture seed was `1416973757` with its input hashes in the JSON. Binary hashes, per-process paired medians, relative output errors, frequencies, scheduler wait and output hashes are preserved in the JSON/CSV. This is one protected f64 cell, not the full DUCC matrix or f32 cumulative screen; no broader performance conclusion follows.
