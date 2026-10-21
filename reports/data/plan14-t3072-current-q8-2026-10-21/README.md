# Plan 14 current-source T3072 q8 analysis rebaseline

This recheck uses RustSHT source `140b4bff8cb5a8d53ec87666229be2f5f0a2b5ee`
and pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. It confirms the
existing Plan 14 finding that T3072 full-order q8 analysis no longer has a
Rust excess. The source change since the prior rebaseline affects only f64
n=361 inverse synthesis.

The saved JSON records the geometry, warm/first-call samples, process medians,
output errors and process scheduling counters. Three interleaved Rust/DUCC
process pairs each recorded five individually timed warm calls per backend;
one call exceeded 100 ms, so calls were timed whole rather than batched. The
deterministic coefficient outputs were compared after each pair. This targeted
recheck omitted the benchmark harness's separate post-timing edge-sidecar
sweep; exceptional-input and adjoint release tests had passed on this source.

The CPU was an AMD Ryzen 9 5950X. Rust used release, strict FP,
`-C target-cpu=x86-64-v3` and runtime Pulp V3. DUCC used GCC 15.2 with its
pinned x86-64-v3 SHT flags. Rust's process medians were 105.014, 106.014 and
112.200 ms; DUCC's were 114.496, 115.653 and 118.248 ms. The median-of-sample
latencies were 106.014 and 115.653 ms (0.917x, Rust faster by 9.639 ms), with
relative L2 error `1.9613e-13`. The Rust process runs accumulated substantially
more involuntary context switches, so preserve the observed spread and treat
the magnitude as directional.

The benchmark runner and binaries were the existing native full-transform
parity harness at `scripts/benchmark_ducc_full_transform_parity.py` and its
compiled worker binaries. The one-off runner used that harness's deterministic
fixture, pinned worker launch, timing/result helpers and 25 ms calibration
setting; for calls above 100 ms it retained five individual warm calls per
process. Build flags and source revisions are in `summary.json`.
