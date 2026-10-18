# Per-order coefficient-safety cache screen v2 protocol

This prospective screen isolates the cached safety decision from the frozen
tile-128 candidate. It does not accept the tile-128 synthesis kernel or change
production routing.

## Frozen builds

- Worktree source HEAD: `739275d20987fec35506916cb682f961e8a7ffaf` plus the existing isolated tile-128 integration edits.
- Pre-cache synthesis source: SHA-256 `0c0ee956205f782626e6ab0ffc6914cfa9f0d1e20caf5075111a53a9c9449b25`; executable SHA-256 `cf46dc4ce6bf4bf389f5e90b62d22e90b21c697e1f1189dda1bb0f6e43ddb26b`.
- Cached synthesis source: SHA-256 `da5fe000c53cc2d5a28aea4de79fc770709de1cde7d26671dec70aa1ecfc2c7e`; executable SHA-256 `c4dc9af2ab5ce97b6d6e837bed961c1c4af4527c71ee5851774e674765535bc5`.
- The binaries were rebuilt sequentially with the same Rust 1.99.0 release toolchain, build settings, remaining source files, and benchmark harness. The harness SHA-256 is `79d58ef6cd8c8b1e4c3a5c3cb54ff47b6cea0e8f9db78d1c5ea780b5c6153fc4`.

## Workload and acquisition

Run q1 scalar f64 GL `STANDARD` synthesis at T2047, full `mmax`, `phi0=0.37`,
2048x4096, using the same single allowed CPU for every process. Use six fresh
process pairs, alternating order: pre-cache/cache, cache/pre-cache, repeated
for pairs three through six. Each process performs one untimed transform and
five timed transforms. The command template is:

```sh
taskset -c "${RUSTSHT_CPUSET:?set one allowed CPU}" \
  <binary> 0 2047 1 synthesis GL STANDARD 2048 4096 2047 requested-only
```

Preflight and postflight CPU PSI `some avg10` above 2.00% invalidates that
process. Values from 0.50% through 2.00% are warnings and must be recorded.
Clear evidence of interference also invalidates the process. Each five-call
series must have population CV at most 5% and first-three/final-two median
drift at most 5%. Inspect affinity, worker placement, CPU frequency, context
switches, concurrent CPU-intensive processes, cgroup limits, and PSI before
continuing. On any invalid process, stop this screen. Do not retry, remove
samples, or replace a failed pair.

For a performance signal, calculate six paired cached/pre-cache median ratios.
Use 50,000 paired bootstrap resamples, seed `20261017`, the median per
resample, and linearly interpolated 2.5th/97.5th percentiles. A useful screen
requires a median ratio at most 0.97 and an interval upper bound below 1.00.
This q1 result is only evidence for the cache change; the tile-128 candidate
still requires its T511 control and the remaining q8 and regression matrix.

All raw host and process captures remain local and untracked. Commit only the
sanitized numerical results and validity decision.
