# Tile-128 prototype numerical test checks

Both Rust test configurations passed against the isolated candidate at source
commit `739275d20987fec35506916cb682f961e8a7ffaf`, plus the uncommitted
tile-128 candidate patch. The tested `src/legendre/synthesis_fast.rs` SHA-256
was `0c0ee956205f782626e6ab0ffc6914cfa9f0d1e20caf5075111a53a9c9449b25`;
the frozen release extension SHA-256 was
`5eeff4dd2087238f18278928ad2af84437d21b7caa38eb360783f9b627cf8620`.

| Command | Passed | Ignored | Failed | Wall time | Peak RSS |
|---|---:|---:|---:|---:|---:|
| `cargo test --workspace --all-features` | 216 | 15 | 0 | 4:37.66 | 446.3 MiB |
| `cargo test --no-default-features` | 177 | 8 | 0 | 6:14.23 | 1.28 GiB |

The totals include the root library, FFT integration, f32 integration, and
workspace-compatibility test targets. The test totals, wall times, and peak RSS are summarized above. Raw build/test
logs, GNU-time files, and host snapshots are local and untracked. Both commands
used Rust 1.99.0, a local temporary directory, and `RUST_TEST_THREADS=1`. No benchmark ran concurrently with either test
suite. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The checks cover the candidate's focused tile, polar-lane, mixed-fallback and
scaled-to-ordinary transition tests, plus workspace compatibility and the
existing DUCC comparisons. These suites did not compare the complete T1023 or
T2047 candidate transforms with DUCC; the later paired benchmark does so for
each process and records the errors in the [q1 comparison](../paired-q1-128/summary.md).
The candidate remains isolated because its T511 stability control and broader
regression matrix are incomplete.

## Targeted extreme finite coefficient fallback

A temporary test in the frozen tile-128 source exercised the scaled fallback
with an out-of-normal-range but finite prepared scalar coefficient of `1e300`
at GL `lmax=511` with 513 latitude points. It asserted that the guard on
ordinary coefficients declined the fast transition, the resulting spectrum
contained only finite values, and the spectrum matched the paired scaled
recurrence reference at the module's existing `2e-11` relative-L2 and
`2e-12` maximum-absolute tolerances. The test passed in both configurations:

- `cargo test --release --lib --features parallel legendre::synthesis_fast::tests::extreme_finite_prepared_coefficient_matches_scaled_reference -- --exact`
- `cargo test --release --no-default-features --lib legendre::synthesis_fast::tests::extreme_finite_prepared_coefficient_matches_scaled_reference -- --exact`

The test-only patch is
[`extreme-finite-fast-route.patch`](extreme-finite-fast-route.patch), SHA-256
`f8607872e7de6d39c37058297f1a159b87df122932a326d2e5822d58174d4a80`. The
candidate source was restored to SHA-256
`0c0ee956205f782626e6ab0ffc6914cfa9f0d1e20caf5075111a53a9c9449b25` after
the test. This internal check does not test every extreme public coefficient
input or establish a full transform accuracy bound for arbitrary finite values.

## Public high-bandlimit extreme monopole

An end-to-end public API test used GL `lmax=511`, `ntheta=512`, `nphi=1024`,
`phi0=0.37`, scalar full `mmax`, and `a00=1e300` with all other coefficients
zero. The synthesized map was finite and matched the analytic constant
`1e300/sqrt(4*pi)` at relative error below `1e-12`. One test passed in both
parallel and no-default-features serial builds:

- `cargo test --release --features parallel --test m6_extreme_fast_route -- --exact high_bandlimit_extreme_finite_monopole_stays_finite`
- `cargo test --release --no-default-features --test m6_extreme_fast_route -- --exact high_bandlimit_extreme_finite_monopole_stays_finite`

The temporary integration-test source is archived as
[`public-extreme-fast-route.rs`](public-extreme-fast-route.rs), SHA-256
`0639d12e278bf375065683911a4b171ca65984acf03a5bbd0b19dd494c10371b`.
The test files were removed from the candidate worktree after both runs; the
kernel source hash is unchanged. These checks exercise a high-bandlimit
monopole and a prepared-coefficient fallback, not every possible extreme
coefficient combination.
