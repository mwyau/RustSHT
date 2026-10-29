# Recent substantive checkpoints

- **2026-10-29 — native-generic scalar synthesis follow-up measured.** An
  exact f32 AVX2 scaler for the common binary exponent `-80` improves three
  cells against the generic parent, but full GL T720 remains 1.25–1.26x DUCC
  and reduced q8 1.08x; reduced q1 is inconclusive near parity. The protected
  f64 control is neutral at q1 and within noise at q8. The near-pole zonal
  result is 0.88% high versus the independent f64 recurrence; pinned DUCC
  f32 agrees with that reference because its scalar Legendre state uses
  double SIMD. A test-only native-f32 double-single arithmetic probe with
  high/low constants reaches about `1e-9` absolute error. A full m=0 basis
  correction reached `1.48e-8` zonal error, but added up to 45% runtime, left
  random-transform error unchanged and used an out-of-scope plan-time f64
  recurrence. Both remain isolated; production dispatch is unchanged. See
  the [candidate archive](archive/experiments/native-generic-scalar-synthesis-2026-10-29/README.md),
  [basis-dot diagnostic](archive/experiments/native-generic-scalar-synthesis-2026-10-29/zonal-basis-dotproduct/README.md)
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
