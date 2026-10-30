# Reproduction

This package contains exact source patches, standalone trace/reference programs, the GL T720 theta-bit fixture, raw traces, derived machine-readable comparisons, and SHA-256 manifests. The patches are test-only or isolated candidate patches. Apply them only in disposable checkouts. Set `SCRATCH_DIR` to a fresh writable directory (for example, `SCRATCH_DIR=$(mktemp -d)`) and `PACKAGE` to this package directory before running commands.

## Rust native-f32 candidate trace

Prerequisites used for the recorded run: rustc 1.99.1 (58cf8f9c3 2026-10-13), Cargo release test build, x86_64-v3 / AVX2+FMA host. On this host the test asserts eight f32 lanes.

Starting from a RustSHT checkout with b6a96aa7161a64f678b614d73478496f50f0551e available:

~~~sh
git worktree add --detach $SCRATCH_DIR/rustsht-numeric-candidate b6a96aa7161a64f678b614d73478496f50f0551e
cd $SCRATCH_DIR/rustsht-numeric-candidate
git apply /path/to/numerical-causality/patches/native-f32-candidate.patch
git apply /path/to/numerical-causality/patches/rust-sparse-trace-only.patch
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  NUM_TRACE_RUST_CSV=$SCRATCH_DIR/rust-native-f32.csv \
  cargo test --release --lib native_f32_ordered_t720_numeric_trace_probe -- --ignored --nocapture
~~~

For the public-map controls, run the ignored test named native_f32_public_map_t720_numeric_probe and set NUM_TRACE_RUST_MAP_CSV, NUM_TRACE_RUST_LOW_CSV, and NUM_TRACE_THETA_FILE to output paths. For the active public zonal leaf, run native_f32_zonal_production_t720_numeric_trace_probe with NUM_TRACE_RUST_ZONAL_CSV set. These two tests distinguish the public m=0 and m=1–2 leaves from the direct generic kernel probe.

The candidate patch is based on b6a96aa7 and reconstructs isolated candidate commit 45380d7e4bd849a33ffef7d91978ee18fd7e4193. The trace-only patch is applied after it. The recorded instrumented tree SHA is 9e6f800d6102de2ca87a8d6c86ac3feaeef59cb7; the recorded test binary SHA-256 is 0aecdda7267cdf3e2f0cb501c3fa7fb04400283832533e6b79589596702b0063. The test writes debug traces and is not a performance binary.

## Pinned DUCC trace

Use a disposable DUCC checkout at 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b. Apply patches/ducc-trace-instrumentation.patch. The patch supplies CMakeLists-C++.txt, adds sht.cc to its static library, and adds the smoke sources. For this standalone C++ build, copy that build file over CMakeLists.txt in the disposable checkout, then configure and build:

~~~sh
cp CMakeLists-C++.txt CMakeLists.txt
cmake -S . -B build-cpp -DCMAKE_BUILD_TYPE=Release \
  -DDUCC0_USE_THREADS=OFF \
  -DDUCC0_ARCH_FLAGS=-march=x86-64-v3 \
  -DCMAKE_CXX_FLAGS=-DDUCC_NUMERIC_TRACE
cmake --build build-cpp --parallel 1
~~~

Compile the trace harness against the resulting library with the same optimization and floating-point flags:

~~~sh
c++ -std=c++17 -O3 -DNDEBUG -ffp-contract=fast -fexcess-precision=fast \
  -fno-math-errno -fno-signed-zeros -fno-trapping-math -fassociative-math \
  -freciprocal-math -march=x86-64-v3 -DDUCC_NUMERIC_TRACE \
  -I src tracers/numeric_trace_smoke.cc build-cpp/libducc0.a -pthread \
  -o build-cpp/numeric_trace_smoke
~~~

Run a smoke case and then the complete direct-stage matrix with the packaged driver. The driver reads the exact theta bit fixture and passes each theta to C++ with a 17-digit decimal round-trip:

~~~sh
python3 tracers/run_ducc_modes.py --binary build-cpp/numeric_trace_smoke \
  --smoke --output $SCRATCH_DIR/ducc-smoke.csv
python3 tracers/run_ducc_modes.py --binary build-cpp/numeric_trace_smoke \
  --output $SCRATCH_DIR/ducc-modes.csv
~~~

The full invocation traces m=0,l=720,re and m=1,2,5,8,16 with l=600,650,700,719,720 and separate re/im unit coefficients, at rings 0,1,5,360,715,719,720. The recorded mode-trace executable SHA-256 is af456fe772d4546090b5d8644cbc657bbf58da55419f92cf0eff139fe4785429; it was built from the pinned source with the trace patch and flags above. Do not use this instrumented executable for timing.

The public-map smoke harness is tracers/numeric_map_smoke.cc. It reads the theta bit fixture and runs the public float API with nphi=34 and phi0=0. Build it with the same compiler flags and link against build-cpp/libducc0.a.

## Dense upper-band trace

The dense fixture in data/high-band-alm0.f32 contains exactly m=1..16, l=600..720 as complex little-endian f32. Its SHA-256 is bb40e9dc604d336095a364e11c10303ec90de8547aa721176101278fce386a8d. It was extracted from the archived GL T720 alm0 fixture whose SHA-256 is 03cc74b6eb5508e6c1c85f427ac183ece8177d7b36a31aefc3bfe4ece62beb96. The theta file is the same exact data/gl-t720-theta.bits used by the single-mode traces.

~~~sh
PACKAGE=/path/to/RustSHT/reports/archive/experiments/numerical-causality-ducc-0.42.0-2026-10-29
~~~

Starting from a candidate checkout at 45380d7e4bd849a33ffef7d91978ee18fd7e4193 with the Rust trace patch applied, run:

~~~sh
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
NUM_TRACE_BAND_ALM_FILE="$PACKAGE/data/high-band-alm0.f32" \
NUM_TRACE_RUST_DENSE_CSV=$SCRATCH_DIR/rust-dense.csv \
cargo test --release --lib native_f32_ordered_t720_dense_high_band_trace_probe -- --ignored --nocapture
~~~

The direct generic probe runs the native-f32 recurrence on the four northern representative rings 0, 1, 5, and 360 and emits both reflected outputs. It does not run the longitude FFT. The Rust test asserts AVX2 with eight f32 lanes.

Build the dense DUCC tracer against the pinned DUCC library configured above:

~~~sh
c++ -std=c++17 -O3 -DNDEBUG -ffp-contract=fast -fexcess-precision=fast \
  -fno-math-errno -fno-signed-zeros -fno-trapping-math -fassociative-math \
  -freciprocal-math -march=x86-64-v3 -DDUCC_NUMERIC_TRACE -I src \
  "$PACKAGE/tracers/numeric_trace_band_smoke.cc" build-cpp/libducc0.a -pthread \
  -o build-cpp/numeric_trace_band_smoke
build-cpp/numeric_trace_band_smoke \
  "$PACKAGE/data/high-band-alm0.f32" "$PACKAGE/data/gl-t720-theta.bits" \
  > $SCRATCH_DIR/ducc-dense.csv
~~~

The actual harness accepts the coefficient and theta paths, followed by an output CSV path. Its implementation restores the raw coefficient array before each ring because DUCC's inner loop prepares even/odd pairs in place. It derives each ring's north/south indices through the same `make_ringdata` pairing as public DUCC.

Generate independent binary128 and long-double outputs and compare all stage tables:

~~~sh
cc -O2 "$PACKAGE/tracers/quad_band_trace.c" -lquadmath -lm -o $SCRATCH_DIR/quad_band_trace
$SCRATCH_DIR/quad_band_trace "$PACKAGE/data/high-band-alm0.f32" "$PACKAGE/data/gl-t720-theta.bits" > $SCRATCH_DIR/quad-dense.csv
c++ -O2 "$PACKAGE/tracers/longdouble_band_trace.cpp" -o $SCRATCH_DIR/longdouble_band_trace
$SCRATCH_DIR/longdouble_band_trace "$PACKAGE/data/high-band-alm0.f32" "$PACKAGE/data/gl-t720-theta.bits" > $SCRATCH_DIR/longdouble-dense.csv
python3 "$PACKAGE/tracers/compare_dense_band.py" \
  $SCRATCH_DIR/rust-dense.csv $SCRATCH_DIR/ducc-dense.csv $SCRATCH_DIR/quad-dense.csv $SCRATCH_DIR/dense-comparison
python3 "$PACKAGE/tracers/compare_longdouble_band.py" \
  $SCRATCH_DIR/quad-dense.csv $SCRATCH_DIR/longdouble-dense.csv $SCRATCH_DIR/ducc-dense.csv \
  $SCRATCH_DIR/rust-dense.csv $SCRATCH_DIR/longdouble-comparison
~~~

The packaged `data/dense-band/` traces and derived tables are the verified outputs for these commands. They include degree-by-degree recurrence operands/states, the first divergence per order/ring, coefficient pairs, accumulation controls, prefix counts, output error tables and the binary128/long-double cross-check. The independent reference executables are diagnostic only. The dense comparison stops before FFT; the separately archived public-map matrix covers single-mode FFT and final-map propagation. Full/reduced random end-to-end fixtures remain unrun because the direct single-mode stages diverge materially.

## Independent reference and archived comparisons

Compile the independent binary128 tracer:

~~~sh
cc -O2 tracers/quad_trace.c -lquadmath -lm -o $SCRATCH_DIR/quad_trace
$SCRATCH_DIR/quad_trace 8 720 3f7f567b765c8ee8 trace
~~~

Arguments are order, selected degree, theta f64 bits, and optional trace. The code independently forms normalization, recurrence factors, sectoral seed, recurrence, even/odd accumulation, and north/south values in binary128; fmaq is used at recurrence updates.

The delivered data files are the raw traces and the derived comparisons for this recorded source/build identity. Verify file integrity from this directory with:

~~~sh
sha256sum -c SHA256SUMS
~~~

machine-readable comparisons:
- data/mode-comparison.csv.gz: 357 matched pre-FFT cases versus binary128.
- data/first-divergence-selected.csv.gz: geometry, seed, scale-transition, and first recurrence divergence fields.
- data/recurrence-growth.csv.gz: degree-by-degree native f32 / DUCC / binary128 recurrence and conditioning rows.
- data/controlled-v2.json.gz and data/controlled-v2.csv.gz: one-change precision substitutions and recurrence state comparisons.
- data/accum-stage-comparison.csv.gz: even/odd sums, reflected reconstruction, and stage outputs.
- data/public-map-comparison.json.gz and data/map-summary.json.gz: public Rust/DUCC/reference map summary and FFT consistency checks.
- data/quad-selected-traces.csv.gz: selected binary128 plan, coefficient-pair, recurrence, and output traces.

The complete raw Rust and DUCC observations are retained in data/rust-modes.csv.gz and data/ducc-modes.csv.gz. Geometry and fixture identity are in data/fixtures.json and data/gl-t720-theta.bits. Full/reduced random benchmark fixtures were deliberately not run because the direct single-mode stages already diverged.
