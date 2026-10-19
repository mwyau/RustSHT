# Scalar analysis AVX2 code generation checkpoint

## Scope

The clean Rust baseline was commit `0cccda66`. The candidate changes only the
fused scalar-analysis accumulator in `scalar_paired.rs`. Rust used 1.99.0,
LLVM 23.1.1, release mode, and `-C target-cpu=x86-64-v3` on an AMD Ryzen 9
5950X with AVX2/FMA. The comparison build for DUCC's pinned
`80388904b8cfc32400ba84af8a6a45aabfe60f86` kernel used GCC 15.2 and optimized
AVX2/FMA code. The installed comparison library and newer DUCC checkout were
not used for this assembly comparison.

The Rust ordinary recurrence already emitted FMA instructions. Its four
SIMD coefficient accumulators emitted separate `vmulpd` and `vaddpd`
instructions. The pinned DUCC `map2alm_kernel` emitted `vfmadd231pd` for the
corresponding ordinary accumulations. Rust now uses `mul_add_f64s` for the
same-parity channels and for opposite-parity channels after cosine
preweighting. The unweighted fallback retains its existing multiplication
order. This removes one vector instruction per accumulated channel and uses
the same fused operation order as DUCC's optimized path.

## Bounded timing screen

Each Rust call reused its plan and workspace. The screen used one warmup and
five timed calls per process; two launch batches were collected for each
revision. Values below pool the ten call samples. They are full analysis
latencies, including FFT and scheduling, rather than isolated Legendre times.
The workload was f64 scalar GL analysis with `phi0=0.37`; T720 used full
`mmax` and 721x1442 sampling, while T300 used `mmax=220` and 301x603 sampling.

| Workload | q | Baseline median (range) | FMA median (range) | Change |
| --- | ---: | ---: | ---: | ---: |
| T720, full `mmax` | 1 | 31.071 ms (30.788–31.535) | 30.315 ms (28.677–32.453) | -2.4% |
| T720, full `mmax` | 8 | 5.583 ms (5.162–6.078) | 5.361 ms (4.764–5.755) | -4.0% |
| T300, `mmax=220` | 1 | 4.127 ms (3.988–4.539) | 3.859 ms (3.740–4.008) | -6.5% |
| T300, `mmax=220` | 8 | 0.797 ms (0.764–0.851) | 0.842 ms (0.774–0.927) | +5.7% |

The small T300 q8 case has wide, overlapping ranges and does not establish a
regression. The other medians favor FMA, but the spread also shows host noise.
No hardware performance counters were collected. This screen does not
establish a new Rust/DUCC latency ratio; the matched stage results in
`scalar-analysis-stage-profile-2026-10-18.md` use a different Rust revision.

## Remaining execution differences

The Rust fused loop advances and validates one coefficient pair at a time.
Pinned DUCC advances two pairs (four degrees) per SIMD latitude pass. Rust
also retains per-pair recurrence range checks so an unsafe block can fall
back without committing partial coefficients. These checks and loop grouping
remain likely contributors to the recurrence gap; the previous two-pair
prototype regressed and is not restored. The Rust block has a 1208-byte stack
frame and a per-block 512-byte cosine-square array, while the inspected DUCC
kernel has a 416-byte frame and reads prepared `csq` data. The frame sizes
alone do not establish a performance cause; these are candidates for targeted
follow-up inspection.

## Validation

On the clean `0cccda66` source plus this isolated change,
`cargo fmt --all -- --check` and release clippy passed. The complete release
Rust library suite passed: 213 tests passed and 15 remained ignored. This
includes the scalar-analysis DUCC reference, analytic, adjoint, mixed-scale,
tail, polar, and exceptional-value tests.
