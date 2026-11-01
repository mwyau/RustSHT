# B1 scalar-analysis profile at `410c9ec`

## Result

The avoidable work is in the Legendre recurrence and accumulation stage, not the longitude FFT. In the warm T720 phase probe, recurrence plus accumulation took 94–97% of Rust f32 B1 time across full/reduced q1/q8; FFT took 3–6%. Rust f64 also spent 82–91% in that stage. These phase timings are test-instrumented stage measurements, not production cycle attribution.

The strongest current f32-specific lead is the scale-state path: at full T720 q1, 125,629 of 237,538 recorded coefficient-pair accumulations were scaled (52.9%); reduced q1 had 52,459 of 114,302 (45.9%). Callgrind source lines attribute 5.79% of total f32 instructions to scanning the 64-ring tile for zero scale factors and 2.25% to building factors. Those costs matter, but explain only part of the roughly 3.9x f32/f64 dynamic-instruction ratio. Hardware cycle counters were unavailable, so the remaining root cause is not yet proven.

Two code-size hypotheses were screened and rejected. Replacing partial SIMD stores with direct lane views made whole calls slower or neutral in all four cells. Outlining the exceptional scalar normalizer reduced the f32 typed V3 kernel body from 22,751 to 18,928 bytes, with a 2,467-byte cold helper, but still slowed all four three-pair medians by 4.5–5.3%. Both candidates produced bitwise-identical output to the exact parent and passed reset checks. Neither change is retained in production.

## Profile and execution paths

The source and binaries are from parent commit `410c9ec2594007a1e081c949767fc5f7c28d2654`; DUCC is pinned to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The host was an AMD Ryzen 9 5950X. Rust release builds used `RUSTFLAGS="-C target-cpu=x86-64-v3 -Aunexpected_cfgs"`; runtime dispatch was Pulp V3 AVX2/FMA, eight f32 lanes. q1 was pinned to CPU 0 and q8 to CPUs 0–7. DUCC used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`.

The Rust production route is `ShtPlan<T>::analysis_into` in `src/transform/scalar.rs:1103`, through `analyze_map_to_coefficients`, the truncated forward longitude FFT, and `analyze_order_group`/`analyze_order_jobs_dynamic`, into `fused_scalar_analysis_pair_block_preweighted_dispatched` and `FusedScalarAnalysisPairTyped<T>::with_simd` in `src/legendre/scalar_paired/analysis.rs`. The pinned DUCC path is `alm2leg` in `src/ducc0/sht/sht.cc:576`, then `calc_map2alm` and `map2alm_kernel` in `src/ducc0/sht/sht_inner_loop.h:636,592`; `iter_to_ieee` and `rescale` cover the exceptional prefix before the ordinary kernel. The source path is identified by the pinned revision because the DUCC checkout is external to this repository.

`perf stat` could not access hardware events: `/proc/sys/kernel/perf_event_paranoid` was 4 and the process lacked perf capabilities. Callgrind recorded instruction counts only. Its full q1 run counted 1,239,125,687 Ir for Rust f32 and 318,157,830 Ir for Rust f64 (3.89x; Ir is not cycles). In the f32 source annotation, factor construction accounted for 27,825,495 Ir (2.25%), the tile zero-factor scan 71,802,176 Ir (5.79%), and the two scaled complex multiply calls together 18,007,584 Ir (1.46%). Only 14.42% of the total was line-annotated in the available source report, so this is a partial hotspot distribution.

The instrumented phase measurements (medians of warm iterations) were:

| Precision | mmax | q | Total ms | FFT ms | Recurrence + accumulation ms | Route counts |
|---|---:|---:|---:|---:|---:|---|
| f32 | 720 | 1 | 72.790 | 2.646 | 70.123 | 111,909 normal / 125,629 scaled |
| f32 | 720 | 8 | 11.827 | 0.680 | 11.128 | 111,909 normal / 125,629 scaled |
| f32 | 180 | 1 | 24.223 | 0.640 | 23.570 | 61,843 normal / 52,459 scaled |
| f32 | 180 | 8 | 3.406 | 0.121 | 3.274 | 61,843 normal / 52,459 scaled |
| f64 | 720 | 1 | 20.739 | 3.404 | 17.316 | 665 typed successes / 474 seed declines |
| f64 | 720 | 8 | 3.128 | 0.562 | 2.558 | 665 typed successes / 474 seed declines |
| f64 | 180 | 1 | 8.688 | 0.741 | 7.927 | 230 typed successes / 132 seed declines |
| f64 | 180 | 8 | 1.282 | 0.147 | 1.125 | 230 typed successes / 132 seed declines |

The probe separates FFT from the combined Legendre recurrence/accumulation stage, but does not yet isolate recurrence advancement, normal accumulation, scaled accumulation, or normalization as independent timings. The committed [phase-summary.json](phase-summary.json) contains the machine-readable measurements and route counts. Verbose phase console captures remain local and untracked; their hashes are listed as `local_untracked_capture_sha256` in the manifest.

## Assembly comparison and hypothesis

Rust's ordinary path is `FusedScalarAnalysisPairTyped<T>::with_simd` in [`analysis.rs`](../../../../src/legendre/scalar_paired/analysis.rs#L1111). It vectorizes latitude lanes, accumulates same/opposite real and imaginary channels in four YMM accumulators, horizontally reduces them for each coefficient pair, and advances the recurrence with two FMAs. The f32 scaled path prepares per-lane exponent factors and separately tracks hard lanes in `F32ScaledAnalysisState` ([`analysis.rs`](../../../../src/legendre/scalar_paired/analysis.rs#L77)); native f32 state normalization is in [`scalar.rs`](../../../../src/legendre/scalar.rs#L1414).

The Rust V3 f32 disassembly has a 22,751-byte typed kernel body and shows four `vfmadd231ps` operations per eight-lane basis vector, followed by `vextractf128`/`vaddps` horizontal reductions for all four channels. The f64 typed body is 5,327 bytes. The same f32 routine includes ordinary, scaled and exceptional handling, so size alone does not show which blocks execute.

Pinned DUCC's `map2alm_kernel` is 722 bytes. Its source unrolls two same-parity recurrence updates (`l += 4`), accumulates both coefficient outputs across the latitude-vector scan, then reduces them at lines 596–616 of `src/ducc0/sht/sht_inner_loop.h` at the pinned DUCC revision. The AVX2 loop uses YMM double state and FMA. The Rust and DUCC loops have related mathematical work but different staging and precision. The small DUCC kernel and larger Rust f32 kernel support a code-layout/dataflow hypothesis; they do not prove instruction-cache pressure or identify the remaining cycle cost.

The selected instruction excerpts are in [hot-loops.asm.txt](hot-loops.asm.txt). The parent Rust and DUCC disassembly checksums, profile checksums, binary hashes, and fixture hashes are in [binary-and-raw-checksums.json](binary-and-raw-checksums.json). Full binaries and fixtures are not duplicated.

## Candidate screens

Three interleaved q1/q8 pairs screened all four f32 T720 cells. Parent, candidate and DUCC used matched deterministic maps; the candidate and parent used the same V3 ISA. The screen medians were:

| Cell | Parent ms | Candidate ms | DUCC f32 ms | Candidate / parent | Candidate / DUCC |
|---|---:|---:|---:|---:|---:|
| Full q1 | 75.045 | 78.636 | 15.240 | 1.047 | 5.195 |
| Full q8 | 10.762 | 11.526 | 2.460 | 1.053 | 4.686 |
| Reduced q1 | 23.311 | 24.071 | 6.022 | 1.045 | 4.045 |
| Reduced q8 | 3.529 | 3.691 | 1.041 | 1.046 | 3.544 |

These are screening medians, not promotion intervals. Candidate-versus-parent errors were zero, outputs were finite, and workspace resets were consistent. The outline candidate's patch is retained at `raw/rejected-cold-exception-normalizer-outline.patch` (SHA-256 `fd493382eb26579237564fdc6c9cd76bcf9a97e536f3c268f72051d015a9c22b`). The direct-view patch SHA-256 is `26a4e07d589b0a61dfd6b0c7dc00242724c8ad04a58552fb8526d064a1c4595f`; its per-cell results are summarized in [screen-summary.json](screen-summary.json).

## Next

`perf stat` and `perf record` are blocked on this host by `perf_event_paranoid=4` and missing perf capabilities, so this profile establishes stage time, source-attributed Ir and executed instructions, not cycles. The direct-view normalization and cold exceptional-normalizer outline candidates both preserved output but regressed every three-pair f32 cell (direct-view: 1.5–11.4%; outlining: 4.5–5.3%); neither is retained. **NEXT:** use test-only Callgrind region gates on the existing nonzero T720 fixture to separate ordinary recurrence, ordinary accumulation, scaled recurrence-plus-accumulation, scaled-factor preparation, and normalization. Keep Callgrind Ir explicitly separate from elapsed time. Use that attribution to select a different kernel change, then screen its relevant microbench and three interleaved whole-call pairs against this exact parent before retaining it.
