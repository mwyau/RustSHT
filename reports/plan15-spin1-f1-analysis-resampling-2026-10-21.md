# Plan 15 F1 spin-1 analysis resampling checkpoint (2026-10-21)

## Result

F1 spin-1 analysis was paying a large serial meridional-preparation cost at q8. The existing bounded order-parallel CC resampler also applies to F1 profiles, including its F1 half-grid phase shift. Enabling that route for sufficiently large f64 spin-1 F1 analysis work reduced all four matched q8 Rust medians. It does not change the F1 synthesis path.

At T1023 full order, the phase probe moved median meridional resampling from 91.0 ms (serial-order route) to 38.7 ms (parallel-order route), while the total probed Rust analysis phase fell from 196.5 to 136.0 ms. In the independent native whole-transform comparison, Rust q8 analysis fell from 214.8 to 141.6 ms (-34.1%); its Rust/DUCC ratio moved from 3.267× to 2.470×. The other three q8 analysis cases improved 22–45%.

## Matched whole-transform results

Warm latencies are medians across each source run. `after/before` compares Rust medians; DUCC was rebuilt once and reused, but its process medians varied between the independent campaigns. Ratios use each campaign’s paired process comparisons. Error is maximum relative L2 against DUCC. The full process/batch distributions and correctness records are in the [baseline JSON](data/plan15-spin1-f1-analysis-resampling-2026-10-21/baseline-results.json) and [candidate JSON](data/plan15-spin1-f1-analysis-resampling-2026-10-21/candidate-results.json).

| Case | Op | q | Before Rust ms | After Rust ms | DUCC ms (before/after) | Rust after/before | Rust/DUCC (before→after) | Max relative L2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| F1-T1023-m1023-n1024-exact-p2048-even-smooth | A | 1 | 475.893 | 482.611 | 262.891/254.741 | 1.014 | 1.908→1.876 | 7.33e-14 |
| F1-T1023-m1023-n1024-exact-p2048-even-smooth | A | 8 | 214.798 | 141.648 | 67.204/57.683 | 0.659 | 3.267→2.470 | 7.33e-14 |
| F1-T1023-m1023-n1024-exact-p2048-even-smooth | S | 1 | 464.718 | 309.099 | 159.270/141.383 | 0.665 | 2.918→2.228 | 1.23e-12 |
| F1-T1023-m1023-n1024-exact-p2048-even-smooth | S | 8 | 125.069 | 124.839 | 31.715/27.429 | 0.998 | 4.098→4.381 | 1.23e-12 |
| F1-T1023-m255-n1024-exact-p512-even-smooth | A | 1 | 192.961 | 289.639 | 67.824/99.801 | 1.501 | 2.848→2.902 | 8.90e-14 |
| F1-T1023-m255-n1024-exact-p512-even-smooth | A | 8 | 75.062 | 58.407 | 22.078/23.234 | 0.778 | 3.481→2.480 | 8.90e-14 |
| F1-T1023-m255-n1024-exact-p512-even-smooth | S | 1 | 201.895 | 206.637 | 58.456/54.676 | 1.023 | 3.450→3.760 | 1.30e-12 |
| F1-T1023-m255-n1024-exact-p512-even-smooth | S | 8 | 75.391 | 75.092 | 8.933/8.509 | 0.996 | 8.809→8.974 | 1.30e-12 |
| F1-T720-m180-n721-exact-p378-even-smooth | A | 1 | 80.804 | 121.495 | 56.791/57.023 | 1.504 | 2.042→2.143 | 6.76e-14 |
| F1-T720-m180-n721-exact-p378-even-smooth | A | 8 | 40.219 | 26.030 | 17.180/17.714 | 0.647 | 2.384→1.469 | 6.76e-14 |
| F1-T720-m180-n721-exact-p378-even-smooth | S | 1 | 45.681 | 76.843 | 21.140/23.603 | 1.682 | 2.175→3.262 | 8.08e-13 |
| F1-T720-m180-n721-exact-p378-even-smooth | S | 8 | 43.596 | 43.805 | 4.876/4.257 | 1.005 | 8.497→10.170 | 8.08e-13 |
| F1-T720-m720-n721-exact-p1458-even-smooth | A | 1 | 205.467 | 206.937 | 191.469/190.981 | 1.007 | 1.345→1.524 | 6.11e-14 |
| F1-T720-m720-n721-exact-p1458-even-smooth | A | 8 | 121.212 | 67.287 | 35.958/36.691 | 0.555 | 3.377→1.808 | 6.11e-14 |
| F1-T720-m720-n721-exact-p1458-even-smooth | S | 1 | 106.435 | 178.429 | 49.279/54.547 | 1.676 | 2.150→3.268 | 5.23e-13 |
| F1-T720-m720-n721-exact-p1458-even-smooth | S | 8 | 88.955 | 89.223 | 10.224/9.216 | 1.003 | 9.525→9.567 | 5.23e-13 |

For q8 analysis, Rust warm medians changed as follows:

- T720, `mmax=180`: 40.219→26.030 ms (-35.3%); Rust/DUCC 2.384→1.469×.
- T720, full `mmax`: 121.212→67.287 ms (-44.5%); 3.377→1.808×.
- T1023, `mmax=255`: 75.062→58.407 ms (-22.2%); 3.481→2.480×.
- T1023, full `mmax`: 214.798→141.648 ms (-34.1%); 3.267→2.470×.

Across the three candidate process pairs, q8 analysis Rust/DUCC ratios were 1.45–1.51×, 1.71–1.86×, 2.41–2.62× and 2.34–2.59× in the same case order. Relative L2 errors were unchanged from the baseline fixtures (6.76e-14, 6.11e-14, 8.90e-14 and 7.33e-14). Synthesis remained the expected control: Rust medians changed by at most 0.5% for q8, while its ratios were 8.50–10.17× and host/DUCC variation was visible. The q1 path does not meet the multiworker gate; q1 timings were multimodal between process campaigns and are not evidence of a source regression.

First-call Rust q8 analysis medians also fell: T720 reduced/full `44.87/130.07`→`30.01/71.55` ms; T1023 reduced/full `91.00/248.33`→`64.51/154.33` ms. Setup medians changed from `7.30/21.61/10.37/40.36` to `8.65/23.15/13.42/40.77` ms in the same order. These setup measurements are reported separately from transform latency.

## Measurement and implementation

- Baseline Rust source: `595e088026efef523f2f009ce27173358cff606c` (before enabling F1 order-parallel resampling). Candidate: `29e4b126ff5f9049ad0c3879aa4038bba9cb72a5`.
- Both used Rust 1.99.1 / LLVM 23.1.1 with `-C target-cpu=x86-64-v3`; pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2, numerical SHT sources `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, validation driver separately disables fast-math.
- Host: AMD Ryzen 9 5950X. q1 and q8 were pinned to the same physical-core policy within one L3 domain. Three interleaved process pairs; short cases used ten calibrated ~25 ms batches per process, and operations over 100 ms used repeated individually timed calls. Plans and workspaces were reused, checksums were outside timed regions, and reset/repeat validation passed.
- Identical case IDs and fixture SHA-256 hashes were used. All 4 geometries × 2 operations × q1/q8 completed in both runs; all 48 numerical comparisons in each run passed, with zero failures. q8 worker wait fractions had medians of 9.3% (baseline) and 12.1% (candidate), with an overall 2.2–20.9% range; the speedup direction is consistent across process pairs, but the magnitude remains subject to host scheduling noise. q1 timing spread was larger and is explicitly treated as inconclusive.
- The production gate is f64 spin-1 on CC/F1, resampling required, estimated order×ring work at least 65,536, and more than one active worker. Scratch remains bounded in the plan workspace and reused by tiled order jobs. q1 and small-work paths stay serial.

## Reproduction

From the repository root, with `DUCC_SOURCE` pointing to DUCC 0.42.0 at the pinned SHA and `BUILD_DIR` set to a writable build directory:

```sh
BUILD_DIR=build/plan15-f1 DUCC_SOURCE=../ducc-42ce
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' cargo build --release --example ducc_spin1_full_transform_parity --features parallel
cmake -S reports/data/plan15-spin1-native-baseline-2026-10-21 -B "$BUILD_DIR" -DDUCC_SRC="$DUCC_SOURCE" -DCMAKE_BUILD_TYPE=Release -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -DCMAKE_CXX_COMPILER=g++
cmake --build "$BUILD_DIR" --parallel 4
RUSTSHT_BENCH_ROOT="$PWD" python3 scripts/benchmark_plan15_f1_subset.py --ducc-src "$DUCC_SOURCE" --lmax 720 --lmax 1023 --grid F1 --process-pairs 3 --target-ms 25 --build-dir "$BUILD_DIR" --results-dir reports/data/plan15-spin1-f1-analysis-resampling-2026-10-21/rerun
```

The focused wrapper and CSV generator are archived beside the JSONs as [`benchmark_plan15_f1_subset.py`](data/plan15-spin1-f1-analysis-resampling-2026-10-21/benchmark_plan15_f1_subset.py) and [`summarize_ab.py`](data/plan15-spin1-f1-analysis-resampling-2026-10-21/summarize_ab.py). Run `python3 reports/data/plan15-spin1-f1-analysis-resampling-2026-10-21/summarize_ab.py` to regenerate the compact [summary CSV](data/plan15-spin1-f1-analysis-resampling-2026-10-21/summary.csv).

## Next

Keep the F1 analysis resampling route. Continue Plan 15 with integrated GL/CC/F1 validation and attribution of the remaining F1 synthesis order/recurrence cost; do not interpret the q1 run variation as a regression or extend this change to other operations.
