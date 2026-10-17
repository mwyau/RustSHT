# M6 CPU performance parity

Branch: `opt/sht-performance-parity`, based on integrated revision
`a918e80e6bd1488d738518c87d26deb4a9c11d0f`. The campaign follows
`ASTRA_INTEGRATED_REVIEW.md`; performance parity is unproven.

## Checkpoint state

The branch contains three production checkpoints: C01 workspace validation
(`c0e11ad`), C02 FFT range and scaling (`8df2ecf`), and C03 unsafe spin
conversion fallback (`197c3d3`). C01 and C03 are committed correctness fixes. C02 is committed as a numerical checkpoint, but its per-operation control
gate remains unresolved; its scalar T255 q1 synthesis and analysis controls
exceed the predeclared 2% threshold. P01 remains uncommitted and unaccepted;
it is shelved after its default-link spin-1 control exceeded the guard. P02 is
uncommitted and rejected because its isolated comparison showed no material
gain. The isolated M2 general-visitor candidate is also rejected and remains
outside production. M1 route-profile evidence is committed; its test-only
counters, benchmark harness changes, and numerical-plan edits remain
uncommitted. The campaign report and raw measurements are recorded in
diagnostic evidence checkpoints.

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
and E/B potential conversion. Full tables, errors, raw samples, and build
provenance are in [`reports/data/m6-current-baseline/`](data/m6-current-baseline/). The q1 route profile reports per-worker CPU timer sums, not transform wall
time. Fused/visitor/transition sums were 7.421/26.408/0 ms at T511,
55.352/191.910/13.420 ms at T1023, and 362.784/1505.354/1181.839 ms at
T2047. Thus the visitor dominates at T511 and T1023 even though it handles
about 9–10% of committed route units. At T2047 the visitor and the shared
two-channel transition account for most profiled route work. Profiling
overhead changed q1 wall time by 0.6% at T1023 and −0.04% at T2047. q8 route
counts match the q1 totals, but workers are imbalanced at T2047 (maximum to
minimum active route timer ratio 9.53); q8 timer sums are descriptive. The
route tables and worker-level evidence are in
[`reports/data/m6-current-baseline/route-profile/`](data/m6-current-baseline/route-profile/).

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
and its retained source, binary hash, and raw output. The default-link T511 spin-1 synthesis control has a repeatable greater-than-
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
microarchitectural mechanism, or a portable default-link solution. The
successful linker command was not retained. The ordering was applied at the
final link of the local `m1b_bench` executable; the RustSHT rlib does not
encode it, so P01 remains unaccepted for downstream builds. Raw output,
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
mechanism for the layout effect. Shelve P01 in its current form. The first follow-up general visitor candidate and P02 coefficient reuse are
screened and rejected below. Route timing identifies scalar synthesis work,
but does not itself establish a portable source change. Symbol and
disassembly evidence is in
[`reports/data/m6-p01/codegen-diagnosis/`](data/m6-p01/codegen-diagnosis/).

ordered-link P01 T511 spin-1 executables found nearly identical total dynamic
instruction references and last-level instruction misses. Its model reported
more L1 instruction misses and conditional branch mispredicts for the faster
microarchitectural mechanism. See

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
the local benchmark executable. P01 remains archived, unaccepted and pending
a portable source-level solution. The full suites below ran with P01 and P02
together. Do not infer downstream consumer performance from the ordered link.

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
isolated M2 visitor candidate are rejected. The matched DUCC/SHTns and M1
route-profile evidence is in `reports/data/m6-current-baseline/`, and the
rejected M2 candidate is preserved separately under
`reports/data/m6-m2-general-visitor/`. M1 findings have been reviewed: the
visitor dominates at T511/T1023, while visitor plus the shared transition
dominate at T2047. The ordered P01 matrix passes its measured gates but does
not resolve the default-link spin-1 control; source relocation also left the
linked symbol layout unchanged. C02's q1 T255 performance gate remains
unresolved, and P01 still lacks an accepted default-link control solution.
