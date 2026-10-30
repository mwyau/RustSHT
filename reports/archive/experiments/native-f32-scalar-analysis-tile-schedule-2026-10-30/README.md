# Native-f32 scalar-analysis tile schedule screen (2026-10-30)

## Scope and provenance

This bounded experiment changes only the latitude-pair tile used by the shared scalar-analysis path. It leaves recurrence, accumulation formulas, precision dispatch, and public API unchanged. The isolated candidate patch is recoverable as `tile32.patch.gz`; its uncompressed SHA-256 is `471f15a6f57a5d47ff9569dcc554def7a6b5886ccc6d157374e4b58df8b240ef`. Its parent source is `797d2720cfb17cd443c1ccdf835063b2433006be`; the candidate uses native-f32 inputs, recurrence, SIMD, accumulation, spectra, and FFT. f64 keeps the 256-pair tile. This schedule is not integrated into `dev` because high-degree numerical accuracy is not independently validated.

Reference: pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; DUCC f32 worker SHA-256 `1e8412966f7bf306a1fea8141969208f661d5a705f7737e7a8f0dc1cc74cc36d`. Rust f32 and f64 workers: `b7e668b51d2aef4d564a6626e88975a94bcfae70701188a15b2e32378400077f` and `700a733871e44ce77bd6382686e81206f1bc986fcc0a67f373fbbbbc191c678c`. Rust version: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`; flags: `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. Hardware: AMD Ryzen 9 5950X; q1/q8 used the same fixed taskset affinity across backends. Exact CPU assignment remains in the per-run JSON.

The runners regenerate deterministic f32 fixtures, widen identical component values only for the matched f64 diagnostic worker, and perform three interleaved fresh-process pairs. Fixture SHA-256 values and all pair timings are in the JSON files. Generated input copies, output maps, and benchmark binaries are not archived.

To rerun, check out the stated parent source, build the Rust parity examples with release/parallel features and `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, and set `RUSTSHT_DUCC_F32_WORKER`, `RUSTSHT_Q1_CPUSET`, `RUSTSHT_Q8_CPUSET`, and `RUSTSHT_CPU_DESCRIPTION` to the pinned worker and matching benchmark setup recorded in the JSON. Run `python3 reports/archive/experiments/native-f32-scalar-analysis-tile-schedule-2026-10-30/run_baseline.py` before applying `tile32.patch.gz`, then run `run_tile32.py` after applying it. Apply a saved patch with `gzip -dc <patch>.gz | git apply`. The tile16/tile64 patches reproduce the bounded schedule variants. Runners write regenerated inputs and outputs under separate `rerun-*` directories; do not include those files in the archive.

## Median whole-call screen

| Geometry / threads | Tile | f32 ms | f64 ms | f32/f64 | DUCC f32 ms | f32/DUCC |
|---|---:|---:|---:|---:|---:|---:|
| M720 / q1 | 256 baseline | 127.407 | 19.865 | 6.419x | 14.375 | 8.884x |
| M720 / q1 | 16 | 54.655 | 20.346 | 2.689x | 14.779 | 3.698x |
| M720 / q1 | 32 | 50.908 | 19.960 | 2.550x | 14.366 | 3.531x |
| M720 / q1 | 64 | 64.718 | 19.834 | 3.263x | 14.404 | 4.438x |
| M720 / q8 | 256 baseline | 17.735 | 3.555 | 5.249x | 2.330 | 7.612x |
| M720 / q8 | 16 | 8.551 | 3.555 | 2.483x | 2.365 | 3.609x |
| M720 / q8 | 32 | 8.253 | 3.558 | 2.319x | 2.335 | 3.535x |
| M720 / q8 | 64 | 9.138 | 3.559 | 2.568x | 2.175 | 4.223x |
| M180 / q1 | 256 baseline | 41.724 | 8.532 | 4.890x | 5.826 | 7.153x |
| M180 / q1 | 16 | 21.226 | 8.416 | 2.539x | 8.515 | 3.608x |
| M180 / q1 | 32 | 20.610 | 8.482 | 2.430x | 5.829 | 3.555x |
| M180 / q1 | 64 | 24.828 | 8.626 | 2.888x | 5.834 | 4.256x |
| M180 / q8 | 256 baseline | 6.333 | 1.437 | 4.408x | 1.002 | 6.321x |
| M180 / q8 | 16 | 3.418 | 1.457 | 2.334x | 1.029 | 3.322x |
| M180 / q8 | 32 | 3.215 | 1.391 | 2.255x | 1.227 | 2.621x |
| M180 / q8 | 64 | 3.712 | 1.458 | 2.564x | 1.045 | 3.551x |

These are three-pair screening medians, without a consequential-performance confidence claim. Tile32 is best of the bounded 16/32/64 screen, but misses the required <=2x f64 target in every cell (2.255–2.550x); it remains 2.621–3.555x DUCC. Tile16 and tile64 are slower. The unmodified 256 tile was substantially slower (4.408–6.419x f64).

## Route counts and blocker

At tile256 the diagnostic route count reports scaled accumulation for 52.9% of full-grid and 45.9% of reduced-grid pair visits. At tile32 the scaled fractions fall to 20.0% and 13.3%, respectively; more blocks take the ordinary SIMD path. This accounts for most of the measured improvement from tile256. The remaining scaled path and increased block/job overhead keep all cells above 2x f64. Tile16 and tile64 measurements bound the tile-size search; no further tile sweep is justified before moving to spin-1 analysis.

A cached zero-factor-lane list improved q1 in several cases but was neutral or marginal under q8 and did not close the gap; its patch and measurements are preserved as rejected diagnostic context. No recurrence or accumulation microkernel was changed in this experiment.

## Numerical and test status

The tile32 random T720 f32-vs-DUCC relative L2 is `1.8391e-5` full and `2.2203e-5` reduced, unchanged to the JSON precision from the 256-tile screen. These are DUCC comparisons, not an independent numerical oracle. High-degree accuracy remains UNMEASURED and is not accepted by this performance screen.

On the exact tile32 source, `cargo fmt --all -- --check` passed; scalar-analysis library tests passed (18 passed, 8 manual diagnostics ignored); `cargo test --release --features parallel --test f32 scalar_analysis` passed (1 analytic/adjoint/workspace-reset test). Strict Clippy with `-D warnings` fails on the unchanged base as well as this candidate: dead-code fields in `scalar_ordered::kernel::WorkCounters` and two `assign_op_pattern` diagnostics in `scalar_ordered/tests.rs`. Keep the source patch isolated pending independent high-degree numerical validation.

## Decision / NEXT

Keep tile32 isolated as the best bounded scalar-analysis schedule result and mark all four interim cells GAP. Do not spend more time tuning tile size or claim final accuracy. Continue to the remaining spin-1 analysis bottleneck, then revisit scalar-synthesis numerical convergence only after the breadth-first families are complete.

## Files

- `baseline.json`, `run_baseline.py`: source-matched 256-tile baseline.
- `tile32.json`, `run_tile32.py`, `tile32.patch.gz`: best schedule candidate and reproducer.
- `tile16.json` / `tile16.patch.gz`, `tile64.json` / `tile64.patch.gz`: bounded schedule variants.
- `hard-lanes.json`, `run_hard_lanes.py`, `hard-lanes.patch.gz`: rejected one-change micro-optimization.
- `SHA256SUMS`: checksums for the archived JSON, runner, and patch files (README excluded).
- Patches are context diffs against the source parent stated above. `SHA256SUMS` verifies the compressed artifacts; uncompressed patch hashes are:
  - `tile16.patch.gz`: `3ad63fa312fe435c2a31a12714e3bf6424c1a5a928adefc81f29e5c4282cd1a9`
  - `tile32.patch.gz`: `471f15a6f57a5d47ff9569dcc554def7a6b5886ccc6d157374e4b58df8b240ef`
  - `tile64.patch.gz`: `64e069980221de4c5b0ee32854bb6d6a81c78751862f0402b56e969786e82f12`
  - `hard-lanes.patch.gz`: `47b4a5cc3da45b6efe2b2b2a3584d1cd072445d013de20e833610554013c64ad`
