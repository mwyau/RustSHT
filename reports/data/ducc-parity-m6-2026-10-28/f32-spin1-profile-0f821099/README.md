# Native-f32 GL T720 spin-1 synthesis and analysis profile

**Status:** current-source profile only; no production change was made. The
profile identifies a larger algorithm and routing difference for spin-1
synthesis than another callback-level tweak. The full active status is in the
[M6 ledger](../../../DUCC_PARITY_M6.md).

## Source and measurement boundary

- RustSHT: exact profiled source `0f82109968236ff03f2f347fffb509ee834d763b`.
  The spin synthesis source is unchanged from the three-pair family baseline
  at `b3289762c7ba019523e4908b6172109d6975bfca`; later source changes in
  `src/legendre/mod.rs` add the scalar-analysis leaf and do not alter the
  spin-synthesis route.
- DUCC: pinned 0.42.0 commit
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- GL T720, `ntheta=721`, `nphi=1458`, `lmax=mmax=720`, `spin=1`, `phi0=0.23`.
  The same full fixture was used for both binaries. Its four input SHA-256
  values are recorded below.
- q1 was pinned to CPU 3. Each worker completed an unprofiled transform before
  Callgrind collected Ir for exactly one subsequent warm transform. Callgrind
  3.26.0 collected instruction references only; these values are not cycles
  or elapsed timings.
- rustc 1.99.1 / LLVM 23.1.1, GCC 15.2.0, AMD Ryzen 9 5950X. Both builds
  used x86-64-v3/AVX2; Pulp selected its eight-lane f32 V3 path. DUCC release
  flags were `-O3 -g -DNDEBUG -ffast-math -ffp-contract=fast
  -march=x86-64-v3`; the benchmark adapter used `-fno-fast-math`.
- Rust worker SHA-256:
  `50208673d4c2920c322a8251b84b49d8d564a2ff1a12f1f371b95136e5dfbcab`.
  DUCC worker SHA-256:
  `bbf5fab97167ef2b79f3037d4b60f9bf11fc491c6712e938ca6bbbadc0e89980`.

## Whole-transform q1 Callgrind

| Operation | Rust Ir | DUCC Ir | Ir ratio | Logical degree-latitude work | Rust Ir/work | DUCC Ir/work |
|---|---:|---:|---:|---:|---:|---:|
| Spin-1 synthesis | 22,433,360,982 | 520,826,788 | 43.07x | 161,323,854 | 139.06 | 3.23 |
| Spin-1 analysis | 15,752,365,732 | 430,971,670 | 36.55x | not counted | — | — |

The synthesis work denominator is the test-only Rust census count of useful
degree-by-output-latitude contributions. DUCC pairs reflected rings and
produces both outputs from a northern-ring recurrence, so this is a logical
output-work normalization, not a claim that both implementations execute the
same recurrence-update count. The q1 census at source `746c7e60` (same spin
source as this profile) measured 4,503,885,120 Rust recurrence lane updates
and 322,647,708 calls to `scaled_basis_value_f32`: two scaling calls for each
useful contribution.

In the Rust synthesis profile, the general spin callback is the hot path.
The callback has 7,135,806,288 self Ir (31.81%); its child
`scaled_basis_value_f32` has 8,364,308,822 inclusive Ir (37.29%) over
322,647,708 calls. These are parent/child inclusive attributions and must not
be added together. Spin-1 analysis spends 99.48% of Rust Ir in
`analyze_spin_order_group`, with 98.97% attributed to the shared spin
recurrence traversal.

The matched three-pair whole-transform baseline at the code-equivalent source
`b3289762` measured full spin-1 synthesis at 1595.214/42.627 ms q1 and
276.636/5.765 ms q8 (Rust/DUCC: 37.654x/47.938x). Reduced synthesis measured
657.734/18.363 ms q1 and 105.409/2.594 ms q8 (35.864x/40.444x). These are
elapsed timings from the earlier matched baseline, not timings inferred from
Callgrind and not a fresh 15-pair acceptance campaign at `0f821099`.

## Executed route difference

Rust f32 `ShtScalar::supports_paired_spin()` is false, so the T720 general
spin route reaches `synthesize_general_spin_order_to_order`. Its visitor
walks every ring, checks the polar order limit lane by lane, converts both
plus/minus recurrence states with `scaled_basis_value_f32`, then performs the
Q/U complex accumulation. The executed callback assembly has two indirect
calls to the scaling routine for each active lane and a scalar ring loop. The
recurrence producer has typed f32 SIMD, but that does not vectorize this
callback's accumulation across latitude lanes.

DUCC `synthesis<float>` takes a different high-Lmax route in `sht.cc`: for
standard spin 1 it splits ring data at `abs(sin(theta)) < 1e-4`; the minimum
GL T720 sine is `0.0033330851`, so every ring enters `rdata_fast`. It applies
`spin1to0`, pairs the 721 rings into 361 north/equator ring records, and calls
the scalar `inner_loop_a2m` for each of the two converted scalar channels.
Callgrind confirms that `alm2map_kernel` executes (122,212,914 Ir) and that
the spin-specific `calc_alm2map_spin` / `alm2map_spin_kernel` symbols do not
execute for this case.

The emitted DUCC kernel uses four AVX2 double lanes (`ymm` packed doubles),
FMA recurrence/accumulation, and a four-degree/two-pair loop. Representative
instructions from the executed DUCC scalar kernel are packed `vfmadd213pd` and
`vfmadd231pd` recurrence/accumulation operations on `%ymm` values. The Rust
callback contains two indirect `call` instructions to the scaling routine,
then scalar `vaddss`/`vsubss` Q/U arithmetic and a lane loop. The full raw
Callgrind profiles and disassemblies remain as local untracked captures in this
directory, as required by the experiment hygiene checks; the committed report
records their hashes, counts, route attribution and instruction patterns.

## Bounded next hypothesis

Test whether the existing shared paired-scalar coefficient conversion and
recurrence can serve the native-f32 spin-1 synthesis route over reflected
north/equator ring groups. This would target the measured differences together:
the current f32 route updates both spin states over all rings and scales each
contribution in the callback, while DUCC converts to scalar channels and
reuses reflected rings. A native-f32 dual-channel accumulation leaf may be
needed; it should share recurrence control flow and use existing typed f32
SIMD primitives, with the original general spin callback retained for polar,
scaled, nonfinite and tail cases. No candidate has yet been built, numerically
validated, timed, or accepted. Keep f32 scalar synthesis as a GAP and continue
to use the rejected suffix-transition evidence rather than repeating that
route.

## Fixture SHA-256

```text
alm0.f32  6a6e405f086a31606318c0d5dd3547ab900b659bbe79287e73a43bcf8b425413
alm1.f32  3add7c9249f142a4c2bfb0f83dce8f3b6f56af8a85955e5f21b44518db8d4072
map0.f32  2f5c74d4144bb78cd6b4b7507ab7fedf1d5605b848c9f88a4bbbf4d8099bc032
map1.f32  14bfa8b7db3131a688e18b8477dca389e189aa66966d0360b9a1ac786895fb9a
```

## Local raw-capture hashes

These captures stay local and untracked. Their hashes make the retained files
verifiable without committing raw profiler output or machine-specific
disassembly paths.

```text
ducc-spin1-analysis-q1.callgrind.gz   2ec624c007734a9cd9e3ce516a48ce3f17337e24e95ad1056027534a1a14194d
ducc-spin1-synthesis-q1.callgrind.gz  24e2386fab67c2cdfc3088759ef5ab412721d1bf62ab34d1df52a0ef129b7793
rust-spin1-analysis-q1.callgrind.gz   363aeac91e101307d52028b61822d084099a7677569669370e96d3360cc4e014
rust-spin1-synthesis-q1.callgrind.gz  49e1d0dcf48b4b39e8aa0c0e010490c243be78e0aefdba4ac0f73b1c2a30142b
ducc-executed-scalar-kernel.asm        ce4708fbde792fd760b72b1564344a823f8f82142fdaf21bb086e02b41630bd8
rust-synthesis-callback.asm            d1fed637e7767c860807e948dcb350810831c92ec5449249c50d1d7afe2c06d0
```
