# Per-order coefficient-safety cache screen v2 result

## Decision

The isolated cache candidate is **not accepted**. Three complete paired
T2047 q1 comparisons favor the cache candidate by 3.72–4.35%, but the
pre-cache process in pair 4 failed the fixed timing-stability gate. The
predeclared six-pair screen stopped there. No retry, replacement pair,
bootstrap interval, q8 run, or regression matrix was collected. This result
does not accept the tile-128 kernel or change production routing.

The cache reuses the existing per-order coefficient-safety decision across
the order's 128-ring tiles. It keeps the same safety predicate and scaled
fallback. The change is isolated in the synthesis-fast worktree.

## Frozen build and workload

- Worktree source revision: `739275d20987fec35506916cb682f961e8a7ffaf` plus
  the isolated synthesis-fast integration edits.
- Pre-cache source SHA-256: `0c0ee956205f782626e6ab0ffc6914cfa9f0d1e20caf5075111a53a9c9449b25`.
- Pre-cache executable SHA-256:
  `cf46dc4ce6bf4bf389f5e90b62d22e90b21c697e1f1189dda1bb0f6e43ddb26b`.
- Cache source SHA-256: `da5fe000c53cc2d5a28aea4de79fc770709de1cde7d26671dec70aa1ecfc2c7e`.
- Cache executable SHA-256:
  `c4dc9af2ab5ce97b6d6e837bed961c1c4af4527c71ee5851774e674765535bc5`.
- Harness SHA-256:
  `79d58ef6cd8c8b1e4c3a5c3cb54ff47b6cea0e8f9db78d1c5ea780b5c6153fc4`.
- Both binaries were rebuilt sequentially with Rust 1.99.0, LLVM 23.1.1,
  Cargo release settings, and identical remaining source and harness files.
- Workload: scalar f64 GL `STANDARD` synthesis, T2047, `mmax=2047`,
  2048x4096, `phi0=0.37`, one worker pinned to one allowed CPU. Each process
  performed one untimed transform and five timed full transformations.
- Host: Ubuntu 26.04.1 LTS, x86-64 AMD Ryzen 9 5950X, 16 physical and 32
  logical cores, AVX2/FMA, 96 GiB RAM.

The acquisition criteria and command template were frozen in the
[v2 protocol](coefficient-safety-cache-screen-v2-protocol.md) before these
measurements.

## Process measurements

Each row retains the complete five-call timing series. PSI columns report the
preflight reading, postflight reading, and maximum sampled during-run reading.
Values from 0.50% through 2.00% are warnings; values above 2.00% fail the
pressure gate. Peak resident memory is the per-process high-water mark.

| Pair | Variant | Timing samples (ms) | Median (ms) | CV | First-three/final-two drift | Peak memory (KiB) | Stability |
| --- | --- | --- | ---: | ---: | ---: | ---: | --- |
| 1 | Pre-cache | 1753.758, 1747.917, 1745.941, 1748.240, 1748.909 | 1748.240 | 0.1485% | +0.0376% | 332192 | Pass |
| 1 | Cache | 1674.542, 1676.829, 1675.063, 1678.672, 1683.044 | 1676.829 | 0.1831% | +0.3460% | 332212 | Pass |
| 2 | Cache | 1676.787, 1674.976, 1674.654, 1678.537, 1691.660 | 1676.787 | 0.3766% | +0.6043% | 332172 | Pass |
| 2 | Pre-cache | 1756.564, 1752.800, 1752.677, 1752.955, 1757.237 | 1752.955 | 0.1150% | +0.1310% | 332252 | Pass |
| 3 | Pre-cache | 1745.779, 1743.983, 1742.703, 1747.717, 1751.445 | 1745.779 | 0.1757% | +0.3210% | 332192 | Pass |
| 3 | Cache | 1679.966, 1677.575, 1680.900, 1681.088, 1681.488 | 1680.900 | 0.0837% | +0.0787% | 332200 | Pass |
| 4 | Cache | 1675.970, 1673.767, 1674.481, 1675.998, 1677.090 | 1675.970 | 0.0708% | +0.1232% | 332188 | Pass |
| 4 | Pre-cache | 1888.923, 2090.918, 1777.279, 1744.555, 1754.335 | 1777.279 | 7.0477% | -7.3840% | 332196 | **Fail** |

CPU-pressure warnings were reviewed under the declared protocol; exact transient readings remain local. The local interference review did not identify a competing workload or cgroup-related constraint. Frequency behavior was reviewed locally but cannot be aligned with individual transform calls; exact telemetry remains untracked, so interference cannot be ruled out. The failed baseline process exceeds both 5% stability limits; its
samples remain in the table and were not retried or removed.

Peak memory ranged from 332172 to 332252 KiB across all eight processes. The
paired cache/pre-cache median ratios for the three completed pairs were
0.95915, 0.95655, and 0.96284, corresponding to descriptive reductions of
4.08%, 4.35%, and 3.72%. The protocol requires six complete pairs and a
bootstrap interval, so these three ratios are not an acceptance result.

This screen measured complete synthesis latency only. It did not collect
route-resolved prefix/suffix timings, numerical comparisons, or spin-1,
T511, T1023, q8, analysis, or workspace regression results. The separate
route diagnostics still identify scaled-prefix work as the largest measured
Legendre component; the [M6 report](../../../m6-cpu-performance-parity.md)
retains those measurements and the failed control status. Raw host, process,
frequency, and PSI captures remain local and untracked.
