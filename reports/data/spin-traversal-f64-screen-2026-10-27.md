# Native-spin recurrence traversal: focused f64 screen

**Date:** 2026-10-27 UTC  
**Candidate parent:** `291465759133aea1685cb4a99dd5ca3d8da27aae`  
**Candidate patch SHA-256:** `8a8dd5d139b026679c36f3d64be56a1571c2c06528c0886dc3c0e7a48404e00c`  
**DUCC:** pinned 0.42.0 source `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`  
**Host/toolchain:** AMD Ryzen 9 5950X; rustc 1.99.1 (58cf8f9c3 2026-10-13); Rust `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.

## Method and correction

Three alternating parent/candidate/DUCC process pairs were run for GL full-mmax spin-1 analysis T720 q1/q8 and synthesis T1023 q1/q8. Each worker targeted 25 ms batches and checked repeat/reset behavior. Parent and candidate outputs were byte-identical and finite in every initial pair. A follow-up added three more T720 analysis pairs per thread count. Its q8 samples had elevated scheduler wait and were excluded from timing conclusions; the follow-up q1 samples remain wait-affected secondary evidence.

The initial runner mistakenly reused q1 pair records while computing q8 summaries. The scratch JSON summaries are therefore invalid. The durable JSON sidecar recomputes the summaries from the per-process CSV rows, with each case/thread group isolated. The CSV preserves all initial and follow-up process medians, host wait/frequency diagnostics, output hashes, and timing-admissibility labels.

## Corrected paired results

| Case | q | Pairs used | Candidate / parent ratios | Interpretation |
|---|---:|---:|---|---|
| T720 spin-1 analysis | 1 | 3 | 1.022765, 1.001831, 1.017162 | Wait-affected q1 evidence; no speed claim. |
| T720 spin-1 analysis | 8 | 2 | 1.005441, 0.999507 (excluded pair 3) | Low-wait paired sample; too few pairs for a speed claim. |
| T1023 spin-1 synthesis | 1 | 3 | 0.997885, 1.016977, 0.992083 | Wait-affected q1 evidence; no speed claim. |
| T1023 spin-1 synthesis | 8 | 2 | 0.968127, 0.993088 (excluded pair 3) | Low-wait paired sample; too few pairs for a speed claim. |

T720 q8 uses only the first two pairs: the third candidate had 4.98% scheduler wait versus 0.54% for parent. Their median ratio is 1.002474. The added q8 follow-up had scheduler wait from 1.4% to 9.8% and ran while host CPU pressure was elevated, so it adds no reliable timing evidence. T1023 q8 similarly excludes the third pair (candidate scheduler wait 4.98%); two retained pairs lean faster, but are not enough to claim a gain.

T720 q1 ratios trended slightly slower in the first three pairs; the added three pairs were mixed. These q1 runs showed about 6.5–6.9% worker scheduler wait. The paired evidence does not establish a material f64 regression. The low-wait T720 q8 evidence is effectively neutral (+0.25% median across two pairs). No speedup or DUCC parity claim is made.

## Correctness and source decision

Candidate and parent output bytes matched for every screen pair. Every execution returned finite output and passed worker repeat/reset checks. The production change removes the duplicated f32 native-spin scheduling/degree/state-rotation body by routing f32 through the shared typed traversal. The f64 recurrence and pair-normalization SIMD/scaling leaves remain typed and unchanged. Retain the shared traversal: this screen does not show a repeatable material f64 whole-transform slowdown, and the low-wait protected q8 cell is effectively neutral.

Focused validation on the candidate: `cargo test --locked --all-features --lib spin` (95 passed, 0 failed, 11 ignored); `cargo clippy --locked --all-targets --all-features -- -D warnings`; `cargo fmt --all -- --check`; `git diff --check`.

Limitations: this is a narrow f64 screen, not the pinned-DUCC all-cases matrix or f32 timing baseline. The q8 conclusions use two retained pairs because high-wait samples were excluded. Preserve the per-run evidence and do not infer a runtime win.
