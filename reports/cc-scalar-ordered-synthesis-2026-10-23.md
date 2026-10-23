# Ordered scalar synthesis on CC and F1

## Change

Commit `4d5dcf97941d2b4561a58affc88e67ba3edaa92f` routes f64 scalar synthesis
on sampling-complete CC and F1 grids through the existing ordered/reflected
SIMD recurrence. This reuses DUCC's latitude-pair dataflow already translated
for GL: prepare each order's coefficients, evaluate north-latitude batches,
combine even and odd contributions for the reflected rows, apply the
longitude phase, then run the existing inverse longitude FFT. CC poles and
F1's half-grid midpoint remain covered by the shared geometry and parity
checks. Under-resolved grids and inputs rejected by the ordered kernel retain
the general scaled recurrence fallback.

The selector requires `ntheta >= lmax + 2` for CC and `ntheta >= lmax + 1`
for F1. The ordered kernel still validates reflected sine/cosine values and
monotone polar order limits before writing output; the parallel route clears
partial output if a later order needs fallback.

## Correctness

The new release test compares exact CC and F1 spectra against the general
ring-group recurrence and checks analytic low-degree fields at q1 and q8. It
also exercises the parallel ordered route directly and verifies that
under-resolved grids keep using the general path. `cargo test --release --lib`
passed (271 passed, 36 ignored); strict example Clippy, the release
no-default-features library check, and formatting checks passed.

The matched native benchmark produced 12 accepted random-map comparisons,
zero failure records, and accepted output-reset checks. Maximum relative L2
was `6.451e-14`. The benchmark's full sidecar validation status is incomplete
because this focused run did not include the broad validation sidecar; the
analytic and general-recurrence checks above passed separately.

## CC synthesis results

Warm milliseconds; `q` is the Rust and DUCC thread count. Rust baseline values
are from the saved general-route task-grain experiment. Each new candidate and
DUCC row has 30 timed batches across three interleaved process pairs.

| T / mmax | q | Saved general Rust | Ordered Rust p10–p90 | DUCC | Rust / DUCC | Rust reduction |
|---|---:|---:|---:|---:|---:|---:|
| 255 / 255 | 1 | 6.957 | 1.538–1.669 | 1.504 | 1.056× | 77.6% |
| 255 / 255 | 8 | 3.128 | 0.318–0.342 | 0.330 | 1.013× | 89.5% |
| 720 / 720 | 1 | 96.000* | 19.917–22.768 | 19.623 | 1.038× | 78.6%* |
| 720 / 720 | 8 | 47.916 | 3.756–4.401 | 3.258 | 1.323× | 91.1% |

The T720 q1 saved Rust measurement had only 20 timed batches, so its reduction
is directional rather than a qualified matched A/B. The T720 q8 Rust/DUCC
process-pair ratio range was 1.203–1.343; this remains the largest measured
CC excess here at about `1.03 ms` per transform. T255 q8 is within the
process-pair spread of parity. The code change substantially narrows the
measured gap but does not close it across all cases.

The T720 GL control measured Rust/DUCC at `20.074/19.323 ms` q1 and
`4.431/3.775 ms` q8 (ratios `1.037×` and `1.167×`). GL stayed on its existing
route. F1 passed the analytic and general-recurrence tests, but no F1
performance comparison was run, so F1 parity remains unknown.

## Measurement record

The comparison used pinned DUCC 0.42.0 at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` and commit
`4d5dcf97941d2b4561a58affc88e67ba3edaa92f`, on a Ryzen 9 5950X. Rust used
`-C target-cpu=x86-64-v3`; DUCC used the pinned x86-64-v3 build. q1 used one
physical core and q8 used eight physical cores. Host runqueue wait was 6.6%
median and 18.5% p90; no samples were removed. T255 had a few batches outside
the 20–100 ms target window, while all rows retained qualified timing quality.

Candidate Rust binary SHA-256 is
`6f5478fdd1981acadca7684338de2ef962a8fe3f20d0af41e14865e6c59992ae`; DUCC
binary SHA-256 is
`3df1d4408407015c65a85e2909aa34219871bf29a75126b23e497bcbb396c89a`. The
benchmark harness SHA-256 is
`045104b7ebff8f2b87c01fa3ea1c0ce8f3d3283bb8c581c26e99d0b97f0c6a2b`.

The sanitized case summary is in
[`data/cc-scalar-ordered-synthesis-2026-10-23/summary.csv`](data/cc-scalar-ordered-synthesis-2026-10-23/summary.csv).
Raw candidate and GL-control JSON/CSV captures and local host telemetry remain
untracked under [`raw/candidate`](data/cc-scalar-ordered-synthesis-2026-10-23/raw/candidate)
and [`raw/gl-control`](data/cc-scalar-ordered-synthesis-2026-10-23/raw/gl-control).
Candidate JSON SHA-256:
`ad9414582d3096419836491ff878de1cec93a7864d615c995a3bf67f4369ca66`;
GL-control JSON SHA-256:
`d5a7fa1a02aba27638a417f5ae64e92ce4d5a03ad5a9b20b5b03c66ceaab7594`.
The existing checkpoint compatibility wrapper only supplies the runner's
missing empty build-record field; it does not change worker commands or timed
loops. The CC run can be repeated with:

```sh
python3 reports/data/cc-scalar-synthesis-grain-2026-10-23/checkpoint_identity_compat.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --grid CC --lmax 255 --lmax 720 --full-mmax-only \
  --longitude-mode even_smooth --operation synthesis \
  --threads 1 --threads 8 --results-dir "$RESULTS_DIR" --no-report \
  --sweep-id cc-ordered-synthesis-m1 --retain-local-host-telemetry
```
