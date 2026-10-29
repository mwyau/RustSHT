# Current-source reflected f32 scalar-synthesis candidate

**Status: quantified rejection; no production change.** The candidate removes a
large part of the native-f32 scalar-synthesis gap, but it has repeatable
adjacent regressions in scalar analysis and spin-1 synthesis. Preserve the
patch and results for a later synthesis-only repair; continue to Stage B using
the shared f64 algorithms behind the existing f32 public contract.

## Source and implementation

- Exact parent: `1b2041491e621fffe94464667619baf788d31835` (`dev`, also
  `origin/dev` before this evidence checkpoint).
- Pinned DUCC: 0.42.0,
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- The candidate adds a GL T720 f32 scalar-synthesis dispatch into a reflected
  order traversal. It reuses `FusedSynthesisBlock`, the paired recurrence
  visitor and `F32SynthesisAccumulator`; it does not add a second maintained
  recurrence. The currently accepted scalar-analysis SIMD implementation is
  still present in the candidate source.
- Complete current-parent patch SHA-256:
  `9027f1818cc145d162053d74cf9400bae2478ec230ab362546bbcd7e79c22fc9`.
  The method-order triage variant patch SHA-256 is
  `93d0e7fcc57360705071b4af039e0592dcfdebebffc489f6d4014e1ce895f285`.
  Both compressed patches are preserved beside this report.
- The benchmark runner recorded SHA
  `daa10ddb3deb602736a2e447f8d6d554a816d3c69af91ca955d70436fc61e40f`
  for its `git diff` output, which omitted the then-untracked new Rust module.
  The complete patch above was reconstructed directly from the exact
  benchmark worktree, including that module; binary hashes below identify the
  measured source build.

## Measurement setup

The whole-call workers use the native f32 RustSHT API and native f32 DUCC
interface. GL T720 used `ntheta=721`, `phi0=0.23`, `nphi=1458` at full
`mmax=720`, and `nphi=378` at reduced `mmax=180`. The four target synthesis
cells each have 15 interleaved, matched parent/candidate/DUCC process pairs.
The workers warmed reusable plans/workspaces and timed repeated complete
transforms; setup was outside the timed batches. q1 was pinned to CPU 0; q8
was pinned to CPUs 0,2,4,6,8,10,12,14. Each timed batch targeted 25 ms. Paired
ratios are medians of matched per-pair ratios; the synthesis 95% intervals use
20,000 bootstrap resamples over those pairs.

The host was an AMD Ryzen 9 5950X. Rust used rustc 1.99.1 / LLVM 23.1.1 with
`-C target-cpu=x86-64-v3`; the DUCC C++ numerical sources used GCC 15.2.0 with
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, and the driver used
`-fno-fast-math`.

## GL T720 scalar-synthesis results

Times are medians in milliseconds. C/P is candidate/parent; C/D is
candidate/DUCC. All 60 candidate-parent pairs favored the candidate, but every
cell remains far from parity.

| Cell | Parent / candidate / DUCC | C/P paired median (95% CI) | C/D | Candidate faster | Status |
|---|---:|---:|---:|---:|---|
| Full q1 | 640.796 / 355.900 / 18.954 | 0.5550 [0.5536, 0.5585] | 18.75x | 15/15 | GAP |
| Reduced q1 | 190.142 / 116.132 / 8.359 | 0.6106 [0.6091, 0.6164] | 13.92x | 15/15 | GAP |
| Full q8 | 97.884 / 46.869 / 2.933 | 0.4798 [0.4759, 0.4876] | 16.11x | 15/15 | GAP |
| Reduced q8 | 51.740 / 19.124 / 1.282 | 0.3615 [0.3498, 0.3693] | 14.55x | 15/15 | GAP |

Candidate and parent outputs differed by at most `1.14e-7` relative L2 and
`2.09e-7` absolute on these fixtures. Maximum candidate-to-DUCC errors were
`2.73e-4` relative L2 and `1.55e-3` absolute (reduced mmax). The independent
analytic and adjoint tests listed below also passed; these random-fixture
comparisons alone are not an accuracy proof.

## Protected and adjacent controls

The 15-pair f32 scalar-analysis extensions and spin-1 q8 synthesis control
showed repeatable regressions. Parent/candidate/DUCC are median milliseconds.
The 95% intervals shown for controls were recomputed from the stored paired
ratios using 20,000 bootstrap resamples.

| Control | Parent / candidate / DUCC | C/P (95% CI) | Candidate-faster pairs | Result |
|---|---:|---:|---:|---|
| f32 scalar analysis, full q1 | 454.060 / 460.762 / 14.044 | 1.0124 [1.0100, 1.0147] | 2/15 | Regression |
| f32 scalar analysis, full q8 | 59.808 / 60.388 / 2.269 | 1.0097 [1.0087, 1.0114] | 0/15 | Regression |
| f32 scalar analysis, reduced q1 | 140.191 / 141.102 / 5.795 | 1.0050 [1.0008, 1.0113] | 2/15 | Regression |
| f32 scalar analysis, reduced q8 | 19.835 / 19.923 / 1.009 | 1.0073 | 5/15 | Inconclusive/slower median |
| f32 spin-1 synthesis, full q8 | 272.906 / 280.901 / 5.939 | 1.0243 [1.0067, 1.0377] | 3/15 | Regression |
| f32 scalar synthesis, T1023 q8 | 401.595 / 404.553 / 6.990 | 1.0082 [0.9913, 1.0248] | 6/15 | Inconclusive |

Three-pair spot checks found no regression in f32 spin-1 analysis q1/q8
(candidate/parent `0.9863` / `0.9790`) or full spin-1 synthesis q1
(`1.0042`, inconclusive). A three-pair T511 scalar-synthesis q8 screen was
faster (`0.9694` candidate/parent), but is not broad enough to establish an
adjacent-bandlimit gain.

Protected f64 GL T1023 full-mmax spin-1 synthesis was measured for 15 pairs at
each q. q1 parent/candidate/DUCC was `89.613/88.140/119.739 ms`, C/P
`0.9874 [0.9779, 0.9910]`, with 15/15 candidate-faster pairs. q8 was
`17.225/17.042/19.937 ms`, C/P `1.0041 [0.9382, 1.0418]`, 7/15 faster.
Both candidate outputs were bitwise equal to parent and candidate/DUCC
relative-L2 was below `2.61e-13`; repeated-call and workspace-reset checks
passed. The f64 q8 aggregate's `all_worker_checks` flag is false only because
that driver reports `finite_outputs` as null; independent output metrics were
finite, and repeat/reset checks passed.

A method-order-only variant was screened for three pairs per cell. Moving the
new f32 trait overrides did not remove the full-mmax scalar-analysis slowdown
(C/P `1.0137` q1, `1.0112` q8); no 15-pair acceptance run was justified.

## Correctness checks actually run

On the complete candidate patch:

- `cargo fmt --check --all` passed.
- `cargo test --locked --release --features parallel --test f32 f32_t720_ -- --nocapture`
  passed 3 tests covering f32 scalar synthesis analytic modes, adjoints,
  sectoral behavior, SIMD tails, exceptional declines and workspace reset,
  plus scalar-analysis analytical controls. Maximum printed synthesis
  analytic relative L2 was `1.51e-7`; adjoint relative error was at most
  `7.32e-6`.
- `cargo test --locked --release --features parallel --lib scalar_paired::synthesis::tests`
  passed 5 tests covering scaled packet references, subnormal/out-of-range
  decline, mixed exponents and partial SIMD tails.
- All benchmark workers reported finite output, repeat consistency and
  poisoned-workspace reset for the f32 screens. Parent and candidate outputs
  were bitwise equal in all adjacent f32 controls and in the protected f64
  control.

This did not run the full GL/CC/F1, higher-spin, nonfinite and extreme-value
matrix. The candidate remains an isolated experiment and is not production
code.

## Decision and next work

Do not promote this patch unchanged. The target improvement is material, but
the full-mmax scalar-analysis slowdowns repeat across q1/q8 and the spin-1
synthesis q8 slowdown is independently repeatable. The method-order variant
did not resolve the effects. The remaining 13.92x–18.75x scalar-synthesis gap
also fails the intended performance objective.

Proceed to Stage B: benchmark the existing shared f64 transform algorithms
behind f32 inputs/outputs with conversion, FFT, transform, writeback, reusable
plan/workspace and allocation costs included. Start with f32 scalar synthesis
at GL T720 full/reduced q1/q8 against the native f32 DUCC interface; then
extend only if the complete public-call result supports it.

## Evidence files

- `candidate-full.patch.gz` and `candidate-v2-full.patch.gz`: complete
  recoverable source diffs, including the new reflected module.
- `raw/stage-a-core-15-pairs.json.gz`: all primary paired synthesis timings,
  errors, checks, fixture hashes, build hashes and benchmark settings.
- `raw/f32-controls-initial.json.gz` and
  `raw/f32-controls-15-pair-extensions.json.gz`: adjacent f32 controls.
- `raw/f64-protected-q1-results.json.gz` and
  `raw/f64-protected-q8-results.json.gz`: protected f64 control pairs.
- `raw/method-order-v2-triage.json.gz`: three-pair method-order triage.
- `runners/`: exact measurement runners; `ducc-f32-worker/` and
  `ducc-f64-worker/`: benchmark driver and CMake sources.
- `raw/test-*.log.gz`: focused test outputs.

Binary SHA-256: Rust parent f32
`7552d0705709309a15988c78627a6e0d6a54edce4161085b13ba2dc3dfa41bb4`,
Rust candidate f32
`54f411d71ba41afd8acd9667b7868e6c9dfb8b014bf1b03d16404b6d913d9614`, native
DUCC f32 `acc82387af2c42aed53b25678e3872157ed3fb6e5d5874322badc472a1d35d5d`;
Rust parent/candidate f64 spin-1
`43a1fab83fc3257bcbbefc8f6b71f40ef52eea9d17f7ad8cedd86c7d4c4c48b0` /
`180114eed6cbd99f1f75bafbc93a6ca299d5790bf29e420f5034c195a9373759`, and
native DUCC f64 `ee3ea51c2eaeb208c2c52ee9ee3e2c8f999993e8133c22f15a2f082e5a63fc06`.
