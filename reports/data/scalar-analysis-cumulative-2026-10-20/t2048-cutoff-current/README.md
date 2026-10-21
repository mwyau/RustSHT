# T2047/T2048 cutoff comparison data

Summary: `summary.csv` contains 24 rows (three GL geometries × q1/q8 × DUCC
and Rust pair tiles 64/128/256). It records per-backend absolute medians and
p10/p90, paired Rust/DUCC batch ratios, numerical errors, cold first-call and
setup medians, scheduler wait, and sampled CPU frequency. The per-process JSON
with full batch arrays and host diagnostics is intentionally kept local and
untracked.

## Reproduction

Use Rust 1.99.1, the pinned DUCC source at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2, and the same CPU target
used by the original run. The DUCC worker is built from the committed native
full-transform harness; its SHT sources use `-O3 -ffast-math
-ffp-contract=fast -march=x86-64-v3`, while the driver has strict FP checks.

Build the Rust workers with their selected pair tile:

```sh
BUILD_ROOT=${BUILD_ROOT:-build/t2048-cutoff}
for tile in 256 128 64; do
  cfg=""
  if [ "$tile" = 128 ]; then cfg="--cfg rustsht_scalar_analysis_pair_tile_128"; fi
  if [ "$tile" = 64 ]; then cfg="--cfg rustsht_scalar_analysis_pair_tile_64"; fi
  target="$BUILD_ROOT/rust-target-$tile"
  RUSTFLAGS="-C target-cpu=x86-64-v3 -Aunexpected_cfgs $cfg" \
    CARGO_TARGET_DIR="$target" \
    cargo build --release --example ducc_full_transform_parity --features parallel
  cp "$target/release/examples/ducc_full_transform_parity" \
    "$BUILD_ROOT/rust-tile-$tile"
done
```

Verify the worker's selected tile by running
`scalar_analysis_pair_tile_matches_build_configuration` in `src/legendre/mod.rs`
with the same `RUSTFLAGS` and `CARGO_TARGET_DIR` for each build. Build the DUCC
worker using the CMake command documented in
[`ducc-full-transform-parity-2026-10-20.md`](../../../ducc-full-transform-parity-2026-10-20.md).

Build the DUCC native worker with the same CMake target used by the full
transform benchmark:

```sh
DUCC_SRC=${DUCC_SRC:?set the pinned DUCC 42ce8ab5 checkout}
DUCC_BUILD_DIR=${DUCC_BUILD_DIR:-build/ducc-t2048}
cmake -S reports/data/ducc-full-transform-parity-2026-10-20 \
  -B "$DUCC_BUILD_DIR" -DDUCC_SRC="$DUCC_SRC" \
  -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_COMPILER=g++-15
cmake --build "$DUCC_BUILD_DIR" --parallel 4
```

```sh
BUILD_ROOT=${BUILD_ROOT:-build/t2048-cutoff}
DUCC_BUILD_DIR=${DUCC_BUILD_DIR:-build/ducc-t2048}
DUCC_SRC=${DUCC_SRC:?set the pinned DUCC 42ce8ab5 checkout}
RAW_JSON=${RUSTSHT_RAW_JSON:?set a local path for untracked process diagnostics}
python3 scripts/benchmark_scalar_analysis_tile_cutoff.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-binary "$DUCC_BUILD_DIR/ducc_full_transform_parity" \
  --tile256-binary "$BUILD_ROOT/rust-tile-256" \
  --tile128-binary "$BUILD_ROOT/rust-tile-128" \
  --tile64-binary "$BUILD_ROOT/rust-tile-64" \
  --raw-json "$RAW_JSON" \
  --summary-csv reports/data/scalar-analysis-cumulative-2026-10-20/t2048-cutoff-current/summary.csv
```

The committed CSV was derived from three interleaved rounds. Tile order rotates
around the DUCC process. Each worker includes one cold call and a warmup; long
operations provide five whole-call samples per process and faster q8 calls use
calibrated 25 ms batches. The numerical output is read and compared only after
each timed worker exits. To regenerate the compact CSV from a retained local
capture, pass `--summarize-raw "$RAW_JSON"` and
`--summary-csv <path>`.
