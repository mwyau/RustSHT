# Exponent-epoch prefix component profile, protocol v1

## Purpose

Measure the internal costs of the scaled-prefix transition in the isolated
128-ring scalar candidate. The profile is a single q1 route diagnostic. It is
not a whole-transform performance comparison and has no acceptance interval.
The candidate source, binary and q1 comparisons are identified in the
[q1 protocol](exponent-epoch-q1-screen-v1-protocol.md).

## Frozen source and instrumentation

Reconstruct the q1 candidate from commit `2148c7b6a7db332bb3678640efd18b4e979fc6e2`,
apply the [tile-128 integration patch](../../tile128-screen/candidate-source.patch)
(SHA-256 `bd5ce130e0b619f87660eaa80d04e004605fcabc35e6769d3ca18051d0f0b2c7`),
then apply the [exponent-epoch patch](prefix-exponent-epoch-accumulator.patch)
(SHA-256 `badc0b317705d27f2d846028baf577fd4fb4d880ab3b7e603a92c5bff89c1fb2`).
The uninstrumented `src/legendre/synthesis_fast.rs` SHA-256 is
`0f12d2aa0881d97dff66fb2b0a63ddbbfae0fb295bc67b066fe9fcced624d072`.

Apply the test-only
[component instrumentation patch](exponent-epoch-prefix-component-profile-v1.patch),
SHA-256 `8110c32350e662eb934dc913288d011d1f0f2a8adbfa81b66acc8403474feaa5`.
The resulting source SHA-256 is
`50f84bb8c449c8fde97da636070147de8d3995447119c27e33c272ec10929252`.
It adds counters and timers under `cfg(test)`; it does not change the release
kernel. Apply it with `git apply --unidiff-zero`. The compiled release test
binary SHA-256 is
`521b35bb5d8f462749cf62ceecab7bb707a632ba604e0647e6b860594bcfc4c9`.

The profile samples a prefix degree pair when `(pair + (m & 63)) & 63 == 0`.
It times factor checks and scaled complex accumulation, recurrence updates,
normalization, exponent-change scanning and rebasing, transition eligibility
checks, and final prefix materialization separately. It also records the
sampled pair count, lane visits and exponent changes. Sampled nanosecond totals
are measurements of selected iterations only. Do not extrapolate them to the
whole prefix or add them to the nested route timers.

## Fixed workload and execution

Use GL f64 scalar synthesis with `lmax=mmax=2047`, `2048×4096`, `phi0=0.37`,
and one Rayon worker. The test builds deterministic coefficients at scale
`1e-5`, runs five unprofiled complete transformations, then one profiled
transformation. It asserts bitwise equality between the profiled and warm-up
maps. The CPU model, compiler and dependency versions match the q1 protocol.

Build the test binary with Rust/Cargo 1.99.0, release profile and all features:

```sh
CARGO_TARGET_DIR="$PROFILE_TARGET" cargo +1.99.0 test --release \
  --all-features --lib m6_fast_route_counts --no-run
```

Run the exact ignored test once:

```sh
taskset -c "${RUSTSHT_CPUSET:?one allowed logical CPU}" \
  env RAYON_NUM_THREADS=1 RUSTSHT_FAST_ROUTE_LMAX=2047 \
  RUSTSHT_FAST_ROUTE_THREADS=1 \
  "$PROFILE_BINARY" --ignored --exact \
  legendre::synthesis_fast::tests::m6_fast_route_counts --nocapture
```

The local capture records preflight, during-run and postflight CPU PSI, CPU
frequency, affinity, thread placement, cgroup throttling and process activity.
Do not run another benchmark or compilation workload concurrently.

## Validity and interpretation

- Reject preflight or postflight CPU PSI `some avg10` above 2.00%. Values from
  0.50% through 2.00% are warnings; report exact preflight and postflight
  readings.
- Review actual worker placement, sampled CPU frequency, involuntary context
  switches, cgroup throttling and concurrent CPU-intensive work. Clear
  interference makes the profile inconclusive below the PSI threshold.
- Run exactly one profile. Do not retry or discard a result after viewing its
  component timings. Preserve raw host captures and test output locally.
- Report route counts, sampled component totals, full profiled-call time,
  aggregate peak RSS, source and binary hashes, PSI readings, and the map
  equality check. A single profiled call has no timing-stability interval.

This profile can identify which measured prefix operations deserve a separate
source-level hypothesis. It cannot establish that removing or reorganizing an
operation improves a complete transform.
