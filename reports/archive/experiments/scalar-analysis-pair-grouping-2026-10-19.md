# Scalar-analysis SIMD pair-grouping checkpoint

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
target. Each measured batch was 33.91–59.59 ms; all 1,620 measured batches
were within 20–100 ms.

The `kernel_only` boundary restores recurrence inputs and clears output outside
the per-operation timer. The `reset_inclusive` boundary includes those copies,
the output clear and the kernel. Coefficient preparation, ring packing, FFT,
scheduling and complete transforms are excluded from both. Both adapters
prevent dead-code elimination and compare final results outside timed regions.

| Lanes | Rust checked | Rust unchecked | DUCC | Checked / DUCC | Unchecked / DUCC |
|---:|---:|---:|---:|---:|---:|
| 64 | 7.047 µs (IQR 7.027–7.068) | 4.784 µs (4.772–4.794) | 4.126 µs (4.112–4.139) | 1.708× | 1.160× |
| 128 | 12.657 µs (12.627–12.676) | 8.822 µs (8.801–8.844) | 8.107 µs (8.091–8.131) | 1.561× | 1.088× |
| 256 | 24.826 µs (24.750–25.996) | 17.416 µs (17.353–17.681) | 15.874 µs (15.789–15.927) | 1.564× | 1.097× |

Reset-inclusive medians were 7.114/4.848/4.236 µs (64 lanes),
12.745/8.915/8.253 µs (128), and 24.977/17.831/16.071 µs (256), in
checked/unchecked/DUCC order. Reset-inclusive timing exceeds kernel-only timing
by 0.06–0.42 µs per call; reset work does not explain the remaining kernel
gap. DUCC clears exactly the active coefficient span `m..=lmax`, matching
Rust's staged output length.

The measured cost of `analysis_pair_group_state_in_range` is 2.263 µs at 64
lanes, 3.835 µs at 128, and 7.410 µs at 256. At 256 lanes this is 29.8% of
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
and process scheduling counters were available. One 64-lane Rust process
showed elevated sampled run-queue wait, while its latency median matched the
other invocations.
At 256 lanes, the third Rust checked process median was 26.061 µs, versus
24.781 and 24.777 µs in the first two runs; its unchecked median was 17.748 µs,
versus 17.401 and 17.360 µs. The cause of this lane-specific increase is
unclear, so all samples remain in the reported distributions. DUCC process
medians stayed within 0.9% across the 256-lane runs. All 1,620 measured kernel
or reset-inclusive batches were 33.60–63.13 ms, within the 20–100 ms target.
The prior 64-lane DUCC values of 9.599 and 5.621 µs and 256-lane values of
25–39 µs did not recur. Their exact cause cannot be recovered without
contemporaneous host measurements; the calibrated measurements replace them
for current comparisons.

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

## Next implementation target

Measure and reduce the ordinary kernel's post-group validation traversal while
retaining the checked fallback contract. Then repeat the same direct comparison
and full T720 q1/q8 screen before deciding whether the residual gap comes from
range-safety work, instruction scheduling, or another stage.
