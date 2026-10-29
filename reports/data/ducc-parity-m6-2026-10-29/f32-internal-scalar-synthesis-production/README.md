# Production f32-I/O / f64-internal scalar synthesis

## Decision and scope

The Stage B harness showed that the existing shared f64 transform could reach
DUCC with complete f32 input/output conversion. This bounded production
candidate installs that route for the four measured GL T720 spin-0 synthesis
geometries only. It reuses the existing f64 plan, Legendre recurrence and FFT;
it does not add another maintained recurrence. Ordinary finite inputs use the
f64 child plan and reusable conversion buffers. Subnormal, nonfinite, and
extreme inputs, plus output narrowing outside the normal finite f32 range,
decline to the existing native-f32 transform.

The native reflected-ring scalar-synthesis patch from Stage A remains rejected:
on its current-source 15-pair run it was still 13.92–18.75x DUCC and had
repeatable adjacent regressions. The internal-f64 route closes all four
scalar-synthesis T720 cells, with the measured q8 full cell closest to parity
at 0.961x DUCC. This is not f32-family parity: all 12 scalar-analysis and
spin-1 cells remain GAP.

## Provenance

- Source parent: `17a23bd1345fa4c5aad2bc1c67cfada8d2e9c2d4`.
- Candidate used for timed builds: exact patch SHA256
  `19a19bc691dd0fa4aad95a9cd0989bd9a02479a08d5c8febe118dcfbd50cd9e9`.
  The compressed patch is under `sources/candidate-built.patch.gz`.
- Current production-only diff (includes the v2 workspace-check placement):
  SHA256 `76ae61ebb68cd174a0c9c56f13e78a2e3cd040545cdf60082ba0a5a17adb7a9a`.
  The only difference from the timed production implementation is subsequent
  test-only additions; the production code is byte-for-byte unchanged.
- Pinned DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0).
- Rust: rustc 1.99.1 / LLVM 23.1.1, `--release --features parallel`,
  `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.
- DUCC: GCC 15.2, `-O3 -ffast-math -ffp-contract=fast
  -march=x86-64-v3`; benchmark driver `-fno-fast-math`.
- Host ISA: AMD Ryzen 9 5950X, AVX2/FMA. q1 affinity `[0]`; q8 affinity
  `[0,2,4,6,8,10,12,14]`. Warm calls targeted 25 ms batches. Plans and
  workspaces were created before timing. Each pair interleaved parent,
  candidate and DUCC process order. The reference was DUCC's native f32 API.
- Fixtures came from `scripts/benchmark_ducc_f32_native.py::make_fixture`,
  `phi0=0.23`; their hashes match the Stage B fixture catalog copied here.
  All timed synthesis entries include promotion, transform, FFT, narrowing and
  output writeback. Per-stage conversion time was not separately instrumented
  in this production timing run.
- Binary SHA256: Rust parent
  `7552d0705709309a15988c78627a6e0d6a54edce4161085b13ba2dc3dfa41bb4`, Rust
  candidate
  `8e462e4f574f1614309dd437dbb852630739344589581ee50353fe142ae18e20`, DUCC
  f32 `acc82387af2c42aed53b25678e3872157ed3fb6e5d5874322badc472a1d35d5d`.

## T720 scalar-synthesis results

Times are medians in milliseconds over 15 paired comparisons. The confidence
interval is the paired Rust/DUCC ratio bootstrap interval recorded by the
runner. `C/P` is the paired candidate/parent ratio.

| Cell | Parent / Rust / DUCC ms | Rust/DUCC [95% CI] | Rust/parent | Max rel-L2 vs DUCC | Status |
|---|---:|---:|---:|---:|---|
| Full q1 | 645.518 / 15.836 / 19.131 | 0.829 [0.823, 0.834] | 0.0245 | 3.89e-8 | **PASS** |
| Reduced q1 | 191.266 / 6.486 / 8.385 | 0.775 [0.770, 0.779] | 0.0340 | 3.88e-8 | **PASS** |
| Full q8 | 98.573 / 2.790 / 2.884 | 0.961 [0.934, 0.974] | 0.0281 | 3.89e-8 | **PASS** |
| Reduced q8 | 56.005 / 1.184 / 1.298 | 0.885 [0.793, 0.918] | 0.0217 | 3.88e-8 | **PASS** |

The target output comparison is against native DUCC f32; all workers also
reported finite, repeat-consistent output and poisoned-output reset. The
separate Rust parent output differs by about `1.72e-4` relative L2 because the
parent uses native f32 recurrence, while the candidate matches DUCC's f64
internal behavior more closely.

## Adjacent controls and remaining matrix

The current-source 16-cell table is `results-16-cell.csv`. It reports complete
warm-call times, Rust/DUCC medians, paired counts, recorded uncertainty, route,
accuracy evidence, and conversion/allocation measurement limits. The four
spin-0 synthesis cells are **PASS**; the other 12 cells are **GAP**. Their
Rust/DUCC ratios are:

| Family | Full q1/q8 | Reduced q1/q8 | Status |
|---|---:|---:|---|
| Spin-0 synthesis | 0.829 / 0.961 | 0.775 / 0.885 | **PASS** |
| Spin-0 analysis | 31.97 / 26.02 | 22.77 / 19.74 | **GAP** |
| Spin-1 synthesis | 35.92 / 40.68 | 34.57 / 45.29 | **GAP** |
| Spin-1 analysis | 36.07 / 31.02 | 31.59 / 26.67 | **GAP** |

The scalar-analysis checks that could see the newly provisioned eligible-plan
workspace were repeated: full and reduced q8 and reduced q1 have 15 current
source pairs; full q1 has three. Full q8 was 0.994x parent with a paired
95% interval `[0.989, 0.996]`. Reduced q8 was 1.005x parent with interval
`[0.990, 1.065]`, so it shows no established regression. The reduced-q1
15-pair interval is `[0.990, 1.001]`. q8 summary output was bitwise identical
to the parent. The q1 full scalar-analysis screen measured 0.995x parent and
31.97x DUCC; its large performance gap is unambiguous, but this three-pair
screen is not a precision estimate for a prospective analysis optimization.

Spin-1 paths stay on native f32. The q8 reduced synthesis control was extended
to 15 pairs (1.000x parent, 95% paired interval `[0.993, 1.071]`); other
spin-1 cells retain three-pair directional controls from the v1 source
candidate because the v2 change does not construct an adapter for spin-1
plans or change their dispatch. Their 26.67–40.68x analysis / 34.57–40.68x
synthesis gaps are large. These controls do not certify all-cell parity.

The protected f64 GL T1023 spin-1 synthesis control used 15 pairs. Candidate /
parent was `0.984x` q1 (89.948 / 88.822 ms) and `0.993x` q8 (14.282 / 14.206
ms); candidate / DUCC was `0.740x` and `0.715x`. Output was bitwise equal to
the parent, and all repeat/reset checks passed.

## Workspace and numerical evidence

The f32 plan/workspace adapter adds reusable full-mmax live storage of 29.7 MB
at q1 and 32.6 MB at q8 over the native parent; reduced-mmax adds 8.6 MB and
10.0 MB. Conversion buffers account for 12.57 MB full and 4.01 MB reduced.
After warm-up the measured synthesis makes zero allocation calls. Plan and
workspace construction used 80 / 1258 allocation calls full q1 / q8 and 80 /
972 reduced q1 / q8. These capacities are specific to the tested Rayon worker
counts. The eligible scalar plan reserves the sidecar even when reused for
another operation, so the memory increase is part of the policy cost.

Focused tests verified the route uses the existing f64 `ShtPlan`, repeats
deterministically after poisoned output, and declines min-subnormal,
min-normal-to-subnormal-output, maximum-finite and infinity cases. Independent
T720 DC/dipole and sectoral modes, adjoints, poles/sectoral controls, SIMD
tails, exceptional scaling, and reset tests passed in the existing f32 suite.
The f32 numerical policy is documented in
[`docs/F32_INTERNAL_PRECISION.md`](../../../../docs/F32_INTERNAL_PRECISION.md).

## Validation run

- `cargo fmt --check` — passed.
- `cargo test --release --features parallel --test f32` — 14 passed, 1 ignored.
- `cargo test --release --features parallel --lib f32_t720_internal_scalar_synthesis`
  — 2 passed (shared-plan route and guarded range declines).
- `cargo test --release --features parallel --lib f32_internal_scalar_synthesis_stays_scoped`
  — 1 passed (T255/T511/T1023 remain native f32).
- `cargo test --release --features parallel --test extreme_fft_scaling` — 4 passed.
- `cargo check --all-features` — passed.
- `cargo check --no-default-features` — still fails on the parent at the
  existing unconditional `synthesize_hemisphere_pair_block_dispatched` import
  in `src/legendre/scalar_paired.rs:843`; this candidate does not touch it.

Full paired result details are in the compressed `pair-data/` files; CPU
telemetry was removed from committed artifacts. `SHA256SUMS.txt` covers this
package. This checkpoint closes scalar-synthesis parity only. The next
experiment is a bounded high-band spin-1 route study using the existing
hemisphere scheduler and shared algorithms, with explicit q1/q8 route counters
and complete f32 public-call timing. M6 remains open.
