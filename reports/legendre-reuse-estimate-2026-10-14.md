# Caller-provided Legendre reuse estimates

## Decision

Keep `ShtPlan::new` and its automatic policy unchanged. Add the opt-in
`ShtPlan::new_with_legendre_reuse_estimate` constructor for applications that
know their expected table-eligible STANDARD transform reuse. The estimate can
admit a compact north-hemisphere table on a cold plan only in measured GL f64
cells whose estimate reaches a conservative break-even. Unmeasured cold cells
retain recurrence; after an eligible operation initializes the shared table,
a positive estimate can use it where the current operation policy selects it. A caller can set an execution direction to `Some(0)` to keep it on recurrence. The policy does not calibrate, consume estimates,
change numerical operators, or allocate a table at plan construction. The fields group table-eligible calls by execution direction. For scalar
plans, STANDARD analysis and its synthesis adjoint can both use the
map-to-coefficient table path. For spin-1, STANDARD analysis can use that
table, but adjoint synthesis currently has no ring weights and stays on the
general path. Callers must estimate only calls that can execute the table
path; do not count spin-1 adjoint synthesis toward map-to-coefficient table
reuse. Coefficient-to-map covers table-eligible synthesis, including the
adjoint of analysis where that operation reaches the table-backed synthesis
path. The executing Rayon pool is queried at transform time. Plan construction pool
size is not retained as a selection input. A built table is shared by both
directions; after its setup cost has already been paid, a positive estimate
may use it when the existing operation policy chooses the table. `Some(0)`
continues to force recurrence for that direction even if the shared table was
built by the other direction.

## Strategy and eligibility

The candidate algorithms are the DUCC-derived on-the-fly recurrence and the
existing compact triangular table of northern-hemisphere basis values. The
table is reflected using existing scalar or spin-1 parity rules; it does not
materialize a dense full-sphere matrix. Current production eligibility is
limited to the measured f64 GL scalar and spin-1 STANDARD paths, existing
band and grid limits, safe input ranges, and the existing 34 MiB payload cap. The table remains lazily initialized. f32, CC/F1, unsupported operation
modes, unmeasured estimated-reuse cells, unsafe values, and out-of-range grids
keep their current recurrence or specialized fallback. This is a small caller-supplied hint, not a plan-wide promise that all
operators benefit equally. It is checked independently for each execution
direction. `None` delegates to the existing automatic steady-state policy. Before table initialization, finite estimates are accepted only where the
lookup has paired cold/warm measurements for the exact shape, spin, operation,
and executing pool. No interpolation is made across bandlimits, oversampling
ratios, or thread counts. SHTns runtime selection and the read-only `sht_bench` results motivated
measuring setup cost and steady-state speed separately. The useful transfer is
that algorithm choice depends on workload shape and reuse. SHTns-specific
initialization, storage conventions, and implicit machine calibration were
not copied. The broader strategy audit, grid matrix, SHTns comparison, and
Torch-harmonics memory comparison are in the
[adaptive selector report](adaptive-legendre-strategy-2026-10-14.md), the
[latest-head results](adaptive-legendre-latest-head-results-2026-10-14.md),
and the [over-resolved GL follow-up](overresolved-gl-legendre-table-2026-10-14.md).

## Paired cold and warm measurements

The current-source comparison used `examples/m1b_bench.rs`, one release
binary, identical deterministic inputs, reused plans and workspaces, one
warm-up, and five timed transforms per process. Recurrence and table processes
were run in paired order with CPU affinity. The host was shared; process
ranges indicate variability but are not confidence intervals. The table
candidate was selected using a sufficiently high reuse estimate. The T85 and
spin-1 T159 q4 estimates in the captured commands reflect earlier candidate
thresholds; final thresholds below are more conservative than the measured
crossovers. Kernel implementations and measured data were unchanged by later
threshold and API-documentation corrections.

| Case | Exact grid; spin; pool; operation | Recurrence first / warm (ms) | Table first / warm (ms) | Warm crossover; min total calls | Selected minimum |
| --- | --- | ---: | ---: | ---: | ---: |
| Scalar T127 | GL `128x257`, full mmax; spin 0; q8; synthesis | `0.468 / 0.370` (`0.421–0.518`, `0.369–0.391`) | `4.344 / 0.324` (`4.230–4.514`, `0.313–0.329`) | `85.3`; 86 | 90 |
| Scalar T85 atmospheric | GL `128x256`, full mmax; spin 0; q8; synthesis | `0.162 / 0.165` (`0.156–0.192`, `0.159–0.182`) | `2.069 / 0.136` (`1.916–2.137`, `0.129–0.170`) | `66.8`; 67 | 70 |
| Spin-1 T159 | GL `160x321`, full mmax; spin 1; q1; synthesis | `5.456 / 5.064` (`5.313–8.688`, `5.042–5.124`) | `12.611 / 4.824` (`12.389–12.797`, `4.803–4.869`) | `30.8`; 31 | 32 |
| Spin-1 T159 atmospheric | GL `320x640`, full mmax; spin 1; q4; analysis | `2.495 / 2.529` (`2.406–3.090`, `2.175–3.311`) | `17.551 / 1.928` (`17.428–19.830`, `1.850–2.739`) | `26.1`; 27 | 30 |

The crossover uses total transform calls, including the first call:

```text
N* = 1 + (table_first - recurrence_first)
         / (recurrence_warm - table_warm)
```

The integer minimum profitable count is `ceil(N*)`. The selected minimum is
rounded upward beyond that value to retain margin against observed process
variation. Scalar T127 q8 crosses at 86 calls and selects at 90; spin-1 T159
atmospheric q4 crosses at 27 and selects at 30. These thresholds are not
extrapolated to other cells.

### Total elapsed time over repeated transforms

Values are recurrence/table milliseconds using the measured first call plus
`N-1` warm calls.

| Calls | Scalar T127 q8 S | Scalar T85 atmospheric q8 S | Spin-1 T159 q1 S | Spin-1 T159 atmospheric q4 A |
| ---: | ---: | ---: | ---: | ---: |
| 1 | `0.468 / 4.344` | `0.162 / 2.069` | `5.456 / 12.611` | `2.495 / 17.551` |
| 5 | `1.948 / 5.640` | `0.822 / 2.613` | `25.712 / 31.907` | `12.611 / 25.263` |
| 10 | `3.798 / 7.260` | `1.647 / 3.293` | `51.032 / 56.027` | `25.256 / 34.903` |
| 25 | `9.348 / 12.120` | `4.122 / 5.333` | `126.992 / 128.387` | `63.191 / 63.823` |
| 50 | `18.598 / 20.220` | `8.247 / 8.733` | `253.592 / 248.987` | `126.416 / 112.023` |
| 100 | `37.098 / 36.420` | `16.497 / 15.533` | `506.792 / 490.187` | `252.866 / 208.423` |
| 1,000 | `370.098 / 328.020` | `164.997 / 137.933` | `5,064.392 / 4,831.787` | `2,528.966 / 1,943.623` |

Plan construction remained about `0.006–0.022 ms` in these runs. Workspace
allocation was separately reported, about `0.2–1.5 ms` depending on geometry
and pool; both strategies pay that cost. First-call timing includes lazy
table creation and the transform. Creation cost is estimated as the matched
difference from recurrence first-call timing; the builder was not isolated. Table payloads are calculated from the current triangular storage layout; RSS
was not measured in this paired series:

| Case | Table payload |
| --- | ---: |
| Scalar T127 exact GL | `4.03 MiB` |
| Scalar T85 atmospheric GL | `1.83 MiB` |
| Spin-1 T159 exact GL, including the `lmax+1` extension | `7.96 MiB` |
| Spin-1 T159 atmospheric GL, including the `lmax+1` extension | `15.92 MiB` |

All remain below the existing `34 MiB` maximum. These are retained payload
estimates, not process peak-memory measurements.

## Reproduction

Environment: x86-64 CPU (host topology omitted), Linux, x86-64 AVX2/FMA; Rust `1.99.0` (`b940084d7`), LLVM
`23.1.1`; release `opt-level=3`, 16 codegen units, LTO off, runtime Pulp
dispatch, no `target-cpu=native`. Installed DUCC was `ducc0==0.41.0`, source
revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. DUCC was not part of the
reuse timing pairs because this change selects between two RustSHT paths;
DUCC numerical comparisons are in the linked selector reports. Representative commands:

```sh
RUSTSHT_LEGENDRE_REUSE_ESTIMATE=90,- taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" \
  target/release/examples/m1b_bench 0 127 8 synthesis GL STANDARD 128 257 127
RUSTSHT_LEGENDRE_REUSE_ESTIMATE=1,- taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" \
  target/release/examples/m1b_bench 0 127 8 synthesis GL STANDARD 128 257 127
RUSTSHT_LEGENDRE_REUSE_ESTIMATE=-,29 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" \
  target/release/examples/m1b_bench 1 159 4 analysis GL STANDARD 320 640 159
RUSTSHT_LEGENDRE_REUSE_ESTIMATE=-,28 taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" \
  target/release/examples/m1b_bench 1 159 4 analysis GL STANDARD 320 640 159
```

Raw output has every measured command and sample in
[`paired-process-samples.txt`](data/legendre-reuse-estimate-2026-10-14/paired-process-samples.txt);
calculations are in
[`paired-process-summary.csv`](data/legendre-reuse-estimate-2026-10-14/paired-process-summary.csv). The release executable predates final threshold corrections, but the kernels,
inputs, and table/recurrence implementations match. Focused tests cover the
corrected selection boundaries.

## Correctness and review

The API change does not alter kernel arithmetic. Existing table-versus-
recurrence tests compare all scalar operators and spin-1 conversions;
analytic, pinned-DUCC and adjoint tests cover numerical behavior. Current DUCC
checks recorded in the preceding reports include scalar T127 synthesis
relative-L2 error `1.283e-14`, spin-1 T159 synthesis `5.210e-14`, and spin-1
T255 analysis `2.048e-14`. Reuse-specific tests cover measured threshold
boundaries, a Rayon pool different from the construction context,
execution-direction mapping, shared lazy initialization, `Some(0)` and the
fact that spin-1 adjoint synthesis does not count as a table-eligible call. No tolerance or scientific convention changed. The read-only Sol High review initially found incorrect mathematical-operator
naming and total-call thresholds that omitted the first-call `+1`. The API
was renamed to coefficient-to-map/map-to-coefficient, adjoint routing was
tested, and thresholds were moved conservatively above the corrected
crossover. The follow-up review found that spin-1 adjoint synthesis cannot
enter the analysis table path and that two Rust docs omitted the initialized-
table exception. The estimate contract now counts only calls that can execute
the table path for that plan and grid, and the Rust docs state the shared-table
exception. No code was changed by either review. The measurements cover four cells, not the full recurrence/table matrix. They
establish only the listed reuse thresholds. Pool contention and frequency
variation affect ranges. No RSS series was captured here. The broader grid,
precision, numerical and DUCC matrix remains in the linked reports. Unknown or
unmeasured workloads keep the existing deterministic policy. The API is `LegendreReuseEstimate` plus
`ShtPlan::new_with_legendre_reuse_estimate`. `ShtPlan::new` is unchanged. The
benchmark environment variable is read only by `examples/m1b_bench.rs`, not
by the library. The lookup returns no threshold for unmeasured combinations. Validation passed after the final documentation and regression changes:
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `cargo test --workspace --all-features`
(160 Rust and 10 f32 tests passed; 9 ignored),
`cargo test -p rustsht --no-default-features` (132 Rust and 10 f32 tests
passed; 7 ignored), and `uv run pytest python/tests` (20 passed). The full
Rust suites include spin-3/4 regression coverage. The focused selector tests
also passed independently after the last code change. The validation record is
in [`plans/05-validation-and-milestones.md`](../plans/05-validation-and-milestones.md).
