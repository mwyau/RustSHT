# RustSHT performance status

**OPEN:** Per-case f64 spin-0/1 parity against pinned native DUCC 0.42.0 remains required across GL/CC/F1, synthesis/analysis, supported mmax values and thread counts. This is not a complete matrix.

- **GL T1023 full spin-1 synthesis q1: PASS for this measured cell.** Candidate 7ef6d6e1: 90.402 ms; exact parent bff7ed62: 173.557 ms; DUCC: 124.899 ms. Candidate/DUCC 0.72×, output rel-L2 2.60e-13. The existing reflected-component adapter now handles all 1,024 q1 orders. See reports/T1023_Q1_ROOT_CAUSE.md.
- **Next:** GL T720 full spin-1 analysis q1 (older accepted snapshot 49.878/29.723 ms), then q8 (6.968/4.251 ms). Both remain open; later measured gaps include CC T720 synthesis q8 full/reduced (1.24×/1.45×), CC T1023 synthesis q8 (1.13×), and GL T511 synthesis q8 (1.33×).
- Preserve the active plan's numerical/adjoint guardrails and finish every UNMEASURED or slower cell before claiming overall parity. Two older reduced-order CC/F1 one-vs-eight-thread tolerance failures remain unresolved. See plans/f64-ducc-parity-all-cases-2026-10-24.md.
