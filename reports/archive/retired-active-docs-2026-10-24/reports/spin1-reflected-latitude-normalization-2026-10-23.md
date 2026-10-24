# Spin-1 reflected latitude normalization

**Status:** implemented and validated on `opt/sht-performance-parity`; parent `dd2221cf66372c379c78b3eed9d17375d4c1b772`.

`prepare_spin_latitudes` now evaluates spin-1 latitude geometry once for the northern member of each reflected pair and copies its sine, negated cosine, and polar order limit to the southern member. Grid constructors define these colatitudes as exact reflections, while independent `sin_cos` calls can differ by a few ULPs. The normalization makes the recurrence geometry and order cutoff exactly symmetric.

## Focused measurement

Native Rust and DUCC 0.42.0 (`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`) were interleaved on an AMD Ryzen 9 5950X. Rust used `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. Each exact T720 case used even-smooth longitude counts, q8 on the same eight physical cores, three independent process pairs, and 10 calibrated ~25 ms timed batches per process (30 batches per backend and case). Values below are warmed medians and p10–p90 in milliseconds.

| Case | Parent | Candidate | DUCC | Candidate change | Candidate / DUCC |
|---|---:|---:|---:|---:|---:|
| CC T720 full (`nphi=1458`) | 15.512 [15.294–16.669] | 15.103 [14.978–15.729] | 6.094 | −2.6% | 2.48× |
| CC T720 quarter (`nphi=378`) | 7.064 [6.989–7.326] | 6.962 [6.878–7.525] | 2.660 | −1.4% | 2.62× |
| F1 T720 full (`nphi=1458`) | 14.481 [14.351–15.205] | 14.131 [14.039–15.322] | 5.868 | −2.4% | 2.41× |
| GL T720 full (`nphi=1458`) | 14.448 [14.332–15.303] | 14.109 [13.942–14.635] | 5.885 | −2.3% | 2.40× |

The candidate median was lower in all four controls, but the p10–p90 ranges overlap. Treat these as small directional gains, not a statistically decisive speedup. The remaining full-transform gap is still about 2.4–2.6× DUCC; parity is not closed.

The candidate matched the parent and DUCC outputs. Maximum candidate-versus-DUCC relative L2 error across these cases was `5.22e-13`; the new unit test checks exact reflected sine, cosine, and order-limit values for GL, CC, and F1 with both odd and even latitude counts.

Validation on the candidate source:

- `cargo fmt --all -- --check`
- `cargo test --release --all-features` — 279 passed, 36 ignored
- `cargo test --release --no-default-features` — 227 passed, 17 ignored
- `cargo clippy --all-features --all-targets -- -D warnings`

Raw host measurements and the temporary runner remain local and untracked. The recorded source revisions, compiler flags, case geometry, pinning, batch protocol, and summaries identify the comparison without adding machine-local paths to the repository.
