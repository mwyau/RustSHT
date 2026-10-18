> **Historical review, not an active implementation plan (2026-10-18).** This review describes a specific earlier source snapshot, diagnostic ideas and example benchmark dimensions. Do not treat its experiment proposals, checkpoint sequence or T511/T1023/T2047 examples as current instructions. Read [AGENTS.md](AGENTS.md), the concise [PERFORMANCE_STATUS.md](PERFORMANCE_STATUS.md) and the new [active roadmap](plans/05-validation-and-milestones.md) instead. Preserve this report as evidence.

# Integrated RustSHT architecture and performance review

Reviewed checkout: `ac2970d33469461c02b21fe99bb2ec04a0fd5ae3` (`main`). Review completed 2026-10-17. This is a static source review. No builds, tests, benchmarks, profilers, project programs, or external-library programs were run. No numerical or timing result below was newly measured during this review. Evidence labels distinguish **Verified source finding**, **Previously measured result**, **Reasoned hypothesis**, and **Requires benchmark**. A source mechanism can be verified while its performance benefit still requires measurement. Historical measurements apply to their recorded revision and environment.

## 1. Executive assessment

RustSHT has implemented the principal transform families and several substantial optimizations recommended by the earlier reviews. The current f64 code includes paired-degree scalar synthesis, fused scalar analysis, restricted shared-channel spin-1 analysis, spin-2 scalar conversion and analysis fusion, explicit workspace execution capacity, capacity-proportional analysis partitioning, selected parallel forward FFTs, and bounded lazy Legendre tables. These implementations survive in the reviewed checkout. Reimplementing them would not address the remaining gap. The largest obstacle is high-bandlimit synthesis. The atmospheric comparison reports scalar synthesis at T1023 taking about 307 ms versus DUCC's 74.5 ms at one thread, and 77.5 ms versus 8.9 ms at eight threads. At T2047 the corresponding measurements were 3449.3 versus 578.8 ms and 845.6 versus 113.6 ms. These are approximately 4.1–6.0 times DUCC latency at one thread and 7.4–8.7 times at eight threads for those two resolutions. T1279 had a still larger eight-thread gap. This cannot be explained by the FFT backend or parallel scheduling alone. The most promising remaining direction is a coordinated change to the **execution of the existing paired recurrence**: increase the fraction of work that reaches an ordinary-range fused kernel, evaluate reflected latitude pairs together, keep several independent recurrence chains active, and amortize coefficient/factor preparation over larger useful tiles. The mathematical Ishioka transformation is already present. DUCC and SHTns differ materially in their treatment of scaled prefixes, parity accumulators, degree panels, active-latitude packing, and order ownership. These differences establish hypotheses, not a measured attribution of the gap. The rejected order-owned synthesis prototype is important counterevidence. Its T511 eight-thread time increased by 32.65%, and T1023 did not improve. A new ownership experiment is justified only after changing the arithmetic/transition kernel and bounding layout costs; an equal-order partition followed by a full transpose should not be repeated. The f32 architecture is a second major opportunity. Its separate unfused recurrence paths are much slower than the optimized f64 paths in the atmospheric measurements. DUCC's SHT inner arithmetic and ring FFT scratch use double precision even with a float public array. A private experiment that widens f32 coefficients and computes Legendre work in f64 is therefore supported by both source structure and existing measurements. It needs whole-transform accuracy, memory, and conversion-cost measurements before adoption. RustSHT-FFT is **present but not integrated**. The root crate uses RustFFT; its manifest has no dependency on `rustsht-fft`. The standalone crate is a separate Cargo workspace. Its production source is unchanged from accepted M32 (`234ab2d…`), including the radix-19 kernel; M33/M34 candidates were removed. M34 already established that two 240-point transforms consume about 80% of its modeled radix-241 Rader pass. A narrowly isolated 240-point executor is a reasonable FFT experiment. It is a lower priority for high-resolution SHT synthesis: the measured T2047 scalar inverse-FFT/writeback phase was only 2.7% of total time. Two source-proven correctness defects deserve independent fixes before performance work: an unchecked workspace order-prefix capacity can cause a safe-Rust slice panic, and the root FFT adapter/scaling chain can overflow for finite representable results. Specialized spin conversion guards also have incomplete dispatch coverage; the algebraic overflow and missing fallback are visible, but their complete public reproductions remain to be run. The recommended sequence is: close these correctness gaps; establish route-resolved current measurements; improve ordinary/scaled synthesis execution and parity together; then test bounded order/latitude ownership. Pursue f32 widening and bounded independent-field batching as separate workstreams. Require progress toward matched DUCC and SHTns latency, rather than accepting a sequence of small baseline-relative gains as evidence of parity.

## 2. Repository audit

### 2.1 Revisions and state

| Repository | Local path | Inspected HEAD | State at inspection |
|---|---|---|---|
| RustSHT | `<local-home-path> | `ac2970d33469461c02b21fe99bb2ec04a0fd5ae3` | `main`; clean before this report |
| DUCC | `<local-home-path> | `80388904b8cfc32400ba84af8a6a45aabfe60f86` | Clean |
| SHTns | `<local-home-path> | `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e` | Tracked files clean; pre-existing untracked `CODEX_HANDOFF.md` and `shtns-codex-goal/` |
| Admiral | `<local-home-path> | `ce67bc00ff9e6de96bf5b75f7a789581105ddb58` | Clean |
| FFTW3 | `<local-home-path> | `40e620ec4441d0502e55d73744ef5d29a340a288` | Clean |
| ecTrans | `<local-home-path> | `7de2ee295a0df06299a9cc78595d5148643e487b` | Clean |

Existing local directories were located before inspection. No external checkout was downloaded, updated, or modified. The pinned DUCC revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` was also read directly through local Git. The relevant SHT files differ from the local DUCC HEAD in namespace spelling, not the inspected algorithms. The FFT implementation does differ: current HEAD removes an earlier cap on SIMD width. Comparisons to the historical native/portable FFT benchmarks refer to the pinned revision, not an assumed current build. Historical SHT comparisons use `ducc0` 0.41.0. FFT comparisons use pinned DUCC, RustFFT 6.4.1, system FFTW 3.3.10, and Admiral `ce67bc00…`. The inspected FFTW source HEAD is not established as the source of that system FFTW binary. SHTns `configure.ac:5` identifies version 3.7.5; no matched SHTns timing campaign is available in the reviewed RustSHT reports.

### 2.2 Complete production-source coverage

All production Rust in the following files was read. Embedded test modules were inspected selectively; they are not counted as production coverage.

| Area | Files read in full through their production definitions |
|---|---|
| Public core and ownership | `src/lib.rs`, `src/error.rs`, `src/harmonics.rs`, `src/plan.rs`, `src/workspace.rs` |
| Geometry and FFT adapters | `src/grid.rs`, `src/fft.rs`, `src/meridional.rs` |
| Transform orchestration | `src/transform.rs`, `src/spin_transform.rs`, `src/spin_transform/hemisphere.rs` |
| Legendre dispatch and scalar kernels | `src/legendre/mod.rs`, `scalar.rs`, `scalar_f32.rs`, `scalar_paired.rs`, `scalar_table.rs` |
| Spin kernels and conversions | `src/legendre/spin.rs`, `spin_f32.rs`, `spin_fast.rs` |
| Python bindings | `python/src/lib.rs`; also `python/rustsht/__init__.py`, `__init__.pyi`, and presence of the empty `py.typed` marker |
| Standalone FFT planning/execution | `crates/rustsht-fft/src/lib.rs`, `complex.rs`, `real.rs`, `rader.rs`, `real_multipass.rs` |
| Standalone FFT SIMD/helpers | `src/simd.rs`, `src/complex/radix11_ido1.rs`, `src/real_multipass/inverse_terminal.rs`, `src/simd/x86/f64_kernel/radix11_odd_ido_tail.rs`, `src/simd/x86/f64_kernel/radix19_ido1.rs`, relative to the standalone crate |

The file inventory was checked again against `rg --files`. Separate root test files are `src/transform/tests.rs`, `src/spin_transform/tests.rs`, and `tests/f32.rs`. Selected cases cover analytic fields, DUCC fixtures, adjoints, dispatch boundaries, workspace migration, extreme values, and high-degree checks. Python tests include arbitrary-map analysis, all three geometries, preallocated outputs, overlap rejection, and shared readonly inputs. Standalone tests include independent direct DFTs, runtime-route assertions, unaligned slices, tails, inverse scaling, Rader fallbacks, and concurrency with separate workspaces. Also inspected: root/Python/FFT Cargo manifests, toolchain and packaging configuration, `.github/workflows/ci.yml`, plans 00–06, both previous ASTRA reviews, the latest milestone entries, atmospheric and order-owned synthesis reports, selected recent optimization reports, FFT M31–M34 reports, and relevant benchmark harnesses. Raw sample archives and generated FFTW codelet trees were not exhaustively read.

### 2.3 What the merge preserved

`git diff 234ab2d30b8d542457398e2f7be17e7e7b9aa25e HEAD -- crates/rustsht-fft/src` is empty. This establishes production-source equivalence to accepted M32, not binary equivalence or a new performance result. The root changes since atmospheric native revision `2756ba4a1d9bcf6b7b1a1293a185e98e8e6c74b2` comprise f32 recurrence changes and test/phase-probe changes in the inspected transform/workspace files. The f64 production mechanisms discussed here are retained. Compiler layout, executable composition, or environment differences still prevent treating old timings as measurements of a freshly built current HEAD. Root CI invokes the root workspace only. Because `crates/rustsht-fft/Cargo.toml` declares its own workspace and is absent from root members, the standalone FFT is not covered by those root commands. This is a validation-maintenance gap, not evidence of a failed build.

## 3. Previous review reconciliation

### 3.1 SHT review at `d9105f6…`

| Prior recommendation | Current classification | Current-source disposition |
|---|---|---|
| R1: safe preparation at every paired synthesis entry | **Partially implemented** | `scalar_paired.rs:63` rejects nonfinite transformed pairs; scalar fallback uses original coefficients. Ordinary spin-1 orchestration checks conversion finiteness at `spin_transform.rs:1450`. Hemisphere spin-1 and spin-2 conversion paths remain distinct and lack equivalent original-coefficient fallback coverage; see C03. |
| R2: explicit workspace execution capacity | **Fully implemented** | `plan.rs:437` exposes `prepare_workspace`; retained allocation and execution budget are separate. Effective worker count respects both the pool and requested capacity. Do not replace this with implicit pool-sized growth. C01 is a separate compatibility defect. |
| R3: fused f64 scalar analysis | **Fully implemented within the accepted gate** | `scalar_paired.rs:1270` fuses recurrence and accumulation; `transform.rs` selects it at the accepted high-degree gate. Lower-band broadening was rejected. The original visitor remains a numerical/unsupported-path fallback. |
| R4: shared-channel spin-1 analysis fusion | **Partially implemented by measured scope** | `scalar_paired.rs:1470` and spin orchestration share the recurrence for Q/U. Accepted primarily for q1 at L≥255; broad multicore selection was not accepted. Spin-2 fused analysis has also been added. Recommending generic “spin analysis fusion” is obsolete. |
| R5: work prefixes and capacity-proportional splits | **Fully implemented in recurrence analysis** | `transform.rs:187,1471,1701` and `spin_transform.rs:792,3892` use active-ring work prefixes and child-capacity fractions. Residual elapsed imbalance remains an experiment, not missing implementation of the original proposal. |
| R6: order-owned synthesis | **Attempted and rejected by benchmark evidence** | The prototype in `reports/order-owned-synthesis-2026-10-15.md` lost at T511 and did not establish a T1023 gain. Current production retains latitude ownership. Section 11 defines a materially different, conditional experiment. |
| Earlier serial longitude FFT bottleneck | **Partially implemented / superseded by phase evidence** | Selected large analysis row FFTs are parallel. Synthesis inverse rows and meridional resampling remain serial. Atmospheric synthesis phase measurements now bound their possible contribution. |
| Dispatch/transactional fallback documentation | **Still outstanding in part** | The source has careful staged consumers, but eligibility and fallback decisions span several large modules and special paths. C03 demonstrates why a route inventory remains useful. |
| Existing paired synthesis and tables | **No longer applicable as new recommendations** | They were already present and have received subsequent tuning. The remaining opportunity is execution coverage, parity, and tiling, not adding a paired recurrence or an unrestricted table. |

### 3.2 Standalone FFT review

| Prior recommendation | Current classification | Current-source disposition |
|---|---|---|
| Pair positive/negative prime outputs and reuse symmetric inputs | **Partially implemented** | `complex.rs:2256` computes positive/negative outputs together, but still forms input sums/differences inside each output-pair loop. Specialized radix-31/103 kernels reuse more input/root work. The original paired-output identity is implemented; generic once-per-input-pair storage remains a separate, lower-priority trade against scratch traffic. |
| Native real radix 11, then assess 7 | **Partially implemented** | Fixed real radix 11 is selected for measured mixed passes; the isolated length-11 case retains its measured choice. Radix 7 remains general. No evidence makes 7 a principal SHT target. |
| Bounded prime routes and Bluestein padding | **Partially implemented** | Selected real Rader-257/241/683, scoped p241 padding, and preserved range fallbacks exist. Rader-103 and several padding candidates were rejected. A universal route switch remains unsupported. |
| Fuse real Rader gather/scatter | **Fully implemented** | `real_multipass.rs` and `rader.rs` directly use convolution storage. Later M33 safety-scan fusion is a separate rejected experiment. |
| Reachable SIMD prefixes/tails | **Largely implemented for f64; partial for f32/nested routes** | Power-of-two prefixes, radix 3, mixed/terminal radix 11, radix 19 terminal N722, and radix 103 N1442 are retained. Some complexify/Bluestein children still call generic execution; see F02. |
| Share mutually exclusive scratch and report plan bytes | **Partially implemented** | p241 Rader shares the existing Bluestein buffer. Broader sharing was rejected at N257. Retained-plan diagnostics exist under tests; scratch-byte methods are public. |
| Across-row SIMD | **Attempted and rejected in the tested prototype** | Public batch methods remain serial loops. A new real-tile design would need to fuse conversion and prove its whole-batch advantage; it is not a default next step. |
| Radix-19 optimization | **Fully implemented for accepted `(38,1)` pass** | Dedicated private AVX2 source remains. The `(2,19)` pass is still scalar. |
| Rader subphase measurement | **Fully implemented** | M34 measured the phases with a direct-DFT-validated model. Asking for the same measurement as a prerequisite is obsolete; current production route frequencies and end-to-end attribution are the missing evidence. |
| General radix-five dispatch | **Attempted and rejected** | All three M34 placements were removed after unrelated regressions. F01 proposes a private N240 boundary, not reintroducing a shared dispatcher arm. |

## 4. RustSHT architecture

### 4.1 Numerical contract and operator composition

`GridSpec` owns north-to-south colatitudes, ring weights, longitude count, and `phi0`. `HarmonicSpec` owns inclusive `lmax/mmax/spin`, checked packed lengths, and order-major indexing. It rejects nonreal m=0 inputs and nonzero coefficients below the spin floor. Exact analysis requires sufficient latitude sampling and excludes an even-grid Nyquist order; synthesis and its adjoint have a different sampling contract, including a real Nyquist treatment. These distinctions must survive backend and layout changes. The operators are distinct:

| Operator | Principal dataflow |
|---|---|
| S | Packed coefficients → scalar/spin latitude projection → phase and Fourier scaling → inverse longitude FFT → map |
| S* | Forward longitude FFT → phase/endpoint treatment → unweighted transpose projection |
| A, GL | Forward longitude FFT → quadrature/phase/normalization → weighted projection |
| A*, GL | Corresponding weighted synthesis composition, with its own FFT scaling |
| A, CC/F1 | Forward longitude FFT → reflected meridional Fourier resampling to prepared CC geometry → exact weighted projection |
| A*, CC/F1 | Transpose projection on prepared geometry → transpose resampling/restore operation → longitude reconstruction |

The real harmonic inner product gives m>0 twice the weight of m=0. S* is not A, and A* is not S. GRAD_ONLY and DERIV1 belong to synthesis and its adjoint; rectangular analysis remains STANDARD. `grid.rs` implements the DUCC-derived iterative GL construction through 100 rings and Bogaert-style large-grid construction thereafter, plus CC/F1 weights. `meridional.rs:215,331` implements the exact-analysis resampling and its transpose using reflected profiles, parity `(-1)^(m+spin)`, F1 phase shifts, Fourier resize, and CC weights. This is essential numerical work, not an optional layout convenience. Replacing it with low-resolution direct quadrature would change the operator.

### 4.2 Plans, scratch, and data movement

Plans retain geometry, normalization/recurrence metadata, FFT plans, optional resampling plans, and bounded immutable lazy tables. Execution scratch belongs to `ShtWorkspace` and its workers. Lazy table initialization uses synchronized one-time storage; the completed table is shared immutably. No hidden shared mutable recurrence scratch or parallel output race was identified in the reviewed ownership structure. The workspace contains ring-major spectra, a second spin spectrum, prepared spectra for CC/F1, latitude metadata, order-major scalar-analysis storage, row FFT scratch, and per-worker recurrence/coefficient buffers. Capacity preparation can increase storage while preserving a deliberately smaller execution budget. This supports independent-field concurrency without nested oversubscription. Synthesis owns latitude groups and writes disjoint Fourier rows. Every group revisits orders, recurrence factors, and, outside accepted cache gates, spin conversion. Scalar analysis prepares order-major spectra and assigns disjoint packed coefficient ranges. Spin analysis often reads ring-major spectra with order-dependent stride, using block compaction and paired channels. These choices trade preparation duplication against contiguous output and bounded scratch; neither is universally superior.

### 4.3 Scalar, spin, and specialized routes

f64 scalar synthesis uses a paired-degree recurrence with prepared transformed coefficients. Ordinary lanes can run a fused Pulp kernel. Mixed scaled/ordinary blocks can partition lanes; other cases use staged transition or the scaled visitor. Polar cutoff compaction avoids inactive work but can also divert a block from a fused path. Ordinary scalar analysis already exploits north/south parity and fused reductions at its accepted gate. Spin 1 converts E/B or derivative coefficients to two scalar sequences, shares paired recurrence work where supported, and divides by sine with a polar-safe general-spin fallback. A bounded parallel coefficient cache is used through L720, with a 17 MiB cap. A separate hemisphere synthesis path applies only to full-band minimum GL grids at L319–511 and capacities 4–8 (`spin_transform.rs:47`). These are deliberately narrow performance selections. Spin 2 uses a two-cosine scalar conversion and sine-squared reconstruction for eligible high-bandlimit rings (L≥500, sine≥0.01). Analysis applies the transpose conversion and selected fused scalar-channel accumulation. General positive/negative-spin recurrences remain for polar rings, unsupported modes and spins 3/4. The tested spin contract ends at four; support must not be inferred merely from generic-looking formulas.

f32 has separate scaled scalar and spin recurrences. It does not inherit f64 paired/table/fused scalar-conversion performance merely because the public plan type is generic. Its latest scalar normalization-skip selector matches exactly L1023 or L1279. T2047 was screened and removed.

### 4.4 Python boundary

`python/src/lib.rs:30` stores one lazily initialized workspace behind a mutex per Python plan. Calls release the GIL using `detach`, but concurrent calls on the same plan serialize on that workspace. Readonly contiguous NumPy input can be borrowed; STANDARD `*_into` methods validate writable output and reject byte-range overlap. Allocating STANDARD methods can move the output Vec into NumPy without an additional final element copy. Some adjoint/gradient/derivative paths construct and combine component Vecs, adding allocation/copy work. The binding is f64-only. It exposes reusable plans and procedural functions, but not the core's explicit execution-capacity policy, f32, or a bounded batch/workspace pool. Procedural functions construct a plan per call. These API limitations matter for throughput and small transforms; they do not explain multi-second high-L Legendre work. Public stubs agree with the inspected methods, and `py.typed` is present.

## 5. RustSHT-FFT architecture

### 5.1 Planning and reusable execution

The standalone crate has concrete real/complex plans, private scalar abstractions, caller-owned workspaces, checked sizes, and serial batch methods. RustFFT is a development reference only. Production dependencies are `num-complex` and `num-traits`. Complex planning stores roots, pass descriptors and twiddles. Factorization favors 8, then 4, moves a remaining factor 2 to the front, and processes odd factors. Fixed radices are 2/3/4/5/7/8/11; smaller odd primes use paired direct arithmetic, while radices at least 110 use Bluestein. p241 has a scoped 512-point complex convolution choice. Passes alternate input/output scratch; final parity can require a copy. Normal inverse execution defers normalization, while large inputs use guarded per-pass scaling and residual correction and bypass unsafe SIMD. Real planning uses native FFTPACK-like packed multipasses with fixed 2/3/4/5 and selected 11, general odd factors below 135, and Bluestein for larger eligible terminal primes. Larger even lengths can complexify into a half-length C2C transform: the current condition is even N>1000 and either N>10000 or a factor larger than five. Odd or unsafe cases retain native/recursive routes. Public forward spectra are nonnegative complex bins; inverse is normalized. Imaginary endpoint components are ignored by this crate's real API, whereas the root adapter rejects them. Integration must preserve the root contract explicitly.

### 5.2 Prime convolutions and SIMD

Selected f64 real routes are:

| Real length | Prime stage | Cyclic child | Fallback |
|---|---:|---:|---|
| 257 | 257 | 256 | Bluestein |
| 723 | 241 | 240, three groups | Scoped Bluestein length 512 |
| 2049 | 683 | 682, three groups | Bluestein length 1372 |
| 4097 | 241 | 240, seventeen groups | Bluestein length 484 |

Rader precomputes modular permutations and transformed convolution kernels. Real gather/scatter is already fused into convolution storage. p241 reuses the parent Bluestein buffer; other routes retain their own storage where broader sharing failed measurements. Rader checks dynamic range, including small nonzero values, before selection. Consequently an ordinary-looking SHT row with a long decaying spectral tail may take the fallback even if a random standalone row takes Rader. Runtime AVX2 covers reachable power-of-two prefixes and selected f64 radix 3/11/19/31/103 shapes. f32 SIMD coverage is narrower. The N682 child has factorization `[2,11,31]`; its specialized radix-11 odd tail and radix-31 paired-output kernel are retained. N722 is `[2,19,19]`, with the accepted terminal `(l1,ido)=(38,1)` kernel. N1442 is `[2,7,103]`, with a plan-level radix-103 root-pair cache. These are already optimized routes, not missing kernels. Unsafe architecture intrinsics are confined to private SIMD code, selected after runtime detection, using unaligned loads where necessary and explicit tails. Inspection found no demonstrated out-of-bounds SIMD access or alignment violation. Some apparently general layout combinations are unreachable by the planner; for example, concerns about a native real radix-2 stage with arbitrary outer shape must be checked against its actual first-stage placement before being called defects.

### 5.3 Remaining structural constraints

Real complexify calls generic C2C execution (`real.rs:452` and inverse branches around 740/785), and Bluestein's nested transforms also use generic execution. Thus accepted public C2C AVX2 kernels are not automatically used by every nested transform. Rader's specialized executor does reach them. This distinction should be tested before attributing gains from a standalone C2C kernel to real transforms. Plans/workspaces retain both ordinary and fallback machinery. Memory is bounded by planned lengths, but is not a single minimal scratch arena. Public `scratch_bytes` and test-only retained-plan accounting support measurement. Serial batch methods reuse a workspace without allocating per row; they do not supply across-row SIMD or parallelism.

## 6. DUCC comparison

References in this section use the pinned source unless stated otherwise: `src/ducc0/sht/sht_inner_loop.h`, `src/ducc0/sht/sht.cc`, and `src/ducc0/fft/fft1d_impl.h` under `<local-home-path>

### 6.1 The paired recurrence is already equivalent in mathematical purpose

DUCC prepares epsilon, alpha, and paired factors (`sht_inner_loop.h:187`) and transforms adjacent even coefficients before using a recurrence in cosine squared (`:1532`). Rust's `scalar_paired.rs:25,63` implements the same essential degree-pair transformation. A proposal to “adopt Ishioka” would misidentify the problem. For an ordinary pair, write schematically

`p_next = (a*x² + b)*p + p_previous`,

with transformed even/odd complex coefficients. Rust's fused one-hemisphere loop uses two vector FMAs for recurrence, two for combining even/odd coefficients with cosine, and two for real/imaginary accumulation: approximately six vector FMAs per pair, excluding setup, tails, loads, and loop control. DUCC uses the recurrence plus four separate real/imaginary even/odd accumulations, also approximately six vector FMAs, then combines parity at extraction to produce both hemispheres. The useful output covered by those counts differs. This is a stronger source-level explanation than a blanket claim that Rust does not use FMA. It does not imply a twofold whole-transform gain: transition work, factor preparation, final stores, active-lane shapes, and the existing restricted spin hemisphere route differ. A previous scalar hemisphere prototype lost. The arithmetic count identifies a candidate kernel, not an accepted implementation.

### 6.2 Blocking, dependence, and coefficient reuse

DUCC scalar scratch has 256 latitude slots and spin scratch 128 (`sht_inner_loop.h:252` and associated constants). Its ordinary scalar loop advances short degree panels across multiple SIMD latitude vectors; AVX2 and wider variants differ in panel shape. State can be stored between panels, but coefficients are reused across the vector sweep. Rust uses 64-value blocks; its fused scalar synthesis kernel (`scalar_paired.rs:1982`) runs the degree loop inside each SIMD vector. On AVX2 a full block means sixteen coefficient traversals. This favors recurrence-state residency, but each vector has a long serial recurrence dependency and revisits coefficient data. The meaningful experiment is therefore **loop order plus register tile plus degree panel**, not 64 versus 256 in isolation. A larger Rust array under the same vector-outer loop may only enlarge traffic or change cutoff mixtures. A small number of independent vector chains can hide recurrence latency without forcing every block value into registers. SHTns provides an independent example of this distinction.

### 6.3 Scaled prefixes and transition to ordinary arithmetic

DUCC's `iter_to_ieee` skips degrees while all relevant scaled values are below its significance threshold; mixed vectors use correction factors until ordinary execution is possible. Its normal kernel then avoids per-degree normalization tests. Active rings are compacted, and final SIMD padding duplicates a valid lane while suppressing duplicate output writes. Rust retains stronger coefficient-aware safeguards. The partitioned synthesis route (`scalar_paired.rs:247`) can fuse enough ordinary lanes while a scaled subset continues through a visitor for the entire remaining order. The high-L transition route is gated by block shape and sine; mixed polar-cutoff compaction can end in the visitor without retrying the fused route. The transition implementation also carries two-channel machinery when scalar synthesis supplies a zero second channel. These structures can leave expensive checks and scalar work around an otherwise efficient recurrence. The fraction of degrees in each route has not been measured. It is a leading uncertainty. Copying DUCC's basis-only zero threshold would be unsafe for Rust's extreme finite coefficients: a tiny basis multiplied by a huge coefficient may have a representable contribution. A candidate must establish a coefficient-dependent ordinary envelope or use exact binary scaling and preserve the current fallback outside it.

### 6.4 Ownership and FFT layout

DUCC schedules orders dynamically in chunks of one (`sht.cc:684,729,878,922`), with per-worker generators and coefficient preparation once per order. It chooses Fourier strides by operation/threading (`allocate_leg`, `:566`): multicore synthesis can use an order-contiguous allocation viewed with the strides needed by FFT gather. It does not require the rejected Rust prototype's full separate transpose in every such case. Rust's latitude groups own contiguous output rows and repeat per-order setup. This has good disjointness and write locality. DUCC's organization removes that repetition and allows dynamic work distribution, but can require strided FFT input. The correct comparison includes that gather, all coefficient preparation, and final map writes. A raw Legendre timing omitting layout conversion would favor the wrong design. DUCC schedules ring FFT work dynamically (synthesis chunks of eight, analysis chunks of four). Its ring scratch and FFT plans use double precision even for a float map (`sht.cc` ring helper and `map2leg`/`leg2map` regions). A disabled batch experiment is enclosed in `#if 0`; it is not evidence of active batched FFT execution.

### 6.5 Operation-specific differences

| Operation | Current RustSHT | DUCC comparison and remaining implication |
|---|---|---|
| Scalar S | Fused paired ordinary kernel, partition/transition fallbacks, latitude ownership; mostly separate hemispheres | DUCC combines parity after four accumulations and uses dynamic orders. Main candidate for architectural work. |
| Scalar A/S* | Parity and accepted fused reductions; capacity-weighted order groups | The original fusion gap is closed. Remaining panels, route coverage, weighting and scheduling need measurement. |
| Spin-1 STANDARD | Scalar conversion, shared two-channel kernels, bounded cache/hemisphere gates | DUCC's high-L scalar conversion invokes scalar inner work separately for components. Rust already shares work; copying DUCC's call structure could regress it. |
| Spin-2 STANDARD | High-L scalar conversion and selected fused analysis; general polar fallback | Similar scalar-potential principle; verify both conversion and paired-coefficient safety before extending gates. |
| GRAD_ONLY/DERIV1, spin 3/4 | General spin and derivative-specific paths where specialized routes are unavailable | Benchmark independently. STANDARD speedups do not establish speed or correctness for these operators. |
| CC/F1 A/A* | Prepared-CC Fourier resampling and its transpose | Same essential exact-analysis composition. Rust's serial per-order resampling is a later target only if its elapsed fraction is material. |
| f32 | Separate f32 recurrences and f32 FFT adapter | DUCC widens inner arithmetic. This is a substantive architecture difference with a practical experiment. |

DUCC applies GL analysis scaling in ring processing and can use unnormalized synthesis FFTs; Rust builds N-scaled spectra for its normalized inverse. That difference matters for finite range (C02), independently of speed. DUCC also permits some synthesis longitude aliasing beyond Rust's supported order limit. This is an API scope difference, not a correctness defect in a documented rejection. DUCC's FFT planner has the same broad fixed-radix/general-prime/Bluestein structure inspected in RustSHT-FFT, with cutoffs 110 complex and 135 real. No Rader route appears in the inspected DUCC planner. DUCC vector multipasses gather into separate real/imaginary SIMD representations and can pad a partial vector group. This differs from Rust's current interleaved complex SIMD lanes, but packing costs must be included before recommending a layout change.

## 7. SHTns comparison

The inspected source is SHTns 3.7.5 at `4e69ceb…`. `configure.ac:38,345` enables Ishioka by default unless disabled. Generated variants depend on build options and runtime selection; this review did not generate or execute them. The evidence is the checked-in meta-code in `SHT/kernel_SH_to_spat.gen.c`, `SHT/kernel_spat_to_SH.gen.c`, and OpenMP driver templates.

### 7.1 Genuinely different mechanisms

1. **Register blocking is separate from outer latitude storage.** Synthesis variants use NWAY=1/2/3/4/6/8 under `VSIZE2*NWAY<=32`; several independent vectors remain live. Analysis has an additional `NBLK=NWAY*16` storage level and ISA/operation-dependent LSPAN (`kernel_spat_to_SH.gen.c:45`). This is not equivalent to enlarging Rust's 64-element scratch array.
2. **Parity accumulates before reconstruction.** The synthesis template accumulates even and odd real/imaginary pieces and combines north/south outputs after the degree loop. Rust has this principle in analysis and selected spin synthesis, but not throughout high-L scalar synthesis.
3. **Configuration-time recurrence factors.** `sht_legendre.c:400,467` prepares recurrence and Ishioka coefficient factors (`clm/xlm`) across orders. Rust prepares much of this per group/order. Storing O(L²) factors is much smaller than storing O(L³) basis values, but still needs a memory/setup policy.
4. **Explicit ordinary/high-L kernels.** Scaled sine powers and degree skipping precede normal kernels. The templates use significance thresholds and shared scale decisions based on ordered lanes. These assumptions require a fresh coefficient-aware safety analysis in Rust.
5. **Dynamic order/batch scheduling.** `omp_SH_to_spat.gen.c:84` collapses order and batch with dynamic chunk one, with batch innermost for reuse. Analysis also iterates fields within each order. This is stronger evidence for a bounded field-reuse experiment than for storing every basis value permanently.
6. **Measured algorithm selection.** `sht_init.c:819` times candidate functions per operation and selects a variant. Rust already has deterministic measured gates. The transferable idea is a controlled comparison among small kernel families; hidden constructor benchmarking is not recommended.

### 7.2 Techniques already shared, and limits of the comparison

Rust already uses normalized scaled recurrences, polar cutoffs, paired-degree/Ishioka factors, SIMD latitude lanes, spin-to-scalar conversion, hemisphere symmetry in several routes, and reusable plans. SHTns's scalar/vector templates are not a new general spin-2/3/4 solution. Its vector convention must be mapped to Rust's E/B and geographic component conventions before comparisons. SHTns's m=0 analysis separately accumulates/removes a mean before remaining projection; synthesis adds the monopole late. This may help cancellation for particular inputs, but is not a source-proven explanation of Rust's high-L full-band gap. It belongs in a focused accuracy/performance experiment only after the principal recurrence changes. There is no measured SHTns parity result here. A future comparison must fix normalization, Condon–Shortley phase, m>0 storage weight, grid, nphi, batch, thread count, planner/setup policy, and allocation scope. SHTns auto-grid choices must not silently replace the requested GL dimensions. Its regular-grid algorithms are not automatically equivalent to Rust's CC/F1 exact-analysis contract. SHTns source carries CNRS/Nathanael Schaeffer CeCILL notices. Recommendations here concern mathematical and scheduling techniques. Any direct source adaptation requires a separate file-level license/provenance decision and retained notices.

## 8. Admiral/FFTW findings

### 8.1 Admiral: useful fixed-shape and layout evidence

`include/admiral/detail/butterfly.hpp:22` forms symmetric input sums/differences once and emits positive/negative frequency pairs together using compile-time roots. RustSHT-FFT already implements this principal identity. Admiral additionally separates real/imaginary SIMD planes (`simd_swizzle.hpp:48`) and simplifies exact root forms at compile time. Its decomposition helpers distinguish coprime prime-factor splits from general Cooley–Tukey splits. These are candidates for a fixed N240 executor, not reasons to copy its complete planner. `rader.hpp:25,46,68,141` chooses Rader according to child support and a cost model, then selects codelet, iterative DIF, or four-step inner execution. That supports evaluating the actual child transform, not selecting Rader from prime size alone. The default compiled codelet catalog is 2–64 plus specified extras (`src/CodeletCatalog.cmake:2`); **240 is not a default straight-line codelet in that catalog**. The inspected source does not justify claiming Admiral wins by a special 240-point codelet. Its actual benchmark route would need to be recorded. Admiral's real reconstruction uses the familiar half-length even transform (`real_recombine.hpp:9`). It also contains sum-then-half expressions. It should not be treated as a universal extreme-finite oracle for C02. The targeted source reading establishes arithmetic/layout alternatives, not stronger numerical guarantees. The local license identifies `BSD-3-Clause-Attribution` and requires a specific acknowledgment of Marco Barbone/Admiral in addition to retained notices. It is not interchangeable with an unqualified three-clause header. No Admiral source expression was copied here.

### 8.2 FFTW: plan the convolution as a complete problem

FFTW `dft/rader.c:213` makes Rader a planner candidate and marks difficult p−1 factorizations slow. Its two child plans are constructed with their actual input/output strides (`:233`), and the convolution normalization is incorporated into precomputed omega (`:50`). This provides two useful experiments for Rust's N240 child: fuse normalization only under a proven safe envelope, and optimize the whole child layout rather than one general dispatcher arm. `dft/bluestein.c:55` similarly pre-scales the convolution kernel. Its inverse-by-forward identity reuses a child plan. These strategies can save a separate scale traversal, but scaling tiny kernels/inputs changes underflow behavior. Rust's existing residual-preserving fallbacks must remain authoritative for difficult inputs. FFTW's generated-codelet/planner ecosystem is not a proportionate replacement for this small Rust crate. The relevant lesson is to isolate a bounded transform shape and measure its composition. No generated codelet tree was exhaustively read, no code was generated, and no claim is made about which codelet the system FFTW 3.3.10 benchmark selected. FFTW files inspected carry GPL-2.0-or-later notices; copying them into the BSD standalone crate would not preserve that crate's current licensing arrangement without additional decisions.

## 9. Correctness and numerical risks

### 9.1 Source-proven defects

#### C01 — P1: workspace compatibility omits the analysis work-prefix capacity

**Verified source finding.** `src/workspace.rs:522,571` allocates `order_work_prefix` with `mmax+2` elements. `src/transform.rs:2188` validates many workspace dimensions but not this vector. `src/plan.rs:437` calls that validator before reserving execution workers; it does not resize the prefix. Parallel scalar analysis slices it at `transform.rs:1472`, and spin analysis at `spin_transform.rs:793`.

**Source-based reproducer.** Construct two f64 scalar GL plans with identical grid `512×1024`, L511 and spin zero, but mmax 0 and 511. Construct the low-mmax workspace in a one-thread pool. Pass it to the full-mmax plan's `prepare_workspace(...,2)`, then run full-mmax analysis in a two-thread pool with correctly sized map and output. All checked geometry/recurrence lengths are the same, preparation adds workers, but the prefix remains length two. The parallel path requests `[..=512]`, requiring 513 entries. The slice panics. No numerical input is needed to trigger the length mismatch; zero maps suffice. A direct high-capacity low-mmax workspace provides another route if its worker allocation meets the existing checks.

**Impact.** Public Result-returning operations can panic for an incompatible workspace instead of returning the documented compatibility error. This is safe-Rust bounds checking, not memory corruption. It does not affect a correctly matched plan/workspace pair.

**Difficulty/risk.** Small implementation; low numerical risk. Prefer validating all plan-dependent workspace fields before execution. Either reject this mismatch or deliberately grow every required field under a documented compatibility policy. Merely clamping mmax would silently change the transform and is unacceptable.

**Minimal experiment and numerical validation.** Add the exact public sequence above, the reverse mmax direction, spin-1 L255, q1/q2/q3, and no-default-feature controls. Check a clean error before output mutation or correct results after explicit repair. Also check the existing pool-migration and field-batching cases.

**Acceptance.** No panic; no silent truncation; correctly sized operations preserve outputs and adjoint identities. **Rejection.** A fix that only avoids the slice by reducing order count, or that silently expands deliberately serial execution capacity.

#### C02 — P1: root real-FFT reconstruction and synthesis scaling can overflow prematurely

**Verified source finding.** `src/fft.rs:119` forms `(z+mirror)*0.5`; inverse reconstruction at `:177` forms `(a+b)*0.5`. These additions can overflow before multiplication reduces the result. Independently, synthesis multiplies its Fourier amplitude by nphi (`src/transform.rs:1065` onward; analogous spin scaling in `spin_transform.rs:1430`) so that the normalized inverse returns an unnormalized Fourier series.

**Exact adapter demonstrations.** At N=2, forward input `[1e308,0]` packs into a length-one transform. For k=0, z and mirror are both `1e308+0i`; their sum overflows, although both correct real DFT bins equal `1e308`. For inverse input bins `[1e308+0i,1e308+0i]`, the same addition overflows although the correct normalized result is `[1e308,0]`. These examples do not depend on a long FFT's rounding behavior.

**Public synthesis demonstration.** Use GL `ntheta=1,nphi=8`, L=mmax=spin=0, phi0=0, and `a00=1e308`. The mathematical map is the finite constant `a00/sqrt(4π)≈2.82095e307`. The latitude amplitude is representable; multiplying it by eight overflows before the inverse FFT cancels that scale. Subsequent reconstruction cannot recover the finite value. Fixing only the half-sum expression does not fix this second failure.

**External comparison.** DUCC's synthesis ring FFT can consume unscaled Fourier amplitudes; its GL analysis can apply weight/N in ring processing (`sht.cc` ring helper and `analysis_2d`). Admiral's half-sum formula is not a counterexample to the defect; it has the same potentially overflowing expression.

**Impact.** Finite valid coefficients or spectra can produce nonfinite output despite a finite mathematical result. The adapter is shared by precisions and operators. Ordinary atmospheric inputs are far below these bounds, so this is not an explanation for their performance gap.

**Difficulty/risk.** Medium. An internal inverse convention that accepts Fourier-series amplitudes can eliminate unnecessary N multiplication, but all S/A/S*/A* scales and Nyquist factors must be derived together. Half-before-add reduces overflow but can lose subnormal contributions; use a bounded ordinary path and a carefully scaled fallback rather than an unconditional reassociation.

**Minimal experiment and validation.** First add the two length-two analytic cases and the public monopole case. Cover f32 analogues near its maximum, odd/even lengths, DC/Nyquist, tiny values, cancellation, and all four operators. Compare ordinary helper outputs against direct complex FFTs and direct Hermitian sums. Use analytically scaled expectations for extremes so the reference does not overflow first.

**Acceptance.** The exact cases remain finite and accurate; established normalization, phase, endpoint, and adjoint tests retain their thresholds. **Rejection.** Merely rejecting all large finite coefficients, changing public normalization, or fixing synthesis while altering analysis/adjoints.

### 9.2 Verified guard gaps; complete numerical manifestations still require execution

#### C03 — P1: original-coefficient fallback is incomplete across spin synthesis dispatch

**Verified source finding.** Ordinary spin-1 orchestration checks both scalar-converted rows for finiteness and falls back to the original spin recurrence (`spin_transform.rs:1450–1513`). The hemisphere implementation converts or copies rows at `spin_transform/hemisphere.rs:64–88` without that check. Failed paired preparation falls back to a scalar consumer of the already-converted rows. Spin-2 conversion at `spin_transform.rs:2072` similarly has no original-E/B finiteness escape; its later visitor at `:2224` recomputes transformed pairs from those rows. There are two distinct safety boundaries: E/B-to-scalar conversion, and scalar-to-paired coefficient preparation. Rejecting the second does not repair nonfinite data created by the first. In spin 2, the fallback also recomputes paired expressions directly, so a pair-preparation failure must not be assumed to select an unpaired original-coefficient reference.

**Algebraic evidence.** `spin_fast.rs:221–225` forms `-E ± iB` before later scaling. For l=2,m=1, E=`1e308` and B=`i*1e308`, one branch forms `-2e308`. Subsequent multiplication/subtraction can propagate infinity or NaN. Thus valid finite E/B values can overflow the conversion itself; this is not a speculative CPU effect. Whether a particular public case's every output should be finite must be assessed separately from that intermediate overflow.

**Precise future reproductions.**

- Reuse the existing conversion-overflow construction in `src/spin_transform/tests.rs:2314` (E at l2,m1=`1.44e308`; B at l1,m1=`-i*1.44e308`) at full L319 and L511, minimum GL sampling, q4/q8, to enter the hemisphere route. Compare pre-FFT spectra with the forced general-spin path component by component, including finite reference positions. Test q1 as a control and both cached/uncached conversion.
- For spin 2, use L500 or L511 with only l2,m1 E/B as above. Compare eligible interior-ring spectra with the general recurrence and an independently evaluated spin-2 harmonic; do this **before Fourier N scaling** to isolate C03 from C02. Normalize the reference by `1e308` during analytic evaluation. Repeat with one nonzero E channel to separate conversion overflow from paired-coefficient overflow.
- Force a failure after earlier SIMD lanes/degree pairs have staged contributions, then confirm no partial contribution is committed twice on fallback. Include subnormal coefficients and opposite-sign cancellation.

**Impact and evidence limit.** Guard coverage is definitively incomplete. The specified full public transform differences were not executed in this review and are not reported as observed failures. The existing tests predominantly exercise the ordinary spin-1 driver at low L; they do not prove the specialized routes safe.

**Difficulty/risk.** Medium, with high numerical sensitivity. Route every unsafe conversion/pair to a reference that still has original coefficients and restored recurrence state. Preserve two-channel atomicity. Avoid adding a separate full coefficient scan per latitude vector when preparation can record a per-order safety result.

**Acceptance.** Specialized and forced-general results agree at finite reference positions, preserve legitimate infinities consistently where the mathematical result is unrepresentable, and do not double-commit staged output. Ordinary route selection and whole-call latency remain within declared controls. **Rejection.** A fallback that consumes already nonfinite converted rows or merely disables one threshold while leaving another entry exposed.

### 9.3 Other numerical and robustness observations

**Verified source finding.** Scalar transformed-coefficient validation, staged fused analysis, scalar/spin product safety tests, and residual-preserving inverse FFT paths are real protections. They must not be removed to improve route occupancy. `longitude_phase` avoids forming an overflowing m*phi0 directly in the large-angle fallback; large finite phi0 is already tested. Scalar tables exclude unsupported Nyquist layouts and unsafe amplitudes. These are not newly discovered defects.

**Previously measured result.** The FFT M5 report records a p241 Bluestein limitation for near-maximum cancellation combined with tiny terms at both candidate padding lengths. `crates/rustsht-fft/examples/p241_cancellation_diagnostic.rs:8` retains a concrete diagnostic: tiny DC/index-2 components and opposite near-maximum index-1/index-(N−1) components. That is a known exceptional-range limitation, not evidence that every finite input is handled exactly. Re-run the diagnostic and compare scaled analytic x0 before broadening integration claims; no new result is asserted here.

**Reasoned hypothesis.** Range gates based on minimum nonzero magnitude may send smooth SHT spectra to slower Rader fallbacks much more often than the benchmark's deterministic random-like rows. This is a performance/data-distribution concern, not a correctness bug. Count actual route selections before changing thresholds. Input/buffer validation generally precedes transform execution, and checked sizing covers principal allocation products. Safe indexing catches inconsistencies rather than causing memory corruption. No additional demonstrated integer-wrap, alignment, parallel race, hidden shared scratch, or unsupported-intrinsic defect was found. This is a static finding, not a substitute for runtime sanitization on non-AVX2 machines. Result-returning operations also should not be assumed transactional for every possible late numerical error unless their docs promise it; test output state explicitly where a new fallback/error can occur.

## 10. Single-thread performance analysis

### 10.1 What the measured gap does and does not establish

The atmospheric report's matched Python comparison uses reusable Rust plans and DUCC 0.41.0, f64, GL, phi0=0.37, full mmax, and the stated longitude sizes. That extension was built from `cde39cbcbfb4ea3eded307f6bbcc6c259c90681e`; the separate native source revision is recorded in section 2.3. The native study uses Rust 1.99.0/LLVM 23.1.1 on an x86-64 CPU (host CPU topology omitted, AVX2/FMA, no AVX-512), portable compilation and runtime dispatch. Timings are historical medians on a shared host.

| Scalar STANDARD workload | Rust S / DUCC S, q1 (ms) | Rust S / DUCC S, q8 (ms) | Rust A / DUCC A, q1 (ms) | Rust A / DUCC A, q8 (ms) |
|---|---:|---:|---:|---:|
| T720, nphi1458 | 99.2 / 30.2 | 37.4 / 3.5 | 41.8 / 28.6 | 11.6 / 3.7 |
| T1023, nphi2048 | 307.0 / 74.5 | 77.5 / 8.9 | 114.6 / 71.5 | 21.3 / 9.0 |
| T1279, nphi2560 | 652.6 / 136.8 | 296.6 / 17.2 | 230.7 / 133.5 | 42.8 / 17.0 |
| T2047, nphi4096 | 3449.3 / 578.8 | 845.6 / 113.6 | 1193.7 / 555.5 | 237.2 / 81.3 |

The analysis gap is smaller than synthesis but remains significant. The high-L spin-1 gap also persists: T2047 q1 S/A was 4343/1906.9 ms versus DUCC 1224.1/1137.6; q8 was 1072/393.1 versus 221.1/186.9. A scalar-only success must not be presented as vector parity. The profile captures are separate runs, so their phase totals should not be numerically substituted into the table above. They nevertheless locate the dominant phase:

| q8 scalar synthesis profile | Legendre fraction | Inverse FFT/writeback fraction |
|---|---:|---:|
| T720 | 92.6% | 6.7% |
| T1023 | 91.5% | 7.4% |
| T1279 | 96.6% | 2.9% |
| T2047 | 96.8% | 2.7% |

At T2047, eliminating the entire 2.7% FFT/writeback phase would yield at most `1/(1−0.027)=1.028` times speedup under that profile. Even eliminating all non-Legendre work leaves the overwhelming gap. This is an Amdahl bound from a measured fraction, not a forecast for a new backend.

### 10.2 P01 — P1: measure and increase ordinary fused synthesis coverage

**Verified source finding.** `scalar_paired.rs:247,320` separates ordinary/scaled lanes; fewer than four ordinary lanes or inactive-lane patterns can decline this path. The scaled subset continues through the visitor. `transform.rs` high-L staged-transition/compaction dispatch and `scalar_paired.rs` transition helpers add further gates. `scalar_paired.rs:2855` remains a per-pair visitor with normalization state. DUCC's transition/normal split is described in section 6.3.

**Reasoned hypothesis — Requires benchmark.** A substantial part of high-L work may execute outside the efficient ordinary kernel, including degrees after a scaled prefix has become ordinary. This could explain a large gap despite good isolated fused arithmetic. The actual fraction is unknown; no percentage of runtime is assigned to it here.

**Impact/difficulty/risk.** Potentially high benefit across scalar and converted spin synthesis; medium-to-large implementation. Numerical risk is high if transition, underflow, coefficient safety, or fallback state is simplified incorrectly.

**Minimal experiment.** Add opt-in counters to a diagnostic build, not atomics in the release inner loop: per worker/order count active/inactive lanes, initial scale class, degree of transition, ordinary fused pairs, scaled-visitor pairs, compacted fallback pairs, coefficient-safety rejection, and replayed pairs. Time preparation, transition, ordinary accumulation, and writes separately with a low-overhead model and whole-call control. Start T511/1023 q1 with selected m bins, then full T2047. If expensive fallback occupancy is low, reject it as the main explanation before rewriting.

**Candidate.** Give compacted active lanes another eligible fused/transition entry; let a scaled subset transition once into ordinary accumulation; add a scalar-only transition consumer; prepare coefficient safety once per reusable order tile. Keep the existing exact scaled path for unsafe coefficients and polar cases.

**Numerical validation.** All C03-style restoration tests, huge/subnormal coefficients, late transitions, m≈L, active-cutoff boundaries, odd tails/equator, all operators sharing the kernel, and pinned DUCC plus analytic fields.

**Acceptance/rejection.** Require route counters to show the intended work transfer and a reproducible whole-transform gain. Reject basis-only thresholds that discard representable huge-coefficient contributions, or a kernel win canceled by repeated preparation/replay. If this is the principal parity milestone, require more than a few percent at the primary high-L cells or explain quantitatively which remaining phase prevents it.

### 10.3 P02 — P1: evaluate parity-aware fixed-degree SIMD panels

**Verified source finding.** `scalar_paired.rs:1982` is vector-outer fused synthesis; `:2409` provides an existing hemisphere kernel used in restricted circumstances. DUCC ordinary scalar kernels/extraction (`sht_inner_loop.h` scalar kernels and `:1432`) and SHTns synthesis templates maintain separate parity accumulators. The six-FMA comparison in section 6.1 describes source arithmetic, not retired instructions.

**Reasoned hypothesis — Requires benchmark.** A panel with several independent latitude-vector chains and even/odd accumulators can produce both hemispheres, share coefficient loads, and reduce recurrence dependency stalls. It can also spill more registers, increase stores between panels, or perform extra work for mixed range classes. The prior failed hemisphere experiment makes the net outcome uncertain.

**Impact/difficulty/risk.** High potential for the largest scalar-S gap; large kernel work and medium/high numerical risk. No source evidence establishes an optimal block size or a predicted speedup.

**Minimal experiment.** Isolate one order with already-safe coefficients and ordinary active lanes. Compare current vector-outer execution with degree panels of 2/4/8 degrees over 1/2/4 independent SIMD vectors; keep the outer storage block at 64 initially. Test 128/256 storage only after selecting a register tile. Measure useful north-plus-south output per unit time, including final parity reconstruction, coefficient preparation, and tails. Inspect compiler output for spills and bounds-check branches in the future implementation session; none was generated here.

**Numerical validation.** Compare both hemispheres with the original scaled reference, including parity sign for odd m, singleton equator, nonzero phi0, m=0, reduced mmax, poles, cancellation and extremes. Extend first to scalar S/S*, then to shared spin channels and CC/F1 only after their operator tests pass.

**Acceptance/rejection.** Require a faster complete projection at representative m values and a whole-transform gain at q1 and q8, not just lower flop count. Retain the existing kernel where register pressure or small shapes lose. Reject blanket FMA substitution or a simple larger array without a measured mechanism.

### 10.4 P03 — P2, enabling P01/P02: amortize factor and coefficient preparation

**Verified source finding.** Latitude-owned `transform.rs::synthesize_ring_group` and spin orchestration prepare epsilon/paired factors for each group. `scalar_paired.rs:25,63` separates factors and transformed coefficients; `spin_fast.rs` supplies conversions. DUCC prepares an order per worker job, while SHTns retains configuration-dependent factors (`sht_legendre.c:400`). Rust's accepted spin-1 cache already removes some duplication through L720.

**Reasoned hypothesis — Requires benchmark.** Reusing factor/converted-coefficient tiles can remove O(number-of-groups × L²) setup and repeated safety scans. It cannot by itself remove the leading O(L³) projection cost. Measure setup fraction before promising a large gain.

**Impact/difficulty/risk.** Medium or enabling benefit; medium implementation and memory risk. A full set of recurrence factors is O(L²), much smaller than a basis table but still tens of MiB at T2047 depending on packing. A bounded per-order or few-order tile avoids changing the 34 MiB basis policy or materializing all coefficients twice.

**Minimal experiment.** Record factor/conversion time separately. Prepare 1/4/8-order tiles once, including safety flags, and reuse them across latitude jobs. Start with scalar q1 and q8; separately assess spin 1 above L720 and spin 2. Report bytes, setup, cache misses if available, and whole-call latency. Existing parallel spin-cache and failed direct-cache-read experiments are controls.

**Validation and gate.** Preserve transformed coefficient bits where possible; compare near-overflow/subnormal safety decisions and original-coefficient fallback. Accept only if saved preparation exceeds extra traffic/synchronization and memory remains bounded. Reject unconditional full-basis precomputation or a cache that delays ordinary execution more than it saves.

### 10.5 Lower-level explanations that remain unproven

Bounds checks, abstraction overhead, instruction-cache pressure, recurrence latency, memory bandwidth and cache misses are plausible contributors. Source alone cannot establish that LLVM retains a given check or spills a particular array. The reports include instruction-reference profiles and some phase timings, but hardware performance counters were unavailable on the benchmark host. Do not claim “memory-bound,” “FMA-bound,” or “instruction-cache regression” as established causes. P01/P02 create small enough experiments to distinguish these possibilities.

## 11. Parallel scaling and scheduling

### 11.1 What is already fixed

The active-ring prefix weights an order approximately by `active_rings(m)*(L−m+1)` and is built once using u128 arithmetic (`transform.rs:187`). Recursive splits allocate work in proportion to child worker capacities, including odd thread counts. These changes remove the earlier repeated scanning and equal-half/unequal-capacity defect. The small table reducer still has separate scheduling choices; broadening that observation into “analysis uses equal halves” would be wrong. A fixed set of coarse leaves remains. Rayon can schedule those leaves, but once every worker is occupied, a long leaf's remaining orders are not automatically exposed as stealable work. Estimated equal work also ignores scaled-prefix lengths, compacted-vector efficiency, and fallback/replay costs. Reported leaf estimates were within about 1%, while elapsed leaves differed by roughly 1.6–2.5 times and useful-capacity fractions were about 0.59–0.72. Those observations do not distinguish runtime scheduling, host interference, or order-dependent arithmetic costs by themselves.

### 11.2 P04 — P1 after P01/P02: bounded order/latitude ownership with measured costs

**Verified source finding.** Current synthesis owns row groups; current analysis owns static order ranges (`transform.rs:1701`, `spin_transform.rs:3892`). DUCC dynamic order jobs and SHTns dynamic order/batch jobs expose finer units. The rejected Rust order-owned prototype used a different layout/partition combination from a parity-aware tiled kernel.

**Previously measured result.** Against baseline `8ccc0d5`, order-owned scalar S changed T511 q1 from 42.852 to 43.847 ms and q8 from 11.901 to 15.788 ms; T1023 q1 from 326.079 to 335.247 and q8 from 107.469 to 109.390. The report has no isolated phase or RSS attribution. It rules out accepting that prototype, not every possible order-oriented design.

**Reasoned hypothesis — Requires benchmark.** A bounded tile can prepare each order once, evaluate north/south pairs using P01/P02, and make unfinished tiles stealable while retaining contiguous private writes. An order-major Fourier tile can feed a blocked transpose or FFT gather. The gather/transpose cost must be paid exactly once and included in total latency. Dynamic analysis could use immutable prefix ranges with smaller disjoint coefficient slices, but not per-degree tasks or shared accumulation.

**Impact/difficulty/risk.** Potentially high for parallel synthesis; high implementation and layout complexity. Medium numerical risk from changed accumulation and transpose/phase handling. More tasks can worsen cache reuse; previous leaf overpartitioning already lost.

**Minimal experiment.** First keep existing ownership and use route-aware cost diagnostics to predict actual leaf work. Then compare only two designs: (a) current row ownership plus reusable factor tiles; (b) weighted order tiles with parity-aware projection and a bounded blocked output conversion. Use q1/3/6/8/16, T511/1023/1279 and bounded T2047, full and quarter mmax. Fix tile storage before timing, and include FFT gather, transpose, clear, and output writes. A short dynamic queue should expose at least two tasks per worker without creating a full extra spectrum per task.

**Numerical validation.** All scalar/spin adjoints, exact and overresolved GL, CC/F1 composition, phase, Nyquist, workspace reuse, and separate-worker ownership checks. Compare q1 to qN with the established tolerances rather than demanding unsupported bitwise determinism.

**Acceptance/rejection.** Accept only if whole S approaches reference latency at both one and multiple threads, total scratch remains bounded, and independent-field throughput is preserved. Reject another equal-order/full-transpose retry, a q8 gain bought by a material q1 regression, or a claimed scheduler fix supported only by equal estimated costs.

### 11.3 Serial stages and external concurrency

Inverse longitude FFTs/writeback are still serial in scalar and spin synthesis (`transform.rs:650`, `spin_transform.rs:1215`). Large forward analysis FFTs already have parallel paths. CC/F1 resampling remains serial over orders. Parallelizing these is mechanically plausible but should follow their measured fraction: at T1023 analysis on the profile's odd longitude grid, FFT occupied roughly 23–24%, whereas T2047 even-grid scalar synthesis FFT/writeback was 2.7%. These are different grids and operations; fractions must not be transferred between them. A plan's explicit capacity and the executing pool determine internal parallelism. An outer batch should normally use capacity-one workspaces and a bounded number of concurrent fields. Increasing internal workers for each outer task risks oversubscription and multiplied scratch. The existing capacity API is the mechanism to preserve, not an obstacle to remove.

**Previously measured result.** The atmospheric q16 screen reduced scalar S from about 306 to 52.3 ms at T1023 and 643 to 195 ms at T1279 relative to its own q1 runs. Spin-1 S was 291→56.4 and 574→194 ms. Concurrent process-placement details were retained only in local diagnostics. These are noisy scaling observations, not evidence that sixteen threads remove the one-thread arithmetic gap or establish reference parity. They justify keeping q16 and topology/affinity in the next controlled campaign.

## 12. f32 versus f64 analysis

### 12.1 Structural and measured differences

f32 scalar recurrence (`scalar_f32.rs:86,204`) performs per-degree visitation and scaled contribution logic. Its exponent scale and normal-range envelope differ from f64. The T1023/T1279 SIMD normal-range skip avoids some repeated scalar normalization scans but does not create a paired fused projection. f32 spin computes positive and negative sequences and retains per-degree normalization/reduction structure (`spin_f32.rs`); the accepted analysis SIMD reducer improves that path without giving it the f64 scalar-conversion architecture.

**Previously measured result.** Atmospheric native T1023 f32 scalar S was about 4064/675 ms at q1/q8 and A about 2058/367 ms. Spin-1 S was about 8353/1155 ms and A about 6872/1287 ms. The corresponding f64 native cells were orders of magnitude closer to the references. Inputs/rounding and exact measurement scopes differ, so these are architecture-screening evidence rather than a controlled type-only speed ratio. The latest scalar f32 skip improved T1023 by about 4.39%/2.95% for S and 4.49%/1.10% for A at q1/q8, with bitwise baseline agreement. T1279 improved by about 4.13%/1.36% and 6.50%/3.42%. These accepted improvements do not close the architectural gap. The T2047 screen was inconclusive and removed; it must not be described as enabled. Full T1023 comparisons against DUCC 0.41.0 report relative-L2 errors `1.67e-5` to `4.87e-5`. T1279 scalar S/A errors were `3.77e-5`/`2.05e-5`. The milestone explicitly says no T1023 f32 acceptance threshold is established. Bitwise agreement with an older f32 path proves preservation, not sufficient absolute accuracy for every application.

### 12.2 P05 — P1: prototype an f32 interface with f64 projection

**Verified source finding.** f64 paired/fused kernels are unavailable through current f32 dispatch (`legendre/mod.rs` precision implementations; `scalar_f32.rs:86`; `spin_f32.rs`). DUCC uses double SHT inner vectors (`sht_inner_loop.h:252`) and double ring FFT scratch/plans. This is a concrete alternative already used by the numerical reference.

**Reasoned hypothesis — Requires benchmark.** Converting O(L²) coefficients/maps while reusing optimized f64 O(L³) projection can outperform the native f32 recurrence despite half as many SIMD lanes and larger scratch. It may also reduce recurrence error. Existing measurements make this a high-priority experiment rather than a speculative long-term rewrite.

**Design alternatives.** Start with a diagnostic bridge to the existing f64 plan to establish an upper bound including full conversion. If promising, use f64 worker coefficient/latitude tiles and either f64 Fourier storage with final downcast or f32 FFT plus explicitly widened projection inputs. Compare both numerical/dataflow choices; the latter does not automatically match DUCC. Reuse storage across calls and avoid a full f64 map copy when a row/tile downcast suffices. Preserve the f32 public packed format and output dtype.

**Impact/difficulty/risk.** Potentially large for scalar and spin 1; medium prototype, substantial production dispatch/memory work. All finite f32 values widen exactly, but output downcast can overflow, underflow, or change signed-zero/subnormal behavior. More accurate intermediates do not guarantee every cancellation-sensitive result is closer after downcast. Spin conversion, derivatives and adjoints require explicit checks.

**Minimal experiment.** T127/511/1023 scalar and spin-1 STANDARD, q1/q8, minimum odd/even GL. Measure conversion, projection, FFT, output conversion, setup, and peak RSS separately. Compare native f32, widened bridge, native f64 with equivalent rounded inputs, and DUCC float/double outputs. Then test T1279/T2047 and CC/F1 only if the first screen wins.

**Numerical validation.** Full arrays, independent low-degree fields and derivatives, S/S*/A/A* identities in a suitably accurate test inner product, m=0/m=L/reduced mmax, high-m polar values, f32 maximum finite/subnormal inputs, cancellation and nonzero phi0. Define a high-L f32 error budget before evaluating new results; do not turn the existing observed error range into an automatic tolerance.

**Acceptance/rejection.** Require a substantial whole-transform gain after conversion and a documented memory bound, with accuracy at least meeting the predeclared f32 contract. Reject a fast bridge that multiplies live batch memory without bound, changes public types, or relies on runtime allocation per order. If it wins, prefer one shared optimized arithmetic family over separately porting every f64 microkernel to f32 without evidence.

## 13. Batching, precomputation and alternative algorithms

### 13.1 A01 — P2: bounded independent-field execution before matrix methods

**Previously measured result.** The atmospheric T720 study found eight independent fields using capacity-one workspaces and an eight-worker outer pool achieved about 2.7–2.8 times the throughput of processing fields with the single-field q8 configuration. Increasing to 32 fields did not improve synthesis further. Keeping 32 workspaces retained roughly 520 MiB of workspace payload, with total process memory around 1.5–2 GiB including data. This is evidence for bounded concurrency, not a need for 32 live workspaces.

**Verified source finding.** `plan.rs:437` and separate `ShtWorkspace` ownership already permit this. `examples/m5_batch_bench.rs:145` compares sequential and outer-parallel schedules. Python's one mutex-held workspace (`python/src/lib.rs:30,663`) does not expose that policy.

**Reasoned hypothesis — Requires benchmark.** A reusable pool of at most q capacity-one workspaces, processing many fields in waves, can retain the observed throughput with bounded scratch. Later, an order tile can reuse basis/factors across a small field block. SHTns's order/batch loop and ecTrans's field-column matrix operations support the reuse mechanism, but not a promised speedup in Rust.

**Impact/difficulty/risk.** High for multi-field workloads, no direct single-field latency gain. Small/medium core helper or example, medium Python API work; low numerical risk if each field retains independent accumulation. A basis-sharing field kernel is a separate, higher-risk experiment.

**Minimal experiment.** B=1/2/4/8/32 at fixed total threads and memory cap, with only min(B,q) live workspaces. Compare outer-only, inner-only and a bounded hybrid. Time allocation outside warm execution and also report cold total. Include Python separate-plan, same-plan and proposed batch calls.

**Validation/gate.** Every field must match its independent call and adjoints. Accept if throughput gains survive bounded memory and no nested oversubscription. Reject a batch API that just allocates a full workspace per queued field or claims faster single-field execution from aggregate throughput.

### 13.2 Alternatives and their rejection experiments

| Strategy | Expected workload and mechanism | Setup / memory | Numerical and implementation risk | Smallest useful rejection experiment |
|---|---|---|---|---|
| Different paired recurrence formulation | Ordinary scalar/spin-converted projection; move cosine/parity combination out of the pair loop | O(L) order scratch; little permanent setup | Same mathematics can have different cancellation and range; moderate kernel complexity | One-order north/south projection including preparation, extreme coefficients and tails; reject if arithmetic savings do not survive complete projection |
| Fixed degree panels / order-latitude tiles | Single-field high-L latency | Bounded worker state and coefficient tiles | Spills, range transitions, output gather; medium/high | P01/P02 then a two-layout whole-call comparison |
| Immutable recurrence-factor cache | Repeated same-plan execution | O(L²) factors, not O(L³) basis; setup amortized over calls | Extra cache traffic, plan memory, stale safety flags if input-dependent data is mixed in | Measure preparation fraction and 1/4/8-order bounded tiles before full caching |
| Existing compact basis table | Small/medium repeated GL | Already capped at 34 MiB, L63–255, selected shapes | Existing amplitude/dispatch guards; current policy already tuned | Recheck only changed cells after a kernel change; do not broadly expand the table |
| Streamed batched basis tile | Several fields sharing a grid; generate a latitude-degree tile once and apply to B columns | O(tile×B) scratch; no permanent full basis | Changed summation, packing cost, field-tile selection; medium/high | B=4/8/16 on T511/T1023, include basis generation and packing; reject if no gain over bounded independent fields |
| Dense precomputed GEMM | Many repeated fields/calls with a large memory budget | Full north basis approximately 0.70 GiB at T720, 2.00 GiB at T1023, 3.91 GiB at T1279, 16.0 GiB at T2047, before spin/multiple copies | Conflicts with current bounded-memory policy; setup and BLAS/dependency policy; high | One order/parity block with actual field count and setup amortization; reject if required reuse exceeds workload or memory limit |
| Butterfly/interpolative compression | Repeated very high-L plans, possibly batches | Rank-dependent factors and expensive construction; dense setup may still be required | Error tolerance, rank growth near transitions, transpose consistency and extreme values; very high | Compress one representative low/mid/high-order parity block at the required error, measure rank, application, setup and transient memory before a full transform |
| Hybrid selector | Different precision/grid/spin/batch regimes | Small deterministic metadata once validated candidates exist | Combinatorial dispatch/testing burden | Admit one narrowly defined measured region at a time; reject selectors that only fit one noisy run |

The dense memory figures follow `ceil(ntheta/2)*(L+1)*(L+2)/2*8` bytes for a full scalar f64 north table at minimum GL sampling. They are formula estimates, not measured ecTrans allocations. A small recurrence-factor cache must not be conflated with this basis table.

### 13.3 ecTrans evidence and limits

The local ecTrans source confirms multiple fields as matrix columns: `src/trans/cpu/internal/leinv_mod.F90:120` selects GEMM or butterfly application by order/parity block size; analysis uses the corresponding transpose. `suleg_mod.F90:182,219` sets compression precision `1e-7` and an initial column width 64. `algor/butterfly_alg_mod.F90:68` takes an already formed dense matrix and builds a hierarchy of factors. That source supports a setup/reuse trade, not an immediate replacement. Its `1e-7` parameter is insufficient evidence for Rust's f64 error targets, and dense construction remains a transient-memory cost even if factors reduce persistent storage. A future candidate must independently derive the algorithm, choose error/rank budgets, and implement both weighted operators and their adjoints. Spin 1/2 require validated scalar conversion or their own basis factorization. ecTrans carries Apache-2.0 notices; no source adaptation is proposed here. The current direct recurrence has not demonstrably reached diminishing returns: parity, ordinary-route coverage and f32 architecture remain substantial differences from fast references. Compression should be a separate research prototype after those differences are measured, not a substitute for explaining them.

### 13.4 A02 — P3: gate matrix/compressed alternatives on a bounded block experiment

**Verified source finding.** `src/legendre/scalar_table.rs:27,244` limits current precomputed bases to a measured small-band region and bounded storage. The ecTrans locations in section 13.3 demonstrate dense multi-field application and compression constructed from dense input. Neither supplies a RustSHT implementation or its error budget.

**Reasoned hypothesis — Requires benchmark.** A streamed field tile may improve arithmetic intensity without a full basis allocation; a compressed parity block may reduce repeated application cost at high L. These are distinct alternatives and should compete with the improved direct recurrence, not an obsolete baseline.

**Impact/difficulty/risk.** Potential repeated-plan/batch benefit; high complexity for streamed application and very high complexity for compression. Setup, temporary dense memory, rank growth, approximation error and adjoint consistency are the main risks. Section 13.2 states each alternative's storage and workload.

**Minimal experiment.** For streamed application, generate one low/mid/high-m tile at T1023 and apply it to B=4/8/16 fields, counting generation and packing. For compression, construct one representative parity block at the required f64 tolerance and measure rank, forward/transpose application, construction time, and peak temporary bytes. No full-grid implementation is needed to reject either hypothesis.

**Numerical validation.** Compare block outputs with the scaled recurrence for random, isolated and cancellation-sensitive coefficients; verify the block transpose identity. Before whole-transform admission, require pinned DUCC, analytic harmonics/derivatives, all weighted adjoints and spin conversion checks. Compression error must have its own budget rather than consume the entire transform tolerance unnoticed.

**Acceptance/rejection.** Accept further development only if a bounded-memory block beats the direct/bounded-batch baseline after amortization over the intended number of calls. Reject if required reuse exceeds the workload, compression ranks approach dense dimensions, setup exceeds the memory cap, or forward/transpose error budgets fail. Do not raise the existing table cap as an incidental implementation detail.

## 14. FFT integration assessment

### 14.1 Present integration and practical ceiling

**Verified source finding.** Root `Cargo.toml` depends on RustFFT and contains only `python` as an explicit workspace member. `src/fft.rs` wraps RustFFT; `src/grid.rs` and `src/meridional.rs:42` also construct RustFFT plans. No root execution path calls the standalone crate. Improving its radix 19/103/240 work currently changes no RustSHT transform timing. Root policy still specifies RustFFT as the sole production engine. A future integration experiment needs a concrete recorded scope decision before changing that policy. It does not require introducing a generic public backend trait. A private adapter experiment can compare identical normalization, buffers, geometry and inputs while retaining the production default. The accepted M32/M33 cross-library matrix is mixed:

| f64 operation | Accepted RustSHT-FFT latency | RustFFT / RustSHT-FFT | Native DUCC / RustSHT-FFT |
|---|---:|---:|---:|
| C2C forward N722 | 7.278 µs | 0.933 | 0.500 |
| C2C inverse N722 | 9.344 µs | 0.691 | 0.398 |
| R2C N723 | 10.119 µs | 0.466 | 1.156 |
| C2R N723 | 10.933 µs | 0.435 | 1.066 |
| R2C N2049 | 31.631 µs | 1.172 | 1.201 |
| C2R N2049 | 35.178 µs | 1.048 | 1.091 |
| R2C N4097 | 84.475 µs | 0.724 | 0.891 |
| C2R N4097 | 99.171 µs | 0.613 | 0.745 |

A ratio below one means the reference is faster. In particular, N4097 standalone real FFT is **slower**, not faster, than RustFFT. The current N723 path is roughly 2.15–2.30 times slower than RustFFT despite beating the measured native DUCC FFT. Choosing it because of the DUCC comparison would regress the existing root backend for that isolated workload.

### 14.2 F01 — P2: isolate the N240 Rader child instead of expanding general dispatch

**Previously measured result.** M34's direct-DFT-validated phase model reports N723 R2C total Rader pass 8.372 µs, with 2.798 forward and 3.941 inverse; N4097 R2C pass 47.517 µs, with 15.713 and 22.411. C2R is similar. The two FFTs account for about 80% of the **modeled Rader pass**, not 80% of the complete real FFT or SHT.

**Verified source finding.** `crates/rustsht-fft/src/rader.rs:51,372,397` constructs and executes the 240-point child for p241. Its factors are `[2,8,3,5]`; the terminal radix-five `(48,1)` pass remains scalar. Existing radix-two/eight/three SIMD must be retained. M34's three shared-dispatch placements were removed. Current source is identical to accepted M32.

**Reasoned hypothesis — Requires benchmark.** A private p241/N240 executor selected once at the Rader boundary can specialize that fixed factor chain, reuse the accepted passes, and isolate a terminal symmetric radix-five kernel from unrelated hot dispatch. A still narrower experiment is a standalone N240 leaf called only by p241. This may reduce the code-layout side effects seen in M34, but isolation is not proof that they disappear.

**Impact/difficulty/risk.** Medium standalone real-FFT benefit; negligible present SHT benefit because it is disconnected. Medium/high implementation risk from duplicated scheduling or normalization logic. Admiral's fixed-shape SIMD helpers and FFTW child-plan/scaling choices are relevant references; neither proves an optimal N240 route.

**Minimal experiment.** Compare isolated N240 forward/inverse, then full N723/N4097 R2C/C2R at actual row counts, then the complete existing 34-cell matrix. Keep generic `SimdKernels` layout and unrelated pass dispatch unchanged in the first candidate. Try safe fused inverse normalization only as a separate change; retain the existing exceptional-input executor. Record code size and actual route, and use repeated fresh processes to detect layout-sensitive regressions.

**Numerical validation.** Direct DFT both directions, odd/even outer groups, unaligned inputs, guard regions, maximum/subnormal/cancellation inputs, exact inverse scale, Rader safety fallback, N2049/682 control and f32 controls. Include the retained p241 diagnostic.

**Acceptance/rejection.** Require full real-call gains and no repeatable unrelated regressions under predeclared gates. Reject any of the already-tested general-dispatch placements without a new isolation mechanism. A successful N240 kernel alone is not acceptance, and instruction counts alone cannot overrule latency regressions.

### 14.3 F02 — P2: connect safe nested execution to existing SIMD before adding more kernels

**Verified source finding.** `real.rs:442,452` calls a generic half-length C2C complexify plan constructed at `:136`; inverse branches use generic scaled execution. Bluestein children similarly call generic execution in `complex.rs:1348,1375`. Public f64 C2C and Rader can use runtime-selected kernels and the scoped N1442 root cache; those facilities are not automatically inherited by generic nested plans.

**Reasoned hypothesis — Requires benchmark.** Reusing safe existing runtime kernels may improve complexified real lengths such as 1452 or 2884 and selected Bluestein children without new butterfly mathematics. A necessary condition is that the nested transform's direction, scale and safety classification match the public optimized contract. It is not enough to replace a function name blindly.

**Impact/difficulty/risk.** Medium, workload-dependent FFT gain; medium implementation, high inverse-range sensitivity. Some roots/cache payload would increase; friendly native real powers of two may be unaffected.

**Minimal experiment.** Add route assertions for generic/nested versus public execution at 1002/1452/2046/2884 and their half lengths, plus p241/683 fallback convolutions. Compare the same child with runtime kernels enabled under the existing safe envelope. Report parent real-call latency and bytes; do not infer reachability from a public C2C microbenchmark.

**Validation/gate.** Preserve prescaling residuals and underflow-sensitive fallback, f32 behavior, endpoint packing and normalization. Accept only parent-call gains with no regression to unsafe inverses or ordinary controls. Reject broad dispatch changes that reproduce M33/M34's unrelated regressions.

### 14.4 F03 — P2: optional integration must use real SHT distributions and all FFT sites

**Verified source finding.** The root adapter's endpoint rejection and Fourier scaling differ from the standalone API, and meridional/grid FFTs bypass that adapter. Current Rader eligibility is amplitude-distribution dependent (`real.rs:1532,1545`). Standalone row benchmarks use deterministic input formulas and serial row loops (`benches/rustfft.rs:814,879`), not captured SHT Fourier distributions.

**Impact/difficulty/risk.** Potentially useful for specific odd-longitude or CC/F1 workloads, but bounded by their phase fractions. Medium integration work with high normalization/adjoint risk.

**Reasoned hypothesis — Requires benchmark.** A restricted backend choice could improve particular SHT shapes after adapter overhead and range fallbacks are included. The current standalone matrix does not establish such a shape-specific whole-SHT win.

**Minimal experiment.** After C02, compare a private adapter at root longitude FFTs with RustFFT for GL odd/even nphi, scalar/spin, S/A/S*/A*, using sparse harmonics, smooth spectra, random coefficients, arbitrary maps and broad dynamic range. Record actual Rader/Bluestein/native/complexify frequency. Separately evaluate meridional lengths and quadrature setup; do not silently switch them as part of longitude timing.

**Numerical validation.** Full maps/coefficients against pinned DUCC and analytic fields; all adjoints; C02 extremes; endpoint rejection; CC/F1 resampling parity and weights; warm reuse and bounded per-worker scratch.

**Acceptance/rejection.** Accept only a documented shape-specific whole-SHT gain with unchanged semantics and an explicit production-policy decision. Reject a default backend replacement based on standalone speedups, particularly at N723/N4097 where the existing RustFFT baseline is faster.

## 15. Prioritized findings

Priority denotes implementation order and consequence, not a promised percentage improvement. P1 correctness work should be independent of performance acceptance; a performance regression requires redesign, not retaining a demonstrated numerical defect. No P0 memory-safety or ordinary-input catastrophic failure was established.

| Rank | ID | Priority | Finding | Evidence confidence | Expected benefit / cost |
|---:|---|---|---|---|---|
| 1 | C01 | P1 | Validate workspace prefix capacity | Source-proven public panic path | Correctness; small fix |
| 2 | C02 | P1 | Remove premature FFT/reconstruction overflow | Exact source arithmetic and analytic finite result | Correctness across shared adapter; medium scope |
| 3 | C03 | P1 | Complete original-coefficient fallback across spin dispatch | Missing guards and conversion overflow proven; public manifestations pending | Exceptional-value correctness; medium scope |
| 4 | P01 | P1 | Measure and increase ordinary fused synthesis coverage | Structure proven; runtime share unknown | Potentially large high-L gain; medium/large |
| 5 | P02 | P1 | Parity-aware panels with multiple independent SIMD chains | Arithmetic mechanism proven; net speed uncertain and earlier negative evidence | Principal scalar-S parity candidate; large |
| 6 | P05 | P1 | f32 public data with f64 projection experiment | Strong structural and historical evidence | Potentially large f32 gain; medium prototype, larger integration |
| 7 | P04 | P1, dependent | Bounded order/latitude ownership after kernel changes | Dynamic reference designs; rejected previous Rust prototype | Potential parallel gain; high cost/risk |
| 8 | P03 | P2, enabling | Reuse bounded factor/coefficient tiles | Repeated preparation proven; fraction unknown | Medium or enabling gain; medium cost |
| 9 | A01 | P2 | Bound live batch workspaces and expose useful concurrency | Existing throughput evidence; API gap proven | High multi-field relevance; medium cost |
| 10 | V01 | P2 | Sustain separate FFT and dispatch validation | Workspace/CI omission proven | Prevent regression; small/medium maintenance |
| 11 | F01 | P2 | Private N240 Rader executor | M34 phase and candidate gains; unrelated regressions constrain design | Standalone FFT gain; medium/high cost |
| 12 | F02 | P2 | Reach existing SIMD from safe nested transforms | Generic child calls proven | Shape-dependent FFT gain; medium cost |
| 13 | F03 | P2 | Bounded optional SHT backend experiment | No integration; mixed FFT comparison | Only after end-to-end evidence; medium/high risk |
| 14 | A02 | P3 | Streamed matrix tiles or compressed basis | Mathematical plausibility and external source only | Batch/reuse research; high or very high cost |

### V01 — P2: validation must follow workspace and dispatch boundaries

**Verified source finding.** `.github/workflows/ci.yml:17` runs root Cargo commands; root `Cargo.toml:20` names the Python member, while the FFT manifest declares a separate workspace. The standalone FFT's 111-test history therefore does not make it part of current root CI. Python exposes only f64, so Python parity tests do not cover f32. The existing high-L checks include ignored opt-in cases and several sample/sparse rather than full-array checks.

**Impact/difficulty/risk.** Maintenance and numerical confidence, not immediate speed. Small workflow/documentation work plus medium dispatch-matrix work. There is no reason to add temporary workflows: extend the existing permanent workflow with explicit standalone-manifest commands if this crate remains supported.

**Minimal experiment.** Inventory route gates against named tests: table/recurrence; scalar paired/unpaired/transition; spin conversion/general/hemisphere; f32 skip on/off; FFT public/nested SIMD/fallback. Add only missing boundary cases during implementation. Confirm the permanent workflow invokes both workspaces, and run portable fallback cases on a machine without AVX2 rather than treating compile success as execution evidence.

**Numerical validation.** Follow section 17, including C01–C03. A route test must prove the intended route was entered as well as compare output. Shared plans require separate concurrent workspaces in Rust; Python same-plan tests should account for its mutex serialization.

**Acceptance/rejection.** Accept complete permanent coverage with explicit ignored/high-cost cases and reference versions. Reject a “workspace green” claim that omits the nested workspace, or a broad support claim extrapolated from one sparse high-L case. No new CI job or test was created during this review.

## 16. Detailed implementation roadmap

Each milestone below produces a concrete decision, a bounded source diff, and a report. They are future work, not actions performed by this review. Use a feature branch from current main for an implementation session. Keep correctness fixes separate from performance experiments and preserve rejected patches/results without leaving them in production dispatch.

### M0 — Close numerical and compatibility gaps

**Files/responsibility:** `src/fft.rs`, `src/transform.rs`, `src/spin_transform.rs`, `src/spin_transform/hemisphere.rs`, `src/plan.rs`, workspace checks, and their existing test modules.

1. Add C01's cross-mmax workspace regression and decide explicit rejection versus deliberate complete adaptation.
2. Add C02's analytic N2 and monopole tests. Derive the Fourier scaling of S/A/S*/A* on paper before changing the inverse convention. Preserve endpoint validation.
3. Add C03 route-specific pre-FFT comparisons. Make conversion safety a per-order result and select a reference with original coefficients. Test paired preparation failures separately from conversion failures.
4. Audit every fallback for staged-output commit and recurrence reset; document the route conditions in `plans/03-api-and-python.md` or an existing numerical plan, not in README.

**Deliverable:** small correctness commits with exact analytic/forced-reference evidence and ordinary-route controls. No tolerance relaxation. **Stop condition:** an unresolved finite-reference failure blocks broadening that specialized route, but does not block independent read-only measurement or a guarded ordinary kernel experiment.

### M1 — Establish a current, matched parity baseline and route accounting

**Files/responsibility:** existing examples, test-only phase probes in `src/workspace.rs`, and report infrastructure. Avoid introducing always-on instrumentation into production kernels.

**Cells:** scalar and spin-1 GL STANDARD at T255/511/720/1023/1279/2047, minimum odd/even and the atmospheric nphi; q1/3/6/8/16 where resources permit. Add spin 2 at T511/1023 and modest CC/F1/adjoint controls. Measure current source against pinned DUCC and the local SHTns revision under matched conventions.

**Record:** complete call, coefficient/factor preparation, scaled transition, ordinary fused work, fallback/replay, Fourier layout conversion, FFT, output copy, setup, workspace bytes and RSS. Route counters must be disjoint enough to avoid counting a replayed pair as useful work twice. Compare diagnostic instrumentation with an uninstrumented binary.

**Decision:** determine whether P01 occupancy, P02 ordinary arithmetic, or preparation is the largest removable one-thread cost. A flat phase profile is insufficient; this milestone should reject at least one causal hypothesis or rank them by measured cost. Do not postpone all optimization until hardware counters become available; elapsed phases and route counts are sufficient to choose bounded candidates.

### M2 — Improve the synthesis arithmetic that dominates T1023–T2047

**Files/responsibility:** `src/legendre/scalar_paired.rs`, private dispatch in `src/legendre/mod.rs`, and scalar synthesis orchestration. Start with f64 scalar, recurrence-only full GL.

**Experiment order:**

1. P01: compacted active-lane transition and scalar-only transition staging, preserving coefficient-aware fallback.
2. P02: ordinary parity-aware kernel, with 1/2/4 independent SIMD chains and 2/4/8-degree panels.
3. P03: bounded preparation reuse only if measured preparation is material or needed by the chosen loop order. Change one axis per comparison. The first isolated kernel must include both hemispheres and its final reconstruction. A whole-transform candidate must include all preparation and FFT/layout effects. Keep existing kernels as fallback and small-shape controls.

**Acceptance targets:** predeclare a minimum practically useful whole-call gain and regression tolerance. For this principal parity milestone, a ≥10% reduction in the primary high-L synthesis cells is a reasonable screening target, not a prediction. Smaller gains may be retained as enabling changes only with an explicit explanation of the next removable cost. Track Rust/reference latency ratios, not just Rust/baseline. A candidate that speeds an isolated ordinary kernel but leaves total latency unchanged is rejected.

**Numerical gate:** full-array ordinary comparisons through T1023, bounded full T2047, analytic sparse high-degree fields, all range/transition tests, scalar S* and CC/F1 composition before enabling shared routes. AVX2 evidence does not authorize an untested AVX-512 specialization.

### M3 — Change ownership only after the kernel evidence supports it

**Files/responsibility:** scalar/spin orchestration, workspace layout, and a private bounded tile executor. Preserve disjoint output ownership and explicit execution capacity.

**Two candidates only initially:** current latitude ownership with reused factors, and dynamic weighted order tiles with parity-aware projection plus one bounded output-conversion stage. Use a fixed memory budget and no public stride/unsafe API. Compare q1 first, then odd/even worker capacities and q8/q16. Include capacity-one outer batches as regression controls.

**Decision evidence:** useful pair count, preparation duplication, active task count, leaf elapsed distribution, transpose/gather bytes/time, and whole S latency. Require a material reduction in the matched reference gap. Reject designs with the same mechanism as the failed equal-order/full-transpose prototype or with more tasks but worse elapsed utilization.

**Follow-up:** only if analysis's route-resolved leaves remain imbalanced, test a few additional disjoint order tiles using measured scaled/ordinary costs. Do not repeat blanket overpartitioning already rejected at T511/T720.

### M4 — Resolve f32 architecture independently

**Files/responsibility:** private precision dispatch and workspace conversion buffers; existing public f32 API stays intact. First use a diagnostic bridge to the current f64 implementation so arithmetic-porting effort is not confused with conversion cost.

**Gate:** compare native f32 and widened execution at T511/T1023 with complete arrays and all conversion included. If widened execution is substantially faster and meets a predeclared error/memory budget, derive a tile-based reusable implementation. If not, measure whether FFT or conversion dominates before implementing f32 paired kernels.

**Scope progression:** scalar/spin-1 STANDARD → S*/A* → gradient/derivative → spin 2 and CC/F1. Do not silently use a different precision policy for the adjoint of an operator without checking its identity residual. Record that outputs can differ from native f32 while remaining more accurate; bitwise preservation is not the sole acceptance criterion.

### M5 — Deliver bounded batch throughput and a usable Python policy

**Files/responsibility:** a small core execution helper or documented caller pattern, `python/src/lib.rs`, package stubs/tests, and API plans. Avoid a generic executor/backend hierarchy.

**First deliverable:** at most q live workspaces for B fields, with explicit outer versus inner concurrency. Compare same-plan Python serialization, separate plans, and a batch method. Measure allocation/copy and GIL/mutex time for small transforms separately from high-L arithmetic.

**Second, conditional experiment:** stream a basis/factor tile across B=4/8 fields. Compare with the now-bounded independent-field baseline. Do not compare against an artificially oversubscribed or allocation-heavy baseline. Only investigate dense GEMM/compression if repeated-field reuse remains dominant and the memory/setup crossover is plausible.

### M6 — Maintain FFT validation and isolate the N240 experiment

**Files/responsibility:** existing permanent CI workflow; standalone `rader.rs`, private N240 implementation if justified, and existing FFT tests/benchmarks. Do not enlarge generic dispatch in the first N240 candidate.

**Sequence:** verify M32-equivalent baseline → F01 private child → full 34-cell controls → F02 nested reachability as a separate patch. Keep N722, N1442, N257, N1443 and N2049 as mandatory controls because they exposed earlier regressions or share convolution machinery.

**Gate:** repeated whole-real-call gain, unchanged difficult-input behavior, bounded memory, and no reproducible unrelated latency loss beyond the declared gate. Only then consider F03's optional SHT experiment. Improving FFTs remains subordinate to M2/M3 for high-L synthesis parity.

### M7 — Decide whether direct recurrence has reached a practical limit

After M2–M4, compare full-call ratios to DUCC and SHTns for each target regime. The requested objective is met for a regime only when matched median latency is at or below the reference within a declared uncertainty analysis, while numerical and memory requirements hold. Being 5% faster than an old Rust baseline is not parity. If the remaining gap is still large and ordinary projection dominates, prototype one representative compressed parity block at the required f64 tolerance. Measure setup, rank, transient/persistent bytes and B-field application. Reject before building a full transform if ranks or setup amortization are unsuitable. If direct recurrence reaches parity for single fields but batching remains weak, prioritize streamed matrix tiles instead of compression. Preserve separate conclusions for GL, CC/F1, spin, precision and batch size.

## 17. Benchmark and numerical validation plan

### 17.1 Evidence status of the existing suite

The latest recorded root checks report 188 unit and 11 f32 tests passing with all features, with 13 unit and one f32 test ignored; no-default-feature checks report 150 unit and 11 f32 passing, with seven unit and one f32 ignored. Python tests were not repeated for the latest f32-only changes. FFT M34 records 111 passing tests in debug and release after restoring M32. These are historical reports, not checks run during this review. The validation evidence has different scopes:

| Area | Evidence already recorded | What must not be inferred |
|---|---|---|
| Geometry | Analytic moments, symmetry and 4π weights; pinned DUCC GL nodes/weights including threshold 100/101 | Universal accuracy for every allocatable grid size |
| Low/moderate f64 operations | Analytic harmonics/derivatives, arbitrary-map analysis, DUCC and separate adjoints on GL/CC/F1 | That a roundtrip alone validates A or an adjoint |
| High-L f64 scalar/spin 1 | Atmospheric full-array S/A comparisons through T2047 plus other selected checks | Full-operation extreme-range validation at every high-L grid |
| Spin 2 | Selected high-L synthesis/analysis and fused-path checks, including T1023 cases | Full T2047 performance parity or all extreme specialized-route coverage |
| Spin 3/4 | All-grid low-degree full-operation tests, sparse synthesis through T2047 and selected analysis coefficients through T1023 | Dense full-array high-L coverage or support for spin≥5 |
| f32 | Lower-degree operation tests; full T1023 scalar/spin-1 comparison and T1279 scalar outputs | A settled high-L acceptance threshold or full T2047 reference validation |
| FFT | Direct DFTs, analytic modes, reference libraries, range/tail/route tests | Non-AVX2 execution coverage from an AVX2 host or universal exceptional-finite exactness |

Test source supports these distinctions. For example, `python/tests/test_plan.py:54` compares scalar S and arbitrary-map A with DUCC; `src/spin_transform/tests.rs:4057` tests distinct adjoints; `:4795` covers spin 3/4 low-degree operations; `:7305,7590,7774,7984` are selected high-degree checks. `tests/f32.rs` evaluates map inner products in f64 but its harmonic products are formed in f32 before widening; a new high-accuracy f32 adjoint study should widen operands before multiplication rather than mistake test arithmetic error for transform error.

### 17.2 Required numerical matrix for future changes

| Axis | Required cases |
|---|---|
| Geometry | GL, CC, F1; minimum exact-analysis latitudes; overresolved grids; CC poles; odd/even latitude counts and equator |
| Bandlimits | L0/1/2; small analytic cases; 63/127/159/255; 319/511 hemisphere boundaries; 499/500 spin-2 boundary; 720/1023/1279/2047 as resources permit |
| Orders | m0, m=L, near-sectoral, quarter mmax, mmax=0, full mmax; polar-cutoff boundary and mixed active blocks |
| Longitude | `2*mmax+1`, `2*mmax+2`; synthesis Nyquist where supported; FFT-friendly and adjacent awkward lengths; phi0=0/0.37 and large finite phase |
| Precision | f64, f32 native, proposed widened f32; test rounding/downcast separately |
| Spin/mode | 0/1/2 STANDARD; spin 3/4 controls; GRAD_ONLY; DERIV1; separate S*, A*; rejection of invalid analysis modes |
| Values | Zero; impulse/isolated harmonic; random; smooth decaying coefficients; arbitrary maps; cancellation; maximum finite; normal/subnormal boundary; smallest subnormal; invalid m0/l<spin |
| SIMD | Scalar reference, runtime AVX2, executed AVX-512 if available, non-x86/portable fallback; all lane tails and unaligned FFT offsets |
| Ownership | Correct workspace, cross-mmax C01, capacity transitions 1→8→1, q3/q6, repeated calls, immutable plan sharing with separate workspaces |
| Specialized paths | Table cold/warm/declined, paired/general, transition, compacted mixed lanes, hemisphere, cached/uncached conversion, nested FFT generic/SIMD and safe/unsafe normalization |

Use independent analytic harmonics and derivatives, quadrature moments, direct small transforms, and real-field adjoint identities alongside pinned DUCC. For high dynamic range, compute expectations in scaled form or higher-precision arithmetic in the future test harness; a naive reference sum can overflow or cancel incorrectly too. Check finite/nonfinite classifications before relative norms. A near-zero floor must be explicit. For ordinary f64 low/moderate cases, retain the existing documented tolerances; the plan's provisional scalar GL through T255 target is relative L2≤5e-11 and normalized adjoint residual≤1e-10, with separately defined near-zero absolute treatment. Do not apply these blindly to every high-L spin/f32 case. Predeclare the appropriate error budget from independent evidence before accepting a new implementation. Record worst absolute error, relative L2, RMS, selected difficult entries and adjoint residual; a single aggregate norm can hide polar or small-residual failures.

### 17.3 Timing design

1. Record exact source revision, dirty diff hash if applicable, binary hash, compiler/version/flags/profile, CPU/ISA, topology, affinity, governor/boost state, precision, grid, spin/mode, threads, execution capacity, batch, input formula, reference version, elapsed time and memory. Portable distributed builds must not acquire global `target-cpu=native` or AVX-512 flags.
2. Separate plan construction, workspace allocation, lazy table build, first call, warmed reusable execution, and procedural/Python allocation-inclusive execution. Some existing FFT setup loops include destruction in their timed repetition (`benches/rustfft.rs:816`); a new setup comparison should label that scope or exclude destruction consistently across libraries.
3. Use alternating fresh candidate/reference processes, at least six pairs for acceptance when feasible, and several warm samples per process. Report paired ratios and intervals, not just the fastest sample. Shared-host interference should be recorded; rerun contaminated pairs without selectively discarding unfavorable clean samples.
4. Compare matched input arrays and conventions. The f32 atmospheric harness synthesizes into its maps before timing analysis; its later analysis uses those synthesized maps. Python's arbitrary-map analysis harness uses a different input formula. These are both useful workloads but not interchangeable numerical/performance comparisons.
5. SHTns must use the exact requested GL grid and documented normalization/component mapping. Separate its setup/planner cost from warm execution. Record generated variant/ISA and thread settings; do not assume the source template implies the executed path.
6. Keep end-to-end calls as the acceptance measurement. Profile/model phases explain a result but do not replace it. The M34 Rader model validates its arithmetic, yet its component medians need not sum to the separately measured complete pass.
7. Define regression gates before measurement. One workable new experiment policy is a paired interval showing ≥3% target gain, no repeatable >2% loss on mandatory controls, and a separately declared memory cap. The main SHT parity milestone should seek a materially larger change as in M2. These are proposed experiment gates, not historical acceptance rules or predicted gains. M34 rejected smaller repeatable regressions under its own decision policy.

### 17.4 Commands to run in an implementation session, not this review

Root/binding changes require the repository's established checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
uv run pytest python/tests
```

Also run appropriate no-default-feature and release tests, explicit ignored high-L cases affected by the change, and scalar/runtime-dispatch comparisons. The separate FFT workspace needs explicit commands, for example:

```sh
cargo fmt --manifest-path crates/rustsht-fft/Cargo.toml --all -- --check
cargo clippy --manifest-path crates/rustsht-fft/Cargo.toml --workspace --all-targets --all-features -- -D warnings
cargo test --manifest-path crates/rustsht-fft/Cargo.toml --workspace --all-features
cargo test --manifest-path crates/rustsht-fft/Cargo.toml --workspace --all-features --release
```

No command in this subsection was run during the static review. Report unavailable hardware, skipped expensive cases and missing reference comparisons explicitly. Do not add temporary GitHub Actions workflows to obtain them.

## 18. Rejected ideas and why not to repeat them

| Idea/experiment | Existing evidence | What would make a new experiment different |
|---|---|---|
| Equal-order synthesis plus full transpose | T511 q8 +32.65% latency; T1023 no established gain | Faster parity/transition kernel, weighted dynamic tiles, bounded conversion, and complete phase accounting |
| Scalar hemisphere pairing alone | Earlier T511 screen about +10.4% q1/+6.2% q8 | Different register panel and coefficient/transition organization, not just reflecting output |
| Universal 16-latitude groups | Regressed tested cases | ISA/operation-specific register tile separated from task/storage block size |
| More analysis leaves everywhere | Reported T511/T720 q8 regressions about 3.19%/11.55% | Route-aware costs or demonstrably stealable long tails with measured task overhead |
| Broad spin-1 fusion gate | Multicore selection was not accepted | A new kernel reducing register/staging pressure, measured at q8 as well as q1 |
| Extra recurrence chains/blanket FMA/cosine hoisting | Prior screens rejected variants; some hoists already performed by compiler | A changed dataflow with generated-code and whole-call evidence; no assumption that syntax alone saves instructions |
| Direct spin-1 recurrence instead of scalar conversion | Previous direct-vs-conversion comparison did not justify replacement | A specific supported regime where full measured cost and accuracy improve |
| Serial spin coefficient cache or direct converted-row reads | Existing cache variants regressed relative to accepted parallel/copied design | Bounded factor tiles integrated with a different owner, with cache traffic included |
| f32 normalization SIMD scan/interleaving everywhere | Spin screens regressed severely; scalar skip accepted only at two exact L values | Shared f64 arithmetic experiment or a new measured gate; do not broaden the current selector from intuition |
| T2047 f32 normalization-skip selection | Inconclusive single-pair observations, contaminated controls; removed | Clean repeated measurements and full output validation, with a worthwhile effect |
| Unrestricted dense Legendre table | O(L³) memory reaches 16 GiB at T2047 for scalar north basis alone | Streamed bounded tiles or a separately authorized repeated-plan memory budget |
| Rader-103 / blanket prime thresholds | Direct paired path and specific caches won prior comparisons | A demonstrably better child FFT and complete prime-call evidence |
| Blanket p241 Bluestein padding | Maximum-DC real case constrained the selector; dynamic-range limitation persisted | A separately range-validated route, not copying the complex pad to every real/precision path |
| Share all Rader/Bluestein buffers | N257 C2R regressions led to scoped p241 sharing | Proven compatible lifetime/layout and full-call nonregression, not bytes alone |
| Across-row C2C transpose prototype | Tested design was rejected; serial batch API remains | Bounded real tiles with fused conversion and full packing/scatter cost |
| M33 Rader safety-scan fusion | Target gains but repeatable C2C146/722 regressions | A new isolation boundary; unchanged instruction counts do not invalidate latency results |
| M34 radix-five in general dispatch | Three placements improved target rows but regressed unrelated calls | Private N240 child boundary and full matrix acceptance |
| Replace root RustFFT now | Standalone disconnected; N723/N4097 lose to RustFFT; high-L S FFT fraction small | F03's matched whole-SHT gain and explicit scope decision |
| Copy a reference's small-basis cutoff | Can discard a tiny basis times a huge coefficient | Coefficient-aware error/range proof with preserved scaled fallback |
| Major compressed-transform rewrite now | No Rust rank/setup/error crossover evidence | A successful bounded block experiment at required tolerance after direct-kernel opportunities are measured |

These results constrain design choices; they do not prove an algorithm can never work on another ISA or workload. Any retry should state exactly which mechanism changed and which previous failure it addresses.

## 19. Remaining uncertainties and experiments needed

1. **How much high-L synthesis actually reaches the ordinary fused loop?** Source has several escape paths, but no route-resolved degree/lane census. P01 answers this before extensive kernel work.
2. **How much of the ordinary gap is parity versus recurrence latency versus coefficient traffic?** The source arithmetic count separates useful output coverage, but retired instructions, spills and hardware bottlenecks are unmeasured. P02 isolates these alternatives.
3. **Will order ownership win after changing the kernel?** Current evidence is negative for the prior prototype. P04 needs a different layout/cost design and complete calls.
4. **Why do balanced estimated leaves finish at different times?** Scaled work, host scheduling and cache effects remain confounded. Collect route-weighted leaf costs and repeated clean timings; do not assert a bandwidth or NUMA cause from one host.
5. **What is the actual SHTns parity target?** No matched current SHTns benchmark exists in the reviewed evidence. Source comparison cannot rank its latency against current Rust or DUCC.
6. **What f32 accuracy is required at T1023–T2047?** Current full-array errors are recorded, but a high-L acceptance threshold is not settled. Define it before accepting widened or native f32 changes.
7. **Do C03's specialized routes corrupt finite reference outputs in the proposed public cases?** Missing checks and intermediate overflow are proven; pre-FFT route comparisons and analytic spin fields must establish exact manifestations without C02 masking them.
8. **How often will actual SHT spectra trigger Rader safety fallback?** Random-like standalone rows do not answer this. F03 records distribution-dependent route counts.
9. **Can N240 isolation avoid unrelated code-layout regressions?** M34 established the regressions, not their cause. New source boundaries are a hypothesis; repeat the full matrix.
10. **What changes on AVX-512, non-AVX2 x86 and non-x86?** The principal historical host is AVX2. Portable fallback and wider-vector execution need actual machines; no compilation-only claim should be upgraded to numerical/performance validation.
11. **What is the crossover for shared-field tiles or compression?** Current throughput evidence favors bounded independent fields. Basis rank, setup amortization and error-controlled compression remain unmeasured.
12. **What are current-HEAD binary timings?** Production-source equivalence is strong for FFT and inspected f64 SHT paths, but this review intentionally did not build or time them. All historical numbers retain their source/environment qualifiers. No unresolved item above prevents producing this static review. They are explicit future experiments, not hidden claims of verified performance or correctness.

## 20. Final recommendations and source references

### 20.1 Recommended decisions

1. Fix C01/C02 and test C03's specialized entry points first. Preserve original coefficients and transactional staged fallback.
2. Make high-L scalar synthesis the primary parity project. Measure ordinary/scaled route occupancy, then combine parity-aware output with a bounded SIMD degree panel. Keep q1 as a first-class target; scheduling cannot repair its existing gap.
3. Revisit ownership only with a faster kernel and a different, bounded layout strategy. Preserve the rejected prototype as a mandatory baseline comparison.
4. Run the widened-f32 experiment early. It is supported by a large measured architecture gap and the reference's precision choice.
5. Use bounded capacity-one workspace pools for multi-field throughput, then consider shared basis tiles if that workload justifies them.
6. Keep standalone FFT work separate from SHT claims. Maintain its permanent validation, isolate N240 if pursuing p241 gains, and require end-to-end evidence before any backend-policy change.
7. Defer dense/compressed replacements until measured setup, memory, rank and accuracy justify them. The current recurrence implementation still has substantial untested execution alternatives.

### 20.2 Primary local references

**Current RustSHT source:** [plan and capacity API](src/plan.rs), [workspace allocation](src/workspace.rs), [scalar orchestration](src/transform.rs), [spin orchestration](src/spin_transform.rs), [hemisphere specialization](src/spin_transform/hemisphere.rs), [paired kernels](src/legendre/scalar_paired.rs), [spin conversions](src/legendre/spin_fast.rs), [f32 scalar](src/legendre/scalar_f32.rs), [f32 spin](src/legendre/spin_f32.rs), [tables](src/legendre/scalar_table.rs), [real FFT adapter](src/fft.rs), [meridional resampling](src/meridional.rs), [Python bindings](python/src/lib.rs).

**Numerical/API decisions:** [scope and decisions](plans/00-scope-and-decisions.md), [provenance and license](plans/01-ducc-provenance-and-license.md), [numerics and grids](plans/02-numerics-and-grids.md), [API and Python](plans/03-api-and-python.md), [architecture](plans/04-architecture-and-performance.md), [validation and milestones](plans/05-validation-and-milestones.md), [DUCC operation crosswalk](plans/06-ducc-contract-crosswalk.md).

**SHT history:** [previous SHT review](ASTRA_SHT_REVIEW.md), [atmospheric performance](reports/atmospheric-resolution-performance-ectrans-2026-10-16.md), [order-owned synthesis](reports/order-owned-synthesis-2026-10-15.md), [fused scalar analysis](reports/fused-scalar-analysis-2026-10-15.md), [fused spin-1 analysis](reports/fused-spin1-analysis-2026-10-15.md), [fused spin-2 analysis](reports/fused-spin2-analysis-2026-10-15.md), [workspace capacity](reports/workspace-capacity-paired-2026-10-15.md), [capacity splits](reports/analysis-capacity-split-2026-10-15.md), [latest table selection](reports/adaptive-legendre-latest-head-results-2026-10-14.md), [ecTrans study](reports/ectrans-legendre-strategy-study-2026-10-16.md), [T1023 f32 skip](reports/f32-scalar-normalization-skip-2026-10-16.md), [T1279 f32 skip](reports/f32-scalar-normalization-skip-T1279-2026-10-16.md), [T2047 rejected screen](reports/f32-scalar-normalization-skip-T2047-2026-10-16.md).

**Standalone FFT:** [previous FFT review](crates/rustsht-fft/ASTRA_FFT_REVIEW.md), [complex execution](crates/rustsht-fft/src/complex.rs), [real execution](crates/rustsht-fft/src/real.rs), [Rader](crates/rustsht-fft/src/rader.rs), [native real multipasses](crates/rustsht-fft/src/real_multipass.rs), [SIMD](crates/rustsht-fft/src/simd.rs), [M31 baseline](crates/rustsht-fft/docs/m31-current-head-performance-report.md), [M32 acceptance](crates/rustsht-fft/benches/results/m32-radix19-ido1/README.md), [M33 final matrix](crates/rustsht-fft/docs/m33-final-performance-report.md), [M34 phases and rejected candidates](crates/rustsht-fft/docs/m34-rader-phase-and-radix5.md), [M34 validation](crates/rustsht-fft/benches/results/m34-validation/README.md).

**External source locations, at the revisions in section 2:**

- DUCC: `<local-home-path> (factor preparation, scalar/spin normal/scaled loops, parity extraction); `sht.cc` (order scheduling, Fourier strides, ring FFTs, exact analysis); `src/ducc0/fft/fft1d_impl.h` (fixed/general prime passes, vector multipass, Bluestein and complexify planner). Pinned versions were read through local Git; current namespace changes are distinguished above.
- SHTns: `<local-home-path> `kernel_spat_to_SH.gen.c`, `omp_SH_to_spat.gen.c`, `omp_spat_to_SH.gen.c`; `<local-home-path> `sht_private.h:440`, `sht_init.c:819`, `configure.ac:38,345`.
- Admiral: `<local-home-path> `simd_swizzle.hpp:48`, `rader.hpp:25,141`, `real_recombine.hpp:9`, `plan.hpp:389`; `<local-home-path> local `LICENSE`.
- FFTW: `<local-home-path> `dft/bluestein.c:55`; local `COPYING` and per-file headers. The system benchmark binary is separately identified as FFTW 3.3.10.
- ecTrans: `<local-home-path> `leinv_mod.F90:120`, `src/trans/cpu/algor/butterfly_alg_mod.F90:68`; local `LICENSE`. DUCC-derived source already carries notices for Martin Reinecke and collaborators; retain those notices and pinned provenance for direct translations. New independent Rust code should use Albert M. W. Yau attribution under repository policy. External algorithms can inform experiments without importing source expression or changing package licensing. This report adds no dependency, implementation, test, workflow, or benchmark result.
