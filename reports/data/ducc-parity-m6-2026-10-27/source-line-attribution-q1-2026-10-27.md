# M6 source-line attribution — q1, 2026-10-27

## Result

An exact-source, Rust-only Callgrind run profiled one GL T720/full-mmax spin-1 analysis call at q1 (`ntheta=721`, `nphi=1458`, `phi0=0.23`). It collected `639,748,794` Ir, within 0.002% of the earlier q1 profile. The run completed with status 0; Callgrind printed its known `brk segment overflow` warning. This is source attribution, not elapsed-time evidence or a comparison with DUCC.

The source attribution confirms that the reflected-order fused recurrence and accumulation dominate. `analyze_spin_order_group` accounts for `534,888,388` inclusive Ir (83.61% of the whole call). The `try_fused_scalar_analysis_pair_shared_recurrence` call site accounts for `408,167,451` inclusive Ir across 3,579 calls (63.80%). The hot `FusedScalarAnalysisPair<true,false,false,true,true>` Pulp V3 vectorizer has `106,607,678` directly attributed Ir (16.66%). The two `store_analysis_pair` assignments account for `19,383,051` direct Ir (3.03%) combined; sectoral initialization accounts for `10,210,528` inclusive Ir (1.60%). These figures overlap and are not additive.

The three recent setup changes (shared-cosine safety scan, reflected active-lane mask, paired preweight/cos² fusion) each passed focused correctness but showed no supported whole-transform gain. This profile does not expose a larger low-risk setup target. No new candidate was selected. Keep the M6 f64 gap open; a next candidate needs to reduce core fused-kernel instruction count without weakening the numerical range checks or changing arithmetic order.

## Provenance and checks

- Profile base: `1b875a6bb95d5c0a4ca3cc23d82185d9c1d3990b` (`dev`). The relevant production files match `26941cf45564a9637b72b0eca87ff880472c3505`.
- Relevant production-file SHA-256 values: `src/transform/spin/analysis.rs` `4d90007dd9b16af7db9fc01c3debc2bbc1dbc31e9d1466dd0ed3377bfff44f8e`; `src/legendre/scalar_paired/analysis.rs` `356300490b207d730c93b3bcab6840bf3d9ede17df2e33399d9bdf69c09d1a21`; `src/legendre/scalar.rs` `8aa0bdf308cb433757717608fd5178afd4b1ef3510f390e259127b2d7cb1081a`.
- Profile instrumentation: saved zero-context patch [profile-instrumentation.patch](profile-instrumentation.patch), SHA-256 `ef6991cc0f736971642f0c7f34cdf28a13287a813bdf0a0d199cc8ee826d3c92`. It was applied only in a detached scratch worktree.
- Rust: Cargo/rustc 1.99.1, LLVM 23.1.1; release build with debug symbols, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, `parallel` feature. Runtime Pulp dispatch selected V3.
- Profiler: Valgrind/Callgrind 3.26.0, `--instr-atstart=no --dump-instr=yes`; the example zeros and starts counters around one transform. A local C shim compiled with `gcc -O2` supplied `m6_callgrind_zero`, `m6_callgrind_start`, and `m6_callgrind_stop` by calling the corresponding `CALLGRIND_*` macros.
- Inputs: each map has 1,051,218 f64 values (8,409,744 bytes), generated in order from Python `random.Random(1416973757)` as `random() - 0.5`. SHA-256: map0 `aca3c478da57921910ff4fa38910105c3533f04886fa97499248334d01e9b6ac`; map1 `899cc79972c5c68d754a7499aa6ecd1b06e67bb4d8762225da19312bcfe3fae3`.
- Profile SHA-256: `49f791d7a96a829b9fc149f2b7abaf2d70f7250bb4d66c6b1fbaa4cca772e407`. Worker binary SHA-256: `51930123dcd95e81959d06cfaadf6875cadea00fde0f5f211900b3c147b0bfa1`.
- The harness checked output bitwise equality against its initial result after two warm calls and the profiled call. It did not run an independent numerical reference or a timing comparison.
- The temporary worktree, Cargo home, target, fixtures, binary, and profiler data were confined to the experiment-owned scratch root and removed after this report and its CSV were verified. The scratch measured 525,315,538 apparent bytes and 4,833 files before removal; tmpfs available space rose by 537,174,016 bytes and free inodes rose by 5,588 to 949,018. No experiment worker remained.
