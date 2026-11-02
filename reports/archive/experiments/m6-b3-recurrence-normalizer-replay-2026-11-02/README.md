# B3 recurrence versus normalization replay — 2026-11-02

This test-only diagnostic separates the active f64 B3 scaled-leaf recurrence
from its paired-lane exponent normalizer. It changes no production routing or
code. The captured-state replay is useful for ranking helper costs; it is not
a cycle profile or an estimate of whole-transform savings.

## Result

The active V3 path uses four f64 lanes. For captured q1 states, the ordinary
stable normalization case takes about 1.8–2.1 times the recurrence replay per
vector. Exponent-changing groups are much more expensive but uncommon:

| Geometry | Captured lanes with exponent change | Changed groups | Recurrence replay | Stable normalizer | Exponent-changing normalizer |
|---|---:|---:|---:|---:|---:|
| Full q1 | 136 / 20,914 (0.65%) | 123 / 5,126 (2.40%) | 1.457 ns/vector | 2.730–2.982 ns/vector | 17.307 ns/vector |
| Reduced q1 | 30 / 15,927 (0.19%) | 30 / 3,934 (0.76%) | 1.415 ns/vector | 2.602–2.610 ns/vector | 18.386 ns/vector |

The stable vector path performs horizontal reductions for finiteness, range,
and zero exponents. Its disassembly contains vector comparisons and reductions
(`vextracti128`, `vminpd`, `vshufpd`, `vminsd`, `vucomisd`); the recurrence
replay contains AVX2/FMA `vfmadd*pd` updates. This makes repeated mask
validation a concrete hypothesis to test. The disassembly is from the
diagnostic binary and is not evidence of active-leaf cycle attribution.

Exponent-changing groups take roughly 6–7 times the stable-normalizer helper
time, but their small observed frequency means their total contribution is
unknown. Do not multiply these helper timings into whole-call savings. The
capture-enabled test also perturbs sampled-step time, so only capture-disabled
logs are suitable as the corresponding q1/q8 controls. The q8 stage numbers
are worker-service measurements, not wall-time shares.

The current B3 whole-call screen at `cc60dbdc` remains GAP in all four cells:
full q1 1.252 [1.226, 1.280], full q8 1.293 [1.179, 1.420], reduced q1
1.282 [1.238, 1.326], reduced q8 1.277 [1.217, 1.339] Rust/DUCC f64. These
are three-pair screens, not promotion measurements. No kernel change follows
from this replay.

## Reproduction and provenance

- RustSHT diagnostic base: `be8f72d7fc55c420da5cf12b843b9353d2316eec`.
- `dev` source observed at run time: `1b4952201aeafc809078d57a7b6ab0514f1f58f6`.
- Active B3 blobs match that base: `analysis.rs` `baa5cee6c64ac43aa54828ab4c15055e8d23532e`, spin analysis `88a30c434410d8e44f4b690321380fa3671110ed`, `spin_fast.rs` `64f3e94db724b8fdfc0d71c9d96a6ae6d5f33ba3`, and `spin.rs` `0200fb1784b6d9081608006518dabfdab798e6ca`.
- DUCC reference: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust 1.99.1 / LLVM 23.1.1, release with `parallel`,
  `-C target-cpu=x86-64-v3`; AMD Ryzen 9 5950X; Pulp V3 AVX2/FMA.
- q1 pinned to CPU 0; q8 pinned to CPUs 0–7. GL `ntheta=721`, `lmax=720`,
  `phi0=0.23`; full `mmax=720,nphi=1458`, reduced `mmax=180,nphi=378`.
- Input map hashes are recorded in `data/recurrence-normalizer-summary.json`.
- Diagnostic source SHA-256: `analysis.rs`
  `a1d64fe6fb52b4f71d0f090b6c2b0d762b76c93a77db43649bfb89298ca1faaf`;
  nested test `analysis/tests.rs`
  `e22e21ccecc549ef7a64e5d939042c1b87d19410ad97d31f2404f1a9a4d9e017`.
- Diagnostic test-binary SHA-256:
  `4b32db9e6fe39546a66ab572d88f3ec53d1033aa90818c7ff8f9c3f55791dd34`.

Apply [`test-only-sampled-profile.patch.gz`](../m6-b3-sampled-scaled-leaf-2026-11-02/patches/test-only-sampled-profile.patch.gz)
at the diagnostic base, then apply
[`recurrence-normalizer-replay.patch.gz`](patches/recurrence-normalizer-replay.patch.gz).
The first patch SHA-256 is
`aa63b73d0d67ac37e3f5384a5875c601b48cd51036b6934449226b76ceaf5abf`; the
second is `99f62fb6b70d50d37dcbebe685936e8719caa170ade4affd67af2a8af7a14385`.
Both q1 capture modes and the capture-disabled q1/q8 controls passed the
ignored `m6_b3_sampled_scaled_leaf_timing` test. Parsed per-sample results and
the bounded helper disassembly are retained under `data/`. The raw console
captures remain local and untracked; the committed JSON preserves their
machine-readable measurements without embedding temporary build paths.

## Next diagnostic

Benchmark the normalizer's steady-state checks one at a time on the captured
stable vectors, then test a single combined reduction of finite, in-range and
zero-exponent lane predicates with the existing full checks retained as the
fallback. Verify that the finite/range/overflow decisions remain unchanged.
This distinguishes horizontal-reduction cost from SIMD register pressure
before considering any production candidate. Do not infer a whole-call win
from helper throughput alone.
