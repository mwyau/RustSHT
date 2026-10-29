# T720 f32-I/O/f64-internal spin-1 reflected-route screen

## Decision

Do not promote either isolated spin-synthesis route. The complete f32 public-call adapter that invokes two f64 scalar transforms remains above pinned native-f32 DUCC in three of four screened cells; its only near-parity result is full-mmax q1. Enabling the existing hemisphere dual-channel route at T720 and changing its scheduler to order-major both ran slower than the existing f64 general-spin route. The order-major candidate still measured 2.27–3.97x DUCC in reduced/full q8 in its three-pair screen. These are quantified route rejections, not parity claims.

The required scalar child `lmax=721` is mathematically necessary for the spin-to-scalar conversion: dropping the top degree caused 1.37622e-2 relative error in Q. Preserve that degree if a future scalar-channel route is revisited.

## Provenance and setup

- Parent production source: `c63cde440d91e3ca2810226163a191199b4ca29d`.
- Exact patch contents are retained as `two-scalar-channel-adapter.patch.gz`, `ring-major-reflected.patch.gz`, `order-major-route.patch.gz`, and `order-major-reflected.patch.gz`. The SHA-256 values below are for their decompressed patch bytes: two-scalar `b78a1ef88e7d829b9980eb7c96da87fe17c7809a215ab985598d0c3ba32a3eac`; ring-major `b605bda4e7e5bfc28c44ba20b15cb0ede565064703005f3037b82b6ab471b3a4`; order-major route `c060940909929d6b7140dd491c1119632718159e75df4aea968cc51086839a2e`; scheduler-only `32e8785e935e8105d487220715aae5fd9f3b4862e6661b4a1388aeed32b77598`.
- DUCC: native f32 API, version 0.42.0, source `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; binary SHA-256 `ead63f43909078c3b15dcadf69b4059be0b51c48363f5c9e8b130c2ac7758987`.
- Rust: rustc 1.99.1 / LLVM 23.1.1, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; DUCC GCC 15.2, `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, driver `-fno-fast-math`.
- Host: AMD Ryzen 9 5950X, AVX2/FMA. q1 used one pinned physical core; q8 used eight pinned physical cores. Per-worker telemetry with the exact placement is retained in the local raw results archive.
- Geometry: GL T720, `ntheta=721`, `phi0=0.23`, even longitude; full `mmax=720,nphi=1458`, reduced `mmax=180,nphi=378`. Fixtures match Stage B package hashes.
- Each f32-I/O/f64-internal cell used three interleaved warm public-call pairs. Conversion, spin-to-scalar preparation, transform, FFT, inverse-sine correction, narrowing, and output writeback are timed; plans/workspaces and buffers are reused. Native DUCC f32 is the reference.
- Candidate binaries: two-scalar adapter `15ee6d07359664877df3c6cfc5a44798b32443b8d5b93e840f908345e1182468`; order-major reflected route `ab106c385e4af56fbd0151d1242272d5ebc3632502a2d0dcf9128cbbf2fc221d`. Native-f32 binary `8e462e4f574f1614309dd437dbb852630739344589581ee50353fe142ae18e20`.

## Complete-call two-scalar adapter

| GL T720 spin-1 synthesis | Internal f64 ms | DUCC f32 ms | Paired Rust/DUCC median [95% bootstrap interval] | Error vs DUCC max rel-L2 | Status |
|---|---:|---:|---:|---:|---|
| Full q1 | 44.959 | 44.548 | 1.009 [0.991, 1.011] | 3.91e-8 | **INCONCLUSIVE** |
| Reduced q1 | 29.309 | 18.521 | 1.566 [1.563, 1.790] | 3.89e-8 | **GAP** |
| Full q8 | 10.208 | 7.057 | 1.447 [1.373, 1.483] | 3.91e-8 | **GAP** |
| Reduced q8 | 3.674 | 3.238 | 1.108 [1.037, 1.177] | 3.89e-8 | **GAP** |

All workers passed finite-output, repeat-consistency, and poisoned-output reset checks. These three-pair screens reject production promotion; full q1's interval crosses 1 but does not establish a pass.

## Existing hemisphere route and order-major screen

Widening the f64-only hemisphere selector to T720 activated the existing reflected dual-channel pair leaf. In its route test, full q8 reported 7,647 paired blocks and 3,889 checked fallbacks; reduced q8 reported 2,613 paired and 283 fallback blocks. The current ring-major scheduler was 3.37–6.59x slower than the same-source general f64 route for full/reduced q8 across three matched pairs.

A test-only order-major scheduler used the same hemisphere pair leaf, claimed each harmonic order once, and wrote disjoint Fourier columns. Route counters confirmed one hemisphere dispatch; full q8 executed 2,736 paired blocks and 1,590 fallbacks over 721 orders; reduced q8 executed 905 paired blocks and 181 fallbacks over 181 orders. Its route-equivalence test passed, with maximum relative error no greater than `8e-13` versus the same-source general f64 transform. The complete f32-I/O q8 screen and direct same-source comparison were:

| mmax | Order-major adapter ms | DUCC f32 ms | Order-major / DUCC paired median | Order-major / current f64 general paired ratios | Status |
|---:|---:|---:|---:|---:|---|
| 720 | 97.275 | 36.318 | 2.954 [0.524, 3.731] | 1.503–1.710 | **INCONCLUSIVE vs DUCC; route slower than current Rust** |
| 180 | 33.836 | 17.790 | 1.902 [1.612, 4.438] | 1.540–2.411 | **GAP; route slower than current Rust** |

The full-q8 interval is wide under external CPU pressure, but every direct paired same-source comparison favored the general f64 route. Reduced-q8 ratios remained above parity. No 15-pair extension is justified for a route with this consistent direction and large margin. The prior Stage B 15-pair current f64-internal baseline remains the source of its 16-cell statuses.

## Correctness and route tests

- `cargo test --release --features parallel --lib spin1_t720_hemisphere_route_uses_dual_channel_blocks_with_checked_fallback -- --nocapture` passed in the isolated route worktree for full and reduced output comparisons, route counters, and fallback behavior.
- The spin-to-scalar channel identity check passed with child scalar `lmax=721`; the same check with child `lmax=720` failed at relative error `1.37622e-2`.
- All benchmark workers passed finite, repeat, and reset checks. Random output relative-L2 against native DUCC was about `3.9e-8`.
- This screen did not run the full independent analytic/adjoint/extreme-value matrix for a production backend because no route passed the performance gate.

## Reproduction and retained evidence

The local, untracked `raw-screen-results.tar.gz` archive contains three-pair per-worker results and summaries for the two-scalar adapter, ring-major and order-major route screens, same-source comparisons, and benchmark scripts (SHA-256 `87c7af96e55231176e506913b7b96c732f8cf51755a23edab3fb20eb56e008bf`). Compressed source patches and the exact worker example are beside this report. The parent/route worktrees and build outputs are experiment-owned scratch and will be removed after the evidence commit is checked. No production source changed in this screen.
