# Plan 17 f32 spin recurrence inlining (2026-10-22)

## Decision

Retain `#[inline(always)]` on `spin_f32::recurrence_step`. It preserves the recurrence's operation order and produces bitwise-identical Rust outputs, while reducing warmed full-transform latency by 35–45% on the matched T255/T720 GL spin-1 cases. The candidate still trails pinned DUCC by 19.9–82.4×, so this is a validated improvement, not parity.

## Source and method

- Baseline Rust production source: `b77bdfa2d8f8390d6178e1430c1fea981b05b62d`.
- Candidate: that source plus one `#[inline(always)]` attribute on `src/legendre/spin_f32.rs::recurrence_step`; patch SHA256 `f3880e58cb2248c7fdafd9611aee7c219f07cb54e031b60c239f173619860f3d`.
- DUCC: 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust 1.99.1 / LLVM 23.1.1, Cargo release defaults, no `target-cpu` override. Runtime dispatch selected Pulp V3 AVX2/FMA with eight f32 lanes for every worker.
- DUCC SHT units: GCC 15, `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; validation driver separately used `-fno-fast-math`.
- AMD Ryzen 9 5950X; q1 pinned to CPU 0, q8 to CPUs 0–7 within one L3 domain.
- f32 GL spin-1 STANDARD, full `mmax`, `phi0=0.23`, even-smooth longitude: T255 (`256 × 512`) and T720 (`721 × 1458`), analysis and synthesis, q1/q8. Baseline/candidate and candidate/DUCC used identical deterministic fixtures and reused plans/workspaces. Fresh processes were interleaved in three pairs. Calls below 100 ms used ten calibrated 25 ms batches per process; longer calls used five individual timings per process. T255 q8 synthesis had one high-variance process pair, so it was repeated for three more pairs; all six pairs are retained.

The sanitized per-batch observations, summaries, cold/warm spot checks and complete source/build metadata are in [the machine-readable results](data/plan17-f32-spin-recurrence-inline-2026-10-22/results.json), [summary CSV](data/plan17-f32-spin-recurrence-inline-2026-10-22/results.csv), [sample CSV](data/plan17-f32-spin-recurrence-inline-2026-10-22/samples.csv) and [cold/warm CSV](data/plan17-f32-spin-recurrence-inline-2026-10-22/cold-warm.csv). CPU pressure snapshots and disassembly remain local.

## Full-transform results

Times are median milliseconds. `candidate/base` is the paired Rust candidate-to-baseline ratio; values below one favor the candidate. The last column compares candidate Rust directly with DUCC using paired process medians.

| T | Operation | Threads | Rust baseline | Rust candidate | Candidate/base | DUCC | Candidate/DUCC | Relative L2 vs DUCC |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 255 | Analysis | q1 | 112.189 | 67.850 | 0.605 | 2.758 | 24.60× | 6.54e-6 |
| 255 | Analysis | q8 | 14.869 | 8.934 | 0.601 | 0.452 | 19.89× | 6.54e-6 |
| 255 | Synthesis | q1 | 129.071 | 83.808 | 0.652 | 4.095 | 20.14× | 3.71e-5 |
| 255 | Synthesis | q8 | 21.552 | 15.879 | 0.734 | 0.572 | 27.79× | 3.71e-5 |
| 720 | Analysis | q1 | 2235.459 | 1235.773 | 0.552 | 28.525 | 43.23× | 1.57e-5 |
| 720 | Analysis | q8 | 295.056 | 163.299 | 0.553 | 4.102 | 40.00× | 1.57e-5 |
| 720 | Synthesis | q1 | 2722.221 | 1721.936 | 0.633 | 39.812 | 41.07× | 1.43e-4 |
| 720 | Synthesis | q8 | 705.658 | 463.941 | 0.657 | 5.569 | 82.39× | 1.43e-4 |

The Rust candidate and baseline outputs were bitwise identical for every A/B fixture. DUCC agreement stayed within the existing f32 comparison behavior; no thresholds changed. One T255 q8 synthesis A/B process showed wide batch dispersion and a 0.920 ratio. The additional three interleaved process pairs measured 0.729–0.735, but the original samples were retained; the six-pair median is 0.734. Direct candidate/DUCC timing remained consistent across its three pairs.

First-call behavior was checked separately at q8. T720 analysis first call changed from 294.1 to 160.3 ms and synthesis from 715.0 to 454.1 ms; setup stayed within 3.4–3.7 ms. T255 first calls changed from 16.49 to 9.00 ms for analysis and 21.53 to 15.13 ms for synthesis. These spot checks are listed separately from warmed batch summaries.

## Assembly finding

Before the change, the executed V3 `recurrence_step` was an outlined helper called twice per degree step. Its optimized symbol used a 448-byte stack frame and called Pulp/core-arch helpers for YMM broadcast, FMA and negation, moving vector values through memory. With the attribute, the separate recurrence-step symbol disappears; the eight-lane recurrence FMA instructions are emitted directly in the caller's degree loop. The caller's stack frame grew from 392 to 424 bytes, so the gain is removal of repeated helper-call and vector-wrapper work, not a smaller total frame. The scalar lane-wise `normalize_pair_lanes` pass remains in the caller path and is a concrete next profiling target.

## Validation

- `cargo fmt --all --check`: passed.
- Strict Clippy, all targets/all features and no-default-features: passed with `-A unexpected-cfgs` for the existing benchmark-only paired-FFT cfg name.
- `cargo test --release --all-features`: 269 unit tests, 4 extreme-value tests, 11 f32 integration tests and 1 workspace test passed; 36 unit tests and 1 f32 test ignored.
- `cargo test --release --no-default-features`: 219 unit tests, 4 extreme-value tests, 11 f32 integration tests and 1 workspace test passed; 17 unit tests and 1 f32 test ignored.
- All eight matched benchmark cases reported repeat/reset checks passing. Existing spin-1 through spin-4, analytic, adjoint, polar and extreme-value coverage passed unchanged.

## Reproduction

The checked-in `scripts/benchmark_plan17_spin_recurrence_inline.py` drives the baseline/candidate/DUCC comparison. Build the baseline in a worktree at `b77bdfa2d8f8390d6178e1430c1fea981b05b62d`, build the candidate at the commit containing this change, and build the DUCC worker from pinned `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` with the flags above. Then run:

```sh
python3 scripts/benchmark_plan17_spin_recurrence_inline.py \
  --baseline-rust /path/to/baseline/ducc_f32_full_transform_parity \
  --candidate-rust /path/to/candidate/ducc_f32_full_transform_parity \
  --ducc /path/to/ducc_f32_full_transform_parity \
  --outdir /path/to/results --lmax 255 720 --threads 1 8 \
  --process-pairs 3 --target-ms 25 --q1-cpu 0 \
  --q8-cpus 0,1,2,3,4,5,6,7
```

The script writes JSON summaries and per-batch/cold-warm CSVs. The driver and fixture protocol are shared with `scripts/benchmark_ducc_f32_native.py`; the native worker interface is documented by that script's `--help` and the example's `--worker` usage.

## Next

The inlining change closes one measured code-generation cost. The remaining largest absolute gap is T720 q1 spin-1 synthesis (candidate 1.72 s versus DUCC 39.8 ms); the next Plan 17 step is to separate scalar lane normalization, recurrence, coefficient visitor/accumulation and scheduling costs, then target the largest measured component.
