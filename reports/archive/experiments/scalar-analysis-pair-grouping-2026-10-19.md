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

## Next implementation target

Evaluate the cumulative 256-pair and direct-SoA candidate against the unchecked
ordinary recurrence using the calibrated direct-kernel protocol and full T720
q1/q8 checks. Keep the checked fallback contract until valid GL, polar,
subnormal and extreme-value cases establish that removing the late scan is
safe for production inputs.
