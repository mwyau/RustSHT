# Compensated f32 recurrence replay and AVX2 cost screen

**Status: test-only diagnostic. No production dispatch or source algorithm was
changed.** The probe targets the selected high-degree error for GL T720,
orders `m=1–16`, degrees `l=600–720`, and polar, equatorial and near-south
rings.

## Accuracy replay

The ignored accuracy probe processes 208 order/ring outputs at rings 0–5,
360, and 715–720. It fixes the candidate's f32 sectoral seed, geometry,
recurrence tables, and prepared even/odd coefficient pairs. The reference
widens those same f32 inputs and pairs, then evaluates both the leading
`A*cos²+B` contraction and recurrence state in f64 with fused operations.
Accumulation is widened to f64 in all three paths so this isolates recurrence
arithmetic from summation rounding.

| Replay | Relative RMS error vs full widened recurrence |
|---|---:|
| Ordinary f32 leading contraction and state | `1.64484e-4` |
| Double-single state with the f32-rounded leading contraction | `1.63434e-4` |
| Double-single leading contraction and state | `1.20019e-12` |

The state-only path is within `5.91e-13` relative RMS of a reference that
uses the same f32-rounded leading contraction. It therefore corrects state
update rounding but leaves the leading-contraction error intact. The full
twofold path brings maximum absolute replay error to `2.49e-14`. Both the
leading contraction and state update need compensation to recover the widened
recurrence on this fixture.

This is a focused recurrence diagnostic, not an independent end-to-end
transform check. The sanitized numerical record is in
[`results/accuracy-results.md`](results/accuracy-results.md).

## AVX2 cost screen

The ignored vector probe uses the existing test candidate's double-single
SIMD add/multiply helpers for the full leading contraction, recurrence state,
and real even/odd accumulation. It runs f32 width eight over 16 prepared
orders with 353–360 degree pairs per order. All eight lanes repeat the
equatorial ring input. Fifteen interleaved single-thread pairs, with 16 sweeps
per timing, measured a baseline median of `0.098 ms` and compensated median
of `1.677 ms`; the paired median ratio was `17.240x` (observed pair ratios
`14.95–18.00x`). The run was not CPU-pinned and is only a kernel cost screen:
it does not represent full-transform time or q8 scaling. The sanitized
measurements are in [`results/vector-cost-results.md`](results/vector-cost-results.md).

The generated AVX2 vector callback has a 96-byte stack frame. Its compensated
one-pair loop is 116 instructions, including 40 packed adds, 48 packed
subtracts, 9 packed multiplies, and 4 packed FMA residual operations; the
ordinary loop is 19 instructions, with 5 packed FMAs in this test callback.
The full double-single helpers perform repeated product-residual recovery and
renormalization for both the leading contraction and the state update, then
maintain double-single sums. This explains the large microkernel cost. The
callback is diagnostic code and not a tuned production kernel.

There is no whole-transform timing for an all-order compensated route. Do not
change production dispatch from this result. A cheaper residual-correction
form must first preserve recurrence accuracy and materially reduce vector
instruction cost; only then should it enter a test-only end-to-end transform
screen.

## Provenance and recovery

- Generic candidate base: `b6a96aa7161a64f678b614d73478496f50f0551e`.
- Generic candidate source diff: `cc72cbc20048affbdbd6cdbce619af79edc65896a9a0ed9730006179ea3ce7a8`.
- Test-only probe patch SHA-256: `7f2ef12fc2e3197dd4bb4862f75d2aa90a31e8f03de9771be0aa22fe0b661ba2`.
- Candidate test file before probe: `eac89e21fe072cdde702acd203185f5ef7ab71e7cc1368c6083bc535dbf5faf2`.
- Candidate test file after probe: `f6ba71ef69c94cf72d7f46f10e3540827b12f43afe5dc888ac4ec6d7c1c943ca`.
- Test binary SHA-256: `15bbe0974a8109ed6d1fc0a2eaf3d072e42ef782e19dcf27c5bdb4e69168af26`.
- Compiler: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, release, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.
- Host: AMD Ryzen 9 5950X; AVX2 selected, eight f32 lanes.

The incremental test patch is in [`patches/`](patches/). The exact AVX2
function disassembly is in [`codegen/`](codegen/); sanitized commands, hashes,
measurements and check summaries are in [`results/`](results/). File checksums
are listed in [`SHA256SUMS`](SHA256SUMS).
