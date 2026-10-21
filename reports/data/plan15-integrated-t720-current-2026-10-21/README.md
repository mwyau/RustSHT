# Plan 15 integrated T720 screen

This is a focused current-source integration check after the accepted GL, CC
and F1 spin-1 changes. It covers exact latitude sampling, even-smooth longitude
sampling, full and quarter `mmax`, analysis/synthesis and q1/q8. It is a
representative check, not a replacement for the 170-case Phase A baseline or
an optimization A/B campaign.

The local raw `results.json` contains six geometries, 24 workload rows, 144
backend worker records and 72 numerical/reset comparisons. There were no
failed cases. Raw JSON and per-batch host telemetry stay untracked. The
committed `summary.csv` and sanitized `summary.json` retain latency
distributions, paired ratios, errors, fixture hashes and build metadata; both
are regenerated with `python3 summarize.py` from the local raw JSON.

The measurement used Rust source commit `32bb0a2f` (the recorded HEAD
`1033d499` changes report files only), pinned DUCC
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, Rust 1.99.1 / LLVM 23.1.1 with
`-C target-cpu=x86-64-v3`, and GCC 15.2 with DUCC SHT flags
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`. The validation driver
uses `-fno-fast-math`. The Ryzen 9 5950X q1 run was pinned to CPU 0; q8 used
CPUs 0–7 in one L3 domain. Binary hashes, affinity, batch distributions,
first-call/setup times and host diagnostics are retained in local `results.json`.

Reproduce from the repository root with the pinned DUCC checkout available:

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
python3 reports/data/plan15-integrated-t720-current-2026-10-21/benchmark_plan15_integrated_t720.py \
  --ducc-src "$DUCC_SRC" --lmax 720 --grid GL --grid CC --grid F1 \
  --process-pairs 3 --target-ms 25 \
  --build-dir "$BUILD_DIR" \
  --results-dir reports/data/plan15-integrated-t720-current-2026-10-21
```

The q8 runqueue wait was elevated in this capture; small q8 differences are
therefore descriptive rather than attributable to a specific source change.
