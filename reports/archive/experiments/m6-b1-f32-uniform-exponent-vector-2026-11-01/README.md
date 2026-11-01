# B1 f32 uniform exponent-vector fill

## Result

This B1-only candidate reuses one binary scale factor for each eight-lane vector whose exponents are identical. Mixed vectors still use the prior per-lane computation; tails are unchanged. The helper is called only by `FusedAnalysisPrecision for f32`. It does not alter the f64 path, recurrence, output accumulation, public APIs or exceptional-lane behavior.

The 15-pair whole-transform screen improved every GL T720 f32 cell versus exact parent `098595c653c3ca70ec1916266689e0322814c8d9`; all four paired 95% log-t intervals are below 1.00. Rust output was bitwise identical to the parent across all pairs. The remaining native-f32/DUCC gap is 3.23–4.61×.

| GL T720 B1 | Parent f32 ms | Candidate f32 ms | DUCC f32 ms | Candidate/parent, 95% CI | Candidate/DUCC, 95% CI |
|---|---:|---:|---:|---:|---:|
| Full q1 | 67.547 | 65.310 | 14.173 | 0.96756 [0.96187, 0.97329] | 4.60893 [4.58930, 4.62865] GAP |
| Full q8 | 9.871 | 9.550 | 2.198 | 0.96897 [0.94631, 0.99216] | 4.32505 [4.23656, 4.41538] GAP |
| Reduced q1 | 21.271 | 20.784 | 5.789 | 0.97734 [0.96958, 0.98517] | 3.57399 [3.54052, 3.60778] GAP |
| Reduced q8 | 2.975 | 2.917 | 0.899 | 0.97868 [0.96862, 0.98884] | 3.23133 [3.20228, 3.26064] GAP |

Ratios use 15 interleaved parent/candidate/DUCC pairs, with paired geometric means and 95% t intervals on log ratios. Each recorded pair is the median of ten warm whole-call samples. Calls include scheduling, Legendre work, FFT and output handling. Candidate versus parent maximum absolute and relative L2 output differences were zero. Candidate versus DUCC relative L2 was `1.8391e-5` on full order and `2.2203e-5` on reduced order. Repeat and workspace-reset checks passed.

## Diagnosis and machine code

The matched T720 phase profile at `410c9ec` puts Legendre recurrence and accumulation at 94–97% of f32 call time; FFT is 3–6%. The full-q1 f32 Callgrind profile counted 1.239B instructions. Scaled accumulation plus factor preparation and range check plus normalization each account for about 539M inclusive instructions (43.5% apiece; these buckets overlap). Factor preparation alone is 410M instructions but is nested in scaled accumulation; normalization is 232M and nested in the range/normalization bucket. Ordinary accumulation is 20.9M and recurrence advancement is 49.8M. These are instruction counts, not cycle shares. Hardware `perf stat`/`perf record` are unavailable on this host (`perf_event_paranoid=4`).

A production-input probe found identical exponents in 2,060,540 of 2,232,713 f32 factor vectors (92.3%), while no complete scalar-analysis tile was uniform. Reusing a factor within a vector therefore targets repeated setup without reinstating the rejected whole-tile grouping strategy.

The direct helper microbenchmark used nine interleaved samples of 20,000 fills on CPU 0, with eight vectors containing seven uniform and one mixed vector. The baseline per-lane fill had a 429.844 us median; the candidate had a 324.555 us median. The geometric paired ratio was 0.802 [0.705, 0.913]. The factor probe is local evidence only; the whole-call screen is the acceptance evidence.

The executed V3 kernel is eight-lane AVX2/FMA. The saved excerpt shows vector exponent loads/classification and factor-vector construction, followed by the existing `vmulps`/`vfmadd231ps` scaled accumulation loop. The V3 monomorphized kernel grew by 231 bytes (25,018 to 25,249 bytes); the steady-state accumulation remains vectorized. No f64 or spin-1 code was changed. The f64 B1/DUCC baseline remains 1.226–1.310× from its existing matched screen and was not rerun for this f32-only change.

## Reproduction and provenance

- Candidate source: parent `098595c653c3ca70ec1916266689e0322814c8d9` plus the exact uncompressed Git diff in [`candidate.patch.gz`](candidate.patch.gz), SHA-256 `7bcb28969ff08fb0d49033f61e724c03ed68ec136427e3dffc6979e1fa848a65`.
- Pinned DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- CPU: AMD Ryzen 9 5950X; q1 was pinned to one physical core and q8 to eight distinct physical cores.
- Rust: `1.99.1 (58cf8f9c3 2026-10-13)`, LLVM 23.1.1; flags `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.
- Runtime dispatch: Pulp V3 AVX2/FMA, eight f32 lanes. DUCC used its native public f32 path.
- Worker SHA-256: parent `17a184c6ae41b250c83a9728bfe24ff6f016677f6198f7efe79e9d89999b9b18`; candidate `6fcb31167af51511d63593e5f8ddc871c6ea1a52655cd930ef549333bb27cd06`; DUCC `6c70f3b27a594d2f3e37d12bc5be650183072e439435640aedc397d566dc4100`.
- Per-pair times, fixture hashes, dispatch and output metrics: [`results-15pair.json`](results-15pair.json) and [`paired-ratios.csv`](paired-ratios.csv). The screen template is [`screen-15pair.py`](screen-15pair.py); it takes worker, fixture, candidate-tree and output paths through `RUSTSHT_*` environment variables.
- The direct probe can be appended temporarily to `analysis/tests.rs` and run pinned with `taskset -c "$RUSTSHT_CPU" env CARGO_TARGET_DIR="$CARGO_TARGET_DIR" RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' cargo test --release --lib --all-features benchmark_b1_f32_uniform_factor_fill -- --ignored --nocapture --test-threads=1`; set `RUSTSHT_CPU` to one selected core and `CARGO_TARGET_DIR` to a local isolated target directory first. Its exact snippet and samples are [`factor-fill-microbenchmark-snippet.rs`](factor-fill-microbenchmark-snippet.rs) and [`factor-fill-microbenchmark.json`](factor-fill-microbenchmark.json).
- `perf_event_paranoid=4` prevented hardware-counter collection. The AVX2 excerpt is [`candidate-avx2-excerpt.txt`](candidate-avx2-excerpt.txt).
- All-feature Rust tests passed (328 unit, 4 extreme-scaling, 14 f32 integration, 1 workspace compatibility); strict all-target/all-feature Clippy passed with warnings denied; formatting passed; all 20 Python tests passed. CI run `41285407559` for the parent is green; the candidate's workflow is checked after push.
- This candidate changes only B1 f32 factor preparation; spin-1 synthesis and the f64 path are unchanged. The all-features suite also passed the existing T1023 spin-1 cache/reuse control. I did not repeat the 15-pair T1023 wall-time screen for this isolated f32-only change; its prior result is in the [range-predicate package](../m6-b1-f32-range-predicate-2026-11-01/README.md).

## NEXT

Do not repeat factor-fill variants without new profile evidence. Profile the current B1 f64 path against DUCC f64 at the ordinary/scaled accumulation boundary, since its remaining 1.23–1.31× deficit may expose a shared cost. For f32, isolate the remaining range classification/normalization and exceptional work from the scaled accumulation before selecting a different optimization. Keep B0, B2 and B3 parked.
