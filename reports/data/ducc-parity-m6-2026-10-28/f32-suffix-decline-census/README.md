# Native-f32 T720 suffix-decline position census — 2026-10-28

**Result:** every declined checked suffix attempt failed on its first pair's product-normal check. No attempt completed a fused pair update before falling back, so the candidate discarded zero completed fused work. The fallback replayed 14.47M / 17.69M pair-lane updates for full q1/q8 and 4.70M / 5.64M for reduced q1/q8. This rules out partial-commit batching as a remedy for the measured declines. It does not yet distinguish exact-zero products from subnormal products.

## Provenance and method

- Production code parent: RustSHT `54b166f2c0698f1c70e88f8679d12fa4376704fe`; the current `dev` HEAD during the census was `ebc39e38de2da17b34f4f35a106e2282ef53daba`, with documentation-only successors to the same production source.
- Diagnostic suffix candidate: the existing rejected candidate, uncompressed patch SHA-256 `67bcd6dfd59da3077636aec1bf1df09284e4663ea5c0043148e6900b977db242`; it is documented in the [packet suffix report](../f32-packet-suffix-candidate/README.md). This census adds temporary decline-position counters only. [`instrumented-candidate.patch.gz`](instrumented-candidate.patch.gz) captures the exact working-tree diff from `ebc39e38` (compressed SHA-256 `c441c5f892b01fc421c889ccd5125ce53fa5d8d284ce955cf61e38bcbb6554a4`); [`decline-counts.json`](decline-counts.json) preserves the compact per-cell and per-worker totals (SHA-256 `db1392e08a8ff18fc59bd9a3527946dac66df009d5919fb4691430a706cb0ed2`).
- DUCC reference: pinned 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; no DUCC timing or profile was collected in this route-count-only experiment.
- Build: release, x86-64-v3, temporary `m6_profile_suffix` instrumentation. Exactly one first transform per worker was instrumented; this was not a timed production binary.
- Fixture: GL T720 f32 scalar synthesis, `phi0=0.23`; full `mmax=720,nphi=1458`, reduced `mmax=180,nphi=378`, matching the prior suffix screen.

## Declined work

| Cell | Declines | Failing reason | Fused pairs completed before decline (min/median/p90/max) | Discarded fused pair-lane updates | Scalar suffix pair-lane updates replayed |
|---|---:|---|---:|---:|---:|
| Full q1 | 8,833 | product | 0 / 0 / 0 / 0 | 0 | 14,469,808 |
| Full q8 | 11,058 | product | 0 / 0 / 0 / 0 | 0 | 17,690,688 |
| Reduced q1 | 1,942 | product | 0 / 0 / 0 / 0 | 0 | 4,700,008 |
| Reduced q8 | 2,350 | product | 0 / 0 / 0 / 0 | 0 | 5,636,200 |

The replay count is the declined suffix's number of remaining degree pairs times eight SIMD lanes; it is work that the fallback must perform, not work wasted by the speculative fused attempt. q8 declines were spread across all eight Rayon workers; exact counts are in the JSON. The timer and schedule are not inferred from these counters.

## Decision and next measurement

Do not add a partial-prefix commit path: the observed attempts have no completed fused updates to save, while commit/replay would add state and output complexity. Before any new handoff candidate, classify first-pair product lanes as exact zero, subnormal, normal or nonfinite, then determine whether a common power-of-two bias has a nonempty safe interval across the remaining recurrence, coefficient products and sums. A normal-or-zero check could safely admit exact zero lanes, but it must be justified by the lane census and retain subnormal/nonfinite fallback. No source optimization or elapsed-time claim follows from this instrumentation.
