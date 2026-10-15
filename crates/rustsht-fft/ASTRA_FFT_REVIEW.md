# Independent RustSHT-FFT Technical Review

## 1. Executive Summary

RustSHT-FFT has an appropriate foundation for repeated scientific FFTs: immutable plans, caller-owned reusable workspaces, explicit normalization, native packed real passes, and portable scalar execution with runtime AVX2 selection. Its main shortcomings are concentrated in arithmetic and execution coverage. They do not justify rebuilding FFTW's planner or replacing the library's ownership model.

Five findings determine the next steps:

1. **Verified source finding:** complex general-prime butterflies still calculate every output independently, recomputing input pair sums and differences. Phase-index caching removed modulo operations but did not share the arithmetic of outputs k and p−k. Admiral's symmetric butterfly and FFTW's generator explicitly share those outputs. This is the strongest bounded opportunity for C2C N=146, 722 and 1442, and the complexified real N=1442 path.
2. **Verified source finding:** every requested mixed C2C length outside the powers of two misses the current explicit AVX2 complex kernels. Their power-of-two stages have odd inner dimensions; their remaining odd radices have no explicit SIMD implementation. Adding another butterfly behind the existing divisibility guard would repeat a rejected experiment.
3. **Verified source finding:** real radix 7 and 11 use general-prime loops, including expensive interior passes. Native real radix-11 specialization directly targets N=726. Larger even real lengths, including 1452, instead use half-length C2C, so improving only the real butterfly will not improve those routes.
4. **Verified source finding:** Rader is already selected for ordinary f64 real N=257 and 723. Its transformed kernels and permutations are already cached. Remaining opportunities are fused real gather/scatter, input-dependent fallback analysis, and carefully bounded route expansion. “Add Rader” is stale advice.
5. **Previously measured result:** historical comparisons identify significant prime-stage gaps, but no matched current-head FFTW/Admiral comparison exists in the inspected evidence. Earlier RustFFT/DUCC ratios predate some accepted optimizations. Their ratios cannot establish today's gaps.

Start with a reproducible baseline, then implement paired-output complex prime arithmetic and a bounded real radix-11 experiment. Preserve the numerical fallbacks. Defer full Good–Thomas, general four-step, runtime autotuning and a wholesale SIMD abstraction change.

Evidence for these findings: complex planner and butterflies in src/complex.rs:194–265, 952–983, 1071–1128, 1339–1385; real selection in src/real.rs:134–186; real dispatch in src/real_multipass.rs:373–411; Rader in src/rader.rs:36–198; historical measurements in docs/m5-sht-performance-report.md:13–17, 374–399, 497–542.

## 2. Repository Revisions and Scope

| Repository | Local root | Inspected HEAD |
|---|---|---|
| RustSHT, containing RustSHT-FFT | /home/albert/RustSHT | d9105f6cef722ffab98a05461778f890b478948e |
| FFTW3 | /home/albert/fftw3 | 40e620ec4441d0502e55d73744ef5d29a340a288 |
| Admiral | /home/albert/admiral | ce67bc00ff9e6de96bf5b75f7a789581105ddb58 |

Review date: 2026-10-15. Working directory: /home/albert/RustSHT/crates/rustsht-fft. All three working trees were clean when inspected. References below use **R** for this working directory, **F** for /home/albert/fftw3, and **A** for /home/albert/admiral. Line numbers refer to these checkouts.

Principal source entry points:

- Rust: [complex](src/complex.rs), [real](src/real.rs), [real multipass](src/real_multipass.rs), [terminal inverse](src/real_multipass/inverse_terminal.rs), [Rader](src/rader.rs), [SIMD](src/simd.rs), and the complex/real integration tests.
- FFTW: [planner](/home/albert/fftw3/kernel/planner.c), [complex decomposition](/home/albert/fftw3/dft/ct.c), [Rader](/home/albert/fftw3/dft/rader.c), [Bluestein](/home/albert/fftw3/dft/bluestein.c), [real Rader](/home/albert/fftw3/rdft/dht-rader.c), real decomposition/conversion solvers, SIMD applicability, transpose helpers, and FFT/codelet generators.
- Admiral: [plan](/home/albert/admiral/include/admiral/detail/plan.hpp), [cost model](/home/albert/admiral/include/admiral/detail/base_cost_model.hpp), [butterfly](/home/albert/admiral/include/admiral/detail/butterfly.hpp), twiddles/factor scheduling, Rader, Bluestein, real_fft, Good–Thomas, four-step and scratch headers.

Historical evidence includes M0, M3 prime/scalar reports, M4 real multipass, real Bluestein, FMA and across-row reports, M5 workload/performance results, M6 phase/Rader CSV records, readiness/API documents and the review log. Production source takes precedence over their descriptions. FFT-facing RustSHT adapters were inspected only to confirm continued RustFFT use, normalization and buffer contracts. SHT numerical kernels were excluded.

This was static inspection only. No compilation, tests, benchmarks, profiling, package installation, code generation, network access, subagents, source changes, commits or pushes were performed. This report is the sole file created. The externally referenced writing skill was not found locally; its remote contents were not retrieved under the network prohibition.

The evidence labels mean:

- **Verified source finding:** directly established by the inspected source, not a runtime measurement.
- **Previously measured result:** a recorded result with its historical configuration.
- **Reasoned hypothesis:** an algorithmic or architectural inference.
- **Requires benchmark:** a proposed performance claim remains unestablished.

The FFTW checkout exposes generators and kernel interfaces; this review did not generate or exhaustively inspect generated codelets. Neither a compiler's actual instruction selection nor universal numerical correctness can be proved from this inspection.

## 3. RustSHT-FFT Architecture Assessment

### Planner and kernels

**Verified source finding:** the standalone crate is its own Cargo workspace, BSD-3-Clause, with num-complex and num-traits runtime dependencies and RustFFT as a development reference. Root RustSHT still uses RustFFT. The root project's restriction on custom FFTs describes its existing integration policy; this review concerns the explicitly requested independent crate, not an integration change. See R/Cargo.toml and /home/albert/RustSHT/src/fft.rs:39–79.

Complex factorization consumes radix 8, then 4, moves a remaining radix 2 to the front, and appends odd factors in ascending order. Fixed butterflies cover 2, 3, 4, 5, 7, 8 and 11. Odd factors 13–109 use direct paired-input arithmetic with cached u8 phase indices; factors at least 110 use Bluestein. Each pass records radix, l1, ido and twiddles. Plan construction stores a full root table and builds nested convolution plans. There is no cost comparison or measured planning. Sources: R/src/complex.rs, try_new_with_bluestein:194–265, factorize:1582–1615.

C2C executes its first pass in place and alternates later passes between caller storage and workspace.output. Depending on stage parity, a final copy restores the promised in-place result. Normal inverse inputs defer 1/N scaling until the last pass; large or nonfinite values retain stage scaling and residual recovery. Sources: execute_with_simd_unchecked:372–423; execute_pass_inner:849–949.

Real factorization uses radix 4, a leading residual 2, and ascending odd factors. It chooses half-length C2C only for even N>1000 with either N>10000 or a factor>5. Otherwise it constructs native packed passes, with recursive fallback. R2C traverses passes in reverse construction order; C2R uses construction order. “Terminal prime” means ido=1 in the descriptor: that pass executes first in R2C. Sources: R/src/real.rs:134–186, 1278–1306; R/src/real_multipass.rs:235–340.

### Important workload routes

The following routes are derived from current source for ordinary finite inputs. They are not measured plan traces. Gp means a direct general-prime pass; Bp means a Bluestein prime pass. Real native Gp has explicit f64 AVX2 only when p≥17 and ido=1. f32 uses scalar general-prime arithmetic.

| Real N | Factorization / native descriptor order | Current real route |
|---:|---|---|
| 256 | 4,4,4,4 | Native packed real |
| 257 | 257 | f64 bounded Rader-257; f32 B257; f64 unsafe-input fallback B257 |
| 512 | 2,4,4,4,4 | Native packed real |
| 513 | 3,3,3,19 | Native; terminal G19 |
| 722 | 2,19,19 | Native; interior and terminal G19 |
| 723 | 3,241 | f64 bounded Rader-241; f32 B241; f64 fallback B241 |
| 726 | 2,3,11,11 | Native; interior and terminal G11 |
| 1023 | 3,11,31 | Native; interior G11 and terminal G31 |
| 1024 | 4,4,4,4,4 | Native packed real |
| 1025 | 5,5,41 | Native; terminal G41 |
| 1441 | 11,131 | Native; interior G11 and terminal G131 |
| 1442 | 2,7,103 | Complexify at 721: C2C 7,103 |
| 1443 | 3,13,37 | Native; interior G13 and terminal G37 |
| 1452 | 4,3,11,11 | Complexify at 726: C2C 2,3,11,11 |
| 2047 | 23,89 | Native; interior G23 and terminal G89 |
| 2048 | 2,4,4,4,4,4 | Native packed real |
| 2049 | 3,683 | Native terminal B683, M=1372 |

N=723 is composite, not a prime-length FFT. It executes three real radix-241 child transforms. N=1441's real radix 131 remains direct even though the complex planner would use Bluestein for 131. These distinctions invalidate a single universal prime threshold.

| C2C N | Descriptor order | Current explicit SIMD opportunity |
|---:|---|---|
| 146 | 2,73 | None; G73 |
| 150 | 2,3,5,5 | None |
| 300 | 4,3,5,5 | None |
| 256 | 8,8,4 | Interior power-of-two passes |
| 512 | 8,8,8 | Interior passes and f64 terminal radix 8 |
| 1024 | 2,8,8,8 | Interior passes and f64 terminal radix 8 |
| 722 | 2,19,19 | None; two G19 passes |
| 726 | 2,3,11,11 | None |
| 1442 | 2,7,103 | None; G103 |
| 1452 | 4,3,11,11 | None |
| 2904 | 8,3,11,11 | None |

“None” concerns explicit AVX2 dispatch; compiler autovectorization was not examined. Every listed mixed length has an odd ido at its power-of-two stage, failing the lane-divisibility guard. References: R/src/complex.rs:952–983, 1071–1128, 1582–1615.

### Reuse and memory

Public methods validate lengths and use preallocated buffers. Batch methods validate complete rows, then loop serially. Real and complex workspaces include multiple buffers for execution and numerical fallback. scratch_bytes counts initialized payload, not allocator overhead, vector metadata, retained capacity or plan tables. These are useful APIs, but do not establish total resident memory. Sources: R/src/complex.rs:144–185, 269–291, 488–538; R/src/real.rs:53–117, 201–247, 309–328, 508–527.

## 4. Three-Library Architecture Comparison

| Dimension | RustSHT-FFT | FFTW3 | Admiral |
|---|---|---|---|
| Planning | Fixed factorization, thresholds, two bounded Rader selections | Registered recursive solvers; estimated or measured costs; pruning and wisdom | Route catalogue, fitted small-size ranking, deterministic gates, optional measurement |
| Complex schedule | Multipass with first pass in place | DIT/DIF, vector recursion, transpose and buffering alternatives | Codelets, DIF chains, four-step families, limited Good–Thomas |
| Real transforms | Native real passes; conditional complexify; recursive safeguards | Native real/halfcomplex codelets and decompositions, conversion and complex alternatives | Half-length complex even transforms; full complex odd transforms; batched real tiles |
| Prime algorithms | Direct, Bluestein; bounded real Rader | Direct/codelet prime methods, complex Rader/Bluestein, real Hartley/Rader | Direct/codelets, eligible Rader and Bluestein, depending on route availability |
| SIMD | Runtime AVX2, narrow coverage; scalar fallback | Build-enabled ISA families with runtime checks and layout applicability | xsimd batch types selected by build architecture; shape/register-sensitive templates |
| Scratch | Caller-owned reusable workspace | Solver-dependent; inspected prime/conversion paths allocate during execution | Stack scratch or heap; some external arenas; real execution also allocates |
| Threading | No internal worker pool | Separate threaded solver infrastructure | Pools for large transforms/ND/row execution; short inner plans can stay serial |

**Verified source finding:** FFTW's planner actually invokes applicable solvers and compares their costs. ESTIMATE uses arithmetic/other-operation counts; measured modes call measure_execution_time. DIT and DIF construct different strided child problems, rather than merely changing a factor list. Sources: F/kernel/planner.c:426–469, 518–567; F/dft/ct.c:34–57, 132–205. Its wisdom machinery adds reproducibility and applicability concerns that RustSHT does not need immediately.

FFTW's generator chooses direct primes, Rader, coprime prime-factor decomposition, split-radix or Cooley–Tukey, then schedules expression dependencies. This infrastructure can eliminate shared expressions and specialize strides. Source availability alone does not predict which generated codelet wins a particular runtime plan. Sources: F/genfft/fft.ml:53–78, 242–307; F/genfft/schedule.ml:22–31; F/genfft/gen_r2cf.ml:29–55.

**Verified source finding:** Admiral's fitted base model is explicitly limited to N=2…512. Larger lengths fall through a gate ladder. Its DIF planner separately searches divisor states and candidate radix chains. Thus “Admiral has a universal learned planner” would be inaccurate. Sources: A/include/admiral/detail/base_cost_model.hpp:27–28, 260–301; plan.hpp:389–406; twiddles.hpp:636–688, 765–766.

Admiral's small radix set depends on available vector registers. Wider builds permit 16, 32 and composite radices in addition to 2,3,4,5,7,8,11. Its generic DIF catalogue extends through selected primes up to 97. Those build choices and fitted coefficients should not be transplanted into a runtime-dispatched Rust build. Sources: A/include/admiral/detail/twiddles.hpp:52–79; cmake/CompilerOptions.cmake:43–67.

## 5. Prime-Length Algorithms

### Current Rader: useful, already integrated, still bounded

**Verified source finding:** RaderPrimePlan accepts only p=241 or 257. It stores primitive-root powers, inverse powers, both transformed direction kernels, a reusable length-(p−1) complex plan and a runtime executor. Execution gathers nonzero indices while accumulating DC, runs one forward FFT, multiplies by the cached kernel, runs a normalized inverse, and scatters with the inverse permutation. It performs no runtime kernel FFT or allocation. Sources: R/src/rader.rs:36–101, 155–198.

For p=257, M=256 replaces Bluestein M=525. For p=241, M=240 replaces M=484. Each invocation still requires two inner FFTs. N=723 invokes the latter three times, followed or preceded by its real radix-3 stage. The convolution inverse contributes 1/M; the outer real inverse contributes 1/N exactly once. Adding an extra 1/p would be wrong.

FFTW's complex Rader similarly uses two child transforms, but supports strided input/output and uses output storage for a convolution intermediate. Its inspected implementation computes permutation indices incrementally and allocates a temporary per execution. Admiral caches index vectors and a direction-specific transformed kernel; its four-step inner route creates a temporary vector. Rust's current reusable scratch is preferable for predictable repeated execution. Sources: F/dft/rader.c:45–73, 95–164, 213–261; A/include/admiral/detail/rader.hpp:75–164.

**Previously measured result:** the production Rader campaign compared f2c24008… with aaf3bb3e…, using Rust 1.99.0, default optimized bench profile, portable runtime dispatch, one worker pinned to CPU 30 on Ryzen 9 5950X, six alternating process pairs and reused buffers. Recorded paired gains span 3.15–3.40× at N=257 and 2.09–2.26× at N=723 across directions/call patterns. The host was contested. Recorded workspace payload increased from 37,584 to 46,032 bytes and 48,640 to 56,576 bytes respectively. These are historical source-relative results, not current cross-library speedups. Source: R/docs/m5-sht-performance-report.md:47–68, 497–525; benches/results/m6-rader-integration-process-samples.csv.

**Reasoned hypothesis:** fallback frequency may materially change realistic throughput. Rader rejects nonzero components smaller than a bound proportional to maximum×epsilon×16N, as well as near-subnormal, excessive-magnitude and nonfinite data. Almost-zero trigonometric samples or small spectral components can therefore select Bluestein despite moderate overall amplitude. Benchmark ordinary, sparse, cancellation-heavy and broad-dynamic-range rows separately, and record route frequency. Do not relax this guard to recover a timing. Sources: R/src/rader.rs:109–152; R/src/real.rs:1386–1424.

### Bluestein and convolution selection

Rust chooses the smallest admissible 2/3/5/7/11-smooth M≥2p−1. It precomputes reduced-square chirps and an unscaled transformed kernel; runtime scales inputs or kernel products conditionally to protect finite and subnormal results. The nested FFT executes through the scalar generic entry point. Sources: R/src/complex.rs:554–692, 1618–1660.

FFTW's inspected complex Bluestein solver selects a small-prime length, stores convolution normalization in the kernel and uses a forward plan for both convolution directions through conjugation/swap identities. Admiral's implementation uses a capped 7-smooth search, despite its file header describing power-of-two convolution. Its fallback Bluestein cost function still prices a power-of-two pad, while its fitted base model uses the actual smooth pad. These are examples of why source paths must be checked independently of labels. Sources: F/dft/bluestein.c:37–78, 82–116, 148–211; A/include/admiral/detail/bluestein.hpp:49–127; math.hpp:224–248; base_cost_model.hpp:187–209.

**Reasoned hypothesis / Requires benchmark:** the smallest M can be slower than a slightly larger M with better butterflies and usable SIMD. B241 currently chooses 484=4×11², B257 chooses 525=3×5²×7, and B683 chooses 1372=4×7³. Candidate comparisons should include 512 for B241, a few nearby admissible lengths and 1024 for B257, and nearby smooth lengths plus 2048 for B683. Price two inner transforms, multiplication, clearing, chirps and bytes—not M alone. Keep the candidate set bounded and preserve current numerical scaling.

Simply propagating SIMD to the existing nested plans is insufficient: the documented R36 experiment found no eligible passes for 275, 484, 525, 1372 or 2025 under the existing guards and reverted propagation. Changed pad selection or new tail/group kernels would be materially different experiments. Source: R/docs/review-log.md:55–58.

### Direct prime arithmetic and real symmetry

The best immediate arithmetic change is independent of convolution. For h=(p−1)/2, form a_j=x_j+x_(p−j) and d_j=x_j−x_(p−j) once. For k=1…h, compute C_k=x_0+Σa_j cos(2πjk/p) and Q_k=Σd_j sin(2πjk/p), then emit X_k=C_k−iQ_k and X_(p−k)=C_k+iQ_k for the forward transform. DC is x_0+Σa_j. The inverse changes the sign and retains existing normalization.

This identity holds for complex input; it does not assume conjugate symmetry. Rust currently recomputes both output sums separately. Admiral's radix_sym_dft explicitly shares pair arrays and output pairs; FFTW's dft_prime expresses the same reuse. Sources: R/src/complex.rs:1362–1385; A/include/admiral/detail/butterfly.hpp:21–56; F/genfft/fft.ml:53–78.

FFTW also provides a real Hartley/Rader route using real halfcomplex convolutions, optionally padded. That is a genuine further opportunity for real primes. Rust and Admiral's inspected Rader paths use complex convolution. A real specialization requires a complete derivation for both directions; simply discarding half a complex convolution is invalid. Source: F/rdft/dht-rader.c:24–30, 53–122, 249–327. Defer this larger redesign until fusion and current Rader measurements establish its value.

## 6. Real FFT Optimization

**Verified source finding:** most targeted lengths already exploit real symmetry throughout native packed passes. Forward execution starts directly from caller input. Inverse packs while checking safety, then its last pass writes normalized samples directly to caller output. Recommending either optimization as new would repeat completed work. Sources: R/src/real_multipass.rs:235–340, 1549–1588; inverse_terminal.rs:14–90.

The remaining format conversions are ordinary complex half-spectrum output after R2C and packed input before C2R. Even complexify routes pack adjacent samples into N/2 complex values and reconstruct paired bins using precomputed roots. Their large-value path may retain a second residual transform. DC and even Nyquist are explicitly real; odd N has no Nyquist endpoint. Sources: R/src/real.rs:330–417, 529–663.

FFTW offers real CT/halfcomplex-to-complex stages as well as conversion solvers. Some paths incur explicit conversion buffers; it is not universally “zero-copy.” Admiral uses the half-length method for all even lengths and full complex transforms for odd lengths. Its batched even path fuses row transposition with packing/recombination, but allocates tile scratch. Sources: F/rdft/ct-hc2c.c:31–82; rdft/rdft2-rdft.c:41–129; A/include/admiral/detail/real_fft.hpp:53–94, 111–229, 232–306.

**Reasoned hypothesis:** missing real small-prime specialization offers more targeted value than a new universal real transform. In R, only radices 2/3/4/5 have dedicated packed passes. General interior passes evaluate DC and complex lanes with phase-table loops. N=726 repeats radix 11; N=1023 and 1441 also contain interior radix 11. Implement fixed-size equations and constant roots in the existing packed layout, first scalar and shared between f32/f64. A later radix-7 version can cover N=98 and 147. Source: R/src/real_multipass.rs:373–411, 562–820.

The f64 terminal real-prime SIMD kernel and paired inverse outputs are already present. It uses narrower two-double arithmetic in several reductions despite an AVX2 entry point. Wider vectors are not automatically better when horizontal reductions, tails and coefficient loads dominate. f32 lacks this prime SIMD path; a shared scalar improvement should precede duplicate intrinsic code. Sources: R/src/simd.rs:146–151, 196–230, 1160–1263; R/src/real_multipass.rs:173–194, 717–767.

## 7. Complex FFT and SIMD

Current explicit C2C SIMD stores interleaved complex values: two f64 or four f32 complex numbers per AVX2 vector. Unaligned loads/stores preserve ordinary slice alignment. Complex multiplication permutes lanes and combines products with add/sub. There is no current FMA dispatch or ARM vector path. Sources: R/src/simd.rs:117–243, 247–280, 557–586.

The normal kernels vectorize ido and require complete lane groups. The special f64 radix-8 ido=1 kernel instead groups independent butterflies across l1. This demonstrates an existing alternative vectorization axis; a new odd-prime or radix-11 kernel should select a useful axis explicitly. Sources: R/src/complex.rs:1079–1128; R/src/simd.rs:461–503.

**Reasoned hypothesis:** for general primes, paired-output arithmetic should be established before deciding between SIMD over frequencies and SIMD over butterfly groups. Frequency blocks allow contiguous precomputed coefficients but can enlarge plan tables. Group lanes reuse scalar roots but may require interleaved gathers/transposes. An O(p²) floating-point coefficient table can exceed L1 at p=103 or 131; the existing u8 table is much smaller. Measure total public calls and retained plan bytes.

For smooth mixed lengths, allow full vector chunks plus a scalar tail in early radix-2/4/8 passes. Separately test group-vectorized small odd butterflies where ido is odd or one. A tail-capable radix-4 stage would actually run at N=300 or 1452; the earlier radix-7 experiment could not run at its tested shapes. The benefit remains unmeasured and may be small when a later prime dominates.

Admiral uses separate real/imaginary vectors, compile-time roots, paired odd outputs and register-budget-sensitive radix choices. FFTW constrains codelet applicability by stride, alignment and vector length, with CPU/OS checks before enabling an ISA. These support deliberate layout specialization, not an inference that SoA is always faster. Sources: A/include/admiral/detail/butterfly.hpp:21–56; twiddles.hpp:52–79; F/dft/simd/common/genus.c:26–46; F/simd-support/avx2.c:46–80.

No new raw-intrinsic framework is needed. The standalone crate already has private raw AVX2 kernels and no pulp dependency. Extend that boundary only for a measured reachable kernel, retaining a scalar algorithm. There is no evidence here that pulp is inadequate; introducing or replacing an abstraction would be a separate maintenance decision. Do not introduce global target-cpu=native or AVX-512 requirements.

## 8. Planning and Alternative Decompositions

The smallest useful planner improvement is a private prime-pass descriptor selecting Direct, Rader or Bluestein with a small admissible convolution-pad set. It should initially preserve existing defaults and expose route metadata to tests/benchmarks, not become a public backend plugin interface.

A useful cost estimate includes number of butterflies N/p, direct paired arithmetic, two inner FFT costs, achievable SIMD lanes/tails, permutations, bytes moved, safety scans and retained workspace. Precision and operation matter: real terminal G131 is different from complex B131, and an interior real prime differs from ido=1. No fitted coefficient can be justified from this static review.

**Verified source finding:** Admiral's Rader gate prices two inner transforms plus an element term, and four-step split selection penalizes strides. Its DIF chain search considers register/vector shape. FFTW compares whole candidate plans. These are useful cost categories, not portable parameter values. Sources: A/include/admiral/detail/rader.hpp:36–72; four_step.hpp:48–72; twiddles.hpp:636–688; F/kernel/planner.c:426–469.

Good–Thomas replaces interstage twiddle products with CRT permutations for coprime factors. Admiral's whole-transform catalogue is limited to selected small lengths, including 10,12,15,20,24,30,40,48,60; it also uses coprime decomposition inside butterfly templates. It does not supply a general Good–Thomas engine for N=1442. FFTW's generator uses prime-factor decomposition when its chosen factors are coprime. Sources: A/include/admiral/detail/good_thomas.hpp:28–32, 96–105, 320–329; butterfly.hpp:77–104; F/genfft/fft.ml:281–296.

Four-step reorganizes N=n1n2 into strided child transforms, twiddle multiplication and the other transform dimension, with transposition/gathering between layouts. Admiral implements this directly and also has large-cache blocked variants. It is different from reversing Rust's pass order. At N=1442, eliminating a few twiddles does not eliminate G103's quadratic work; 19×19 in N=722 is not coprime. Sources: A/include/admiral/detail/four_step.hpp:119–138; four_step_large.hpp:32–62; F/dft/ct.c:166–205; F/kernel/transpose.c:68–105.

**Requires benchmark:** a small 6×121 or 12×121 schedule might become interesting after improving radix 11. A general six-step implementation and cache-scale streaming stores are poorly matched to predominantly 256–4096-point transforms. Defer them.

The existing factor-reversal screen was mixed and did not justify changing defaults. Its summary lacks a retained candidate source path in the inspected benchmark sources and reports C2C timings unlike the main production table; do not use it as a current-head quantitative baseline. It still documents that simple reversal was investigated, not that Good–Thomas was tested. Source: R/docs/m5-sht-performance-report.md:548–585; benches/results/m5-factor-order-screen.csv.

Deterministic heuristics should remain the default. Optional calibration is justified only after multiple validated candidates exist and saved execution time over repeated rows exceeds measured planning overhead. Bound its candidates, record CPU/build/precision/route identity, and avoid silently timing plans during ordinary construction.

## 9. Batching, Memory and Parallel Execution

Actual consumers reuse a longitude plan over contiguous rows and a complex plan over meridional profiles. The historical workload map records L+1 scalar longitude rows and 2(L+1) spin-shaped rows; its T511 convention uses N=1025, not N=1024. GL cases do not introduce meridional resampling FFTs. These are workload dimensions, not a review of SHT mathematics. Sources: R/docs/rustsht-workloads.md; docs/m5-sht-performance-report.md:32–45; FFT-facing /home/albert/RustSHT/src/fft.rs:102–208 and src/meridional.rs:166–172, 287–309.

Keep shared immutable plans and one workspace per worker. Externally partition contiguous row ranges; reuse worker-local scratch and avoid false sharing between outputs. Existing APIs already permit this without internally threading each short FFT. No general unsafe stride API is required by the current contiguous workload.

**Previously measured result:** the earlier across-row C2C prototype converted the whole batch to sample-major order and used radix-2 stages. It lost to serial RustSHT-FFT in 22/24 reported cells, with large conversion costs. That experiment covered f64 N=256/1024, 2–32 rows, portable runtime AVX2, six processes on CPU 30, seven paired measurements per configuration, shared Ryzen host. It is not a rejection of every tiled real batch design. Source: R/docs/m4-across-row-prototype.md:3–36, 38–80, 173–211.

A materially different later experiment would use a bounded tile of rows, SoA scalar lanes and existing mixed-radix arithmetic, fuse packing with the first layout conversion, and recombine/store directly. Admiral's even real tiles demonstrate this organization. Tile scratch must be O(tile_rows×N), independent of total batch size. First compare against a simple external parallel loop; all conversions belong inside measured execution.

Memory improvements should distinguish active from retained scratch. The real workspace simultaneously owns recursive, multipass, Bluestein and Rader storage even when only one route executes. Nested complex workspaces also exist per prime stage. Safe reuse of mutually exclusive buffers could reduce per-worker memory, but removing fallbacks or allocating them lazily during execution would violate existing behavior. Sources: R/src/real.rs:201–247; src/complex.rs:269–291.

FFTW's Rader/Bluestein and conversion solvers allocate in apply; Admiral uses stack/heap scratch and allocates real tile/scalar buffers. They are useful algorithm references, not models for Rust's execution allocation contract. Sources: F/dft/rader.c:104; dft/bluestein.c:87; rdft/rdft2-rdft.c:85; A/include/admiral/detail/scratch.hpp:15–20, 51–70; real_fft.hpp:84–94, 232–258.

## 10. Numerical Correctness and Safety

No new numerical defect was established by this review. That conclusion is limited to inspected source and existing test definitions; tests were not executed.

**Verified source findings:**

- Forward exponent is negative and unnormalized; inverse is positive with 1/N. Rader/Bluestein private transforms return unnormalized outer prime results, with normalized inner convolution.
- Real inverse intentionally ignores imaginary DC and even Nyquist. The RustSHT adapter rejects them, so future integration must preserve its stricter behavior. R/src/real.rs:471–475; /home/albert/RustSHT/src/fft.rs:159–166, 192–194.
- Safe Rust slices and private workspace fields prevent ordinary input/output aliasing misuse. C2C promises in-place results but uses out-of-place internal scratch. Real public operations have distinct input/output buffers.
- AVX2 selection checks runtime support; unsafe accesses are guarded by pass dimensions, lane widths or explicit tails. New tail/group paths require their own bounds proof.
- Allocation lengths are checked before factorization; chirp products use u128 modular arithmetic; batch extent multiplication is checked. R/src/complex.rs:199, 566–576, 1764–1788, 1808–1835.
- Input scans, conditional prescaling and residual correction are intentional numerical work. Their removal is not a neutral optimization. R/src/complex.rs:624–691, 878–945, 1276–1287; R/src/real.rs:552–629.

Existing tests independently exercise low lengths, analytic impulses/constants/modes, direct forward and independently supplied inverse data, endpoint layouts, large finite and subnormal inputs, invalid buffers and separate-workspace concurrency. Relevant locations: R/tests/complex.rs:155–272, 301–425, 428–711, 714–907; R/tests/real.rs:208–460, 463–687, 689–1107, 1111–1784. The Rader module also checks prime permutations against direct DFTs and safety rejection.

Coverage is not universal. The focused meridional direct-DFT test covers 146/722/1442/1452, not every listed workload. Nonterminal complex Bluestein at 113² is checked forward only in its dedicated test; real 137² covers both directions. N=2047 is absent from the large real matrix inspected at tests/real.rs:361–364. Add explicit coverage as these routes change. Large f32 tests use f64 references and enforce 2e−5 relative-L2/scaled-max bounds; this is an existing test limit, not a claim about every input.

Risks requiring targeted tests include changed summation order, cancellation between paired outputs, underflow when pre-scaling kernels, odd-last-bin imaginary components, l1>1 and ido>1 real layouts, vector tails, and mixed ordinary/extreme rows in a batch. The old interior-prime layout bug required N=98 with l1=2 and ido=7; N=4097 alone could not expose it. Source: R/docs/review-log.md:52.

Future adaptive planning also affects safety: current execution validates workspace length, relying on deterministic private layouts. If identical N can acquire different route storage, length alone may no longer prove compatibility. Validate a private layout identity or required capacities before unchecked execution. This is a future design hazard, not a demonstrated current bug.

## 11. Licensing and Maintainability

The inspected standalone manifest and source headers select BSD-3-Clause. Its third-party notice identifies DUCC's offered BSD terms and revision c1ed01b8fda69d2a378190f0915f1a19762dddbf, including Martin Reinecke, Peter Bell and Max-Planck-Society provenance. Preserve applicable notices for adapted expressions and use Albert M. W. Yau for new attribution. Sources: R/Cargo.toml; LICENSE-THIRD-PARTY; src/real_multipass.rs:1–8.

FFTW's inspected source headers state GPL-2.0-or-later and attribute Matteo Frigo and MIT. Do not copy generated C or directly translate its source into a purportedly unchanged BSD-only implementation. Use independent mathematical derivations and record provenance. Admiral's local LICENSE is BSD-3-Clause-Attribution and includes a specific retained acknowledgment requirement; it is not simply ordinary BSD-3-Clause. These are observations of the local notices, not a broader legal assessment.

Maintain one generic scalar numerical implementation for f32/f64 where possible, with small private specialized kernels. A catalogue of hundreds of generated routines, per-CPU fitted models, public backend traits and a new allocation framework would impose substantial maintenance for this workload. Preserve the current standalone boundary; integration remains a separate project.

## 12. Prioritized Optimization Roadmap

The seven recommendations below are ranked within priority. Performance benefits are hypotheses until measured. Section 14 defines the common comparison and acceptance protocol.

### P0 — Highest-priority opportunities

**1. Share complex prime input pairs and output pairs.** Current: R/src/complex.rs, radix_general_prime_precomputed_phases:1362–1385. References: Admiral radix_sym_dft, butterfly.hpp:21–56; FFTW dft_prime, genfft/fft.ml:53–78. Form input pairs once, calculate k/p−k together, and handle DC separately. This reduces repeated arithmetic without changing O(p²) complexity; a radix-p pass remains O(Np), with O(p) reusable pair scratch. Target C2C 73/103/146/722/1442 and real 1442, both directions. Difficulty medium; confidence high in redundant work, medium in latency benefit. Risks: cancellation, prescale residuals, scratch aliasing and changed accumulation. Validate full direct DFTs and extreme inverse cases; benchmark these public calls against current HEAD and all available references, with 512/1024/1452 controls. Add SIMD only after scalar evidence.

**2. Specialize native real radix 11, then assess 7.** Current: R/src/real_multipass.rs:373–411, 562–820 and inverse_terminal.rs:90. References: FFTW real generator gen_r2cf.ml:71 onward and halfcomplex CT stages; Admiral butterfly.hpp:21–56 and real_fft.hpp:232–252 show fixed complex alternatives. Derive constant-root, fixed-size real equations inside existing packed layouts. Target real 726/1023/1441, then 98/147; both directions and precisions. Difficulty medium; confidence medium. Fixed-radix work remains O(N) per stage, with small fixed scratch and reduced indexing/table loads. Risks: interior packed layout and endpoint signs. Benchmark against current native passes and a half-length complex candidate where legal; do not attribute real-1452 gains to this change, since it uses C2C.

### P1 — Focused performance experiments

**3. Add a bounded prime route/pad comparison.** Current: fixed cutoffs in R/src/complex.rs:250–255 and src/real.rs:149–160; good_size_complex:1618–1660. References: A/rader.hpp:25–72, math.hpp:224–248; F/dft/rader.c:213–226 and bluestein.c:180–211. Compare direct paired arithmetic, eligible Rader and a small Bluestein pad set using actual child shapes. Start p=73/103 and existing p=241/257; retain p=683 as a Bluestein control before considering expansion. Complexity becomes O(p log p) only when inner transforms are efficient; memory O(M), plus preserved fallback. Difficulty high; confidence medium. Risks: poor p−1 factorization, plan growth and numerical range. Benchmark C2C 146/1442, real 257/723/1442/2049, direct prime calls and f32 controls. No new default without paired whole-call gains.

**4. Fuse real Rader gather and output conversion.** Current: R/src/real_multipass.rs:415–445, 487–523 surrounds Rader's own gather/scatter at src/rader.rs:166–196. References: F/dft/rader.c:116–158 uses output storage directly; A/rader.hpp:107–125 gathers directly into convolution storage. Add private real-pass accessors so strided real input goes directly into permuted convolution scratch and inverse results go directly to packed/real destinations. Keep two FFTs and O(p log p), but remove O(p) intermediate traffic; workspace reduction is possible. Target f64 real 257/723 both directions. Difficulty medium; confidence medium. Risks: DC accumulation, inverse scaling and endpoint reconstruction. Benchmark full calls and fallback-heavy distributions. Hartley/Rader is a separate P2 follow-up, not part of this fusion.

**5. Make SIMD usable for actual mixed pass shapes.** Current: R/src/complex.rs:952–983, 1071–1128; src/simd.rs:403–552. References: A/butterfly.hpp:21–56 and real_fft.hpp:111–229; F/dft/simd/common/genus.c:26–46. Add vector-prefix/scalar-tail execution for existing power-of-two passes; independently prototype group lanes for radix 3/5/11 or frequency lanes for recommendation 1. Keep arithmetic complexity unchanged and use bounded temporary storage. Target C2C 150/300/726/1452/2904 and nested 240/484/1372; include 256/512/1024 controls. Difficulty medium to high; confidence medium. Risks: tails, spills, code size and inaccessible dispatch. Prove route entry before timing. Enable nested SIMD only where a new kernel is reachable and wins.

**6. Reuse mutually exclusive fallback storage and measure plan bytes.** Current: R/src/real.rs:201–247; src/complex.rs:269–291; scratch_bytes methods. References: A/scratch.hpp:51–70 supports external arenas; F prime solvers illustrate buffer lifetime separation but allocate during execution. Establish lifetimes and share compatible typed buffers across mutually exclusive routes; deduplicate repeated immutable tables where worthwhile. Retain O(N+M) scale but lower constants and per-worker multiplication. Target 257/723/1442/2049/4097 at 1/2/4/8 workers. Difficulty medium; confidence high in redundant residency, uncertain speed effect. Risks: fallback overwrites and workspace compatibility. Require fewer retained bytes, no execution allocations, unchanged numerical behavior and no material latency regression.

### P2 — Larger architectural work

**7. Prototype bounded real batch tiles with fused conversions.** Current serial batch loops: R/src/real.rs:309–328, 508–527; rejected prototype in benches/prototypes/c2c_across_row.rs. References: A/real_fft.hpp:53–94, 111–229; F/dft/vrank-geq1.c:54–64. Use a fixed tile and mixed-radix schedule, not whole-batch radix-2 transposition. Complexity O(BN log N), tile scratch O(WN), plus unavoidable O(BN) I/O. Target even real 256/512/1024/2048 first; compare C2C only if real tiles succeed. Difficulty high; confidence low to medium. Risks: conversion cost, row tails and workspace growth. Require whole-batch improvement against current serial and external-worker baselines; defer if only isolated butterfly throughput improves.

## 13. Detailed Next Coding Session Plan

These are future implementation tasks; none was performed during this review. Each milestone should be independently reviewable and revertible. Do not combine planner changes with kernel changes in the same performance comparison.

### M0 — Freeze current evidence and route visibility

**Objective/code:** establish current-head performance before optimizing. Extend R/benches/rustfft.rs, benchmark-only route inspection around plan construction, and the reference harness. Record pass radix/l1/ido, selected prime algorithm, M, dispatch eligibility and fallback frequency. Preserve current defaults.

**Tests/benchmarks:** run existing numerical suites and the complete section-14 matrix, adding missing required sizes such as real 2047. Compare current HEAD, RustFFT 6.4.1, the inspected FFTW/Admiral revisions and pinned DUCC where available. Keep normalization and input preservation equal.

**Setup/memory:** separate fresh-process first use, plan construction, workspace creation, warmed execution, retained plan payload, workspace capacity and total worker memory. Existing setup loops also drop temporary plans/workspaces; label that lifecycle cost or separate destruction.

**Accept/revert:** accept only an auditable baseline with full-output numerical validation and exact configuration records. If a reference cannot be built later, mark that comparison missing; never fill it from unrelated reports. No algorithm prerequisite.

### M1 — Paired-output complex prime scalar kernel

**Objective/code:** implement recommendation 1 in radix_general_prime_precomputed_phases and its workspace allocation. Keep the original path as a test reference and for any safety cases whose reordered arithmetic has not been justified. Use generic T; do not introduce a coefficient matrix yet.

**Tests:** direct forward and independent inverse at 13/17/19/31/41/73/103/109, mixed 146/722/1442, plus extreme DC, subnormal and high-dynamic-range tests. Test real complexify 1442 and separate-workspace concurrency. Preserve tolerances.

**Benchmark/baselines:** both C2C directions at those primes/mixed lengths and real 1442; controls 256/512/1024/1452. Compare M0 and references under the common protocol. Record pair-scratch bytes, phase-table bytes, setup time and generated-code size during future implementation.

**Accept/revert:** retain only with repeatable target-call improvement and no material controls regression. Revert or restrict routing if changed summation breaks extreme behavior. Prerequisite: M0.

### M2 — Real radix-11 specialization

**Objective/code:** add forward/inverse radix-11 functions in real_multipass.rs and terminal handling in inverse_terminal.rs; reuse existing layout/scale conventions. Implement terminal and interior cases separately.

**Tests:** direct f64/f32 at 11/22/121/242/363/726/1023/1441; vary l1 and odd ido. Include independent inverse spectra, zero/impulse/constant/modes, endpoints, cancellation and finite extremes. Keep 98 as an interior-layout regression.

**Benchmark/baselines:** R2C/C2R 726/1023/1441; controls 513/1025/1443/1452. Compare M0/current accepted source and available reference libraries. Measure table reduction, fixed temporary size, setup and worker scratch.

**Accept/revert:** require public-operation improvement across repeated rows; retain existing general fallback where necessary. Reject a stack-only reshuffle without an arithmetic/layout benefit. Prerequisite: M0; independent of M1. Attempt radix 7 only after radix 11 evidence warrants it.

### M3 — Reachable SIMD experiments

**Objective/code:** extend complex pass loops and simd.rs with one vector-prefix/tail kernel, then one odd-group or paired-prime kernel. Add private test observability for selected dispatch. Do not re-enable old nested propagation in isolation.

**Tests:** f64/f32 vector-width boundaries, scalar tails, shifted slices, l1 odd/even, ido=1 and odd ido, both directions and extreme inverse fallback. Include 150/300/726/1452/2904 and standalone inner 240/484/1372.

**Benchmark/baselines:** compare scalar, existing runtime dispatch and candidate; use section-14 references and smooth controls. Measure complete real wrappers where nested plans change. Record setup, scratch, code size and register-spill evidence separately from latency.

**Accept/revert:** require actual kernel entry and whole-call gains. Reject unreachable routes, out-of-bounds tails, spill-driven regressions or code growth without benefit. Prerequisite: M0; paired-prime SIMD also requires M1.

### M4 — Bounded prime route and pad experiment

**Objective/code:** factor route construction into private descriptors in complex.rs/rader.rs/real.rs. Permit benchmark-only candidate forcing and compare a few M values. Implement general checked primitive-root support only for explicitly tested new primes.

**Tests:** validate permutation bijections, kernel orientation, convolution normalization and direct transforms at 73/103/241/257. Include B683 and fallback boundaries, f32-rounded inputs against f64 references, and old nonterminal Bluestein tests. Reject inappropriate Rader p−1 routes.

**Benchmark/baselines:** C2C 146/1442, real 257/723/1442/2049; existing direct primes and smooth controls. Compare every candidate against the accepted current route and all reference libraries. Measure kernel setup, retained fallback plans and per-worker memory.

**Accept/revert:** select a bounded deterministic rule only after paired improvements survive both directions and workload distributions. Keep old selection if candidate benefit is noise or memory/setup cost is unjustified. Prerequisites: M0, preferably M1/M3 so costs describe improved kernels.

### M5 — Fuse the existing real Rader boundary

**Objective/code:** change MultipassTransforms and RaderPrimePlan execution interfaces to avoid intermediate natural-order complex arrays at real boundaries. Keep selected lengths and safety predicates fixed.

**Tests:** direct prime 241/257, public 257/723, nonzero-index impulses, independent inverse bins, batch mixed rows, maximum-finite sparse exceptions and bitwise fallback agreement. Confirm one outer 1/N scale.

**Benchmark/baselines:** f64 R2C/C2R at 257/723, with 241 control; current source, RustFFT, FFTW, Admiral and pinned DUCC. Record ordinary/fallback route counts, permutation traffic estimates, plan setup and workspace bytes.

**Accept/revert:** retain only if public-call improvement exceeds run variability and fallback behavior is unchanged. Revert if removed copies were needed for ownership or residual recovery. Prerequisite: M0; independent of M4.

### M6 — Workspace lifetime reduction

**Objective/code:** document lifetimes before modifying RealFftWorkspace, ComplexFftWorkspace and nested convolution scratch. Reuse same-type buffers first; avoid unchecked reinterpretation. Add private layout compatibility validation if route variability requires it.

**Tests:** alternate ordinary/extreme inputs and forward/inverse operations on one workspace, reuse after rejected calls, same-length plans with different candidate routes, concurrent workers, and zero-row batches. Use future allocation instrumentation to confirm execution remains allocation-free.

**Benchmark/baselines:** 257/723/1442/2049/4097, all relevant directions, 1/2/4/8 workers versus the immediately preceding revision. Reference libraries provide context; own-source comparison establishes the memory change.

**Accept/revert:** require lower measured retained memory with no numerical/ownership regression and no material latency loss. Report payload and peak separately. Prerequisite: route layouts stabilized after accepted experiments.

### M7 — Optional tiled real batch prototype

**Objective/code:** prototype recommendation 7 outside production routing, using fixed worker-local tile scratch and fused real packing/recombination.

**Tests:** both precisions, directions, row counts 0/1/2/3/W−1/W/W+1 and mixed extreme rows; compare every row with direct/reference and ordinary plan execution.

**Benchmark/baselines:** real 256/512/1024/2048 with 32 and SHT-shaped rows, one worker and external 2/4/8 workers. Compare current serial API, direct row loops, prior rejected prototype where applicable, FFTW plan-many and Admiral's actual batch path. Include conversions, tail handling and scheduling in wall time.

**Accept/revert:** require complete-operation gains with scratch bounded by tile size. Keep as an experiment if any advantage depends on excluding transposes or allocating a full batch workspace. Prerequisite: M0 and stable scalar/SIMD kernels; optional, not a release blocker.

## 14. Benchmark and Validation Matrix

| Axis | Required coverage |
|---|---|
| Real lengths | 256,257; 512,513; 722,723,726; 1023,1024,1025; 1441,1442,1443,1452; 2047,2048,2049 |
| C2C lengths | 146,150,300; 256,512,1024; 722,726,1442,1452,2904 |
| Prime/inner probes | 19,41,73,103,109,113,131,137,241,257,683; 240,256,484,525,1372; changed candidate pads |
| Regression controls | 1–64, 98,147, 4096,4097, 113² C2C and 137² real |
| Precision/direction | f64 first; f32 full correctness and target timings; R2C, normalized C2R, C2C forward/inverse |
| Row counts | 1 and 32; L+1 and 2(L+1): 128/256,256/512,361/722,512/1024,721/1442,1024/2048; meridional workload rows separately |
| Workers | Sequential single/batch at 1; external 2/4/8-worker batches; identical total work and one workspace per worker |
| Input classes | Deterministic dense, random, impulse, constant, Fourier modes, sparse spectra, cancellation, broad dynamic range, minimum-normal/subnormal, representable high amplitudes |
| Memory/setup | Fresh-process first call, plan lifecycle, workspace lifecycle, warmed calls, plan tables/capacity, worker scratch, allocator/stack overhead and peak memory |
| Portability | Portable Rust runtime dispatch and forced scalar test path; x86 AVX2 hosts separately; ARM/scalar validation and timing when available; no universal host claim |

Use at least six alternating paired processes for a first decision, with repeated timed blocks long enough to exceed timer noise. Fix CPU affinity and worker counts, record CPU/cache/OS/compiler/flags/precision/routes, and keep other load quiet. Report median paired ratios and uncertainty, not only ratios of independent medians. Broaden a campaign only when uncertainty or a regression needs resolution.

Compare the candidate with its immediate predecessor and original reviewed HEAD. Use RustFFT 6.4.1 and the exact local FFTW/Admiral SHAs above. Specify FFTW ESTIMATE versus MEASURE, wisdom state, alignment and input-preservation flags. Specify Admiral effort, build architecture, fast-math settings and thread count. Run one-thread comparisons before library-specific threading. Normalize FFTW/RustFFT inverse outputs inside the same measured public-operation boundary used for RustSHT-FFT.

DUCC historical evidence uses source c1ed01b8fda69d2a378190f0915f1a19762dddbf, g++ 15.2.0, portable -O3 -DNDEBUG or separately labeled -march=native builds. A later ducc0 comparison must record its package version and Python/batching boundary; it cannot be merged silently with direct C++ 1D timings.

Restore destructive inputs outside the timing interval consistently, but include any copy required by the chosen public preservation contract. Include packing, normalization and output conversion for real transforms. Record single-transform latency, serial throughput, parallel throughput and setup separately. Existing R/benches/rustfft.rs:669–789 uses serial row loops and a first-output checksum; that checksum is not numerical validation.

Validate full outputs before timing. Use direct DFTs and analytic inputs, independent inverse spectra, Hermitian endpoints, Parseval/inner-product identities and reference-library comparisons. For real Parseval, weight interior positive-frequency bins twice and DC/even Nyquist once. Use scale-aware metrics and existing tolerances; roundtrips alone are insufficient. Extreme tests should use analytically representable results rather than overflowing the reference sum.

A performance change is accepted only when paired uncertainty supports an improvement in its declared target cells, both directions are characterized, and controls show no material regression. Set any numeric regression gate before collecting candidate results. A lower arithmetic count is not a speedup measurement. A memory optimization may be accepted for reduced residency with neutral speed. No FFT result establishes a complete RustSHT speedup.

## 15. Rejected Ideas and Open Questions

Do not repeat these experiments without a materially different mechanism:

- **Blanket radix reversal:** already screened without stable overall benefit; it can also disable terminal real-prime specialization. R/docs/m5-sht-performance-report.md:548–585.
- **Blind nested SIMD propagation or the former radix-7 SIMD kernel:** existing guards made the relevant kernels unreachable; the experiment was reverted. R/docs/review-log.md:55–58.
- **Mechanical FMA substitution:** reverted after small mixed gains and consistent 4096 regressions in the recorded campaign. R/docs/m4-c2c-fma.md:72–90. This does not prohibit FMA in a new independently justified kernel.
- **Radix-11 stack temporaries alone:** the previous experiment regressed nested inverse 484 and was reverted. Fixed real equations or a different vectorization axis are different work. R/docs/m3-scalar-performance.md:392–395.
- **Whole-batch transposition with the old radix-2 prototype:** rejected on whole-call performance and workspace. R/docs/m4-across-row-prototype.md:203–211.
- **Reintroducing terminal real radix-4 SIMD alone:** the documented experiment did not justify retention; interior SIMD and fused inverse stores are already present. R/docs/m4-real-multipass.md:688–827.
- **All-even complexify, all-odd full-complex conversion, or changing longitude sampling:** none follows from the source evidence. They change distinct algorithms or application contracts.
- **FFTW-scale planning/generation, enormous-transform six-step machinery, global native/AVX-512 flags, internal threading per short FFT, or removing safety scans/fallbacks:** excessive scope or incompatible with the stated execution/correctness priorities.

Open questions remain: how often do realistic rows trigger Rader fallback; how much memory is retained by all plan tables; does paired C2C arithmetic outperform convolution at p=73/103; do better inner kernels change B683's optimal pad; can f32 prime kernels share enough machinery without code growth; and can small real tiles repay conversions? N=525 and N=4097 historical control regressions remain unresolved, not proof of a cache or compiler cause. Source: R/docs/review-log.md:53–54.

There is no current matched FFTW/Admiral performance ranking, no measured eight-worker throughput in the principal RustSHT-FFT report, and no end-to-end integration speedup. Those evidence gaps should determine future measurements, not be filled with extrapolated ratios.

## 16. Final Recommendation

Implement the baseline milestone, then paired-output scalar C2C prime arithmetic. Follow with native real radix 11 and SIMD that handles the actual odd/tail pass shapes. These changes address identifiable repeated work while preserving the existing planner, public API and numerical safeguards.

Next, compare a bounded set of prime routes and convolution lengths, fuse existing real Rader boundaries, and reduce mutually exclusive scratch residency. Keep Good–Thomas, general four-step, real Hartley/Rader and tiled batch redesign as conditional later work. Select each production change from whole-operation measurements at the actual RustSHT lengths, with setup, memory and numerical behavior reported alongside execution time.
