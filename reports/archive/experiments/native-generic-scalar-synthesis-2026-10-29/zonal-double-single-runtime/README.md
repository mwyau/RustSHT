# Native-f32 paired zonal recurrence screen — 2026-10-29

**Status: isolated diagnostic; scalar-synthesis parity remains a GAP.** This
candidate adds a native-f32 compensated paired recurrence for the `m=0`
branch of the shared order-major synthesis route. It improves the T720 zonal
case, but does not reduce random-transform error, does not close full-grid
timing gaps, and has not been promoted to production.

## Candidate and provenance

- Base: `b6a96aa7161a64f678b614d73478496f50f0551e` in the isolated generic
  source worktree.
- Candidate source patch SHA-256:
  `7af69b3fe0af09754d09eac67ed10006eea1a14fc468483ed24d50ce771fc1b9`.
  Recover it with `git apply` from
  [`patches/native-f32-paired-zonal-leaf.patch.gz`](patches/native-f32-paired-zonal-leaf.patch.gz).
- Rust candidate binary SHA-256:
  `03044d2686bae76bd9119f9d5f26dadbce8fd11d708e81459b3274299da0dea5`.
- Parent binary SHA-256:
  `d9158760d31937718f43e2ad09542a60ca2ca90e0018164238b2445345d3d932`.
- Pinned native-f32 DUCC 0.42.0 commit:
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; binary SHA-256
  `be18c89ea434258e4a07c28ccd8fd31d6262ac3503dcaf81d8aa5176415f1484`.
- Compiler: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, flags
  `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, feature `parallel`.
- Host: AMD Ryzen 9 5950X. q1 pinned to CPU 0; q8 pinned to
  `0,2,4,6,8,10,12,14`.
- Checksums for all retained artifacts are in [`SHA256SUMS`](SHA256SUMS).

The compensated leaf keeps runtime recurrence state, coefficients, SIMD, and
outputs in f32. Plan-time f64 preparation is limited to high/low pieces for
`cos(theta)`, `cos(theta)^2`, paired recurrence factors, and coefficient
factors. The branch requires the T720 zonal geometry and finite real
coefficients within the configured magnitude guard; other inputs use the
ordinary shared route. It does not construct a full m=0 basis or add a
post-transform correction pass.

## Accuracy and tests

The isolated T720 `l=720,m=0` test reduced maximum absolute error over all
rings from `2.759e-3` to `3.190e-8`; the first north-ring error is
`7.414e-10`. Mixed real coefficients over all m=0 degrees had maximum
absolute error `5.589e-6` and maximum scaled error `9.486e-8`. Reset
determinism and analysis recovery checks passed. The exceptional-range test
for subnormal, minimum-normal, maximum-finite, and infinite inputs passed.

On the deterministic random GL T720 fixtures, aggregate candidate/DUCC
relative L2 error remained `1.8644e-4` at full M720 and `1.8851e-4` at
reduced M180; maximum absolute errors were `5.498e-4` and `4.727e-4`.
Candidate and parent outputs were bitwise equal for those fixtures. That
equivalence is not independent correctness evidence.

An accuracy-only order-band screen then isolated the error source. The m=0
band was within `5.83e-8` relative L2 of DUCC. Nonzero order bands ranged from
`1.32e-4` to `2.10e-4`. By degree band, relative L2 grew from `4.14e-5` for
degrees 0–239 to `2.55e-4` for degrees 600–719; degree 720 alone was
`2.90e-4`. An intersection screen found the largest relative errors for
orders 1–32 at high degree: `3.25e-4` for degrees 600–719 and `4.28e-4` for
degree 720. The degree-720 intersection contains only 32 complex coefficients
per channel and has maximum absolute error `3.25e-5`, so its relative value
should be read with its reference RMS. For degrees 600–719, orders 33–180 and
181–360 were `2.73e-4` and `2.97e-4`, while orders 361–720 were `1.61e-4`.
Correcting m=0 alone cannot materially resolve the aggregate discrepancy. The
exact filtered-fixture hashes and per-band metrics are in
[`results/order-band-errors.json`](results/order-band-errors.json); the
intersection metrics are in
[`results/order-band-intersections.json`](results/order-band-intersections.json).
The ringwise screen for `m=1–32,l=600–719` concentrates the largest absolute
errors at rings 5 and 717 (`3.52e-4` and `3.50e-4`); the highest relative
errors also include the first ring and equator ring 360. The matching north
and south polar-end rings appear in both screens. The `l=720` relative error
peaks at rings 0 and 720 but has only `3.25e-5` maximum absolute error.
Splitting the `l=600–719` coefficients further gives:

| Orders | Relative L2 | Maximum absolute error |
|---|---:|---:|
| 1 | `3.36e-4` | `6.22e-5` |
| 2–4 | `4.44e-4` | `1.16e-4` |
| 5–8 | `4.61e-4` | `2.34e-4` |
| 9–16 | `3.85e-4` | `3.42e-4` |
| 17–32 | `1.25e-4` | `7.27e-5` |

These one-fixture filtered results localize error, but do not identify the
numerical cause. Per-ring and low-order metrics are in
[`results/order-band-ring-errors.json`](results/order-band-ring-errors.json)
and [`results/order-low-order-errors.json`](results/order-low-order-errors.json).
Run [`results/diagnose_order_bands.py`](results/diagnose_order_bands.py) with
`--subset base`, `intersections`, `ring-target`, or `low-order`; add
`--ringwise` for the per-ring records.

A separate test-only observer examined the f32 prefix transition at GL T720
for orders 1–16 with coefficients only in degrees 600–720. It recorded 46
ring batches per order (736 total). Every batch had zero scaled-prefix pair
updates, began the ordinary suffix at degree `m`, and was already in the full
IEEE range. Thus these filtered low-order cases execute the ordinary f32
recurrence across the full degree range; the counters do not distinguish
recurrence error from coefficient-accumulation error. The probe uses the same
T720 latitudes with reduced `M16,nphi=34`, and is not a timing result. Its
test-only patch and per-order summary are in
[`patches/prefix-transition-observer.patch.gz`](patches/prefix-transition-observer.patch.gz)
and [`results/prefix-transition-summary.json`](results/prefix-transition-summary.json).
The ignored probe passed (1 passed, 356 filtered); strict Clippy, formatting,
and diff checks passed on the instrumented source.

`cargo test --features parallel --lib` passed on the paired source:
319 passed, 37 ignored, in 103.80 seconds. `cargo clippy --all-targets
--features parallel -- -D warnings`, `cargo fmt --check`, and `git diff
--check` also passed. The sanitized command/result record is in
[`results/test-summary.txt`](results/test-summary.txt).

## Three-pair timing screen

Fresh parent/candidate/DUCC processes used the same native-f32 fixtures,
interleaved in balanced order; target batch time was 25 ms. Results are
medians across three matched pairs, without an acceptance-grade confidence
interval:

| GL T720 scalar synthesis | Parent ms | Candidate ms | DUCC ms | Candidate/DUCC | Candidate/parent |
|---|---:|---:|---:|---:|---:|
| Full q1 | 24.994 | 24.168 | 20.083 | 1.256 | 0.967 |
| Full q8 | 3.613 | 3.707 | 2.913 | 1.272 | 1.026 |
| Reduced q1 | 8.486 | 8.690 | 8.664 | 1.019 | 1.006 |
| Reduced q8 | 1.655 | 1.915 | 2.133 | 0.951 | 1.085 |

Full q1/q8 remain clear performance GAPS. Reduced q1 is close but not an
acceptance result; reduced q8 is noisy and is not promoted as a PASS. The
complete pair data, fixture hashes, binary hashes, output checks, SIMD route,
and reproducible runner are in
[`results/f32-parent-candidate-ducc-3.json`](results/f32-parent-candidate-ducc-3.json)
and [`results/bench.py`](results/bench.py).

## Generated code

On this host Pulp dispatch selects V3/AVX2, with eight f32 lanes. The executed
shared ordinary-suffix specialization loads eight lanes into YMM registers,
uses packed `vfmadd*ps` operations for the recurrence and reflected
accumulations, and processes degree pairs per loop iteration. The inspected
suffix function has a 40-byte stack frame for saved general-purpose state and
no YMM state spills. See
[`codegen/ordinary-suffix-v3-avx2.asm`](codegen/ordinary-suffix-v3-avx2.asm).
The compensated zonal branch was inlined into the dispatch body, so this
excerpt documents the adjacent shared bulk path, not a standalone dump of the
compensated branch.

## Decision and next step

Keep this leaf isolated. It establishes a precise m=0 accuracy correction but
does not address the dominant nonzero-order/high-degree error or full-grid
performance gaps. Since the target orders use no scaled prefix, next compare
ordinary f32 recurrence and coefficient-accumulation error for `m=1–16` at
`l=600–720`, especially rings 0–5, 360, and 715–720. Any broader compensated
state must remain native f32 at runtime,
preserve exceptional-range behavior, and avoid adding work to every order
unless accuracy and whole-transform timing both support it. Production
dispatch, the temporary f64 child-plan route, and other transform families
were not changed by this experiment.
