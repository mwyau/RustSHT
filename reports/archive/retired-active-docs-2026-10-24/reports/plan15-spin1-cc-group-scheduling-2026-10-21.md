# Plan 15: CC surplus-group scheduling and synthesis phase attribution

**Production change:** `ae3278a1bdd8e154d0bf2beb03b182193917578e`.
**Static baseline:** `1b832770c4dc052deb48b65a62eff34bed3864f5`.
**DUCC:** 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Finding

Exact T720 CC q8 synthesis has 12 ring groups for 8 workers. Its static
parallel iterator applies `with_min_len(ceil(12/8))`, or a minimum of two
groups per leaf, which can expose at most six leaf tasks. The existing
one-group queue already used by the GL and F1 spin-1 paths can keep eight
recurrence workspaces active and rebalance those 12 group jobs. The change
selects that queue only for the checked coefficient-cache route when
`group_count > execution_threads`; cache-ineligible and non-surplus cases keep
the static iterator.

The test-only phase probe corroborates the mechanism: the queue used Rayon
worker IDs 0–7 in every measured repetition. The q8 CC order/recurrence/spectral
phase fell from 37.01 to 18.99 ms full-order and 20.00 to 15.64 ms at
`mmax=180`. Its inverse FFT/writeback phase stayed at 1.21 and 0.18 ms. This
is a scheduling correction around the existing numerical kernel; recurrence,
coefficient transformation, FFTs, and safety behavior are unchanged.

## Matched CC T720 results

The native benchmark reused plans/workspaces and identical fixtures, used
three interleaved process pairs, and collected ten calibrated ~25 ms batches
per short process. q8 was pinned to physical CPUs 0–7. Values are pooled
medians with p10–p90 across 30 batches; paired candidate/baseline ratios use
the three process medians.

| Order | Rust static ms | Rust queued ms | DUCC ms | Candidate/static paired ratios | Rust/DUCC before → after | Rust−DUCC before → after |
|---|---:|---:|---:|---|---:|---:|
| Full (`mmax=720`) | 40.027 (39.785–40.693) | 23.596 (23.298–24.108) | 6.087 (6.051–6.307) | 0.593, 0.581, 0.591; median **0.591** | 6.575× → 3.868× | +33.94 → +17.51 ms |
| Reduced (`mmax=180`) | 21.237 (20.780–21.835) | 17.179 (17.075–17.465) | 2.647 (2.637–2.680) | 0.800, 0.825, 0.806; median **0.806** | 8.032× → 6.484× | +18.59 → +14.53 ms |

All three pairs improved: 40.9% full-order and 19.4% reduced-order median
latency reductions. The change does not close the remaining recurrence gap.
q1 synthesis and q8 analysis controls stayed within about 2% of baseline,
with paired ratios crossing 1.0. All 24 Rust/DUCC numerical comparisons
passed. Queued output was bitwise identical to static output; maximum DUCC
relative L2 error was `3.64e-13` reduced and `2.29e-13` full. Every worker
reported finite and repeat/reset-consistent outputs.

## Gate control at T511

A second exact-CC A/B used the same binaries, three interleaved pairs, and
q1/q8 analysis/synthesis controls. The q8 synthesis rows show why the route
is gated by actual cached work and surplus groups rather than applied
universally.

| Order | Rust static ms | Rust candidate ms | DUCC ms | Candidate/static paired ratios | Rust/DUCC before → after |
|---|---:|---:|---:|---|---:|
| Full (`mmax=511`), cache and 9 groups eligible | 17.768 (17.456–18.634) | 14.076 (13.562–15.264) | 2.516 (2.496–2.561) | 0.819, 0.775, 0.781; median **0.781** | 7.073× → 5.501× |
| Reduced (`mmax=127`), below cache work gate | 12.591 (12.253–13.211) | 12.625 (12.428–13.350) | 1.092 (1.081–1.109) | 1.001, 1.005, 1.009; median **1.005** | 11.494× → 11.546× |

The full case improved 21.9% in paired medians. Reduced order did not enter the
cache/queue route and stayed within about 1% of static. Outputs were bitwise
identical to static; all outputs were finite and reset/repeat checks passed.
Maximum relative L2 versus DUCC was `1.21e-13` full and `1.44e-13` reduced.
This supports the geometry/work gate at one lower eligible bandlimit; it is
not a universal dispatch claim for other CPUs.

## CC and F1 stage attribution

The ignored test-only phase helper ran 12 warmed q8 calls at exact geometry.
The timers are Rust stage boundaries; they are not directly matched DUCC
substage timings. DUCC values below are whole-transform measurements. Phase
medians need not sum exactly to the separately measured call median.

| Grid/order | Profile total ms | Latitude/spectrum setup ms | Coefficient-cache prep ms | Order scheduling + recurrence + spectrum ms | Two inverse rows + writeback ms |
|---|---:|---:|---:|---:|---:|
| CC T720 full, static | 40.740 | 0.605 | 1.822 | 37.012 | 1.209 |
| CC T720 full, queued | 22.781 | 0.625 | 1.857 | 18.994 | 1.205 |
| CC T720 `mmax=180`, static | 20.820 | 0.082 | 0.560 | 20.001 | 0.196 |
| CC T720 `mmax=180`, queued | 16.464 | 0.086 | 0.540 | 15.641 | 0.181 |
| F1 T720 full | 22.516 | 0.613 | 1.871 | 18.762 | 1.198 |
| F1 T720 `mmax=180` | 14.148 | 0.107 | 0.569 | 13.293 | 0.183 |

The inverse/writeback stage is only 1.2 ms full and below 0.2 ms reduced. The
queued CC recurrence/spectral stage alone remains 3.1× and 5.9× the respective
DUCC whole-call medians, so further output-FFT work cannot explain or close
the largest residual. The next useful investigation must target recurrence,
coefficient handling, or per-order scheduling/dataflow. Odd-length FFT work
remains deferred.

A current-source native F1 recheck also explains why the earlier F1 latency
screen did not match the phase probe. At source `1b832770`, q8 full F1 synthesis
measured 22.382 ms Rust / 5.884 ms DUCC (3.78×), and reduced order measured
14.338 / 2.545 ms (5.65×). Rust and DUCC runqueue-wait medians were both below
1%. The earlier F1 report measured 42.75 / 9.44 ms with 17–19% wait. Since
F1 execution was not changed by `ae3278a1`, that earlier high-wait run is not
evidence of a source regression; retain it as historical host-sensitive data.
The refreshed values and binary metadata are in `f1-current-control.json`.

## Reproduction and validation

Rust uses 1.99.1 / LLVM 23.1.1, release `-C target-cpu=x86-64-v3`, and Pulp
runtime V3. DUCC uses GCC 15.2, SHT `-O3 -ffast-math -ffp-contract=fast
-march=x86-64-v3`, and a strict `-fno-fast-math` driver. The host is an AMD
Ryzen 9 5950X; q1 uses CPU 0 and q8 CPUs 0–7 on one L3 domain.

The existing coefficient-cache A/B harness now accepts `--grid CC` and a
repeatable `--lmax`; sanitized summaries label CC correctly. For a baseline
worker built at the static source and a candidate worker at the production
source:

```sh
python3 scripts/benchmark_spin1_f1_coefficient_cache.py \
  --grid CC --lmax 720 \
  --baseline /path/to/static-worker \
  --candidate target/release/examples/ducc_spin1_full_transform_parity \
  --ducc /path/to/pinned-ducc-worker \
  --baseline-source 1b832770c4dc052deb48b65a62eff34bed3864f5 \
  --candidate-source ae3278a1bdd8e154d0bf2beb03b182193917578e \
  --process-pairs 3 --target-ms 25 --output cc-t720-raw.json
python3 scripts/summarize_spin1_f1_coefficient_cache.py cc-t720-raw.json \
  reports/data/plan15-spin1-cc-group-scheduling-2026-10-21
```

Use `--lmax 511` and a separate output directory for the lower-bandlimit gate
control. The q8 CC phase profile is the ignored
`spin_transform::tests::profile_spin1_standard_synthesis_phases` helper with
`RUSTSHT_PROFILE_GRID=CC`, `RUSTSHT_PROFILE_LMAX=720`, exact `NTHETA=722`,
`NPHI=1458` or `378`, `THREADS=8`, and CPU affinity 0–7. F1 uses `NTHETA=721`
with the same full/reduced longitude counts. The aggregate phase values and
sanitized distributions are in the linked data directory; raw process and
worker telemetry remains local and untracked.

Validation passed: targeted CC cache/fallback and F1 dynamic-scheduling tests,
`cargo test --workspace --all-features --release -- --test-threads=1`
(249 passed, 33 ignored), package no-default release tests (201 passed, 16
ignored), strict all-target/all-feature Clippy, formatting, and Python bytecode
compilation for the generalized benchmark harness.
