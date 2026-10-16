# FFT reference benchmarks

The benchmarks compare DUCC's direct 1D plans, RustFFT, and, after M1/M2,
RustSHT-FFT. The pinned DUCC checkout is not stored in this crate. Set
`DUCC_SRC` to a checkout at `c1ed01b8fda69d2a378190f0915f1a19762dddbf` and
build the reference program with:

```sh
DUCC_SRC=/path/to/ducc benches/reference/build.sh
```

The C++ program calls `ducc0::pocketfft_c` and `ducc0::pocketfft_r` directly. It defaults to DUCC's scalar planner path. Set `RUSTSHT_FFT_DUCC_VECTORIZE=1` to
request the optional vector pass and vectorization-specific real complexify
selection; the reference report records this flag. The binary converts DUCC's
FFTPACK real packing to and from the public nonnegative
complex spectrum before comparisons. C2C, R2C, and C2R comparisons use an
unnormalized forward transform and an inverse scaled by `1/n`. Use `dump MODE N` on the C++ binary and `dump-native MODE N` in the Rust
benchmark harness to compare a focused length outside the default dump set. `reference/build-f32.sh` builds `ducc_fft_f32.cpp` against the same pinned
revision. Its portable default is `-O3 -DNDEBUG`; set
`CXXFLAGS='-O3 -DNDEBUG -march=native'` for a separately labelled native
build. The f32 harness uses the same f32-rounded real and Hermitian-spectrum
inputs as `f32_real_simd.rs`, runs one transform and 32 independent rows, and
records seven timed samples per operation plus plan and scratch setup. Set
`RUSTSHT_FFT_DUCC_VECTORIZE=1` to select DUCC's vectorized planner passes. Record
that setting and the compiler flags because they change the reference path. The input helpers preserve the Rust f32 rounding sequence even when the native
compiler can contract arithmetic in DUCC's transform kernels. Plan construction and vector allocation are each measured 20 times; the
reported setup value is the upper median, and object destruction occurs after
the timed interval. Plan scratch and the reusable packed-data adapter buffer
are reported separately. The RustFFT baseline uses reusable plans and scratch. Its real adapter packs
even real inputs into a half-length complex transform and uses a full complex
transform for odd lengths, matching the current RustSHT workload. Plan
construction, scratch allocation, transform execution, real-spectrum packing,
whole-call latency, and batches of independent rows are measured separately. The standard length set is `31, 32, 33, 64, 127, 128, 241, 257, 511, 512,
683, 1023, 1024, 1025, 1026, 2047, 2048, 2049, 4097`. Threshold probes add
`107, 109, 113, 131, 137, 998, 1000, 1001, 1002, 1008, 1013, 10000, 10001,
10008`. The T workloads include T127, T511, T1023, and T2047 with the explicit
GL dimensions in `docs/rustsht-workloads.md`. CC/F1 complex sizes are derived
from the matching latitude counts. Run `benches/run-reference.sh --sht` to benchmark the actual longitude FFT row
counts for T127, T511, T1023, and T2047: 128, 512, 1,024, and 2,048 rows at
lengths 257, 1,025, 2,049, and 4,097. `bench-one MODE N ROWS` accepts an
explicit row count for additional focused measurements. Each transform timing uses one warm-up operation followed by seven timed
samples; the median is reported. Plan construction and scratch allocation use
20 repetitions. Record the DUCC commit, C++ compiler and flags, rustc version
and profile, CPU, ISA path, process affinity, thread count, and input formula
with each result. A measurement made while another compute-heavy process is
active is provisional. Use repeated process-level runs for comparisons. `cargo bench --bench rustfft -- bench-native-first-call MODE N` reports the
first public operation after plan, workspace, and input construction, followed
by the median of seven warmed calls. It uses one row and excludes process
startup, plan construction, and input allocation. Pair candidate and baseline
processes externally when comparing the values. `cargo bench --bench rustfft -- bench-native-first-call-set N ...` reports the
same first-call and warmed-call measurements for R2C and C2R at each listed
length. It uses one row per operation. `cargo bench --bench rustfft -- bench-native-rader [N ...]` measures real R2C
and C2R at N=241, 257 and 723 for ordinary, sparse, cancellation-heavy and
broad-dynamic-range inputs. It reports one transform and 32 sequential rows
per reused plan and workspace. Use `benches/paired_processes.py` and
`benches/summarize_pairs.py` for source-to-source comparisons; the M4 boundary
experiment and route evidence are in
[`docs/m4-rader-boundary.md`](../docs/m4-rader-boundary.md). `benches/summarize_rader_boundary.py` derives the predeclared six-pair gate
from the per-cell summary CSV. `cargo bench --bench rader_real_prototype -- profile-phases N r2c|c2r
[ITERATIONS]` measures the Rader radix-241 pass phases at N=723 or N=4097. Each invocation validates the modeled pass against direct DFTs before timing. The phase method and captures are documented in
[`docs/m34-rader-phase-and-radix5.md`](../docs/m34-rader-phase-and-radix5.md). The M2 native real radix-11 implementation, validation, and paired results are
documented in [`docs/m2-real-radix11.md`](../docs/m2-real-radix11.md). `summarize_first_call.py` summarizes the paired first-call captures. The paired M4 measurement for fusing real inverse eligibility with packed
spectrum conversion is in
[`docs/m4-real-inverse-pack.md`](../docs/m4-real-inverse-pack.md). `bench-profile-complex N forward|inverse [ITERATIONS]` runs repeated public
f64 C2C operations with the same input restored before each transform. Its
[`docs/m4-c2c-ido1.md`](../docs/m4-c2c-ido1.md).
