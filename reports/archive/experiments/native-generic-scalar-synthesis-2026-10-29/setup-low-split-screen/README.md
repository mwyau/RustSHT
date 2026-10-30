# Native-f32 split-setup residual screen

**Status: diagnostic only; no production change.** This screen estimates the
AVX2 cost of propagating high/low f32 recurrence state when recurrence factors,
`cos²`, cosine, and the sectoral seed retain low setup parts. The corresponding
scalar replay isolates setup error against a widened reference. It does not
measure a whole transform or establish production acceptance.

## Findings

For 208 high-degree observations at `m=1–16` on selected GL T720 rings, the
ordinary native-f32 recurrence had `1.64484e-4` relative RMS error against a
widened recurrence using the same f32 setup. Replaying setup constants as a
single f32 high part left `4.55908e-4` relative RMS error against the full
widened reference. Carrying f32 high/low setup and recurrence state reduced
that diagnostic error to `2.71612e-7`. A hybrid replay with high/low
correction through only `m=1–2` measured `3.08760e-4`; correction through
`m=1–13` was needed to get below `1e-4` (`9.41334e-5`). Precombining the
leading high/low factor at setup time gave `1.99215e-6` for `m=1–16`; its
hybrid cutoff values are close to the split replay. These are selected
order/ring diagnostic values, not whole-transform errors. The replay is
test-only f64 analysis of native-f32 high/low values; it is not runtime f64
execution.

The pinned AVX2 microkernel screen used 8 f32 lanes, `m=1–16`, 353–360 degree
pairs per order, one equatorial ring broadcast across lanes, 15 rotated paired
runs, and 16 repetitions per run. Median total sweep times were 0.098 ms for
ordinary recurrence, 0.254 ms for residual recurrence with f32 setup, and
0.508 ms for residual recurrence with split setup: 2.583× and 5.210× the
ordinary screen. Applying the measured ratio to raw degree-pair counts gives a
rough added recurrence cost of `2.3%` for `m=1–2` and `15.0%` for `m=1–13`.
This is only an operation-count extrapolation; it excludes geometry-dependent
skips, FFT work and scheduling. A low-order-only whole-transform trial remains
open only if its implementation can keep measured full-transform cost inside
the DUCC acceptance gate.

The follow-up precombined path computes the high/low leading recurrence factor
outside the vector degree loop. Its pinned median was 0.249 ms versus 0.103 ms
ordinary (`2.439x`), compared with `5.150x` for the full split-setup loop. The
precombined replay is `1.99215e-6` relative RMS over `m=1–16`; correction only
through `m=1–2` gives `3.08761e-4`. A raw pair-count model estimates additional
recurrence work of `0.80%` for `m=1–2` and `5.13%` for `m=1–13`. The exact
screen has 15 rotated paired runs. The [V3 disassembly summary](codegen-summary.txt)
records 28 instructions per precombined degree pair and no stack access in its
inner loop. These cost and accuracy results still do not substitute for a
whole-transform measurement.

The diagnostic indicates that narrowing setup constants to one f32 value does
not preserve the widened result, while high/low setup helps in the scalar
replay. Precombining lowers the microkernel cost, but the extra state and
accumulation work remains more than twice the ordinary loop. Neither vector
screen predicts end-to-end scheduling overhead or polar scaled-prefix
behavior. The whole-transform residual integration in the adjacent archive is
rejected and must not be treated as a numerical or performance pass.

## Reproduction and provenance

The source parent is `b6a96aa7161a64f678b614d73478496f50f0551e`. Recreate the
isolated candidate by applying
[`../residual-recurrence-integration/patches/residual-integration-candidate.patch.gz`](../residual-recurrence-integration/patches/residual-integration-candidate.patch.gz)
to that source, then use `scalar_ordered-tests.snapshot.rs` as
`src/legendre/scalar_ordered/tests.rs`. Apply
`accuracy-source-update.patch.gz` to that test file for the later per-order
cutoff breakdown and precombined microkernel. The patch was reapplied and its
result hash verified. Source, test binaries, disassembly, and logs are hashed
in `SHA256SUMS`. Build and run the ignored probes with:

```sh
taskset -c <one-allowed-cpu> env RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  cargo test --release --features parallel --lib \
  native_f32_vector_residual_recurrence_cost_probe -- --ignored --nocapture

taskset -c <one-allowed-cpu> env RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  cargo test --release --features parallel --lib \
  native_f32_ordered_recurrence_vs_accumulation_error_probe -- --ignored --nocapture
```

The host is an AMD Ryzen 9 5950X; each timing used one allowed CPU pinned. Rust
was `1.99.1`, release profile, `parallel` feature, AVX2/FMA via
`-C target-cpu=x86-64-v3`. The benchmark source, deterministic GL geometry,
coefficient formula, exact results, and ratio samples are in the source
snapshot, logs, and `results.json`.
