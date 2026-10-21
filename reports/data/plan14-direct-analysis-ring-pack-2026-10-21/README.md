# T720 direct ring-spectrum analysis input A/B

Candidate source: RustSHT commit `e51da6494823debe8868e43bc2b5f722ce456e2a`. Baseline source: `f2249ac7c8272579fdc783a776f5fed24a3da99c`. DUCC: pinned `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0). The Rust baseline and candidate used Rust 1.99.1/LLVM 23.1.1, strict FP, `-C target-cpu=x86-64-v3`, runtime Pulp V3 (four f64 lanes). DUCC GCC 15.2 SHT objects used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the validation driver remained strict FP. CPU: AMD Ryzen 9 5950X. q1 was pinned to CPU 0; q8 to physical CPUs 0–7.

`warm-screen.csv` preserves two interleaved source A/B screens at T720 GL (`721x1458`, full `mmax=720` and reduced `mmax=180`), q1/q8, 30 calibrated batches per backend/case/thread across three independent process pairs. It includes medians, p10/p90, paired ratios, and batch durations. The second screen is the reproducibility check; both runs remain visible. q1 processes observed 11–16% scheduler wait. In the repeat q8 run, candidate scheduler wait was lower than baseline in each pair (0.8–1.7% vs 1.6–5.1%), so the q8 reduction is directional and should be remeasured on a quieter host. Do not infer a precise speedup from the q8 percentage alone.

`cold-call.csv` reports setup, first-call and warmed latencies separately for the repeat screen. `validation.csv` records candidate/baseline identity and error versus DUCC per process. The candidate matched baseline output bitwise; relative L2 error to DUCC was `4.63e-14` for full `mmax` and `4.96e-14` for `mmax=180`.

`stage-profile.csv` gives three independent q8 Rust test-phase profiles at the same geometry before the change and three instrumented DUCC stage runs. Rust's test probe used a deterministic finite map; the DUCC stage runner used the parity fixture, so phase values rank work and are not exact input-by-input cycle comparisons. Rust phases had 30 calibrated 50 ms batches each. The temporary DUCC stage harness collected 10 timed batches per process (140/150/140 stage calls); whole-call A/B comparisons above use 30 batches per process. DUCC `map2leg` and `leg2alm_internal` are not subtractable from Rust phase boundaries.

The order-major Rust workspace area at T720 full `mmax` is `721 x 721` complex f64 values (`8,317,456` bytes; about 7.9 MiB). The direct route avoids writing that entire weighted matrix and reading it back during pair packing. The workspace allocation itself remains unchanged. DUCC's analogous packing is in `fill_m2a` inside `leg2alm_internal`.

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
