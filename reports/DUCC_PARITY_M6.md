# M6 — live DUCC parity ledger

**Current source:** production code at `c63cde440d91e3ca2810226163a191199b4ca29d`,
unchanged through report-only HEAD `83dd570d413a9e0c0dd998e1fbfb34f7d8a92650`.
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
| Scalar synthesis, full q1 | Public f64 sidecar `15.836 / 19.131`; native baseline `641.339 / 18.902` | `0.829 [0.823, 0.834]`; native `≈33.93x` | `c63cde440d91e3ca2810226163a191199b4ca29d` / `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** — generic route unmeasured |
| Scalar synthesis, full q8 | Public f64 sidecar `2.790 / 2.884`; native baseline `98.183 / 2.838` | `0.961 [0.934, 0.974]`; native `≈34.60x` | `c63cde440d91e3ca2810226163a191199b4ca29d` / `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** — generic route unmeasured |
| Scalar synthesis, reduced q1 | Public f64 sidecar `6.486 / 8.385`; native baseline `190.225 / 8.317` | `0.775 [0.770, 0.779]`; native `≈22.88x` | `c63cde440d91e3ca2810226163a191199b4ca29d` / `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** — generic route unmeasured |
| Scalar synthesis, reduced q8 | Public f64 sidecar `1.184 / 1.298`; native baseline `54.588 / 1.314` | `0.885 [0.793, 0.918]`; native `≈41.54x` | `c63cde440d91e3ca2810226163a191199b4ca29d` / `fdae803a01273ac7e4e941936f022fa11e770df5` | **GAP** — generic route unmeasured |
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
- Native-generic f32 execution is unmeasured for every focused cell. Other
  f32 shapes, grids, adjoints and the broader f64 parity matrix remain
  **UNMEASURED** or **GAP**. M6 is open.

**NEXT:** genericize `scalar_ordered.rs` and `scalar_ordered/kernel.rs` in a
fresh isolated worktree. First prove native-f32 reflected scalar synthesis on
full/reduced GL T720 q1/q8 against the exact parent and pinned DUCC. Preserve
the current f64 controls and run independent numerical checks before changing
production dispatch. See the [active plan](../plans/m6-ducc-parity-hotpath-cleanup-2026-10-27.md).
