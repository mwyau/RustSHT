# M6 CPU performance parity

Branch: `opt/sht-performance-parity`, based on integrated revision
`a918e80e6bd1488d738518c87d26deb4a9c11d0f`. The campaign follows
`ASTRA_INTEGRATED_REVIEW.md`; performance parity is unproven.

## Checkpoint state

The branch contains three production checkpoints: C01 workspace validation
(`c0e11ad`), C02 FFT range and scaling (`8df2ecf`), and C03 unsafe spin
conversion fallback (`197c3d3`). C01 and C03 are committed correctness fixes. C02 is committed as a numerical checkpoint, but its per-operation control
gate remains unresolved; its scalar T255 q1 synthesis and analysis controls
exceed the predeclared 2% threshold. P01 remains an uncommitted, unaccepted
candidate because its default-link spin-1 control exceeds the guard. The
precomputed-visitor P02 showed no material gain, and the separate P02
two-chain degree panel regressed the T1023 scalar synthesis target; both remain
outside production. The isolated M2 general-visitor candidate is also rejected
and remains outside production. The scalar M1 route-profile evidence is
committed; the new spin-1 profile is recorded below. Test-only counters,
benchmark harness changes, and numerical-plan edits remain uncommitted. The
campaign report and raw measurements are recorded in diagnostic evidence
checkpoints.

## C01: workspace order-prefix compatibility

A public GL scalar T511 reproducer used identical `512×1024` grids and
`mmax=0`/`511` plans. A low-order workspace passed the full-order plan's
`prepare_workspace(..., 2)`, then `analysis_into` panicked at the prefix
slice in `src/transform.rs:1472` (length two, requested end index 512). The workspace validator now requires at least `mmax+2` prefix entries in
parallel builds. Incompatible preparation and execution return
`ShtError::BufferLength` before output mutation or capacity changes. A larger
prefix can serve a reduced-order plan. Serial builds have no prefix allocation
and retain compatible cross-order execution. The public regression covers scalar T511 and spin-1 T255 at q1/q2/q3,
unchanged error outputs and worker allocation, reverse-order synthesis and
analysis, and comparison with a matching workspace. It uses analytic m=0
fields; no new DUCC comparison is required for the capacity check. Validation on Rust 1.99.0 (2026-10-17):

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo test --workspace --all-features`: 188 unit, 11 f32 and one public
  compatibility test passed; 13 unit and one f32 test ignored.
- `cargo test --no-default-features`: 150 unit, 11 f32 and one public
  compatibility test passed; seven unit and one f32 test ignored.
- `cargo test --release --test workspace_compatibility`: passed. Commands used `TMPDIR=<local-home-path> Existing numerical
comparisons in these suites identify DUCC 0.41.0, pinned source
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Python, explicit ignored high-L
and standalone FFT tests were not run for C01; bindings, numerical kernels,
and the standalone workspace did not change.

## C02: real FFT range and synthesis normalization

The baseline failed all three length-two extreme FFT tests: f64 forward
`[1e308,0]`, f64 normalized inverse bins `[1e308,1e308]`, and f32 forward
`[2e38,0]`. Scalar GL `1×8` monopoles with `a00=1e308` (f64) or `2e38`
(f32) produced alternating NaNs and infinities, although their constant maps
are finite. The candidate retains the ordinary normalized reconstruction arithmetic and
recomputes nonfinite reconstruction results with overflow-aware half-sums. Only an overflowing sum of finite operands is halved before addition. This
preserves the ordinary path's subnormal behavior. Synthesis now uses an
unnormalized inverse on Fourier-series amplitudes, eliminating the temporary
`nphi` multiplication. Recurrence, spin, derivative and hemisphere routes use this convention. The bounded f64 scalar table path retains its original scaled spectra and
normalized inverse; its existing input limit excludes extreme coefficients. CC/F1 adjoint analysis keeps its
normalized inverse. [The numerical plan](../plans/02-numerics-and-grids.md)
derives the four operators' scaling. Targeted debug checks passed: direct complex FFT and Hermitian sums,
length-two extreme cases, smallest-subnormal f64 DC values at lengths
1/2/3/4/5/8/31/32/33, extreme monopoles at longitude counts
1/2/3/8/9/32, and large-range controls for S/A/S*/A* on GL/CC/F1,
spins 0–4, f64/f32, nonzero `phi0`, and odd/even longitudes. The range
controls compare scaled complete arrays with ordinary execution at L4;
independent analytic and DUCC checks are supplied by the existing suite. Private pre-FFT tests now compare Fourier-series amplitudes instead of
`nphi`-scaled DFT bins, with unchanged tolerances. The first candidate passed full debug/release/serial suites, eight opt-in
high-L tests, formatting and Clippy, and 111 standalone FFT tests in both
debug and release. Its six-pair controls failed the predeclared per-operation
regression gate: scalar T255 q1 synthesis increased by 3.68% (paired-median
95% bootstrap interval 3.10%–4.39%). Analysis increased by 2.51% at that cell,
with additional slowdown signals in other analysis controls. These results
are preserved in `paired/summary.csv`; S+A totals do not replace the separate
S and A gates. A diagnostic build without the per-bin exceptional guard improved scalar
T255 q1 analysis by 4.35% relative to the guarded candidate across three
rotated triples. It still synthesized 2.55% slower than baseline. This
isolates a guard cost without establishing that it prevents vectorization:
both disassemblies use 128-bit packed complex arithmetic. The unsafe
diagnostic change was removed. The revised candidate uses a row-level
forward range classification and the original normalized inverse for the
bounded table path. It passed the 12 FFT unit tests, four extreme-range
integration tests, and the 194-test library suite in debug and release (13
tests ignored). The independent six-pair, per-operation screen against the
pre-C02 binary still found a repeatable scalar T255 q1 synthesis regression:
candidate/baseline median 1.03003, 95% interval 1.02166–1.03604, with all six
pairs above 1.02. Scalar T255 q1 analysis was 1.02069 (95% interval
1.01719–1.02564; four of six pairs above 1.02). CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Their raw samples were retained. The revised C02 candidate therefore
passes these numerical checks but does not pass the per-operation performance
control. It is not accepted. The summary, method, C02-only source patch, and
all 96 process outputs are in
[`reports/data/m6-c02/revised-validation/paired/`](data/m6-c02/revised-validation/paired/). Baseline timing artifacts are in [the C02 data directory](data/m6-c02/). The
private FFT adapter still inherits the FFT engine's range limits; these
checks do not prove arbitrary large-spectrum cancellation is overflow-free.

## C03: specialized spin fallback

Both reported conversion guard gaps are reproduced before FFT execution. The spin-1 hemisphere route at T319 produced infinity where the original
spin recurrence gave a finite value. The spin-2 scalar route at T500
produced NaN for finite E/B input with a finite general-reference value. The reproducer log is `target/m6-c03-reproduce.log`. The candidate extracts the existing general-spin order projection into a
private helper. Spin-1 hemisphere and spin-2 scalar synthesis check converted
coefficients and paired preparation before committing any order output. A
failure projects the original E/B coefficients with freshly initialized spin
factors. Hemisphere fallback covers both hemispheres and both channels. Spin-1 paired preparation is performed once per order before its latitude
blocks. No workspace arrays or public APIs are added. The C03 extreme release regressions and five existing focused regressions pass:
spin-1 hemisphere pairing, cached versus uncached coefficients, channel cases,
the explicit nonfinite-conversion fallback, and spin-2 mixed-polar blocks. The
extreme cases cover spin-1 T319/T511 cached and uncached conversions, 67-lane
blocks and both hemispheres, plus spin-2 T500/T511 E-only, B-only and E+iB
inputs. `cargo fmt --all -- --check` also passes. The checked fallback keeps
the original E/B rows and runs before order output is committed; the paired
preparation helper declines nonfinite prepared coefficients. Six alternating q1 process pairs compared C02+C03 against the C02-only
candidate at T511, GL STANDARD f64, full mmax, 512x1024 and phi0=0.37. Ratios
are candidate/baseline, separated by complete operation. Spin-1 synthesis was
1.02016 (95% paired bootstrap interval 1.00816–1.02561; three of six pairs
above 1.02); spin-1 analysis was 0.99292 [0.97984, 1.00546]. Spin-2 synthesis
was 1.00024 [0.98688, 1.00373], and spin-2 analysis was 0.97381
[0.96815, 0.97854]. Preflights found no competing build or benchmark process
and refused no run. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The ordinary
controls show no repeatable greater-than-2% signal in six pairs, but do not
accept C03. Raw output, time/RSS and preflight files are in
[`reports/data/m6-c03/ordinary-controls/`](data/m6-c03/ordinary-controls/). The two focused conversion fixtures do not separately establish a paired-
preparation failure when the conversion itself is finite. A clean six-pair spin-1 T511 q1 confirmation compared C02+C03 with the C02-only
binary. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Synthesis was 1.01685 (95% paired
bootstrap interval 1.00470–1.02254; two of six pairs above 1.02); analysis was
0.98692 [0.97148, 0.99866]. This confirmation does not show a repeatable
greater-than-2% synthesis regression, although its interval still crosses the
guard. Combined with the earlier pressure-affected screen, the ordinary C03
control remains inconclusive. Raw stdout, time/RSS and preflight files are in
[`reports/data/m6-c03/clean-confirmation/`](data/m6-c03/clean-confirmation/).

## M1 route costs and reference gap

The matched GL STANDARD f64 baseline uses DUCC 0.42.0 at source revision
`80388904b8cfc32400ba84af8a6a45aabfe60f86` and SHTns 3.7.5 at
`4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`. At T1023 q1, Rust scalar
synthesis measured 296.372 ms versus 49.555 ms for DUCC and 34.670 ms for the
documented SHTns adapter. Spin-1 synthesis measured 276.295 ms versus 121.291
ms for DUCC and 81.186 ms for SHTns. SHTns timings include input preservation
and E/B potential conversion. An additional matched T2047 scalar q1 run measured
Rust/DUCC/SHTns synthesis at 3355.273/370.438/269.622 ms and analysis at
1093.244/243.378/605.659 ms. Rust was 9.06x slower than DUCC and 12.44x
slower than SHTns for synthesis; for analysis it was 4.49x slower than DUCC
and 1.80x slower than SHTns. Its Rust-to-DUCC relative L2 errors were
3.015e-13 for synthesis and 1.747e-13 for analysis; SHTns-to-DUCC errors were
2.425e-13 and 1.866e-13. The matched T2047 spin-1 q1 medians were
4124.493/776.432/498.233 ms for synthesis and 1767.881/496.897/1158.457 ms
for analysis. Rust was 5.31x slower than DUCC and 8.28x slower than SHTns for
synthesis, and 3.56x and 1.53x slower for analysis. Rust-to-DUCC errors were
3.707e-13 and 1.729e-13; SHTns-to-DUCC errors were 3.403e-13 and 1.837e-13. The q1 scalar and spin-1 cells used 1.04 and 1.60 GiB peak RSS. Matched q8
T2047 references were then collected with the same source snapshot and
reference libraries:

| Spin | Synthesis Rust / DUCC / SHTns (ms) | Rust/DUCC | Analysis Rust / DUCC / SHTns (ms) | Rust/DUCC | Peak RSS |
|---:|---:|---:|---:|---:|---:|
| 0 | 818.883 / 58.599 / 71.983 | 13.97x | 221.369 / 40.820 / 124.631 | 5.42x | 1016.0 MiB |
| 1 | 1027.308 / 126.510 / 138.667 | 8.12x | 358.881 / 81.650 / 242.638 | 4.40x | 1640.2 MiB |

These are 15-sample q8 baselines with full-array checks before timing; the Rust
relative-L2 errors versus DUCC were 3.015e-13/1.747e-13 for spin-0
synthesis/analysis and 3.707e-13/1.729e-13 for spin-1. The measurement binary
does not include P01. The scalar and spin-1 q1 cells and q8 extension
provenance, errors, raw samples, host snapshots and memory records are in
[`reports/data/m6-current-baseline/`](data/m6-current-baseline/). The q1 route profile reports per-worker CPU timer sums, not transform wall
time. Fused/visitor/transition sums were 7.421/26.408/0 ms at T511,
55.352/191.910/13.420 ms at T1023, and 362.784/1505.354/1181.839 ms at
T2047. Thus the visitor dominates at T511 and T1023 even though it handles
about 9–10% of committed route units. At T2047 the visitor and the shared
two-channel transition account for most profiled route work. Profiling
overhead changed q1 wall time by 0.6% at T1023 and −0.04% at T2047. q8 route
counts match the q1 totals, but workers are imbalanced at T2047 (maximum to
minimum active route timer ratio 9.53); q8 timer sums are descriptive. The
route tables and worker-level evidence are in
[`reports/data/m6-current-baseline/route-profile/`](data/m6-current-baseline/route-profile/). A separate test-only profile covered spin-1 synthesis at T511, T1023, and
T2047 with q1 and q8. It verified instrumented outputs against same-process
uninstrumented outputs bitwise; DUCC relative-L2 errors ranged from
`4.718e-14` to `3.707e-13`. At T2047, the per-worker CPU timer sums attributed
about 1.99 s (q1) and 2.05 s (q8) to the scalar visitor, and about 1.39 s and
1.42 s to the transition route. The q8 T511 profile also exercised the
hemisphere path; it was absent at T1023 and T2047. These timers are diagnostic
worker sums, not wall-time components, and do not imply performance acceptance. Per-worker counts, timers, hashes, preflights, raw output, and provenance are
in [`the spin-1 M1 profile`](data/m6-spin1-routes/summary.md).

### q8 T2047 scheduling-granularity screen (incomplete)

An isolated candidate changed only the exact full-mmax GL scalar T2047 q8
scheduling path from eight contiguous 256-ring groups to sixteen independent
128-ring groups, with separate per-group scratch and Rayon minimum grain one. It left recurrence arithmetic, route selection and guards unchanged. Focused
maps were bitwise equal to the source-matched baseline; each matched DUCC
0.42.0 at relative L2 error `3.015e-13`. Four of eight planned alternating
timed pairs completed. Their exploratory candidate/baseline synthesis ratios
were `0.83268`, `1.15892`, `0.84561`, and `1.18379`; the median was `1.00226`
with a four-pair percentile-bootstrap interval of `0.83268–1.18379`. In each
pair the second process was faster regardless of binary, so the partial sample
does not identify a scheduling effect. Pair 5 candidate was refused at the
CPU-pressure validity was assessed under the declared protocol; raw readings remain local.
eight-pair gate is incomplete and not met by the exploratory point estimate. No controls or route-profile rerun were started; the candidate is unaccepted. This incomplete scheduling experiment is separate from the rejected general
visitor candidate described under M2. All isolated source, binary hashes, raw outputs, time/RSS and preflight
records are in
[`reports/data/m6-m1-scheduling/`](data/m6-m1-scheduling/). The matched q8 T2047 reference run confirms that RustSHT remains 13.97x slower
than DUCC for scalar synthesis and 8.12x slower for spin-1 synthesis under
eight threads in this baseline.

## P01: scalar synthesis transition (unaccepted)

The scalar-only transition replaces eligible general visitor work and avoids
computing the unused second scalar channel. Six paired whole-transform runs
measured scalar q1 synthesis ratios of 0.6647 at T1023 and 0.7330 at T2047;
the T511 scalar control was 0.9880. The large-degree scalar gains align with
the route profile, which attributes 191.910 ms at T1023 to visitor work and
1505.354 ms to visitor work at T2047, plus 1181.839 ms in the shared
two-channel transition at T2047. An instrumented P01-only T1023 q1 profile, with P02 excluded, measured visitor
CPU timers decreasing from 191.910 to 69.752 ms and shared-transition timers
increasing from 13.420 to 39.293 ms. Fused-route time stayed near 55 ms; the
sum of the three route timers decreased from 260.682 to 163.448 ms (37.30%). The committed route-unit count was unchanged and the instrumented map matched
the uninstrumented map bitwise. This single diagnostic profile supports the
route-level P01 explanation; the paired native benchmark remains the timing
evidence. See [the P01-only route profile](data/m6-p01/route-profile/p01-only-summary.md)
and its retained source, binary hash, and raw output. The P01-only q8 profiles extend that route evidence to T1023 and T2047. The
total fused, visitor, and transition CPU timers changed from 285.548 to
196.492 ms at T1023 (−31.2%) and from 3197.412 to 2521.024 ms at T2047
(−21.1%); committed route-unit counts were unchanged, and each instrumented
map matched its uninstrumented output bitwise. The maximum-to-minimum active
worker timer ratio fell from 6.93x to 4.09x at T1023 and from 9.53x to 7.02x
at T2047, so imbalance remains, especially at T2047. These are single-run
diagnostic profiles, not acceptance measurements. The P01-only q8 summary,
exact source and executable hashes, raw logs, PSI snapshots, and time/RSS
records are in
[`reports/data/m6-p01/route-profile/p01-only-q8-summary.md`](data/m6-p01/route-profile/p01-only-q8-summary.md). A separate T2047 q8 lane-retry route-accounting test compared one
uninstrumented call with one call that collected per-worker route counters. The maps matched bitwise; the times were 512.470 and 512.947 ms (+0.09%). The test counted 1.734 billion
committed lane-pair units and 67,676 declined transitions. Worker route-timer
sums ranged from 77.4 to 431.5 ms. This single run measures instrumentation
overhead and route counts, not candidate-versus-baseline performance. The
outer launch requested 8 pinned CPU cores; the host CPU model was not recorded. Source, executable hash, raw output, timing, preflight, and limitations are in
[`the route-accounting diagnostic`](data/m6-p01/decline-diagnostic/summary.md). The default-link T511 spin-1 synthesis control has a repeatable greater-than-
2% screen. Across the initial and confirmation screens (12 paired ratios),
the median was 1.02073,
the 95% paired bootstrap interval was 1.01791–1.02761, and 7/12 pairs were
above 1.02. The point estimate exceeds the guard, while the interval crosses
it. Analysis did not show a slowdown. A diagnostic build with the
scalar transition call disabled ran spin-1 synthesis at 0.9882 relative to
the pre-P01 binary, but also lost the T1023 scalar gain (298.385 ms versus
296.153 ms for the pre-P01 baseline and 197.335 ms with P01 enabled). The
disabled binary text was 28,560 bytes smaller. Outlining the SIMD body
increased text by 24,496 bytes and lost the T1023 gain; outlining only the
outer wrapper kept the gain (ratio 0.6596) but measured spin-1 at 1.0235 with
an interval crossing 1.02 and elevated CPU pressure. These screens did not
separate code layout from source changes. A same-source link-layout comparison
is reported below. Evidence and source snapshots are in
[`reports/data/m6-p01/`](data/m6-p01/). A build-specific ordering experiment used the saved P01-only source patch and
excluded P02. Its initial `--section-ordering-file` attempt failed; the
retained successful build log does not include the exact linker invocation. The later-created `symbol-order.txt` and observed symbol movement are
consistent with a `--symbol-ordering-file` link, but do not prove which option
or file produced the retained ordered binary. `nm` shows the scalar dispatcher
at `0x2127f0` in the default binary and `0x79090` in the ordered binary; the
V4/V3 transition kernels moved from `0x1f8130`/`0x204fb0` to
`0x7a900`/`0x7c950`. This ordering was applied at the final link of the local
`m1b_bench` executable. It is not embedded in the RustSHT rlib and is not
guaranteed for downstream executable or extension links. Six alternating q1 synthesis pairs against the retained pre-P01 binary gave
an ordered/baseline ratio of 0.99061 [0.98131, 0.99295] for T511 spin-1 and
0.65974 [0.65738, 0.66357] for T1023 scalar. This screen clears the 2%
spin-1 control and retains a material scalar gain. One T1023 preflight at
CPU-pressure validity was assessed under the declared protocol; raw readings remain local.
missing process and the remaining pairs completed. Raw files and the refused
attempt are retained in
[`reports/data/m6-p01/link-order/`](data/m6-p01/link-order/). The complete ordered-binary matrix then covered six alternating pairs for
T511 scalar/spin-1 q1, T1023 scalar q1/q8, and T2047 scalar q1/q8, with both
synthesis and analysis. After excluding and rerunning a T2047 q1 pair that
overlapped external DUCC build work, and rerunning a second pair with a strong
timing anomaly adjacent to the same build, all 12 operation medians and
bootstrap upper bounds were below 1.02. T2047 q8 analysis was nearest the
guard at 1.00993 [1.00439, 1.01875]. Scalar synthesis ratios were 0.65845
[0.65754, 0.68422] at T1023 q1, 0.62971 [0.62663, 0.65412] at T1023 q8,
0.73957 [0.73325, 0.74639] at T2047 q1, and 0.68709 [0.68489, 0.68813] at
T2047 q8. The full matrix is summarized in
[`reports/data/m6-p01/link-order/acceptance/summary.md`](data/m6-p01/link-order/acceptance/summary.md),
with all raw samples, PSI/process snapshots, refused attempts, and replaced
runs retained alongside it.

## P01 ordered versus default final link

Six alternating q1 synthesis pairs compared two binaries built from the same
P01 source patch (SHA-256
`fff74a3059aec58741072b3eb1a96421c006ddcc4ba6847949098d17266faecf`). The
default-link binary SHA-256 is
`3388c0219a9d9ba214241763bd8749a35ef6e60bbe685ec779b842291ff6da2e`; the
ordered binary SHA-256 is
`22cb00437aa6cbe33222fc1a8140694c69ef8ff63694fbaa7310f50561b968f2`. Both
ran on one CPU core with GL STANDARD f64, full `mmax`, `phi0=0.37`, requested-only
output, and five warmed synthesis samples per fresh process. The ordered/default paired ratio was 0.96668 [0.96024, 0.97375] for T511
spin-1 synthesis (range 0.95980–0.97703), and 1.00025 [0.99767, 1.01304] for
T1023 scalar synthesis (range 0.99738–1.01926). All six spin-1 pair ratios
favored the ordered binary. The scalar result shows no measurable loss of the
P01 gain. Several five-sample process records contain high outliers; they are
included in the reported ranges and summaries. One initial T1023 ordered
CPU-pressure validity was assessed under the declared protocol; raw readings remain local.
checks, both sides of that pair were rerun. Preflight records show no compiler
or benchmark process during accepted runs. This same-source comparison shows that the spin-1 timing shift tracks final
text layout. It does not establish a particular instruction-level or
microarchitectural mechanism, or a portable default-link solution. This
isolates the current regression evidence to executable layout sensitivity;
the specific hardware effect remains unknown. The q8 route profiles cover
scalar synthesis and do not narrow the spin-1 cause. P01 remains unaccepted
until the complete default-link control matrix passes. The successful linker
command was not retained. The ordering was applied at the final link of the
local `m1b_bench` executable; the RustSHT rlib does not encode it, so the
ordered result is not a portable default-link solution. Raw output,
time/RSS, preflight snapshots,
scripts and summary are in
[`reports/data/m6-p01/link-order/causal-ab/`](data/m6-p01/link-order/causal-ab/).

## P01 source-order relocation screen

An isolated P01-only copy moved the scalar transition implementation to the
end of `scalar_paired.rs` and used the standard Rust 1.99.0 release link for
`m1b_bench`; no custom linker flags were set. The candidate ELF
(`23f2c95f…`) and default P01 comparison ELF (`3388c021…`) are retained under
`target/`. In `nm -nC` output, the V4 transition, V3 visitor closure, V3
transition and scalar dispatcher all shifted by the same `-0x36670`; the V3
visitor-to-transition gap remained `0xA590`, and the preceding hot scalar
block-to-V4 transition gap remained `0xC050`. Since source relocation did not
change relative symbol placement, the predeclared screen condition was not
met and no benchmark was run. The P01 source patch, build log, source hashes
and symbol listings are in
[`reports/data/m6-p01/function-layout/`](data/m6-p01/function-layout/). The measured visitor cost (191.910 ms at T1023 and 1505.354 ms at T2047 in
the q1 route profile) and the matched T1023 scalar gap (296.372 ms for Rust,
49.555 ms for DUCC and 34.670 ms for the documented SHTns adapter) support
keeping the scalar synthesis bottleneck as the next optimization target. P01
has a material scalar gain, but remains unaccepted because its default-link
spin-1 control exceeds the 2% guard and the same-source ordering result is
specific to a local final executable link. The codegen diagnosis below found
unchanged normalized instructions and function sizes for the spin-1 scalar
block, safe-lane helper and V3/V4 recurrence kernels across the baseline,
default P01 and ordered P01 binaries; it did not identify a source-level
mechanism for the layout effect. The original P01 candidate remains preserved
for review; no additional source-layout or code-sharing variant is selected
without a new measured hypothesis. The first follow-up general visitor candidate and P02 coefficient reuse are
screened and rejected below. Route timing identifies scalar synthesis work,
but does not itself establish a portable source change. Symbol and
disassembly evidence is in
[`reports/data/m6-p01/codegen-diagnosis/`](data/m6-p01/codegen-diagnosis/).

ordered-link P01 T511 spin-1 executables found nearly identical total dynamic
instruction references and last-level instruction misses. Its model reported
more L1 instruction misses and conditional branch mispredicts for the faster
microarchitectural mechanism. See

A bounded code-sharing screen targeted the added P01 text size by replacing
the scalar suffix's duplicate SIMD projection loop with one checked helper. Formatting and the release Rust library suite passed (197 tests, 14 ignored),
but the candidate `.text` was 1,504 bytes larger than default-link P01 and
26,768 bytes larger than baseline. It failed the size hypothesis and was not
benchmarked. The source patch, hashes, build log, size comparison, and test
output are retained in
[`reports/data/m6-p01/code-sharing-screen/`](data/m6-p01/code-sharing-screen/).

### P01 shared recurrence-step redesign (rejected)

A bounded source redesign factored the scaled-prefix recurrence update into an
out-of-line SIMD helper shared by scalar and paired transitions, keeping the
normalized scalar suffix inline. Six alternating T1023 scalar q1 synthesis
pairs measured 0.73059 [0.72670, 0.73164] relative to the pre-P01 binary. The CPU-pressure check failed the declared gate; exact transient readings remain local. A direct six-pair comparison with
the original default-link P01 binary measured 1.07780 [1.07426, 1.09262]. The shared helper replaced the recurrence step in both transition paths. The
paired transition is used by spin-1 synthesis, so this redesign adds an
out-of-line call to the spin-1 path at each degree step. The direct source
comparison and timings isolate that added shared-step cost; it is separate
from the original P01 final-link layout effect. Reject this redesign and stop
P01 source-order/layout variants. The T1023 scalar gain remains a screen, not
acceptance evidence. The full control matrix was not run after the T511
spin-1 failure. Source snapshots, tests, route profile, raw pairs, preflight
logs, and binary hashes are retained in
[`reports/data/m6-p01/shared-step-redesign/`](data/m6-p01/shared-step-redesign/). The ordered results pass the measured operation gates, but are specific to
the local benchmark executable. The original P01 candidate remains preserved
and unaccepted until the full default-link control matrix passes and a
portable source-level solution removes the spin-1 regression. The full suites
below ran with P01 and P02 together. Do not infer downstream consumer
performance from the ordered link.

## P02: reuse prepared scalar coefficients in visitor fallback

The scalar paired visitor fallback was recomputing the same even/odd degree
coefficients for every latitude block. The P02 screen reuses arrays prepared
once per order, preserving their degree-pair layout, arithmetic and scaled
visitor behavior. Low-degree analytic synthesis, pinned DUCC 0.41 GL samples,
and T255 mixed-synthesis DUCC samples pass in release. Formatting also passes. The two-hunk patch and all benchmark files are under
[`reports/data/m6-p02/precomputed-visitor/`](data/m6-p02/precomputed-visitor/). An initial six-pair comparison of the P01+P02 candidate against the retained
pre-P01 binary found scalar synthesis ratios of 0.65569 [0.64994, 0.66776] at
T1023 and 0.73163 [0.72944, 0.73523] at T2047. Those gains include P01 and do
not isolate P02. The same screen measured T511 spin-1 synthesis at 1.02770
[1.00506, 1.05212]. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. A second six-pair screen directly compared the saved P01-only and P01+P02
binaries, using synthesis-only processes. Median P02/P01 ratios were 1.00188
[0.99684, 1.00366] for T511 scalar, 0.99752 [0.98429, 1.00363] for the T511
spin-1 control, 1.00691 [1.00167, 1.04024] for T1023 scalar, and 0.99539
[0.98828, 1.00067] for T2047 scalar. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. P02 shows no material improvement at either target
degree and no greater-than-2% spin-1 regression in this isolated comparison. Reject P02 and retain its files as diagnostic evidence.

## M2 P02 parity panel: two SIMD chains (rejected)

The parity-panel screen targeted the fused ordinary scalar kernel. M1
attributed 55.352 ms of the T1023 q1 three-route total of 260.682 ms to this
route, compared with 191.910 ms to the visitor. The candidate interleaved two
SIMD latitude vectors and unrolled two paired-degree contributions. It used
the pre-P01 source and changed no transition or visitor code. Six low-pressure T1023 scalar synthesis pairs measured candidate/baseline
1.03358 [1.02877, 1.03524]. Four T511 scalar control pairs measured
1.03383 [1.03290, 1.03551]. The runner refused T511 scalar pair 5 at CPU PSI
CPU-pressure validity was assessed under the declared protocol; raw readings remain local.
rejected: the primary cell was 3.36% slower and the clean T511 scalar control
was 3.38% slower. The full control matrix was not run. Formatting, three fused-kernel checks, and three pinned DUCC 0.41 scalar
synthesis checks passed. The candidate source, raw pairs, process snapshots,
refusal, tests, and hashes are in
[`reports/data/m6-p02/parity-panel/`](data/m6-p02/parity-panel/). This is a
separate P02 experiment from coefficient reuse in the visitor.

## M2: guarded ordinary-product scalar visitor (rejected)

The isolated candidate started from production HEAD a50f700 and used the
paired visitor's normal-range status to classify ordinary coefficient
products once per degree-pair/block. It retained the existing scaled helpers
for unsafe products and scaled visits, and did not change coefficient
preparation, recurrence updates, or degree accumulation order. This target
came from the M1 profile: visitor timer sums were 26.408 ms at T511 and
191.910 ms at T1023 q1. Formatting and focused debug checks passed. The checks cover coefficient
exponent edges and intermediate fallbacks, pinned DUCC 0.41 scalar synthesis
at low degree and T255, T511 scalar GL sectoral values checked against DUCC
and an analytic field, T255 spin-1 GL DUCC values, CC/F1 scalar DUCC and
adjoint identities, and f32 scalar S/A against DUCC on GL/CC/F1. Six alternating q1 synthesis pairs compared fresh release binaries built
with Rust 1.99.0, empty Rust flags, GL STANDARD f64, full mmax, phi0=0.37,
one CPU core, and ntheta/nphi of 512x1024 at T511 and 1024x2048 at T1023. Each
process warmed once and recorded five samples. Candidate/baseline median
ratios and paired bootstrap 95% intervals were:

| T | Spin | Ratio | 95% interval | Result |
|---:|---:|---:|---:|---|
| 511 | 0 | 1.06980 | [1.06676, 1.07341] | Reject |
| 511 | 1 | 1.00604 | [0.99565, 1.01099] | Control passes |
| 1023 | 0 | 1.03446 | [1.02346, 1.03882] | Reject |

Both scalar target intervals exceed the 1.02 gate; all six T511 scalar pairs
were slower and five of six T1023 scalar pairs were slower by more than 2%. The run preflights observed no competing build or benchmark and refused no
process. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. This screen rejected the candidate; it did not proceed to the full operation
matrix or current-source DUCC/SHTns timings. No production change was accepted. The complete patch, focused test log, release build logs, raw stdout/time
files, preflight records, hashes, and paired summary are in
[reports/data/m6-m2-general-visitor](data/m6-m2-general-visitor/).

## M3 f32-to-f64 bridge screen (unaccepted)

A diagnostic GL STANDARD matrix compared native f32 transforms with a bridge
that widened the same f32-rounded coefficients or maps, ran the f64 plan, and
downcast outputs. The source-linked replay covered T127/T511/T1023, scalar and
spin-1, synthesis and analysis, and q1/q8. It measured a 3.49x–31.74x
improvement over native f32 and reduced the largest relative-L2 error versus
DUCC f32 from `4.873e-5` to `5.474e-8`. The bridge remained up to 13.00x slower
than DUCC f32 at T1023 scalar q8 synthesis; its bridge and DUCC medians were
86.278 ms and 6.637 ms. The largest bridge temporary payload was 48.0 MiB
for T1023 spin-1; the 237.6 MiB peak RSS includes all comparison plans,
workspaces and buffers. A clean build in a detached worktree reproduced the matrix ELF hash
`2d5ddf6febd6ef9fd480a0c2cd97feb1f10f89ee0e003097cfa2b2be0ea6e4fb` from the
archived benchmark source. The rebuilt binary passed the T127 spin-1 smoke and
all 24 matrix cells. Two earlier smoke/replay failures used an unrecorded or
different ELF hash and are retained but excluded. The initial timing table is
preserved separately because some DUCC medians differ from the source-linked
replay. The bridge remains unaccepted: the screen lacks independent paired
process repetitions, odd-longitude, adjoint, and extreme-value checks, and
the high-resolution performance gap to DUCC remains large. Build provenance,
the current table, raw timings, full-array comparisons and source are in
[`reports/data/m6-f32-bridge/`](data/m6-f32-bridge/).

## Post-C03 regression checks

These checks ran on 2026-10-17 after C03 and before the final P01 link-layout
comparison. The tested worktree included C03 and uncommitted P01 and P02
source changes, plus the M1 route instrumentation. These results validate that
combined worktree under the listed configurations; they do not accept either
performance candidate.

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
  passed.
- `cargo test --workspace --all-features`: passed; 197 unit, four extreme
  FFT integration, 11 f32, and one workspace compatibility test passed.
- `cargo test --workspace --no-default-features`: passed with the same test
  counts.
- `uv run pytest python/tests`: passed, 20 tests.
- `cargo test --workspace --all-features --release`: passed with the same
  Rust test counts. Each Rust suite ignored 14 unit tests and one opt-in f32 test, including the
documented high-bandlimit and diagnostic tests. No additional tests were
manually skipped. Full command output is in
[`reports/data/m6-validation/post-c03/`](data/m6-validation/post-c03/).

## Other uncommitted work

The current worktree contains uncommitted P01/P02 production candidates,
test-only M1 route counters, the extended Rust/Python benchmark harness, and
numerical-plan edits. Their source changes are in `examples/m1b_bench.rs`,
`examples/python_perf_baseline.py`, `plans/02-numerics-and-grids.md`,
`src/legendre/`, and `src/transform.rs`. P01 is unaccepted; P02 and the
isolated M2 visitor candidate are rejected. The separate P02 parity-panel
screen is rejected and archived. The matched DUCC/SHTns and M1
route-profile evidence is in `reports/data/m6-current-baseline/`, and the
rejected M2 candidate is preserved separately under
`reports/data/m6-m2-general-visitor/`. M1 findings have been reviewed: the
visitor dominates at T511/T1023, while visitor plus the shared transition
dominate at T2047. The ordered P01 matrix passes its measured gates but does
not resolve the default-link spin-1 control; source relocation also left the
linked symbol layout unchanged. C02's q1 T255 performance gate remains
unresolved, and P01 still lacks an accepted default-link control solution.

## M2 spin-1 fused Q/U visitor (preliminary pass; full matrix inconclusive)

An isolated candidate fused shared recurrence-basis and inverse-sine work in
the spin-1 Q/U visitor prologue. Per-channel coefficient/exponent handling,
multiply order, safety checks, and the scaled fallback remained intact. This
was not a SIMD change and did not modify shared production source. The source
starts from the C02+C03 baseline with P01/P02 excluded. Candidate patch,
archived source, focused checks, hashes, and raw timing evidence are in
[`reports/data/m6-m2-spin1-fused-visitor/`](data/m6-m2-spin1-fused-visitor/). The preliminary six-pair screen passed: T2047 spin-1 q8 synthesis measured
candidate/baseline `0.948295` [0.945511, 0.960977], and the T511 spin-1 q1
control measured `1.006177` [1.002531, 1.018994]. Focused analytic and DUCC
checks passed; the two output-map comparisons were bitwise equal to baseline. The full matrix is **incomplete and inconclusive**, so the candidate remains
**unaccepted**. Scalar T511 q1 and spin-1 T511 q1 each completed 12 pairs and
passed their control gates. Scalar T511 q8 also completed 12 pairs, but its
raw samples were severely nonstationary: within-process sample CV median
40.1%, range 21.9–107.4%, and paired ratios ranged 0.628–1.489. Its numerical
bootstrap interval is therefore not treated as a valid gate pass. No valid
sample was discarded. The next spin-1 T511 q8 cell has two completed pairs;
CPU-pressure validity was assessed under the declared protocol; raw readings remain local.
limit. The matrix stopped there, without relaxing the limit. The complete
audit, retained refusals/orphans, and partial cell records are linked from the
candidate evidence directory.

## T511 baseline timing trace (diagnostic only)

The archived baseline extension (`_rustsht.abi3.so`, SHA-256
`a8641456e1fcdf8d37ce3dd583dd1b7cf224306ca792d38c2b75f3bfcac63847`) was
traced on GL 512x1024 scalar f64 at phi0=0.37. The original `backend=both`
harness does one correctness comparison before `time_runs`; `time_runs` then
runs one untimed warm-up and five timed calls. The diagnostic recorded one
Rust correctness-call equivalent, one warm-up, then 20 individual Rust
synthesis calls. It omitted the intervening DUCC correctness synthesis, so it
is not an exact reproduction of the full harness sequence. The q8 trace (8 pinned CPU cores) measured 9.041-9.869 ms per call, median 9.486 ms
(CV 2.37%). The q1 control (one CPU core) measured 39.599-40.154 ms, median 39.834
ms (CV 0.50%). Neither reproduced the previously retained descending q8
samples 21.948, 17.882, 15.990, 13.217, 9.555 ms. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Per-call process involuntary context
switch deltas and thread CPU snapshots, as well as sysfs frequency readings,
are in the raw diagnostic files. No thermal-zone readings were exposed. This standalone diagnostic leaves all historical samples unchanged and does
not identify the cause of their variability. It defined no revised warm-up or
acceptance protocol. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local.

## Exact-harness baseline stability check

Four fresh baseline-only processes ran q8, q1, q1, q8 with the archived
`backend=both` harness unchanged. The baseline extension hash was
`a8641456e1fcdf8d37ce3dd583dd1b7cf224306ca792d38c2b75f3bfcac63847`; the
archived harness hash was
`72cd1986a220573c1a445bd6a6aa96b9df272a6bff1dc833843ee3f274c77739`. Configuration: scalar f64 GL T511, grid 512x1024, full mmax, phi0=.37,
STANDARD, scope `into`, synthesis, `runs=1`. The harness performed its
Rust-vs-DUCC correctness synthesis, one untimed warm-up, and five timed Rust
calls per process.

| Process | Five Rust samples (ms) | CV | First 3 median | Final 2 median |
| --- | --- | ---: | ---: | ---: |
| q8-01 | 24.938, 20.991, 14.161, 9.736, 9.289 | 48.96% | 20.991 | 9.512 |
| q1-02 | 39.646, 39.516, 39.475, 39.645, 39.560 | .19% | 39.516 | 39.603 |
| q1-03 | 39.787, 39.963, 39.534, 39.745, 39.827 | .39% | 39.787 | 39.786 |
| q8-04 | 21.038, 15.110, 17.596, 9.379, 9.433 | 33.84% | 17.596 | 9.406 |

Every preflight met PSI `some avg10 <= .50`, verified actual affinity of
8 pinned CPU cores for q8 and one CPU core for q1, and listed no competing build or benchmark. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. GNU time records whole-process context switches; the unchanged harness cannot
attribute involuntary context switches per call. A contemporaneous root-side
cgroup check reported unlimited `cpu.max` and zero throttling. Thermal
measurements were unavailable. The q8 decline reproduced on both baseline
processes, while q1 remained stable; the cause remains unresolved. Under the
prospective rule, both exact-harness q1 processes pass the CV and drift
checks, with first-three/final-two drift of 0.22% and 0.003%. Both q8
processes fail both checks, with drift of 81.1% and 54.2% and CVs of 48.96%
and 33.84%. Raw
process evidence is in
[`reports/data/m6-m2-spin1-fused-visitor/t511-q8-instability-diagnostic/`](data/m6-m2-spin1-fused-visitor/t511-q8-instability-diagnostic/). The visitor candidate remains **unaccepted and deferred** because q8 host
stability prevents its control gate. This is not a candidate performance
rejection; do not run the candidate acceptance matrix now. For future acceptance sampling only, retain the exact original sequence above
and all five timed Rust samples. Require process CV
`sample_sd(samples) / median(samples) <= 5%` and first-three-to-final-two drift
`abs(median(first3) - median(final2)) / median(all5) <= 5%`. Keep preflight
PSI `some avg10 <= .50` and require no competing build or benchmark. If any
process fails, retain it and stop the cell or matrix as inconclusive; do not
delete samples or retry until one passes. This proposed rule is not applied
retroactively. It is anchored to the clean standalone q8/q1 trace CVs
2.37%/.50% and exact-harness q1 control CVs .19%/.39%, compared with exact-
harness q8 CVs 48.96%/33.84%. The standalone trace and all historical samples
remain unchanged.

## Legendre redesign: source and machine checkpoints

The pinned DUCC `alm2leg` path prepares each harmonic order once, schedules
orders dynamically, packs active latitude rings, and integrates recurrence
with coefficient accumulation. Rust's current path assigns latitude groups;
each group repeats order setup, and its scaled paired visitor invokes a
coefficient callback for each degree pair and latitude block. Rust already has
paired-degree Ishioka-style recurrence and a fused ordinary SIMD block. This
comparison identifies duplicated setup and callback work as testable causes;
it does not establish their timing share. Exact source references and hashes
are in [`the source comparison`](data/m6-legendre-redesign/source-comparison.md). The clean baseline SIMD build at source `91c2cb8` used Rust 1.99.0 and
portable runtime dispatch on a x86-64 CPU (AVX2/FMA, no AVX-512). Emitted
`x86-v3` ordinary fused synthesis uses four f64 lanes and FMA without calls in
the vector degree loop. The scaled visitor emits a callback call per degree
pair. This is a code-generation observation, not a measured cause. The
reproducible build configuration, host record, symbol selectors, and excerpts
are in [`the assembly inspection`](data/m6-legendre-redesign/cpu-isa-asm.md). The full generated assembly remains in the local diagnostic capture with its
SHA-256 recorded there; it is not a production artifact.

## Clean-source baseline screen (incomplete)

An isolated release extension built from clean source `91c2cb8` ran one
exact-harness cell: GL scalar f64 T511, 512x1024, full mmax, phi0=0.37,
STANDARD, `synthesis_into`, q1 pinned to one CPU core. The Python harness was
`72cd1986…`; the references were DUCC 0.42.0 at
`80388904b8cfc32400ba84af8a6a45aabfe60f86` and SHTns 3.7.5 at
`4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`.

| Backend | Five timed samples (ms) | Median (ms) | CV | Relative L2 vs DUCC |
|---|---:|---:|---:|---:|
| RustSHT | 40.266, 40.139, 41.471, 40.322, 40.082 | 40.266 | 1.27% | 6.632e-14 |
| DUCC | 7.620, 7.531, 7.517, 7.517, 7.498 | 7.517 | 0.57% | — |
| SHTns adapter | 4.608, 4.675, 4.648, 4.651, 4.653 | 4.651 | 0.47% | 4.286e-14 |

The Rust median is 5.36x DUCC and 8.66x the SHTns adapter for this cell. CPU-pressure warnings were reviewed under the declared protocol; exact transient readings remain local. The
next T1023 preflight exceeded that limit, so the screen stopped without
reruns or omitted samples. Whole-process GNU time was 34.18 s and peak RSS was
96.8 MiB; the process time includes setup and reference work and is not
transform latency. This is one diagnostic cell, not a matched acceptance
matrix or a revalidation of the historical T1023/T2047 baselines. Raw samples,
host snapshots, refusal, provenance, hashes, and reproduction scripts are in
[`the clean baseline evidence`](data/m6-legendre-redesign/baseline/summary.md).

## M2 native scalar synthesis prototype (rejected implementation; screen incomplete)

The isolated f64 GL prototype packs active rings into reusable 256-lane tiles,
keeps the scaled recurrence and its accumulation together until every lane is
ordinary, then continues with a one-channel SIMD recurrence/accumulator. A tile
that cannot complete the route stages the robust paired visitor result before
one scatter. The source snapshot, patch, test code, build provenance and raw
process records are retained in
[`the prototype screen`](data/m6-legendre-redesign/prototype-screen/). The single stable T511 q1 full-transform control numerically matched both the
clean baseline and DUCC (`relative L2 = 6.632e-14`). Its Rust median rose from
40.228 ms to 118.984 ms (2.96x; +195.8%). DUCC and SHTns timings were unchanged
within measurement noise. This exceeds the 2% operation regression guard by a
large margin, so this implementation is rejected as a performance candidate;
it is not accepted into production. The comparison used clean baseline source
`91c2cb8` and prototype base `739275d` plus the archived patch. The host and
timing gates passed for both T511 processes. The CPU-pressure check failed the declared gate; exact transient readings remain local. No retry was made. T1023/T2047 cells and the complete
control matrix therefore remain unmeasured. This result rejects the current implementation, not the broader recurrence
and accumulation hypothesis. The next step is to account for fast-route
success, scaled-prefix length, and fallback work by order and latitude tile,
then make at most one source change that removes a measured source of duplicate
work. No further performance claim should be made from this incomplete screen.

## Polar-lane route diagnostic and split experiment (rejected)

Test-only route accounting at T511 q1 found 974 active tiles. Before the
split, 206 tiles declined because one active lane had `sin(theta) < 0.01`,
forcing the entire tile through the robust visitor. Those tiles accounted for
about 12.38 million fallback lane-pairs. Only four additional tiles declined
after attempting the staged transition. This identifies the overly broad
tile-level polar guard as the main concrete cause of the 2.96x result; the
route counts are not themselves a timing attribution. The isolated follow-up partitions each packed tile in place by the existing
`sin(theta) >= 0.01` ordinary-route condition, preserving ring indices. It
runs ordinary lanes through the staged SIMD path and polar lanes through
scaled recurrence, then scatters once. At the T511 q1 route diagnostic,
970/974 tiles completed the ordinary route; 47,534 polar lane-pairs and
255,104 other lane-pairs used the scaled visitor. The map was bitwise
identical with route counters enabled and disabled. The paired release screen does not meet the performance gate. The baseline
Rust median was 40.021 ms and the candidate median was 46.307 ms, a 15.71%
increase. Candidate samples also failed the fixed drift limit (6.043% against
5%), so this cell cannot establish an accepted comparison. The result rejects
the current implementation for performance work; T1023/T2047 and q8 were not
run. DUCC agreement was unchanged at relative L2 `6.632e-14`. The candidate
workspace suite passed: 200 unit tests, 4 FFT-scaling tests, 11 f32 tests and
1 workspace-compatibility test; 14 opt-in tests were ignored. The source,
paired raw measurements, and test summary are in
[`the polar split screen`](data/m6-legendre-redesign/polar-split-screen/). The implementation remains isolated from production routing.

## Tile-128 scalar synthesis screen (promising, unaccepted)

The isolated polar-split candidate was rebuilt with `ACTIVE_TILE_SIZE=128`;
the rest of the source matched the archived tile-256 candidate. The T511 q1
Rust median was 32.212 ms versus 40.061 ms for clean baseline source `91c2cb8`
(ratio 0.8041, −19.59%). Rust timed-sample CV and drift were 0.13% and 0.19%. Relative L2 error against DUCC was `6.632e-14` for both builds. Peak process RSS
was 99,184 KiB for the candidate and 99,144 KiB for baseline. The CPU-pressure check failed the declared gate; exact transient readings remain local. The screen therefore fails the host gate and does not establish an
accepted performance comparison. No retry, T1023/T2047 cell, q8 cell, analysis
cell, or production change followed. This T511 result is a raw positive signal
for the 128-slot tile; its numerical, performance, and regression controls
remain incomplete. The source patch, binary hashes, raw processes, and
stability calculations are in
[`the tile-128 screen`](data/m6-legendre-redesign/tile128-screen/).
