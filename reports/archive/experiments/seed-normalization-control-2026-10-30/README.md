# Native-f32 sectoral-seed normalization control

This follow-up tests only the seed-normalization change suggested by the `3bb97c1` numerical trace. It keeps the recurrence, coefficient preparation, cosine-squared input, accumulation and FFT arithmetic unchanged. The isolated candidate is diagnostic evidence; it is not a production patch.

## Result

The change avoids storing a nonzero sectoral seed as a subnormal when the ordinary f32 normalization loop would cross the normal range. It retains the preceding scaled value and its exponent so the existing scaled-prefix handoff can rescale it.

That improves the known order-8 near-pole case, but does not improve every polar case:

| GL T720 single mode | Parent f32 bits | Seed-control bits | Binary128 rounded f32 | Absolute error, parent → control |
|---|---:|---:|---:|---:|
| `m=8,l=720,r=1`, north real pre-FFT | `3ebb1f1f` | `3ebb1b60` | `3ebb1617` | `6.89083735e-5 → 4.03279463e-5` |
| `m=16,l=720,r=0`, north real pre-FFT | `2d1a4351` | `2d1a4370` | `2d1a3a40` | `2.01319686e-15 → 2.04008507e-15` |

The first error falls by **41.48%**. The second, very small exact-pole output error grows by **1.34%**. Across the matched 357 pre-FFT cases, 28 improve, 12 worsen and 317 are unchanged. Neither version correctly rounds to the binary128 f32 result in any of these 357 cases; the maximum absolute error is unchanged at `9.56558577e-3`, and RMS absolute error moves only from `2.31854140e-3` to `2.31851923e-3` (a 0.00096% reduction).

The public single-mode matrix contains 385 mode/ring cells and 13,090 pixels. Against the archived independently computed long-double values, 1,704 pixel results have the correctly rounded f32 bits in both versions. The seed control changes 1,340 pixels in 40 mode/ring cells: 936 get closer to the correctly rounded reference and 404 get farther. Maximum absolute error is unchanged (`5.67914588e-3`); RMS absolute error falls from `1.09244758e-3` to `1.09236035e-3` (0.00799%). The order-8 portion improves, while the changed order-16 exact-pole cells worsen. This is not a uniform polar correction and does not establish an accuracy pass.

The result is not a basis for a whole-transform timing run or production promotion. It shows that seed normalization is a real, avoidable error source in the order-8 example, while this broad conditional also shifts other tiny polar seeds in the wrong direction. No application accuracy tolerance has been supplied.

## Separate cosine-squared evidence

The earlier one-factor replay in the numerical-causality package already isolated cosine-squared from the seed and recurrence controls; it need not be repeated here. Substituting the DUCC/binary128 cosine-squared value rounded to f32 changed the `m=5,r=5,l=720` relative error from `1.23586439e-3` to `3.26359013e-4`, and the `m=16,r=5,l=720` error from `1.01521247e-3` to `2.38930773e-4`. These are selected sensitivity results, not a plan-time implementation or a full-matrix result. The independent control data is `../numerical-causality-ducc-0.42.0-2026-10-29/data/controlled-v2.json.gz` (compressed SHA-256 `a267c3ccb3a529d4717c72f37b7e56406897a79c28fec107a727f0831a011e26`). A separate plan-time cosine-squared candidate and its full 357-case / public-map results are in [the follow-up package](../cos2-plan-time-control-2026-10-30/README.md); it improves the selected recurrence and aggregate map error but remains unaccepted pending dense-band and full/reduced random checks.

## Provenance and reproduction

- Rust generic-candidate base: `45380d7e4bd849a33ffef7d91978ee18fd7e4193`.
- Exact isolated candidate plus trace diff: [`data/candidate.diff.patch.gz`](data/candidate.diff.patch.gz); decompressed patch SHA-256 `16a42debe5294e46d99b68511b48b962a8a2e2b74571bcedbe38cc72dee8aa3b`.
- Trace test binary SHA-256: `37911ea2f1cdffe06e023f07fc1b0d33e6aebfd2a3169ef7b8ab2417165862d6`.
- Rustc `1.99.1 (58cf8f9c3 2026-10-13)`, release test binary, features `default,parallel`, flags `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; direct probe asserted eight AVX2 f32 lanes. The public-map probe called `prepare_workspace(..., 1)`.
- Geometry: GL T720, 721 exact f64 theta bit patterns, `phi0=0`, `nphi=34`. The theta fixture SHA-256 is `1f16212ea7ef50ccaf9a52b552f9a822fb7052da1e8b164efdd858bdfd2a6f03`.
- Pinned DUCC reference: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The comparison script also checks that the archived parent trace's selected north component matches all 357 `rust_stage_bits` in the binary128 comparison table.
- Full local candidate capture hashes: direct trace `f75977cd166eb1dccd4d6f7c5f3132631593c95c5c08e7730a09c3378b79fab4`; public map `b5a9b06f0564bd03dcbd263ea5aca60aecf1680f77711c90b95ff2da15ed73e2`. Compact, necessary outputs are preserved under [`data/`](data/); millions of recurrence records are not duplicated.
- Archived reference hashes: binary128 single-mode table `41fddf9d882055b383fac52be56d80e0ecaa97bf4da073ee26eb5e6233277f32`; public-map independent reference `cb57628d64136afc8a5af9f83e41cd2ff9f57d350b1fa3871315a6b6aef77a16`; parent Rust stage output `dd3b5d1d6a2cd227c8f040ccb371f86bb26ddf1c759fa58e1e55ca5afade4ee3`; parent Rust public map `188bc9994489e76b4ba381e56a1400b3777baf570d72556d26e2d60cf1c25817`.

The isolated candidate patch changes source only in its disposable worktree; no production code was changed in the current `dev` checkout. The compact comparison is reproducible from the repository root:

```sh
python3 reports/archive/experiments/seed-normalization-control-2026-10-30/tracers/compare_seedonly.py
```

It regenerates [`comparison.json`](data/comparison.json), [`direct-comparison.csv`](data/direct-comparison.csv) and [`public-cell-comparison.csv`](data/public-cell-comparison.csv). The isolated Rust probes can be rebuilt by checking out the base above, decompressing `data/candidate.diff.patch.gz`, applying the resulting patch, and running the ignored `native_f32_ordered_t720_numeric_trace_probe` and `native_f32_public_map_t720_numeric_probe` tests with the recorded Rust flags. All candidate and trace source changes were confined to the disposable worktree. The `dev` production source was not changed by this experiment.

## Decision and NEXT

Do not promote this seed rule or claim scalar-synthesis accuracy. The separate plan-time cosine-squared control is now measured in the follow-up package; it shows a clear selected-fixture improvement but still needs dense high-band and full/reduced random validation. No performance claim was collected from instrumented code.
