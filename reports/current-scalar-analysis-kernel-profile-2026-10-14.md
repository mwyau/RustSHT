# Current scalar analysis profile and unfinished fused-kernel investigation

## Scope and result

This note records a current-source f64 scalar GL analysis profile at commit
`5d3ed6b3fd3d8e7d2e3c248d525e91e1e515db62`. It is an investigation result,
not a production change. The follow-up register-resident recurrence and
accumulation kernel was not implemented. The only source edits made while
sketching its trait boundary were removed; the tree is source-clean apart from
this report and its raw timing file.

The profile identifies paired recurrence and accumulation as the main
remaining RustSHT analysis work. It also shows a concrete code-generation
difference: the recurrence kernel writes its current degree values to worker
scratch, while a per-degree `FnMut` callback then invokes a separate SIMD
reducer and horizontal reduction. This suggests a fused scalar-analysis
kernel could avoid the callback and the second read of the recurrence basis.
That mechanism remains a hypothesis until an implementation is measured.

## Machine and builds

- CPU: AMD Ryzen 9 5950X; 16 cores / 32 hardware threads; AVX2/FMA, no AVX-512.
- OS: Linux `7.0.14-20-pve`; CPU affinity was set with `taskset`.
- Rust: `1.99.0`; release binary used `opt-level=3`, 16 codegen units,
  portable Pulp runtime dispatch, and no `target-cpu=native`.
- `perf_event_paranoid=4` prevents hardware event collection. Callgrind 3.26.0
  was available. Its instruction references are not cycles or elapsed time.
- Pinned DUCC reference elsewhere in the session: `ducc0==0.41.0`, source
  revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.

## Native baseline

`examples/m1b_bench.rs` used f64 GL STANDARD analysis, full `mmax`,
`phi0=0.37`, deterministic map data, a reused plan/workspace, one warm-up and
five timed calls in each fresh process. Each row below has five process
medians. q1 was pinned to CPU 0 and q8 to CPUs 0–7. Dimensions were the exact
grid `ntheta=lmax+1`, `nphi=2*lmax+3`.

| Band | Threads | Process medians (ms) | Median (ms) |
|---|---:|---|---:|
| T127 | 1 | 0.612, 0.608, 0.607, 0.618, 0.613 | 0.612 |
| T127 | 8 | 0.334, 0.316, 0.281, 0.337, 0.338 | 0.334 |
| T511 | 1 | 22.607, 25.534, 22.788, 22.791, 22.701 | 22.788 |
| T511 | 8 | 6.875, 7.068, 6.982, 6.959, 6.894 | 6.959 |

The complete process output is in
[`native-process-samples.txt`](data/current-scalar-analysis-profile-2026-10-14/native-process-samples.txt).
T511 q1 contains one slower process; subsequent candidate comparisons should
alternate source binaries and compare paired process medians.

The historical matched Python-plan comparison in
[`scaled-lane-normalization-2026-10-14.md`](scaled-lane-normalization-2026-10-14.md)
reported T511 scalar analysis at `23.058/7.344 ms` for RustSHT and
`6.470/1.074 ms` for DUCC at one/eight threads, respectively. Those values
include Python plan-method and per-call workspace boundaries and are context,
not a native reused-workspace comparison to the timings above. Relative-L2
coefficient error in that comparison was `3.484e-14`.

## Callgrind and generated code

The current release `m1b_bench` binary was profiled with five requested-only
analysis calls plus process setup. Hardware counters were unavailable.

- Scalar T511 q8: `3,248,451,599` instruction references. The paired Pulp
recurrence vectorizer accounted for about 37.91%; the normal
`AnalysisPairAccumulation` reducer for 15.52%; mixed-exponent reduction for
4.99%; `normalize_pair_lanes` for 4.47%; and the RustFFT column butterfly path
for 3.16%. Inclusive and exclusive entries in Callgrind have different
denominators and overlap; these shares are attribution, not timing fractions.
- Spin-1 T511 q8: `4,874,552,669` references. The paired recurrence
vectorizer accounted for 25.35%, `AnalysisPairChannelsAccumulation` 18.28%,
mixed reduction 6.89%, and the recurrence callback 6.50%. The column
butterflies accounted for 4.21%. This profile is context only; no spin kernel
was changed in this investigation.
- `finite_prefix_sums` appeared in the scalar and spin profiles even though
  its production call sites are in synthesis staging. Its attribution was not
  resolved, so it is excluded from analysis conclusions.

The AVX2 `AnalysisPairAccumulation` body uses packed `vmulpd`/`vaddpd`, two
partial chains per component, and eight YMM accumulators. It reduces those
vectors horizontally once for each degree-pair callback. The channel reducer
loads one basis/cosine pair for the shared spin-1 Q/U work and keeps its
accumulator vectors live without a vector spill in the main loop.

The AVX2 `ForEachPairedYlm` body advances recurrence vectors and writes the
next state to scratch. The generated body calls its `FnMut::call_mut` visitor
for each degree pair. Analysis then dispatches the separate accumulator over
the basis slice. Thus the proposed scalar-analysis experiment would differ
from the previously rejected spin-analysis visitor: it would remove a
recurrence-basis store/read and the per-degree accumulator dispatch, with
Fourier-product range checks already hoisted outside the degree loop. It must
still prove that this saves whole-transform time and does not regress the
scaled path.

## Earlier rejected work that constrains a retry

- The rejected fused spin-analysis visitor removed one dispatch but increased
  T127 Callgrind references by 33.1% because checks and callback code grew.
  Those eligibility checks were later hoisted. A scalar-only register-resident
  kernel would be a different implementation, but must avoid repeating the
  old callback structure.
- A two-chain reducer is retained. A third chain caused YMM stack traffic and
  slowed q1 analysis 6.8–7.2%; a two-vector unroll helped q1 modestly but q8
  was inconclusive.
- FMA reduced simulated instruction references but slowed spin-1 q8 analysis
  2.84%; it was removed.
- Table, recurrence, scheduling, compaction, symmetry, scaling, and other
  rejected measurements are indexed in `plans/05-validation-and-milestones.md`
  and detailed under `reports/`.

## Unfinished candidate and safe continuation point

No fused analysis kernel, new selector, public API, or numerical behavior was
changed. A potential prototype should use existing worker coefficient scratch
to stage each block's partial coefficients. It should commit those partials
only after the whole fused block remains in normal range; otherwise it should
discard them and run the established paired scaled recurrence. This avoids a
partial-output rollback problem if a recurrence lane crosses the scaling
boundary. The implementation must preserve mirrored-ring weights, parity,
the equatorial singleton, cutoff compaction, extreme finite products and
scalar tails. Begin with f64 scalar GL analysis, then check T127/T511 and
q1/q8 against this baseline. Do not extrapolate a performance result to
spin-1 or other grids without measurements.

Reproduction build and command:

```text
cargo build --release --example m1b_bench
taskset -c 0 target/release/examples/m1b_bench 0 511 1 analysis GL STANDARD 512 1025 511 requested-only
taskset -c 0-7 target/release/examples/m1b_bench 0 511 8 analysis GL STANDARD 512 1025 511 requested-only
```

The same command with `0 127 1/8` in place of `0 511 1/8` reproduces the
T127 workload. Each execution prints its five samples and median.
