# Recent substantive checkpoints

- **2026-10-29 — native-generic scalar synthesis follow-up measured.** A
  precision-gated exact-zero restore shortcut improves the isolated generic
  parent by 5–7% in 15 pairs, but full GL T720 remains 1.24–1.27x DUCC;
  reduced q1 is inconclusive near parity and q8 is too noisy. The protected
  f64 control is neutral at q1 and inconclusive at q8. Skipping four FMAs after
  zero restoration did not prove an added gain. Parent-equivalent outputs do
  not resolve the near-pole accuracy discrepancy. Production dispatch is
  unchanged. See the [candidate archive](archive/experiments/native-generic-scalar-synthesis-2026-10-29/README.md)
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
