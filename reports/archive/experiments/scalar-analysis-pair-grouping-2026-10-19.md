# Scalar-analysis SIMD pair-grouping checkpoint

## Five-process calibrated refresh — 2026-10-20

The calibrated runner now records five independent Rust/DUCC process pairs
instead of three and prints each process median and p10/p90 beside the matching
host diagnostics. It used the current `205c20b6` kernel source, Rust 1.99.0,
Pulp V3 AVX2/FMA (`-C target-cpu=x86-64-v3`), and pinned DUCC
`80388904b8cfc32400ba84af8a6a45aabfe60f86` with GCC 15.2.0
(`-O3 -DNDEBUG -march=x86-64-v3`). The same `m=1`, `lmax=720` fixture and
transformed coefficients were used at 64, 128 and 256 active latitude lanes.
Each process ran 30 calibrated batches; repetition counts were 8,257, 4,260
and 2,186 respectively. All 2,700 measured kernel or reset-inclusive batches
were 33.72–80.57 ms, with none outside the 20–100 ms target.

Entries are median (p10–p90) µs per operation. The checked column measures the
experimental in-loop range-check specialization; the unchecked column omits
the ordinary post-group scan. Production still uses the post-group scan.

| Lanes | Boundary | Rust checked | Rust unchecked | DUCC | Ratios checked / unchecked |
|---:|---|---:|---:|---:|---:|
| 64 | Kernel only | 8.746 (8.653–9.382) | 4.742 (4.699–4.797) | 4.151 (4.105–4.213) | 2.107× / 1.142× |
| 128 | Kernel only | 16.862 (16.731–17.876) | 8.723 (8.677–8.786) | 8.038 (7.980–8.107) | 2.098× / 1.085× |
| 256 | Kernel only | 33.038 (32.776–34.055) | 17.478 (17.339–17.740) | 15.873 (15.746–15.950) | 2.081× / 1.101× |
| 64 | Reset inclusive | 8.912 (8.792–9.703) | 4.808 (4.763–4.864) | 4.260 (4.212–4.323) | 2.092× / 1.129× |
| 128 | Reset inclusive | 16.919 (16.824–17.641) | 8.783 (8.750–8.846) | 8.178 (8.115–8.247) | 2.069× / 1.074× |
| 256 | Reset inclusive | 33.139 (32.864–34.253) | 17.559 (17.410–17.750) | 16.073 (15.957–16.174) | 2.062× / 1.092× |

The `kernel_only` timer excludes input copies and output clearing; the
`reset_inclusive` timer includes both. DUCC clears the active `m..=lmax`
coefficient span, matching Rust's staged output. Coefficient preparation,
packing, FFT and scheduling are outside these direct-kernel boundaries. Both
adapters fence or black-box mutable state during repeated operations, compare
Rust and DUCC outputs after timing, and validate the transformed factors.
The optimized DUCC adapter disassembly contains a `map2alm_kernel` call between
the per-operation clock reads and a backward branch to the repetition loop;
the kernel contains packed AVX2 `vfmadd*pd` operations. Rust reports Pulp V3 at
runtime and black-boxes state after every measured operation.

DUCC's per-process kernel medians stayed within 0.9%, 1.2% and 0.7% at 64,
128 and 256 lanes; this run therefore does not reproduce the earlier 64-lane
9.599/5.621 µs or 256-lane 25–39 µs process regimes. Their original cause is
not recoverable. Rust unchecked medians also stayed within 1.0% across its five
processes. The checked experimental route had elevated process medians in two
64-lane runs, one 128-lane run and one 256-lane run, despite narrow within-run
spreads. Frequency and scheduler sampling were available. The sampled clock
rates for the slower checked processes were similar to those of the faster
processes, run-queue wait remained negligible, and DUCC showed more involuntary
preemption without corresponding shifts in its stable medians. These
diagnostics do not identify a single cause for the checked-route process
shifts; retain all timing samples and treat the checked aggregate ratio as
descriptive, not a stable production estimate. Exact transient frequency and
scheduler readings remain local. Output relative L2 error was `6.670e-14`,
`1.628e-13` and `5.928e-14` for 64, 128 and 256 lanes.

## Scalar-analysis pair-tile screen — 2026-10-20

The scalar-analysis block loop now has compile-time 64/128/256 pair tiles;
256 is the candidate default. Only f64's paired scalar-analysis path uses this
tile. The shared 64-lane synthesis/spin blocks and f32 scalar-analysis blocks
are unchanged. Workspace scratch was already allocated for 256 lanes due to
`SCALAR_ORDER_BATCH_SIZE`, so the 256 tile adds no scratch bytes.

The ignored active-pair diagnostic invokes the production packer on the
T720 GL full-order (`721x1443`) and T300 GL reduced-order (`301x603`,
`mmax=150`) geometries. It confirms identical active-lane counts and SIMD tail
capacity at all three tiles:

| Case | Tile 64 blocks | Tile 128 blocks | Tile 256 blocks | Active pairs | SIMD lane capacity |
|---|---:|---:|---:|---:|---:|
| T720, `mmax=720` | 3,384 | 1,851 | 1,139 | 193,993 | 195,204 |
| T300, `mmax=150` | 453 | 302 | 151 | 22,413 | 22,592 |

This reduces repeated per-order block preparation by 66–67% at 256 lanes while
preserving all active latitude work. The SIMD tail capacity is unchanged and
the lane-sized recurrence scratch remains 55,296 bytes per worker.

The lightweight full-analysis screen reused plans/workspaces and ran five
timed calls in each of three independent processes per tile, interleaving tile
order. Each single-thread process was pinned to one physical core; q8 processes
used the same eight physical cores. Results are medians of the three process
medians, with the process-median range in parentheses (ms):

| Grid and `mmax` | Threads | Tile 64 | Tile 128 | Tile 256 |
|---|---:|---:|---:|---:|
| T720, full | q1 | 27.581 (27.475–27.811) | 26.462 (26.320–26.500) | 26.467 (26.300–26.657) |
| T720, full | q8 | 4.487 (4.412–6.066) | 4.342 (4.270–4.353) | 4.290 (4.290–4.311) |
| T300, `mmax=150` | q1 | 2.859 (2.839–2.925) | 2.752 (2.729–2.760) | 2.704 (2.677–2.718) |
| T300, `mmax=150` | q8 | 0.494 (0.494–0.660) | 0.572 (0.533–0.633) | 0.548 (0.466–0.552) |

Tile 256 improved the q1 medians by 4.0% at T720 and 5.4% at T300, and the
q8 T720 median by 4.4%. q8 T300 did not show a reliable improvement; all three
process groups had substantial spread relative to the sub-millisecond
operation. The first q8 T720 tile-64 process also had one elevated process
median. Treat these as development screens, not final parity measurements.
Release all-feature tests passed for the 64, 128 and 256 configurations;
release no-default-feature tests and all-target/all-feature Clippy also passed.

## Calibrated checked/unchecked comparison — 2026-10-20

**Boundary correction:** the table and ratios in this historical subsection
measure Rust's fused recurrence and writes into `staged_coefficients`, but
DUCC's `map2alm_kernel` also accumulates directly into `alm`. Rust's subsequent
per-block coefficient commit was outside its timer, so the old cross-library
ratios are not equivalent-work and must not be used for parity claims. The
updated runner reports Rust `kernel_plus_commit`, `commit_only`, and
`reset_inclusive` separately; use the corrected values in
[`PERFORMANCE_STATUS.md`](../../../PERFORMANCE_STATUS.md). The historical
samples below remain for traceability.

The earlier nine-call direct measurements are retained below as historical
evidence. This remeasurement uses the reusable harness in
[`reports/data/scalar-analysis-pair-grouping-2026-10-19`](../../data/scalar-analysis-pair-grouping-2026-10-19)
and `scripts/benchmark_scalar_analysis_kernel.py`. It verifies the pinned DUCC
revision against the source snapshot, warms and calibrates with 20–100 ms
batches, then records 30 batches in each of three independent process
invocations. Rust and DUCC process order alternates by invocation. Rust checked
and unchecked variants alternate order within each process. Every backend uses
the same `m=1`, `lmax=720` fixture, transformed coefficients and initial
recurrence state at 64, 128 and 256 active latitude lanes. Rust selects Pulp
V3 AVX2/FMA; DUCC uses four-lane f64 AVX2/FMA compiled with the same x86-64-v3
target. Each measured batch was 30.02–72.43 ms; all 1,620 measured batches
were within 20–100 ms.

The `kernel_only` boundary restores recurrence inputs and clears output outside
the per-operation timer. The `reset_inclusive` boundary includes those copies,
the output clear and the kernel. Coefficient preparation, ring packing, FFT,
scheduling and complete transforms are excluded from both. Both adapters
prevent dead-code elimination and compare final results outside timed regions.

Values below give median `(p10–p90; full range)` in µs/op.

| Mode | Lanes | Rust checked | Rust unchecked | DUCC |
|---|---:|---:|---:|---:|
| Kernel only | 64 | 7.124 (7.055–7.305; 7.017–7.929) | 4.847 (4.770–5.011; 4.758–5.600) | 4.159 (4.132–4.369; 4.099–4.795) |
| Kernel only | 128 | 12.656 (12.594–12.900; 12.526–15.810) | 8.824 (8.759–9.206; 8.726–10.177) | 8.113 (8.044–8.600; 7.976–9.626) |
| Kernel only | 256 | 24.896 (24.708–25.381; 24.643–31.350) | 17.508 (17.381–18.409; 17.277–20.572) | 15.965 (15.886–16.427; 15.768–24.278) |
| Reset inclusive | 64 | 7.189 (7.127–7.347; 7.090–7.679) | 4.880 (4.834–4.936; 4.812–5.215) | 4.264 (4.239–4.441; 4.197–4.783) |
| Reset inclusive | 128 | 12.755 (12.708–13.230; 12.613–15.197) | 8.912 (8.870–9.192; 8.829–10.643) | 8.252 (8.194–8.727; 8.121–10.764) |
| Reset inclusive | 256 | 24.977 (24.818–25.695; 24.706–33.499) | 17.635 (17.426–18.121; 17.348–21.288) | 16.171 (16.100–16.951; 16.023–25.352) |

Kernel-only Rust/DUCC ratios at 64/128/256 lanes were 1.713/1.560/1.559
for checked and 1.166/1.088/1.097 for unchecked. Reset-inclusive ratios were
1.686/1.546/1.545 and 1.144/1.080/1.090, respectively. Reset-inclusive
timing exceeds kernel-only timing by 0.03–0.21 µs per call; reset work does
not explain the remaining kernel gap. DUCC clears exactly the active
coefficient span `m..=lmax`, matching Rust's staged output length.

The measured cost of `analysis_pair_group_state_in_range` is 2.277 µs at 64
lanes, 3.832 µs at 128, and 7.388 µs at 256. At 256 lanes this is 29.7% of
the current checked kernel time. Relative to the historical checked timing of
26.590 µs, the matched A/B difference is 27.9%; that percentage compares
different runs and is not a same-run decomposition of the historical sample.
The compile-time unchecked specialization removes the post-group state scan
from the optimized V3 loop. The checked assembly contains a separate vector
pass over the updated previous/current arrays using packed absolute-value
masks, `vcmpeqpd`, `vcmpge_oqpd`, and mask combination before continuing. The
unchecked specialization proceeds from coefficient reduction without this
pass. Entry checks, scaled transitions and scalar-tail checks remain enabled.

Checked and unchecked direct outputs matched exactly. The DUCC recurrence
coefficients had a maximum difference scaled by `max(1, |reference|)` of
`1.970e-16`; output relative L2 errors were
`6.670e-14`, `1.628e-13`, and `5.928e-14` at 64, 128, and 256 lanes, with
maximum absolute errors `5.009e-12`, `1.896e-11`, and `2.559e-11`. Release
tests passed for zero/cancellation inputs, SIMD tails, extreme finite products,
subnormal lanes, T255 DUCC coefficients, T511 GL DUCC/analytic sectoral
analysis, scalar adjoint identities and full T720 GL analysis with q1/q8.
The full T720 q1 and q8 checked runs recorded zero late group-range declines.
That result covers valid `fill_factors` coefficients at T720; the existing
artificial `1e-308` factor test remains a separate fallback-contract test.
The checked implementation remains the production default while the
unchecked route stays available for cumulative candidate evaluation.

The AMD Ryzen 9 5950X host supports AVX2/FMA and has 16 physical and 32 logical
cores reported, with 96 GiB installed RAM, running Ubuntu 26.04.1 on Linux
7.0.14-pve. Rust 1.99.0, LLVM 23.1.1, Pulp 0.22.3 and GCC 15.2.0 were used;
the Rust target was `-C target-cpu=x86-64-v3`, and DUCC used
`-O3 -DNDEBUG -march=x86-64-v3`. Linux cpufreq data for the pinned CPU policy
and process scheduling counters were available; sampled run-queue wait was
low in this run. Process medians for checked Rust and DUCC were stable. One
256-lane unchecked Rust process median was 18.287 µs, versus 17.470 and
17.445 µs in the other two runs. Several batches had large maxima, including
DUCC at 256 lanes; those outliers did not shift the DUCC process medians. Their
source is unknown and all samples remain in the reported distributions. All
1,620 measured kernel or reset-inclusive batches were 30.02–72.43 ms, within
the 20–100 ms target.
The prior 64-lane DUCC values of 9.599 and 5.621 µs and 256-lane values of
25–39 µs did not recur. Their exact cause cannot be recovered without
contemporaneous host measurements; the calibrated measurements replace them
for current comparisons.

## Fused group-range-check candidate — 2026-10-20

The candidate validates final recurrence states in the SIMD recurrence loop,
accumulating one lane mask across vector blocks and reducing it once per group.
It avoids the post-group reread of the `previous` and `current` arrays. It is
selected only by `rustsht_fused_analysis_pair_range_check`; ordinary builds
keep the existing checked post-group scan. The benchmark script enables the
candidate by default and accepts `RUSTSHT_KERNEL_BENCH_RUSTFLAGS` to compare
the post-group-scan route.

This run used the same pinned `m=1`, `lmax=720` fixture, Pulp V3/GCC AVX2/FMA
builds, CPU pinning, reset boundaries, and three-process, 30-batch protocol as
the previous section. Entries are median `(p10–p90; full range)` in µs/op.

| Mode | Lanes | Fused checked | Unchecked | DUCC |
|---|---:|---:|---:|---:|
| Kernel only | 64 | 8.626 (8.591–8.687; 8.547–8.751) | 4.704 (4.671–4.737; 4.656–4.775) | 4.104 (4.081–4.124; 4.059–4.147) |
| Kernel only | 128 | 16.784 (16.688–16.911; 16.625–17.003) | 8.745 (8.691–8.785; 8.636–8.828) | 8.052 (7.995–8.156; 7.949–8.526) |
| Kernel only | 256 | 32.977 (32.789–33.139; 32.564–33.298) | 17.652 (17.499–18.030; 17.355–18.310) | 15.969 (15.826–16.169; 15.697–16.297) |
| Reset inclusive | 64 | 8.757 (8.712–8.824; 8.662–8.874) | 4.777 (4.745–4.810; 4.729–4.850) | 4.210 (4.184–4.233; 4.168–4.264) |
| Reset inclusive | 128 | 16.909 (16.787–17.016; 16.680–17.116) | 8.810 (8.754–8.857; 8.707–8.907) | 8.205 (8.122–8.304; 8.091–8.490) |
| Reset inclusive | 256 | 33.093 (32.955–33.308; 32.666–33.367) | 17.659 (17.572–17.926; 17.422–18.268) | 16.177 (16.016–16.330; 15.808–16.692) |

The fused checked candidate was 21.1%, 32.6%, and 32.5% slower than the
post-group-scan checked medians in the previous run at 64, 128, and 256 lanes.
Its Rust/DUCC kernel-only ratios were 2.102/2.085/2.065. The unchecked
specialization remained 1.146/1.086/1.105× DUCC. Reset-inclusive ratios for
the fused checked candidate were 2.080/2.061/2.046×. All 1,620 timed batches
were 33.31–73.24 ms. Output errors remained at the levels recorded above, and
the candidate full release test suite passed. A T720 GL q1/q8 production-plan
counter run also recorded zero late group-range declines.

The exact optimized V3 candidate body inserts packed validity comparisons
while the recurrence and eight coefficient accumulators are live. In the
executed loop, disassembly shows a YMM stack store and reload (`vmovupd` and
`vandpd` through `0x100(%rsp)`). The previous checked loop has no YMM stack
operand in its recurrence loop. This register spill coincides with the 21–33%
regression. The fused check is retained only as an experimental compile-time
candidate; it does not replace the production scan.

### Scalar reduction variant — 2026-10-20

To remove the vector-mask accumulator that spilled in the first fused
candidate, this variant reduces each four-lane validity mask immediately into
a scalar flag. The checked and unchecked decisions match the scanned route on
the zero/cancellation, subnormal, and SIMD-tail fixtures.

The same pinned `m=1`, `lmax=720`, x86-64-v3, three-process, 30-batch protocol
measured these kernel-only medians (p10–p90) in µs/op:

| Lanes | Scalar-reduced check | Vector-mask check | Unchecked | DUCC |
|---:|---:|---:|---:|---:|
| 64 | 10.017 (9.952–10.098) | 8.626 (8.591–8.687) | 4.719 (4.690–4.736) | 4.150 (4.128–4.169) |
| 128 | 18.667 (18.553–18.763) | 16.784 (16.688–16.911) | 8.743 (8.685–8.769) | 8.059 (7.989–8.097) |
| 256 | 37.215 (37.017–37.350) | 32.977 (32.789–33.139) | 17.541 (17.440–17.633) | 15.888 (15.825–15.957) |

The per-vector reductions were 16.1%, 11.2%, and 12.9% slower than the
vector-mask candidate, and 41–50% slower than the earlier post-group scan.
They remove the vector-mask spill from the recurrence body but serialize a
mask reduction for every four-lane chunk. The unchecked path remained
1.137/1.085/1.104× DUCC. Reset-inclusive medians (scalar check/vector-mask
check/unchecked/DUCC) were 10.037/8.757/4.786/4.260, 18.664/16.909/8.807/8.208,
and 37.410/33.093/17.640/16.090 µs at 64/128/256 lanes; unchecked ratios were
1.123/1.073/1.096. All 1,620 batches measured 33.52–91.07 ms, with no batch
outside the 20–100 ms target. Numerical errors matched the preceding run. The
full release all-feature suite passed with this cfg, and valid T720 GL q1/q8
analysis still had zero late declines. This variant remains an experimental
compile-time candidate; production retains the post-group scan.

## Historical pair-grouping checkpoint

## Changes and validation

The scalar-analysis fast route now advances two insignificant recurrence pairs
and applies DUCC's rescaling threshold in one SIMD latitude traversal. Its
ordinary route also computes and accumulates two recurrence pairs per latitude
pass. The existing checked scaled fallback remains available when range
validation rejects the fast route. The two-pair accumulator performs its range
validation after the register-heavy accumulation pass.

The direct ordinary comparison used the Rust parent revision `d8b40fd` and the
candidate in this commit, against DUCC
`80388904b8cfc32400ba84af8a6a45aabfe60f86`. Both builds targeted AVX2/FMA:
Rust 1.99.0, LLVM 23.1.1, Pulp 0.22.3 with
`-C target-cpu=x86-64-v3`; DUCC used GCC 15.2.0 and
`-O3 -DNDEBUG -march=x86-64-v3`. The CPU was an AMD Ryzen 9 5950X (16 cores,
32 logical CPUs, AVX2/FMA), with 103 GB RAM, running Ubuntu 26.04.1 LTS.
Runtime Pulp selection and optimized disassembly confirmed AVX2/FMA execution.

The direct ordinary fixture used `m=1`, `lmax=720`, and 64 or 256 latitude
lanes. Each Rust process had one warmup and five timed calls; process medians
were summarized across three runs. DUCC had one warmup and nine timed calls per
run. Coefficient relative differences were at most `1.970e-16`; output relative
L2 differences were `6.670e-14` (64 lanes) and `5.928e-14` (256 lanes).

| Lanes | Rust parent | Rust candidate | Candidate / parent | DUCC median | Candidate / DUCC |
|---:|---:|---:|---:|---:|---:|
| 64 | 10.370 µs | 9.458 µs | 0.912× | 4.844 µs | 1.95× |
| 256 | 29.556 µs | 26.590 µs | 0.900× | 23.660 µs | 1.12× |

DUCC process medians ranged from 4.308–5.621 µs at 64 lanes and 20.258–32.211
µs at 256 lanes. That variation makes the cross-library ratios provisional,
especially at 256 lanes. The Rust candidate improved both lane sizes against
its parent, but this short screen does not establish stable DUCC parity.

The prefix comparison used a synthetic all-insignificant state (`m=1`,
`lmax=720`, zero previous state, `2^-100` current state, scale epoch `-1`).
Scale epochs matched DUCC exactly; state relative L2 differences were
`2.013e-13` (64 lanes) and `4.279e-14` (256 lanes).

| Lanes | Rust parent | Rust candidate | Candidate / parent | DUCC | Candidate / DUCC |
|---:|---:|---:|---:|---:|---:|
| 64 | 49.874 µs | 6.873 µs | 0.138× | 2.665 µs | 2.58× |
| 256 | 175.161 µs | 19.687 µs | 0.112× | 12.504 µs | 1.57× |

This synthetic prefix is useful for comparing the transition loop, not for
estimating its contribution to full transforms.

## Full-analysis screen and generated code

For f64 GL T720 (`721×1443`, full `mmax`), plans and workspaces were reused.
The five-call within-process medians gave Rust-only comparisons:

| Threads | Parent | Candidate | Candidate / parent |
|---:|---:|---:|---:|
| q1 | 33.298 ms median of 32.984/33.611 | 28.065 ms median of 27.719/28.411 | 0.843× |
| q8 | 5.957 ms | 5.258 ms | 0.883× |

These are full analysis timings, including FFT and scheduling; they are not
DUCC ratios. The q8 result has one process median per revision and should be
treated as a lightweight screen.

The Rust ordinary hot loop now has the same two-pair recurrence grouping as
DUCC's `map2alm_kernel`. Its inspected AVX2 loop contains eight coefficient
accumulation FMAs and four recurrence FMAs without YMM stack references. DUCC's
loop has the corresponding recurrence/accumulation structure and no YMM stack
traffic. Rust still performs a separate post-group range-check traversal, which
preserves subnormal fallback behavior and remains a candidate cost; its impact
has not been isolated. The direct 64-lane result still shows a substantial
gap, so simpler loop structure alone is not evidence of parity.

`cargo fmt --all -- --check`, all-target/all-feature RustSHT Clippy, release
RustSHT tests with all features, and release RustSHT tests without default
features passed. The direct Rust and DUCC fixture drivers are retained in
[`reports/data/scalar-analysis-pair-grouping-2026-10-19`](../../data/scalar-analysis-pair-grouping-2026-10-19).
No full DUCC transform timing or host performance counters were collected.

## Direct parity-spectrum packing candidate — 2026-10-20

The analysis packer now writes north-plus-south and north-minus-south
directly into the four real/imaginary SIMD input arrays. This removes the
intermediate complex buffers and their later copy. Compact north-ring indices
allow the checked path to reconstruct the unweighted opposite parity if the
preweighted fused route declines. The compaction test covers north-only,
south-only, paired and equatorial lanes, including fallback restoration.

A full T720 GL analysis screen used `m1b_bench`, reused plans and workspaces,
and ran five calls per process on the same pinned physical core set. Three
process invocations per revision were interleaved; map inputs were the same
deterministic nonzero field. Values below are medians of those process
medians, with the process-median range in parentheses:

| Threads | Before | Direct-SoA candidate | Candidate / before |
|---:|---:|---:|---:|
| q1 | 26.705 ms (26.580–26.877) | 26.851 ms (26.685–27.758) | 1.005× |
| q8 | 4.348 ms (4.335–4.355) | 4.327 ms (4.307–5.151) | 0.995× |

The q8 candidate range includes one slow process. The screen shows no reliable
latency change, so the layout change remains in the cumulative candidate but is
not reported as a performance win. The all-feature release suite passed,
including DUCC and analytic scalar-analysis references, adjoint identities,
f32 and extreme-value tests. These full-transform numbers include FFT and
scheduling; they are not direct Legendre timings.

## Production post-group scan A/B — 2026-10-20

This run explicitly selected the production post-group scan with
`RUSTSHT_KERNEL_BENCH_RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'`.
It used Rust 1.99, Pulp V3 AVX2/FMA, GCC 15.2, and pinned DUCC
`80388904b8cfc32400ba84af8a6a45aabfe60f86`. Rust and DUCC used four f64
latitude lanes. The five-process, 30-batch protocol pinned each single-thread
trial to one physical core and interleaved backend order. All 2,700 timed
batches were 33.51–82.40 ms. Checked and unchecked Rust coefficients matched
exactly; DUCC relative output L2 errors were `6.670e-14`, `1.628e-13` and
`5.928e-14` at 64, 128 and 256 lanes.

Kernel-only medians are µs per block; parentheses give p10–p90 across batches:

| Lanes | Rust checked | Rust unchecked | DUCC | Checked / unchecked / DUCC |
|---:|---:|---:|---:|---:|
| 64 | 7.067 (7.024–7.153) | 4.746 (4.717–4.797) | 4.129 (4.099–4.156) | 1.712× / 1.149× |
| 128 | 12.705 (12.608–12.869) | 8.782 (8.723–8.865) | 8.073 (8.000–8.197) | 1.574× / 1.088× |
| 256 | 24.790 (24.645–24.901) | 17.344 (17.221–17.451) | 15.860 (15.755–15.922) | 1.563× / 1.094× |

The post-group validation costs 2.321, 3.923 and 7.446 µs at the respective
lane counts. At 256 lanes it is 30.0% of checked Rust time and explains 83.4%
of the 8.930 µs checked-Rust excess over DUCC. The remaining unchecked excess
is 1.484 µs, or 9.4% of DUCC time. Reset-inclusive Rust/DUCC ratios were
1.687/1.135×, 1.559/1.081× and 1.550/1.087×; reset work is small relative to
the validation pass.

A full odd-longitude T720 GL (`721×1443`, full `mmax`) check used the same
deterministic nonzero map, reused plan/workspace, and five interleaved process
pairs with five calls per process. Medians of process medians were:

| Threads | Checked | Unchecked | Unchecked / checked |
|---:|---:|---:|---:|
| q1 | 26.932 ms (26.613–27.510) | 23.541 ms (23.430–23.784) | 0.874× |
| q8 | 4.415 ms (4.298–4.864) | 3.936 ms (3.879–5.159) | 0.892× |

The q8 unchecked range includes one slow process; its cause was not sampled.
Frequency and scheduling observations for the calibrated direct-kernel run
showed similar median clocks across backends and less than 0.05% run-queue wait.
The ignored T720 production counter test recorded zero late declines at q1 and
q8. The complete release suite passed both with the checked default and with
the unchecked specialization selected, including polar, subnormal, extreme,
zero/cancellation, SIMD-tail, analytic and adjoint cases. Keep the checked path
as production default while confirming the unchecked behavior across the
remaining high-order fallback cases.

## Calibrated grouped-reduction A/B — 2026-10-20

The Pulp candidate groups four accumulator vectors, transposes their AVX2
lanes, and sums matching degree/parity channels before storing the four scalar
results. It is selected only by `rustsht_group_analysis_reduction`; the
standard independent reductions remain the default. The baseline was
`90bdde14`; the candidate was `7cd69307`. Both used the same `m=1`,
`lmax=720` f64 fixture and pinned DUCC revision
`80388904b8cfc32400ba84af8a6a45aabfe60f86`. Rust 1.99.0 with Pulp 0.22.3
used `-C target-cpu=x86-64-v3`; DUCC used GCC 15.2 with
`-O3 -DNDEBUG -march=x86-64-v3`. Runtime dispatch selected Rust Pulp V3 and
both implementations used four AVX2 f64 lanes.

The comparison used five independent Rust/DUCC process pairs per lane count,
30 calibrated batches per process, alternating backend order and pinning all
single-thread trials to the same physical core. Each batch measured 20–100 ms;
baseline batches were 33.55–66.83 ms and candidate batches were 33.76–62.60
ms. Results are median µs per operation with p10–p90 batch ranges.
`kernel_only` excludes reset; `reset_inclusive` includes identical state and
coefficient reset work for both libraries.

| Lanes | Rust unchecked baseline | Grouped candidate | Candidate / baseline | DUCC baseline → candidate | Candidate / DUCC |
|---:|---:|---:|---:|---:|---:|
| 64 | 4.782 (4.752–4.809) | 5.444 (5.382–5.571) | 1.138× | 4.128 (4.097–4.144) → 4.125 (4.092–4.169) | 1.320× |
| 128 | 8.790 (8.740–8.829) | 9.403 (9.341–9.449) | 1.070× | 8.055 (8.005–8.089) → 8.049 (8.007–8.085) | 1.168× |
| 256 | 17.357 (17.253–17.419) | 18.206 (18.077–18.298) | 1.049× | 15.805 (15.707–15.893) → 15.849 (15.757–15.895) | 1.149× |

Reset-inclusive medians were 4.845/5.584/4.233/4.232 µs at 64 lanes,
8.874/9.473/8.196/8.189 µs at 128 lanes, and
17.446/18.318/16.005/16.043 µs at 256 lanes (Rust baseline/candidate,
DUCC baseline/candidate). Grouped reduction slowed the unchecked kernel by
13.8%, 7.0% and 4.9%. Its AVX2 output path adds packed transposes and stack
materialization/reloads, explaining why fewer independent horizontal
reductions did not improve latency.

The candidate passed the all-feature release suite (220 unit tests, four
extreme-value integration tests, 11 f32 tests and workspace compatibility),
all-target/all-feature Clippy and the focused grouped-sum test. DUCC output
relative L2 errors remained `6.670e-14`, `1.628e-13` and `5.928e-14`; the
maximum scaled coefficient difference was `1.970e-16`. Sampled median CPU
frequencies were 4.607 GHz for baseline and 4.617 GHz for candidate. Median
run-queue wait was 0% in both runs, with maxima below 0.05%.

The earlier single-call DUCC timings of 9.599/5.621 µs at 64 lanes and
25.318–39.165 µs at 256 lanes were not reproduced. Calibrated DUCC process
medians stayed between 4.125–4.128 µs at 64 lanes and 15.743–15.866 µs at 256
lanes. The old runs do not support a parity conclusion; current evidence does
not identify the exact cause of their process-level shifts.

## AVX2 horizontal-reduction A/B — 2026-10-20

The candidate applies DUCC's four-vector AVX2 horizontal-sum sequence to the
eight ordinary recurrence accumulators. It is selected only with
`rustsht_avx2_group_analysis_reduction`; the checked production build keeps
Pulp's individual reductions. The matched baseline was source `7cd69307`,
with the AVX2 candidate built from the same source plus the experimental cfg.
Rust used 1.99.0, Pulp 0.22.3 and `-C target-cpu=x86-64-v3`; DUCC remained
pinned to `80388904b8cfc32400ba84af8a6a45aabfe60f86`, compiled with GCC 15.2,
`-O3 -DNDEBUG -march=x86-64-v3`. Both selected four f64 latitude lanes.
Disassembly of the candidate V3 specialization confirmed `vhaddpd`,
`vperm2f128` and packed additions, without the Pulp candidate's output
partial-store call and stack reloads.

The benchmark used five independent Rust/DUCC process pairs per lane count,
30 calibrated batches per process, alternating backend order and pinning all
single-thread trials to one physical core. Every batch was in the 20–100 ms
window: 33.47–76.01 ms for baseline and 31.63–78.33 ms for candidate.
`kernel_only` excludes reset; `reset_inclusive` applies the same recurrence
state and coefficient reset for each implementation.

Kernel-only medians are µs per operation; unchecked Rust p10–p90 ranges are
shown in parentheses.

| Lanes | Checked baseline → AVX2 | Unchecked baseline → AVX2 | DUCC baseline → candidate |
|---:|---:|---:|---:|
| 64 | 7.066 → 7.087 | 4.797 (4.752–4.930) → 4.668 (4.628–4.750) | 4.152 → 4.138 |
| 128 | 12.645 → 12.815 | 8.814 (8.757–8.986) → 8.742 (8.674–8.842) | 8.062 → 8.053 |
| 256 | 24.834 → 25.091 | 17.412 (17.331–17.866) → 17.297 (17.182–17.556) | 15.868 → 15.872 |

Checked Rust/DUCC ratios changed from 1.702/1.568/1.565× to
1.713/1.591/1.581×. Unchecked ratios changed from 1.155/1.093/1.097× to
1.128/1.086/1.090×. Every unchecked candidate process median was below every
baseline process median. The candidate therefore saves
0.129/0.072/0.115 µs at 64/128/256 lanes, while its checked route is slightly
slower. Keep it opt-in until a larger combined improvement justifies the
checked-path cost; both checked and unchecked variants use the same candidate
reduction order so the fixture's exact-output assertion remains valid.

Reset-inclusive checked Rust medians changed 7.139→7.237, 12.742→13.019 and
24.924→25.319 µs. Unchecked medians changed 4.858→4.729, 8.910→8.833 and
17.506→17.406 µs. DUCC measured 4.260→4.246, 8.213→8.191 and 16.081→16.087
µs. CPU-frequency monitoring showed no material median shift between runs.
Median run-queue wait was zero, with maxima below 0.05%. One slow batch
appeared in each run, but the per-process medians remained consistent.

Maximum scaled coefficient difference from DUCC remained `1.970e-16`; relative
output L2 errors were `6.670e-14`, `1.628e-13` and `5.928e-14`. The candidate
passed the all-feature release suite with the experiment cfg, including exact
checked/unchecked fixture equality, polar, subnormal, extreme finite,
cancellation, SIMD-tail, analytic and adjoint tests. Default release tests and
all-target/all-feature Clippy also passed. The checked production ratios remain
1.702×/1.568×/1.565×; the unchecked AVX2 candidate is still 1.128×/1.086×/
1.090× DUCC.

## AVX2 lane-mask reduction A/B — 2026-10-20

Both builds used the source at `4fcb1136`, Rust 1.99, Pulp 0.22.3, the V3
AVX2/FMA dispatch, and the same `m=1`, `lmax=720` fixture. The Pulp build had
`rustsht_fused_analysis_pair_range_check` and
`rustsht_scalar_fused_analysis_pair_range_check`; the candidate added only
`rustsht_avx2_mask_all`. DUCC used the pinned revision and GCC 15.2 with
`-O3 -DNDEBUG -march=x86-64-v3`. Each run used five interleaved Rust/DUCC
process pairs and 30 batches per process. `kernel_only` excludes reset;
`reset_inclusive` applies the same state and coefficient reset to both.

Kernel-only medians are µs/op, with p10–p90 batch ranges in parentheses:

| Lanes | Pulp in-loop check | AVX2 mask check | DUCC | AVX2 / DUCC |
|---:|---:|---:|---:|---:|
| 64 | 9.985 (9.940–10.046) | 7.608 (7.563–7.807) | 4.113 (4.086–4.137) | 1.850× |
| 128 | 18.901 (18.792–20.082) | 14.224 (14.122–14.582) | 8.045 (7.997–8.134) | 1.768× |
| 256 | 37.553 (37.359–38.518) | 28.573 (28.382–29.670) | 15.857 (15.703–20.026) | 1.802× |

The AVX2 mask route reduced the fused in-loop check by 23.8%, 24.7% and
23.9%. Its optimized loop converts each four-lane range mask to a scalar
validity flag with `vtestpd`; the Pulp version carries a vector mask through
the group and reduces it afterward. Reset-inclusive medians were
10.091/18.908/37.646 µs for the Pulp check, 7.888/14.327/28.732 µs for AVX2,
and 4.223/8.180/16.060 µs for DUCC. Numerical errors stayed at the established
levels (`1.970e-16` maximum scaled coefficient difference and
`6.670e-14`/`1.628e-13`/`5.928e-14` relative output L2).

This fused candidate remains slower than the production post-group scan,
whose prior calibrated medians were 7.067/12.705/24.790 µs: the AVX2 fused
check is 7.7%, 12.0% and 15.3% slower. Do not select it in production. The
DUCC 256-lane candidate run had one process median near 20.0 µs while the other
four stayed near 15.8 µs. Available frequency and scheduler summaries did not
explain that process shift; the aggregate median was unaffected. Baseline and
candidate batches were 33.62–87.00 ms and 33.32–94.31 ms, with none outside
the 20–100 ms target.

The AVX2 mask helper was also tested on the production post-group path with
only `rustsht_avx2_mask_all` enabled. Relative to the calibrated Pulp scan,
kernel-only medians changed from 7.067/12.705/24.790 µs to
6.995/12.559/24.726 µs at 64/128/256 lanes (−1.0%/−1.1%/−0.3%). Those small
changes are within the observed batch variation and do not establish a win.
DUCC medians were 4.130/8.060/15.864 µs; reset-inclusive Rust/DUCC medians
were 7.087/4.237, 12.656/8.202 and 24.828/16.070 µs. All 2,700 batches were
33.09–74.11 ms. The full release suite and candidate Clippy passed. Keep the
helper gated for analysis; leave production on the Pulp check.

## Next implementation target

Reduce the production post-group validation cost without carrying a lane mask
through the eight-accumulator recurrence loop. The calibrated checked path
remains about 1.56× DUCC at 256 lanes; the AVX2 mask helper did not materially
change it. Keep the checked scan and scaled fallback while testing a source-
grounded reduction in repeated state traffic.
