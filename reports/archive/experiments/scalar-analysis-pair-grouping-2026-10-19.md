# Scalar-analysis SIMD pair-grouping checkpoint

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
