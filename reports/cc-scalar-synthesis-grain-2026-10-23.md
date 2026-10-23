# CC scalar synthesis task-grain experiment

**Status:** The CC-only finer task grain is retained as a measured q8 improvement. It does not close the remaining CC synthesis gap. The full CC coverage campaign remains incomplete.

## Change

The general scalar synthesis route groups CC rings and passed a minimum chunk length of `ceil(parallel_groups / execution_threads)` to Rayon. The T720 q8 profile showed twelve 64-ring groups being scheduled as four coarse tasks. CC now permits one ring group per task. Each group already owns its recurrence workspace, so this does not add transform-sized scratch. GL and the serial q1 route keep their existing behavior.

## q8 results

Three independent process pairs produced 30 timed batches per backend for each short case. The table compares the candidate against the saved pre-change CC baseline and gives both Rust/DUCC ratios. Latencies are warm milliseconds. Full per-case details are in the [summary CSV](data/cc-scalar-synthesis-grain-2026-10-23/summary.csv); sanitized provenance and raw artifact hashes are in [provenance.json](data/cc-scalar-synthesis-grain-2026-10-23/provenance.json).

| T / mmax | Baseline Rust / DUCC | Candidate Rust / DUCC | Rust change | Rust / DUCC ratio, baseline → candidate |
|---|---:|---:|---:|---:|
| 255 / 63 | 0.723 / 0.133 | 0.520 / 0.160 | −28.1% | 5.44× → 3.60× |
| 255 / 255 | 3.128 / 0.299 | 2.199 / 0.363 | −29.7% | 10.45× → 6.45× |
| 511 / 127 | 6.780 / 0.698 | 5.989 / 0.698 | −11.7% | 9.71× → 9.04× |
| 511 / 511 | 21.164 / 1.402 | 16.393 / 1.557 | −22.5% | 15.10× → 10.36× |
| 720 / 180 | 18.820 / 1.645 | 14.257 / 1.474 | −24.2% | 11.44× → 9.94× |
| 720 / 720 | 47.916 / 2.951 | 27.231 / 3.226 | −43.2% | 16.24× → 8.11× |

The candidate still trails DUCC by 3.60–10.36× across these q8 cases. The q1 implementation path is unchanged; its measurements are retained as controls in the CSV, not credited to this change. The T720 full-order q1 Rust sample had 20 timed batches, so its apparent latency shift is not treated as a result.

## Correctness and measurement limits

All 48 candidate Rust/DUCC random-map comparisons were accepted; the maximum relative L2 error was `1.742e-12`. All 96 worker reset flags passed, with no failure records. These are the benchmark's random-output and reset checks, not a replacement for the complete analytic/adjoint validation suite.

The candidate ran on a Ryzen 9 5950X with AVX2/FMA, q1 pinned to one physical core and q8 to eight cores. Rust used `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; DUCC 0.42.0 was built for x86-64-v3 at the pinned revision `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The competing JAX build had stopped, but scheduler wait remained elevated: the run's median wait fraction was 14.3%, p90 27.1%. T255 q8 and T511 quarter-order batches were partly outside the 20–100 ms target window. Treat the observed changes as a strong directional gain under host contention, not as a final low-noise crossover estimate.

The saved baseline used binary SHA-256 `26203773054c1e0a37e015d435cac269a59b3e599ea31e988dae6917a2c7da3a`; the candidate binary SHA-256 is `75d47f8ddded269dd47fb7b20b7138b2360a44f0fa1bcb8bf08e13d3345416ba`. The production source files were unchanged between the saved baseline revision and the candidate base revision; the candidate adds the patch identified in `provenance.json`. Raw candidate JSON and per-process host telemetry remain outside the repository; their capture identifiers and hashes are recorded in `provenance.json`.

The runner currently raises an `AttributeError` while checkpointing a fresh build because `binary_build_record` is `null`. This run used the small in-memory compatibility wrapper in [the data directory](data/cc-scalar-synthesis-grain-2026-10-23/checkpoint_identity_compat.py), which normalizes only that metadata field; the timed worker and measurement loops were unchanged.

## Reproduction and checks

Run from the repository root:

```sh
python3 reports/data/cc-scalar-synthesis-grain-2026-10-23/checkpoint_identity_compat.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --grid CC --lmax 255 --lmax 511 --lmax 720 \
  --operation synthesis --threads 1 --threads 8 \
  --longitude-mode even_smooth --results-dir "$RESULTS_DIR" \
  --no-report --sweep-id cc-grain-candidate --retain-local-host-telemetry
```

`cargo fmt --all -- --check`, release build through the benchmark runner, strict example Clippy, and `cargo check --release --no-default-features --lib` passed. The benchmark does not establish full CC parity or explain the q1 synthesis gap; the ordered CC synthesis path remains the larger follow-up.
