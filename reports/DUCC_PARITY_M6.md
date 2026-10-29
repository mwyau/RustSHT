# M6 — live DUCC parity ledger

Production transforms remain unchanged from
`c63cde440d91e3ca2810226163a191199b4ca29d`. The isolated generic scalar
candidate and follow-ups are based on `b6a96aa7161a64f678b614d73478496f50f0551e`.
**Reference:** DUCC 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
All times below are warm whole-transform milliseconds. Geometry is GL T720
(`ntheta=721`; full M720 uses `nphi=1458`, reduced M180 uses `nphi=378`).
`q1` and `q8` are thread counts. The exact benchmark methods and detailed
records are linked.

## Focused native-f32 GL T720 matrix

The current f32 T720 scalar-synthesis dispatch uses a guarded f64 child plan
and conversion buffers. It meets DUCC timing in four cells, but does not meet
the native-f32 architecture requirement. Other cells remain native f32. The
`fdae803a` ratios below are ratios of the reported median times; they are
baseline evidence, not a paired confidence interval.

| Cell | Current/last measured route | Rust/DUCC | Source | Native-generic status |
|---|---|---:|---|---|
| Scalar synthesis, full q1 | Public f64 sidecar `15.836 / 19.131`; native baseline `641.339 / 18.902`; generic `26.061 / 19.233`; uniform-scale `24.474 / 19.618` | Sidecar `0.829 [0.823, 0.834]`; native `≈33.93x`; generic `1.355 [1.349, 1.370]`; uniform-scale `1.249 [1.233, 1.258]` | Production `c63cde44`; native `fdae803a`; generic parent `b6a96aa7` + patch `661a1167`; candidate diff `9d5c7b57` | **GAP** — native candidate remains 25% slower |
| Scalar synthesis, full q8 | Public f64 sidecar `2.790 / 2.884`; native baseline `98.183 / 2.838`; generic `3.924 / 2.946`; uniform-scale `3.739 / 2.998` | Sidecar `0.961 [0.934, 0.974]`; native `≈34.60x`; generic `1.338 [1.223, 1.360]`; uniform-scale `1.256 [1.135, 1.320]` | Production `c63cde44`; native `fdae803a`; generic parent `b6a96aa7` + patch `661a1167`; candidate diff `9d5c7b57` | **GAP** — native candidate remains 26% slower |
| Scalar synthesis, reduced q1 | Public f64 sidecar `6.486 / 8.385`; native baseline `190.225 / 8.317`; generic `8.846 / 8.418`; uniform-scale `8.519 / 8.530` | Sidecar `0.775 [0.770, 0.779]`; native `≈22.88x`; generic `1.054 [1.045, 1.058]`; uniform-scale `0.995 [0.992, 1.009]` | Production `c63cde44`; native `fdae803a`; generic parent `b6a96aa7` + patch `661a1167`; candidate diff `9d5c7b57` | **INCONCLUSIVE** — timing interval crosses 1; accuracy remains open |
| Scalar synthesis, reduced q8 | Public f64 sidecar `1.184 / 1.298`; native baseline `54.588 / 1.314`; generic `1.773 / 1.339`; uniform-scale `1.671 / 1.504` | Sidecar `0.885 [0.793, 0.918]`; native `≈41.54x`; generic `1.134 [1.078, 1.344]`; uniform-scale `1.082 [1.032, 1.148]` | Production `c63cde44`; native `fdae803a`; generic parent `b6a96aa7` + patch `661a1167`; candidate diff `9d5c7b57` | **GAP** — latest paired interval is above 1 |
| Scalar analysis, full q1 | Native f32 `453.143 / 14.064` | `≈32.22x` | `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** |
| Scalar analysis, full q8 | Native f32 `60.005 / 2.274` | `≈26.39x` | `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** |
| Scalar analysis, reduced q1 | Native f32 `139.936 / 5.774` | `≈24.24x` | `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** |
| Scalar analysis, reduced q8 | Native f32 `20.132 / 1.001` | `≈20.11x` | `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** |
| Spin-1 synthesis, full q1 | Native f32 `1487.599 / 41.068` | `≈36.22x` | `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** |
| Spin-1 synthesis, full q8 | Native f32 `273.640 / 5.962` | `≈45.90x` | `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** |
| Spin-1 synthesis, reduced q1 | Native f32 `625.249 / 18.059` | `≈34.62x` | `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** |
| Spin-1 synthesis, reduced q8 | Native f32 `125.539 / 2.673` | `≈46.97x` | `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** |
| Spin-1 analysis, full q1 | Native f32 `1040.405 / 28.807` | `≈36.12x` | `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** |
| Spin-1 analysis, full q8 | Native f32 `138.878 / 4.879` | `≈28.46x` | `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** |
| Spin-1 analysis, reduced q1 | Native f32 `375.930 / 11.759` | `≈31.97x` | `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** |
| Spin-1 analysis, reduced q8 | Native f32 `58.159 / 2.258` | `≈25.76x` | `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** |

The f64 sidecar cells used 15 interleaved pairs. The native baselines are from
the 15-pair Stage B matrix; full/reduced q1/q8 native routes were finite and
passed repeat/reset checks. These measurements do not prove independent
accuracy for the native-generic replacement. See the [production adapter
checkpoint](data/ducc-parity-m6-2026-10-29/f32-internal-scalar-synthesis-production/README.md)
and [16-cell Stage B matrix](data/ducc-parity-m6-2026-10-29/f32-io-f64-internal-stage-b/README.md).

## Protected f64 control

At source `c63cde44`, f64 GL T1023 full spin-1 synthesis remained bitwise equal
to its parent. Candidate/parent was `0.984` q1 and `0.993` q8; Rust/DUCC was
`0.739` and `0.714`. The 15-pair source evidence is in the [production
checkpoint](data/ducc-parity-m6-2026-10-29/f32-internal-scalar-synthesis-production/README.md).

## Isolated native-generic scalar-synthesis candidate

The initial generic patch used native f32 throughout and missed all four
GL T720 cells. A precision-gated exact-zero restore shortcut improves its
parent by 5–7% in 15 matched pairs, but full q1/q8 remain 1.24–1.27x DUCC;
reduced q1 is INCONCLUSIVE near parity and reduced q8 is too noisy. Outputs
match the parent bitwise and pass repeat/reset checks, but that is not
independent accuracy evidence. The near-pole zonal probe prints `0.018705474`
in f32 versus `0.018541995` from its independent f64 recurrence reference.
The FMA-skip follow-up did not establish an added gain. Source patches,
pair-level data, intervals, fixture hashes and runners are in the
[archived candidate record](archive/experiments/native-generic-scalar-synthesis-2026-10-29/README.md).

The subsequent uniform-exponent f32 SIMD scaler improves the generic parent
in three of four cells, but the 15-pair screen remains a GAP in both full
cells and reduced q8; reduced q1 is INCONCLUSIVE. It is bitwise
parent-equivalent, but the high-degree near-pole zonal result still differs by
0.88% from the independent f64 recurrence reference (`0.018705474` versus
`0.0185419947`). Pinned native-f32 DUCC returns `0.0185419954` on the same
zonal input; its source uses double SIMD recurrence state despite f32 I/O.
The diagnostic does not assert a tolerance, so this candidate remains
numerically unaccepted. A test-only f32 double-single recurrence with
high/low geometry and recurrence constants reached `1.0e-9` absolute error.
An integrated compensated f32 recurrence now replaces the shared route's
`m=0` recurrence for its guarded zonal case; it corrects the zonal probe but
does not change random-transform error. Its timing and accuracy evidence is
recorded separately below. The archive contains the prior full measurements,
work counts, exact scaler tests and probe source.

The f32 vector-rescale leaf and forced-inline follow-up were rejected after
three-pair screens. Callgrind and V3 disassembly attribute substantial setup
cost to the helper; neither candidate is promoted. A separate f64 T1023
control for the original generic patch was about 3% slower than parent. The
latest precision-gated zero-restore source was bitwise equal to its parent and
neutral at q1 (`0.996`, 95% interval `0.991–1.002`); q8 was mildly slower and
INCONCLUSIVE (`1.016`, `0.999–1.055`). Keep this control in the next candidate
screen.

A separate per-lane i32 AVX2 exponent-restoration leaf also missed all four
cells (1.112–1.623x DUCC); its q1 profile spent 36.9% of instruction
references in the scaler. It did not establish a useful SIMD speedup and
should not be repeated unchanged. Exact patch, binary and profile provenance
are in the [isolated synthesis record](archive/experiments/native-generic-scalar-synthesis-2026-10-29/README.md).

The native-generic suffix work-count probe found that its two-vector fused
path already handles 74.6% of full and 83.8% of reduced ordinary lane-pair
updates. This favors further work on the scaled prefix; the counter does not
prove that individually tiny contributions can be omitted safely.

A separate `#[inline(always)]` change on the ordinary suffix was screened
against the exact non-inlined generic binary. The original suffix already
emits direct AVX2 FMA and uses a 40-byte stack frame; the attribute change
produced no repeatable whole-transform gain. The reduced q8 screen was noisy.
Keep the suffix unchanged; see the [codegen and paired delta record](archive/experiments/native-generic-scalar-synthesis-2026-10-29/ordinary-suffix-inline/).

A subsequent m=0 basis-dot diagnostic corrected the isolated GL T720
`l=720,m=0` zonal probe: maximum absolute error across 721 rings fell from
`2.759e-3` to `1.483e-8`, with `7.414e-10` error at the first north ring.
The variant computes the full m=0 basis with an f64 plan-time recurrence,
which exceeds the allowed small constant preparation, then adds a compensated
f32 dot pass after synthesis. Its three-pair candidate/DUCC medians were
`1.249`, `1.325`, `1.029`, and `1.343` for full q1/q8 and reduced q1/q8.
Random-input error versus DUCC stayed at `1.864e-4` / `1.885e-4` relative L2;
the correction does not justify its extra pass. Reject this design. The exact
patch, binaries, fixtures, tests, work counts, AVX2 disassembly and pair data
are in the [prepared-basis archive](archive/experiments/native-generic-scalar-synthesis-2026-10-29/zonal-basis-dotproduct/README.md).

A follow-up skipped generic m=0 recurrence work and restored its column after
the other orders, but still used the full f64-prepared basis. Its three-pair
candidate/DUCC medians were `1.276`, `1.265`, `1.036`, and `1.284` for full
q1/q8 and reduced q1/q8. The zonal error stayed at `1.483e-8`; random-input
error was unchanged. The full-basis plan-time route is outside the allowed
design, and this variant missed DUCC in every cell. It passed `cargo check`
and the focused zonal test; the full library suite was not run on this exact
variant. Preserve it as rejected evidence only; see the [integrated
skip-and-restore archive](archive/experiments/native-generic-scalar-synthesis-2026-10-29/zonal-integrated-order-skip/README.md).

The subsequent native-f32 paired-recurrence leaf replaces the `m=0` recurrence
inside the order-major loop and uses high/low plan constants with f32 runtime
state. It reduces the `l=720,m=0` maximum absolute error to `3.19e-8` and
passes the exact paired-source library suite (319 passed, 37 ignored), but a
three-pair screen leaves full q1/q8 at `1.256/1.272x` DUCC. Reduced q1 is
`1.019x`; q8 is noisy. The random full/reduced relative-L2 errors remain
`1.864e-4/1.885e-4`. An accuracy-only order-band screen puts the isolated m=0
error at `5.83e-8`; nonzero order bands range `1.32e-4–2.10e-4`, while degree
600–719 is `2.55e-4` and degree 720 is `2.90e-4`. This narrows the next
diagnostic to high-degree nonzero orders and their scaled-prefix/ordinary-
suffix transition. Keep the leaf isolated. Source patch, pair data, tests,
assembly and filtered-fixture hashes are in the [paired zonal leaf archive]
(archive/experiments/native-generic-scalar-synthesis-2026-10-29/zonal-double-single-runtime/README.md).

## Decisions and remaining scope

- The production sidecar is temporary and must be removed after native shared
  scalar synthesis is validated and provides a credible improvement.
- Native-f32 scalar-synthesis prefix/suffix handoffs were rejected for unsafe
  exponent transitions or repeatable regressions. The earlier reflected-ring
  kernel result is superseded by current-source route/control evidence; do not
  restore it unchanged. Details are in the [full archived M6
  ledger](archive/DUCC_PARITY_M6-2026-10-29-pre-native-generic.md).
- The f32 scalar-analysis direct-spectrum/f64-recurrence diagnostic remains
  above DUCC in three cells; tile-512 and mask-reduction screens did not close
  the gap. No conversion-based analysis route is promoted. See the [direct
  spectrum result](data/ducc-parity-m6-2026-10-29/f32-analysis-direct-spectrum-1bd4bd27/README.md)
  and [recurrence profile](data/ducc-parity-m6-2026-10-29/f32-analysis-recurrence-profile-d507297c/README.md).
- Native-f32 reflected spin-1 leaf variants V1–V3 regressed q8 substantially;
  the f64-internal spin-1 adapter still misses three cells. Keep both isolated.
  See the [V1](data/ducc-parity-m6-2026-10-29/f32-spin1-native-f32-reflected-leaf-v1/README.md),
  [V2](data/ducc-parity-m6-2026-10-29/f32-spin1-native-f32-reflected-leaf-v2-packet-scale/README.md),
  [V3](data/ducc-parity-m6-2026-10-29/f32-spin1-native-f32-reflected-leaf-v3-fourrow/README.md)
  and [adapter result](data/ducc-parity-m6-2026-10-29/f32-spin1-f64-internal-backend-v1/README.md).
- Native-generic scalar synthesis has a measured **GAP** in both full cells;
  the reduced cells are **INCONCLUSIVE** after the latest isolated shortcut.
  The other 12 native-f32 focused cells remain **GAP** at the last measured
  native production source; no shared-generic candidates exist for them. Other
  f32 shapes, grids, adjoints and the broader f64 parity matrix remain
  **UNMEASURED** or **GAP**. M6 is open.

**NEXT:** characterize error within the `m=1–32`, `l=600–720` region and
around the scaled-prefix/ordinary-suffix transition. The m=0 paired leaf is
accurate for its zonal probe but does not improve aggregate random accuracy or
full-grid timing. Keep production dispatch unchanged while identifying which
nonzero-order recurrence contributions dominate. Any next compensated leaf
must retain native-f32 runtime state and pass independent accuracy, adjoint,
reset, exceptional-range, and whole-transform timing checks. See the [active
plan](../plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md) and the [paired
zonal leaf record](archive/experiments/native-generic-scalar-synthesis-2026-10-29/zonal-double-single-runtime/README.md).
