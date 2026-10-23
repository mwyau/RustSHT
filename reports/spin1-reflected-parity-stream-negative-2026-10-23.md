# Reflected parity-stream mapping: rejected

## Candidate

This diagnostic tried one full-width AVX2 vector block per reflected latitude pair, carrying the spin-1 recurrence once across both channels and mirrored hemispheres. It accumulated even and odd degree parity separately and reconstructed each channel after the degree loop. It did not use degree tiles or the earlier half-width category-lane mapping. Checked scaled transitions, polar fallback, and block fallback to the existing safe path were retained.

The focused test `spin1_order_major_synthesis_reuses_workspace_across_changed_coefficients` passed for the GL, CC, and F1 T720 reduced-order cases at q8, including changed coefficients, sparse values, extreme finite inputs, subnormals, and route execution.

## Bounded CC T720 screen

The full-order CC case used `lmax=mmax=720`, `ntheta=722`, `nphi=1458`, q8, and ten calibrated batches targeting 25 ms each. Candidate and parent used one call per batch; DUCC used three. The candidate binary was built from the local candidate changes on source parent `61ae7717`; DUCC was pinned at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. This was a rejection screen, not final performance evidence.

| Backend | Median | Range | Relative L2 error vs candidate, map0 / map1 |
| --- | ---: | ---: | ---: |
| Candidate | 41.633 ms | 36.187–54.815 ms | 0 / 0 |
| Parent | 25.873 ms | 24.772–29.971 ms | 1.49e-13 / 1.01e-13 |
| DUCC | 9.265 ms | 9.106–11.952 ms | 3.09e-13 / 2.54e-13 |

The candidate was about 61% slower than the parent. Its p10 remained above the parent's p90, so the screen was sufficient to reject it without reduced-order, F1, q1, or T1023 follow-up. Host noise was higher during the candidate run (13.6% wait and 3,194 involuntary switches versus 3.4% and 932 for the parent), but does not explain the separated ranges. Outputs stayed finite and within the existing numerical tolerance; parent and DUCC maximum absolute errors versus candidate were respectively `1.57e-9` / `9.58e-10` and `3.16e-9` / `1.99e-9` for map0 / map1.

Raw screen data and detailed binary/fixture hashes remain local and were not pushed; the timing and numerical summary is recorded above.

## Bounded codegen diagnosis

Inspection of the candidate's AVX2 `ReflectedPairParityBlock` loop (symbol `0x1aad70`, loop near `0x1ab030–0x1ab23f`) identifies register pressure as the source-level failure mechanism:

- **Recurrence work:** the two channels and mirrored hemispheres do share one recurrence, so the mapping removes duplicated recurrence advancement.
- **SIMD lanes:** the main loop retains four f64 latitude lanes per YMM vector; this is not the half-width lane-utilization failure of the rejected C64 mapping.
- **Accumulation and registers:** the even/odd sums for two channels require eight live vector accumulators. The compiler broadcasts all eight coefficient vectors together near `0x1ab0f9–0x1ab128`, then emits FMAs through stack-resident operands at `rsp+0x110`, `+0x130`, `+0x150`, `+0x170`, and `+0x190`, storing accumulator vectors back to those slots inside the degree-pair loop. At least five of the eight accumulators are therefore round-tripped through the stack on each pair. The full-width loop pays for eight parity FMAs per pair while register spills erase the benefit of sharing recurrence work.
- **Reconstruction:** reconstruction happens after the degree loop. The bounded disassembly does not show it dominating; no separate timing was collected for it.

This distinguishes the failure from recurrence duplication and lane underutilization. The material additional cost visible in the generated hot loop is spills caused by the eight parity accumulators and coefficient-vector live ranges. The mapping and its dispatch were removed; no production change was retained or pushed.
