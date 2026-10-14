# Operation-specific synthesis latitude groups

## Decision

Retain a narrower 32-ring schedule for selected high-bandlimit f64 GL STANDARD
synthesis calls. It improves the measured whole-transform times for scalar q4
through T720 and spin-1 q8/q16 from T640 through T720. The policy excludes
spin-1 q4, reduced `mmax`, CC/F1, over-resolved GL, f32, scalar q1/q2, and
spin-1 modes other than STANDARD. GRAD_ONLY and DERIV1 keep their prior group
size. This avoids the measured regressions when extra groups repeat spin-1
coefficient preparation or when derivative work is grouped more finely.

The policy is deterministic and queried at execution time, so it follows the
Rayon pool actually executing the operation. A workspace initially allocates
the existing number of recurrence workers; an eligible STANDARD synthesis
call grows that private scratch on demand. The immutable plan and numerical
kernels are unchanged. The extra worker records remain in the workspace for
subsequent calls.

## Investigation and hypothesis

A temporary, removed per-group timer measured scalar GL T720 q8 groups at about
4.3–6.2 ms for central/equatorial ranges and 14–20 ms for polar ranges. The
existing active-degree-lane estimate ranked some central groups as more work,
so it did not predict observed time. A smaller ring group can expose more
parallel tasks across these heterogeneous recurrence regions. It also
duplicates per-group setup and may hurt paths that cache or prepare spin
coefficients per group; therefore group size and mode are selected
independently rather than globally.

The candidate changes task partitioning only. It does not compact latitude
lanes, change polar cutoffs, alter recurrence transitions, or reorder the
degree accumulation within a ring. Existing kernels continue to handle
excluded rings and scaled recurrence states.

## Method

- Baseline source: `8d5db59` (`Avoid unsafe spin-1 table setup`). Candidate:
  the group policy and lazy scratch growth recorded in the commit containing
  this report.
- CPU: AMD Ryzen 9 5950X; Linux `7.0.14-20-pve`.
- Compiler: Rust 1.99.0, LLVM 23.1.1; release `opt-level=3`, 16 codegen units,
  LTO disabled. Pulp runtime AVX2/FMA dispatch; no `target-cpu=native`.
- Operations used f64, full `mmax=lmax`, GL, STANDARD, `phi0=0.37`, a reused
  plan/workspace, one warm-up, and five timed calls per process. Baseline and
  candidate ran as paired, pinned processes on the same CPUs. Three process
  pairs were used for the reported T640/T720 comparisons and T511 scalar q4;
  the candidate was compared at q4, q8, or q16 as shown. The benchmark is
  `examples/m1b_bench.rs`.
- Exact grids used `ntheta=lmax+1`, `nphi=2*lmax+3`: T511 `512x1025`, T640
  `641x1283`, and T720 `721x1443`.
- Values below are process medians in milliseconds, each formed from five
  timed executions. The table reports the median of those process medians;
  individual process medians are preserved in the data file.

Build and invocation shape:

```sh
cargo build --release --example m1b_bench
taskset -c 0-15 target/release/examples/m1b_bench 0 720 8 synthesis GL STANDARD 721 1443 720 requested-only
taskset -c 0-15 target/release/examples/m1b_bench 1 720 8 synthesis GL STANDARD 721 1443 720 requested-only
```

For each row, the thread count and dimensions were replaced with the values in
the table. Baseline execution used the example binary built from `8d5db59`;
the candidate binary was built from this working tree with identical release
settings.

## Native results

| Band | Operation | Threads | Baseline process medians (ms) | Candidate process medians (ms) | Median change |
|---|---|---:|---|---|---:|
| T511 | Scalar STANDARD synthesis | 4 | `21.489, 20.965, 21.119` | `15.952, 15.840, 15.842` | `-25.0%` |
| T640 | Scalar STANDARD synthesis | 4 | `38.398, 38.491, 38.835` | `29.974, 30.258, 29.178` | `-22.1%` |
| T640 | Scalar STANDARD synthesis | 8 | `25.975, 26.397, 25.943` | `21.342, 21.796, 21.841` | `-16.1%` |
| T640 | Spin-1 STANDARD synthesis | 8 | `38.330, 42.288, 35.407` | `35.072, 36.699, 35.094` | `-8.4%` |
| T720 | Scalar STANDARD synthesis | 4 | `50.573, 51.063, 50.625` | `44.098, 44.048, 43.951` | `-13.0%` |
| T720 | Scalar STANDARD synthesis | 8 | `34.187, 35.146, 34.850` | `28.056, 28.097, 27.774` | `-19.5%` |
| T720 | Scalar STANDARD synthesis | 16 | `33.423, 35.131, 31.645` | `25.331, 24.596, 25.555` | `-24.2%` |
| T720 | Spin-1 STANDARD synthesis | 8 | `45.849, 45.877, 51.242` | `41.858, 42.107, 46.745` | `-8.2%` |
| T720 | Spin-1 STANDARD synthesis | 16 | `45.191, 44.205, 44.764` | `41.762, 42.521, 41.757` | `-6.7%` |

The narrower policy was excluded from several controls. T511 scalar q8 already
uses 32-ring groups and showed no useful change. At T720 q8, spin-1 GRAD_ONLY
medians were `50.934 -> 50.861 ms` (flat); scalar DERIV1 q4 was
`70.162 -> 71.134 ms` (+1.4%, within shared-host variation). Spin-1 STANDARD
q4, which remains on the existing grouping, was `63.717 -> 65.539 ms` in
three noisy process medians and showed no consistent material effect.

## Cost and memory

Workspace creation time at T720 q16 was `7.561 ms` at baseline and `7.552 ms`
with the candidate. The additional recurrence records are allocated lazily on
the first eligible synthesis, so that call includes scratch growth. The
latencies in the table are warm steady-state measurements and exclude this
first-call cost. At q16, one
`/usr/bin/time -v` pair measured peak RSS of `63,196 KiB` baseline and
`64,028 KiB` candidate (`+832 KiB`); this is one process pair, not a stable
memory survey. No full map, spectrum, or harmonic table is added.

The prototype initially applied finer groups to all spin modes. That version
regressed q8 GRAD_ONLY and q4 DERIV1, and q4 spin-1 STANDARD was about 15%
slower because it repeated scalar-conversion work across more groups. It was
replaced by the present mode- and pool-specific policy. Eagerly allocating
extra workers in `ShtWorkspace::new` also slowed those control paths; worker
growth is now lazy and operation-local.

## Numerical and DUCC checks

Focused q4 T511 scalar and q8 T720 spin-1 tests verify synthesis and analysis
adjoint identities after dynamic scratch growth. The spin test includes low,
interior, and sectoral degree/order coefficients and mixed E/B channels.
Scalar T511/T1023/T2047 GL, CC and F1 release comparisons against DUCC 0.41.0
and analytic fields passed under q4 after the scheduling change. Across those
grids, maximum map error against DUCC was `2.083e-13`, maximum analytic map
error was `9.958e-12`, and scalar analysis coefficient error was
`4.175e-14`. These establish that the shared numerical code remains within
the existing criteria; the accepted scheduling policy itself changes no
arithmetic.

Current Python-plan comparisons against installed `ducc0==0.41.0` (source
revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`) were run for context, not
as before/after evidence. At T511 scalar q4, RustSHT was `15.522 ms` versus
DUCC `2.211 ms`, with relative-L2 map error `6.632e-14`. At T720 q8, scalar
was `30.468` versus `2.865 ms`, error `8.545e-14`; spin-1 was `42.749` versus
`6.292 ms`, error `9.123e-14`. Python input copies (`0.1–0.2 ms`) and output
allocation (`<0.002 ms`) were small relative to transform latency. The
remaining native gap is substantial; this schedule is an incremental
improvement, not parity.

## Validation

- `cargo fmt --all -- --check` — passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  passed.
- `cargo test --workspace --all-features` — 148 Rust tests and 10 f32 tests
  passed; 8 high-band tests ignored.
- `cargo test -p rustsht --no-default-features` — see final validation result
  in the commit record; 7 high-band tests are intentionally ignored.
- `uv run pytest python/tests` — see final validation result in the commit
  record.
- Opt-in release scalar DUCC/analytic checks through T2047 — passed on GL,
  CC and F1.

## Conclusion

The group schedule should be selected per operation and execution pool. The
measured gains are large enough at scalar q4 and at high-band spin-1 q8/q16 to
retain this small policy, while lazy scratch growth and mode guards avoid the
observed regressions. The active-lane work estimate was not a reliable proxy
for group time, so future scheduling work should measure actual per-worker
completion and setup costs before adding more complex weights. The next
profiling target remains the large recurrence/accumulation cost relative to
DUCC, especially spin-1 analysis and the remaining high-band synthesis gap.
