# Focused scalar F1 performance comparison (2026-10-23)

This checks F1 performance after the ordered/reflected scalar synthesis and
parallel prepared-grid analysis changes. It is a targeted baseline, not a
general F1 parity claim.

Pinned DUCC 0.42.0 revision: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
Rust production source: `b495f35d180c3ec75c9312f8d052f4460feba9fe`.
Hardware: AMD Ryzen 9 5950X. Rust used `-C target-cpu=x86-64-v3`; DUCC used
`-march=x86-64-v3`. q1 used one physical core; q8 used eight. Each row has
30 timed batches across three independent process pairs; all short-operation
batches met the 20–100 ms calibration window.

The fixtures use exact F1 geometry (`ntheta=lmax+1`), full order, and even
smooth longitude counts. Warm milliseconds are median call latencies; brackets
show p10–p90. Ratio is Rust/DUCC, and paired ranges are process-pair ratios.

| T | Operation | q | Rust ms [p10, p90] | DUCC ms [p10, p90] | Ratio | Paired ratio range | Relative L2 |
|---:|---|---:|---:|---:|---:|---:|---:|
| 255 | Analysis | 1 | 2.871 [2.857, 2.885] | 3.854 [3.834, 3.871] | 0.746× | 0.745–0.746× | 5.73e-15 |
| 255 | Analysis | 8 | 0.557 [0.553, 0.572] | 0.977 [0.955, 1.033] | 0.568× | 0.567–0.579× | 5.73e-15 |
| 255 | Synthesis | 1 | 1.591 [1.532, 1.599] | 1.381 [1.375, 1.388] | 1.148× | 1.120–1.155× | 5.64e-15 |
| 255 | Synthesis | 8 | 0.235 [0.233, 0.243] | 0.228 [0.226, 0.232] | 1.037× | 1.024–1.038× | 5.64e-15 |
| 720 | Analysis | 1 | 39.431 [39.181, 39.986] | 57.906 [57.524, 58.220] | 0.680× | 0.676–0.690× | 5.98e-14 |
| 720 | Analysis | 8 | 6.890 [6.479, 7.791] | 10.418 [10.300, 10.631] | 0.651× | 0.646–0.719× | 5.98e-14 |
| 720 | Synthesis | 1 | 19.931 [19.795, 20.425] | 19.255 [19.091, 19.373] | 1.038× | 1.028–1.052× | 3.87e-14 |
| 720 | Synthesis | 8 | 2.808 [2.770, 2.873] | 2.727 [2.692, 2.772] | 1.022× | 1.018–1.044× | 3.87e-14 |

All eight rows passed numerical and output-reset checks. The run completed 24
numerical comparisons with no failure records; roundtrip, basis, adjoint,
finite extreme-value and subnormal checks were accepted. Maximum reported
relative L2 was `5.98e-14`.

F1 analysis is faster than DUCC in all selected cases. F1 synthesis is close
at T720 and q8 T255; T255 q1 remains 14.8% slower, an absolute excess of
`0.210 ms`. At T720 q1 the excess is `0.676 ms` (3.8%), and at q8 it is
`0.060 ms` (2.2%). This does not cover reduced order, odd longitude sizes,
or other bandlimits.

The sanitized row data is in
[`summary.csv`](data/ducc-scalar-f64-f1-performance-2026-10-23/summary.csv).
The raw result JSON and local host telemetry remain in the local results
directory and are not committed.
Raw result JSON SHA-256: `92c9a59a1136d29bbf12e670abdb79b3c10bc14ad9ada9662580b9271848d736`.
Rust binary SHA-256: `c2f89e79bdc1b74534edf21fdeb2ddef30332709c9d090332b53a0cd17587645`.
DUCC binary SHA-256: `d8feb9adeca29eed4ac09162fb44883178469d912598e99b5a85490fa8478f12`.

Reproduce with the pinned DUCC checkout at `$DUCC_SRC`:

```sh
python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --grid F1 --lmax 255 --lmax 720 --full-mmax-only \
  --longitude-mode even_smooth \
  --operation analysis --operation synthesis \
  --threads 1 --threads 8 \
  --results-dir "$RESULTS_DIR" --no-report \
  --sweep-id f1-scalar-t255-t720-current --retain-local-host-telemetry
```
