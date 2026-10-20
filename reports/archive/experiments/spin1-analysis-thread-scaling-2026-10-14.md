# Spin-1 GL analysis thread scaling

## Result

At T511, RustSHT's reusable native spin-1 STANDARD analysis improves from
`39.403 ms` at one thread to `12.378 ms` at eight threads, then is slightly
slower at sixteen threads (`12.604 ms`). Installed DUCC 0.41.0 improves from
`13.056 ms` at one thread to `1.943 ms` at eight and `1.500 ms` at sixteen. The RustSHT/DUCC ratio grows from `3.02x` at one thread to `6.37x` at eight
and `8.40x` at sixteen. This points to weaker RustSHT multicore scaling in the
Legendre analysis path; FFT optimization remains deferred.

## Matched workload

The Rust source was `772213e349f67c20a3298299134c784ff4fd6bcf`. The workload
was f64 GL T511, `512x1025`, `mmax=511`, STANDARD analysis, `phi0=0.37`, and
deterministic full-band inputs. RustSHT used a reused plan and workspace from
the release `m1b_bench` executable. DUCC used the Python plan benchmark with
installed `ducc0==0.41.0`, whose pinned source revision is
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`. DUCC plan construction and NumPy
input preparation were outside the reported operation samples; those copy
timings were also recorded separately. Thus the native Rust and Python DUCC
timings have slightly different call boundaries, although the transform,
grid, values, and requested thread counts match. Each q value ran in three processes pinned to a CPU set matching the requested thread count; Rayon and DUCC
were configured with q threads. Rust had one warm-up and five timed calls per
process. DUCC used two inner timing batches with five samples per batch. The
table reports the median of the three process medians. The report retains the numerical medians and ranges; raw process captures remain local and untracked.

| Threads | RustSHT ms (process-median range) | DUCC ms (process-median range) | Ratio | Excess ms |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 39.403 (39.114–39.874) | 13.056 (12.953–13.233) | 3.02x | 26.347 |
| 2 | 23.622 (23.612–23.996) | 6.599 (6.589–6.665) | 3.58x | 17.023 |
| 4 | 16.505 (16.372–16.612) | 3.426 (3.423–3.545) | 4.82x | 13.079 |
| 8 | 12.378 (12.240–12.592) | 1.943 (1.940–1.986) | 6.37x | 10.435 |
| 16 | 12.604 (12.441–12.617) | 1.500 (1.484–1.561) | 8.40x | 11.104 |

RustSHT q1-to-q8 speedup is `3.18x`, while DUCC's is `6.72x`. Raising RustSHT
from q8 to q16 does not help; DUCC gains a further `1.30x`. These measurements
do not identify whether the remaining difference comes from scheduling,
kernel throughput, or memory effects. They do establish that the next
optimization target should be analysis-side recurrence and accumulation,
with q8/q16 scaling measured explicitly.

## Scheduling-weight experiment

Per-leaf diagnostics from this source showed the active-ring work estimator
balanced the eight predicted order groups to within about 2%, but measured
group times still varied. A temporary benchmark-only candidate changed each
order's estimated work from
`degree_count * active_ring_count` to
`(degree_count + K) * active_ring_count`, representing per-order preparation
overhead. The environment-controlled candidate was removed after timing. At q8, six alternating process pairs compared `K=0` and `K=4`. The median
paired reduction was only `0.15 ms` (about 1.2%), with differences of
`+0.170, -0.175, +0.129, +0.240, -0.377, +0.815 ms`; four of six pairs favored
K=4 and two favored K=0. Process variability overlaps. The temporary control
also parsed its environment variable inside the recursive midpoint helper,
so it slightly perturbed both sides. This is insufficient evidence for a
production scheduling change. Larger K values were slower or tied in the
initial sweep. The current weighted split remains unchanged. The report retains the scheduling comparison summary; raw process captures remain local and untracked. The temporary environment control is absent from production source.

## Environment and scope

CPU: x86-64 CPU, Linux. Rust was
`1.99.0 (b940084d7 2026-09-28)` with LLVM 23.1.1; release profile,
`opt-level=3`, 16 codegen units, LTO off, portable Pulp runtime dispatch
(AVX2/FMA on this host), and no `target-cpu=native`. DUCC is the installed
0.41.0 wheel; wheel compiler flags are not available in this comparison. The host was shared, though CPU affinity was fixed. The adaptive recurrence/table selector is already implemented and measured
in [`adaptive-legendre-strategy-2026-10-14.md`](adaptive-legendre-strategy-2026-10-14.md). The current mainline contains no Legendre-kernel or selector changes after
that selection commit; the newer spin-1 comparison adds no evidence that the
small-band table thresholds should change. `sht_bench` remains a read-only
historical reference (local revision
`0d5462232c24ad926cc5a2e8bf653b131c93445f`); its `2F x 4F` GL grids and
endpoint-including CC definitions are not substituted for RustSHT's exact
minimum grids. SHTns source revision
`4e69ceb3fc5f19475d55b03ff0fdec210dc3075e` selects a Gaussian grid for
`sht_auto`, `sht_gauss_fly`, and `sht_quick_init`. Its `choose_best_sht`
performs plan/grid-time timing among available on-the-fly kernels in this
source (`on_the_fly_only` is enabled). Operation-specific selection is
relevant; hidden timing tied to plan construction remains inappropriate for
RustSHT's immutable plans and execution-time Rayon pools. This run is a timing profile, not a numerical validation. Existing spin-1
analytic, DUCC, adjoint, grid and high-band tests remain the correctness
evidence. No production kernel, numerical tolerance, public API or FFT code
changed in this profile.

## Paired analysis FMA experiment

A temporary candidate changed only the normal-range `AnalysisPairAccumulation`
reducer in `src/legendre/scalar_paired.rs` to use Pulp `mul_add_f64s` and scalar
`mul_add` tails. It left the recurrence, scaled/mixed fallback, cutoff behavior
and reduction boundaries unchanged. For cosine-weighted terms it retained the
existing product order by forming `opposite * basis` before the FMA. The source
change was reverted after benchmarking. AVX2 disassembly contained `vfmadd231pd` and no YMM stack spills in the
candidate loop. The ordinary vector arithmetic sequence fell from 20
multiply/add instructions to 12. That did not translate into a whole-transform
gain. Five alternating q8 process pairs at T511 gave scalar medians of
`7.266 -> 7.169 ms` (1.34% lower, with a large candidate outlier) and spin-1
medians of `12.000 -> 12.341 ms` (2.84% higher). Every paired spin-1 process
median favored the baseline; the four pairs without the largest candidate
outlier were about 1.5–3.2% slower with FMA. Independent three-process q1
screen medians were `23.704 -> 23.341 ms` for scalar and `38.319 -> 38.666 ms`
for spin 1. The q1 screen is small and not treated as a reliable scalar gain. `5,045,565,736` to `4,862,893,298` (3.62%), while instrumented whole-operation
are not cycles or native elapsed time. The lower reference count therefore
does not override the slower paired native timings. Focused paired-analysis
tests (7 passed) and the DUCC 0.41.0 numerical checks (7 Rust tests and 2 f32
tests passed) passed on the candidate, including extreme finite fallback
coverage. No high-band release suite was run specifically for this rejected
candidate. The report retains the paired numerical summary; raw process captures remain local and untracked. The experiment does not justify changing the reducer. The current AVX2
multiply/add sequence remains in production; a future accumulation change
needs to improve measured wall time, not only reduce simulated instruction
references.

## Spin-1 order-major analysis layout experiment

A separate prototype copied the GL spin-1 Fourier orders from ring-major rows
into two compact order-major planes in the workspace's existing
`spin1_scalar_coefficients` scratch. The paired scalar-conversion analysis
then read contiguous latitude rows. Ring-major spectra remained intact for
the scaled/general fallback. This tested whether strided access to fixed `m`
was limiting the q8 analysis kernel; it changed no recurrence, accumulation,
quadrature or FFT arithmetic. The candidate ran only for f64-compatible spin-1 GL STANDARD analysis at eight
or more executing Rayon threads, with all rings inside the existing fast
latitude range and a transposed payload under 17 MiB. It allocated scratch on
first eligible execution and reused it afterward. T511 used 8 MiB and T720
used 15.864 MiB. The source was a temporary prototype on `2e1fa65`; the
production implementation was restored after measurement. The release `m1b_bench` comparison used exact GL grids `512x1025` at T511 and
`721x1443` at T720, full `mmax`, f64 STANDARD analysis, `phi0=0.37`, reused
plans/workspaces, one warm-up and five samples per process. Five alternating
process pairs ran at T511; three ran at T720, 8 pinned CPU cores on the x86-64 CPU under Linux. Rust 1.99.0 / LLVM 23.1.1 used
release `opt-level=3`, 16 codegen units, LTO off and portable Pulp runtime
dispatch (AVX2/FMA), without `target-cpu=native`. Commands were:

```sh
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" <temporary-path> 1 511 8 analysis GL STANDARD 512 1025 511 requested-only
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" target/release/examples/m1b_bench 1 511 8 analysis GL STANDARD 512 1025 511 requested-only
```

The baseline binary was built from the same production source before the
temporary layout edit. Each table entry is the median of process medians in
milliseconds. The pair-specific values all include a transform warm-up and
five timed transforms:

| Grid | Baseline q8 process medians | Order-major q8 process medians | Median change |
| --- | --- | --- | ---: |
| T511 | `14.950, 12.305, 12.674, 12.563, 12.092` | `15.569, 12.922, 12.986, 13.214, 12.563` | `+3.4%` slower |
| T720 | `33.848, 33.817, 35.106` | `35.082, 35.828, 34.904` | `+3.6%` slower |

All five T511 paired process medians favored ring-major input; at T720 two of
three pairs favored ring-major. Candidate warm medians increased from
`12.563` to `12.986 ms` at T511 and from `33.848` to `35.082 ms` at T720. The first-call medians increased from `13.365` to `17.717 ms` at T511 and
from `35.216` to `42.676 ms` at T720, including lazy scratch allocation and
the transpose. Workspace construction did not allocate this scratch. The
measured q1 control had baseline medians `39.570 ms` at T511; the candidate
selector excludes q1, so this was not a paired comparison. Exact individual
samples are in
[`the layout experiment data`](../../data/spin1-order-major-analysis-2026-10-14/process-samples.txt). The q8 release DUCC T511 test passed with `RAYON_NUM_THREADS=8`; spin-1 GL
analysis coefficient maximum absolute error was `4.970e-18`, and the
synthesis-analysis coefficient maximum absolute error was `1.255e-14`. The
test also covered spin 1/2 on CC and F1 and their existing DUCC checks. The
candidate's repeated warm regressions or 9 CPU cores MiB lazy scratch do not justify
keeping the transpose. RustSHT continues to read ring-major input in this
path. The scalar order-major layout remains separate because its measured
whole-transform result was positive and it reused already allocated
workspace storage.
