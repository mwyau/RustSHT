# Native-f32 plan-time cosine-squared control

This isolated candidate changes only the cosine-squared operand supplied to the shared generic scalar-synthesis recurrence. It computes `cos(theta)^2` from the original f64 plan angle and narrows that small plan-time constant to f32. Runtime coefficients, recurrence, SIMD, accumulation, Fourier spectrum, FFT and map remain native f32. The f64 generic branch retains its previous cosine-squared expression. No production source was changed on `dev`; no timing was collected from this diagnostic candidate.

The candidate is based on the isolated shared-native scalar-synthesis source at `45380d7e4bd849a33ffef7d91978ee18fd7e4193`. Its exact trace-and-candidate patch is `data/candidate.diff.patch.gz` (decompressed SHA-256 `ebedf095a6dceac343ee98e0465b3702e23a7d86b37bdcc1f0ebf01fc58f0ee5`); file hashes are in `SHA256SUMS`. Decompress and apply it only in a disposable worktree. Applying it is for a disposable worktree only.

## Numerical result

The direct comparison covers 357 single-mode pre-FFT outputs at GL T720, `phi0=0`, `nphi=34`, with the archived binary128 recurrence/output reference. It confirms the baseline parent's selected output bits before comparing the candidate.

| Measure | Existing f32 geometry arithmetic | Plan-time f64 `cos²`, rounded to f32 |
|---|---:|---:|
| Direct cases closer to binary128 | — | 102 / 357 |
| Direct cases farther / unchanged | — | 0 / 255 |
| Correctly rounded direct outputs | 0 / 357 | 0 / 357 |
| Direct RMS absolute error | 2.31854140e-3 | 2.13214617e-3 |
| Direct RMS relative error | 6.62542523e-3 | 5.02144412e-3 |
| Direct maximum absolute / relative error | 9.56558577e-3 / 5.79835575e-2 | unchanged |

For the public single-mode probe, 13,090 map pixels (385 mode/ring rows × 34 longitudes) are compared with the independent long-double map reference. The candidate changes 2,008 pixels: 1,992 move closer and 16 move farther; 11,082 are unchanged. Correctly rounded pixel count stays at 1,704. Maximum absolute error falls from `5.67914588e-3` to `2.66875106e-3`, RMS absolute error from `1.09244758e-3` to `4.48435866e-4`, and aggregate relative L2 error from `3.60792315e-4` to `1.48100666e-4` (58.96% lower). Public `m=0,1,2` outputs are unchanged; the selected changes are in the ordinary higher-order recurrence.

Per-order public relative L2 is:

| Order | Existing | Candidate |
|---:|---:|---:|
| 5 | 5.95748e-4 | 3.41795e-4 |
| 8 | 8.60502e-4 | 2.68827e-4 |
| 16 | 9.29782e-4 | 2.22795e-4 |

This is a substantial selected-fixture improvement, but it is not a correctness pass: some public pixels worsen, no direct output becomes correctly rounded, maximum direct error is unchanged, and no application accuracy tolerance has been set.

## First recurrence divergence and propagation

The plan record comparison finds 24 shared `(m, ring)` rows. The only changed prepared field is `cos²`, in 12 rows (six orders at ring 5 and six at the equator); theta, sine, cosine, sectoral seed, pair coefficients, and other prepared values are bitwise unchanged.

At `m=5`, north ring 5, `cos²` changes from `0x3f7fd6e4` to `0x3f7fd6e5`. The binary128 value is `0.999372803120232096823972339746616469`; absolute operand error falls from `8.24018483e-8` to `2.27972035e-8`. The recurrence starts with identical previous/current state and identical `a,b`. At degree 7, the first changed kernel arithmetic is the leading contraction `(a*cos²+b)`: `0x41264500 → 0x41264501`; the following state update changes from `0xb34c14e6` to `0xb34c14e7`. Both are one-ULP changes, with no recurrence formula or FMA change.

The saved 357-degree `m=5`, ring-5 recurrence table compares each f32 `next` state with the archived binary128 state. By degree 719, the state bits are `0x3f78a7d5 → 0x3f78e1cd`; correctly rounded binary128 is `0x3f78f69a`. Absolute error falls from `1.20191812e-3` to `3.17385191e-4`, and distance falls from 20,165 to 5,325 f32 ULPs. Across the full 357 states, RMS absolute state error falls from `5.19825290e-4` to `1.48874706e-4`, and maximum absolute state error from `1.24105822e-3` to `3.21485990e-4`. Relative RMS state error falls from `1.45808905e-2` to `3.73421940e-3`. ULP distances remain large near cancellation; the absolute and relative error columns are retained in `data/recurrence-csq-comparison.csv.gz`.

At the equatorial ring the selected `cos²` bits change from `0x099bbe75` to `0x099bbe74`; against binary128, the operand absolute error falls from `3.03740326e-40` to `6.36016584e-41`. These controls show that the revised plan constant, not a changed recurrence or accumulation order, causes the measured improvement. The changed leading factor then perturbs every subsequent f32 recurrence state.

## Reproduction and provenance

- Candidate Rust base: `45380d7e4bd849a33ffef7d91978ee18fd7e4193`.
- Pinned DUCC reference source: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; archived binary128 and long-double outputs provide the numerical references.
- Rustc: `1.99.1 (58cf8f9c3 2026-10-13)`; release test build; flags `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; AVX2 trace asserts 8 f32 lanes. The public probe uses workspace capacity 1.
- The exact diagnostic patch is `data/candidate.diff.patch.gz`. It includes test instrumentation and candidate-only changes; none of these source edits were applied to `dev`.
- The full local direct trace SHA-256 is `bd834e729ca198265fa1a6032da995259d30eb4a9fad3a29ef57bed9126cc6fc`; full public-map trace SHA-256 is `d5e7679872608589ed68869657fdb836613f6a5233dc846583be76b9f897a915`. Full traces remain in bounded local scratch; only selected plan, recurrence, output and map records are archived.
- The test binary SHA-256 is `c806026b2bd11e884de229f5874453cd1e4f97beeb8b472cebd83b0dc70ec13f`.
- `cargo fmt`, `git diff --check`, and `cargo test --release --lib` pass in the isolated candidate (320 passed, 44 ignored). Both ignored direct and public-map trace probes pass. No performance timings were collected from the instrumented binary.

From the repository root, rerun the two comparisons with:

```sh
python3 reports/archive/experiments/cos2-plan-time-control-2026-10-30/tracers/compare_recurrence_csq.py
python3 reports/archive/experiments/cos2-plan-time-control-2026-10-30/tracers/compare_csqonly.py
```

## Decision and NEXT

The plan-time constant change is a verified causal improvement on the selected single-mode recurrence and map samples. Keep it isolated: the 16 worsened public pixels, unchanged correctly-rounded count, unchanged maximum direct error, and unset science tolerance mean it cannot be treated as a production accuracy pass. Next, run the saved 256 dense high-band pre-FFT fixture and matched full/reduced random T720 fixtures against independent references using this same isolated candidate. Then decide whether the improvement generalizes before considering any production source promotion or whole-transform timing.
