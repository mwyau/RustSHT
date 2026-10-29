# First-order residual recurrence integration screen

**Status: rejected test-only candidate.** The generic native-f32 route and a
first-order residual state were connected through both the scaled prefix and
ordinary vector suffix. The route stayed isolated; production dispatch and
tracked production code were not changed.

## Matched whole-transform screen

The four GL T720 spin-0 synthesis cells used identical deterministic f32
fixtures and three interleaved fresh-process rounds. Rust and DUCC each reused
a plan and workspace, warmed them, then ran 25 ms calibrated batches. Rust
used native f32 inputs, recurrence, coefficients, FFT storage and outputs,
with AVX2/FMA and eight f32 lanes. DUCC was built from pinned 0.42.0 with the
recorded x86-64-v3 flags. The production parent uses the f64 child-plan
sidecar; it is included only as the exact parent control. Ratio ranges below
are the three paired process observations, not confidence intervals.

| Cell | Parent sidecar / DUCC | Generic f32 baseline / DUCC | Residual f32 / DUCC | Residual relative L2 to DUCC |
|---|---:|---:|---:|---:|
| Full q1 | 0.827 (0.819–0.834) | 1.228 (1.226–1.251) | 2.526 (2.515–2.535) | `1.8880e-4` |
| Full q8 | 0.841 (0.780–0.845) | 1.272 (1.113–1.282) | 2.419 (2.278–2.420) | `1.8880e-4` |
| Reduced q1 | 0.778 (0.760–0.779) | 0.990 (0.972–0.992) | 2.227 (2.182–2.242) | `1.8975e-4` |
| Reduced q8 | 0.803 (0.775–0.808) | 1.001 (0.997–1.025) | 2.221 (2.205–2.227) | `1.8975e-4` |

The residual route is over twice DUCC in every cell. Its full-transform error
is essentially unchanged from the native generic baseline (`1.8676e-4` full,
`1.8914e-4` reduced). Candidate and baseline maps differ by only
`2.60e-5` relative L2 full and `3.15e-5` reduced. This residual update does not
address the dominant DUCC difference and is not suitable for production.

## Numerical interpretation

A 208-value high-degree replay (`m=1–16`, rings 0–5, 360 and 715–720) compares
the kernel against a widened recurrence built from the same f32 geometry,
recurrence tables and coefficient pairs. The ordinary f32 recurrence had
`1.6448e-4` relative RMS error. After residual state propagation and high/low
accumulation, actual output error against that same-setup widened recurrence
was `1.49e-6` relative RMS. This shows that the residual state corrects most
rounding in that isolated recurrence replay.

The full transform still differs from DUCC by about `1.9e-4`. The prior
one-at-a-time setup screen measured material amplification from rounded f32
geometry, recurrence factors and sectoral initialization; its direct source
comparison found matching formulas and nearest-f32 tables. This screen
therefore rejects the all-order residual integration and narrows the next
work to the remaining precision loss in prepared inputs/constants. The replay
is not an independent full-transform acceptance test.

## Verification and exact source state

- `cargo check --release --features parallel --lib`: passed.
- `cargo fmt --all -- --check`: passed.
- Strict release Clippy for the library and tests: passed.
- Focused scalar-ordered unit suite: 14 passed, 3 ignored.
- Focused native-f32 unit filter: 3 passed, 5 ignored.
- T720 scalar synthesis module filter: 4 passed, 1 ignored; the f32 integration
  filter `f32_t720_` passed 3 tests, including analytic low-degree modes,
  sectoral synthesis, adjoint and reset checks.
- The ignored decomposition probe was adapted only in this worktree to remove
  a private bitwise-output assertion; the public mathematical and reset tests
  were not weakened. The adapted diagnostic reports the widened replay error
  above.

The benchmark source base was `b6a96aa7161a64f678b614d73478496f50f0551e`; its
source-diff SHA at timing was
`704666bca0b5fb71adc3cc7346c1f36dbc5d49bb98c99d17c4ea66d0fea5c6ba`. A later
Clippy annotation and test-only diagnostic edit produced the preserved working
patch SHA `bcc2687a714de94254aa7e1e78acd31f8dce396e6c5707aa10e62aa4fee4e515`.
The timing binary SHA was
`3d7eaa76139f53b849629f255d6242de2b0b782447bf98bfe2b06b04e800c7f5`; a
post-screen rebuild after those non-runtime edits was
`f8689610c443b8b4773664566b9bc27869e0191372f98dbb805a3f7154481c44`.
The exact-parent binary SHA was
`8e462e4f574f1614309dd437dbb852630739344589581ee50353fe142ae18e20`; generic
baseline binary SHA was
`d9158760d31937718f43e2ad09542a60ca2ca90e0018164238b2445345d3d932`; pinned
DUCC binary SHA was
`be18c89ea434258e4a07c28ccd8fd31d6262ac3503dcaf81d8aa5176415f1484`.

Rust used `rustc 1.99.1`, release, `-C target-cpu=x86-64-v3
-Aunexpected_cfgs`, parallel feature. DUCC source was clean at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, built with GCC 15.2, `-O3
-ffast-math -ffp-contract=fast -march=x86-64-v3` (the worker driver uses
`-fno-fast-math -ffp-contract=fast`). The host was an AMD Ryzen 9 5950X; q1
was pinned to CPU 0 and q8 to CPUs 0–7. Exact per-pair timings, output errors,
fixture hashes and binary hashes are in [`results.json`](results.json).

The complete isolated source patch is preserved in
[`patches/residual-integration-candidate.patch.gz`](patches/residual-integration-candidate.patch.gz).
It is not a production patch. The f64 sidecar remains the production parent
until a native generic candidate passes the required numerical and performance
gates.
