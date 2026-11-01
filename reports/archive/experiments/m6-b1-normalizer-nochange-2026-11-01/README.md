# B1 f32 no-change normalizer bypass — isolated screen

## Scope and provenance

This is a B1-only candidate based on `d4cc76bfe22410ec410bfffac7f00d7b677fb928`; the current production B1 source last changed at `8560bcae34e0532a230b9a3e0d05cf10380cfccf`. DUCC is pinned to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The exact candidate diff is in [`candidate.patch.gz`](candidate.patch.gz), SHA-256 `de204bb35853e0e7fb86d5613524d4c4c6c6c720803f2215f71ac37a90042866`.

The host is AMD Ryzen 9 5950X. Rust used `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, Pulp V3 AVX2/FMA (eight f32 lanes; four f64 lanes), q1 pinned to CPU 2 and q8 to CPUs 0–7. The production candidate only changes f32 normalization. `perf stat`/`perf record` are blocked by `perf_event_paranoid=4`; cycle attribution is unavailable. Callgrind instruction counts and instrumented stage wall time are not cycles.

## Hotspot evidence

The current B1 route is `ShtPlan::analysis_into` → `analyze_map_to_coefficients` → `analyze_order_jobs_dynamic` → `analyze_order_group` / `analyze_symmetric_order_group` → `fused_scalar_analysis_pair_block_preweighted_typed` → `FusedScalarAnalysisPairTyped<T>::with_simd`. The pinned DUCC route is `leg2alm<T>` → `leg2alm_internal<T>` → `inner_loop_m2a<T>` → `calc_map2alm` / `map2alm_kernel`.

At production source `8560bcae`, test-only phase measurements put Legendre recurrence plus accumulation at 91–95% of f32 and 76–86% of f64 B1 wall time across full/reduced T720 q1/q8. FFT was 5–9% for f32 and 13–22% for f64. The 15-pair live matrix gives f32/DUCC f32 ratios of 2.10–2.72× and f64/DUCC f64 ratios of 1.03–1.13×. The f64 typed-seed decline probe found 474 full and 132 reduced blocks; actual GL declines occur at the first vector. A direct microbenchmark measured the early typed decline at about 16.6 ns against 2.46 µs fallback, so it is not a credible dominant target.

The experiment tested a separate hypothesis: avoid ordinary range normalization work and vector writeback when both recurrence vectors already lie in range. On a 512-block synthetic normalizer microbenchmark, candidate/parent medians were 0.752 for all-normal vectors, 0.884 for a 29%-changed mixture, but 1.067 when each vector required a small-lane scale update. See [`normalizer-microbench.json`](normalizer-microbench.json) for all 15 samples.

## Machine code

[`disassembly-excerpts.txt`](disassembly-excerpts.txt) contains generated V3 code from the exact parent and candidate binaries. The candidate adds a joint range test (`vcmpgt_oqps`, `vcmplt_oqps`, mask combination, `vptest`, branch) that skips update loops and writeback for an unchanged vector. Its f32 fused kernel grows from 22,307 to 23,043 bytes (+736, 3.3%), retains the same 6,120-byte frame, and has the same 204 `vfmadd` instructions. The f64 kernel is not changed: parent and candidate have identical normalized V3 instruction bodies, 1,388 instructions / 7,227 bytes each.

## Whole-call result

Fifteen interleaved parent/candidate/DUCC f32 pairs were run on deterministic GL T720 scalar-analysis fixtures, full M720/reduced M180, q1/q8. The full paired records, fixture checksums, affinity and binary hashes are in [`whole-transform-screen-15-pair.json`](whole-transform-screen-15-pair.json). Candidate and parent outputs were bitwise equal in all pairs; repeat/reset checks passed.

| Cell | Candidate / parent (95% interval) | Candidate / DUCC f32 (95% interval) | Decision |
|---|---:|---:|---|
| Full q1 | 0.907 [0.815, 1.010] | 2.706 [2.634, 2.778] | INCONCLUSIVE |
| Full q8 | 0.837 [0.609, 1.149] | 2.262 [1.687, 3.033] | INCONCLUSIVE; large host outliers |
| Reduced q1 | 0.988 [0.978, 0.997] | 2.197 [2.156, 2.238] | Small resolved improvement |
| Reduced q8 | 1.007 [0.980, 1.034] | 2.136 [2.070, 2.204] | INCONCLUSIVE |

Only reduced q1 resolves a gain (about 1.2%). The other three cells do not establish either a gain or a non-regression, and f32/DUCC remains far above parity. **Do not promote this candidate.** It targets a secondary normalizer cost and expands the f32 fused kernel; the isolated improvement does not survive robustly across whole calls. No production source changed.

## NEXT

On exact production source `8560bcae`, use test-only non-overlapping Callgrind gates and warm wall-phase probes to separate f32 scaled factor materialization, scaled accumulation, normalization and hard-lane recovery. Compare f64 full-q8 fused-kernel and FFT time against the matching pinned DUCC path. PMU cycles remain inaccessible, so keep instruction attribution and elapsed-time evidence distinct. Select one new candidate only after the region profile demonstrates a larger whole-call opportunity; do not repeat the no-change normalizer bypass, factor-cache, output-scale or seed-decline experiments.
