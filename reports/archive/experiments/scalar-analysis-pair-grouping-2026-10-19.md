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
