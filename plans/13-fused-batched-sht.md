# Future plan 13 — Fused batched spherical harmonic transforms

**Status: FUTURE / NOT ACTIVE.** A separate follow-on task after the active
[Plan 11](11-ducc-equivalent-scalar-analysis.md). Coordinate cache-sensitive
algorithm selection with [future Plan 12](12-cache-informed-legendre-dispatch.md),
but neither plan authorizes changing the current single-map production
implementation. **No batch implementation or API change is requested now.**

## Goal and critical distinction

Accept an arbitrary practical number of compatible spherical fields at the
**public API** level (subject to checked array dimensions and available
memory), while processing a **smaller, measured internal fused block**. Do not
set an arbitrary public maximum of 8, 16, 32, or 64 maps.

Most importantly distinguish:

1. **Independent batched calls:** Loop or parallelize existing single-map
   transforms. Can already share an immutable plan and lazily initialized
   Legendre table, but still repeatedly *reads* table data or *computes*
   recurrence for each map; it is a useful baseline, **not fused Legendre work**.
2. **Fused recurrence:** For one harmonic order/latitude tile, compute or
   advance the Legendre recurrence **once for a group of fields** and
   accumulate each field's spectrum/output. Extra field accumulators compete
   for vector registers and scratch.
3. **Fused precomputed table:** Load a block of basis values once and apply
   it across many fields, turning matrix-vector-like operations into a blocked
   matrix-matrix-like operation, potentially suitable for GEMM. The best
   field block can differ from fused recurrence and the single-field cutoff.

The objective is real shared basis computation/memory traffic, not merely
a leading batch dimension accepted by an API. Measure the complete SHT as
well as the Legendre stage and do not assume whichever substage wins will
win overall.

## Library precedents and bounds of what they establish

| Library | Grounded design lesson | Source |
| --- | --- | --- |
| **SHTns** | `shtns_set_many(cfg, howmany, spec_dist)` configures batched transforms, including CPU. Its documented interface requires theta-contiguous spatial layout and setup before configuring the grid; it does **not** establish an optimal CPU batch/block size. | [SHTns initialization/API](https://nschaeff.bitbucket.io/shtns/group__init.html) |
| **ecTrans** | Designed to transform many meteorological fields per call. Its Legendre stage is formulated as a symmetry-aware **matrix-matrix product / BLAS GEMM**. This is the strongest reference for sharing precomputed bases across atmospheric fields, albeit with a different HPC/MPI design. | [Usage](https://sites.ecmwf.int/docs/ectrans/page/usage.html), [design](https://sites.ecmwf.int/docs/ectrans/page/design/index.html) |
| **torch-harmonics** | Tensor interface naturally admits leading batch dimensions; README example constructs 32 fields. This demonstrates batch acceptance, **not** a proven optimal CPU fused block size, nor equivalent numerical/algorithmic behavior to RustSHT. | [README](https://github.com/NVIDIA/torch-harmonics) |
| **DUCC 0.42.0 Python SHT** | Current `analysis_2d`/`synthesis_2d` represent one spin-0 map or two **spin components**, not an arbitrary set of unrelated fields. Do not mistake `ncomp=2` for general batch support. | [sht_pymod.cc](https://github.com/mreineck/ducc/blob/ducc0/python/sht_pymod.cc) |
| **S2FFT and other tensor backends** | Investigate leading-dimension or vmap behavior and whether basis work is truly shared; do not infer a fused CPU recurrence kernel merely from tensorized inputs. | [S2FFT repository](https://github.com/astro-informatics/s2fft) |

Martin Reinecke explicitly noted that transforming several datasets in one call
can reduce recurrence work **or** memory traffic in
[DUCC issue #86](https://github.com/mreineck/ducc/issues/86#issuecomment-6162674657).
His ecTrans example is especially relevant to multiple vertical levels and
physical variables in atmospheric modeling.

## Practical block-size hypothesis — not a fixed maximum

Start with a block-size sweep **B=1, 2, 4, 8, 16, 32**; include 64+ as an
API throughput/stress case. Initial *hypotheses* for CPU f64 (not validated
measurements):

- **B=1:** existing highly tuned single-map fallback.
- **B=2 or 4:** first fused recurrence prototypes; potentially easier to keep
  independent complex accumulators in registers.
- **B=4 or 8:** first throughput candidates for a fused recurrence loop.
- **B=8, 16, 32:** important precomputed-table/GEMM candidates, with blocked
  accumulation and memory-layout experiments.
- **B=32/64+:** accept user batch counts and internally tile into measured
  blocks; do not unroll a 32- or 64-field recurrence inner loop by default.

On x86-64 AVX2 the 16 architectural YMM registers constrain large numbers
of live SIMD accumulators, whereas ARM NEON/SVE and AVX-512 require separate
measurements. SIMD lane width, coefficient dataflow, number of spectra per
map, and compiler register allocation all influence spilling. The true best
block size can be smaller or larger than the above starting hypotheses.
Disassemble executed kernels or inspect compiler diagnostics rather than
assuming no spills.

Do not confuse **total batch length**, **internal fused field block size**,
**thread count**, and **number of harmonic orders/latitude tiles**.
Schedule across field blocks *or* transform orders/tiles as appropriate, and
avoid nested oversubscription. q8 can prefer a different combination than q1.
Consider hybrid splitting if B is large and the harmonic-order parallelism
is insufficient.

## Memory and streaming contract

The public API should not require all B maps to be materialized in a new
packed copy. Reuse existing array/storage when layouts permit; provide
an explicit copying/packing fallback when a strided or differently ordered
layout cannot efficiently share computation. Accept arbitrarily many batches
**only up to checked shape arithmetic, addressability, memory budget and
safe workspace allocation**; avoid hidden O(B * nlat * nlon) scratch
when bounded blocks suffice.

Illustrative input+output payload for **f64 scalar full-order** transforms
with real map [ntheta,nphi] and complex128 coefficients [nalm], no temporary
buffers, tables, FFT scratch, metadata or extra allocations:

| GL case | One field | B=8 | B=32 |
| --- | ---: | ---: | ---: |
| T720, 721 x 1442 map | ~11.9 MiB | ~95 MiB | ~381 MiB |
| T2048, 2049 x 4098 map | ~96.1 MiB | ~769 MiB | ~3.00 GiB |

These are payload estimates, **not** minimum free-RAM requirements or a
recommended maximum B. A full precomputed basis can additionally consume
large memory; blocked reuse does not make its total table allocation free.

Support user-owned output and bounded per-block workspaces where practical.
Keep plan metadata immutable/shareable and make ownership/parallel safety
explicit. Test partial final blocks (e.g., B=5, 17), odd SIMD tails, and
different layouts.

## Proposed API, deliberately provisional

A likely Rust starting point is a shape-aware batch method attached to
`ShtPlan`, with an optional separate Python binding layer:

```rust
// Illustrative only: no public signature is authorized by this plan.
plan.analysis_batch_into(maps, coefficients, &mut workspace)?;
// Conceptual shapes: maps [batch, ntheta, nphi],
// coefficients [batch, nalm], or explicit strides/views.
```

Requirements before standardizing the signature:

- Preserve existing scalar calls, spin/normalization/convention, modes,
  adjoints, GL/CC/F1, numerical edge cases, and no-default-features.
- Clarify what a batched spin-1/2 input means: two spin **components** per
  physical field, times an independent batch axis. Do not conflate them.
- Check all shapes, strides, aliasing and overflows before writes.
- Preserve every field's independent result, input-safety rules,
  scaled/checked fallback and transactional error behavior; a cached
  recurrence proof alone is not a proof that every map's input products are
  safe. Define batch error handling without partially corrupting outputs.
- Reuse immutable FFT/Legendre plan metadata and per-worker scratch
  without data races. Decide how many workspaces and buffers the caller
  supplies before promising zero-copy execution.
- If Python/xarray wrappers are added later, flatten arbitrary leading
  dimensions into a logical batch while retaining dimension labels and
  a documented packing/stride policy. No dependency on xarray belongs in
  the core Rust crate.

## Implementation sequence (future task only)

1. **Independent baseline:** implement/reference sequential and
   pool-parallel single-map loops over a shared plan, measuring API overhead,
   allocations and the best existing single-map throughput.
2. **Fused recurrence proof of concept:** start CPU f64 GL scalar,
   `B=2/4/8`, one shared basis recurrence per harmonic order/latitude tile.
   Retain DUCC-equivalent normalization, scale handling, parity and
   checked fallbacks. Show real reuse in instrumentation/source/assembly.
3. **Fused table proof of concept:** separately evaluate basis-row reuse
   across B and blocked matrix-matrix processing (handwritten or optional
   quality BLAS only if dependency trade-off warrants it). Benchmark cache
   capacity and memory bandwidth, not only FLOPs.
4. **Layout/parallel strategy:** benchmark field-major versus blocked
   lane/field-major scratch, Fourier-stage batching, q1/q8 and scheduling
   alternatives, thread affinity and peak memory.
5. **Select simple block policies:** measure both total batch time and
   time per field at B=1/2/4/8/16/32/64+ over T72/T127/T255/T511/T720
   plus representative larger sizes if feasible. Use full and reduced
   mmax, exact and oversampled sampling; include cold/warm plan life.
6. **Validate and integrate:** correctness against independent single-map
   paths and pinned DUCC/analytic/adjoint references; test extreme finite,
   subnormal and nonfinite data, repeated workspaces, partial batches,
   strided layouts, concurrent callers, fallback and f32/spin where enabled.
   Add ergonomic bindings only once the core semantics are sound.

Use interleaved, calibrated process comparisons on a quiet host and retain
the same source/toolchain and input cases for competing kernels. Classify
uncontended versus host-interfered results separately. A reduction in
Legendre-only time that regresses the full SHT is not a production win.

## Integration with future cache-informed selection (Plan 12)

**One cache-topology probe per process is sufficient regardless of B.** No
new CPUID/sysfs/sysctl/Windows queries and **no memory-latency benchmark**
should run when a different batch size arrives. Estimate actual table bytes
for the plan, while the cheap selector uses additional features:

```text
strategy = choose(
    cache_info, table_bytes, operation, spin, lmax, mmax,
    ntheta, effective_workers, schedule, layout,
    total_batch_size, internal_fused_block_size,
    expected_future_plan_calls
)
```

This is a conceptual input list, **not** an invitation to implement an
overcomplicated hot-path decision tree. Offline calibrate a few simple
operation-specific cases, prefer cache/memory-safe fallbacks on uncertainty,
and decide before entering timed transform loops.

- Separate **independent B calls** (one table initialized once, but basis
  traffic repeated) from **fused B fields** (basis traffic/recurrence work
  genuinely shared). The same batch count means different economics.
- Table setup is amortized over plan reuse and all fields, but a batch of
  eight fused maps is not equivalent to eight separate warm calls.
  RustSHT's existing `LegendreReuseEstimate` counts eligible calls by
  direction; account for reuse *within* a fused call separately.
- Precomputed versus recurrence may switch at different `lmax` for
  B=1 versus B=8/32; **either** kernel can gain from fusion. Do not
  assume bigger B always favors the table, or that one L2 ratio works
  across analysis/synthesis.
- The single-map cache heuristic can be calibrated first under Plan 12;
  update it for genuine fused batches **only after** the fused implementation
  exists and has been profiled. Coordinate the follow-up work explicitly.

## Deliverables / non-goals

Deliver a small, stable or clearly experimental batch API, working fused
implementations with measured basis reuse, reproducible throughput data,
a documented internal blocking/scheduling rule, and rigorous numerical and
memory-safety tests. If true fusion never beats shared-plan independent
calls on CPU, retain the simpler baseline and document the negative result.

**Not goals:** hardcoding a maximum public B, committing to B=8 as optimal,
turning two spin components into a fake batch, adopting a tensor framework in
the Rust core, using runtime autotuning during import/plan construction, or
changing current single-map dispatch before comparative evidence exists.

**Current task authority remains Plan 11.** This plan is a future reference.
