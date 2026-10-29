# Reduced GL T720 spin-1 synthesis q1 public-call profile

- Source parent: `b3b93f17fe4d404fe4c033ffc7420eb190dc00b0`.
- Reference: DUCC 0.42.0, pinned commit `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Cell: Gauss-Legendre, `ntheta=721`, `nphi=378`, `lmax=720`, reduced `mmax=180`, spin 1 synthesis, q1 pinned to one physical core.
- Rust: 1.99.1 / LLVM 23.1.1, `--locked --release --features parallel`, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, with `CARGO_PROFILE_RELEASE_DEBUG=1` for Callgrind symbols. The timed public-call adapter promotes both Complex32 coefficient arrays to Complex64, calls the reusable f64 SHT plan/workspace synthesis, narrows both output maps to f32, and includes the complete call.
- DUCC: exact pinned 0.42.0 source, GCC 15.2, `-O3 -g -DNDEBUG`, `-ffast-math -ffp-contract=fast -march=x86-64-v3` on project C++ target, driver with `-fno-fast-math`; native f32 `ducc0::synthesis_2d` public API.
- Warm-call boundary: each worker constructs its reusable plan/workspace before timing, executes first/warm calls, verifies repeat and poisoned-output reset, then Callgrind collects one `m6_profile_call` invocation. `callgrind --collect-atstart=no --toggle-collect='*m6_profile_call*'`; one complete transform per profile. Callgrind events are instruction references (Ir), not cycles.
- Fixture SHA256: alm0 `738b1dca3d9a775296c355e9675f264b90e1b61c1ee9ba9c92203c70f7ba8a36`; alm1 `aab90e397527bda9331a830748c56a9b9fac2b7607c9533773b2af762446544c`; map0 `68f3d4da093d266b1b5fec6c517d01631f0f8e2485aa0493261a3a04d0d60b80`; map1 `3ded2f8baf61b79a1664e0d6640e3285d1d0d6256b2277248ae1175d9e0f0179`.
- Instrumented Rust binary SHA256: `a011fffd979acdd146f153072589a16734efbfadf823ed15a06ed6b177b7ed25`; DUCC binary SHA256: `b3449be4f571c805dfb5d07b80b8bb16bd677df20a28b0898f4528494ccd6627`.

## Findings

The single warmed Rust call collected `609,836,478 Ir`; DUCC collected `212,818,229 Ir`, a 2.867x reference ratio. Rust's exclusive `FusedPreparedSynthesisPairBlock` SIMD body accounts for `159,961,530 Ir` (26.23% of the whole call); the inclusive spin ring-group synthesis path is 594,254,065 Ir (97.44%), and inverse FFT rows are 14,476,440 Ir (2.37%). Do not sum inclusive rows and callees. DUCC's exclusive `alm2map_kernel` is 61,518,480 Ir (28.91% of its call).

A separate 60-call q1 stage profile measured promotion 0.076144 ms, f64 transform 27.956647 ms, narrowing/writeback 0.085071 ms, whole call 28.116359 ms; reusable conversion buffer capacity was 12,375,968 bytes. Conversion is below 1% of this cell, so the large gap is transform work, not f32/f64 copy overhead.

Source and executed AVX2 disassembly show a concrete dataflow difference. Rust iterates vector latitude blocks outside coefficient pairs, broadcasting the same per-degree coefficient values again for each vector block. DUCC hoists coefficient broadcasts outside multiple SIMD latitude blocks. This motivates one bounded two-latitude-vector tile in the existing shared `FusedPreparedSynthesisPairBlock`, with the original recurrence and checked fallback retained. Tile 4 is not proposed because the AVX2 register budget risks spills. Candidate acceptance still requires whole-call measurements, emitted-assembly inspection, numerical validation and protected controls.

## Reproduction artifacts

Sanitized compressed Callgrind profiles and full annotations, selected Rust/DUCC executed hot-function disassembly, and the scratch-only f32-I/O/f64-internal adapter / stage-profile / DUCC worker are preserved here. The binaries and build trees were intentionally not retained. `SHA256SUMS` covers each saved artifact. The uncompressed profile totals were `609,836,478 Ir` (Rust) and `212,818,229 Ir` (DUCC).
