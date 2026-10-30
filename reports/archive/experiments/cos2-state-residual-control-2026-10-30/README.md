# Corrected cos² and recurrence residual replay

This isolated Phase C replay changes only the `cos²` input used by the existing high-degree recurrence probe. It rounds plan-time f64 `cos(theta) * cos(theta)` to f32, matching the corrected geometry preparation tested in the adjacent `cos2-plan-time-control` package. The generic candidate and production dispatch remain unchanged; the patch changes one line in an ignored test.

## Controlled result

The fixture has GL T720, orders `m=1..16`, nonzero coefficients at `l=600..720`, 13 north/equatorial/southern ring indices, and 208 order/ring outputs. It holds the f32 sectoral seed, recurrence `a,b`, and even/odd coefficient pairs fixed. Every path accumulates diagnostic sums in f64 so the comparison isolates recurrence arithmetic. The reference computes `a*cos²+b` and the state update with f64 fused operations from those exact f32 operands.

| Replay path | Relative RMS error to full f64 replay |
|---|---:|
| Native f32 leading contraction and state update | `1.648789356e-4` |
| Native f32 state-update error against a f64 state replay with the f32 leading factor held fixed | `1.009843351e-5` |
| Double-single state, compared to the matching f32-leading reference | `5.844473102e-13` |
| Double-single state, compared to full f64 replay | `1.636576384e-4` |
| Double-single leading contraction and state update | `1.154223230e-12` |
| First-order residual for leading contraction and state update | `1.138386001e-7` |

The `1.0098e-5` row measures only the native-f32 state-update error against an f64 replay with the same f32-rounded leading factor. State-only compensation nearly removes that error (`5.844e-13`), but its error against the full f64 replay stays at `1.6366e-4`, only about a 0.74% reduction from the native-f32 result. With state roundoff removed, this isolates f32 evaluation of `a*cos²+b` as the larger remaining source in this fixture. A first-order residual for both the leading contraction and recurrence state reduces the replay RMS error by about 1,449× and the maximum absolute error from `3.4380e-6` to `3.5014e-9`.

The ring-level relative RMS values also show the same pattern: at north ring 0, baseline `5.9472e-4` versus first-order residual `1.1781e-7`; at ring 719, `5.6064e-4` versus `4.6521e-7`; at the equator, `1.6892e-4` versus `1.3306e-8`. All 13 ring results are in `data/results.json`.

This identifies the next accuracy target: recover rounding from both `a*cos²+b` and the state update. It does not establish full-transform accuracy or an acceptable production cost. The test uses an eight-lane SIMD initializer, but the recurrence and accumulation replay itself is scalar; no AVX2 recurrence or whole-transform timing was measured. DUCC was not executed in this replay. The f64 comparator is a diagnostic reference over identical rounded f32 inputs, not a new runtime backend. The prior all-order residual transform screen was expensive and did not improve the old-cos² random-field error; this corrected-cos² combination still needs an isolated end-to-end accuracy and cost check before any promotion.

## Reproduction and provenance

- Replay base: `45380d7e4bd849a33ffef7d91978ee18fd7e4193` (isolated shared-native scalar-synthesis candidate).
- DUCC reference SHA: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; DUCC was not run for this control.
- Test-only patch: [`corrected-cos2-stateonly-replay.patch`](patches/corrected-cos2-stateonly-replay.patch), SHA-256 `9b70b1584568e5239bee1f6abe2f25053d7c147b50bb4420cb317c378ec3ec8d`.
- Rustc `1.99.1 (58cf8f9c3 2026-10-13)`, flags `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; AMD Ryzen 9 5950X. Setup selected eight f32 lanes; the replay computations were scalar.
- Test binary SHA-256: `a2adc699477b630b040b2743e949407a544a523d3ce1cf365bc4daf9200f885d`.
- Two ignored-probe runs produced identical numerical metrics. Raw console captures remain in the isolated worktree; the committed [`results.json`](data/results.json) contains normalized numerical output, repeatability and binary provenance. Checksums are in [`SHA256SUMS`](SHA256SUMS).

Run from the repository root after checking out the replay base and applying the patch:

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  cargo test --release --features parallel --lib \
  native_f32_compensated_recurrence_state_probe -- --ignored --nocapture
```
