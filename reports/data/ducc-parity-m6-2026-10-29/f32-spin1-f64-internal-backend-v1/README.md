# Reusable f32-I/O/f64-internal spin-1 synthesis route

**Decision: reject as a family-wide production route.** The isolated backend
dramatically reduces the native-f32 RustSHT cost, and full-mmax q1 reaches
DUCC parity in a 15-pair run. The other three required T720 synthesis cells
remain 16–65% slower than native-f32 DUCC. Keep the exact patch for follow-up;
do not dispatch only the passing cell while the rest of the spin-1 family still
uses a route roughly 35–50× slower than DUCC.

## Source and benchmark contract

- Exact source parent: `f66ff4ed571374c664885f054f31711000075805`.
- Candidate: source-only patch in [`candidate.patch.gz`](candidate.patch.gz),
  whose uncompressed SHA-256 is
  `975523e6ce2616192a3fa8ae934d5a730588e10affc8c54d4a696611b6bd30dc`.
- Pinned DUCC: 0.42.0, commit
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host: AMD Ryzen 9 5950X, AVX2/FMA, Pulp V3. Rust 1.99.1 / LLVM 23.1.1,
  `--release --features parallel`, `-C target-cpu=x86-64-v3`.
  DUCC used GCC 15.2.0 with `-O3 -ffast-math -ffp-contract=fast
  -march=x86-64-v3`; the f32 driver used `-fno-fast-math`.
- Parent/candidate/DUCC f32 benchmark binary SHA-256 values are recorded in
  [`experiment-manifest.json`](experiment-manifest.json).
- The matched whole-call harness is the committed
  [benchmark runner](../f32-spin1-native-f32-reflected-leaf-v1/harness/benchmark.py)
  (SHA-256 `6cdc210de9825deb818e7ae1128c7fa3300f306745d8aac06b130201c6f51e56`)
  and its pinned-DUCC [CMake source](../f32-spin1-native-f32-reflected-leaf-v1/harness/CMakeLists.txt).
  Its deterministic fixtures use `phi0=0.23`; input and output hashes for
  every pair are retained in [`paired-results-sanitized.json`](paired-results-sanitized.json).
- Each timed call used the public f32 spin-1 synthesis API. The f32 and child
  f64 plan/workspace setup was outside the warm timing; coefficient promotion,
  f64 transform, FFT, map narrowing, and output writeback were inside it. q1
  used a fixed one-CPU affinity and q8 eight distinct physical cores; the exact
  pin sets are in the harness. Parent, candidate and native-f32 DUCC ran in
  interleaved order.
- The three clearly failing cells retained the three-pair screen. Full q1 was
  extended to 15 pairs because it was near parity. Ratios below are medians of
  matched pairs; three-pair ranges are observed ranges, not confidence
  intervals.

## Whole-transform results

Times are milliseconds. `C/P` and `C/D` are paired candidate/parent and
candidate/DUCC ratios.

| GL T720 spin-1 synthesis | Parent / candidate / DUCC ms | C/P paired median [range] | C/D paired median [range or 95% CI] | Status |
|---|---:|---:|---:|---|
| Full M720 q1 (15 pairs) | 1482.970 / 39.387 / 40.484 | 0.02667 [0.02623, 0.02697] | 0.97324 [0.96431, 0.97896] | PASS for this cell |
| Reduced M180 q1 (3 pairs) | 597.185 / 29.108 / 17.604 | 0.04873 [0.04800, 0.05096] | 1.65355 [1.61391, 1.73898] | GAP |
| Full M720 q8 (3 pairs) | 269.417 / 8.081 / 6.084 | 0.03027 [0.02866, 0.03033] | 1.32837 [1.15092, 1.38640] | GAP |
| Reduced M180 q8 (3 pairs) | 121.569 / 3.073 / 2.638 | 0.02519 [0.02499, 0.03159] | 1.16123 [1.15605, 1.28406] | GAP |

All workers passed finite-output, repeat-consistency and poisoned-workspace
reset checks. Candidate-to-DUCC random-fixture relative-L2 error was at most
`3.91e-8`, with maximum absolute error `1.20e-7`. The isolated focused test
`f32_t720_internal_spin1_synthesis_uses_shared_f64_plan_and_guards_ranges`
passed (1 passed, 0 failed). It verifies reuse of the f64 plan, exact equality
with the narrowed f64 child result, public output reset, and guarded decline
for f32 min-subnormal, min-normal inputs that would produce subnormal outputs,
max finite, and infinity. The candidate also passed `cargo check --release
--features parallel` and `cargo fmt --check`.

The passing full-q1 cell does not justify family dispatch: reduced q1 is the
largest measured miss at `1.654×` DUCC. No independent analytic/adjoint,
pole, or broader bandlimit suite was run for this candidate after the
family-level performance gate failed. Those checks remain required before any
narrower future production use. The protected f64 result is reported below.

## Protected f64 control

The GL T1023 full-mmax f64 spin-1 synthesis route bypasses the new f32 sidecar.
Candidate and parent outputs were bitwise equal in both controls. q1 used three
matched pairs and was neutral (`1.0013×`, observed range `[0.9956, 1.0130]`).
q8 used 15 pairs because the earlier timings were variable; its paired median
was `1.0009×`, with bootstrap 95% interval `[0.9841, 1.0715]` and seven of 15
pairs faster. This is inconclusive for a small timing change, with no
repeatable regression. The candidate remained faster than DUCC at both q
settings (`0.735×` q1, `0.790×` q8 median candidate/DUCC ratios). Repeat and
workspace-reset checks passed. The legacy worker did not populate its
`finite_outputs` field; independent candidate/DUCC comparisons were finite,
with relative-L2 error at most `2.61e-13`. Exact fixture and binary hashes and
all pair results are in
[`protected-f64-paired-results.json`](protected-f64-paired-results.json).

## Reusable workspace and allocations

Caller-owned buffers are excluded. The child f64 plan/workspace and reusable
conversion buffers are included. Warm transforms made zero allocations,
deallocations or reallocations. The q8 pool's first warmed call grew the
workspace by about 185 KB; later calls had no growth. The f32-to-f64 and
f64-to-f32 buffers account for 25,148,480 bytes at full mmax and 8,015,360
bytes at reduced mmax.

| Cell | Parent live bytes | Candidate live bytes | Additional reusable bytes |
|---|---:|---:|---:|
| Full q1 | 8,539,826 | 50,814,186 | 42,274,360 |
| Reduced q1 | 2,275,634 | 14,897,570 | 12,621,936 |
| Full q8 | 9,112,638 | 55,114,758 | 46,002,120 |
| Reduced q8 | 2,784,342 | 17,312,886 | 14,528,544 |

Exact workspace/allocation measurements are in [`allocation.csv`](allocation.csv);
the temporary counting-allocator program is retained as
[`allocation-harness.rs`](allocation-harness.rs).

## Cause evidence for the remaining gaps

The q8 full-mmax public-call Callgrind run covered seven transforms. Rust used
5.420B instruction references (774.3M per transform) versus DUCC's 4.209B
(601.3M per transform), or 1.288×. Instruction references are not cycles.
Rust's exclusive `synthesize_prepared_component_batch_dispatched` accounted
for 2.007B references (37.0%); DUCC's exclusive `alm2map_kernel` accounted for
2.418B (57.5%). Thus Rust's hot recurrence kernel alone had fewer references
than DUCC's kernel, while references outside that kernel were about 1.62B
higher. The remaining gap cannot be attributed to the fused recurrence alone.
Both binaries executed AVX2 YMM FMA code; selected actual function disassembly
is preserved in [`rust-candidate-hot-functions.asm.gz`](rust-candidate-hot-functions.asm.gz)
and [`ducc-hot-functions.asm.gz`](ducc-hot-functions.asm.gz). Raw Callgrind
files are compressed beside them.

Separate direct-f64 phase instrumentation (not public f32 whole-call timing)
puts order scheduling, Legendre recurrence and accumulation at about 5.04 ms
of a 6.98 ms full-q8 call, with latitude/spectrum setup and two FFT/map writes
about 0.93 ms each. Reduced q1 spends about 26.84 ms of a 27.81 ms call in
recurrence and accumulation; reduced q8 spends about 2.37 ms of 2.85 ms there.
The full phase logs and source-specific scope are retained as compressed files.

The route uses existing f64 spin-1 transform algorithms and keeps f32 as the
public input/output precision. Source changes add reusable sidecar plans and
conversion storage to `ShtPlan`/workspace; the f64 scalar and spin recurrence
algorithms remain the only algorithm owners. The patch is deliberately not
applied to production because three of four tested cells miss DUCC parity.

## Exact evidence and next action

[`paired-results-sanitized.json`](paired-results-sanitized.json) retains every
matched pair's times, ratios, errors, fixture/output hashes, and warm-worker
checks without host telemetry identifiers. `SHA256SUMS.txt` covers the
preserved source, results, tests, profiles, and disassembly. The exact isolated
candidate is recoverable on parent `f66ff4ed` with
`gzip -dc candidate.patch.gz | git apply`.

**Next:** reconcile and rerun the shared reflected-ring f32 scalar-synthesis
candidate from parent `49295a89` on current `origin/dev`, as directed in Stage
A. If Stage A does not provide a safe current-source route, use the profiles
above to choose a single cause-based Stage C experiment; do not widen this
spin-1 adapter route based only on full-q1 parity. M6 remains open.
