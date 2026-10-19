# Three foundational SHT papers: methods versus RustSHT production source

Reviewed 2026-10-18/19 against opt/sht-performance-parity at source SHA ff6f3427af7939d0b98ebe8c667a7aae36a4f56f. **This is an evidence crosswalk, not a standing implementation queue.** Recheck recent commits before assigning any work. Continue the P2 scalar-analysis and P3 spin-1 priorities in [the active roadmap](05-validation-and-milestones.md).

## Papers audited

1. Reinecke & Seljebotn, **Libsharp — spherical harmonic transforms revisited**, A&A 554 A112 (2013): [paper PDF](https://arxiv.org/pdf/1303.4945), [DOI](https://doi.org/10.1051/0004-6361/201321494). Methods in §§2–5, Figs. 1–4.
2. Keiichi Ishioka, **A New Recurrence Formula for Efficient Computation of Spherical Harmonic Transform**, JMSJ 96, 241–249 (2018): [journal PDF](https://www.jstage.jst.go.jp/article/jmsj/96/2/96_2018-019/_pdf/-char/en), [DOI](https://doi.org/10.2151/jmsj.2018-019). Recurrence eq. 26, coefficient changes/hemisphere symmetry eqs. 27–37, scaled polar starts eqs. 41–49, implementation §5.
3. Reinecke, Belkner & Carron, **Improved cosmic microwave background (de-)lensing using general spherical harmonic transforms**, A&A 678 A165 (2023): [paper PDF](https://arxiv.org/pdf/2304.10431), [DOI](https://doi.org/10.1051/0004-6361/202346717). General 2D Fourier-sphere/NUFFT §3, minimal-ring Clenshaw–Curtis analysis Appendix A, lensing adjoint vs inverse Appendix B.

**Classification:** implemented = identifiable production code; partial = limited regimes or alternative algorithm; not found = not in inspected public core, not a claim about experimental branches; alternative = paper's particular method not used because RustSHT already has another valid technique; out of scope = new problem/API, not a defect. Implemented does **not** mean DUCC-speed parity. Code paths are actual RustSHT files checked at the source SHA above.

## 2013 libsharp inventory

| Published method | RustSHT status, evidence | Next action |
| --- | --- | --- |
| Split longitude FFT and Legendre stages (§2) | **Implemented**: [fft](../src/fft.rs), [transform](../src/transform.rs), [spin_transform](../src/spin_transform.rs) | None |
| Portable scalar/SSE/AVX/FMA vector abstraction (§3.1) | **Implemented** with pulp: [scalar_ordered](../src/legendre/scalar_ordered.rs), [scalar](../src/legendre/scalar.rs), [spin](../src/legendre/spin.rs), f32 variants | Retain scalar portable fallback |
| Fuse per-degree recurrence with accumulation, avoid full Legendre-vector materialization (Figs. 1–2, §3.2) | **Implemented for scalar synthesis; partial analysis/spin**: ordered SIMD suffix; [scalar_paired](../src/legendre/scalar_paired.rs) fused analysis blocks; [spin](../src/legendre/spin.rs) visitor/fused paths | **P2/P3** improve eligible analysis and spin hot loops, not rewrite mathematics |
| Several latitude rings per SIMD block (§3.2) | **Implemented**: ordered packed/reflected rings, SIMD padded tails, scratch | None |
| First-use empirical vector-block autotuning (about 0.1 sec, §3.2) | **Not implemented as described**: static/benchmark-derived selection in [plan](../src/plan.rs) and [scalar_table](../src/legendre/scalar_table.rs), not on-device trial of ring block length | **A1 optional** only if benefit exceeds setup/noise |
| Same-spin, same-direction **multiple simultaneous jobs** sharing one recurrence within each degree (Figs. 1–2) | **Not implemented as one fused algorithm**: [m5_batch_bench](../examples/m5_batch_bench.rs) loops separate SHT calls. Multiple tasks/plan reuse do not equal shared recurrence across fields | **B1 recommended independent throughput experiment** |
| Polar optimization by theoretical cutoff involving m, spin, latitude and L; T=max(100,0.01L) (§3.3) | **Implemented equivalent**: [scalar::polar_order_limit](../src/legendre/scalar.rs), scalar_f32, active order ring limits in transform | Verify, do not reimplement |
| Spin-weighted Wigner-d three-term recurrence, general spin (§4.1, eqs. 10–11) | **Implemented; performance partial**: [spin](../src/legendre/spin.rs), [spin_f32](../src/legendre/spin_f32.rs), spin_transform (supported spins 1–4) | **P3** improve actual spin-1 execution |
| Shared-memory order scheduling with load balancing (Figs. 1–2) | **Implemented**: Rayon dynamic m-order queue and worker scratch in transform; spin analysis dynamic scheduling | **P1/P2** optimize only measured remaining overhead |
| MPI distributed rings/orders, all-to-all spectrum transpose (§4.2, Figs. 3–4) | **Not implemented** | **Out of scope**, single node by design |
| First derivatives from scalar coefficients via spin-1 (§4.3) | **Implemented**: synthesis_deriv1_into, synthesis_gradient_into in spin_transform | Keep analytic/adjoint checks |
| Fejer first rule and pole-including Clenshaw–Curtis (§4.4.1) | **Implemented**: [grid](../src/grid.rs), [meridional](../src/meridional.rs) | None |
| Fejer **second** rule (§4.4.1) | **Not implemented**: GridKind only GL/CC/F1 | Optional new grid, not a performance fix |
| Reduced Gauss–Legendre with variable nphi per ring (§4.4.2) | **Not implemented**: one nphi and phi0 for all rings | Optional map-memory feature; paper says little additional compute speedup with polar skip |
| Arbitrary iso-latitude/partial-sky rings, per-ring phi0, custom quadrature weights (§4.4) | **Not implemented**: fixed three geometry families | Defer new geometry API |
| Synthesis and analysis adjoints, real spherical harmonic convention (§4.5) | **Implemented** for supported combinations: transform, spin_transform, [numerical contracts](02-numerics-and-grids.md) | Preserve separate S,A,S*,A* |
| Independent spin/high-L numerical tests (§5) | **Substantial coverage, not exhaustive**: unit, analytic, pinned DUCC and operator tests | Broaden only alongside relevant code |

## 2018 Ishioka inventory

| Published method | RustSHT status, evidence | Next action |
| --- | --- | --- |
| Evaluate Legendre recurrence on the fly, not a giant precomputed dense basis (§3) | **Implemented for high L** in scalar_ordered and scalar_paired; [compact-table path](../src/legendre/scalar_table.rs) deliberately restricted to favorable moderate L | Preserve cost-based table use |
| Paired-degree transformed recurrence, two-FMA ordinary step (eq. 26) | **Implemented**: scalar_paired fill_factors, prepare_synthesis_coefficients, paired SIMD; scalar_ordered kernel | Not missing |
| Even/odd coefficient transforms, hemispheric symmetry, pair accumulation (eqs. 27–37) | **Implemented synthesis, partial analysis performance**: scalar_paired accumulation, reflected spectrum output, scalar_ordered | **P2** strengthen analysis fused degree/accumulation path |
| Omit negligible polar work, start at eligible ring/degree (§4) | **Implemented in another valid scheme**: polar_order_limit, monotone ring boundary, DUCC-style prefix scaling | Do not mistake for identical Ishioka scaling |
| Ishioka's own deferred polar-start ring indices and stored transition states (eqs. 41–42) | **Alternative not ported**: existing path uses binary-exponent prefix advancement | **A2 optional** only for proven high-L/prefix bottleneck |
| Ishioka log-domain initial sectoral values and beta-scaled recurrence (eqs. 44–49) | **Alternative not ported**: Rust uses DUCC-type ScaledValue/binary exponent scaling in scalar and spin | Do not install second stacked fallback automatically |
| IEEE quad-precision preprocessing of recurrence coefficients and GL quadrature (§4) | **Not implemented as exact method**: f64 coefficients; GL roots via [grid](../src/grid.rs) Newton/Bogaert approximations | **A3 optional** accuracy-driven high-L study |
| Latitude-lane SIMD and dynamic m scheduling (§5) | **Implemented** with pulp and Rayon | No duplicate port |
| Compact FFT backend (§5) | **Functional equivalent**, different library: [fft](../src/fft.rs) uses RustFFT; standalone FFT crate is not integrated | No FFT rewrite absent measured bottleneck |
| Large-L timing and L2/L-infinity error tests (§6) | **Partial full combination matrix**, selected analytic/DUCC and exceptional tests exist | Target missing evidence only when affected |

Crucial: Ishioka's *mathematical* paired recurrence is already present. Its **specific** logarithmic scaling and start-state scheme are *not* the same thing as the DUCC-derived enhanced exponent route. Neither implies the other is numerically inadequate.

## 2023 Reinecke–Belkner–Carron inventory

| Published method | RustSHT status, evidence | Next action |
| --- | --- | --- |
| Normal longitude FFT plus isolatitude SHT (§§2–3) | **Implemented** in transform/spin_transform | None |
| Double Fourier sphere via north–south reflection and spin parity (§3) | **Implemented for CC/F1 resampling**, not arbitrary-point evaluation: [meridional::CCResamplingPlan](../src/meridional.rs) reflects each Fourier order using (-1)^(m+spin) | Keep exact parity tests |
| General 2D Fourier sphere, zero padding and arbitrary-point evaluation (§3) | **Only partial primitives**: separate longitude/meridional FFTs for regular grids; no public arbitrary-point Fourier evaluation | Separate scope |
| Type-2 uniform-to-nonuniform FFT spherical synthesis (§3) | **Not implemented**: no NUFFT plan or arbitrary (theta,phi) inputs | **Out of scope** of regular GL/CC/F1 parity |
| Type-1 nonuniform-to-uniform FFT, **true adjoint** and inverse doubling (§3) | **Not implemented for arbitrary points**: current S* and A* are *regular-grid* adjoints | Defer with type-2 feature |
| Tolerance-tunable NUFFT spreading, deconvolution and oversampling (§3) | **Not implemented**: no accuracy-controlled NUFFT backend | Defer |
| Minimal-ring analysis using ntheta=L+2, reconstruct missing half-rings and apply CC quadrature (Appendix A) | **Implemented via equivalent prepared-CC scheme**: meridional::to_prepared, restore_original, phase shifts, theta FFT and prepared quadrature; [grid](../src/grid.rs) enforces CC and F1 limits | Validate high L low m and both adjoints; **not missing** |
| Appendix A optional fold of midpoint contributions back onto existing rings | **Similar folding exists**, but equivalence to the particular explicit midpoint algorithm is **not proven**; current method is general prepared-CC spectral convolution | Verify mathematically before claiming identical method; no rewrite without evidence |
| Adjoint versus inverse remapping; inverse deflection/Jacobian avoided in lensing adjoint (§2, Appendix B) | **Conceptual distinction supported for ordinary SHT**, but lensing D_alpha is **not implemented** | Out of scope |
| Spin-1 deflection, polarization rotation/parallel transport (§§2–3) | **Spin-1 transform available**, lensing geometry and parallel transport **not implemented** | Out of scope |

**Do not conflate:** Appendix A minimal-ring regular CC/F1 analysis, already present, is **different** from the paper's general arbitrary-coordinate 2D NUFFT synthesis. Neither substitutes for the other.

## Incremental priorities from these papers

**P2 now — scalar analysis:** strengthen *existing* fused paired-degree recurrence/accumulation in scalar_paired; measure normal/transition/exception paths and matched DUCC stages, then implement only largest verified shortfall. P3 spin-1 fused recurrence, coefficient and E/B accumulation next. These are already on [plan 05](05-validation-and-milestones.md).

**B1 — genuinely new useful method, optional separate research:** same-spin/direction **multi-map fusion** from libsharp Figs. 1–2. Compare a truly shared per-order recurrence feeding B=2,4,16,64 fields against repeated independent plan calls. Measure latency, throughput/map, q1/q8 and memory, normal/exception paths; require matching maps/coefs and adjoints before production adoption. Existing batch benchmark does not yet measure a shared mathematical kernel. Do not divert active agent until P2/P3 are sound.

**A1 — conditional:** libsharp-style first-use vector-block autotuning; only if fixed blocks measurably underperform across machines and setup amortizes. Existing static break-even heuristics are not this exact technique.

**A2 — conditional:** compare Ishioka's deferred/logarithmic polar starting states to current DUCC-style binary exponent route in an isolated kernel **only if** post-fix very-high-L prefix time or error still warrants it. No second production fallback chain.

**A3 — conditional:** high-precision factor/quadrature preparation for L>=4095 if actual f64 precision error calls for it. Avoid mandatory quad dependencies without evidence.

**Explicit deferrals:** Fejer-II, variable-longitude reduced GL, custom/partial iso-latitude, MPI, arbitrary-coordinate NUFFT and CMB lensing remapping. These require product/API decisions, not a routine P2 benchmark. Do not add external dependencies or a wholesale FFT rewrite to satisfy a literature checklist.

## Verification and licensing

For new algorithms: use f64/f32, GL/CC/F1 as relevant, odd/even longitude, reduced mmax, phi0, high-l low-m, sectoral/polar rings, SIMD tails, q1/q8, extreme finite/subnormal, and independent analytical/pinned DUCC S/A/S*/A* checks. Bench optimized release artifacts with identical work and disclosed setup costs; account for host interference and memory. Multi-map fusion must actually share one recurrence, not just call the independent operation in a parallel loop.

Paper equations are mathematical methods, but **RustSHT already contains explicitly source-derived DUCC files under GPL-2.0-or-later**. Do not remove attribution or treat this paper audit as a provenance rewrite. See [license review](../docs/sht-literature-and-licensing.md).

**Operational instruction:** keep implementing P2 scalar analysis and P3 spin. Use this plan as a research reference when those kernels or a future optional batch API are under consideration.
