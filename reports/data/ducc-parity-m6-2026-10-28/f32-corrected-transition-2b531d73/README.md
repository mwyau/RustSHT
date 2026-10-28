# Native-f32 packet transition screen — exact parent `2b531d73`

**Decision: rejected; no production source promoted.** The packetized fallback made a modest synthesis improvement in three primary cells, but reduced q8 was neutral and adjacent f32 scalar analysis regressed by about 3.9% at both q1 and q8 across 15 paired comparisons. The corrected route made no nonzero-exponent transition: every successful suffix began after the shared normalizer had reduced every lane exponent to zero, and removing the `normal_range` gate exposed no additional states. It neither reproduces DUCC's scaled-prefix handoff nor meets the no-regression gate.

## Sources and method

- Exact parent: `2b531d73049fdbc6e1b94b2132bfb9cb3476d319`, equal to `origin/dev` when fetched before the experiment.
- Candidate V1 patch SHA-256: `3ffceb2b04e5093777e8ad7be5dd7c3930f3250b85512a161b6876bfb99c22fd`.
- Candidate V2, which removed the `normal_range` prerequisite: patch SHA-256 `9bde31c431a71ad3d162b4b61674529c4ebc5b433fca953d35504a0944127c31`. V2 was route-profiled only, not timed.
- Pinned DUCC: 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust: 1.99.1 / LLVM 23.1.1, `--locked --release`, `-C target-cpu=x86-64-v3`; runtime Pulp V3 AVX2/FMA. DUCC: GCC 15.2, `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, driver `-fno-fast-math`.
- GL T720 (`ntheta=721`, `phi0=0.23`), full `mmax=720`, `nphi=1458`, and reduced `mmax=180`, `nphi=378`. q1 was pinned to CPU 3; q8 to CPUs 8–15. The same deterministic fixtures and warm-call boundaries were used for parent, candidate, and DUCC.
- Fifteen interleaved fresh-process pairs ran per primary cell. No samples were removed. Paired confidence intervals are percentile bootstrap intervals over the 15 candidate/parent process ratios.

Exact primary pair results, output checks, route profiles, stage samples, build/input hashes, adjacent screens, and both recoverable patches are beside this report in `paired-results.json`, `route-counts.json`, `stage-samples.json`, `build-provenance.json`, `adjacent-screen-3pairs.json`, `scalar-analysis-extension-12pairs.json`, and the compressed patches.

## Whole-transform results

Times are medians of process medians; ratios and confidence intervals are paired.

| GL T720 f32 scalar synthesis | Parent ms | Candidate ms | DUCC ms | Candidate/parent [95% CI] | Candidate/DUCC | Faster pairs |
|---|---:|---:|---:|---:|---:|---:|
| Full q1 | 680.737 | 643.320 | 20.397 | 0.94630 [0.94251, 0.95308] | 31.59× | 14/15 |
| Full q8 | 100.357 | 96.367 | 2.873 | 0.95932 [0.94338, 0.97653] | 33.32× | 15/15 |
| Reduced q1 | 202.371 | 184.452 | 8.894 | 0.91721 [0.91458, 0.91986] | 20.88× | 15/15 |
| Reduced q8 | 53.510 | 53.061 | 1.324 | 0.98741 [0.93760, 1.00305] | 40.93× | 11/15 |

The q1 whole-call gains are 5.4% full and 8.3% reduced. Full q8 improved 4.1%. Reduced q8 is inconclusive and effectively neutral. All four cells remain large **GAPs**.

The isolated Legendre-phase test measured eight samples per binary/cell, with the test-only phase observer enabled. Full q1 was 652.589 ms parent versus 613.512 ms candidate. Reduced q1 candidate samples ranged 177.954–272.646 ms, too variable to support a stage estimate. q8 medians were 90.954/92.059 ms full and 51.276/51.257 ms reduced. The inverse FFT phase remained below 1.71 ms. These stage samples are diagnostic only; they do not replace the interleaved whole-transform timing evidence.

## Route and handoff evidence

The candidate partitions an already declined 32/64-ring block into native SIMD-width packets, reuses the shared paired recurrence and accumulator, stages output transactionally, and attempts one fused suffix per packet. For full/reduced q1/q8, the fallback-packet route reported:

| Cell | Initial packet fused terms | Scaled callback prefix terms | Fused suffix terms | Suffix packets succeeded / attempted |
|---|---:|---:|---:|---:|
| Full q1 | 15.246M | 16.703M | 59.633M | 22,692 / 22,692 |
| Full q8 | 6.213M | 21.693M | 61.195M | 24,550 / 24,550 |
| Reduced q1 | 6.626M | 5.285M | 26.483M | 6,195 / 6,195 |
| Reduced q8 | 2.699M | 6.479M | 27.159M | 6,559 / 6,559 |

Every attempted suffix completed; there were zero recurrence/contribution declines and zero replays. Crucially, every cell reported `corrected_nonzero_exponent_lanes=0`. V2 removed the `normal_range` prerequisite and repeated the route census; packet and term totals were unchanged. The first state pair that can be corrected into normal native-f32 values still occurs only after the recurrence exponent is zero. This confirms the packet route reduces scalar callback work, but does not reproduce DUCC's early transition while residual scale is nonzero.

## DUCC path and executed code

Pinned DUCC `init_lambda` initializes a double-precision recurrence pair and per-lane scale. `iter_to_ieee` advances the scaled prefix in two-degree pairs and rescales state as needed. `calc_alm2map` accumulates a remaining scaled prefix with correction factors; once the batch reaches its IEEE threshold it applies each lane's correction to both recurrence states once, clears the scale, and enters `alm2map_kernel`. The AVX2 kernel uses four double lanes and unrolls four degree pairs. GL synthesis forms work from northern rings and reconstructs both hemispheres.

Rust's checked suffix does use YMM f32 FMA, but its V3 fallback symbol also contains coefficient/product checks, recurrence range predicates, decline branches, and the packet wrapper. The unprofiled `objdump` symbol is 14,720 bytes, with 3,321 static instructions, 358 static branches, 11 static `vptest`s and 66 FMA instructions; those whole-symbol counts include fallback and tails and are not dynamic work or cycle estimates. The pinned DUCC `alm2map_kernel` extract is 738 bytes and its vector suffix has no per-degree safety branch. The instruction shapes explain a code-generation difference, not elapsed-time causality by themselves.

The existing [f32-I/O/f64-internal diagnostic](../f32-internal-double-current-source/README.md) found the f64 reflected route at 0.75–0.78× DUCC and the same all-ring paired route at 3.7–13.4× DUCC. Its source predates this parent, so use it as an algorithmic diagnostic, not a current-head timing. Its work census attributes another large factor to DUCC's reflected-ring reuse; f32 exponent range alone does not explain the full gap.

## Numerical validation and limits

The candidate passed 23 focused f32 library tests and 12 f32 integration tests (one T1023 integration test remains ignored). The integration set includes the T720 analytic-mode, adjoint, and workspace-reset test, scalar/spin references on GL/CC/F1, and spin adjoints. The library set includes polar decline, overflowing sums, subnormal/out-of-range packets, mixed extreme exponents, partial SIMD tails, and transactional decline for subnormal, overflow, and nonfinite products.

Across the 15 random-fixture pairs, all outputs were finite and repeat/reset consistent. Candidate-parent relative L2 was `1.71e-7`–`1.90e-7`, with maximum absolute difference at most `2.39e-7`. Candidate/DUCC relative L2 remained `1.7234e-4` full and `2.7255e-4` reduced, with maximum absolute errors `9.6441e-4` and `1.5479e-3`.

## Adjacent f32 performance controls and final decision

The first three interleaved pairs screened f32 scalar analysis and spin-1
analysis/synthesis. Scalar-analysis outputs were bitwise equal to parent in
those initial pairs. Because both scalar-analysis cells showed a slowdown, the
comparison was extended to 15 pairs per q setting. The confidence intervals
below are percentile bootstraps over paired candidate/parent ratios.

| Adjacent GL T720 f32 path | Parent / candidate / DUCC median ms | Candidate/parent [95% CI] | Candidate faster pairs |
|---|---:|---:|---:|
| Scalar analysis q1, 15 pairs | 545.836 / 567.252 / 14.017 | 1.03892 [1.03697, 1.04042] | 1/15 |
| Scalar analysis q8, 15 pairs | 72.382 / 75.241 / 2.015 | 1.03950 [1.03518, 1.04559] | 0/15 |
| Spin-1 analysis q1, 3 pairs | 1029.870 / 1025.702 / 29.028 | 0.99765 | 2/3 |
| Spin-1 analysis q8, 3 pairs | 135.843 / 135.690 / 4.882 | 0.99855 | 3/3 |
| Spin-1 synthesis q1, 3 pairs | 1541.417 / 1512.230 / 41.934 | 0.98605 | 2/3 |
| Spin-1 synthesis q8, 3 pairs | 270.164 / 269.924 / 5.585 | 0.99911 | 2/3 |

The scalar-analysis slowdown is repeatable at both thread settings. Its cause
was not isolated; a shared-codegen or layout effect is only a hypothesis. The
protected f64 T1023 spin-1 timing was not run after this clear adjacent f32
regression. The transition candidate is rejected despite its small synthesis
gains. The active M6 plan's next synthesis hypothesis is reflected-ring reuse;
do not reinstate either candidate patch unchanged. M6 remains active and all
four f32 synthesis cells remain **GAP**.
