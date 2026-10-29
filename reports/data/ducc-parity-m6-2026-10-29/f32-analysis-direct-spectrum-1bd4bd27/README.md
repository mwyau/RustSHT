# Direct f32 spectra into the shared f64 scalar-analysis recurrence

## Decision

Keep this test-only route isolated. It removes the complete f32-spectrum to
f64-spectrum promotion buffer and is substantially faster than the promoted
route at q8, but the 15-pair current-source screen still misses DUCC in both
q1 cells and full q8. Reduced q8 is near parity but its 95% interval crosses
1.0. The f32 public API still has no guarded production analysis dispatch for
this candidate, so this is not a production precision-policy change.

The exact Rust parent is `1bd4bd274cacda38f87cc741c2d4f20342107b8f`.
`parent.patch.gz` adds the earlier test-only f32-FFT/f64-recurrence harness;
`candidate.patch.gz` adds the direct f32-spectrum view to that harness. The
candidate keeps the existing shared f64 scalar recurrence and accumulation.
Each complete timed call includes the f32 longitude FFT, per-value f32 to f64
widening as spectra are gathered, the f64 recurrence and accumulation, and
Complex64-to-Complex32 output narrowing. It does not materialize a converted
f64 spectrum. The test harness still constructs a reusable f64 plan and
workspace; the f64 workspace spectrum buffer is allocated but unused on the
direct route.

## T720 public-call comparison

Times are medians in milliseconds across 15 interleaved comparisons per cell.
The Rust parent is the f32-FFT/f64-recurrence route with a reusable promoted
spectrum buffer; DUCC uses its pinned native-f32 API. Intervals are 50,000
bootstrap resamples of paired ratios.

| GL T720 scalar analysis | Parent / candidate / DUCC ms | Candidate/parent median [95% CI] | Candidate/DUCC median [95% CI] | Accuracy and status |
|---|---:|---:|---:|---|
| Reduced q1 | 6.247 / 6.249 / 5.861 | 1.0003 [0.9983, 1.0070] | 1.0696 [1.0608, 1.0787] | GAP |
| Reduced q8 | 0.965 / 0.871 / 0.889 | 0.9039 [0.8948, 0.9184] | 0.9897 [0.9769, 1.0020] | INCONCLUSIVE at parity |
| Full q1 | 15.648 / 15.650 / 14.530 | 0.9815 [0.9598, 1.0191] | 1.0771 [1.0568, 1.0915] | GAP |
| Full q8 | 2.429 / 2.117 / 2.052 | 0.8737 [0.8699, 0.8817] | 1.0319 [1.0214, 1.0443] | GAP |

The direct route is 9.6% faster than its promoted-spectrum parent in reduced
q8 and 12.6% faster in full q8 by paired median. The q1 parent deltas are
inconclusive and do not close either DUCC gap. Candidate and parent outputs
were bitwise identical. Against DUCC, relative-L2 / maximum absolute error was
`1.319e-7 / 1.397e-9` reduced and `1.432e-7 / 6.985e-10` full. All workers
reported finite output and consistent repeated-call and poisoned-workspace
reset results.

The three-pair adjacent native-f32 analysis screen found no clear regression.
The full-q1 control was extended to 15 pairs: candidate/parent median `1.0010`
with bootstrap interval `[0.9974, 1.0104]` and 7/15 faster pairs. Reduced q1,
reduced q8, and full q8 remained three-pair controls with candidate/parent
medians `1.0015`, `1.0044`, and `0.9992`. Outputs were bitwise identical in
all controls. The protected f64 T1023 spin-1 synthesis control was not retimed;
no candidate change has been applied to the production checkout.

## Stage profile and resource limits

The 30-call instrumented profile confirms the direct route has no spectrum
promotion stage. Full q8 stage medians were 0.408 ms f32 FFT, 1.765 ms shared
f64 recurrence and gather, and 0.087 ms output narrowing. Reduced q8 measured
0.196, 0.872, and 0.038 ms respectively. The recurrence and gather therefore
account for roughly 77–79% of each instrumented whole call. The instrumented
whole-call medians did not match the paired benchmark medians closely for
reduced q8 (1.110 ms versus 0.871 ms) or full q8 (2.263 ms versus 2.117 ms);
these stage timings are diagnostic only and do not support a separate timing
claim. The earlier exact-source Callgrind analysis found the f64 recurrence
tree was the dominant cost and identified the AVX2 fused scaled-analysis leaf;
instruction references are not cycles.

The instrumented worker's high-water RSS was 31,684 KiB reduced and 84,428 KiB
full q8. This includes the process, both plans and both reusable workspaces;
it is not a reusable-workspace byte count. No allocator interposition was
used. The direct route adds no persistent spectrum buffer, but its test setup
still reserves the unused f64 workspace spectrum. A production adapter would
need an explicit workspace-memory and allocation audit before acceptance.

## Provenance

- Rust source parent: `1bd4bd274cacda38f87cc741c2d4f20342107b8f`.
- Parent patch SHA-256: `7b796de7b4192a096b8d77c21a2b638306a5b2ae1a15455d973a9ae8b6a06264`.
- Candidate patch SHA-256: `575fa7c157a58d105f29311ccff7f72749496a72909cdb4c4ff16553854c08af`.
- Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust: rustc 1.99.1 / LLVM 23.1.1, `--locked --release --features parallel`,
  `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.
- DUCC: GCC 15.2, SHT `-O3 -ffast-math -ffp-contract=fast
  -march=x86-64-v3`; driver `-fno-fast-math`.
- Host: AMD Ryzen 9 5950X, AVX2/FMA. The runner pins q1 and q8 affinity by
  setting the process affinity to `range(threads)`; exact policy is preserved
  in the runner.
- Grid: GL, `ntheta=721`, `phi0=0.23`; full `mmax=720,nphi=1458`, reduced
  `mmax=180,nphi=378`.
- Each Rust worker creates its plan/workspaces once, warms twice, then takes
  seven timed calls per paired comparison. The DUCC worker uses its matching
  warm-call worker with a 25 ms timed batch. Setup stays outside the timed
  region for both.
- Fixture files are deterministically generated by the repository's f32
  transform fixture generator. Their exact hashes and sizes are in
  `fixture-sha256.json` and `fixture-manifest.json`.
- Rust and DUCC executable hashes are in `binary-hashes.txt`; paired timings,
  ratios and output hashes are compressed in `paired-results.json.gz` and
  `native-controls.json.gz`. Runner snapshots and checksums are included.

## Validation and next experiment

`cargo fmt --all -- --check` and `git diff --check` passed in the isolated
candidate. The direct-route worker compiled and passed finite-output,
bitwise-parent-equivalence, repeat-call, and poisoned-workspace-reset checks.
Independent analytic modes, adjoints, pole/sectoral controls, exceptional
values, SIMD tails, and smaller/larger bandlimits were not run because this
candidate misses the performance gate and remains test-only.

The next bounded hypothesis widens the shared scalar-analysis latitude tile
from 256 to 512 for the T720 analysis diagnostic. Both full and reduced GL
T720 currently use two reflected blocks per order (`ceil(361/256)`); a 512
tile would cover the 361 northern/equatorial pair lanes in one block, closer
to DUCC's single per-order hemisphere traversal. This costs at most 112 KiB
of additional inline recurrence scratch for eight f64 workers. Measure this
with the same direct-spectrum route and compare the whole four-cell matrix,
native-f32 analysis controls, and protected f64 T1023 spin-1 synthesis before
considering any production use.
