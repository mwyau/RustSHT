# Parallel scalar CC/F1 analysis resampling

## Production change

Scalar exact analysis on CC and F1 now uses the existing bounded, tiled
order-parallel resampler when the plan is f64, resampling is required, at least
two workers are active, and estimated order-by-ring work reaches the existing
65,536-pair cutoff. This reuses the cutoff already measured for spin-1; it adds
no new FFT path or workspace API. Small cases and q1 continue to use the serial
route. F64 scalar CC/F1 workspaces now reserve the same bounded order tile and
per-worker FFT scratch as spin-1.

The change is limited to exact scalar analysis. CC pole handling, F1's
half-grid phase, normalization and prepared-grid weights remain in the shared
resampler. Projected analysis and both adjoints are unchanged.

## Validation

`scalar_cc_f1_analysis_parallelizes_only_large_resampling_work` checks q1/q8
agreement for CC and F1 at T255, confirms that full-order work selects the
parallel route while quarter-order work stays serial, and upgrades a q1-created
workspace to q8. Relative coefficient error stayed below `4e-13`. Existing
DUCC, closed-form and adjoint tests also passed.

- `cargo test --release --lib -- --test-threads=1`: 272 passed, 36 ignored.
- `cargo test --release --no-default-features --lib -- --test-threads=1`:
  220 passed, 17 ignored.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt` and `git diff --check`: passed.

The first concurrent full-suite run had one unrelated worker-count assertion
fail; it passed in isolation and in the serial full-suite rerun.

## Matched benchmark

The primary comparison used exact CC T720 (`ntheta=722`), full order,
`nphi=1458`, `phi0=0.23`, q1/q8, and pinned DUCC 0.42.0
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Each backend had 30 timed batches
across three process pairs, with no samples removed. The saved serial-path
baseline for the same full-order geometry was Rust/DUCC `34.557/12.292 ms` at
q8 (frozen Rust source `3f838847`; see the [saved comparison row](data/f64-ducc-independent-review-2026-10-22/cc-gl-selected.csv));
the current candidate measured `14.510/13.108 ms`, cutting Rust latency by
58.0% and reducing the ratio from `2.81x` to `1.09x`. The current q8 process
pair ratios ranged from `1.03x` to `1.16x`, so the median is still slower than
DUCC and parity is not claimed. The q1 control measured `61.456/58.180 ms`
(`1.03x`), close to its saved `60.146/59.120 ms` control.

Displayed backend latencies are aggregated medians; Rust/DUCC ratios are
medians of the interleaved process-pair ratios.

The quarter-order CC control (`mmax=180`) used `nphi=1536` at both backends.
Rust/DUCC measured `26.277/18.872 ms` q1 (`1.39x`) and `5.680/6.263 ms` q8
(`0.95x` paired ratio). This is evidence for the q8 route on reduced work; the q1
recurrence gap remains.

The T720 full-order q8 synthesis excess remains open at `1.32x` DUCC from the
separate M1 measurement. F1 performance is not measured here. Overall CC/F1
parity is not closed.

Sanitized per-case data are in
[`summary.csv`](data/cc-parallel-resampling-m2-2026-10-23/summary.csv). Raw
JSON, timed samples and local host telemetry remain untracked in that data
directory. The candidate was built from the Rust source tree based at
`32a34684d6fbc9ebaf892d3bf75ec50b8edf7c16` with the five-file source patch
SHA-256 `059d90f31d2ee894c434f9113b523b3687075d4d2350b051164836a8e7835744`;
the Rust benchmark binary SHA-256 was
`6553eb49b04dd28d3754648d0c0723a6ace174169d319918beb52533a5b6088c`.

Reproduction commands (with `DUCC_SRC` pointing to the pinned checkout):

```sh
python3 reports/data/cc-scalar-synthesis-grain-2026-10-23/checkpoint_identity_compat.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --grid CC --lmax 720 --mmax 720 --threads 1 --threads 8 \
  --operation analysis --longitude-mode even_smooth \
  --results-dir "$RESULTS_DIR/full" --no-report \
  --sweep-id cc-parallel-resampling-m2-full

python3 reports/data/cc-scalar-synthesis-grain-2026-10-23/checkpoint_identity_compat.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --grid CC --lmax 720 --mmax 180 --fixed-nphi 1536 \
  --threads 1 --threads 8 --operation analysis \
  --results-dir "$RESULTS_DIR/quarter" --no-report \
  --sweep-id cc-parallel-resampling-m2-quarter
```

## Reduced-order Fourier-prefix follow-up

The next source change makes scalar analysis request only `mmax+1` modes from
the existing truncated real FFT on GL, CC and F1. CC/F1 normalization now also
touches only that prefix. The remaining stages already consume at most `mmax`,
so unused ring-major spectrum tails may retain old workspace contents. This
does not change the FFT engine or the guarded paired-odd route.

`scalar_reduced_order_analysis_ignores_unused_spectrum_tail` poisons the
unused tail with NaNs, then checks exact analysis, projected analysis, and the
synthesis adjoint on CC and F1 against clean workspaces. The full release
library suite passed (273 passed, 36 ignored), as did the no-default-features
suite (221 passed, 17 ignored), strict Clippy, formatting and diff checks.

The focused comparison used exact CC T720 (`ntheta=722`), `mmax=180`,
`nphi=1536`, `phi0=0.23`, q1/q8, and DUCC 0.42.0 at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Each backend collected 30 timed
batches across three interleaved process pairs and passed the numerical,
reset, edge-case and adjoint gates (relative L2 `6.47e-14`).

| q | Rust ms | DUCC ms | paired Rust/DUCC ratio | paired ratio p10–p90 | saved Rust ms | Result |
|---:|---:|---:|---:|---:|---:|---|
| 1 | 24.810 | 18.833 | 1.322x | 1.308–1.333x | 26.277 | 5.6% lower Rust time; still 5.98 ms slower than DUCC |
| 8 | 5.988 | 6.275 | 0.924x | 0.871–0.956x | 5.680 | Near DUCC; directional only |

The q1 timing met the 20–100 ms calibrated batch window. For q8, 88.3% of
measured batch durations landed in that window; host runqueue wait was 15.1% at
p90, with up to 24,197 involuntary switches. The q8 result is therefore not a
clean before/after claim. This change narrows the measured q1 gap but does not
close CC analysis parity; F1 performance remains unmeasured.

Reproduce from the committed candidate with the pinned DUCC checkout in
`DUCC_SRC`:

```sh
python3 reports/data/cc-scalar-synthesis-grain-2026-10-23/checkpoint_identity_compat.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --grid CC --lmax 720 --mmax 180 --fixed-nphi 1536 \
  --threads 1 --threads 8 --operation analysis \
  --production-source-revision 684f22f0ba3671ac8a8466de33600ed495f2ae32 \
  --results-dir "$RESULTS_DIR" --no-report --retain-local-host-telemetry \
  --sweep-id cc-reduced-fft-prefix
```

Raw JSON, timed batches and host telemetry remain local under `/tmp` and are
not committed.

## Direct prepared-spectrum follow-up

After CC/F1 resampling, the ring-major spectrum is already normalized and
weighted. The scalar recurrence analysis path now consumes that prepared input
directly while packing reflected latitude pairs, skipping the extra
order-major transpose and copy. The weighted direct-input path used by GL and
projected grids is unchanged. The regression test compares direct prepared
input with the previous order-preparation values for CC/F1.

The focused comparison used CC T720 (`ntheta=722`), `mmax=180`,
`nphi=1536`, q1/q8 and pinned DUCC 0.42.0. Each backend had 30 timed batches
across three interleaved process pairs; all comparisons passed numerical and
reset checks (relative L2 `6.47e-14`).

| q | Rust ms | DUCC ms | paired Rust/DUCC ratio | paired ratio p10–p90 | saved Rust/DUCC ms |
|---:|---:|---:|---:|---:|---:|
| 1 | 24.273 | 18.671 | 1.304x | 1.293–1.305x | 24.810/18.833 |
| 8 | 3.905 | 5.253 | 0.743x | 0.730–0.781x | 5.988/6.275 |

The saved q8 run had noisy host telemetry and missed the calibrated batch
window for some samples, so the q8 comparison is directional rather than a
strict before/after claim. The current q1 case remains 30% slower than DUCC;
the q8 case is faster. This does not close CC analysis parity. A GL T720
quarter-order control (`nphi=1458`) measured Rust/DUCC ratios of `0.911x` at
both q1 (7.635/8.366 ms) and q8 (1.125/1.242 ms); numerical and timing checks
passed.

Reproduction commands, with `DUCC_SRC` pointing to the pinned checkout:

```sh
python3 reports/data/cc-scalar-synthesis-grain-2026-10-23/checkpoint_identity_compat.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --grid CC --lmax 720 --mmax 180 --fixed-nphi 1536 \
  --threads 1 --threads 8 --operation analysis \
  --production-source-revision f03b16bfb6b71f6382055ce0c6cf03cbbc4702c3 \
  --results-dir "$RESULTS_DIR" --no-report --retain-local-host-telemetry \
  --sweep-id cc-prepared-direct-analysis
```

Raw JSON, timed batches and host telemetry remain local under `/tmp`.

## Reflected-profile clear follow-up

The resampler used to clear the full caller-owned profile before constructing
each reflected CC/F1 profile. The reflection writes every entry in the active
FFT span; larger scratch tails are not read before later padding or transforms
overwrite their active range. The redundant clear is removed. Tests poison the
serial and parallel profile scratch with NaNs before resampling and verify the
prepared output remains equal.

The T720 quarter-order CC comparison used the same geometry as the previous
section (`mmax=180`, `nphi=1536`, pinned DUCC 0.42.0), q1/q8, 30 timed batches
across three process pairs per backend, and accepted numerical/reset checks
(relative L2 `6.47e-14`). The prior-source and candidate runs were separate
process-pair sets, so the small differences below are directional, not a
paired source A/B.

| q | Prior Rust/DUCC ms | Candidate Rust/DUCC ms | Prior ratio | Candidate ratio | Candidate paired-ratio p10–p90 |
|---:|---:|---:|---:|---:|---:|
| 1 | 24.273/18.671 | 24.523/18.609 | 1.304x | 1.315x | 1.298–1.335x |
| 8 | 3.905/5.253 | 3.826/5.219 | 0.743x | 0.734x | 0.725–0.740x |

The q1 distributions overlap and the median is effectively unchanged. q8
Rust time fell about 2%, with similar pinned-DUCC time; host runqueue wait p90
was 9.5% for the candidate and 10.4% for the saved prior run. This is a small
directional q8 improvement, not a parity claim. The measured q1 Rust/DUCC
excess remains about `5.9 ms`, the largest open analysis excess in these
focused cases.

After this change, `cargo test --release --lib -- --test-threads=1` passed
(274 passed, 36 ignored), `cargo test --release --no-default-features --lib
-- --test-threads=1` passed (222 passed, 17 ignored), strict all-target
all-feature Clippy passed, and formatting/diff checks passed.

The candidate source is `4a3a173599d81ac7c3078159ead6f08b796886e6`; the prior
source was `f03b16bfb6b71f6382055ce0c6cf03cbbc4702c3`. Reproduce the candidate
comparison with `DUCC_SRC` pointing to the pinned checkout:

```sh
python3 reports/data/cc-scalar-synthesis-grain-2026-10-23/checkpoint_identity_compat.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --grid CC --lmax 720 --mmax 180 --fixed-nphi 1536 \
  --threads 1 --threads 8 --operation analysis \
  --production-source-revision 4a3a173599d81ac7c3078159ead6f08b796886e6 \
  --results-dir "$RESULTS_DIR" --no-report --retain-local-host-telemetry \
  --sweep-id cc-no-profile-clear
```

Raw candidate data and host telemetry remain untracked under `/tmp`.

## Symmetric-cutoff pair preparation follow-up

Scalar analysis now checks the actual ring order limits once per transform.
When mirrored limits match and rise monotonically from the poles, the existing
binary cutoff identifies an all-active suffix. Its reflected pairs can then be
prepared directly without compact ring-index and latitude scratch writes or
per-lane activity branches. Nonmonotone or asymmetric limits keep the previous
compaction path; the checked recurrence fallback reconstructs direct indices
only when it needs them.

The guard test confirms the production T720 CC and F1 cutoff geometry satisfies
the predicate and rejects asymmetric and nonmonotone examples. CC/F1 scalar
DUCC and adjoint tests, mixed-cutoff compaction tests, T191 all-grid compaction,
the full release suite (275 passed, 36 ignored), the no-default-features suite
(223 passed, 17 ignored), strict Clippy, formatting, and diff checks passed.
After the final checked-fallback index reconstruction adjustment, the targeted
geometry, CC/F1, and compaction tests passed again in both release build
configurations. The implementation and regression tests are committed in
`64f4e745`.

Two independent pinned q1 phase-profile runs used exact CC T720, `mmax=180`,
`ntheta=722`, `nphi=1536`, `phi0=0.23`, 30 timed batches per process, two
transform calls per batch, and a 30 ms target batch. Candidate whole-call
medians were `23.855` and `23.806 ms`; recurrence-plus-accumulation phase
medians were `15.767` and `15.747 ms`. The saved pre-change profile had one
30-batch run at `24.101 ms` overall and `16.048 ms` in that phase. This is a
small directional profile improvement, not a matched full-transform A/B. The
last matched end-to-end q1 Rust/DUCC row remains `24.523/18.609 ms` (`1.315x`);
this change does not close the CC q1 gap. The raw profiles remain untracked in
`/tmp`.

## Runtime SIMD-tail padding

Commit `f0ce2aae` pads the final short block of the fused scalar-analysis path
to Pulp's runtime f64 lane width. The additional lanes use equatorial geometry
and zero spectra. If the fused route declines, the recurrence fallback receives
only the original active lanes. The fixed 256-pair tile is unchanged.

The release CC/F1 q1/q8 resampling regression, T191 DUCC reference test across
all grids, scalar-analysis compaction and range-cache tests passed. The
no-default-features scalar-analysis tests and strict all-target/all-feature
Clippy also passed.

The focused post-change benchmark used pinned DUCC 0.42.0 on exact CC T720 at
q1. Each row collected 30 calibrated timed batches across three process pairs;
numerical and output-reset checks passed. Rust used Pulp V3 (AVX2/FMA) and DUCC
used its fixed x86-64-v3 build.

| Order | nphi | Rust ms | DUCC ms | Rust/DUCC | Excess ms | Relative L2 |
|---|---:|---:|---:|---:|---:|---:|
| Full (`mmax=720`) | 1458 | 57.552 | 57.000 | 1.008x | 0.552 | `6.01e-14` |
| Quarter (`mmax=180`) | 1536 | 24.357 | 18.554 | 1.317x | 5.803 | `6.47e-14` |

The quarter-order row is close to the saved prior-source `24.523/18.609 ms`
result, but those runs were not interleaved as a source A/B. No whole-transform
gain is attributed to tail padding. The full-order case is near DUCC; the
quarter-order q1 recurrence gap remains open, and scalar CC/F1 parity is not
closed.

Reproduce the focused quarter-order row with `DUCC_SRC` pointing to the pinned
checkout:

```sh
python3 reports/data/cc-scalar-synthesis-grain-2026-10-23/checkpoint_identity_compat.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --grid CC --lmax 720 --mmax 180 --fixed-nphi 1536 \
  --longitude-mode even_smooth --operation analysis --threads 1 \
  --results-dir "$RESULTS_DIR" --no-report --retain-local-host-telemetry \
  --sweep-id cc-t720-quarter-tail-padding-q1
```

Raw benchmark JSON, timed samples, and host telemetry remain under `/tmp`.

## Safeguarded prefix recurrence for prepared CC/F1 analysis

Exact CC/F1 analysis first resamples longitude spectra onto the prepared
meridional grid. The prepared spectra already carry the resampling weights, so
the previous gate (`analysis_ring_weights.is_some()`) disabled DUCC's
safeguarded insignificant-prefix advancement for these grids. The production
path now marks prepared spectra as eligible for the same f64 prefix route used
by weighted GL analysis. The `2^-60` threshold, recurrence and input range
checks, transactional staged-output behavior, and checked scaled fallback are
unchanged. Projected CC/F1 and adjoint-synthesis inputs remain on the checked
route. F1 uses the same prepared-spectrum eligibility.

`t191_analysis_compaction_matches_ducc_all_grids` now asserts that exact CC and
F1 enter the prefix-enabled scaled route while checking saved DUCC 0.41
reference coefficients and the analysis-adjoint identity. The T191 CC/F1/GL
reference/adjoint test, the CC/F1 q1/q8 resampling regression, the fused
scaled-to-ordinary recurrence test, no-default-features T191 test, and the
benchmark's random-output, reset, zero, extreme, subnormal, and adjoint checks
passed.

The focused post-change comparison used exact CC T720, `mmax=180`,
`ntheta=722`, `nphi=1536`, q1, and pinned DUCC 0.42.0. It collected 30 timed
batches across three interleaved process pairs; all batches landed in the
20–100 ms target window (median batch duration `32.1 ms`). The paired
Rust/DUCC ratio p10–p90 was `0.728–0.733`; host runqueue wait was 0.6% median
and 15.6% p90, with 178 maximum involuntary switches.

| Rust ms | DUCC ms | paired ratio | excess ms | relative L2 |
|---:|---:|---:|---:|---:|
| 13.513 | 18.518 | 0.732x | -5.005 | `6.47e-14` |

The saved pre-change tail-padding row on the same geometry was `24.357/18.554`
ms, Rust/DUCC. The candidate Rust time is about 44.5% lower, but those source
runs were not interleaved; treat that source-to-source change as directional.
The current matched candidate beats DUCC on this quarter-order q1 case. This
does not close full-order, q8, or F1 parity; F1 has not been benchmarked.

Reproduce with `DUCC_SRC` pointing to pinned revision
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`:

```sh
python3 reports/data/cc-scalar-synthesis-grain-2026-10-23/checkpoint_identity_compat.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --grid CC --lmax 720 --mmax 180 --fixed-nphi 1536 \
  --longitude-mode even_smooth --operation analysis --threads 1 \
  --results-dir "$RESULTS_DIR" --no-report --retain-local-host-telemetry \
  --sweep-id cc-t720-quarter-prefix-q1
```

Raw JSON and host telemetry remain local and untracked.
The Rust worker was rebuilt from the working tree based on
`8898e25a353f122aab6198398433b91eaed30fa6` (dirty at build time; binary
SHA-256 `09a7ac5a916e529d44c42dd288722f66cdcc38e80f3c1778f09527322bf3db02`).
The pinned DUCC binary SHA-256 was
`3df1d4408407015c65a85e2909aa34219871bf29a75126b23e497bcbb396c89a`.

## Parallel prepared-output initialization and scatter follow-up

The tiled CC/F1 resampler now initializes its prepared ring-major output in
parallel and scatters each completed order tile across disjoint ring-row ranges.
The bounded order tile, worker scratch and numerical operations are unchanged;
the q1 route remains serial.

The focused A/B used exact CC T720 (`ntheta=722`, `nphi=1458`), full order,
`phi0=0.23`, q8 and pinned DUCC 0.42.0 at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Baseline source was
`f96de1f4`; candidate source adds this scatter change. Both runs used the same
benchmark harness and collected 30 batches per backend across three process
pairs. Rust and DUCC ran in separate source runs, so their source-to-source
comparison is directional. The candidate had 90% of timed batches in the
20–100 ms target window (the baseline had 100%); both rows passed numerical
and reset checks with relative L2 `6.01e-14` and no failure records.

| Source | Rust ms [p10, p90] | DUCC ms | Rust/DUCC | Paired ratio p10–p90 |
|---|---:|---:|---:|---:|
| Before scatter change | 7.124 [6.839, 8.380] | 10.311 | 0.694x | 0.676–0.726x |
| Parallel scatter | 6.811 [6.386, 7.598] | 10.407 | 0.654x | 0.629–0.688x |

Rust latency fell 4.4% (`0.313 ms`). The ratio intervals overlap slightly, so
this is a modest directional gain; the q8 case was already faster than DUCC
before this change. It does not establish current q1, other-degree, or F1
performance. Host runqueue wait p90 was 3.8% for baseline and 3.0% for the
candidate; candidate timed batches were partially outside the target window.

Reproduce each source run from its checkout with `DUCC_SRC` set to the pinned
DUCC tree and `RESULTS_DIR` set to a fresh result directory:

```sh
python3 reports/data/cc-scalar-synthesis-grain-2026-10-23/checkpoint_identity_compat.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --grid CC --lmax 720 --mmax 720 --threads 8 \
  --operation analysis --longitude-mode even_smooth \
  --results-dir "$RESULTS_DIR" --no-report --retain-local-host-telemetry \
  --sweep-id cc-resampling-parallel-scatter
```

Raw JSON, timed samples and host telemetry remain local and untracked.
