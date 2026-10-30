# RustSHT versus DUCC: native-f32 scalar synthesis diagnosis

Date: 2026-10-29

## Finding

The isolated generic RustSHT f32 kernel and pinned DUCC compute the same scalar spherical-harmonic recurrence, but they do not execute the same precision path. DUCC widens the f32 coefficients and performs coefficient preparation, the Legendre recurrence, and accumulation in f64; it narrows the Fourier coefficient to f32 before its longitude FFT. RustSHT keeps those operations in f32. In addition, RustSHT's f32 seed normalization can send a scaled mantissa through the subnormal range, and its f32 geometry and sectoral constants can differ by one ULP from DUCC's f64 values rounded to f32.

These are measurable causal differences, not a generic “f32 is less precise” explanation:

- Correctly rounded f64 geometry inputs sin(theta) and cos(theta) matched Rust's f32 values in all 42 traced order/ring pairs. The generated cos² value matched the nearest f32 rounding of DUCC's f64 cos² in only 24/42 pairs. The branch was the same in each selected trace.
- Rust's f32 recurrence coefficients a and b matched the nearest-f32-rounded DUCC values in all 14,581 deduplicated recurrence rows.
- Sectoral mfac matched in 21/42 pairs. The three mismatching orders (m=0, 2, 16) were each one ULP from the rounded DUCC value at every sampled ring.
- The first differing operation depends on order and ring. It is seed preparation for the zonal pole and m=8 near-pole cases; cos² preparation and its first recurrence use for m=5/ring 5 and m=1/equator; and the scaled-to-ordinary control flow followed by native-f32 recurrence for m=16/ring 0.
- Rust's vector recurrence matched a scalar fmaf replay with identical rounded operands for every recorded recurrence update. This rules out a separate AVX2 lane or SIMD-only arithmetic error in this trace.
- A binary128 independent recurrence/accumulator matched DUCC's returned f32 Fourier-stage value after correct f32 rounding in all 357 matched cases. Rust's direct generic f32 stage did not.

The controlled substitutions show that a subset of the discrepancy is avoidable in f32 initialization and geometry; the remainder persists in repeated f32 recurrence. They do not add linearly, and no one substitution is a full error decomposition.

## Scope and reproducibility identity

The traced geometry is GL T720: 721 theta values from the Rust GL grid, copied as exact f64 IEEE-754 bits, phi0=0, nphi=34, spin=0, ring factor 1. The direct pre-FFT comparisons use one coefficient at a time: m=0,l=720,real; and m=1,2,5,8,16 with l=600,650,700,719,720 and separate real/imaginary unit inputs. Seven rings were traced: 0, 1, 5, 360, 715, 719, 720. The matched direct-stage set contains 357 order/degree/ring/part cases.

Rust candidate base: b6a96aa7161a64f678b614d73478496f50f0551e. The isolated native-generic candidate source is commit 45380d7e4bd849a33ffef7d91978ee18fd7e4193; its patch is in patches/native-f32-candidate.patch. The original single-mode traces were captured with the sparse trace patch/tree/binary recorded in DATA_PROVENANCE.md. The dense high-band trace extends that test-only instrumentation. The portable replay patch is patches/rust-trace-only.patch; DATA_PROVENANCE.md distinguishes the exact executed patch/tree from the packaged patch, whose only source difference is the default trace-output filename (recorded runs set explicit output paths).

DUCC source base: 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b. Its separate trace-only patch is patches/ducc-trace-instrumentation.patch (SHA-256 289165ee3318654a6c5094ef59d9662009561f13714edec6b9c1c9a6bfaa6f6a); patched source tree SHA: 1bcaa09ba03edb584beec10d2ec94a93ea14ad49.

The independent reference uses IEEE binary128 (113-bit significand) for plan constants, sectoral state, recurrence, and accumulation, with fmaq state updates. The earlier sparse/public-map comparison was cross-checked against an 80-bit long-double evaluation with a maximum output difference of 2.7804e-14. The dense high-band replay has its own independent cross-check below: 1.2544e-16 maximum and identical f32-rounded output bits. No application accuracy tolerance was supplied, so this report gives measured errors and does not label a scientific-accuracy pass or fail.

## Dense upper-band trace and independent reference check

The archived deterministic GL T720 fixture was also traced with only orders m=1..16 and degrees l=600..720 nonzero. It uses the exact same 721 f64 theta bit patterns, phi0=0, spin 0, and unit ring factors. The Rust and DUCC traces use the northern representative rings 0, 1, 5, and 360; their reflected southern outputs are 720, 719, 715, and the equator 360. This yields 64 order/ring recurrence cases and 256 north/south real/imaginary pre-FFT Fourier components. It is the archived high-band deterministic fixture, not a full or reduced random transform.

An independent binary128 replay and an independently coded host x86 extended-precision replay agree on the correctly rounded f32 output bits for all 256 components. Their maximum decimal output difference is 1.2543818651e-16 and RMS is 1.1072784055e-17. DUCC's narrowed f32 Fourier outputs also match the binary128 correctly rounded bits for all 256 components. Its maximum absolute distance to the binary128 value is 1.7140727770e-9 (RMS 2.9647029421e-10), consistent with the final f32 quantization. These checks support the reference and show that this DUCC path is within the final f32 rounding cell on this fixture.

| Dense high-band measure | Result |
|---|---:|
| Rust native-f32 vs binary128, max absolute / RMS | 4.5002376476e-5 / 9.9426882310e-6 |
| Rust native-f32 vs binary128, max / RMS contribution-scaled error | 5.0441416941e-3 / 1.0173322277e-3 |
| Rust vs DUCC differing f32 output components | 256 / 256 |
| First differing recurrence input stage | cos² in 32/64 cases; recurrence state in 32/64 |
| Recurrence `a` or `b` differing from nearest-f32 binary128 values | 0 / 22,768 updates |
| Native f32 scaled-prefix updates | 0 / 64 cases; ordinary recurrence starts at degree m |
| Prepared f32 pair values not correctly rounded from binary128 | 1,472 / 3,936; maximum absolute difference 2.5290482063e-10 |

Representative first operations are traceable to exact bits. At m=1/ring 0, cos², `a`, `b`, and previous state agree after f32 rounding; the seed/current state is `ba96effe` in Rust versus `ba96effd` from binary128, the leading contraction is `4036da86` versus `4036da85`, and the next state is `bb579eec` versus `bb579ee9`. At m=5/ring 5, the first plan-input difference is cos² (`3f7fd6e4` versus `3f7fd6e5`); at the first recurrence record the leading value differs by one ULP and the next state by three. At m=12/ring 5, the same one-ULP cos² difference is present at the first recurrence record; by degree 600 the Rust current state differs from binary128 by 0.0017044143240 (0.1182% of that state), with a 14,298-ULP distance. ULP counts near zero are reported as bit diagnostics only; absolute and contribution-scaled errors are more informative there.

| m=12, ring 5 state | Rust f32 | Binary128 | Absolute error | Relative error |
|---|---:|---:|---:|---:|
| degree 600 | 1.44036900997 | 1.44207342430 | 1.7044143240e-3 | 1.181919e-3 |
| degree 612 (largest sampled absolute state error) | -1.26486361027 | -1.26659195070 | 1.7283404342e-3 | 1.364560e-3 |
| degree 650 | -0.28484478593 | -0.28578705876 | 9.4227282769e-4 | 3.297115e-3 |
| degree 700 | 0.81966888905 | 0.81930541659 | 3.6347245112e-4 | 4.436349e-4 |
| degree 720 | -1.19431293011 | -1.19456842357 | 2.5549345851e-4 | 2.138793e-4 |

The degree trajectory is oscillatory: the state error grows through degree 612 and then decreases, so one final relative-error value does not describe its path. Across all high-band cases the maximum absolute current-state error is 0.07408106979; the maximum bit-distance is 9,388,455 ULP, often at small values where that metric is ill-conditioned.

Controlled substitutions identify which stages dominate without adding one-change deltas:

- Replacing only Rust's coefficient pairs with binary128 pairs while retaining the actual Rust recurrence states reduces the maximum output error from about 4.50e-5 only to 4.5001449177e-5. Pair preparation contributes, but does not explain the dominant discrepancy.
- Replacing the recurrence states with binary128 states while keeping Rust's prepared pairs reduces maximum error to 1.2370150951e-7 (RMS 2.6337949606e-8). Conversely, retaining Rust recurrence states with binary128 pairs remains at 4.5001449177e-5. This makes the recurrence path the dominant cause for this fixture.
- With binary128 recurrence and pairs, using Rust's f32 cosine only for north/south reconstruction gives a maximum 2.8201799800e-10 output difference; using the binary128 cosine gives 7.1284324848e-35. Reflection cosine rounding is therefore small here.
- Summing the exact same native-f32 recurrence/coefficient products in binary64 changes the final reflected output by at most 3.4691765904e-8 (RMS 9.0569362404e-9). The maximum error remaining after that substitution is 4.5001445153e-5. Accumulation is secondary on this fixture, though individual sum condition numbers reach 32,666.97.

All mixed-precision substitutions are offline diagnostic calculations. They are not production backends. They are also not additive: recurrence, rounded factors, sectoral state, and coefficient preparation interact. In particular, the sparse m=8 seed-underflow and m=16 scale-transition cases remain distinct from the dense upper-band controls.

## First divergence and final pre-FFT error

All values below are the real north Fourier coefficient before the longitude FFT. The reference column is the binary128 result. Relative error is relative to that value. Ring-scaled error is absolute error divided by the maximum absolute binary128 reference across the seven traced rings for the same order, degree, and component.

| Order, degree, ring | Earliest observed divergence | Rust f32 | DUCC output f32 | Binary128 | Rust absolute / relative / ring-scaled error |
|---|---|---:|---:|---:|---:|
| m=0,l=720,r=0 | Sectoral norm/seed: Rust 3e906eba; correctly rounded high-precision value 3e906ebb. First recurrence leading at degree 2 is also one ULP different. | 0.018705474212765694 | 0.018542390316724777 | 0.018542391016906403 | 1.63083196e-4 / 8.79515e-3 / 5.12341e-4 |
| m=5,l=720,r=5 | Seed is one ULP low versus the rounded DUCC seed; at degree 7, cos² is one ULP low and the leading factor differs by one ULP. | 1.6049379110336304 | 1.6069238185882568 | 1.6069238510049764 | 1.98593997e-3 / 1.23586e-3 / 5.75554e-4 |
| m=8,l=720,r=1 | Seed restoration: Rust 22df4000 versus 22df3b40 for the exact product from Rust's already-rounded f32 inputs; DUCC's f64 seed rounded to f32 is 22df3b44. | 0.36547181010246277 | 0.3654029071331024 | 0.3654029017289601 | 6.89083735e-5 / 1.88582e-4 / 3.31436e-5 |
| m=16,l=720,r=0 | Rust enters its ordinary f32 loop at degree 16 with no scaled-prefix recurrence updates; DUCC performs 136 prefix updates and enters ordinary recurrence at degree 288. First common ordinary leading differs by one ULP at degree 290. | 8.768833749395188e-12 | 8.766820602801317e-12 | 8.766820552540033e-12 | 2.01319686e-15 / 2.29638e-4 / 7.19764e-16 |
| m=1,l=719,r=360 | At degree 3, cos² differs by one ULP (Rust 099bbe75; DUCC cos² rounded to f32 099bbe74); first differing next state is degree 5. | 0.3189089894294739 | 0.3183099925518036 | 0.3183100014732962 | 5.98987956e-4 / 1.88178e-3 / 1.07524e-4 |

For the zonal high-degree pole, the maximum direct generic-stage absolute error over the 357 cases is 9.56558577e-3 (m=2,l=720,r=0, imaginary); maximum relative error is 5.79836e-2 (m=0,l=720,r=719), where the local value is small; maximum ring-scaled error is 8.66673e-3 (m=0,l=720,r=5). The ring-scaled metric keeps small local values from making relative error alone dominate.

The m=0/ring 0 trace shows how a tiny seed difference is amplified without a large first-step mistake. At degree 2, the state error is 2.5658e-8 and one ULP. By degree 100 it is 1.00387e-4 (421 ULP, 3.41e-5 of the local state envelope); at degree 360 it is 1.71136e-4 (718 ULP, 5.81e-5 of the envelope). Near the high-degree root, the state update becomes cancellation-sensitive: at degree 718 its state-update condition number is 6.9996 and envelope-normalized error is 1.4699e-3; at degree 720 the condition number is 6.7850e13, the next-state absolute error is 9.0461e-5, and its envelope-normalized error is 2.1929e-3. Relative error and ULP counts at that near-zero next state are not useful by themselves.

For m=5/ring 5 the largest recorded recurrence update condition number is 61.7734 at degree 491. The maximum envelope-normalized state error is 1.6663e-2 at degree 627; the final output relative error is 1.2359e-3. This trajectory is oscillatory and locally cancellation-sensitive, rather than a smooth fixed percentage drift.

For m=16/ring 0, at the first comparable ordinary update (degree 290), the f32 leading factor is 404d7d87 versus 404d7d86 for the rounded DUCC value. The resulting f32 next state is 220e9dc1 versus 220e9caf, 274 ULP apart. Because this state is tiny, its envelope-normalized error (2.9350e-5) is more informative than the ULP count. The final polar coefficient differs by 2.2964e-4 relative but only 7.1976e-16 of the maximum reference amplitude for that ring/order/degree.

## What the controlled substitutions establish

The controlled-v2 data applies one substitution at a time. It includes the native f32 result, corrected seed from the already-rounded Rust inputs, DUCC seed rounded to f32, DUCC-rounded cos² and a/b values, f64 recurrence on rounded Rust operands, f64 accumulation with the f32 recurrence state, and alternate FMA/separate-rounding orderings.

- m=8,l=720,r=1: replacing only the seed with the correctly rounded product from Rust's f32 sine and f32 sectoral norm changes output from 0.3654718101 to 0.3654432297. Relative error falls from 1.88582e-4 to 1.10366e-4, a 41.5% reduction. The f32 initialization created the larger error: power bits 4b588c98 with scale -1, prefactor 4adf3b40; normalization applied 2^-80 twice, leaving the subnormal mantissa 00000df4 with scale 1. Restoring by 2^80 produced 22df4000. The exact product of the rounded f32 inputs is 22df3b40. The issue is that a 2^-80 scale chunk crosses the f32 normal range; it discards mantissa bits in the subnormal intermediate. DUCC retains a double mantissa and does not lose these bits.
- m=5,l=720,r=5: replacing only cos² by DUCC's f64 cos² rounded to f32 changes output to 1.6063994169; relative error falls from 1.23586e-3 to 3.26359e-4. This is strong sensitivity evidence, not a whole decomposition. Seed, recurrence and FMA substitutions are non-additive.
- m=16,l=720,r=5: replacing only cos² reduces relative error from 1.01521e-3 to 2.38931e-4. The seed-correction-only control instead changes error slightly in the opposite direction, another reason not to sum one-variable deltas.
- m=0,l=720,r=0: replacing the seed with DUCC's rounded f32 seed worsens relative error from 8.79515e-3 to 4.09924e-2. A one-ULP seed difference is observable but is not a standalone correction for the conditioned recurrence.
- In the selected single-mode substitutions, changing only the accumulation to f64 leaves the f32 output unchanged after final f32 rounding. The selected errors therefore originate before or within recurrence/seed/geometry rather than from a long reduction of many coefficients. This does not measure dense-spectrum summation error.

Scalar fmaf replay matches all 14,581 traced native-f32 recurrence updates, including the leading contraction and state update. Rust emits AVX2 eight-lane f32 FMA operations; DUCC emits four-lane f64 operations. Disassembly confirms the two contractions are present in the Rust ordinary loop. No-FMA controls improve some selected cases and worsen others; they do not identify a universal contraction defect.

## Fourier stage, FFT, and public route controls

The mismatch is already present in the Legendre/Fourier stage. Across 48 checked native Rust stage-to-FFT cases, the largest absolute difference between the Rust map pixel and the analytic Fourier expansion of that exact Rust stage value is 4.76837e-7; the largest relative-to-pixel residual is 1.88956e-7. The FFT is not the source of the much larger pre-FFT discrepancies above.

A separate public-map matrix has 385 single-mode cells and 13,090 pixels. Relative to the independent reference, Rust's maximum absolute map error is 5.67913e-3 and maximum per-ring scaled error is 3.28830e-3 (m=8,l=650,imaginary,r=5). DUCC's corresponding values are 9.53674e-7 and 1.19057e-7. Rust's route varies by order: m=0 uses the compensated zonal leaf (maximum per-ring scaled error 1.09356e-7); m=1–2 use the compensated low-order leaf near the poles (2.59767e-7 maximum) but use the generic route at the equator (1.88180e-3 maximum); m>=3 use the generic f32 route (3.28830e-3 maximum). The direct generic m=0 trace is not the public m=0 route.

With phi0=0 there is no nontrivial phase rotation. DUCC's public f32 Fourier buffer is narrowed from double during extraction, then its ring helper converts spectrum values to double and runs pocketfft<double>, and the final double map value is narrowed to f32. Rust's public path retains f32 spectrum, inverse FFT, and map storage. The FFT precision difference is visible in the separate map control but is much smaller than the pre-FFT error in the failing cases.

Full and reduced random T720 end-to-end fixtures were not run. Per the steering, they were deferred because single-mode pre-FFT traces do not agree sufficiently. The public-map matrix above uses deterministic single-coefficient cases, not the full/reduced random benchmark fixtures.

## Answers to the exit questions

1. **First differing operation.** There is no single first operation for every failure. It is sectoral normalization/seed for m=0/ring 0; scaled seed normalization for m=8/ring 1; one-ULP sectoral seed followed by cos² at degree 7 for m=5/ring 5; cos² then recurrence for m=1/equator; and the different scaled-prefix path followed by f32 recurrence for m=16/ring 0.
2. **Cause class.** The recurrence formula and signs agree. Inputs and theta bits agree. sin/cos agree after f32 rounding; a/b agree after f32 rounding. Differences come from finite precision in prepared constants and f32 products, the scaled f32 seed path, cos² rounded from f32 operands, repeated f32 FMA state updates, and a precision-dependent scale/ordinary transition. There is no SIMD-versus-scalar replay mismatch.
3. **Growth.** Errors accumulate over hundreds of recurrence updates, usually with oscillatory high-degree state drift; the zonal pole reaches a cancellation condition number of 6.785e13 on its final tested update. Specific state errors and envelopes are tabulated above and retained per degree in recurrence-growth.csv.gz.
4. **Final map contribution.** Most of the large observed error is already present before the FFT. The independent Fourier-to-pixel check bounds the separate Rust FFT/repacking residual at 4.77e-7 absolute for the 48 checked cases. The public-map route matrix confirms that the generic f32 routes carry the pre-FFT difference into map pixels.
5. **Operations to improve for a tighter error bound.** Avoid the f32 seed path that normalizes through subnormal mantissas; prepare sectoral factors without avoidable one-ULP drift; form cos² with a precision-aware plan-time value; and reduce recurrence-state error under cancellation. The selected accumulation controls do not justify prioritizing f64 accumulation for these single-mode cases. An acceptance tolerance must be supplied before assigning pass/fail.
6. **Remedy and tradeoff.** A native-f32 remedy is plausible: the m=8 seed control recovers 41.5% of that case's output error while leaving the ordinary recurrence unchanged. It does not solve the remaining recurrence error, and correcting cos² similarly leaves residual error. A compensated state update or a narrower scale exponent chunk is a candidate for a later, separately measured experiment, not a proven fix. Higher accuracy may cost instructions/registers; no whole-transform measurement was made here. The evidence does not establish that native f32 cannot meet a future target, nor that it does.

## Recommended NEXT

Set an independent numerical error budget for the intended scientific workload. Then, in an isolated diagnostic branch, change only the f32 seed normalization so no nonzero intermediate crosses into a subnormal mantissa; replay the existing 357-case pre-FFT set and the single-mode public-map matrix against binary128. Keep recurrence, accumulation, and FFT unchanged for that test. If the seed-only change improves the aggregate without harming polar/subnormal cases, benchmark the whole T720 transform before considering it for production. Separately evaluate plan-time cos² preparation and recurrence compensation; do not combine those changes until the one-variable traces identify their effects.

No production source, API, workspace, or benchmark binary was changed by this diagnosis. Instrumentation and candidate changes are confined to the archived patches and scratch checkouts.
