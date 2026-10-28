# Native-f32 packet suffix handoff screen — rejected

**Decision:** do not promote the packet suffix handoff. It moves 30.3–37.1% of the required T720 degree–latitude terms from the scaled callback into the existing fused f32 kernel, but the saved work was not enough for a reliable whole-transform gain across the four cells. The f32 GL T720 spin-1 synthesis q8 control also regressed by 4.1% in a 15-pair matched-ISA screen. Production remains exact source `54b166f2c0698f1c70e88f8679d12fa4376704fe`; all four scalar-synthesis cells remain **GAP** against DUCC.

## Revisions and method

- Parent production source: RustSHT `54b166f2c0698f1c70e88f8679d12fa4376704fe`.
- Candidate: the isolated patch in [`candidate.patch.gz`](candidate.patch.gz), uncompressed SHA-256 `67bcd6dfd59da3077636aec1bf1df09284e4663ea5c0043148e6900b977db242`.
- Reference: pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host: AMD Ryzen 9 5950X; rustc 1.99.1 / LLVM 23.1.1; GCC 15.2.0. q1 used one fixed physical core and q8 eight fixed physical cores; host-local CPU IDs are omitted. Fixtures were GL T720, `ntheta=721`, `phi0=0.23`, full `mmax=720, nphi=1458`, reduced `mmax=180, nphi=378`. Rust binaries used release settings; DUCC used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` with a non-fast-math driver. A separate matched Rust `-C target-cpu=x86-64-v3` build checked ISA sensitivity.
- The main four-cell screen has 15 interleaved parent/candidate pairs per cell, extended to 30 pairs for reduced q8. Each worker used a fresh process, the same warm-call boundary, reusable plan/workspace, and finite/repeat/reset checks. Whole-transform timing is uninstrumented. Raw rows, fixture hashes, binaries and command provenance are in this folder.

## Whole-transform results

Median paired candidate/parent ratios and bootstrap intervals:

| GL T720 f32 scalar synthesis | Parent/DUCC | Candidate/DUCC | Candidate/parent (95% interval) | Candidate faster |
|---|---:|---:|---:|---:|
| Full q1 | 36.591× | 35.750× | 0.972884 [0.968237, 0.975497] | 14/15 |
| Full q8 | 40.257× | 39.100× | 0.983341 [0.970844, 0.987388] | 12/15 |
| Reduced q1 | 25.432× | 23.878× | 0.940336 [0.937153, 0.941216] | 15/15 |
| Reduced q8 | 45.090× | 45.412× | 1.005500 [0.997988, 1.007973] | 11/30 |

Thus the suffix improved three cells, especially reduced q1, but reduced q8 remained inconclusive and slightly slower by its paired median. A separate x86-64-v3 build gave three-pair candidate/parent ratios of `0.9622`, `0.9679`, `0.9370`, and `0.9938` in the same cell order; this is an ISA cross-check, not a replacement for the 15-pair acceptance data.

One-call Legendre-stage diagnostics gave candidate/parent ratios `0.9874`, `0.9964`, `0.9489`, and `0.9988` for full/reduced q1/q8. The q8 stage changed by less than 0.4% in either mmax, even though the scalar suffix route executed. Whole-transform wins, chiefly q1, were therefore not a q8 algorithmic parity step.

## Route work and remaining cost

The isolated state census found a first recurrence-state-safe transition in 93.8–96.6% of initially scaled SIMD packets. On these random T720 fixtures, the two recurrence states had no later range exit. Successfully completed suffixes covered 37.1%, 31.5%, 35.2%, and 30.3% of all required degree–latitude output slots (full/reduced q1/q8); corresponding fused suffix recurrence updates were 29.81M, 25.32M, 14.24M, and 12.28M. The prefix still performed 16.36M, 19.61M, 5.46M, and 6.40M scalar recurrence updates. The kernel declined 24.4–39.3% of state-ready packets after coefficient/product/sum checks. These are route counters from an instrumented build, not timed-binary counts.

The q8 worker profile is retained in [`q8-worker-profile.csv`](q8-worker-profile.csv), but one-call instrumented worker times are noisy and do not provide a stable load-balance conclusion. The 32-ring scheduler remains unchanged.

Matched q1 Callgrind counts fell from 7.390B to 6.880B Ir at full mmax and from 2.388B to 2.119B at reduced mmax, reductions of 6.9% and 11.3%. Per required output slot the counts were 45.84→42.68 Ir (DUCC 1.618) and 29.41→26.09 Ir (DUCC 1.338). These are instruction references, not elapsed time or cycles. Most significantly, `multiply_component_scaled_with_normal_factor_f32` self Ir stayed exactly 2.726B and the inclusive scaled-complex helper stayed 4.422B in parent and candidate. The transition reduced total work but did not remove the dominant per-contribution scaling cost.

The pinned DUCC path in `init_lambda`, `iter_to_ieee`, `calc_alm2map`, and `alm2map_kernel` advances a scaled prefix, applies one residual scale correction, then accumulates the suffix in an unrolled four-lane f64 AVX2/FMA kernel. On these fixtures it enters that fused suffix without scaled-prefix contributions and pairs reflected GL rings, accumulating both outputs from one recurrence lane. Rust's executed suffix is eight-lane native-f32 AVX2/FMA with dynamic range checks and scalar exceptional fallback; the scaled helper still contains exponent extraction, range branches, and scalar multiply/repair. Disassembly extracts are in [`ducc-avx2-path.txt`](ducc-avx2-path.txt) and [`rust-avx2-suffix.txt`](rust-avx2-suffix.txt). The residual gap is therefore not explained by lack of a safe transition alone: Rust still pays prefix/scaled-callback costs, dynamic packet declines, and approximately twice the recurrence traversal work relative to DUCC's reflected pairing.

## Numerical and protected controls

The complete workspace all-features test suite, warning-denied Clippy, all-target/all-feature check, and formatting check passed on the candidate. The added T720 integration coverage exercised low and sectoral analytic modes, coefficient recovery, reduced-mmax scalar adjoint, repeated calls and poisoned-workspace reset. Existing focused tests covered the subnormal transactional decline, exceptional scaled packet, and partial SIMD tail. Candidate/parent relative L2 was `1.62e-7`–`1.79e-7`, with max absolute difference `2.38e-7`–`2.68e-7`. Analytic-field relative L2 was `8.7303e-6`, coefficient-recovery relative L2 `2.7222e-5`, and reduced scalar-adjoint relative error `9.4449e-7`. Candidate/DUCC errors remained `1.72343e-4` relative L2 / `9.644e-4` max absolute full and `2.72547e-4` / `1.548e-3` reduced.

Protected f64 GL T1023 spin-1 synthesis was bitwise equal in all three pairs: candidate/parent `0.9795` q1 and `1.0064` q8, candidate/DUCC `0.7291` and `0.7387`. The matched-ISA f32 GL T720 spin-1 synthesis q8 control was slower: candidate/parent median `1.040964`, 4/15 candidate-faster pairs, bootstrap 95% interval `[1.003509,1.052176]`; candidate/DUCC was `49.455×` versus parent `48.131×`. The new suffix counter was zero in that spin-1 path, so this is a collateral screen result rather than direct suffix work. Even if binary-layout/timing effects contributed, it fails the no-meaningful-regression gate and is not waived.

## Next measured hypothesis

Do not extend this packet-handoff implementation. The two largest remaining algorithmic differences are (1) Rust's repeated scaled contribution helper, whose self instruction count did not move, and (2) the absence of DUCC's reflected-ring pairing, which can nearly halve recurrence traversal for symmetric GL latitudes. Inspect the existing synthesis order/ring scheduling and staging APIs for a bounded shared-state pairing opportunity, quantify exact work and workspace consequences first, then choose one candidate. Keep native-f32 exceptional/polar handling and the accepted 32-ring schedule intact. No production source from this candidate was retained.
