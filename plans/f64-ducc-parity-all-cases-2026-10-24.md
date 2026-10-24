# ACTIVE — f64 DUCC 0.42.0 parity across ALL spin-0 and spin-1 workloads

**Owner objective (2026-10-24):** Achieve **native DUCC parity or better in each supported f64 scalar (spin-0) and spin-1 SHT operation**. A single faster case does not compensate for a slower one. "Within 2x", "near parity", and a few wins are not completion. Only stop when representative coverage is verified and no supported cell remains slower, unmeasured or inconclusive; explicitly document unsupported cases and genuine external blockers.

**Pinned reference:** `mreineck/ducc@42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (DUCC 0.42.0). Use the exact same CPU/threads, data layouts and warmup/sampling for the candidate and DUCC; record rust SHA/date, timings and uncertainty. Keep all numerical/adjoint/API contracts intact, including S/A/S*/A*, spin-0/1, poles/equator, odd/even longitude, phase, reduced order, extreme/subnormal/nonfinite, poisoned workspace and transactional fallback. No loosened tolerances or degraded other-spin results.

## Completion matrix and definition

- **Grids:** GL, CC, F1. **Spins:** 0 and 1. **Transforms:** synthesis (S) and analysis (A); separately measure adjoint pathways (S*/A*) where they have materially distinct kernels, always test numerical identities.
- **Bandlimits:** representative T255, T511, T720, T1023 where supported. **Spectral order:** full and reduced (e.g., mmax=180 at T720). **Thread counts:** q1, q2, q4, q8. Also exercise representative odd/even longitudes and polar/equator layouts for correctness; add speed tests when layouts switch implementation.
- Mark every combination **PASS** (verified reproducibly <= DUCC latency), **GAP** (> DUCC), **UNMEASURED**, **INCONCLUSIVE** (noise/contended host) or **UNSUPPORTED** (clear reason). Include median Rust/DUCC ms, ratio, exact SHA/protocol and correctness status. Preserve known passes, but remeasure stale results on final source.
- **No global pass while any required cell is GAP, UNMEASURED or INCONCLUSIVE.** Do not fabricate a performance tolerance: if small differences overlap measurement noise, repeat quietly rather than automatically treating 1.01x as adequate. The final verification may require a modest, bounded matrix run; **do not** run a whole matrix after every small patch.

## Open backlog — selected *historical*, non-uniform evidence

| Spin-1 workload | Rust / DUCC ms | Ratio | Absolute excess | State |
|---|---:|---:|---:|---|
| GL T1023 full synthesis q1 | 176.605 / 125.492 | 1.41x | +51.113 ms | GAP |
| GL T720 full analysis q1 | 51.536 / 29.414 | 1.75x | +22.122 ms | GAP |
| GL T720 full analysis q8 | 6.968 / 4.251 | 1.64x | +2.716 ms | GAP (candidate `f1f4589b`) |
| CC T720 full analysis q8 | 20.590 / 20.942 | 0.98x | −0.353 ms | PASS for this cell (candidate `f1f4589b`) |
| CC T1023 full synthesis q8 | 24.634 / 21.724 | 1.13x | +2.910 ms | GAP |
| CC T720 full synthesis q8 | 7.534 / 6.091 | 1.24x | +1.443 ms | GAP |
| CC T720 m180 synthesis q8 | 3.850 / 2.662 | 1.45x | +1.188 ms | GAP |
| GL T511 full synthesis q8 | 3.175 / 2.384 | 1.33x | +0.791 ms | GAP |
| F1/GL T720 full synthesis q8; GL T1023 synthesis q8 | selected 0.94–0.97x | faster | — | PASS for those measured snapshots only |
| Other supported spin-0/spin-1 sizes, grids, threads, order cuts and operations | not yet shown in one source-matched matrix | — | — | UNMEASURED |

**Current mixed Q/U scaled-accumulation candidate (`f1f4589b`):** three interleaved parent/candidate/DUCC process triples, 30 calibrated batches/backend, AMD Ryzen 9 5950X, q8 pinned to CPUs 0–7. Exact parent source `53ef4f2b` binary SHA-256 `1edd6f97b93075c11d2bd32ade9b59fb621cec565f4b560f04f7e814fbd02d67`; candidate binary SHA-256 `41402baa1be46485461d11bbbd72152c3c66d851e3349046d03139aea7af773b`; DUCC binary SHA-256 `89125091d06ddd4c24589c6f53e6785682d79f40cf0b991f64ad75a034ae2423`. GL medians: candidate/parent/DUCC `6.968/7.293/4.251 ms`; paired candidate/parent median ratio `0.956×`. CC: `20.590/21.173/20.942 ms`; paired candidate/parent median ratio `0.983×`. Both candidate outputs were bitwise identical to parent; relative L2 to DUCC was `4.76e-14` (GL) and `6.05e-14` (CC), with repeat/reset checks passing. GL remains a gap; the measured CC cell is below DUCC in all three paired process comparisons.

**Action ordering, not exemption:** maximize absolute time saved first with broadly reusable code, then close even small residuals. Finish the running bounded experiment before redirecting. GL T1023 q1 synthesis is currently the biggest measured excess; inspect checked physical-latitude prefix/ordinary-suffix path in `src/legendre/scalar_paired.rs`, retaining accepted AVX2 exponent scaling and proper fallback. GL T720 q1/q8 analysis is next: `src/spin_transform.rs::analyze_spin_scalar_reflected_order` and scalar_paired checked recurrence/accumulation. Targeted GL q8 analysis profiling found **599/3,579** mixed/scaled recurrence blocks over **475/721** order leaves, about **80% of estimated recurrence work**; another 256-pair tile regressed, and input-scan fusion, route caching and independent replay did not win. Investigate scaled recurrence math/codegen itself rather than repeat retired mappings.

**Experiment rhythm:** one distinct mechanism and tested production patch -> exact-parent/candidate/DUCC small A/B -> accept if correct and reproducibly beneficial, otherwise one targeted diagnosis/fix -> select next source-grounded hypothesis. No endless variants of a rejected mapping, no broad benchmark campaign in lieu of optimization, and no arbitrary stopping ratio. Keep `PERFORMANCE_STATUS.md` short, with any matrix ledger maintained here as concise status; commit/reconcile/push safely, no force push and no main modifications.

**Correctness debt:** two reduced-order T720 CC/F1 single-versus-multi-thread comparison failures were confirmed on parent revisions. Diagnose exact discrepancy and internal coefficient-cache assertion independently; do **not** waive, mask or loosen tolerance. Reviewer/subagent only with explicit user request. f32 and spin>1 optimization paused until f64 spin-0/1 parity is established.
