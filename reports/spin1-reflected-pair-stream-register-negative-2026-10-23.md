# Reflected-pair coefficient-stream mapping: rejected

## Candidate and correctness

This was a focused correction to the full-width, no-degree-tile reflected-pair kernel. It kept the two converted channels and both hemispheres on one recurrence, but expressed each coefficient broadcast directly at its FMA to shorten coefficient-vector live ranges. It preserved checked block fallback and the original-spin polar fallback.

The direct kernel comparison against the existing paired-block reference passed at scalar degree 721 with eight reflected lanes. The order-major reuse test passed for GL, CC, and F1 T720 reduced order at q8, including regular, sparse, extreme finite, unsafe converted, subnormal, and changed-coefficient cases. The test confirms that the new route executes.

## Bounded CC T720 q8 screen

The screen used full-order CC T720 (`ntheta=722`, `nphi=1458`), q8 on physical CPUs 0–7, and ten timed single-call batches per backend. Parent source was `61ae7717`; DUCC was pinned at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

| Backend | Median | p10–p90 | Range |
| --- | ---: | ---: | ---: |
| Candidate | 20.225 ms | 19.971–23.830 ms | 19.945–25.206 ms |
| Parent | 15.174 ms | 15.057–15.382 ms | 15.045–15.538 ms |
| DUCC | 7.070 ms | 6.217–8.215 ms | 6.153–8.460 ms |

The candidate was 33% slower than parent; its p10 was above the parent's p90, so it was rejected without reduced-order, F1, q1, or T1023 follow-up. Candidate outputs were finite and matched parent at relative L2 `1.27e-13` (maximum absolute error `1.57e-9`) and DUCC at relative L2 `2.83e-13` (maximum absolute error `3.16e-9`). Candidate wait fraction was 3.4%, parent 0.4%, and DUCC 8.1%.

Raw batches, fixture hashes, binary hashes, and host diagnostics are in [the quick-screen record](data/spin1-reflected-pair-stream-register-2026-10-23/cc-t720-full-q8/quick-screen.json).

## Bounded AVX2 diagnosis

The V3 `FusedReflectedPairStreamBlock` symbol begins at `0x1a71a0`; its degree-pair loop is near `0x1a7460–0x1a7560`.

- Although the Rust expressions broadcast each scalar coefficient at its FMA, the compiler still emits all eight coefficient `vbroadcastsd` instructions together at `0x1a751b–0x1a7551` before updating the accumulators.
- Two accumulator vectors remain memory-resident through stack slots `rsp+0x280` and `rsp+0x1d0` in the loop (`vfmadd213pd` and stores around `0x1a7464–0x1a74d7`). Source-level expression ordering did not remove the register-pressure spill mechanism.
- The weighted odd-parity terms also emit four vector multiplies by cosine per degree pair before their FMAs.

This is a failed register-scheduling correction: the compiler preserves the eight-coefficient live range, and the hot loop retains spills plus extra cosine multiplies. The mapping and dispatch were removed. No production change was retained or pushed; the next candidate needs a different register mapping rather than another spelling of this loop.
