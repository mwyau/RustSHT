# B2 spin-1 synthesis q1/q8 arithmetic-route trace

## Scope and provenance

This is a test-only diagnosis of the q1/q8 split in native-f32 spin-1 synthesis. The traced Rust source is `8f73c67f01531efe13cdb3eb7c4ccc84f986013e`; the pinned comparison source is DUCC 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The isolated trace patch changes tests and test-only observers only; production source and timing binaries are unchanged. The uncompressed patch SHA-256 is `b2d831787a0279a8795237019007c791c7123f5096f02922ceab9fc6292a6ac9`; its compressed archive is listed in `SHA256SUMS`.

The deterministic fixture is GL T720 (`lmax=720`, scalar child `lmax=721`, `mmax=16`, `nphi=34`, `phi0=0`), with f32 E/B coefficients at `l=600..720`, `m=0..16`. The compared northern ring is 5 and its reflected southern ring is 715; the selected order is `m=16`. q1 and q8 use identical geometry, E/B inputs, and prepared channel/pair arrays. The selected trace enters distinct q1 and q8 work scheduling routes. This reduced-order fixture exposes the split; it does not imply that full `mmax=720` q1/q8 use these same routes. The separately archived full-order high-band check found bit-identical q1/q8 outputs.

The host is an AMD Ryzen 9 5950X. The optimized Rust test build used `RUSTFLAGS='-C target-cpu=x86-64-v3'`; runtime selected `pulp::x86::v3::V3` (AVX2/FMA), with 8 f32 lanes. Rust was `1.99.1 (58cf8f9c3 2026-10-13)`. Exact compiler, reference executable, and AVX2-inspection binary hashes are in [`generated/toolchain.txt`](generated/toolchain.txt). A separate GCC `__float128`/libquadmath direct normalized-harmonic reference was used for accuracy; DUCC is not the accuracy oracle in these figures.

The input generator and binary128 reference source are reused from the [cross-family accuracy package](../native-f32-cross-family-accuracy-2026-10-30/README.md), avoiding duplicate harness sources. Their SHA-256 values are `102a521dadf76bbcec56d15b60a347d522ecf64d3e2edf50ec77ab9b62df4373` (`phasec_accuracy_dump.rs`) and `62a162c07baf1ccbee15f0db7d21f84fc77381b8a7284f25a2203a892f20053d` (`phasec_accuracy_reference.cpp`).

## Finding

The earliest route-specific recurrence operand at ring 5 is the latitude factor. q1 constructs `cos²` as `(1-sin(theta))*(1+sin(theta))` near the pole, with each f32 operation rounded; q8's traced general recurrence consumes the prepared f32 cosine directly. The exact inputs to the recurrence therefore differ before degree 17. The q1 route then uses fused `a*cos²+b` and state updates in its reflected order-major kernel. The selected q8 ring/order uses the grouped-ring general scalar recurrence, which computes `cos*current`, `epsilon*previous`, subtraction, then division as separate f32 operations. These are different evaluation paths for the same normalized harmonic calculation, not evidence of a different transform convention.

At ring 5, q1's factor is `0x3f7fd6e4` (`0.9993727207183837890625`). It is 8.240184823e-8 below `cos(theta_f64)^2`; correctly rounded `1-sin_f32^2` is `0x3f7fd6e5`, and rounded `cos_f32^2` is `0x3f7fd6e6`. The q1 factor corresponds to effective cosine `0.99968631115884736`; q8 uses f32 cosine `0.99968636035919189`. The difference is `-4.920034453e-8`.

A one-change high-precision basis replay holds the f32 sine and the captured f32 converted channel coefficients fixed, and changes only q1/q8 cosine geometry. For `m=16`, its relative basis difference grows from `4.92158e-8` at `l=17` to `1.24832e-3` at `l=720` and `1.25594e-3` at `l=721`, monotonically on this ring. The geometry-only replay moves the selected Q spectrum by `2.36371549e-3` and U by `9.13077182e-4`, while the observed Rust q1/q8 difference is `2.24436997e-3` Q and `8.86835300e-4` U. Geometry sensitivity is therefore large enough to explain the route split's scale, but it exceeds the observed split: recurrence, accumulation, and reconstruction differences partly compensate. It is not a complete additive error decomposition.

The independent binary128 full-array spectrum comparison on this reduced-order fixture measures q1 Q/U relative L2 `3.47028120e-4` / `3.61990099e-4`, and q8 `3.04287128e-4` / `3.12280774e-4`. Map relative L2 is `3.46175992e-4` / `3.57951713e-4` for q1 and `3.03035916e-4` / `3.07847387e-4` for q8. The inverse-FFT residual is only `1.15e-7` to `1.18e-7`; the main discrepancy exists in the spectrum before FFT. No accuracy tolerance was supplied, so these measurements do not assign a scientific PASS or FAIL.

### Recurrence and arithmetic checks

- q1's scalar f32 `fmaf` replay matches both traced FMAs on all 704 selected degree-pair rows. The q1 route performs a leading-factor FMA and a recurrence-state FMA; AVX2 disassembly shows packed `vfmadd*ps` operations on YMM registers.
- q8's separate f32 `mul`, `mul`, `sub`, `div` replay matches all 705 selected recurrence rows. Its AVX2 loop emits `vmulps`, `vmulps`, `vsubps`, and `vdivps` across eight f32 lanes. A fused numerator first differs from the separate numerator at degree 19 and differs by one or more ULPs on 316 of 705 updates.
- With the actual q8 f32 rounded cosine and epsilon operands fixed, a Decimal state replay first diverges from the repeatedly rounded f32 state at degree 17. Relative state error is `4.68988e-5` at degree 720, peaking at `4.75785e-5` at degree 712; maximum absolute state error is `1.31841e-4` at degree 715 and maximum distance from the correctly rounded exact state is 553 ULP. Error is oscillatory: relative error decreases on 274 degree transitions. The exponent changes from `-80` to `0` at degree 88.
- Selected-case binary128 errors at ring 5/order 16 are Q `1.90157e-3` (q1) and `3.51361e-4` (q8), U `6.85831e-4` (q1) and `2.13888e-4` (q8). Recurrence and geometry effects dominate the much smaller FFT residual on this fixture.

This diagnosis does not prove that all observed error is unavoidable f32 rounding. It identifies the minimum operations that need controlled comparison: plan-time latitude factor construction; q1's two fused normalized recurrence operations; q8's separately rounded normalized recurrence; channel accumulation and reflected reconstruction. It does not yet identify a remedy that meets a defensible accuracy target at acceptable whole-transform cost.

## Source crosswalk

Line numbers refer to traced source SHA `8f73c67f01531efe13cdb3eb7c4ccc84f986013e`.

| Stage | Rust source and executed operation |
|---|---|
| Route eligibility | `src/transform/spin/synthesis.rs:1363-1374` selects the q1 order-major route only for single-thread full-ring work. q8 uses grouped-ring scheduling. |
| q1 latitude setup | `src/transform/spin/synthesis.rs:2481-2492` initializes the previous spin state from sine and selects the near-pole factored f32 `cos²` formula. |
| q1 recurrence and accumulation | `src/legendre/scalar_ordered/kernel.rs:2037-2082` computes two degree steps, fused leading factors and state transitions, with even/odd coefficient FMAs. |
| q1 reflected reconstruction | `src/transform/spin/synthesis.rs:2513-2523` multiplies the odd part by cosine and forms north/south sum/difference before dividing by sine. |
| q8 selected general path | `src/transform/spin/synthesis.rs:2307-2346` calls the scalar recurrence and accumulates scaled component contributions into the grouped spectra. |
| q8 scalar recurrence | `src/legendre/numeric.rs:402-435` computes the normalized degree recurrence with separate multiply, subtract, and divide operations in its AVX2 f32 vector body and scalar tail. |
| q8 prepared-pair attempt | `src/legendre/scalar_paired/synthesis.rs:1072-1110` initializes/normalizes pair state and selects its fused ordinary pair kernel when the state qualifies; the selected ring 5 trace is in the q8 general fallback. |
| FFT boundary | `src/transform/spin/synthesis.rs:493-510` performs the two inverse longitude FFTs after spectra are populated. |

Pinned DUCC 0.42.0's f32 public synthesis promotes the Legendre work arrays to double precision; the exact source crosswalk for `alm2map`, `Ylmgen`, recurrence scaling and extraction is in the [pinned DUCC numerical-causality report](../numerical-causality-ducc-0.42.0-2026-10-29/REPORT.md). This is an arithmetic precision difference in the reference behavior, not a claim that every Rust f32 result must equal DUCC bitwise.

## Artifacts

- `b2_route_causality.py` regenerates the compact summaries from the captured q1/q8 traces.
- `generated/summary.json` contains provenance, input/trace hashes, operation-replay checks, route-geometry substitutions, binary128 comparisons and FFT residuals.
- `generated/toolchain.txt` records compiler versions, flags, CPU, and executable hashes.
- `generated/validation-tests.txt` records the existing f32 integration and spin-1 unit-test results at the traced source.
- `generated/basis-geometry.csv` contains all `m=16`, `l=16..721` basis values for q1 geometry, q8 geometry, and the one-change `cos(theta_f64)` replay.
- Selected recurrence and accumulation CSVs preserve the degree-by-degree traces. `generated/q8-recurrence-replay.csv` records f32 state versus exact replay. `generated/avx2-recursion-snippets.txt` preserves disassembly and traced binary hash.
- The q1 and q8 raw capture directories are generated by the documented commands but are not checked in; the summarized values and raw-file checksums are preserved here.
- `b2-test-trace.patch.gz` is test-only. Its SHA-256 manifest and summary are covered by `SHA256SUMS`.

## Reproduction

From a clean worktree at the Rust source SHA above, apply the trace patch and reuse the cross-family harness/reference sources:

```sh
REPO=$(git rev-parse --show-toplevel)
PKG="$REPO/reports/archive/experiments/native-f32-spin1-synthesis-arithmetic-route-2026-10-30"
CROSS="$REPO/reports/archive/experiments/native-f32-cross-family-accuracy-2026-10-30"
TRACE_DIR=$(mktemp -d -t rustsht-b2-trace.XXXXXX)
git worktree add --detach "$TRACE_DIR" 8f73c67f01531efe13cdb3eb7c4ccc84f986013e
gzip -dc "$PKG/b2-test-trace.patch.gz" | git -C "$TRACE_DIR" apply
cp "$CROSS/phasec_accuracy_dump.rs" "$TRACE_DIR/examples/"
cd "$TRACE_DIR"
RUSTFLAGS='-C target-cpu=x86-64-v3' RAYON_NUM_THREADS=1 cargo run --release --features parallel --example phasec_accuracy_dump -- diagnostics/q1
PHASEC_SPECTRA_DIR=diagnostics/q1 RUSTFLAGS='-C target-cpu=x86-64-v3' RAYON_NUM_THREADS=1 cargo test --release --features parallel --lib phasec_capture_native_f32_spectra -- --ignored --nocapture
RUSTFLAGS='-C target-cpu=x86-64-v3' RAYON_NUM_THREADS=8 cargo run --release --features parallel --example phasec_accuracy_dump -- diagnostics/q8
PHASEC_SPECTRA_DIR=diagnostics/q8 RUSTFLAGS='-C target-cpu=x86-64-v3' RAYON_NUM_THREADS=8 cargo test --release --features parallel --lib phasec_capture_native_f32_spectra -- --ignored --nocapture
c++ -std=c++20 -O2 -march=x86-64-v3 "$CROSS/phasec_accuracy_reference.cpp" -lquadmath -o diagnostics/phasec_accuracy_reference
diagnostics/phasec_accuracy_reference diagnostics/q1
diagnostics/phasec_accuracy_reference diagnostics/q8
python3 "$PKG/b2_route_causality.py" --q1 diagnostics/q1 --q8 diagnostics/q8 --out "$PKG/generated" --trace-patch-sha256 b2d831787a0279a8795237019007c791c7123f5096f02922ceab9fc6292a6ac9
```

The trace is not a timing binary. Do not use it for performance claims.

## Decision and next step

For this reduced-order fixture, the first route-dependent arithmetic input is q1's rounded factored `cos²` versus q8's rounded cosine; repeated recurrence rounding amplifies the state difference, and q1's fused pair update differs from q8's separate AVX2 operations. The spectrum already carries most of the measured error before FFT. The one-change geometry probe is sufficient to show sensitivity but overpredicts the actual route split, so it cannot be added to the recurrence and accumulation errors as a decomposition.

**NEXT:** finish B1 and B3 full-array independent references and exceptional/adjoint matrices; then add B2 single-mode and full-order FFT-separated maps, exact route-specific accumulation replay, and extreme finite/subnormal inputs. Only after cross-family accuracy is characterized should a narrowly predicted remedy be considered. No science tolerance or numerical PASS is assigned by this package.
