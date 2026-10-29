# T720 m=0 prepared-basis accuracy diagnostic — 2026-10-29

**Decision: reject for implementation.** This isolated correction improves
one high-degree zonal accuracy case, but it adds work to every scalar
synthesis, does not improve the random-input comparison against DUCC, and
misses the native-f32 parity screen. It also prepares the complete m=0
degree-by-latitude basis with an f64 recurrence at plan construction. That is
outside the intended allowance for small plan-time f64 constants and must not
be promoted as the production solution.

## Exact source and binaries

- Base source: `b6a96aa7161a64f678b614d73478496f50f0551e`.
- Complete candidate source diff SHA-256: `71e4312d772bc4533688aead4ab0a7034c3ddfb681c5013953bebfaf145c2ce3`;
  recover from [`source.patch.gz`](source.patch.gz). The uncompressed patch has
  the same SHA-256.
- Candidate binary SHA-256: `fdfd096534fb58f58b65fd101648148f211a5f58c5c8186b3d22d125fe7c5c63`.
- Uncorrected uniform-scale comparison binary SHA-256:
  `d9158760d31937718f43e2ad09542a60ca2ca90e0018164238b2445345d3d932`.
- Pinned native-f32 DUCC 0.42.0 binary SHA-256:
  `be18c89ea434258e4a07c28ccd8fd31d6262ac3503dcaf81d8aa5176415f1484`;
  source commit `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

The f32 coefficients, runtime dot products, Fourier buffer, FFT and output
remain native f32. Plan construction computes and stores the full normalized
m=0 basis in f64, split into f32 high/residual values. The runtime SIMD leaf
uses a compensated f32 dot product across the degree axis. The table was
limited to GL T720 in the candidate.

## Three-pair screen

The screen used the same native-f32 fixtures, fresh parent/candidate/DUCC
processes in balanced interleaved order, 25 ms target batches, AMD Ryzen 9
5950X, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, q1 affinity `0`, and q8
affinity `0,2,4,6,8,10,12,14`. See [`results-3.json`](results-3.json) for
pair timings, fixture hashes, output checks, SIMD route and per-pair errors.

| GL T720 scalar synthesis | Uncorrected parent ms | Basis-dot candidate ms | DUCC ms | Candidate/parent | Candidate/DUCC |
|---|---:|---:|---:|---:|---:|
| Full q1 | 23.788 | 24.127 | 19.137 | 1.011 | 1.249 |
| Full q8 | 3.554 | 3.816 | 2.839 | 1.074 | 1.325 |
| Reduced q1 | 8.309 | 8.735 | 8.488 | 1.051 | 1.029 |
| Reduced q8 | 1.317 | 1.872 | 1.286 | 1.451 | 1.343 |

These are three-pair diagnostics, not promotion intervals. The candidate
median is slower than DUCC in all four cells and is substantially slower
than its uncorrected parent in reduced q8. Random full/reduced fixture error
against DUCC remains `1.864e-4` / `1.885e-4` relative L2, with maximum
absolute errors `5.498e-4` / `4.727e-4`; the zonal correction does not move
the whole-transform error at the reported precision.

## Accuracy, work and code generation

The asserted f32 GL T720 zonal test compares an `l=720,m=0` field across all
721 rings with an independent f64 recurrence at the same f32 latitude
coordinates. The uncorrected native-f32 recurrence reaches `2.759e-3`
maximum absolute error; the prepared-basis correction reduces that to
`1.483e-8`, with `7.414e-10` error at the first north ring. The saved log is
[`zonal-accuracy-test.log.gz`](zonal-accuracy-test.log.gz). This is a narrow
case and does not validate other orders, random transforms, adjoints or
exceptional values.

The q1 work probe records full/reduced ordinary pair updates of
`37,826,696` / `19,676,552`, of which `28,201,344` / `16,485,168` use the
two-vector fused loop (`74.6%` / `83.8%`). The maximum scaled-prefix length
is 341 pairs in both shapes. See
[`work-count-test.log.gz`](work-count-test.log.gz). The executed Rust route is
Pulp V3/AVX2 with eight f32 lanes; [`ordinary-f32-v3.asm`](ordinary-f32-v3.asm)
contains YMM loads and `vfmadd*ps` in the order-major suffix, and
[`zonal-dot-v3.asm`](zonal-dot-v3.asm) shows the V3 correction leaf. This
diagnostic identifies native vector execution but does not explain away the
whole-transform gap.

The exact candidate passed `cargo fmt`, `git diff --check`, the focused zonal
test, and `cargo test --features parallel --lib` (`318 passed, 37 ignored`).
The full library log is [`full-lib-test.log.gz`](full-lib-test.log.gz). No
candidate source was promoted to `dev` and no production dispatch changed.

## Next

Do not repeat the full m=0 basis table or the post-transform dot pass. Return
to the planned native-f32 compensated recurrence leaf with plan-time high/low
geometry and recurrence constants only. Integrate it into the shared
order-major execution so it replaces recurrence work instead of adding a
second transform pass; then re-run the zonal accuracy check, exceptional and
workspace checks, and the four-cell screen. Preserve native f32 arithmetic
for the runtime recurrence.
