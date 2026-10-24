# Plan 16 Milestone D: order-major spin-1 synthesis dataflow

## Result

The order-major queue improves warmed q8 spin-1 STANDARD synthesis at T720 on
GL, CC, and F1, with full and reduced order and both even-smooth and
odd-minimum longitude sampling. It is now the production route when the
existing prepared-coefficient cache and dynamic group queue are eligible and
the order queue supplies at least two 8-order tiles per worker. q1, analysis,
derivative synthesis, f32, and cases without a prepared coefficient cache keep
their existing routes.

The change replaces latitude-group tasks with dynamically claimed tiles of
eight harmonic orders. Each task evaluates one order over all latitudes and
writes that order's column into the existing ring-major spectrum. It reuses the
prepared spin-1 coefficients and computes recurrence factors once per order,
instead of once per order in every ring group. The worker scratch holds two
latitude-length complex rows. At T720 q8 this is 184,832 bytes total, allocated
lazily on the first eligible synthesis and reused by the workspace. No
transform-sized order-major intermediate is created.

## Matched q8 whole-transform results

All rows use T720 GL/CC/F1, `phi0=0.23`, exact latitude counts, full
`mmax=720` or reduced `mmax=180`, and three interleaved Rust/DUCC process pairs.
Each warmed short-call result contains 30 calibrated batches targeting 25 ms.
The table shows median milliseconds; candidate p10/p90 are included in
brackets. Ratios below 1 would favor RustSHT.

| Grid | mmax | nphi | Sampling | Old Rust | New Rust [p10, p90] | DUCC | Rust/DUCC | New/old | Relative L2 error |
|---|---:|---:|---|---:|---:|---:|---:|---:|---:|
| GL | 180 | 378 | even-smooth | 13.819 | 6.580 [6.481, 6.695] | 2.542 | 2.585 | 0.476 | 5.2e-13 |
| GL | 180 | 361 | odd-minimum | 14.381 | 7.121 [7.015, 7.327] | 2.614 | 2.727 | 0.495 | 4.6e-13 |
| GL | 720 | 1458 | even-smooth | 19.509 | 15.199 [15.014, 16.075] | 5.903 | 2.578 | 0.779 | 3.9e-13 |
| GL | 720 | 1441 | odd-minimum | 21.476 | 17.454 [17.039, 18.107] | 8.253 | 2.106 | 0.813 | 3.8e-13 |
| CC | 180 | 378 | even-smooth | 17.664 | 7.209 [7.095, 7.927] | 2.659 | 2.715 | 0.408 | 3.6e-13 |
| CC | 180 | 361 | odd-minimum | 17.795 | 7.723 [7.579, 8.326] | 2.731 | 2.831 | 0.434 | 3.5e-13 |
| CC | 720 | 1458 | even-smooth | 23.982 | 15.779 [15.524, 16.722] | 6.142 | 2.573 | 0.658 | 2.3e-13 |
| CC | 720 | 1441 | odd-minimum | 25.743 | 17.722 [17.389, 19.445] | 8.845 | 2.029 | 0.688 | 2.3e-13 |
| F1 | 180 | 378 | even-smooth | 14.524 | 6.677 [6.591, 6.739] | 2.526 | 2.642 | 0.460 | 8.1e-13 |
| F1 | 180 | 361 | odd-minimum | 14.933 | 7.226 [7.119, 7.494] | 2.626 | 2.764 | 0.484 | 8.3e-13 |
| F1 | 720 | 1458 | even-smooth | 22.747 | 15.312 [15.146, 16.340] | 5.899 | 2.605 | 0.673 | 5.2e-13 |
| F1 | 720 | 1441 | odd-minimum | 24.799 | 17.173 [16.999, 17.358] | 8.221 | 2.084 | 0.692 | 4.7e-13 |

The candidate reduced Rust latency by 50–59% at `mmax=180` and 19–34% at
full order. The residual warmed q8 gap is 2.03–2.84x DUCC. Same-binary q1 and
q8-analysis controls showed no consistent shift: median changes were below
1% in most cases; the largest observed control change was 5.3%.

The q8 driver reported finite outputs, repeat consistency, output-reset
consistency, and zero failed comparisons for every benchmark case. The separate
release test compares q1 and q8 using one reused q8 workspace across regular,
sparse sectoral-tail, extreme finite coefficients, subnormal, and zero inputs.
It includes a `1.44e308` E/B case whose converted order is marked unsafe and
therefore exercises the checked recurrence fallback. A separate `1.4e308`
cancellation case has some unrepresentable map samples in both q1 and q8; the
test compares their finite results and per-sample infinity/NaN classifications
rather than incorrectly requiring overflowed values to remain finite. It also
verifies that caller map buffers are overwritten and that order scratch is
allocated only when the q8 route is selected.

## Stage attribution

These are eight-repetition Rust phase profiles for exact T720 CC q8. Phase
durations are wall milliseconds. Per-worker factor preparation is a summed
counter, not wall time.

| mmax | Phase | Group queue | Order-major |
|---:|---|---:|---:|
| 720 | Scheduling, Legendre recurrence, spectrum accumulation | 19.678 | 12.579 |
| 720 | Spin-1 scalar coefficient-cache preparation | 1.883 | 1.885 |
| 720 | Inverse longitude FFT and map writeback | 1.318 | 0.929 |
| 720 | Worker-summed recurrence-factor preparation | 13.518 | 1.330 |
| 720 | Worker slowest/fastest task ratio | 2.69 | 1.01 |
| 180 | Scheduling, Legendre recurrence, spectrum accumulation | 15.628 | 6.470 |
| 180 | Spin-1 scalar coefficient-cache preparation | 0.571 | 0.490 |
| 180 | Inverse longitude FFT and map writeback | 0.192 | 0.166 |
| 180 | Worker-summed recurrence-factor preparation | 6.434 | 0.578 |
| 180 | Worker slowest/fastest task ratio | 5.48 | 1.30 |

The measured gain comes from avoiding repeated recurrence-factor setup across
ring groups and keeping the workers busy as polar limits change the useful
latitude work. The paired E/B coefficient-cache preparation itself is nearly
unchanged. FFT time is a small part of the total and was not optimized.

## Current production confirmation

After replacing the temporary experiment switch with the work-based dispatch,
the actual production binary passed the same exact T720 CC whole-transform
comparison with zero failures. Its q8 synthesis medians were 7.154 ms / 7.744
ms for reduced even/odd and 15.646 ms / 18.526 ms for full even/odd, against
DUCC at 2.647 / 2.731 ms and 6.163 / 8.615 ms. Relative L2 errors were at
most 3.7e-13. First-call Rust times were 9.3–10.1 ms reduced and 23.5–28.2 ms
full; workspace setup was 5.1–5.9 ms and 13.2–14.6 ms respectively. The lazy
order scratch allocation is included in first-call time, not warmed latency.

## Reproduction and build identity

- Base branch commit for the controlled experiment: `a7ed9912`.
- Production confirmation base: `4bc972b9` (documentation-only fast-forward).
- DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- CPU: AMD Ryzen 9 5950X. q1 was pinned to one physical core; q8 to eight
  physical cores within one L3 domain. Frequency and scheduling telemetry was
  monitored; the reported sample distributions include observed short-run
  variability rather than relying on single-call timing.
- Rust: `rustc 1.99.1`, LLVM 23.1.1, release and
  `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.
- DUCC: GCC 15.2.0; SHT sources used
  `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the benchmark driver
  used `-fno-fast-math`.
- Current production Rust benchmark executable SHA-256:
  `641f2f53b76fbc6f5be73097bcfbf5f9f0acef7097333a6fca362b4e3678a439`.
- Pinned DUCC benchmark executable SHA-256:
  `e3908ae1ce2502b1a7b9c9ede1442d85d4f977efbb62adc39542482d2352f61d`.

The existing native harness builds and runs the Rust and pinned DUCC drivers.
For example, after building the DUCC driver from the pinned source:

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  cargo build --release --example ducc_spin1_full_transform_parity --features parallel
python3 reports/data/plan16-spin1-paired-coefficient-block-reuse-2026-10-22/run_exact_cc.py \
  "$PWD" --ducc-src /path/to/ducc-42ce \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --build-dir /path/to/ducc-build --skip-build \
  --results-dir results/order-major-cc --lmax 720 --grid CC \
  --process-pairs 3 --target-ms 25
```

Use `--grid GL` or `--grid F1` for the other measured grids. The controlled
off/on comparison used the same feature-enabled Rust binary and identical
fixtures; the temporary environment switch has been removed now that the
production work-based dispatch is in place.

## Next

Continue Plan 16 Milestone E: measure the T1023 spin-1 coefficient-cache
eligibility and, separately, CC/F1 analysis resampling overhead. The order-major
route is eligible only when that existing cache is ready, so T1023 remains on
the current group queue until its memory and cold/warm costs are measured.
