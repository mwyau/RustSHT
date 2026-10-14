# Spin-1 GL analysis thread scaling

## Result

At T511, RustSHT's reusable native spin-1 STANDARD analysis improves from
`39.403 ms` at one thread to `12.378 ms` at eight threads, then is slightly
slower at sixteen threads (`12.604 ms`). Installed DUCC 0.41.0 improves from
`13.056 ms` at one thread to `1.943 ms` at eight and `1.500 ms` at sixteen.
The RustSHT/DUCC ratio grows from `3.02x` at one thread to `6.37x` at eight
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
grid, values, and requested thread counts match.

Each q value ran in three processes pinned to CPUs `0..q-1`; Rayon and DUCC
were configured with q threads. Rust had one warm-up and five timed calls per
process. DUCC used two inner timing batches with five samples per batch. The
table reports the median of the three process medians. Raw samples are in
[`native-process-samples.txt`](data/spin1-analysis-thread-scaling-2026-10-14/native-process-samples.txt)
and [`ducc-process-samples.txt`](data/spin1-analysis-thread-scaling-2026-10-14/ducc-process-samples.txt).

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
overhead. The environment-controlled candidate was removed after timing.

At q8, six alternating process pairs compared `K=0` and `K=4`. The median
paired reduction was only `0.15 ms` (about 1.2%), with differences of
`+0.170, -0.175, +0.129, +0.240, -0.377, +0.815 ms`; four of six pairs favored
K=4 and two favored K=0. Process variability overlaps. The temporary control
also parsed its environment variable inside the recursive midpoint helper,
so it slightly perturbed both sides. This is insufficient evidence for a
production scheduling change. Larger K values were slower or tied in the
initial sweep. The current weighted split remains unchanged.

Raw candidate samples are in
[`scheduling-overhead-paired-samples.txt`](data/spin1-analysis-thread-scaling-2026-10-14/scheduling-overhead-paired-samples.txt).
The temporary environment control is absent from production source.

## Environment and scope

CPU: AMD Ryzen 9 5950X, Linux `7.0.14-20-pve`. Rust was
`1.99.0 (b940084d7 2026-09-28)` with LLVM 23.1.1; release profile,
`opt-level=3`, 16 codegen units, LTO off, portable Pulp runtime dispatch
(AVX2/FMA on this host), and no `target-cpu=native`. DUCC is the installed
0.41.0 wheel; wheel compiler flags are not available in this comparison.
The host was shared, though CPU affinity was fixed.

The adaptive recurrence/table selector is already implemented and measured
in [`adaptive-legendre-strategy-2026-10-14.md`](adaptive-legendre-strategy-2026-10-14.md).
The current mainline contains no Legendre-kernel or selector changes after
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
RustSHT's immutable plans and execution-time Rayon pools.

This run is a timing profile, not a numerical validation. Existing spin-1
analytic, DUCC, adjoint, grid and high-band tests remain the correctness
evidence. No production kernel, numerical tolerance, public API or FFT code
changed in this profile.

## Paired analysis FMA experiment

A temporary candidate changed only the normal-range `AnalysisPairAccumulation`
reducer in `src/legendre/scalar_paired.rs` to use Pulp `mul_add_f64s` and scalar
`mul_add` tails. It left the recurrence, scaled/mixed fallback, cutoff behavior
and reduction boundaries unchanged. For cosine-weighted terms it retained the
existing product order by forming `opposite * basis` before the FMA. The source
change was reverted after benchmarking.

AVX2 disassembly contained `vfmadd231pd` and no YMM stack spills in the
candidate loop. The ordinary vector arithmetic sequence fell from 20
multiply/add instructions to 12. That did not translate into a whole-transform
gain. Five alternating q8 process pairs at T511 gave scalar medians of
`7.266 -> 7.169 ms` (1.34% lower, with a large candidate outlier) and spin-1
medians of `12.000 -> 12.341 ms` (2.84% higher). Every paired spin-1 process
median favored the baseline; the four pairs without the largest candidate
outlier were about 1.5–3.2% slower with FMA. Independent three-process q1
screen medians were `23.704 -> 23.341 ms` for scalar and `38.319 -> 38.666 ms`
for spin 1. The q1 screen is small and not treated as a reliable scalar gain.

Callgrind T511 q8 spin-1 analysis instruction references fell from
`5,045,565,736` to `4,862,893,298` (3.62%), while instrumented whole-operation
medians rose from `2020.900 ms` to `2077.121 ms` (2.8%). Callgrind references
are not cycles or native elapsed time. The lower reference count therefore
does not override the slower paired native timings. Focused paired-analysis
tests (7 passed) and the DUCC 0.41.0 numerical checks (7 Rust tests and 2 f32
tests passed) passed on the candidate, including extreme finite fallback
coverage. No high-band release suite was run specifically for this rejected
candidate. Exact five-sample process inputs are in
[`paired-process-samples.txt`](data/paired-analysis-fma-2026-10-14/paired-process-samples.txt).

The experiment does not justify changing the reducer. The current AVX2
multiply/add sequence remains in production; a future accumulation change
needs to improve measured wall time, not only reduce simulated instruction
references.
