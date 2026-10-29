# Direct f32 spectra into the shared f64 scalar-analysis recurrence

## Decision

Keep this route test-only. It removes the f32-spectrum to f64-spectrum
promotion buffer and improves the promoted-spectrum parent in q8, but misses
pinned native-f32 DUCC in three of four GL T720 cells. Reduced q8 has a paired
median below DUCC, but its 95% interval crosses parity. The public f32 API has
no production analysis dispatch for this candidate, and the candidate did not
receive the independent numerical matrix required for promotion.

The exact Rust parent is `1bd4bd274cacda38f87cc741c2d4f20342107b8f`.
`parent.patch.gz` adds the test-only f32-FFT/f64-recurrence harness;
`candidate.patch.gz` adds the direct f32-spectrum view to that harness. The
candidate keeps the existing shared f64 reflected scalar recurrence and
accumulation. Every timed candidate call includes the f32 longitude FFT,
on-demand f32-to-f64 widening during spectrum gathers, f64 recurrence and
accumulation, and Complex64-to-Complex32 writeback. It does not materialize a
converted f64 spectrum. Plans and workspaces are created and prepared once per
worker, then reused across warm and timed calls. The test harness still
reserves an f64 workspace spectrum that the direct route does not use.

## Corrected v3 whole-call comparison

The first pushed version of this package accidentally recorded q1 extensions
without `RUSTFLAGS`, mixing generic and x86-64-v3 binaries. Those rows and
profiles were invalidated and replaced here. The complete four-cell screen
was rerun with the exact v3 flags below; all rows in `paired-results.json.gz`
now use those matched binaries.

Each cell has 15 interleaved parent/candidate/DUCC pairs. Times are medians in
milliseconds. Ratios are paired candidate/parent and candidate/DUCC medians;
95% percentile intervals resample the 15 paired ratios 50,000 times (seed
9062029 per cell and metric). Raw pairs and outputs are preserved in the
compressed results file; summarized statistics are in
`paired-statistics.json`.

| GL T720 f32 scalar analysis | Parent / candidate / DUCC ms | Candidate/parent [95% CI] | Candidate/DUCC [95% CI] | Status |
|---|---:|---:|---:|---|
| Reduced q1 | 6.048 / 6.024 / 5.746 | 0.9925 [0.9193, 1.0048] | 1.0490 [1.0355, 1.0613] | **GAP** |
| Reduced q8 | 0.966 / 0.872 / 0.897 | 0.9060 [0.8860, 0.9511] | 0.9727 [0.9643, 1.0248] | **INCONCLUSIVE at parity** |
| Full q1 | 14.618 / 14.617 / 14.139 | 0.9943 [0.9921, 1.0009] | 1.0346 [1.0306, 1.0385] | **GAP** |
| Full q8 | 2.473 / 2.142 / 2.053 | 0.8844 [0.8360, 0.9401] | 1.0414 [1.0298, 1.1263] | **GAP** |

The direct route is 9.4% faster than its promoted-spectrum parent in reduced
q8 and 11.6% faster in full q8 by paired median. Its q1 medians are effectively
neutral. Reduced-q1 Rust measurements had several timing outliers, reflected
in the wide parent-relative interval; the DUCC-relative interval remains
above 1.0. Candidate and parent output bytes were identical in every pair.
Candidate-vs-DUCC relative-L2 / maximum absolute errors were
`1.319e-7 / 1.397e-9` reduced and `1.432e-7 / 6.985e-10` full. These are
cross-implementation diagnostics, not an independent accuracy proof. Every
worker reported finite values and repeat-call/poisoned-workspace reset success.

## Native-f32 adjacent controls

All four native-f32 public scalar-analysis controls were rerun for 15 pairs
with the same v3 Rust build. The direct-spectrum patch changes no native route;
outputs were bitwise identical in every pair.

| GL T720 native-f32 scalar analysis | Parent / candidate ms | Candidate/parent [95% CI] | Status |
|---|---:|---:|---|
| Reduced q1 | 138.999 / 139.569 | 1.0033 [0.9995, 1.0075] | **INCONCLUSIVE; effectively neutral** |
| Reduced q8 | 18.691 / 18.716 | 0.9969 [0.9902, 1.0125] | **INCONCLUSIVE; effectively neutral** |
| Full q1 | 456.880 / 456.036 | 1.0051 [0.9908, 1.0090] | **INCONCLUSIVE; effectively neutral** |
| Full q8 | 59.634 / 59.617 | 0.9994 [0.9980, 1.0036] | **INCONCLUSIVE; effectively neutral** |

These controls use the native route and establish no repeatable adjacent effect
from adding the isolated candidate hooks. Protected f64 T1023 spin-1 synthesis
was not retimed because no production code changed.

## Stage profile, workspace, and allocations

A separate 30-call x86-64-v3 stage profile used the direct route, with no
spectrum-promotion stage. Stage medians in milliseconds were:

| Cell | Whole call | f32 FFT | Promotion | f64 recurrence and gather | f64-to-f32 writeback | Peak RSS |
|---|---:|---:|---:|---:|---:|---:|
| Reduced q1 | 5.982 | 0.634 | 0 | 5.311 | 0.034 | 27,020 KiB |
| Reduced q8 | 0.894 | 0.104 | 0 | 0.747 | 0.038 | 31,760 KiB |
| Full q1 | 14.597 | 2.586 | 0 | 11.927 | 0.083 | 76,660 KiB |
| Full q8 | 2.139 | 0.377 | 0 | 1.680 | 0.086 | 84,504 KiB |

These instrumented stage medians are diagnostic; paired whole-call timings in
the previous tables determine the performance status. Recurrence and gather
remain about 79–89% of profiled work, depending on the cell.

The test-only capacity audit sums reusable workspace vector capacities and
FFT scratch, separately from the reusable f64 coefficient scratch and public
output buffer. The f64 workspace includes its full spectrum buffer even though
the direct route leaves it unused. Plan memory and the input map are outside
these workspace totals.

| Cell | f32 workspace bytes | f64 workspace bytes | f64 coefficient scratch | f32 output buffer | Whole-call peak RSS |
|---|---:|---:|---:|---:|---:|
| Reduced q1 | 10,669,210 | 4,438,848 | 1,827,376 | 913,688 | 27,020 KiB |
| Reduced q8 | 11,107,034 | 4,661,760 | 1,827,376 | 913,688 | 31,760 KiB |
| Full q1 | 37,968,370 | 16,927,968 | 4,164,496 | 2,082,248 | 76,660 KiB |
| Full q8 | 38,787,442 | 17,463,008 | 4,164,496 | 2,082,248 | 84,504 KiB |

A separate test-only global-allocator probe counted allocations during 30
warmed calls. q1 allocated zero bytes on every call. q8 allocated 4,344 bytes
once per reduced call and 17,304 bytes once per full call; the last of 30 calls
in each cell made one additional 1,520-byte allocation. The same pattern
appeared in the promoted f64-recurrence and native-f32 routes, so it is not
specific to the direct-spectrum view. Its call site remains unattributed. The
allocator probes used distinct instrumented test binaries and their timings
are not used in the paired performance tables. Their hashes are in
`memory-allocation-diagnostic-binary.txt` and
`shared-route-allocation-control-binary.txt`; raw profiles and allocation
counts are in `stage-profiles.json`, `workspace-alloc-diagnostics.json`, and
`shared-route-allocation-controls.json`. The reproducible diagnostic patch and
runner are `workspace-allocation-diagnostic.patch.gz` and
`benchmark-workspace-alloc-diagnostic.py`.

## Provenance

- Rust source parent: `1bd4bd274cacda38f87cc741c2d4f20342107b8f`.
- Parent patch SHA-256: `7b796de7b4192a096b8d77c21a2b638306a5b2ae1a15455d973a9ae8b6a06264`.
- Candidate patch SHA-256: `575fa7c157a58d105f29311ccff7f72749496a72909cdb4c4ff16553854c08af`.
- Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Timed Rust parent binary: `f2a1201b74b1b9c13cd61df546ea6b3339b766a6a7fe0ecfe283cf67d61c9d97`.
- Timed Rust candidate binary: `c71034d205a31241771fab8f62139134af71b43f2c93f88094596d2efb3a53de`.
- DUCC worker: `acc82387af2c42aed53b25678e3872157ed3fb6e5d5874322badc472a1d35d5d`.
- Rust: rustc 1.99.1 / LLVM 23.1.1, `--locked --release --features parallel`,
  `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.
- DUCC: GCC 15.2, SHT `-O3 -ffast-math -ffp-contract=fast
  -march=x86-64-v3`; driver `-fno-fast-math`.
- Host: AMD Ryzen 9 5950X, AVX2/FMA. Each worker pins its q1/q8 affinity to
  `range(threads)`. The test process builds the plan and prepared workspace
  once, warms twice, then takes seven calls per pair. DUCC uses its matching
  warmed worker with a 25 ms batch. Setup is outside both timed regions.
- Grid: GL, `ntheta=721`, `phi0=0.23`; full `mmax=720,nphi=1458`, reduced
  `mmax=180,nphi=378`. Deterministic f32 fixture hashes and sizes are in
  `fixture-sha256.json` and `fixture-manifest.json`.

## Validation and next experiment

The paired workers passed finite-output, bitwise-parent-equivalence,
repeat-call and poisoned-workspace-reset checks. `cargo fmt --all -- --check`
passed in the isolated source tree. Independent analytic modes, adjoints,
pole/sectoral controls, exceptional values, SIMD tails, and representative
smaller/larger bandlimits were not run because the candidate misses the
performance gate and remains test-only. No production code is changed.

Next, screen the shared scalar-analysis tile width at 512 versus 256 using
this direct-spectrum route, all four T720 q1/q8 cells, native-f32 analysis
controls, and the protected f64 T1023 spin-1 synthesis control. The tile
candidate must use the same v3 flags and retain this package's exact parent,
fixture and DUCC provenance. M6 remains open.
