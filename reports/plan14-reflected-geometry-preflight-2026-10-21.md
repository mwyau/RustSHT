# Plan 14: Hoist reflected synthesis geometry validation

**Implementation:** `93ba59fb27490aa6cce51f15ccb4ccd2b221a3ba`  
**Baseline:** `4909c3b856a0de31197293d0ed0fbb8e6947dbe9`  
**DUCC:** 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`

## Change

The scalar GL ordered synthesizer requires reflected north/south latitudes
and monotone north-side polar-order limits. The baseline checked the same
immutable plan geometry inside every per-order dispatch. At T720 with
`mmax=180`, that repeated 361 reflected-pair checks for each of 181 orders
(65,341 pair checks per transform). The change preserves the same tolerance
and checks, validates once before launching q8 order workers, and passes a
private immutable validated-geometry view to each order. Serial synthesis
still validates once per transform. Invalid geometry still declines the
optimized route before output is written, preserving the existing fallback.

## Matched q8 synthesis result

The native parity harness measured T720 GL, `ntheta=721`, `mmax=180`,
`nphi=361`, `phi0=0.23`, q8. Baseline and candidate used the same
deterministic inputs, plans, workspace reuse and output semantics. Runs were
interleaved baseline/candidate/candidate/baseline. Each source contributed
60 timed samples from six independent worker processes, with 10 calibrated
approximately 25 ms batches per process and three interleaved Rust/DUCC
process pairs per capture.

| Source | Rust ms, median [p10, p90] | DUCC ms, median [p10, p90] | Paired Rust/DUCC, median [p10, p90] | Excess ms |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 1.6033 [1.5843, 1.6735] | 1.2607 [1.2463, 1.3098] | 1.273 [1.246, 1.328] | +0.3426 |
| Candidate | 1.5379 [1.5163, 1.6127] | 1.2619 [1.2484, 1.2706] | 1.224 [1.195, 1.269] | +0.2760 |

Rust median latency fell by 65.4 µs (4.1%); DUCC changed by 1.2 µs between
captures. The measured Rust/DUCC excess fell by 66.7 µs, about 19% of the
baseline excess. Rust and DUCC output relative L2 error was
`5.195e-14`; output reset checks passed. The paired-ratio ranges overlap, so
the observed improvement is bounded to this workload and host interval.

The q1 control measured 11.268 ms baseline and 10.296 ms candidate Rust, but
median runqueue wait was 6.9–7.2% and p90 was 14–15%; it is inconclusive.
The q1 serial route already performed this geometry scan once.

## Dispatch and code-generation evidence

Both benchmark workers reported runtime Pulp V3 with AVX2/FMA and four f64
lanes. Rust used 1.99.1/LLVM 23.1.1, strict FP and
`-C target-cpu=x86-64-v3`. DUCC used GCC 15.2, x86-64-v3 SHT objects with
`-O3 -ffast-math -ffp-contract=fast`; its benchmark driver remained strict.
The host was a Ryzen 9 5950X, with q8 pinned to physical CPUs 0–7. CPU PSI
`avg10` was zero at the start and end. Runqueue wait stayed below 2.1% at
p90 in the q8 captures, although one candidate capture recorded an
involuntary-switch outlier.

In the baseline optimized binary, the per-order
`synthesize_range_dispatched` body contains the reflection and polar-limit
checks before dispatching the SIMD kernel. In the candidate binary,
`synthesize_scalar_ordered_parallel` calls the same validator once before
the Rayon region, and the per-order route no longer contains those checks.
The generated code also changed: LLVM inlined the V3 synthesis body into the
candidate range dispatcher, whereas the baseline makes a direct call to the
separate V3 body. The dispatcher frame grew from 248 to 1,224 bytes; this
comparison does not isolate the validation scan's share from the associated
code-generation change. The whole-transform A/B supports retaining this
change, but the full 65.4 µs saving should not be attributed exclusively to
the removed scan.

## Validation and next step

The new unit test checks acceptance of symmetric geometry and rejection of
asymmetric latitudes and nonmonotone order limits. Parallel ordered-versus-
serial synthesis, extreme/subnormal fallback tests, all release tests,
no-default-feature release tests, strict Clippy and formatting passed.

Reproduction used the existing
[`ducc_full_transform_parity` example](../examples/ducc_full_transform_parity.rs)
and [`benchmark_ducc_full_transform_parity.py`](../scripts/benchmark_ducc_full_transform_parity.py).
The compact batch data, per-capture distributions, actual dispatch strings,
host summaries, errors and fixture hashes are in
[`summary.json`](data/plan14-reflected-geometry-preflight-2026-10-21/summary.json).
For each source, build the example with
`RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'` and a separate
`CARGO_TARGET_DIR`; run the harness with `--skip-build --lmax 720 --mmax
180 --fixed-ntheta 721 --fixed-nphi 361 --operation synthesis --threads 8`
and the pinned DUCC build. Repeat in baseline/candidate/candidate/baseline
order. The checked-in summary records the benchmark binary settings and
source revisions.

The current q8 excess is still about 0.276 ms. Reprofile recurrence and
inverse-row work on this exact combined source before choosing another
optimization; the q1 result does not justify a source change.
