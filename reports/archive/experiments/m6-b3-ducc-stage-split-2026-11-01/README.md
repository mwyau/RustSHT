# B3 f64 — pinned DUCC stage split

This diagnostic splits the pinned DUCC spin-1 analysis fast branch into its two scalar-channel calls, recurrence/accumulation, `spin0to1`, and surrounding work. It uses a separate linked worktree of DUCC 0.42.0 and a compile-time `DUCC_B3_PROFILE` guard. No RustSHT production source or benchmark binary was changed.

## Provenance and reproduction

- RustSHT branch source: `a254923e4a26247f3a39ba51a43ee20937f43b95`; B3 hot files match the profile parent `31c7cd5046d2389d5f0de230577dc147ff37f873`.
- Rust hot-file SHA-256: `transform/spin/analysis.rs` `a6015d8bdb1702e04dcedbca4cf4370b26f10ebaa1598d36914071d8a512a837`; `legendre/scalar_paired/analysis.rs` `1dfb13e16df59c87196a66f34adb95dcfb15ce1a7de4aff1aadc26dff5f4f59e`.
- DUCC source: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Compiler: GCC 15.2.0; Ryzen 9 5950X; q1 pinned to CPU 0, q8 to CPUs 0–7.
- DUCC SHT flags: `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; driver overrides with `-fno-fast-math`.
- Geometry: GL T720, `ntheta=721`, `phi0=0.23`; full `mmax=720,nphi=1458`, reduced `mmax=180,nphi=378`.
- Inputs match the existing deterministic PCG64 fixture hashes in [`rust-route-trace.json`](rust-route-trace.json). Three interleaved whole-call pairs compare the instrumented binary with the uninstrumented DUCC control. E/B output hashes matched in all pairs. These pairs bound profiler perturbation; they are not a promotion-grade performance benchmark.
- Instrumented binary SHA-256: `dbae1d68c9fa3b2a6a492d9f3ef53ba22c16cccf24eb15b2fe4fee06ca4975e1`; uninstrumented binary: `5dacb5987ee621a5115fed405775dc85231361ab703b3eae1694d086a5a7bdf8`.
- The reproducible, test-only DUCC patch is [`ducc-b3-stage-profile.patch.gz`](patches/ducc-b3-stage-profile.patch.gz); its SHA-256 is recorded in `SHA256SUMS`.

## Source crosswalk

| Stage | DUCC 0.42.0 | RustSHT at `31c7cd50` |
|---|---|---|
| B3 fast-ring route, two scalar calls, normalization, spin conversion, writeback | `src/ducc0/sht/sht.cc:851–910`; `spin0to1` at `:382–410` | `src/transform/spin/analysis.rs:596–960` |
| Ring grouping and per-order phase/weight fill | `src/ducc0/sht/sht_inner_loop.h:1661–1698` (`fill_m2a`) | reflected ring gathering/compaction in `src/transform/spin/analysis.rs:655–760` |
| Scaled prefix and recurrence/accumulation | `src/ducc0/sht/sht_inner_loop.h:426–456` (`iter_to_ieee`), `:592–684` (`map2alm_kernel`/`calc_map2alm`) | `src/legendre/scalar_paired/analysis.rs:6296–6520` shared two-channel dispatch; native fallback at `:5929–6090` |
| Scalar post-correction | `src/ducc0/sht/sht_inner_loop.h:1752–1808` (`inner_loop_m2a`) | E/B reconstruction after the fused scalar leaf in `src/transform/spin/analysis.rs` |

## Measured fast-branch stages

Times are medians across profile call 1 and call 2 in the three instrumented processes. q1 is the sole worker. q8 values are the worker on the critical path; its stage times are cumulative worker time, not additive call wall time. Branch time is wall time around `execDynamic`.

| Cell | Branch wall ms | Ring fill ms (both calls) | `calc_map2alm` ms (both calls) | Post-correction ms (both calls) | `spin0to1` ms |
|---|---:|---:|---:|---:|---:|
| Full q1 | 22.68 | 3.30 | 14.24 | 0.37 | 2.44 |
| Full q8 critical worker | 3.56 | 0.53 | 2.30 | 0.05 | 0.31 |
| Reduced q1 | 10.13 | 0.80 | 7.13 | 0.16 | 1.07 |
| Reduced q8 critical worker | 1.58 | 0.13 | 1.15 | 0.02 | 0.14 |

DUCC recurrence/accumulation is the largest timed stage: about 63–73% of branch wall by cell; the two scalar-channel calls together are about 80–82%. `spin0to1` is about 8–11%. Filling and preparation are material but smaller. The three-pair profiled/plain whole-call ratios (95% t intervals) are full q1 `1.002 [0.968,1.037]`, full q8 `1.007 [0.999,1.015]`, reduced q1 `1.010 [0.986,1.034]`, reduced q8 `1.036 [0.924,1.163]`; output hashes match. The reduced q8 timing is noisy, so profiler overhead is not resolved for that cell.

## Equivalent recurrence work

Each DUCC scalar-channel call records the same active geometry and recurrence work:

| Geometry | Active ring/order lanes per call | `iter_to_ieee` prefix updates | Scaled-loop updates | Ordinary updates | Four-lane vector slots | `calc_map2alm` calls |
|---|---:|---:|---:|---:|---:|---:|
| Full | 194,197 | 0 | 0 | 40,541,801 | 40,786,596 | 1,140 |
| Reduced | 64,364 | 0 | 0 | 20,386,534 | 20,522,448 | 362 |

These are actual branch counts collected inside `iter_to_ieee`, the exceptional scaled loop, and the ordinary suffix. The random fixtures take the ordinary recurrence path throughout. The ring cutoffs were recomputed from the Rust `GridSpec` geometry and spin-1 `polar_order_limit` formula and match DUCC's active lane totals. Rust's `NativeFusedAnalysisPairChannels` advances the same paired-degree recurrence once while accumulating both Q/U channels. It therefore performs 40.54M/20.39M recurrence advances; DUCC performs that count in each of its two channel calls (81.08M/40.77M total).

Rust's sampled full-q1 fused-compute region is estimated at 22.16 ms, while the two DUCC recurrence/accumulation calls take 14.17 ms. These timer boundaries are not identical and the Rust timer does not identify the selected SIMD leaf. The matched work counts rule out “Rust is slower because it performs more recurrence updates,” but do not by themselves identify the per-update cost.

## Actual Rust dispatch and AVX2 leaf

GDB entry breakpoints on the exact parent benchmark binary `3364b487f4d9fa1caa8a7de485c602fdc135620ef98376e80f5893b0207104ba` were run with the same four deterministic fixture files and q1/q8 affinity used by the paired benchmark. In all four cells, the executed leaves were `FusedScalarAnalysisPair<true,true,false,true,true>` (weighted, checked) and `FusedScalarAnalysisPair<true,false,false,true,true>` (weighted, unchecked). No `NativeFusedAnalysisPairChannels<f64, _>` entry breakpoint fired. The route trace and fixture hashes are in [`rust-route-trace.json`](rust-route-trace.json); each trace retained repeat and workspace-reset checks.

The active V3 checked weighted leaf is 12,126 bytes, has a 632-byte stack frame, and contains 252 static `vfmadd*`, 44 `vaddpd`, and no `vmulpd`. It stores and reloads a YMM temporary through `0x250(%rsp)` outside the inner AVX2 reduction body. The separate native leaf is 3,224 bytes with a 280-byte frame, 44 static `vfmadd*`, one `vmulpd`, eight `vaddpd`, and no vector stack references. Full disassembly is [`raw/rust-active-weighted-v3.asm.gz`](raw/rust-active-weighted-v3.asm.gz). Static size/instruction counts do not predict runtime; the following screens measure the two existing alternate routes.

## Rejected execution-route screens

Three interleaved parent/candidate/DUCC process pairs were run for each full/reduced q1/q8 cell. The parent is `a254923e` and has the same B3 hot files as `31c7cd50`; all use Rust 1.99.1 / LLVM 23.1.1, `-C target-cpu=x86-64-v3`, Ryzen 9 5950X, pinned CPUs 0 and 0–7, the same PCG64 fixtures, and pinned DUCC binary `5dacb5987ee621a5115fed405775dc85231361ab703b3eae1694d086a5a7bdf8`.

| Cell | Native compact leaf / parent (95% paired CI) | Two scalar-channel passes / parent (95% paired CI) |
|---|---:|---:|
| Full q1 | 2.058 [2.012, 2.105] | 1.228 [1.208, 1.249] |
| Full q8 | 1.949 [1.876, 2.024] | 1.218 [1.194, 1.241] |
| Reduced q1 | 2.110 [1.787, 2.490] | 1.207 [1.168, 1.246] |
| Reduced q8 | 1.936 [1.908, 1.963] | 1.202 [1.192, 1.212] |

The native-leaf candidate disables opposite-channel preweighting and selects the existing `NativeFusedAnalysisPairChannels<f64>` path. It is 1.94–2.11× the parent and 2.41–2.70× DUCC. Its additional per-degree cosine multiply outweighs the smaller leaf. The second candidate disables shared recurrence and runs the existing weighted single-channel kernel twice, matching DUCC's channel split. It is 1.20–1.23× parent and 1.49–1.55× DUCC. Both candidates returned finite outputs, passed repeat/reset checks, and changed parent outputs by only 2.1–2.3e-16 relative L2 (native leaf) or 0.61–0.71e-16 (separate passes). They are rejected; no candidate was promoted. Exact per-pair times, uncertainty, fixture/binary hashes, errors, and provenance are in [`rejected-native-leaf-3pair.json`](rejected-native-leaf-3pair.json) and [`rejected-separate-channel-3pair.json`](rejected-separate-channel-3pair.json). Reproduction patches are [`rejected-native-leaf-route.patch.gz`](patches/rejected-native-leaf-route.patch.gz) and [`rejected-separate-channel-route.patch.gz`](patches/rejected-separate-channel-route.patch.gz).

## Decision

No production change is justified. The original excess-work hypothesis is disproven, the proposed preweight was already present, and two distinct route changes both lost. The measured B3 f64 gap remains 1.24–1.29× DUCC. Stop B3 kernel experimentation here; resume only with a new profile that attributes time inside the active weighted leaf and identifies a different, measurable bottleneck.
