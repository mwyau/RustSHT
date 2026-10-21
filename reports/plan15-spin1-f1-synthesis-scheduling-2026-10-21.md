# Plan 15 F1 spin-1 synthesis scheduling checkpoint (2026-10-21)

## Result

The q8 F1 synthesis profile showed that the recurrence and spectrum stage dominates the remaining Rust latency. The ordinary T1023 partition has eight 128-ring groups for eight workers, so static assignment leaves no spare job to rebalance. Splitting those groups into 64-ring pieces increased T1023 reduced-order Rust latency by 9.6% while changing full-order latency by only -1.3%; that finer-group experiment is rejected.

T720 already produces 12 64-ring groups for eight workers. Enabling the existing bounded dynamic queue only when the default geometry has spare groups lets it rebalance that existing work without adding coefficient-preparation passes or changing the group size. The final q8 screen reduced Rust synthesis latency by 25.9% for reduced order and 41.2% for full order. The remaining F1 gap is still large: 7.07× (reduced) and 5.70× (full) Rust/DUCC.

## Matched q8 whole-transform results

| Geometry | Op | Baseline Rust ms | Candidate Rust ms | Baseline DUCC ms | Candidate DUCC ms | Rust/DUCC before→after | Max relative L2 |
|---|---:|---:|---:|---:|---:|---:|---:|
| F1-T1023-m1023-n1024-exact-p2048-even-smooth | A | 141.648 | 145.242 | 57.683 | 66.538 | 2.470→2.202 | 7.33e-14 |
| F1-T1023-m1023-n1024-exact-p2048-even-smooth | S | 124.839 | 132.370 | 27.429 | 32.965 | 4.381→4.238 | 1.23e-12 |
| F1-T1023-m255-n1024-exact-p512-even-smooth | A | 58.407 | 52.403 | 23.234 | 23.645 | 2.480→2.215 | 8.90e-14 |
| F1-T1023-m255-n1024-exact-p512-even-smooth | S | 75.092 | 78.168 | 8.509 | 9.785 | 8.974→8.709 | 1.30e-12 |
| F1-T720-m180-n721-exact-p378-even-smooth | A | 26.030 | 28.286 | 17.714 | 17.838 | 1.469→1.533 | 6.76e-14 |
| F1-T720-m180-n721-exact-p378-even-smooth | S | 43.805 | 32.459 | 4.257 | 4.608 | 10.170→7.073 | 8.08e-13 |
| F1-T720-m720-n721-exact-p1458-even-smooth | A | 67.287 | 67.053 | 36.691 | 35.568 | 1.808→1.822 | 6.11e-14 |
| F1-T720-m720-n721-exact-p1458-even-smooth | S | 89.223 | 52.422 | 9.216 | 9.186 | 9.567→5.700 | 5.23e-13 |

Candidate q8 process-pair ratios for T720 synthesis were 6.96–7.18× reduced and 5.43–6.80× full, below the corresponding baseline ranges 9.61–11.54× and 9.38–9.83×. Candidate Rust medians changed from 43.805 to 32.459 ms reduced and 89.223 to 52.422 ms full. Candidate DUCC was 4.608 and 9.186 ms. The candidate outputs remained within the same DUCC error as baseline (maximum `1.30e-12` across all measured rows); all 48 comparisons per run passed finite, reset and repeat checks.

T1023 stayed on its original group size because the existing eight groups equal the eight active workers. Its +4–6% Rust medians between campaigns are not attributed to this change: the route is not selected, candidate q8 wait fractions rose to 5–13% from 2–4%, and DUCC medians also rose 15–20%. The full first-call, setup, batch, host wait and frequency distributions are retained in the [baseline JSON](data/plan15-spin1-f1-synthesis-scheduling-2026-10-21/baseline-results.json) and [candidate JSON](data/plan15-spin1-f1-synthesis-scheduling-2026-10-21/candidate-results.json).

## Stage attribution and policy

The candidate T720 q8 test-only phase probe measured median order scheduling/Legendre recurrence/spectral accumulation at 31.4 ms reduced and 46.0 ms full; two inverse longitude FFTs and map writeback were 0.77 and 2.25 ms. This is not a DUCC-matched stage comparison, but confirms that the remaining Rust cost stays in the recurrence/order path rather than output FFTs.

The production route is restricted to f64 STANDARD spin-1 F1 synthesis with at least eight active workers, existing ring-group count greater than worker count, and estimated `mmax × ntheta` work of at least 12,000 pairs per worker. It reuses the existing ring-group size and dynamic queue, with no added workspace buffers. q1, T1023's one-group-per-worker partition, lower work and other grids retain their existing route.

The production candidate is commit `32bb0a2f` (based on `48b8efa9`). The measurement binary was built from the matching pre-commit working tree whose recorded HEAD is `48b8efa9`; its SHA-256 is in the JSON. That measurement used a fixed 100,000 order-ring work gate; the committed code expresses the same gate as 12,000 pairs per worker. Every measured case takes the same route under both expressions. The baseline binary is from `29e4b126`. Both campaigns use Rust 1.99.1 / LLVM 23.1.1 with `-C target-cpu=x86-64-v3`, and pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2, SHT `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the validation driver uses `-fno-fast-math`.

Three process pairs measured the exact T720/T1023 full and quarter-order even-smooth F1 cases. Rust and DUCC were interleaved within each pair; short calls used ten calibrated ~25 ms batches and long calls were individually timed. q8 host contention was elevated (Rust wait 9–19% for T720 and 5–15% for T1023), so absolute gain size remains host-sensitive; the direction was consistent across all T720 q8 process pairs. All numerical outputs were finite, repeated calls reset consistently, and the focused serial/q8 workspace-reuse and adjoint test passed. The complete release and no-default-feature suites pass with the Rust test harness serialized; one unrelated worker-observation test was flaky under concurrent Rust test execution and passed alone and in the serialized suite.

## Reproduction

With DUCC 0.42.0 at the pinned SHA in `DUCC_SOURCE`:

```sh
BUILD_DIR=build/plan15-f1 DUCC_SOURCE=../ducc-42ce
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' cargo build --release --example ducc_spin1_full_transform_parity --features parallel
cmake -S reports/data/plan15-spin1-native-baseline-2026-10-21 -B "$BUILD_DIR" -DDUCC_SRC="$DUCC_SOURCE" -DCMAKE_BUILD_TYPE=Release -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -DCMAKE_CXX_COMPILER=g++
cmake --build "$BUILD_DIR" --parallel 4
RUSTSHT_BENCH_ROOT="$PWD" python3 scripts/benchmark_plan15_f1_subset.py --ducc-src "$DUCC_SOURCE" --lmax 720 --lmax 1023 --grid F1 --process-pairs 3 --target-ms 25 --build-dir "$BUILD_DIR" --results-dir reports/data/plan15-spin1-f1-synthesis-scheduling-2026-10-21/rerun
python3 reports/data/plan15-spin1-f1-synthesis-scheduling-2026-10-21/summarize_ab.py
```

The wrapper and summary generator are archived beside the raw distributions. The T1023 64-ring rejected experiment is retained locally for historical comparison; it is not a production path.

## Next

Keep the T720 spare-group queue. Do not split T1023 groups again without a new mechanism that avoids the measured reduced-order regression. Continue with the Plan 15 integrated GL/CC/F1 validation and closeout assessment, recording the remaining recurrence-stage gap.

