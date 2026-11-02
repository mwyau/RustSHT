# B3 ordinary fused-leaf profile — 2026-11-02

This profile measures the accepted f64 seed-handoff source at `f4344353` and compares its executed ordinary recurrence leaf with pinned DUCC 0.42.0. It follows the [seed-handoff result](../m6-b3-seed-materialize-2026-11-02/README.md). No production code changed during this profile.

## Findings

The Rust candidate profiles exactly one warm transform inside a Callgrind client-request region. The matching DUCC process profiles eight transforms (first call, warmup, reset call, then five timed batches); its counters below are divided by eight. Both use GL T720, q1, `phi0=0.23`, the same deterministic full/reduced maps, release AVX2 builds and one pinned logical CPU.

| B3 f64 q1 | Rust total Ir | Rust ordinary leaf Ir (share) | DUCC total Ir / transform | DUCC `map2alm_kernel` Ir / transform | Rust / DUCC total Ir | Rust leaf / DUCC kernel Ir |
|---|---:|---:|---:|---:|---:|---:|
| Full | 577.66M | 262.90M (45.5%) | 432.91M | 223.15M | 1.334× | 1.178× |
| Reduced | 235.07M | 135.18M (57.5%) | 174.76M | 112.05M | 1.345× | 1.207× |

The 15-pair whole-call candidate/DUCC ratios are 1.113 [1.105, 1.120] full q1 and 1.098 [1.091, 1.104] reduced q1. Callgrind instruction counts do not predict cycles. Hardware PMU access is blocked on this host, so these measurements do not establish cycle attribution.

Data references per transform are lower in Rust (213.86M full, 91.95M reduced) than DUCC (435.92M, 192.06M). The Callgrind cache model records modestly more D1 misses in Rust (5.84M full, 2.18M reduced) than DUCC (4.78M, 1.32M), while miss rates remain low. This points away from raw memory-reference volume as the sole explanation, but the simulated cache counts are not hardware measurements.

The active Rust leaf is the V3 four-lane AVX2/FMA instantiation of `FusedScalarAnalysisPair<true,false,false,true,true>`. Its static body is 11,160 bytes with a 568-byte stack frame; the ordinary path uses `vfmadd231pd` and has no YMM stack loads/stores. The separate `AnalysisTwoBasisAccumulation` pass accumulates the second channel from the saved basis states. Repeated slice validation in the ordinary helper accounts for under 1% of whole-transform Ir and is not a worthwhile isolated target. The DUCC `map2alm_kernel` symbol is 722 bytes and uses YMM FMA with horizontal reductions. Both paths use four f64 lanes on this AVX2 machine.

The profile establishes the ordinary leaf as the dominant Rust instruction-count hotspot after the seed handoff. It does **not** isolate a cycle-level reason for the remaining time gap. No new kernel candidate is promoted from these counts alone.

## Next

Measure the ordinary leaf's recurrence and second-channel accumulation separately, then inspect an isolated per-degree shared-channel accumulation prototype for register spills and whole-call headroom. Keep the exceptional scaled fallback unchanged. Run a three-pair whole-call screen only if the helper profile predicts a material transform-level gain.

## Provenance and reproduction

- RustSHT source: `f43443531a2cfa247b1cae6a97e7cc7dcd75de45`; B3 source blob `b0077a7236f7e2315fe0c6b07da74f3eb4e845ba`.
- Rust profiling binary SHA-256: `dc2a5f16e5bff0c20289b9ac122aa9697e3632cad4a8f3881e14b50549fa4181`.
- DUCC source: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; clean DUCC binary SHA-256: `b646183e9df8aac030f796e2357b8d92e3b4a194d21be98b460eae043ab63443`.
- Host: AMD Ryzen 9 5950X; Rust 1.99.1 / LLVM 23.1.1; GCC 15.2.0; Valgrind/Callgrind 3.26.0.
- Rust flags: `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; DUCC flags: `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`.
- q1 used one pinned logical CPU; its exact index remains in local capture metadata. Rust client-region fixture/profile harness and C Callgrind shim are under `repro/`; V3/DUCC disassembly is compressed under `codegen/`.
- Full fixture hashes (map0/map1): `e3379278cb0dfc9a9f4d5f2c8ece40995f965bd727dcb4ce884b85fc62d7abba` / `dc3a132282b074d16cea41cf7305e64e796a48beac398221776ee8c7ad74d5cc`.
- Reduced fixture hashes (map0/map1): `33ed76ff3e7ebfbe866484fe2473634df9b5380cfe123936d263515be97b3eed` / `e92ffce48a2fa329a5bf113e0cdc6dd9b212bbea6c0f63568fb2e304e6bcf881`.

`results/summary.json` contains normalized counters. Full Callgrind captures remain in local, untracked experiment scratch. Rebuild with the Rust source at the listed SHA, add the included profile harness, link the included C shim, and invoke under `valgrind --tool=callgrind`. For DUCC, build the pinned source and run the same f64 spin-1 parity harness with target 1 ms; under Callgrind its warmup exceeds 100 ms, producing five timed batches plus the first, warmup and reset transforms.
