# Plan 17 f32 spin normalizer inlining A/B (2026-10-22)

## Decision

Reject `#[inline(always)]` on `spin_f32::normalize_pair_lanes`. It removed two out-of-line calls per recurrence degree step, but the matched full-transform results were neutral to slower in seven of eight cases and did not give a repeatable gain in the remaining q8 synthesis case. The production source is restored to the baseline implementation at `3b3c1ccb`.

## Source and method

- Baseline production commit: `3b3c1ccb26d5f450f53dc999e7659fbf1731870b` (includes the accepted inlining of `recurrence_step`).
- Candidate: baseline plus `#[inline(always)]` on `spin_f32::normalize_pair_lanes`; no arithmetic or fallback changes.
- Rust: 1.99.1 / LLVM 23.1.1, Cargo release defaults; runtime Pulp V3 AVX2/FMA, eight f32 lanes.
- DUCC: 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; GCC 15, `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` for SHT and strict driver flags.
- Host: AMD Ryzen 9 5950X. q1 pinned to CPU 0; q8 pinned to CPUs 0–7 in one L3 domain.
- f32 GL spin-1 STANDARD, full order, `phi0=0.23`, even-smooth longitude: T255 (`256 × 512`) and T720 (`721 × 1458`), analysis and synthesis, q1/q8.
- Baseline/candidate and candidate/DUCC were interleaved across three fresh process pairs. Calls below 100 ms used ten calibrated batches near 25 ms per process; longer calls used five individually timed invocations per process. Plans and workspaces were reused, and reset/repeat checks passed.
- One q8 segment showed mild shared-CPU contention; all observations remain included. The T255 q1 synthesis p90 also reflects a high outlier and is shown rather than filtered.

The [CSV](data/plan17-f32-spin-normalizer-inline-2026-10-22/results.csv) contains medians, p10/p90, process-pair ratios, sample counts and DUCC comparison. The [JSON](data/plan17-f32-spin-normalizer-inline-2026-10-22/results.json) retains every timed batch and output check. Baseline/candidate Rust outputs were bitwise identical in all eight cells.

## Results

Times are milliseconds. Ratio is the median of the three per-process candidate/baseline median ratios. DUCC ratios use matched candidate/DUCC process pairs.

| T | Operation | Threads | Rust baseline median [p10–p90] | Candidate median [p10–p90] | Candidate/baseline | Candidate/DUCC | Candidate vs DUCC relative L2 |
|---:|---|---:|---:|---:|---:|---:|---:|
| 255 | Analysis | q1 | 68.21 [67.60–69.60] | 70.68 [70.40–74.61] | 1.042 | 25.59× | 6.54e-6 |
| 255 | Analysis | q8 | 8.98 [8.94–9.48] | 9.36 [9.29–9.79] | 1.040 | 20.54× | 6.54e-6 |
| 255 | Synthesis | q1 | 85.31 [84.14–99.42] | 86.04 [85.48–89.97] | 1.016 | 20.99× | 3.71e-5 |
| 255 | Synthesis | q8 | 16.08 [15.71–16.54] | 16.18 [15.94–18.89] | 1.002 | 27.94× | 3.71e-5 |
| 720 | Analysis | q1 | 1274.31 [1247.80–1296.60] | 1332.39 [1322.51–1364.84] | 1.044 | 45.93× | 1.57e-5 |
| 720 | Analysis | q8 | 164.20 [162.78–166.12] | 173.77 [172.12–174.90] | 1.059 | 42.42× | 1.57e-5 |
| 720 | Synthesis | q1 | 1661.12 [1642.55–1709.05] | 1721.29 [1701.46–1753.45] | 1.046 | 42.36× | 1.43e-4 |
| 720 | Synthesis | q8 | 457.41 [454.66–468.92] | 460.04 [457.32–461.92] | 0.984 | 82.81× | 1.43e-4 |

T255 short calls have 30 observations per Rust variant and 30 DUCC observations per case; T720 calls have 15 Rust observations per variant and 30 DUCC observations. The T720 q8 synthesis 1.6% candidate improvement is smaller than its observed run-to-run spread and is not evidence of a gain. The other seven cells regress by 0.2–5.9% on their paired medians.

## Assembly finding

In the baseline optimized V3 analysis specialization, `normalize_pair_lanes` is a 521-byte outlined symbol. The degree loop makes two indirect calls to it per recurrence step, each preceded by `vzeroupper` after packed recurrence work. In the candidate, that symbol and those calls disappear. However, the V3 `ForEachSpin32` analysis specialization grows from 3,823 to 4,882 bytes (about 28%); its stack allocation remains `0x1a8` bytes. No cache counter was captured, so code footprint is a plausible cost, not a proven cause of the measured regressions. Removing helper calls alone did not improve end-to-end execution.

## Reproduction

Start at the baseline commit, add the single inline attribute immediately above `normalize_pair_lanes`, and build the candidate with `cargo build --release --features parallel --example ducc_f32_full_transform_parity`. Use the baseline and candidate binaries with the pinned DUCC worker:

```sh
python3 scripts/benchmark_plan17_spin_recurrence_inline.py \
  --baseline-rust /path/to/baseline/ducc_f32_full_transform_parity \
  --candidate-rust /path/to/candidate/ducc_f32_full_transform_parity \
  --ducc /path/to/ducc_f32_full_transform_parity \
  --outdir /path/to/results --lmax 255 720 --threads 1 8 \
  --process-pairs 3 --extra-t255-q8-synthesis-pairs 0 --target-ms 25 \
  --q1-cpu 0 --q8-cpus 0,1,2,3,4,5,6,7
```

The baseline, candidate and DUCC binary SHA256 values are in the JSON metadata.

## Next

Keep the accepted `recurrence_step` inlining. Do not inline the full scalar
normalizer. The frequency-only follow-up below narrows the next experiment to a
SIMD range precheck, but timing evidence is still required before production
changes.

### Follow-up: normalizer rescale frequency

A test-only counter build of the T720 f32 GL spin-1 phase probe measured three
identical samples per operation/thread count. At q1, analysis and synthesis
each called the normalizer 6,229,416 times over 374,284,078 lanes. Q8 analysis
had the same recurrence work. Q8 synthesis used the order-major route and made
5,488,990 calls over 341,980,148 lanes. Depending on route, 15.9–20.3% of
calls had at least one lane rescaled, but only 0.39–0.55% of lanes required
scaling. Each scaled lane required one exponent step in this deterministic
T720 workload. Counts and fractions are in the
[machine-readable summary](data/plan17-f32-spin-normalizer-inline-2026-10-22/normalizer-counts.csv).

The counters were enabled only for the ignored unit-test profile via
`--cfg rustsht_profile_spin32_normalizer`; they do not compile into production
or ordinary test builds. Atomic counting materially distorts elapsed time, so
those timings are excluded. The same test checks finite outputs, and the
normalizer arithmetic is unchanged. The probe uses the matched even-smooth
T720 geometry (`721 × 1458`, `phi0=0.23`). Reproduce the counts with:

```sh
RUSTFLAGS='--cfg rustsht_profile_spin32_normalizer --check-cfg=cfg(rustsht_profile_spin32_normalizer)' \
  cargo test --release --features parallel --lib \
  plan17_f32_t720_native_phase_profile -- --ignored --nocapture
```

This confirms that the helper scans many lanes on every recurrence step while
only a small fraction actually rescale. It does **not** quantify the helper's
own latency separately from recurrence and accumulation, so it is not enough
to claim a performance gain from replacing the scan. Next compare a portable
SIMD range precheck against the scalar lane scan in a benchmark-only variant,
including complete transforms and exceptional-value fallbacks, before any
production change.
