# Native-f32 spin-1 synthesis full-order binary128 screen

## Scope and provenance

This diagnostic compares the integrated native-f32 B2 implementation with an
independent GCC binary128/libquadmath reference. The tested source is
`f01fb9b161769173397e50dae6c76aae97351c7f`; its `src/` tree is identical to
the current `dev` source and includes B2 integration commit
`db93f14268e0320cc10d5e0e800f0143f41d657a`. The pinned DUCC source was not used
in this accuracy diagnostic; f32/DUCC performance remains in the separate
15-pair timing matrix.

The GL T720 fixture uses `lmax=mmax=720`, 721 latitude rings, `nphi=1458` and
`phi0=0`. Deterministic f32 E/B inputs activate every order, with nonzero
coefficients for `l=max(600,m)..720`. The reference independently converts
spin-1 coefficients to two scalar channels, evaluates normalized scalar
harmonics and computes the complete pre-inverse-FFT spectrum for every order
and ring. It then directly evaluates inverse Fourier sums at selected pixels.
Matched f64 controls consume exact f32 input values widened componentwise.

The fixture and reference are test-only. No production transform source was
changed. Source, compiler, harness, test patch, executable and fixture hashes
are recorded in [`results.json`](results.json), and all retained files are
checked by [`SHA256SUMS`](SHA256SUMS).

## Results

| Full-order spectrum | Native-f32 relative L2 | Matched-f64 relative L2 | Max absolute f32 error | Max envelope-scaled error |
|---|---:|---:|---:|---:|
| Q | `1.4670325630e-4` | `1.3652559145e-13` | `2.3956642735e-2` at ring 0 | `1.3386727273` at ring 482 |
| U | `1.3865401393e-4` | `1.3340454701e-13` | `2.5124916332e-2` at ring 0 | `0.9882410384` at ring 239 |

The comparison covers all 721 orders on all 721 rings for the high-degree
fixture. q1 and q8 inputs, spectra, maps and geometry are byte-identical on
this full-order route. Dispatch metadata reports execution capacity 1/8,
ring-group sizes 768/64, no cached scalar coefficients, and 1/12 recurrence
workers respectively.

Channel preparation is small relative to the largest downstream errors in the
sampled cases: its relative L2 error is about `1.6e-8` to `6.3e-8` for the
recorded orders. At `m=1`, ring 0, the absolute channel-preparation error is
`2.86e-4` for Q and `1.22e-4` for U; after feeding the rounded f32 channels to
the independent recurrence, the remaining spectrum errors are `2.41e-2` and
`2.52e-2`. At `m=719`, ring 482, Q's binary128 spectrum is about
`1.16e-46 + 5.17e-46 i`, while the f32 spectrum contains the minimum positive
subnormal in both components. These observations point to numerical loss
after channel preparation, but the first recurrence FMA/state-update
divergence has not been traced.

The independent inverse-Fourier check samples selected pixels only. For the
recorded ring-0/longitude-0 example, Q differs by `0.08466` and U by `0.06281`;
the f32 inverse-spectrum residuals are `8.79e-7` and `1.23e-6`. This confirms
that the selected map errors are already present in the spectrum. The separate
full-array extension below now checks the whole map; neither result establishes
a scientific tolerance or numerical acceptance PASS.

## Full-array map and FFT-stage extension (2026-10-30)

The isolated observer now saves the actual f32 and f64 Q/U spectra immediately
before inverse longitude FFT. It reruns spin synthesis and asserts that the
maps are bit-identical to the deterministic dump fixture. q1 and q8 inputs,
spectra and maps have matching hashes. The independent binary128 channel and
harmonic calculation writes the complete reference spectra for all 721 rings
and valid orders, rounded once to binary64; NumPy 2.3.5 pocketfft performs the
inverse transform for the full-map reference. Its selected ring-0,
longitude-0 values agree with the preexisting direct-binary128 pixel calculation
within `4e-15` absolute.

| Component | f32 spectrum vs BQ rel L2 | f32 full-map vs BQ rel L2 | f32 map max envelope-scaled | f64 spectrum / map rel L2 | f32 inverse-FFT residual rel L2 |
|---|---:|---:|---:|---:|---:|
| Q | `1.46703256e-4` | `1.46456280e-4` | `3.31179327e-3` at ring 0, longitude 1379 | `1.3653e-13` / `1.3614e-13` | `1.4222e-7` |
| U | `1.38654014e-4` | `1.38104893e-4` | `2.65182005e-3` at ring 0, longitude 1189 | `1.3340e-13` / `1.3255e-13` | `1.4156e-7` |

The BQ spectrum error and full-map error are nearly equal in scale, while the
f32 inverse-FFT residual is about three orders smaller. This isolates the main
error before inverse FFT for this high-band fixture; it does not yet divide
that spectrum error across coefficient-channel preparation, recurrence and
accumulation for every order. The f64 spectra/maps independently agree with the
same reference near `1.3e-13`. This GL-only result was not an accuracy PASS.
The cross-grid, selected-mode, exceptional-range and adjoint extensions below
add coverage but still do not assign a scientific tolerance or accept B2. The
full summary, per-order spectra tables,
observer patch, reference source, exact hashes and reproduction commands are
in `generated/full-array-map-summary.json`, `generated/*-spectrum-by-order.csv`,
and `fullarray-map-provenance.txt`.

## Cross-grid, selected-mode and range extension (2026-10-30)

The same all-order, `l=600..720` binary128 channel/harmonic reference was run
on Fejer-1 T720 (`ntheta=721`) and Clenshaw-Curtis T720 (`ntheta=722`), with
`nphi=1458`, `phi0=0`, and all valid `m=0..720`. F1 q1 Q/U map relative-L2
errors are `6.5049e-5` / `7.4006e-5`; CC q1 errors are `6.1782e-5` /
`6.7531e-5`. Matched f64 controls are `1.8–2.2e-13` on F1 and
`7.5–7.7e-14` on CC. The inverse-FFT residual remains `1.42e-7` on both
grids, so the dominant discrepancy is in the pre-FFT spectrum. The full
per-grid results and q1/q8 file hashes are in the cross-family package's
`results.json`.

This cross-grid extension uses the geometry-aware reference revisions whose
source and executable hashes are recorded in that package's provenance. The
original GL-only source and executable hashes above remain the provenance for
the original GL capture; the extension did not replace those historical
measurements.

On F1, isolated high-degree modes include `(l=600,m=1,E.re)`,
`(720,1,E.im)`, `(650,5,B.re)`, `(720,16,B.im)`, `(720,719,E.re)`, and
`(720,720,E.re)`. Their map relative-L2 errors range from `1.13e-5` to
`1.94e-3`; the largest is Q for `(720,1,E.im)`. The CC `(720,1,E.re)` case
is `1.127e-3` Q and `1.685e-3` U at q1. q1/q8 F1 mode outputs are
bitwise-equal; the CC selected-mode q1/q8 outputs are not, and each is
compared independently with the same binary128 reference. These low-order,
near-pole-sensitive cases exceed the provisional `5.16e-4` operation-count
screen described in the cross-family report. This is a numerical GAP, not a
tolerance change.

For `(l=720,m=720,E.re)` on F1, the coefficient was scaled through the
minimum positive subnormal (`0x00000001`), `2^-140` (`0x00000200`), minimum
normal (`0x00800000`) and `2^120` (`0x7b800000`). The first two cases have
relative errors from `0.035` to `1` because expected map values are zero or
subnormal; their maximum absolute errors are at most `8.01e-44`. At minimum
normal and `2^120`, relative-L2 errors are `1.16e-5–1.63e-5`; the large case
has no nonfinite output. q1/q8 f32 outputs match for all four cases. Detailed
component and FFT-stage values are in the cross-family `results.json`.

High-band adjoint identities were also checked on GL, CC and F1 at q1/q8.
The largest condition-scaled f32 residual is `2.64e-7` for synthesis,
`1.79e-7` for spin analysis, and `5.02e-9` for scalar analysis. These establish
adjoint self-consistency for these probes, not closeness to an independent
mathematical reference. B2 F1 synthesis maps are byte-identical q1/q8; the
CC q1/q8 maps differ by `1.395e-4` (Q) and `1.318e-4` (U) relative L2, and
each thread route is compared separately with binary128. The reduced-order
q1/q8 arithmetic cause remains the existing trace at `dc1aa4f`; it was not
repeated here.

## Reproduction

From a clean detached worktree at the tested source commit, apply
`test-only-spectrum-capture.patch.gz` and copy
`phasec_fullorder_accuracy_dump.rs` into `examples/`. Build and generate the
q1 and q8 fixtures with:

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' RAYON_NUM_THREADS=1 \
  cargo run --release --features parallel --example phasec_fullorder_accuracy_dump -- diagnostics/q1
PHASEC_SPECTRA_DIR=diagnostics/q1 RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' RAYON_NUM_THREADS=1 \
  cargo test --release --features parallel --lib phasec_capture_spin_synthesis_fullorder_spectra -- --ignored --nocapture
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' RAYON_NUM_THREADS=8 \
  cargo run --release --features parallel --example phasec_fullorder_accuracy_dump -- diagnostics/q8
PHASEC_SPECTRA_DIR=diagnostics/q8 RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' RAYON_NUM_THREADS=8 \
  cargo test --release --features parallel --lib phasec_capture_spin_synthesis_fullorder_spectra -- --ignored --nocapture
c++ -std=c++20 -O2 -march=x86-64-v3 b2_spin1_synthesis_binary128_reference.cpp -lquadmath -o diagnostics/b2-reference
diagnostics/b2-reference diagnostics/q1
diagnostics/b2-reference diagnostics/q8
```

The main reference run regenerates the summary CSVs. Optional
`B2_CHANNEL_ONLY=1` and `B2_CHANNEL_PROPAGATION_ONLY=1` modes produce the
channel comparison tables. Full raw spectrum arrays and other large temporary
outputs remain outside the repository; the compact all-order summary, selected
rings/pixels, full-map summary, per-order spectrum tables, channel evidence and
exact hashes are preserved here.

To reproduce the full-map extension, use the same fixture directory after the
dump example has created q1/q8 inputs, apply
`fullarray-spin-spectrum-capture.patch.gz`, then run the ignored capture test
with `PHASEC_FULLARRAY_DIR` set to each q directory. Compile
`fullarray_map_reference.cpp` from the repository root and run:

```sh
PHASEC_FULLARRAY_DIR=diagnostics/q1 RUSTFLAGS='-C target-cpu=x86-64-v3' RAYON_NUM_THREADS=1 cargo test --release --features parallel --lib phasec_capture_fullarray_spin_synthesis_spectra -- --ignored --nocapture
PHASEC_FULLARRAY_DIR=diagnostics/q8 RUSTFLAGS='-C target-cpu=x86-64-v3' RAYON_NUM_THREADS=8 cargo test --release --features parallel --lib phasec_capture_fullarray_spin_synthesis_spectra -- --ignored --nocapture
g++ -std=c++20 -O2 -march=x86-64-v3 reports/archive/experiments/native-f32-spin1-synthesis-fullorder-accuracy-2026-10-30/fullarray_map_reference.cpp -lquadmath -o diagnostics/fullarray-map-reference
diagnostics/fullarray-map-reference diagnostics/q1
python3 reports/archive/experiments/native-f32-spin1-synthesis-fullorder-accuracy-2026-10-30/fullarray_map_compare.py --q1 diagnostics/q1 --q8 diagnostics/q8 --out diagnostics/full-array-map-summary.json
```

The test-only patch and generated binary128-derived spectrum files do not
modify production code; raw arrays are not retained in Git.
