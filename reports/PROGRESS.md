# Recent substantive checkpoints

- **2026-10-29 — native-generic scalar synthesis candidate measured.** The
  isolated 15-pair f32 GL T720 candidate missed DUCC in all four cells
  (`1.054–1.355x`); vector-rescale follow-ups regressed, and suffix inlining
  showed no repeatable gain. Production dispatch is unchanged, and a
  high-degree near-pole accuracy discrepancy remains. See the [candidate record](archive/experiments/native-generic-scalar-synthesis-2026-10-29/README.md)
  and [live ledger](DUCC_PARITY_M6.md).
- **2026-10-29 — current scalar-synthesis sidecar timing.** The f32-I/O/f64
  route passes four GL T720 spin-0 synthesis cells, with Rust/DUCC `0.775–0.961`.
  This does not satisfy native-f32 execution; the isolated generic candidate
  remains a measured GAP. See the [checkpoint](data/ducc-parity-m6-2026-10-29/f32-internal-scalar-synthesis-production/README.md).
- **2026-10-29 — remaining focused f32 families have material gaps.** Last
  measured native-f32 GL T720 scalar analysis and spin-1 analysis/synthesis
  range from about `20–47x` DUCC. See the [16-cell source evidence](data/ducc-parity-m6-2026-10-29/f32-io-f64-internal-stage-b/README.md).
- **2026-10-29 — recent analysis and spin-1 candidates rejected.** Scalar
  analysis tile/mask screens did not close parity; reflected spin-1 packet
  variants regressed q8. Keep their source patches and results archived; do
  not repeat them unchanged. See the [live ledger](DUCC_PARITY_M6.md).
- **2026-10-29 — f64 performance control preserved.** GL T1023 full spin-1
  synthesis stayed bitwise equal to its parent and below DUCC at q1/q8 during
  the sidecar validation. Continue to protect it during genericization.
