# Production paired odd-longitude FFT validation (2026-10-22)

**Status:** current-source numerical validation and a focused boundary A/B are
complete; the exhaustive native full-transform sweep will provide the broader
Rust/DUCC performance baseline. Default production dispatch and FFT math were
not changed here. A compile-time-only switch was used to build the pair-off
benchmark control.

The paired scalar f64 odd-`nphi` route is already integrated in
`src/fft.rs`/`src/transform.rs` (production commits `c70b19a3` and
`a2c5535d`), with scalar dispatch at `cc6c5aba`. This report supersedes the
older statement that the code remained only on the experiment branch. The
original paired-FFT report's timings remain historical source measurements.

## Exhaustive production-path comparison

An ignored, test-only route switch selected either the production paired path
or the independent-row path in the same binary and plan. Route counters
verified actual paired forward/inverse row calls; the disabled route recorded
no paired calls. Inputs covered the complete scalar coefficient triangle and
deterministic maps. Each plan/workspace was reused, output buffers began with
non-result sentinels, and the workspace spectrum buffer was poisoned between
the paired and independent calls. The paired and independent results were
compared per coefficient and map sample using the existing `4e-13` scaled
absolute tolerance.

| Build | Cases | Coverage | Successful row pairs | Guard declines | Max relative L2 | Max scaled point error |
|---|---:|---|---:|---:|---:|---:|
| Release, `parallel`, x86-64-v3 | 2,056 | T0–T256 × q1/q8 × A/S × odd-minimum/even-smooth | 65,920 | 0 | `1.91e-15` | `5.91e-14` |
| Release, `--no-default-features`, x86-64-v3 | 1,028 | T0–T256 × serial × A/S × odd-minimum/even-smooth | 32,960 | 0 | `1.91e-15` | `5.91e-14` |

The exhaustive data files contain one row per degree, thread count, longitude
shape and operation. Every integer degree from 0 through 256 appears in each
build. T0 has one latitude and therefore no row pair. The forward route is
intentionally disabled for `nphi=255`: T127 odd-minimum analysis consequently
uses independent rows, while its inverse synthesis still pairs all 64 row
pairs. Even-longitude routes use independent rows in both directions.

- [Parallel q1/q8 production-versus-independent CSV](data/paired-fft-validation-2026-10-22/production-ab.csv)
- [Serial/no-default production-versus-independent CSV](data/paired-fft-validation-2026-10-22/production-ab-serial.csv)

## Safety and edge-case checks

- Direct odd-FFT comparison, zero rows, opposite rows, unequal-amplitude
  fallback, nonfinite/subnormal/extreme finite input and poisoned FFT
  workspace reuse passed. Existing poisoned-workspace coverage repeats the
  paired forward and inverse at N=129, 361 and 509 after filling the FFT
  transform/scratch buffers with NaNs and infinities.
- A new distinct-spectrum test pairs equal-peak rows with energy concentrated
  at frequency 3 and near Nyquist. Forward and inverse results match
  independent transforms at the existing `4e-13` tolerance for N=129, 257,
  361 and 509. This tests the empirical 8:1 row-scale guard against disparate
  Fourier support; it does not prove a per-mode error bound.
- A late-decline fixture uses normal `3e-308` samples. The paired forward
  route writes DC, then detects a subnormal in bin 1 and declines. The row-pair
  wrapper recomputes both complete spectra independently, exactly matching
  the independent result rather than leaving the earlier partial writes.
- A nonfinite inverse spectrum declines to the independent route; the
  neighboring finite row remains equal to its independent inverse.
- The existing GL scalar tests continue to cover roundtrip, normalization,
  basis modes and `S/A/S*/A*` adjoint identities. The exhaustive comparison
  itself tests output reset and workspace reuse for every degree.
- `cargo test --release --features parallel`: unit 268 passed/35 ignored;
  integration suites 4 passed, 11 passed/1 ignored, and 1 workspace test
  passed. `cargo test --release --no-default-features`: unit 219 passed/17
  ignored with the same integration results. Strict Clippy (`-D warnings`)
  passed for both feature configurations, and `cargo fmt --all --check`
  passed. Test-only table benchmark helpers were gated to the parallel test
  configuration so the supported serial build remains warning-free.

The row-component 8:1 check remains an empirical filter. Equal peak amplitudes
do not imply equal energy at each Fourier mode, and the current evidence is
not a rigorous bound for all spectral distributions. The current route also
checks finite/normal inputs, component headroom, produced subnormals and
falls back to independent rows when a guard declines. No tolerance was
changed.

## Same-source paired-on/paired-off boundary screen

To measure the route on the current Legendre policy, the benchmark-only
`rustsht_disable_paired_odd_fft` compile cfg removed only the paired caller
route; both binaries came from production revision `cc6c5aba`, used identical
fixtures and the same pinned DUCC executable. It is absent from the default
build. The screen covers T63/64, T126/127/128 and T255/256, all full and
quarter-order controls (plus the harness's T255 `mmax=1` control), odd-minimum
longitude, A/S and q1/q8. Each variant collected three process pairs and 30
calibrated batches per case, targeting 25 ms per batch. Rust and DUCC were
interleaved within each variant, but all paired-enabled cases ran before all
pair-disabled cases; this route-to-route order is a limitation.

| Operation | Threads | Full-order cases | Geomean paired/disabled Rust latency | All-order cases | Geomean paired/disabled Rust latency |
|---|---:|---:|---:|---:|---:|
| Analysis | q1 | 7 | 0.963x | 15 | 1.002x |
| Analysis | q8 | 7 | 0.943x | 15 | 0.957x |
| Synthesis | q1 | 7 | 0.905x | 15 | 0.934x |
| Synthesis | q8 | 7 | 0.945x | 15 | 0.947x |

The result supports keeping inverse pairing for the sampled full-order cases.
Forward pairing was mixed for reduced-order small-`nphi` analysis: q1 cases at
`nphi=3, 31, 33, 63, 65` were 5–11% slower with pairing; T126 full-order q8
analysis was 2.9% slower. Full-order T64/128/255/256 cases generally favored
pairing. At full T127, forward pairing remains explicitly disabled at
`nphi=255`, as shown by the zero route count in the exhaustive validation.
These measurements are a bounded screening result, not a new threshold
decision: source variants were not alternated case-by-case and worker wait
tails reached 26.9% p90 in both campaigns. Median frequency was 4.538/4.539
GHz and median runqueue wait 1.13%/1.41% for enabled/disabled runs.

All 60 cases completed in each variant; each produced 180 numerical
comparisons with no failure. Maximum relative L2 error versus DUCC was
`2.242e-14` enabled and `2.240e-14` disabled. The merged table and original
per-process distributions are linked here:

- [Paired on/off case CSV](data/paired-fft-validation-2026-10-22/matched-boundary-ab.csv)
- [A/B protocol and host metadata](data/paired-fft-validation-2026-10-22/matched-ab-metadata.json)
- [Paired-enabled process samples](data/paired-fft-validation-2026-10-22/matched-ab-enabled.json)
- [Pair-disabled process samples](data/paired-fft-validation-2026-10-22/matched-ab-disabled.json)

## Source and reproduction

- Production baseline: `cc6c5aba` on `opt/sht-performance-parity`.
- Host: AMD Ryzen 9 5950X, x86-64; default test run used a private q1/q8
  Rayon pool, serial run used `--no-default-features`.
- Rust: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, LLVM 23.1.1.
- Build flags: `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.
- Pinned DUCC for the following full-transform sweep:
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (DUCC 0.42.0). The present
  paired-versus-independent test is an internal Rust comparison and does not
  report DUCC timings.

```sh
RUSTSHT_PAIRED_FFT_VALIDATION_CSV=reports/data/paired-fft-validation-2026-10-22/production-ab.csv \
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
cargo test --release --lib --features parallel \
  paired_scalar_fft_end_to_end_exhaustive_t0_to_t256 -- --ignored --test-threads=1

RUSTSHT_PAIRED_FFT_VALIDATION_CSV=reports/data/paired-fft-validation-2026-10-22/production-ab-serial.csv \
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
cargo test --release --no-default-features --lib \
  paired_scalar_fft_end_to_end_exhaustive_t0_to_t256 -- --ignored --test-threads=1

RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
cargo test --release --lib --features parallel paired_odd_fft -- --test-threads=1

# Paired-on/off whole-transform boundary screen; DUCC remains pinned above.
export DUCC_SRC=/path/to/ducc-42ce
CARGO_TARGET_DIR=target-paired-enabled \
RUSTSHT_FULL_TRANSFORM_RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --production-source-revision cc6c5aba --lmax 63 --lmax 64 --lmax 126 \
  --lmax 127 --lmax 128 --lmax 255 --lmax 256 \
  --longitude-mode odd_minimum --results-dir paired-on --no-report

CARGO_TARGET_DIR=target-paired-disabled \
RUSTSHT_FULL_TRANSFORM_RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs --cfg rustsht_disable_paired_odd_fft' \
python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --production-source-revision cc6c5aba --lmax 63 --lmax 64 --lmax 126 \
  --lmax 127 --lmax 128 --lmax 255 --lmax 256 \
  --longitude-mode odd_minimum --results-dir paired-off --no-report
```
