# M6 experiment index

This index groups the source-specific evidence behind the current gaps. The
[live ledger](../DUCC_PARITY_M6.md) contains current status; the [full prior
ledger](DUCC_PARITY_M6-2026-10-29-pre-native-generic.md) retains the dated
experiment narratives. DUCC reference for these screens is pinned at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

| Family | Finding | Detailed evidence |
|---|---|---|
| Native generic scalar synthesis | The order-major/reflected kernel was isolated and instantiated in native f32/f64. The 15-pair T720 candidate missed DUCC in all four cells; repeated scaled-prefix leaf variants did not close the gap. Two full-basis m=0 correction variants fixed a narrow zonal probe but used an out-of-scope plan-time f64 recurrence and missed DUCC. Production dispatch was unchanged. | [Candidate and rejected follow-ups, patches, screens, Callgrind, and numerical caveats](experiments/native-generic-scalar-synthesis-2026-10-29/README.md), [m=0 basis dot](experiments/native-generic-scalar-synthesis-2026-10-29/zonal-basis-dotproduct/README.md), [integrated m=0 skip-and-restore](experiments/native-generic-scalar-synthesis-2026-10-29/zonal-integrated-order-skip/README.md) |
| f32 scalar synthesis | Guarded f32-I/O/f64-internal route passes the four GL T720 cells, but does not satisfy native f32. Native prefix/suffix transitions were unsafe or regressed. | [Production checkpoint](../data/ducc-parity-m6-2026-10-29/f32-internal-scalar-synthesis-production/README.md), [suffix transition](../data/ducc-parity-m6-2026-10-28/f32-corrected-transition-2b531d73/README.md), [route census](../data/ducc-parity-m6-2026-10-28/f32-fallback-transition-census/README.md) |
| Reflected scalar synthesis | A prior shared-kernel route looked promising, but current-source routing and adjacent controls did not support promotion. | [Current-source screen](../data/ducc-parity-m6-2026-10-29/f32-reflected-shared-kernel-current-3d46caaf/README.md), [historical screen](../data/ducc-parity-m6-2026-10-28/f32-reflected-shared-kernel-49295a89/README.md) |
| f32 scalar analysis | Native f32 remains far above DUCC. Direct f32 spectra into the f64 recurrence is diagnostic only and still misses in three cells. Tile-512 and mask-reduction trials did not close the gap. | [Direct-spectrum screen](../data/ducc-parity-m6-2026-10-29/f32-analysis-direct-spectrum-1bd4bd27/README.md), [current recurrence profile](../data/ducc-parity-m6-2026-10-29/f32-analysis-recurrence-profile-d507297c/README.md), [tile-512 rejection](../data/ducc-parity-m6-2026-10-29/f32-analysis-tile512-screen-1bd4bd27/README.md) |
| f32 spin-1 synthesis | Native reflected packet variants regressed q8; the f64-internal adapter misses three cells. | [Native V1](../data/ducc-parity-m6-2026-10-29/f32-spin1-native-f32-reflected-leaf-v1/README.md), [V2](../data/ducc-parity-m6-2026-10-29/f32-spin1-native-f32-reflected-leaf-v2-packet-scale/README.md), [V3](../data/ducc-parity-m6-2026-10-29/f32-spin1-native-f32-reflected-leaf-v3-fourrow/README.md), [adapter screen](../data/ducc-parity-m6-2026-10-29/f32-spin1-f64-internal-backend-v1/README.md) |
| f64 performance protection | GL T1023 spin-1 synthesis remained a protected PASS in the scalar-synthesis adapter control. Other f64 gaps still require current-source coverage. | [Production checkpoint](../data/ducc-parity-m6-2026-10-29/f32-internal-scalar-synthesis-production/README.md) |

The repository already contained untracked raw M6 files when this cleanup
started. No benchmark process was running, but those files were left untouched
because their ownership and uniqueness were not established. Do not delete
them as part of plan cleanup. The linked compact reports are the durable
index; raw scratch should be reconciled separately before removal.
