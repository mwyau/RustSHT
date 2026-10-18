# RustSHT performance status

Snapshot: `opt/sht-performance-parity` based on `c58d94c2` (2026-10-18). DUCC remains faster in the matched atmospheric workloads. The integrated tile-128 scaled-prefix route remains the multiworker implementation and robust fallback/reference. After the existing compact scalar Legendre table check, a separate order-major DUCC-parity candidate is selected for eligible f64 scalar GL synthesis when workspace capacity is one; it falls back on unsafe coefficients or recurrence states. The initial T255/T300 screen improved on the retained single-thread ring-group route, but full-call medians remained 8.3–32.3% above installed DUCC 0.41.0. These ungated measurements do not establish parity.

## DUCC scalar candidate

- **Current implementation:** The existing tile-128 route remains in the multiworker path and is retained as the single-thread correctness reference and safety fallback. Existing CC/F1 and spin paths are unchanged.
- **Candidate:** `src/legendre/scalar_ducc.rs` implements order-major active-ring packing in 256-lane batches, reflected north/south reconstruction, DUCC-style scaled initialization and insignificant-prefix advancement, transformed coefficients reused across batches, and fused SIMD recurrence/accumulation. `ShtPlan::synthesis_into` checks the existing compact scalar Legendre table first, then dispatches to the candidate for eligible f64 scalar GL inputs when workspace capacity is one; unsafe inputs clear staged spectra and fall back.
- **Numerical status:** T255 mixed and sectoral cases match pinned DUCC references. A release check at capacity one passed GL sectoral synthesis at T511, T1023, and T2047 plus analytic map checks; CC/F1 cases in that test remained on their existing route. q8, extreme normal-range mixtures, and full moderate/high-degree mixed coefficient fields remain unvalidated for this candidate.
- **Measured performance:** At T255 and T300 q1, candidate Legendre-only medians were 56–63% lower than the existing ring-group route; full medians were 44–57% lower. Candidate full medians were 32.3% above DUCC at T255 and 8.3% above it at T300. The Rust plan/workspace are reused; DUCC's public `synthesis_2d` reconstructs geometry per call, so this is directional. See [the screen record](reports/scalar-ducc-kernel-screen-2026-10-18.md).
- **Next engineering task:** Profile per-order coefficient preparation, active-ring packing, and the SIMD recurrence at T255/T300 and T1023. Use the measured costs and generated-code inspection to close the remaining single-thread gap before adding dynamic order scheduling.

## Production architecture

Scalar synthesis uses ring-group workers and an order loop. The f64 GL recurrence route uses the DUCC-derived paired-degree recurrence, a Pulp SIMD ordinary path, and scaled per-lane fallback. At `lmax >= 255`, safely prepared orders use the integrated tile-128 route, which accumulates a scaled prefix, fuses lane normalization into accumulation, vectorizes ordinary prefix factors and sums, transitions to a SIMD suffix when lane ranges permit, and scatters only after the tile succeeds. Coefficient-safety checks are prepared once for each order with ordinary tiles and reused across those tiles. Per-lane exponent tracking remains scalar. For scalar GL at or above `SCALAR_TRANSITION_MIN_LMAX`, the transformed paired coefficients share the existing secondary Fourier buffer when ring groups cover every execution worker and the cache fits; this supports reduced `mmax` and oversampled theta grids without another workspace allocation. Mixed latitude blocks can be compacted into worker-owned scratch. The compact scalar Legendre table is separately dispatched for measured small-band GL cases. Spin synthesis uses the general scaled recurrence, with measured scalar-conversion and hemisphere-pair routes for selected spin-1 cases. Analysis uses paired scalar and spin recurrences, mirrored-ring handling, and reusable worker scratch. The core contains GL, CC, F1, f64/f32, and spin 0–4 paths; test coverage by combination is documented separately.

The production synthesis loop is ring-group-owned: each worker traverses orders for its ring group. When eligible, scalar GL prepares transformed coefficient rows once and shares them across those groups. The broader T1279 and T2047 q8 screens showed overlapping before/after ranges, so the measured T1023 result remains the only demonstrated warm gain. DUCC's synthesis path prepares an order and processes its active rings in batches; the cache reduces coefficient preparation without changing ring ownership.

## Performance and numerical evidence

- The 29-grid atmospheric study covers T42–T2047 and reports 232 native f64 spin-0/spin-1 synthesis and analysis rows at q1/q8. It found DUCC faster in every matched comparison. Its binary came from an earlier revision; use it for workload trends, not as the current branch baseline.
- In the current campaign's matched T2047 baseline (DUCC 0.42.0, source `80388904`), q1 RustSHT/DUCC medians were 3355/370 ms for scalar synthesis, 1093/243 ms for scalar analysis, 4124/776 ms for spin-1 synthesis, and 1768/497 ms for spin-1 analysis. At q8, the corresponding ratios were 13.97, 5.42, 8.12, and 4.40. RustSHT relative-L2 errors against DUCC were between `1.7e-13` and `3.7e-13` for the q1 transforms.
- The first local integration screen compared the prior production revision `8d001252` with the tile route at `5fd41078`. Each value is a five-sample median from one release process per revision and workload; the commands were `examples/m1b_bench 0 L Q synthesis GL STANDARD` with `Q=1` or `Q=8`. The host was an AMD Ryzen 9 5950X (x86_64, 16 cores/32 threads, AVX2 and FMA), with Rust 1.99.0, Pulp 0.22.3, Rayon 1.12.0, and RustFFT 6.4.1. All cases used f64 scalar synthesis, full `mmax`, and GL. No host-pressure protocol ran, so the gate was not evaluated and interference is unknown. This is not a DUCC comparison.

  | Workload | Prior samples (ms) | Integrated samples (ms) | Medians, prior/integrated (ms) | Peak RSS, prior/integrated (KiB) |
  |---|---|---|---:|---:|
  | T255 GL 256x513 q1 | 5.952, 5.946, 5.988, 5.984, 5.972 | 5.778, 5.765, 5.780, 5.787, 5.797 | 5.972 / 5.780 | 44104 / 44288 |
  | T255 GL 256x513 q8 | 2.139, 2.049, 2.041, 2.045, 2.199 | 2.125, 2.034, 2.030, 2.177, 2.629 | 2.049 / 2.125 | 46764 / 47700 |
  | T511 GL 512x1025 q1 | 42.372, 42.322, 42.218, 43.927, 42.071 | 30.803, 30.553, 30.580, 30.716, 30.797 | 42.322 / 30.716 | 31668 / 31804 |
  | T511 GL 512x1025 q8 | 12.210, 12.324, 12.108, 12.368, 14.893 | 9.326, 9.617, 9.457, 9.358, 9.093 | 12.324 / 9.358 | 34720 / 34756 |

  Integrated medians were 3.2% lower at T255 q1 and 3.7% higher at T255 q8; they were 27.4% lower at T511 q1 and 24.1% lower at T511 q8. These single-process screens do not establish gains at other truncations, thread counts, or grids. The T255 q8 sample range was wide; without paired repetitions, its cause is unknown.
- A separate local screen compared the tile route at `5fd41078` with prefix normalization/accumulation fusion at `9bb3c00a`. Each value is one process per source and workload with five timed samples; the harness was `examples/m1b_bench 0 L Q synthesis GL STANDARD`, for q1/q8, f64 scalar GL synthesis, full `mmax`. The host and software versions were the same as above. The screen did not use a host-pressure protocol, so its gate was not evaluated and interference is unknown. It is an implementation screen, not DUCC or parity evidence.

  | Workload | Tile route samples (ms) | Fused samples (ms) | Medians, tile/fused (ms) | Peak RSS, tile/fused (KiB) |
  |---|---|---|---:|---:|
  | T255 GL 256x513 q1 | 5.778, 5.765, 5.780, 5.787, 5.797 | 5.683, 5.721, 5.837, 5.714, 5.768 | 5.780 / 5.721 | 44288 / 44048 |
  | T255 GL 256x513 q8 | 2.125, 2.034, 2.030, 2.177, 2.629 | 2.131, 2.059, 2.012, 3.058, 2.138 | 2.125 / 2.131 | 47700 / 47008 |
  | T511 GL 512x1025 q1 | 31.653, 31.703, 31.538, 31.655, 31.677 | 29.954, 29.630, 29.252, 29.284, 29.291 | 31.655 / 29.291 | 32168 / 32048 |
  | T511 GL 512x1025 q8 | 9.280, 9.068, 9.116, 9.177, 9.365 | 10.252, 8.903, 8.724, 8.960, 9.299 | 9.177 / 8.960 | 35400 / 34704 |
  | T2047 GL 2048x4097 q1 | 1508.121, 1507.191, 1508.531, 1520.716, 1513.706 | 1366.113, 1366.880, 1368.573, 1378.923, 1372.245 | 1508.531 / 1368.573 | 464268 / 464484 |
  | T2047 GL 2048x4097 q8 | 403.748, 389.850, 405.089, 393.116, 392.914 | 358.449, 355.558, 364.361, 356.718, 368.478 | 393.116 / 358.449 | 472704 / 473376 |

  Fused medians were 1.0% lower at T255 q1 and 0.3% higher at T255 q8, 7.5% and 2.4% lower at T511 q1/q8, and 9.3% and 8.8% lower at T2047 q1/q8. The T255 q8 and T511 q8 samples include wide ranges. This one-process screen supports retaining the fusion in the integrated route for further validation; it does not establish a broad speedup or performance acceptance. The earlier T1023 fusion matrix stopped at an unstable control and was not restarted or pooled with these results.
- A representative-resolution screen compared the production source at `d67d408d` with the SIMD-prefix source at `9559f4a8`. It ran `examples/m1b_bench` for f64 scalar GL synthesis at full `mmax`, with one warm-up and five timed samples in one process per source/workload. The host was an AMD Ryzen 9 5950X (x86_64, 16 cores/32 threads, AVX2 and FMA); software was Rust 1.99.0, Pulp 0.22.3, Rayon 1.12.0, and RustFFT 6.4.1. No host-pressure protocol ran, so there is no gate or interference assessment. This is not a DUCC comparison.

  | Workload | Baseline median (ms) | SIMD-prefix median (ms) | Change | Peak RSS, baseline/candidate (KiB) |
  |---|---:|---:|---:|---:|
  | T255 GL 256x513 q1 | 5.954 | 5.861 | −1.6% | 40524 / 41096 |
  | T255 GL 256x513 q8 | 2.271 | 2.162 | −4.8% | 44996 / 45420 |
  | T720 GL 721x1443 q1 | 67.267 | 64.610 | −4.0% | 43804 / 44276 |
  | T720 GL 721x1443 q8 | 30.143 | 30.020 | −0.4% | 65744 / 65960 |
  | T1279 GL 1280x2561 q1 | 334.541 | 314.136 | −6.1% | 131904 / 131648 |
  | T1279 GL 1280x2561 q8 | 153.435 | 143.062 | −6.8% | 190196 / 190044 |

  | Workload | Baseline samples (ms) | SIMD-prefix samples (ms) |
  |---|---|---|
  | T255 q1 | 5.929, 5.937, 5.954, 5.969, 5.980 | 6.150, 5.769, 5.976, 5.732, 5.861 |
  | T255 q8 | 2.631, 2.222, 2.250, 2.271, 2.278 | 2.157, 2.667, 2.451, 2.162, 2.110 |
  | T720 q1 | 67.857, 67.227, 67.448, 67.267, 67.031 | 64.744, 64.848, 64.610, 64.471, 64.502 |
  | T720 q8 | 30.397, 30.244, 30.139, 30.067, 30.143 | 30.306, 30.043, 30.020, 29.987, 30.012 |
  | T1279 q1 | 337.724, 334.211, 333.584, 334.541, 335.038 | 314.246, 313.288, 315.073, 314.136, 313.828 |
  | T1279 q8 | 153.299, 153.435, 154.887, 157.522, 152.252 | 143.062, 143.041, 142.736, 145.598, 145.120 |

  The T1279 samples were separated from baseline; T255 samples varied more, and the T720 q8 change was within the sample range. Peak RSS differed by at most 1.5%. Treat these medians as a directional screen, not broad acceptance or parity evidence.
- Scalar synthesis caches transformed coefficients in the existing `spin_spectra` buffer on the paired GL transition path (`lmax >= 1023`) when ring groups cover every execution worker and the cache fits. The five-pair T1023 q8 release repeat compared baseline `cda5de38` with the candidate using `examples/m1b_bench`, f64, GL `1024x2049`, `phi0=0.37`, one warm-up, and five timed samples per process. Four pairs favored the candidate by 7.1–9.7%; one pair was 3.2% slower with overlapping ranges. Median process times fell from `71.059 ms` to `65.010 ms` (8.5%). A paired peak-RSS sample changed from `121804 KiB` to `120920 KiB`; workspace allocation was unchanged. The T511 q8 and T720 q8 screens did not show repeatable gains. A one-process selector screen at T1279/T2047 q8 had overlapping before/after ranges and does not establish an additional gain. No host-pressure protocol or DUCC timing ran. Samples are in [the milestone record](plans/05-validation-and-milestones.md#scalar-synthesis-coefficient-reuse-m4-2026-10-18).
- The scaled-prefix route now checks transformed coefficient safety and the conservative sum bound once per order with ordinary active tiles, then reuses that preflight for each 128-ring tile. A one-process T2047 q1 release screen compared `c60b5950` with `857e842a`: median synthesis time fell from `1277.407 ms` to `1244.829 ms` (2.6%), and the five-sample ranges did not overlap. At T1279 q8 the medians changed from `146.521 ms` to `148.454 ms` (+1.3%) with overlapping ranges. No host-pressure protocol or DUCC timing ran; treat these as directional. The [milestone record](plans/05-validation-and-milestones.md#reuse-scalar-prefix-safety-preflight-across-active-tiles-m4-2026-10-18) contains samples and validation.
- At T2047 q8, the atmospheric phase profiles attribute 84–97% of synthesis time and 68–90% of analysis time to Legendre recurrence and coefficient accumulation. Inverse FFT is at most 7.4% for scalar and 14.5% for spin 1 in the measured synthesis cases. RustFFT replacement is not the current bottleneck.
- The current f32 scalar recurrence uses normalization-skip dispatch only at the paired-screened T1023 and T1279 truncations. Those outputs were bitwise equal to the fallback; measured scalar synthesis and analysis improved at both sizes. Broader f32 performance limits remain unestablished. The reusable f64 bridge reduced error but remained as much as 13x slower than DUCC f32 at T1023 scalar q8 synthesis.
- Numerical reports identify DUCC 0.41.0 or 0.42.0 and provide analytic, grid, adjoint, high-bandlimit, and full-array comparisons for their stated cases. Those results do not establish untested configurations or universal limits.

## Technique findings

| Technique | Status | Result | Reason | Reference | Next action |
|---|---|---|---|---|---|
| Ishioka paired-degree scalar recurrence | Production | DUCC-derived recurrence supports fused SIMD and scaled fallback. | It is the current scalar basis kernel; recurrence correctness is not the bottleneck hypothesis. | [`scalar_paired.rs`](src/legendre/scalar_paired.rs), [`source comparison`](reports/data/m6-legendre-redesign/source-comparison.md) | Keep it as the reference while changing accumulation and work organization. |
| SIMD scalar and spin recurrences | Production | Pulp dispatches ordinary lanes; scaled lanes use the robust recurrence. | Mixed and extreme-range lanes need the scaled path. | [`scalar_paired.rs`](src/legendre/scalar_paired.rs), [`spin_fast.rs`](src/legendre/spin_fast.rs) | Improve production hot paths without removing scalar and scaled fallbacks. |
| SIMD transition range reductions | Rejected | Replacing the two scalar lane scans with Pulp reductions measured +5.2% at T2047 q1 and +5.1% at T1279 q8. | Both single-process candidate ranges were above baseline; no host-pressure protocol ran. | [`screen report`](reports/simd-transition-range-check-screen-2026-10-18.md), [`candidate source`](reports/data/simd-transition-range-check-screen-2026-10-18/candidate.patch) | Keep scalar transition checks unless a different whole-call profile supports another design. |
| Scalar north/south hemisphere pairing | Rejected | T511 was 10.4% slower at q1 and 6.2% slower at q8. | Safe fully active blocks covered only part of the workload; coefficient setup and scatter exceeded saved recurrence work. | [`hemisphere experiment`](reports/hemisphere-paired-synthesis-2026-10-14.md) | Do not repeat the coarse block design. |
| Spin-1 hemisphere pairing | Production, restricted | Current dispatch covers GL, full `mmax`, T319–T511, and 4–8 workers. | The measured route is useful only in a bounded geometry and worker range; checked conversion falls back to the general recurrence. | [`spin-1 route`](reports/spin1-hemisphere-pair-synthesis-2026-10-14.md), [`dispatch`](src/spin_transform.rs) | Preserve its numerical checks; broaden only with matched evidence. |
| Order-owned versus ring-owned synthesis | Ring ownership in production; order-owned prototype rejected | The prototype regressed, especially at T511 q8. The transformed-coefficient cache keeps ring ownership and uses work/path eligibility rather than one exact truncation. | One-process T1279/T2047 q8 ranges overlap; T1023 remains the only demonstrated cache gain. | [`order-owned screen`](reports/order-owned-synthesis-2026-10-15.md), [`source comparison`](reports/data/m6-legendre-redesign/source-comparison.md), [`cache evidence`](plans/05-validation-and-milestones.md#scalar-synthesis-coefficient-reuse-m4-2026-10-18) | Retain the bounded shared-buffer path and measure it at matched representative sizes. |
| Active-ring packing and compaction | Block-local compaction in production | Mixed blocks compact active lanes; fully inactive blocks are skipped. No order-wide packing candidate is accepted. | Compaction and active-lane density have not been separated from recurrence time. | [`source comparison`](reports/data/m6-legendre-redesign/source-comparison.md), [`transform.rs`](src/transform.rs) | Measure active counts and compaction time before changing layout. |
| Scaled transition and robust fallback | Production | Coefficient safety and sum-bound preflight are reused across tiles for each order with ordinary lanes; dynamic lane checks still guard each transition. Unsafe and declined tiles use the scaled recurrence. | T2047 q1 had a 2.6% one-process reduction; T1279 q8 was inconclusive. The archived T2047 q1 profile assigned 51.91% of the transform to transition work and 39.41% to the SIMD suffix. | [`campaign profile`](reports/m6-cpu-performance-parity.md), [`synthesis_fast.rs`](src/legendre/synthesis_fast.rs), [`preflight evidence`](plans/05-validation-and-milestones.md#reuse-scalar-prefix-safety-preflight-across-active-tiles-m4-2026-10-18) | Keep dynamic safety checks and fallback; compare more resolutions at an integrated milestone. |
| Tile-128 scalar synthesis | Eligible for f64 GL `lmax >= 255`; screen only | The local screen measured T255 changes of −3.2% at q1 and +3.7% at q8, and T511 changes of −27.4% at q1 and −24.1% at q8. | Each workload has one process per revision and no host-pressure gate; the screen does not establish DUCC parity or broad acceptance. | [`synthesis_fast.rs`](src/legendre/synthesis_fast.rs), [`screen details`](#performance-and-numerical-evidence) | Run controlled q1/q8 and DUCC comparisons at more truncations before broadening dispatch. |
| Scaled-prefix accumulator | Production route; historical candidate evidence | Archived six-pair q1 candidate/baseline ratios were 0.568982 at T1023 and 0.407892 at T2047; the T2047 candidate latency was 1371 ms and relative-L2 error was `3.015e-13` versus DUCC. | Those results came from an isolated source build. The integrated route still needs its own full operation and host-gated comparisons. | [`T1023`](reports/data/m6-legendre-redesign/route-timing/scaled-prefix-accumulator-t1023-q1.md), [`T2047`](reports/data/m6-legendre-redesign/route-timing/scaled-prefix-accumulator-t2047-q1.md) | Complete integrated q1/q8, reduced-order, odd/even longitude, and multiworker comparisons. |
| Exponent-epoch accumulator | Experimental; no clear improvement | Six-pair q1 ratios were 0.594264 at T1023 and 0.414702 at T2047, weaker than the preceding accumulator. | It rebases within exponent epochs but did not improve the accepted high-resolution comparisons; T511 was unresolved. | [`epoch q1 results`](reports/data/m6-legendre-redesign/route-timing/exponent-epoch-q1-screen-v1-results.md) | Prefer the preceding accumulator unless a new profile supports epoch rebasing. |
| Prefix normalization/accumulation fusion | Integrated in tile route; screen only | The one-process comparison against the pre-fusion tile route measured 7.5% and 2.4% lower medians at T511 q1/q8 and 9.3% and 8.8% lower medians at T2047 q1/q8; T255 was effectively unchanged. | The screen had no host-pressure gate, and T255/T511 q8 sample ranges were wide. The earlier T1023 matrix stopped at an unstable control and was not reused. | [`synthesis_fast.rs`](src/legendre/synthesis_fast.rs), [`fusion integration record`](plans/05-validation-and-milestones.md#prefix-normalization-and-accumulation-fusion-m4-2026-10-18), [`historical screen`](reports/data/m6-legendre-redesign/route-timing/prefix-lane-pass-fusion-screen-t2047-q1.md) | Obtain controlled process-pair and DUCC comparisons, including T1023, before treating the timing change as accepted. |
| SIMD prefix factor formation and accumulation | Integrated; directional screen | One-process medians were 0.4–6.8% lower at T255/T720/T1279 q1/q8 versus `d67d408d`; T1279 showed the clearest reductions. | No host-pressure gate or DUCC mate; T255 samples were variable and T720 q8 was effectively unchanged. | [`synthesis_fast.rs`](src/legendre/synthesis_fast.rs), [`screen details`](#performance-and-numerical-evidence), [`historical candidate`](reports/data/m6-legendre-redesign/route-timing/simd-prefix-screen-t2047-q1.md) | Validate multiworker numerical output and compare representative q1/q8 workloads with DUCC before extending the route. |
| SIMD range prechecks | Inconclusive | A diagnostic reduced the prefix estimate 6.41% and whole-call estimate 2.62% in one process. | No paired acceptance matrix; corrected lane replay found little work safe across every degree. | [`prefix and precheck findings`](reports/m6-cpu-performance-parity.md) | Keep the existing checked transition unless controlled whole-call comparisons justify a precheck. |
| Native f32 recurrence | Production; limited performance selector | Exact T1023/T1279 scalar skips were bitwise equal to fallback and improved measured S/A at q1/q8. T720 screens rejected a general selector. | Benefits depend on the recurrence range; a broad scan regressed T720. | [`f32 study`](reports/f32-scalar-normalization-skip-2026-10-16.md), [`T1279 study`](reports/f32-scalar-normalization-skip-T1279-2026-10-16.md) | Preserve the tested selector; improve f32 kernels after f64 synthesis integration. |
| Reusable f64 bridge for f32 | Rejected as a general route | It was 3.49–31.74x faster than native f32 in an unaccepted screen but up to 13x slower than DUCC f32; T1023 spin-1 needed a 48 MiB temporary payload. | Conversion cost and memory do not close the DUCC gap; the screen lacked paired repetitions and odd-longitude controls. | [`bridge screen`](reports/m6-cpu-performance-parity.md) | Keep native f32 recurrence as the implementation target. |
| Scalar analysis | Production; slower than DUCC | Current T2047 q1/q8 ratios were 4.49x/5.42x. Recurrence and accumulation dominate high-resolution q8 profiles. | Synthesis-only work does not automatically improve the separate analysis operator. | [`analysis baseline`](reports/m6-cpu-performance-parity.md), [`analysis profile`](reports/current-analysis-phase-profile-2026-10-15.md) | Continue range-check and accumulation work after scalar synthesis. |
| Spin-1 analysis | Production; slower than DUCC | Current T2047 q1/q8 ratios were 3.56x/4.40x. | The path has channel preparation and spin-specific recurrence costs; synthesis improvements do not transfer automatically. | [`spin-1 baseline`](reports/m6-cpu-performance-parity.md), [`spin-1 analysis report`](reports/spin1-analysis-ducc-refresh-2026-10-15.md) | Reuse proven scalar kernel changes where the spin basis permits. |
| FFT and multicore scheduling | RustFFT production; Legendre work dominates | Row FFT parallelism reduced q6 T1023 analysis by 36.1% for scalar and 43.9% for spin 1. Prior leaf overpartition regressed T511 by 3.19% and T720 by 11.55%. | High-resolution FFT shares are smaller than recurrence/accumulation; balanced estimated leaf work still had 1.6–2.5x timing spread. | [`FFT scheduling`](reports/row-fft-parallelism-2026-10-15.md), [`atmospheric study`](reports/atmospheric-resolution-performance-ectrans-2026-10-16.md) | Keep RustFFT; improve recurrence scheduling only after the single-thread kernel. |
| Atmospheric-resolution comparisons | Historical matrix; current baseline separately measured | The 29-grid T42–T2047 matrix found DUCC faster in every matched case. Current T2047 q8 gaps are 4.40–13.97x by operation. | The 29-grid binary predates this branch snapshot; do not merge its timings with current-baseline measurements. | [`29-grid study`](reports/atmospheric-resolution-performance-ectrans-2026-10-16.md), [`current campaign`](reports/m6-cpu-performance-parity.md) | Repeat a representative resolution panel after major kernel integration. |

## Operation status

- **Scalar synthesis, f64:** After the existing compact scalar Legendre table check, eligible single-thread f64 scalar GL synthesis dispatches to the order-major DUCC-parity candidate; unsafe inputs and unsupported shapes use the existing implementation. Candidate checks include T255 mixed and sectoral coefficients and T511/T1023/T2047 GL sectoral fields, with a DUCC reference and analytic field. The existing tile-128 scaled-prefix path remains the multiworker route and fallback/reference. Initial T255/T300 q1 screens favor the candidate over the retained single-thread route but show a remaining full-call gap against DUCC; q8 and broad mixed-field high-degree validation remain outstanding. CC/F1 and other spins are unchanged.
- **Scalar analysis, f64:** GL, CC, and F1 analysis and the separate adjoint are implemented. The current high-resolution profiles place most q8 time in Legendre recurrence and coefficient accumulation. Scalar synthesis changes require separate analysis tests.
- **Spin-1 synthesis, f64:** The general recurrence, scalar conversion, paired Q/U route, and restricted hemisphere route are present. E/B conversion and polar fallbacks are checked for extreme finite coefficients.
- **Spin-1 analysis, f64:** Paired scalar and fused channel paths are present. DUCC comparisons and thread screens cover selected truncations; synthesis timing gains do not establish analysis gains.
- **f32:** Native scalar and spin recurrences are present. Scalar normalization-skip dispatch is restricted to T1023 and T1279; the reusable f64 bridge remains experimental and is not the general f32 solution.
- **Spin 2–4:** Existing transform operations, supported adjoints, and scaled fallbacks remain. Recent C03 checks cover spin-2 coefficient-conversion overflow cases; performance parity has not been established.
- **Grid and operator coverage:** The project supports GL, CC, and F1, with distinct S, A, S*, and A* contracts. See the numerical specification and DUCC crosswalk for exact sampling, parity, normalization, and adjoint rules.
- **Test limits:** Report the tested truncation, grid, precision, spin, operation, longitude count, and worker count. The campaign results do not establish a universal maximum or a general f32 accuracy threshold.

## Source-level opportunities

The pinned DUCC comparison covers `sht_inner_loop.h` and `sht.cc` at revision
`80388904b8cfc32400ba84af8a6a45aabfe60f86`. It identifies concrete differences
between the two synthesis paths:

- The candidate gathers active rings into order-specific batches with a 256-ring
  limit. The retained multiworker ring-group path still skips inactive blocks
  and compacts mixed lanes in ring-group blocks.
- The candidate transforms scalar coefficients once per order. The retained
  ring-group path shares paired coefficients across workers only when its path,
  group-count and existing-buffer checks pass; recurrence factors remain
  worker-local.
- The candidate currently runs one order at a time on a single thread. The
  multiworker route still assigns ring groups, so DUCC-style dynamic order
  scheduling remains to be implemented.
- Both implementations have scaled recurrence setup, an ordinary SIMD kernel,
  and scaled fallbacks. The likely gains are in repeated coefficient work,
  active-lane handling, and accumulation cost, not in replacing the recurrence
  with a generic dense transform.

The candidate now addresses the principal execution differences: it packs
active rings per order, prepares transformed coefficients once per order,
reconstructs reflected outputs from shared recurrence state, and executes
order-major SIMD recurrence and accumulation. The remaining work is to measure
preparation and packing costs separately, then refine the loop or add order
scheduling based on those measurements. Existing route profiles show that
recurrence plus accumulation dominates high-resolution workloads, but they do
not isolate the candidate's source-level costs. The rejected order-owned and
hemisphere prototypes also show that reducing recurrence counts alone does not
guarantee lower whole-transform time.

## How to interpret the measurements

- The current M6 baseline and isolated candidate results use different source
  builds. Candidate/baseline ratios apply only to the matched source pair and
  workload named in each report.
- q1 and q8 are separate execution regimes. A q1 improvement does not establish
  a q8 improvement, and per-worker CPU timer sums are not wall-time shares.
- The scaled-prefix accumulator has valid six-pair q1 results at T1023 and
  T2047, but its T511 baseline control failed the fixed stability gate. The
  valid high-resolution results justify integration work, not a full-operation
  acceptance claim.
- The exponent-epoch and pre-integration prefix-fusion measurements compare
  experimental sources. The new fusion screen compares two tile-route builds
  but has no pressure gate and does not replace controlled DUCC comparisons.
- The SIMD-prefix screen compares `d67d408d` with `9559f4a8` at T255/T720/T1279.
  It shows useful directional reductions, especially at T1279, but has one
  process per source/workload and no pressure gate or DUCC comparison.
- Retain every valid timing and failed gate in its report. Do not use diagnostic
  profiles, one-pair screens, or an invalid process as performance acceptance.

## Implementation order

1. Profile the single-thread scalar DUCC candidate's order preparation, active-ring packing, and SIMD recurrence at the recorded T255/T300 shapes and T1023. Add mixed high-degree, reduced-order, tail, and numerical-safety checks where the profile does not require a separate operator change.
2. Refine the SIMD loop and coefficient preparation based on those measurements, then compare q1 candidate kernels with pinned DUCC at representative irregular and atmospheric dimensions. Keep the old kernel as fallback/reference.
3. Implement dynamic harmonic-order scheduling after the single-thread candidate is stable; measure q8 and workspace costs against the ring-group baseline.
4. Reuse validated scalar recurrence and scheduling components for spin-1 synthesis, then address scalar/spin-1 analysis and native f32 with operation-specific adjoint and accuracy checks.
5. Report gaps and regressions by operation; do not infer parity from one truncation or from the Legendre-only comparison alone.

Historical reports and artifacts under `reports/` provide full methods, raw scientific samples, and source patches. Read them through the references above when a specific implementation decision needs more evidence.
