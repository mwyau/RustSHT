# Fixed-`sht_ftol` cutoff follow-up

`summary.csv` contains the matched T2047/T2048 q1/q8 matrix after removing
RustSHT's additional latitude-count gate. It retains Rust and DUCC medians,
p10/p90, paired ratios, the 64/128/256 tile diagnostics, output errors,
first-call latency and reset checks. `validation-summary.json` contains the
compact T3072 and cancellation results. Raw per-process samples and host
diagnostics remain local and untracked.

The pre-change results are preserved in
[`t2048-cutoff-current/`](../t2048-cutoff-current/). Baseline source was
`86f43486`; the fixed-threshold candidate is `0bc5f153`. Both use Rust 1.99.1,
Pulp V3, DUCC 0.42.0 at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2, DUCC SHT flags
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, and a strict DUCC
validation driver. The CPU was an AMD Ryzen 9 5950X.

Rebuild tile workers using the commands in the original
[`t2048-cutoff-current/README.md`](../t2048-cutoff-current/README.md), with
the candidate source checked out. The production tile remains 256; 128 and 64
are diagnostic comparisons only. Then run:

```sh
DUCC_SRC=${DUCC_SRC:?set the pinned DUCC 42ce8ab5 checkout}
BUILD_ROOT=${BUILD_ROOT:-build/t2048-fixed-ftol}
RAW_JSON=${RUSTSHT_RAW_JSON:?set a local, untracked path for process diagnostics}
python3 scripts/benchmark_scalar_analysis_tile_cutoff.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-binary build/ducc-t2048/ducc_full_transform_parity \
  --tile256-binary "$BUILD_ROOT/rust-tile-256" \
  --tile128-binary "$BUILD_ROOT/rust-tile-128" \
  --tile64-binary "$BUILD_ROOT/rust-tile-64" \
  --raw-json "$RAW_JSON" \
  --summary-csv reports/data/scalar-analysis-cumulative-2026-10-20/t2048-ducc-fixed-ftol/summary.csv
```

Three interleaved process rounds were used. Long q1 transforms have 15
whole-call observations per backend/tile; q8 used 30 calibrated batches.
The fixed geometry is `mmax=2047`, `nphi=4096`, with T2047 at 2048 and 2049
latitudes and T2048 at 2049 latitudes. The T3072 q1 check used full
`mmax=3072`, `ntheta=3073`, `nphi=6174`; it compared both analysis and
synthesis and ran the native harness's basis, roundtrip, zero, extreme,
subnormal and adjoint validations.
