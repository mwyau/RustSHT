# T720 direct ring-spectrum analysis input A/B

Candidate source: RustSHT commit `e51da6494823debe8868e43bc2b5f722ce456e2a`. Baseline source: `f2249ac7c8272579fdc783a776f5fed24a3da99c`. DUCC: pinned `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0). The Rust baseline and candidate used Rust 1.99.1/LLVM 23.1.1, strict FP, `-C target-cpu=x86-64-v3`, runtime Pulp V3 (four f64 lanes). DUCC GCC 15.2 SHT objects used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the validation driver remained strict FP. CPU: AMD Ryzen 9 5950X. q1 was pinned to CPU 0; q8 to physical CPUs 0–7.

`warm-screen.csv` preserves two interleaved source A/B screens at T720 GL (`721x1458`, full `mmax=720` and reduced `mmax=180`), q1/q8, 30 calibrated batches per backend/case/thread across three independent process pairs. It includes medians, p10/p90, paired ratios, and batch durations. The second screen is the reproducibility check; both runs remain visible. q1 processes observed 11–16% scheduler wait. In the repeat q8 run, candidate scheduler wait was lower than baseline in each pair (0.8–1.7% vs 1.6–5.1%), so the q8 reduction is directional and should be remeasured on a quieter host. Do not infer a precise speedup from the q8 percentage alone.

`cold-call.csv` reports setup, first-call and warmed latencies separately for the repeat screen. `validation.csv` records candidate/baseline identity and error versus DUCC per process. The candidate matched baseline output bitwise; relative L2 error to DUCC was `4.63e-14` for full `mmax` and `4.96e-14` for `mmax=180`.

`stage-profile.csv` gives three independent q8 Rust test-phase profiles at the same geometry before the change and three instrumented DUCC stage runs. Rust's test probe used a deterministic finite map; the DUCC stage runner used the parity fixture, so phase values rank work and are not exact input-by-input cycle comparisons. Rust phases had 30 calibrated 50 ms batches each. The temporary DUCC stage harness collected 10 timed batches per process (140/150/140 stage calls); whole-call A/B comparisons above use 30 batches per process. DUCC `map2leg` and `leg2alm_internal` are not subtractable from Rust phase boundaries.

The order-major Rust workspace area at T720 full `mmax` is `721 x 721` complex f64 values (`8,317,456` bytes; about 7.9 MiB). The direct route avoids writing that entire weighted matrix and reading it back during pair packing. The workspace allocation itself remains unchanged. DUCC's analogous packing is in `fill_m2a` inside `leg2alm_internal`.

## T2047 q8 confirmation

The runner also supports explicit geometry/thread selections. A follow-up
compared the same source revisions at `lmax=2047`, `ntheta=2049`, fixed
`nphi=4096`, q8, for full `mmax=2047` and reduced `mmax=511`. This keeps the
longitude grid fixed while reducing harmonic work. Three interleaved
process orders each ran ten calibrated batches per backend and case (30
batches total), with a 50 ms target. Batch totals were 67.7–83.2 ms for the
full case and 48.9–65.7 ms for the reduced case. The cases used deterministic
shared fixtures; their SHA-256 values and per-process host records are in the
local raw capture, while the committed compact summary is
[`t2047-q8-source-ab.csv`](t2047-q8-source-ab.csv).

The measured executable SHA-256 values were baseline Rust
`1df0f1bfd3c63593e68b30cb7f39b3cc0ad173ad20d9b76fd99f15dff20d1118`,
candidate Rust
`67bf0f29cac78116827843021d043e34c15280a052a840adbdf4373f0ff4f139`, and
DUCC
`3f3970c94e6046c6838619d8efac091b5ca425628e24b91e2b65afd5627641db`.
The Rust builds used release mode, Rust 1.99.1/LLVM 23.1.1, strict FP and
`-C target-cpu=x86-64-v3` (Pulp runtime V3, four f64 lanes). DUCC used GCC
15.2 with `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` for SHT
objects and a strict validation driver. q8 used the same eight physical CPUs
for all backends.

| `mmax` | Baseline Rust ms | Candidate Rust ms | DUCC ms | Candidate/baseline | Baseline/DUCC | Candidate/DUCC | Candidate relative L2 vs DUCC |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 2047 | 39.177 (38.687–39.650) | 34.094 (33.829–34.605) | 41.082 (40.216–41.582) | 0.872 | 0.958 | 0.830 | `1.18e-13` |
| 511 | 16.995 (16.872–18.419) | 16.067 (15.920–16.415) | 16.453 (16.315–16.678) | 0.947 | 1.035 | 0.977 | `1.26e-13` |

Parentheses give pooled batch p10–p90. The per-process median candidate/baseline
ratios were 0.873, 0.872 and 0.864 at full order, and 0.960, 0.943 and 0.917
at reduced order. Candidate outputs were bitwise identical to the baseline;
maximum absolute errors versus DUCC were `1.38e-15` and `1.11e-15`.
Scheduler wait stayed below 0.8% for the full-order runs; one reduced-order
pair reached 2.3% for DUCC and 1.7% for baseline Rust. CPU pressure was low
before and after the run (`avg10` 0.00% and 0.08%). Polled cpufreq samples
spanned about 1.75–4.77 GHz and included transition/idle values, so they do
not establish equal effective clocks. The consistent per-process ordering
supports the source change, while that frequency sampling limitation remains.

The direct-input candidate reduced the full-order T2047 q8 latency by
`5.08 ms` (13%) and the reduced-order case by `0.93 ms` (5.3%). At full order
it moved the whole Rust transform from 0.958x to 0.830x DUCC; at `mmax=511`
it moved from 1.035x to 0.977x. These are full-transform source A/B results;
they do not assign all saved time to memory traffic alone. The outputs and
useful work are unchanged, and the route skips the intermediate weighted
order-major matrix pass as described above.

Reproduction, using the previously built binaries identified by the source
revisions above:

```sh
python3 scripts/benchmark_plan14_analysis_source_ab.py \
  --baseline-binary "$BASELINE_BIN" \
  --candidate-binary "$CANDIDATE_BIN" \
  --ducc-binary "$DUCC_BIN" \
  --ducc-src "$DUCC_SRC" \
  --baseline-revision f2249ac7c8272579fdc783a776f5fed24a3da99c \
  --candidate-revision e51da6494823debe8868e43bc2b5f722ce456e2a \
  --lmax 2047 --mmax 2047 511 --ntheta 2049 --nphi 4096 \
  --threads 8 --batch-ms 50 \
  --results-dir results/plan14-t2047-direct-analysis-source-ab
```

Reproduction runner: `scripts/benchmark_plan14_analysis_source_ab.py`. Build the baseline example from `f2249ac7c8272579fdc783a776f5fed24a3da99c`, the candidate from the source commit above, and the pinned DUCC parity binary; then run:

```sh
python3 scripts/benchmark_plan14_analysis_source_ab.py \
  --baseline-binary "$BASELINE_BIN" \
  --candidate-binary "$CANDIDATE_BIN" \
  --ducc-binary "$DUCC_BIN" \
  --ducc-src "$DUCC_SRC" \
  --baseline-revision f2249ac7c8272579fdc783a776f5fed24a3da99c \
  --candidate-revision e51da6494823debe8868e43bc2b5f722ce456e2a \
  --results-dir results/plan14-direct-analysis-ring-pack
```

Raw per-batch and host-monitor captures are kept local; the committed CSVs contain the compact reproducible summaries.
