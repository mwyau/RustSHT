# B3 f64 scaled-seed handoff

**Status:** accepted improvement; the four GL T720 cells still miss DUCC parity.

## Diagnosis

Rust's `FusedCheckedScaledAnalysisPairChannels::with_simd` enters the checked scaled-prefix recurrence when sectoral seed state is represented as mantissa plus exponent. A diagnostic count on the exact parent recorded 599 such initializations per full transform; all 599 had seeds that could be restored to finite normal f64 values. Those calls performed 27,310 scalar pair updates and 337,763 vector-weighted scaled updates before reaching the ordinary fused recurrence. DUCC 0.42.0 made 2,280 `iter_to_ieee` calls but executed zero scaled-prefix recurrence updates on the same workload.

The accepted path restores the seed only when every nonzero lane remains finite and normal, zeroes the consumed exponents and previous sectoral state, then enters the existing two-channel `FusedScalarAnalysisPair` from pair zero. If any lane cannot be safely restored, the existing checked scaled algorithm continues unchanged. This reuses the existing recurrence and accumulation code; it does not add a second recurrence.

On the pre-change Rust full-q1 profile, the scaled helper accounted for about 87M of 642.4M Callgrind instructions (13.6%); the ordinary fused leaf accounted for 262.9M (40.9%) and the FFT for 71.5M (11.1%). These are instruction counts, not cycles. On the integrated candidate, a five-call q1 full phase probe measured 26.0 ms in combined Legendre recurrence/accumulation and 6.2 ms in FFT at 32.7 ms total. The ordinary Legendre work remains the next bottleneck. PMU events are unavailable (`perf_event_paranoid=4`).

The exact Rust source is `src/legendre/scalar_paired/analysis.rs:3645-3749`; the new guard begins at line 3698. The executed path is the V3 AVX2/FMA instantiation with four f64 lanes. The guard is scalar per seed lane; the ordinary kernel retains the existing vector `vfmadd231pd` recurrence and accumulation. Compressed disassembly for the V3 checked wrapper and ordinary leaf is in `codegen/`.

Pinned DUCC's spin-1 analysis prepares scalar channels in `sht.cc` around lines 880-900, calls `inner_loop_m2a` (`sht_inner_loop.h:1752`), and runs the scalar `calc_map2alm`/`map2alm_kernel` path (`sht_inner_loop.h:592-683`) for both channels before `spin0to1`. The paired Rust path shares recurrence advancement for its two channels. This experiment addresses only Rust's unnecessary scaled prefix; it does not assert a full algorithm match.

## Matched GL T720 results

Fifteen interleaved whole-transform parent/candidate/DUCC pairs per cell. Ratios are paired geometric means with 95% t intervals on log ratios. q1 was pinned to CPU 0; q8 to CPUs 0-7. Full order is M720, reduced order M180; both use 721 latitude rings and `phi0=0.23`. Rust and DUCC each used the same ten warm batch measurements per worker process.

| Cell | Parent ms | Candidate ms | DUCC ms | Candidate / parent | Candidate / DUCC | Parent / DUCC |
|---|---:|---:|---:|---:|---:|---:|
| Full q1 | 36.729 | 32.847 | 29.508 | 0.893 [0.886, 0.901] | 1.113 [1.105, 1.120] GAP | 1.246 [1.237, 1.254] |
| Full q8 | 5.245 | 4.735 | 4.203 | 0.903 [0.896, 0.909] | 1.124 [1.115, 1.132] GAP | 1.245 [1.234, 1.256] |
| Reduced q1 | 14.889 | 12.765 | 11.639 | 0.857 [0.853, 0.862] | 1.098 [1.091, 1.104] GAP | 1.280 [1.275, 1.285] |
| Reduced q8 | 2.149 | 1.856 | 1.709 | 0.865 [0.862, 0.868] | 1.088 [1.081, 1.096] GAP | 1.259 [1.249, 1.269] |

Candidate-to-parent E/B relative L2 was at most `6.69e-17`. Candidate-to-DUCC E/B relative L2 was at most `5.24e-14`. Repeat/reset checks passed for every worker. These checks support this f64 route change; they are not a broader accuracy claim.

## Validation and controls

- Focused checks passed for standard spin-1 analysis versus general recurrence and adjoint on GL/CC/F1, GL paired analysis versus full latitude recurrence, poisoned workspace reuse, extreme finite fallback, and scaled sectoral two-channel lanes.
- `cargo test --workspace --all-features --release`: 328 passed, 41 ignored; 19 integration tests passed and one remained ignored.
- Python CI tests: 20 passed.
- Strict library Clippy passed. All-target strict Clippy is blocked by `clippy::chunks_exact_to_as_chunks` in the pre-existing untracked `examples/m6_b2_synthesis_callgrind.rs`. Workspace format check is blocked by pre-existing unformatted untracked M6 benchmark examples; this changed kernel file passes `rustfmt --check`.
- A separate GL T1023 f64 spin-1 synthesis three-pair control was bitwise equal parent/candidate. Candidate/parent median ratios were 1.0049 q1 and 0.9953 q8; both are INCONCLUSIVE at three pairs and establish no timing effect.
- GitHub Actions status could not be queried locally because `gh` is unauthenticated.

## Provenance

- Rust parent: `ceae53e255fbc0595528b49bee1ec4125d48924a`; B3 analysis source blob: `baa5cee6` (same B3 analysis source as `31c7cd50`).
- Production candidate source blob: `b0077a7236f7e2315fe0c6b07da74f3eb4e845ba`; production source diff SHA-256: `5435cd65ce8a7744fa1789d1cb99eb2ae79873ef90691f41b6659f56a6e9b36a`.
- DUCC source: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host: AMD Ryzen 9 5950X; Rust `1.99.1` / LLVM `23.1.1`; GCC `15.2.0`; Rust `-C target-cpu=x86-64-v3`; DUCC `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`.
- Diagnostic parent/candidate/DUCC binary SHA-256: `1596e673277c202bd83e349f7c66934e2e8876420bcad3d0e7eac0712defe423`, `11fa182ef9101eec7929f3ac8bd2cb81859667e6067d35a04ede81e10716bd46`, `b646183e9df8aac030f796e2357b8d92e3b4a194d21be98b460eae043ab63443`.
- The integrated root production example binary SHA-256 is `93ea351b63ba42092a3797ac0e809dc488bfc1d7395d22aa983e388124bd59cf`; its full-q1 output matched the measured candidate bit-for-bit.
- Full fixture SHA-256, ordered map0/map1/E/B: `e3379278cb0dfc9a9f4d5f2c8ece40995f965bd727dcb4ce884b85fc62d7abba`, `dc3a132282b074d16cea41cf7305e64e796a48beac398221776ee8c7ad74d5cc`, `f4e07903e76707c0c5b7904dd8b32542899c961dd9043f4cb0b5592202bb96b8`, `148b93ae3c31ee14d53d2d45846899d3d53d039a128ce3f596fa93faff9174fd`.
- Reduced fixture SHA-256, ordered map0/map1/E/B: `33ed76ff3e7ebfbe866484fe2473634df9b5380cfe123936d263515be97b3eed`, `e92ffce48a2fa329a5bf113e0cdc6dd9b212bbea6c0f63568fb2e304e6bcf881`, `86f69cdcee1047e6b8d83f39aad923db97ed05d6fcc20fb3f4c4c6df5d256d3b`, `9486b6bf16def3639839b08609f28c9e9531c7cff4d03d49a482665c585b62aa`.

The recoverable production patch, four machine-readable 15-pair result files, T1023 control, and compressed disassembly are included beside this report.
