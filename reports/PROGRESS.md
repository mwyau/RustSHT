# Recent substantive checkpoints

- **2026-10-29 — native-f32 paired zonal leaf and order-band screen.** The
  integrated f32 double-single recurrence corrects the isolated T720
  `l=720,m=0` probe to `3.19e-8` maximum absolute error and passes the exact
  paired-source library suite (319 passed, 37 ignored). Its three-pair full
  q1/q8 timings remain 1.256/1.272x DUCC; random-transform relative-L2 error
  is unchanged. The intersection screen measures `m=1–32,l=600–719` at
  `3.25e-4` relative L2. Ringwise errors peak at polar-end rings 5/717;
  order bands put `m=5–8` at `4.61e-4` relative L2 and `m=9–16` at
  `3.42e-4` maximum absolute error. The leaf remains isolated.
  See the [paired-leaf archive](archive/experiments/native-generic-scalar-synthesis-2026-10-29/zonal-double-single-runtime/README.md)
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
