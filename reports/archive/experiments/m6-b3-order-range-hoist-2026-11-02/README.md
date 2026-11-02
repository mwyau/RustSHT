# B3 order-range hoist — rejected

This isolated f64 experiment tested whether B3 spin-1 analysis could reuse the
spin-1 fast-ring interval already computed by the outer transform, instead of
scanning `theta_sin` inside every order-group invocation. It changed no
recurrence, coefficient arithmetic, FFT, or output operation. **No production
code changed and no optimization was accepted.**

## Source and route

- Rust parent: `3e4bfa9128155ca263a82360c77a3fcc7eba7fca` (`dev` HEAD at the
  time); the isolated worktree had no parent-source edits.
- DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0).
- Executed route: f64 GL spin-1 analysis, full M720/reduced M180, q1/q8.
- CPU: AMD Ryzen 9 5950X; q1 pinned to CPU 0, q8 to CPUs 0–7.
- Rust: 1.99.1 / LLVM 23.1.1, release + `parallel`,
  `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.
- DUCC: `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; driver adds
  `-fno-fast-math`.
- Parent binary SHA-256:
  `121e72638ea3038a0ab7db0d7ea6a08c81d405a37ac7df96986707786ec31ee5`.
- V1 binary SHA-256:
  `70c7543ea61e1b22b6f3e0ebde5decf6bef0aed65ca5f9f3f77239f3933298c4`.
- V2 binary SHA-256:
  `18895237ffac4a64889c71b1771ba41730f168d9f014f9916893e76f87384adb`.
- DUCC worker binary SHA-256:
  `9839f8bc4c9409f4d89996d47d8b859395d75b337e45137025a7fe4b8dac0659`.
- GL fixture hashes are recorded per M720/M180 in `screen-summary.json`.

In the parent, `analyze_spin_order_group` finds the interval at
`src/transform/spin/analysis.rs:74–93` using two `position` scans. For spin-1,
the outer transform already computes the same geometry at
`src/transform/spin.rs:626–640`; that interval also keys the pair-safety
workspace cache. The hypothesis was to pass the known interval through the
q1 and dynamic q8 order scheduling. The first patch used `Option::or_else`;
after disassembly showed its extra helper path, V2 used an explicit `match`.

## Measurements

Each cell has 15 interleaved parent/candidate/DUCC process pairs. Ratios below
are paired geometric means; intervals are 95% bootstrap intervals over paired
log ratios (50,000 resamples). Every parent/candidate E/B output was bitwise
equal across all 60 cell pairs.

| GL T720 B3 f64 | Parent ms | Candidate ms | DUCC ms | Candidate / parent (95% CI) | Candidate / DUCC (95% CI) | Result |
|---|---:|---:|---:|---:|---:|---|
| Full q1 | 32.943 | 33.092 | 29.684 | 0.999 [0.990, 1.009] | 1.112 [1.104, 1.121] | INCONCLUSIVE / GAP |
| Full q8 | 4.923 | 4.918 | 4.309 | 0.996 [0.981, 1.013] | 1.147 [1.134, 1.164] | INCONCLUSIVE / GAP |
| Reduced q1 | 12.903 | 12.885 | 11.785 | 1.004 [0.996, 1.016] | 1.098 [1.090, 1.111] | INCONCLUSIVE / GAP |
| Reduced q8 | 1.962 | 1.975 | 1.762 | 1.014 [0.991, 1.041] | 1.126 [1.099, 1.160] | INCONCLUSIVE / GAP |

V1's three-pair screen was mixed and inconclusive. V2's 15-pair results show
no reliable parent-relative gain in any cell; all candidate/parent intervals
cross 1.0. The candidate remains 9.8–14.7% slower than DUCC. Thus, removing
the scan is not a useful next optimization on this source. It does not change
the stop condition: do not propose another B3 kernel variant until PMU or
equivalent cycle attribution identifies a material cost.

An isolated scan microbenchmark measured about 159.6 ns for the two searches
(about 115 us if repeated serially for 721 orders). A q8 scheduling probe
estimated 13.4 us wall time for the repeated scan, about 0.3% of full-q8 and
0.75% of reduced-q8 whole-call time. Those are diagnostic estimates, not
disjoint production-stage timings. They correctly suggested a small ceiling;
the whole-call screen did not resolve a gain.

## Code generation and test evidence

V1's `Option::or_else` form added the out-of-line option/closure call path to
the order-group code. V2 removed that helper path but retained an explicit
branch. Parent/V2 binary sizes were 3,135,684/3,135,704 bytes total; `.text`
grew from 3,029,636 to 3,030,296 bytes and `.data` from 102,280 to 102,304
bytes. `analyze_spin_order_group` grew from 18,791 to 18,958 bytes. The active
V3 vector leaf and spin dispatch helper were unchanged (8bd and 141d bytes,
respectively). This adds code around each order job without improving the
whole transform.

The explicit-match V2 passed these focused release tests:

- `spin1_gl_paired_analysis_matches_full_latitude_recurrence`
- `spin1_reflected_analysis_subranges` (both subtests)
- `spin1_analysis_reuses_poisoned_workspace_for_extremes_on_all_grids`

The 15-pair run used the existing DUCC worker from the profile worktree. Its
driver has Callgrind marker support and performs one untimed warm call before
the timed batches; the DUCC SHT kernel itself was built without Callgrind
instrumentation. Treat candidate/DUCC times as warm-worker measurements with
that harness caveat. Host telemetry was noisy (q8 frequency samples ranged
roughly 1.75–4.88 GHz and involuntary context switches were numerous), so the
parent-relative intervals crossing 1.0 must remain INCONCLUSIVE. No samples
were discarded.

## Recovery

- `range-hoist-v1.patch`, `range-hoist-v2.patch`: isolated candidate changes.
- `screen-summary.json`: sanitized paired ratios, timing medians, confidence
  intervals, hashes, and the three-pair screen ratios. Full per-run captures
  and host telemetry remain local in the isolated experiment worktree.
- `range_hoist_screen.py`: 15-pair worker harness. Set `M6_B3_FIXTURES`,
  `M6_B3_DUCC_INPUTS`, `M6_B3_PARENT_BIN`, `M6_B3_CANDIDATE_BIN`, and
  `M6_B3_DUCC_BIN`; optionally set `M6_B3_RESULTS_PATH` for its output JSON.
- SHA-256 checksums for all package files: `SHA256SUMS`.

Do not cherry-pick either patch. Keep production B3 unchanged pending cycle
attribution.
