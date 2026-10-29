# Stage B — complete f32 I/O with shared f64 internals

## Decision

This isolated public-call adapter passes the whole-transform DUCC target for all
four GL T720 scalar-synthesis cells and for full-mmax spin-1 synthesis q1. It
is not a production change and does not establish f32-family parity: the other
11 of the 16 required scalar/spin cells remain GAP. The measured scalar
analysis adapter is slower than DUCC, and f64 spin-1 execution does not use the
high-bandlimit reflected route needed to meet most spin cells.

A reusable production backend is justified first for the measured scalar
synthesis family, with a conservative native-f32 fallback for subnormal,
extreme, or nonfinite values. The route must not be generalized to other
operations based on this result. Native f32 and f64-internal edge results
currently differ, so route guards and fallback correctness are a promotion
requirement.

## Provenance and timing contract

- Rust parent/source: `fdae803a01273ac7e4e941936f022fa11e770df5`.
- Candidate: isolated f32-I/O/f64-internal worker example, not a production
  patch. V2 source SHA-256:
  `b6e740da8f0086a5d521bddc1d8d6700edbc57ee36d4fd5397fb67253b28d5e5`.
  It constructs one f64 plan/workspace per worker and preallocates all
  conversion and output buffers. For q>1 it converts in Rayon chunks of 8192;
  q1 conversion is serial. The same shared f64 ShtPlan algorithms execute the
  operation.
- DUCC: `0.42.0`, commit
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, using its public native-f32
  interface.
- Compiler: rustc 1.99.1, LLVM 23.1.1; Rust flags
  `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. DUCC used GCC 15.2 with
  `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; driver
  `-fno-fast-math`.
- Host: AMD Ryzen 9 5950X, x86-64-v3 / AVX2+FMA; 8-lane f32 and 4-lane f64
  Pulp dispatch. worker CPU set q1 `[0]`; q8 `[0,2,4,6,8,10,12,14]`.
- Grid: GL, `ntheta=721`, `phi0=0.23`; full `mmax=720,nphi=1458`, reduced
  `mmax=180,nphi=378`. Each cell has 15 interleaved fresh-process
  parent/internal/DUCC triads. Plans/workspaces and their first warm-up are
  outside timed batches; each timed candidate call includes input promotion,
  the public f64 transform, output narrowing/writeback, copies, and FFT.
  q1/q8 and full/reduced remain separate.
- Fixtures are deterministic outputs of the saved
  `benchmark_ducc_f32_native.py::make_fixture`; their binary inputs are saved
  under `fixtures/` and checked against all per-cell SHA-256 records. Per-pair timing, output metrics, and worker checks are in `pair-data/`; transient host pressure and frequency snapshots are omitted.

The CSV contains every cell's Rust native-f32, candidate f32-I/O/f64-internal,
and DUCC f32 median; paired candidate/DUCC ratio and bootstrap 95% interval;
maximum relative-L2/absolute error versus DUCC; reusable conversion-buffer
bytes; fixture hashes; worker checks; and status. PASS means the paired
interval's upper endpoint is below 1 and all workers passed finite,
repeat-consistency, and poisoned-output reset checks. GAP means the lower
endpoint is above 1. No cell is averaged into another.

## Required 16-cell matrix

Times are milliseconds. Ratios are paired median candidate/DUCC with the
95% bootstrap interval. Error is candidate versus DUCC f32 output.

| Cell | Native f32 | f64 internals | DUCC f32 | Ratio [95% interval] | Max rel-L2 | Status |
|---|---:|---:|---:|---:|---:|---|
| s0 synth full q1 | 641.339 | 15.040 | 18.902 | 0.797 [0.791, 0.801] | 3.89e-8 | PASS |
| s0 synth full q8 | 98.183 | 2.629 | 2.838 | 0.927 [0.920, 0.951] | 3.89e-8 | PASS |
| s0 synth reduced q1 | 190.225 | 6.205 | 8.317 | 0.746 [0.742, 0.748] | 3.88e-8 | PASS |
| s0 synth reduced q8 | 54.588 | 1.163 | 1.314 | 0.849 [0.819, 0.915] | 3.88e-8 | PASS |
| s0 analysis full q1 | 453.143 | 15.334 | 14.064 | 1.094 [1.087, 1.097] | 3.90e-8 | GAP |
| s0 analysis full q8 | 60.005 | 2.678 | 2.274 | 1.179 [1.165, 1.211] | 3.90e-8 | GAP |
| s0 analysis reduced q1 | 139.936 | 6.081 | 5.774 | 1.052 [1.048, 1.060] | 3.87e-8 | GAP |
| s0 analysis reduced q8 | 20.132 | 1.140 | 1.001 | 1.130 [1.102, 1.155] | 3.87e-8 | GAP |
| s1 synth full q1 | 1487.599 | 39.294 | 41.068 | 0.964 [0.945, 0.970] | 3.91e-8 | PASS |
| s1 synth full q8 | 273.640 | 8.219 | 5.962 | 1.370 [1.354, 1.416] | 3.91e-8 | GAP |
| s1 synth reduced q1 | 625.249 | 28.298 | 18.059 | 1.575 [1.564, 1.667] | 3.89e-8 | GAP |
| s1 synth reduced q8 | 125.539 | 3.231 | 2.673 | 1.189 [1.028, 1.224] | 3.89e-8 | GAP |
| s1 analysis full q1 | 1040.405 | 37.908 | 28.807 | 1.315 [1.302, 1.323] | 3.89e-8 | GAP |
| s1 analysis full q8 | 138.878 | 7.473 | 4.879 | 1.635 [1.554, 1.682] | 3.89e-8 | GAP |
| s1 analysis reduced q1 | 375.930 | 15.049 | 11.759 | 1.287 [1.271, 1.293] | 3.91e-8 | GAP |
| s1 analysis reduced q8 | 58.159 | 3.240 | 2.258 | 1.369 [1.281, 1.475] | 3.91e-8 | GAP |

All 16 cells used 15 paired comparisons, and all worker finite/repeat/reset
checks passed. q8 telemetry recorded substantial external CPU pressure during
parts of the matrix (up to about 11% `some avg10`) and varying frequencies.
The q8 reduced spin-synthesis interval is wider than other cells but remains
above 1.00. Full raw telemetry and pair ratios are retained; the status is GAP,
not a parity pass.

## Conversion, memory, and execution cost

The V2 worker preallocates reusable f64 inputs/outputs; the timed call includes
every conversion. Conversion-buffer storage was 12,574,240 bytes for scalar
full, 4,007,680 for scalar reduced, 25,148,480 for spin full, and 8,015,360
for spin reduced. Allocator snapshots measured total live plan/workspace plus
adapter storage of roughly 29.7–32.6 MB for scalar full, 8.6–10.0 MB for
scalar reduced, 42.3–46.0 MB for spin full, and 12.6–14.5 MB for spin
reduced, depending on q. Steady-state synthesis allocated nothing. At q8,
analysis performed one temporary allocation/deallocation per call: 17,304
bytes scalar full, 4,344 reduced, 28,840 spin full, and 7,240 reduced.

A representative separate q8 full scalar-synthesis stage profile measured
0.194 ms f32-to-f64 coefficient promotion, 2.767 ms f64 transform, and
0.280 ms output narrowing/writeback (3.246 ms total). This profile is
instrumented and not the acceptance timing; it shows that the transform,
not conversion, is the larger component. Phase profiles likewise put f64
scalar analysis recurrence plus FFT at about 2.2 ms + 0.75 ms q8, and spin-1
analysis at about 5.7 ms + 1.8 ms. The f64 transform itself remains the
principal blocker in those operations. The f64 spin-1 T720 synthesis route
recorded 118 general fallback blocks and zero paired-hemisphere successes;
the existing reflected route's current degree window excludes T720.

## Numerical evidence and production boundary

The saved q1/q8 validation worker passed independent analytic low scalar
modes, exact scalar synthesis/analysis adjoint identities, the analytic
sectoral `l=m=720` mode, small-grid DUCC E/B-to-Q/U sign values, and spin
synthesis and analysis adjoint identities. Analytic scalar T720 synthesis
relative-L2 error was `2.55e-8`; coefficient recovery relative-L2 was
`1.83e-8` to `2.19e-8`. Random full-transform outputs were finite, repeatable,
and reset correctly after poisoned outputs.

Native f32 and f64-internal behavior diverges at exceptional magnitudes:
min-subnormal scalar input produced 1,051,218 nonzero native synthesis samples
but all zeros after f64 compute/narrowing; min-normal and max-finite synthesis
changed every output bit; max-finite analysis changed finite/nonfinite
classification in 260,281 coefficient components. Infinity/NaN scalar
synthesis returned the same `NonRealFftEdge` error in both paths. These cases
must remain on the native path unless a separately proven guard establishes
safe output range. A production route must also detect narrowed subnormal or
nonfinite outputs and transactionally rerun the native operation. No tolerance
or existing native-f32 support was changed here.

## Reproduction files

- `results-16-cell.csv`: complete focused matrix.
- `pair-data/*.tar.gz`: raw paired summaries and per-worker telemetry.
- `fixtures/` and `fixtures-sha256.json`: exact deterministic f32 inputs.
- `sources/`: candidate adapter, validation/allocation/stage tools, and runner
  scripts.
- `validation-q1.log`, `validation-q8.log`, `allocation-all-ops.txt`, stage
  and phase logs: correctness and cost attribution.
- `SHA256SUMS.txt`: package checksums. Candidate/native/DUCC executable hashes
  are in `experiment-manifest.json`; binaries were transient and are not
  retained in the repository.
