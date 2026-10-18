# Scalar DUCC kernel screen, 2026-10-18

This screen measures the first integrated native Rust port of DUCC's f64 scalar GL synthesis kernel. It is a small implementation screen, not a parity acceptance study.

## Implementations and workload

- The candidate is `src/legendre/scalar_ducc.rs`, based on DUCC `sht_inner_loop.h` and `sht.cc` at revision [`80388904b8cfc32400ba84af8a6a45aabfe60f86`](https://github.com/mreineck/ducc/tree/80388904b8cfc32400ba84af8a6a45aabfe60f86).
- The installed comparison package is `ducc0 0.41.0`, source revision [`c1ed01b8fda69d2a378190f0915f1a19762dddbf`](https://github.com/mreineck/ducc/tree/c1ed01b8fda69d2a378190f0915f1a19762dddbf). A targeted diff of the two revisions' `sht_inner_loop.h` and `sht.cc` found only namespace and namespace-qualification changes; the scalar routines in this comparison are unchanged.
- Workloads use f64, full `mmax`, `phi0=0.37`, and coefficients defined by `re=((7*l+11*m) mod 19)-9`, `im=0` for `m=0` or `((13*l+5*m) mod 23)-11`, with both components scaled by `1e-5`.
- T255 uses GL `256x513`; T300 uses GL `301x603`. Both longitude lengths are odd. T300 also has an odd reflected-ring count and SIMD tail lanes.
- Rust candidate and existing ring-group measurements use one Rayon worker, reusable plans and workspaces, one warm-up, and five timed calls. The old route is forced with workspace capacity 2; the candidate uses capacity 1. The Legendre-only calls both produce the pre-FFT ring spectra. Full calls include RustFFT synthesis.
- DUCC `alm2leg` is the direct Legendre-only reference and produces one accumulated value per `(theta,m)` before longitude phase handling. Rust additionally applies phase and writes the real-FFT spectrum layout, so the call costs include different final output handling. The DUCC full measurement uses `synthesis_2d` with a preallocated map. That public call reconstructs GL geometry and transform state per invocation, while Rust reuses its `ShtPlan`; full-call results are therefore directional rather than plan-lifecycle matched.
- No host-pressure protocol ran. Timing gates were not evaluated, so interference is unknown. Aggregate peak RSS was 44,812 KiB in a separate run of the Rust screen process.

The host was an AMD Ryzen 9 5950X (x86_64, 16 cores/32 threads, AVX2 and FMA, no AVX-512). Rust was `1.99.0 (b940084d7 2026-09-28)` with Pulp 0.22.3, Rayon 1.12.0, and RustFFT 6.4.1. Runtime dispatch selected the AVX2 path.

## Timings

All samples are elapsed milliseconds; medians use the five recorded calls.

| Workload | Rust route | Legendre-only samples | Median | Full synthesis samples | Median |
| --- | --- | --- | ---: | --- | ---: |
| T255 GL 256x513 | DUCC-parity candidate | 2.116268, 1.808587, 1.794740, 1.803237, 1.797265 | 1.803237 | 2.540529, 2.521183, 2.494032, 2.480385, 2.494131 | 2.494131 |
| T255 GL 256x513 | Existing ring-group route | 4.868146, 4.836296, 5.088132, 4.754372, 4.725567 | 4.836296 | 5.821337, 5.769338, 5.864318, 5.805537, 5.806820 | 5.806820 |
| T300 GL 301x603 | DUCC-parity candidate | 2.658311, 2.662830, 2.661167, 2.655867, 2.656589 | 2.658311 | 4.072594, 4.093192, 4.094164, 4.090467, 4.091269 | 4.091269 |
| T300 GL 301x603 | Existing ring-group route | 6.065418, 6.114861, 6.027115, 6.055660, 6.030011 | 6.055660 | 8.239405, 7.230579, 7.275945, 7.308948, 7.328664 | 7.308948 |

The candidate's Legendre medians were 62.7% lower at T255 and 56.1% lower at T300 than the existing ring-group route. Full synthesis medians were 57.0% and 44.0% lower, respectively. Candidate spectra and full maps matched the existing route within relative L2 error `3e-13` for both workloads.

| Workload | DUCC `alm2leg` samples | Median | DUCC `synthesis_2d` samples | Median |
| --- | --- | ---: | --- | ---: |
| T255 GL 256x513 | 1.481850, 1.478473, 1.458395, 1.466220, 1.454427 | 1.466220 | 2.324821, 1.952659, 1.882286, 1.877647, 1.884540 | 1.884540 |
| T300 GL 301x603 | 1.974329, 1.967006, 1.956365, 1.954372, 1.956546 | 1.956546 | 4.355769, 3.768829, 3.778558, 3.777065, 3.802974 | 3.778558 |

The Rust candidate full-call medians were 32.3% higher than DUCC at T255 and 8.3% higher at T300. The T300 DUCC `alm2leg` and `synthesis_2d` ranges were stable after warm-up; T255 full `synthesis_2d` had one high sample. These measurements do not establish parity or a general performance result.

## Numerical checks

- The T255 mixed-coefficient test compares the candidate against the existing route and pinned DUCC 0.41.0 sample values. The T255 sectoral test also passes pinned DUCC samples.
- A `T510`, `mmax=0`, `601x5` GL case packs 301 active northern rings into a 256-ring batch and a 45-ring tail batch. Its low-degree field matches closed-form harmonics; an added degree-510 mode matches the retained ring-group route.
- The release test `m3_scalar_t511_t1023_t2047_all_grids_matches_ducc_and_closed_form` now prepares workspace capacity 1. This exercises the candidate for GL at T511, T1023, and T2047; CC and F1 continue to use their existing route. The largest sampled DUCC middle-ring error was `2.092e-13` at T2047. The largest analytic-map error across the test's grid cases was `9.959e-12`; the GL candidate's T2047 maximum was `9.953e-12`.
- The full workspace suite passed with 207 unit tests, 15 ignored; 4 extreme-FFT integration tests; 11 f32 integration tests, 1 ignored; and 1 workspace-compatibility test. The ignored timing screen and the T511/T1023/T2047 release check also passed.

## Generated code and next work

Inspection of the optimized x86-v3 specialization shows 256-bit YMM operations and packed FMA instructions in the recurrence and accumulation body. The two-pair ordinary recurrence is inlined. The one-pair tail remains an out-of-line helper call. The candidate therefore uses AVX2/FMA through Pulp; this inspection gives no evidence of a scalar Rust code-generation path. It does not identify the entire remaining gap against DUCC.

The next task is to profile order preparation, active-ring packing, and recurrence/accumulation separately at these shapes and T1023, then use those results to refine the SIMD loop and coefficient preparation. Add dynamic harmonic-order scheduling only after preserving the validated single-thread route; q8 has not been measured for this candidate.
