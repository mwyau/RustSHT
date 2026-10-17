# M6 CPU performance parity

Branch: `opt/sht-performance-parity`, based on integrated revision
`a918e80e6bd1488d738518c87d26deb4a9c11d0f`. The campaign follows
`ASTRA_INTEGRATED_REVIEW.md`; performance parity is unproven.

## C01: workspace order-prefix compatibility

A public GL scalar T511 reproducer used identical `512×1024` grids and
`mmax=0`/`511` plans. A low-order workspace passed the full-order plan's
`prepare_workspace(..., 2)`, then `analysis_into` panicked at the prefix
slice in `src/transform.rs:1472` (length two, requested end index 512).

The workspace validator now requires at least `mmax+2` prefix entries in
parallel builds. Incompatible preparation and execution return
`ShtError::BufferLength` before output mutation or capacity changes. A larger
prefix can serve a reduced-order plan. Serial builds have no prefix allocation
and retain compatible cross-order execution.

The public regression covers scalar T511 and spin-1 T255 at q1/q2/q3,
unchanged error outputs and worker allocation, reverse-order synthesis and
analysis, and comparison with a matching workspace. It uses analytic m=0
fields; no new DUCC comparison is required for the capacity check.

Validation on Rust 1.99.0 (2026-10-17):

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo test --workspace --all-features`: 188 unit, 11 f32 and one public
  compatibility test passed; 13 unit and one f32 test ignored.
- `cargo test --no-default-features`: 150 unit, 11 f32 and one public
  compatibility test passed; seven unit and one f32 test ignored.
- `cargo test --release --test workspace_compatibility`: passed.

Commands used `TMPDIR=/home/albert/RustSHT/target/tmp`. Existing numerical
comparisons in these suites identify DUCC 0.41.0, pinned source
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. Python, explicit ignored high-L
and standalone FFT tests were not run for C01; bindings, numerical kernels,
and the standalone workspace did not change.

## C02: real FFT range and synthesis normalization

The baseline failed all three length-two extreme FFT tests: f64 forward
`[1e308,0]`, f64 normalized inverse bins `[1e308,1e308]`, and f32 forward
`[2e38,0]`. Scalar GL `1×8` monopoles with `a00=1e308` (f64) or `2e38`
(f32) produced alternating NaNs and infinities, although their constant maps
are finite.

The candidate retains the ordinary normalized reconstruction arithmetic and
recomputes nonfinite reconstruction results with overflow-aware half-sums.
Only an overflowing sum of finite operands is halved before addition. This
preserves the ordinary path's subnormal behavior. Synthesis now uses an
unnormalized inverse on Fourier-series amplitudes, eliminating the temporary
`nphi` multiplication. Recurrence, spin, derivative and hemisphere routes use this convention.
The bounded f64 scalar table path retains its original scaled spectra and
normalized inverse; its existing input limit excludes extreme coefficients. CC/F1 adjoint analysis keeps its
normalized inverse. [The numerical plan](../plans/02-numerics-and-grids.md)
derives the four operators' scaling.

Targeted debug checks passed: direct complex FFT and Hermitian sums,
length-two extreme cases, smallest-subnormal f64 DC values at lengths
1/2/3/4/5/8/31/32/33, extreme monopoles at longitude counts
1/2/3/8/9/32, and large-range controls for S/A/S*/A* on GL/CC/F1,
spins 0–4, f64/f32, nonzero `phi0`, and odd/even longitudes. The range
controls compare scaled complete arrays with ordinary execution at L4;
independent analytic and DUCC checks are supplied by the existing suite.
Private pre-FFT tests now compare Fourier-series amplitudes instead of
`nphi`-scaled DFT bins, with unchanged tolerances.

The first candidate passed full debug/release/serial suites, eight opt-in
high-L tests, formatting and Clippy, and 111 standalone FFT tests in both
debug and release. Its six-pair controls failed the predeclared per-operation
regression gate: scalar T255 q1 synthesis increased by 3.68% (paired-median
95% bootstrap interval 3.10%–4.39%). Analysis increased by 2.51% at that cell,
with additional slowdown signals in other analysis controls. These results
are preserved in `paired/summary.csv`; S+A totals do not replace the separate
S and A gates.

A diagnostic build without the per-bin exceptional guard improved scalar
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
1.01719–1.02564; four of six pairs above 1.02). The q8 analysis and scalar
T511 q8 cells were noisy; CPU pressure `some avg10` rose to 15.58% during the
screen. Their raw samples were retained. The revised C02 candidate therefore
passes these numerical checks but does not pass the per-operation performance
control. It is not accepted. The summary, method, C02-only source patch, and
all 96 process outputs are in
[`reports/data/m6-c02/revised-validation/paired/`](data/m6-c02/revised-validation/paired/).
Baseline timing artifacts are in [the C02 data directory](data/m6-c02/). The
private FFT adapter still inherits the FFT engine's range limits; these
checks do not prove arbitrary large-spectrum cancellation is overflow-free.

## C03: specialized spin fallback

Both reported conversion guard gaps are reproduced before FFT execution.
The spin-1 hemisphere route at T319 produced infinity where the original
spin recurrence gave a finite value. The spin-2 scalar route at T500
produced NaN for finite E/B input with a finite general-reference value.
The reproducer log is `target/m6-c03-reproduce.log`.

The candidate extracts the existing general-spin order projection into a
private helper. Spin-1 hemisphere and spin-2 scalar synthesis check converted
coefficients and paired preparation before committing any order output. A
failure projects the original E/B coefficients with freshly initialized spin
factors. Hemisphere fallback covers both hemispheres and both channels.
Spin-1 paired preparation is performed once per order before its latitude
blocks. No workspace arrays or public APIs are added.

The C03 extreme release regressions and five existing focused regressions pass:
spin-1 hemisphere pairing, cached versus uncached coefficients, channel cases,
the explicit nonfinite-conversion fallback, and spin-2 mixed-polar blocks. The
extreme cases cover spin-1 T319/T511 cached and uncached conversions, 67-lane
blocks and both hemispheres, plus spin-2 T500/T511 E-only, B-only and E+iB
inputs. `cargo fmt --all -- --check` also passes. The checked fallback keeps
the original E/B rows and runs before order output is committed; the paired
preparation helper declines nonfinite prepared coefficients.

Six alternating q1 process pairs compared C02+C03 against the C02-only
candidate at T511, GL STANDARD f64, full mmax, 512x1024 and phi0=0.37. Ratios
are candidate/baseline, separated by complete operation. Spin-1 synthesis was
1.02016 (95% paired bootstrap interval 1.00816–1.02561; three of six pairs
above 1.02); spin-1 analysis was 0.99292 [0.97984, 1.00546]. Spin-2 synthesis
was 1.00024 [0.98688, 1.00373], and spin-2 analysis was 0.97381
[0.96815, 0.97854]. Preflights found no competing build or benchmark process
and refused no run. CPU pressure `some avg10` rose from 0% to 8.28% during the
screen, making the near-2% spin-1 synthesis estimate inconclusive. The ordinary
controls show no repeatable greater-than-2% signal in six pairs, but do not
accept C03. Raw output, time/RSS and preflight files are in
[`reports/data/m6-c03/ordinary-controls/`](data/m6-c03/ordinary-controls/).
The two focused conversion fixtures do not separately establish a paired-
preparation failure when the conversion itself is finite.

## Remaining work

P01 scalar synthesis performance remains unresolved. In 12 paired T511 spin-1
q1 synthesis runs, the current P01 candidate had a median candidate/baseline
ratio of 1.02073 (95% pair-bootstrap interval 1.01791–1.02761). Disabling the
scalar transition removed this shift, but also removed the T1023 scalar
synthesis gain. Marking the SIMD transition body `#[inline(never)]` raised
the T1023 scalar ratio to 1.74588. Marking only the outer wrapper
`#[inline(never)]` preserved the T1023 gain (ratio 0.65958) but measured a
T511 spin-1 ratio of 1.02351 (95% interval 1.01968–1.04834); CPU pressure
`some avg10` reached 9.28% during this screen. The control therefore did not
clear the 2% median guard, and P01 is not accepted. No T2047 run was made for
the wrapper-outline variant. Raw runs, summaries and patches are in
[`reports/data/m6-p01/`](data/m6-p01/).

C02 performance gate resolution; a cleaner repeat of the near-2% C03 spin-1
synthesis control; matched current-source DUCC/SHTns benchmarks; route
accounting; f64 synthesis experiments; f32 widening experiment. The existing
review records earlier benchmarks, which are not current campaign baselines.
