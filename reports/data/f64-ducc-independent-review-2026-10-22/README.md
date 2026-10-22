# Independent f64 algorithm review evidence

This directory supports [the technical review](../../f64-ducc-independent-review-2026-10-22.md).
It is a review artifact, not the full CC sweep's publication directory.
Production Rust/DUCC binaries, source and all existing sweep results were
preserved. No optimization candidate was promoted.

The reviewed tree is `7b1629d4f45b6a74cb4ee0e3a53c67f4faa51643`.
DUCC is 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
`metadata.json` records toolchains, flags, affinity and available binary hashes.
The scalar frozen production/harness revisions are `3f838847` / `cd8beaa3`;
`src/`, Cargo.toml and Cargo.lock have no changes between that production
revision and the reviewed tree. Raw assembly and host/process telemetry are
kept outside Git.

For reproduction, set `REVIEW_TMP` to a local temporary directory and select
`REVIEW_CPU_Q1` / `REVIEW_CPU_Q8` as one physical core / eight physical cores
on one CCD. The original local artifacts use the basenames below.

```sh
REVIEW_TMP="${TMPDIR:-/tmp}"
```

**Saved-data audit**

```sh
python3 reports/data/f64-ducc-independent-review-2026-10-22/audit_saved_cc.py \
  --shards $REVIEW_TMP/rustsht-cc-full-sweep-2026-10-22/shards \
  --repo "$PWD" --output $REVIEW_TMP/independent-cc-audit
```

The audit reads every saved JSON, hashes its bytes and constructs an
independent manifest using `mmax={lmax,floor(lmax/4)}`. It does not call the
runner's `make_cases` or its self-generated coverage audit. The historical
representative odd-degree selection is explicitly listed in the script.
`audit.json` lists missing/extra groups with named key fields. No partial
high-shard log is counted as a completed result.

`selected-cc-latencies.csv` retains p10/median/p90 and absolute timings for
representative saved cells. `cc-gl-selected.csv` joins by lmax/mmax/nphi,
operation and q, retaining the different CC/GL latitude counts in the source
datasets. The CC baseline is incomplete; no T2049/T2560/T3072 timing is
published here. Existing GL data come from their own frozen source epoch.

`gl-q1-focused-analysis.csv` extracts both longitude shapes from the existing
independent repetitions. `gl-dispatch-classes.csv` stratifies the older
exploratory screen by the actual AVX2 table dispatch condition. The latter
has only five ~1 ms batches per case and does not establish an acceptance
threshold. The focus data use 30 ~20 ms batches per route/process.

**Independent native numerical controls**

```sh
python3 reports/data/f64-ducc-independent-review-2026-10-22/validate_native.py \
  --rust "$PWD/target/release/examples/ducc_full_transform_parity" \
  --ducc $REVIEW_TMP/rustsht-ducc-full-parity-42ce8ab5c5d7-g++/ducc_full_transform_parity \
  --cpu 8 --output $REVIEW_TMP/analytic-controls.json
```

This constructs closed-form constant/zonal/m=1/sectoral fields independently
of either library. It checks synthesis and analysis, output reset, altered
longitude phase, CC endpoints, m=0 low degrees, and q1/q8 T255. It also
compares omitted-GRID legacy GL with explicit GL bit for bit. It uses the
worker ABI for correctness; its incidental timing samples are not analyzed.
`analytic-controls.json` contains ten configurations and forty operation /
backend records. All passed. T255 uses the existing closed-form test's
4e-12 map / 2e-9 coefficient absolute limits; small cases use 3e-12.

`adjoint-cancellation.json` rechecks the T214 CC full-order fixture associated
with the largest saved synthesis-adjoint relative-to-dot residual. The
temporary Rust test writes S(a), S*(map), A(map) and A*(a); Python uses
long-double dot products. Harmonic dots weight m>0 by two. The normwise
denominator is `||S(a)|| ||map|| + ||a||_H ||S*(map)||_H`, with the analogous
expression for analysis. No existing tolerance was relaxed. The result
explains why a small relative-to-dot residual can look large under severe
cancellation; it is not an exhaustive adjoint proof.

`spin1-transition-validation.json` compares complete CC maps at T499/T500/T511
against uninstrumented pinned DUCC, using the shared deterministic splitmix
fixture in the diagnostic Rust profile and `spin_route_driver.cpp`. These
are numerical controls, with maximum relative L2 1.63e-13, not timing tests.
The spin counter build changes scalar loop code generation slightly;
instrumented/clean DUCC relative L2 is <=6.98e-16 and is recorded explicitly.

The preserved maps can be checked again without launching workers:

```sh
python3 reports/data/f64-ducc-independent-review-2026-10-22/validate_spin_transition.py \
  --directory $REVIEW_TMP/ducc-cc-independent-stage-20261022 --read-existing \
  --output $REVIEW_TMP/spin1-transition-validation.json
```

To regenerate maps, omit `--read-existing` and provide `--rust-test`,
`--ducc-clean` and `--ducc-counter` paths built as described below. This
runs three numerical configurations, not a timing sweep.

**Temporary diagnostic sources**

Apply patches only in independent local source copies. The production branch
does not contain these changes in `src/`.

- `rust-scalar-stage.patch` is the timing-only source change: it adds CC
  selection to two existing scalar profile helpers and counts analysis seed
  routes in a separate untimed call.
- `ducc-scalar-stage.patch` brackets the three CC analysis stages and two
  synthesis stages with steady-clock timers. Enable output with
  `RUSTSHT_REVIEW_STAGE=1`.
- `rust-diagnostics.patch` contains the later frequency probes, the T214
  output-dump test, and the optional spin profile output dump. Scalar
  synthesis counters and group timers are disabled unless their dedicated
  ignored test enables them.
- `ducc-diagnostics.patch` includes the timers plus atomic counters for
  scalar/direct-spin entry, calculation and lane counts. Enable the latter
  with `RUSTSHT_REVIEW_SPIN_ROUTE=1`. Their timings must not be used as an
  uninstrumented benchmark.

The initial scalar stage executable was expanded in place after capture;
its capture-time Rust executable hash was not recorded. The timing-only
patch preserves the exact source edits, and all frozen production executable
hashes are recorded. Later expanded diagnostic binaries have their own hashes.
Do not substitute the expanded counter build's hash for the timing build.

A reproducible Rust setup, using a path outside the shared checkout, is:

```sh
REVIEW_SRC="$REVIEW_TMP/rustsht-independent-copy"
mkdir -p "$REVIEW_SRC"
git archive 7b1629d4 | tar -x -C "$REVIEW_SRC"
git -C "$REVIEW_SRC" apply "$PWD/reports/data/f64-ducc-independent-review-2026-10-22/rust-diagnostics.patch"
CARGO_BUILD_JOBS=4 RUSTFLAGS='-C target-cpu=x86-64-v3' \
  cargo test --manifest-path "$REVIEW_SRC/Cargo.toml" --release --lib --no-run
```

For the scalar timing capture, apply `rust-scalar-stage.patch` instead of the
expanded patch. The actual local diagnostic project copied src/, examples/
and Cargo files and removed Python from its temporary workspace members;
production library code was otherwise identical. Archive extraction above
can keep the workspace because `--lib` selects the library tests.

The scalar profile commands use the resulting lib-test executable directly:

```sh
RAYON_NUM_THREADS=8 RUSTSHT_PROFILE_Q=8 RUSTSHT_PROFILE_GRID=CC \
RUSTSHT_PROFILE_LMAX=720 RUSTSHT_PROFILE_MMAX=720 \
RUSTSHT_PROFILE_NTHETA=722 RUSTSHT_PROFILE_NPHI=1458 \
RUSTSHT_PROFILE_BATCHES=30 RUSTSHT_PROFILE_BATCH_MS=30 \
taskset -c "$REVIEW_CPU_Q8" "$REVIEW_TEST_BINARY" \
  --exact transform::tests::plan14_analysis_phase_batch_profile --ignored --nocapture
```

Repeat in a fresh process; use q1/CPU 8, m180/nphi378, or the synthesis helper
as needed. `scalar-stage-batches.csv` retains sixty Rust per-cell observations
over two processes for each selected scalar cell. DUCC has thirty batches
over three processes per full-order cell. Both use the same dimensions and
normal-range fixture construction. Backend stage captures were not
interleaved; these are diagnostic stage comparisons, not a promotion A/B.
The initial two Rust full-analysis captures overlapped an unrelated smoke
job on the other CCD, which then exited. No samples were dropped.

For DUCC, copy the clean pinned src/ tree outside its checkout, apply the
appropriate diagnostic patch, then compile **only `sht.cc`** with the original
SHT command from the frozen build's `compile_commands.json`, replacing its
source/include paths and object output. Link that object with the unchanged
native driver and the other existing CMake objects. The original build is
`$REVIEW_TMP/rustsht-ducc-full-parity-42ce8ab5c5d7-g++`; its `CMakeFiles/ducc_full_transform_parity.dir/link.txt`
gives the exact link command. This preserves the numerical flags and avoids
rebuilding the native reference or touching its frozen executable.

The stage driver uses the unchanged scalar worker ABI. Capture stdout JSON
and stderr stage lines separately. The last `repetitions * len(batch_ms)`
stage lines are the measured calls; earlier lines are warmup/calibration.
The summary script averages stage times within each timed batch.

For spin dispatch, compile `spin_route_driver.cpp` with the strict driver
flags in `metadata.json`, linking either the original SHT object or the
counter object. Its arguments are `LMAX MMAX GRID Q OUTPUT_PATH`, with `-`
to suppress map output. For example:

```sh
DUCC0_NUM_THREADS=8 RUSTSHT_REVIEW_SPIN_ROUTE=1 \
  taskset -c "$REVIEW_CPU_Q8" $REVIEW_TMP/ducc-spin-route 720 180 CC 8 -
```

`ducc-spin-route-counts.txt` records nine one-call diagnostics, including
T499/T500/T511, GL/CC and T720/T1023 full/reduced controls. Counts of scalar
inner calls are twice the number of orders in the accelerated branch.
Direct-spin calculation counts are much smaller than direct-spin order
entry counts because polar order limits exclude most orders.

Current Rust spin profile commands use the existing ignored helper:

```sh
RUSTSHT_PROFILE_GRID=CC RUSTSHT_PROFILE_LMAX=1023 \
RUSTSHT_PROFILE_MMAX=1023 RUSTSHT_PROFILE_NPHI=2048 \
RUSTSHT_PROFILE_THREADS=8 RUSTSHT_PROFILE_REPETITIONS=5 \
taskset -c "$REVIEW_CPU_Q8" "$REVIEW_TEST_BINARY" \
  --exact spin_transform::tests::profile_spin1_standard_synthesis_phases --ignored --nocapture
```

The five calls are route/component diagnostics; they are not a final
short-kernel speed claim. `spin1-profile.csv` distinguishes wall phases and
overlapping worker sums. `spin1-workers.csv` records actual worker IDs and
task durations. `route-counts.json` preserves seed/block classifications.
`scalar-synthesis-routes.json` records the separate scalar underfill probe.
Run that probe with the expanded test binary and
`--exact transform::tests::independent_cc_synthesis_route_probe --ignored --nocapture`.

To regenerate diagnostic summaries from the preserved local logs:

```sh
python3 reports/data/f64-ducc-independent-review-2026-10-22/summarize_diagnostics.py \
  --logs /tmp --ducc $REVIEW_TMP/rustsht-cc-independent-stage-fixture \
  --adjoint $REVIEW_TMP/rustsht-cc-independent-adjoint \
  --output $REVIEW_TMP/independent-diagnostic-summary
```

The optional `--adjoint` directory regenerates the long-double cancellation
check from the preserved input/output arrays. All thirteen generated JSON/CSV
artifacts from the audit, diagnostic summary and transition reader were
compared with the archived evidence and matched.

**Review validation**

Production checks used separate Cargo target directories, preserving frozen
native workers:

- `cargo fmt --all -- --check`: passed.
- `cargo test --workspace --all-features --release`: passed; 270 library,
  4 extreme-scaling, 11 f32 and 1 workspace compatibility tests. 36 library
  and 1 f32 tests were ignored by their existing opt-in gates.
- `cargo test --workspace --no-default-features --release`: passed with the
  same counts. Python's dependency enables the library's default feature,
  so this workspace invocation alone does **not** establish serial coverage.
- `cargo test -p rustsht --no-default-features --release`: passed with true
  serial feature selection; 220 library tests, 4 extreme-scaling, 11 f32
  and 1 workspace compatibility test, with 17 library / 1 f32 opt-in tests
  ignored.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
  passed. No binding or Python implementation changed.
- Eleven focused CC tests passed in the temporary diagnostic project,
  including the T255 sectoral closed form, minimum even longitude, no-initial-
  resample geometry, and scalar adjoints. Existing spin tests were included
  in the production suite; T499/T500/T511 also received the independent
  pinned-DUCC numerical checks above.

The existing paused Plan 17 state, guarded paired odd-FFT scope and owner-
approved pyspharm integration were retained. No full sweep was restarted,
no subagent was spawned, and no production optimization was made in this review.
