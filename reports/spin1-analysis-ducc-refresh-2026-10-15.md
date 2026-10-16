# f64 spin-1 T511 analysis timing refresh

Two follow-up five-pair screens measured native RustSHT at `1.30–1.31x` the
DUCC Python operation at q1 and `1.77–1.82x` at q8. In the second follow-up,
RustSHT medians were `32.995 ms` and `6.345 ms`; DUCC medians were
`25.353 ms` and `3.434 ms`. The paired ranges were narrow in that screen. The
native RustSHT reusable-plan call and DUCC Python `analysis_2d` call have
different boundaries, so these ratios describe this benchmark setup. An earlier screen on 8 pinned CPU cores gave q1 `1.28x` and q8 `3.07x`; RustSHT q8
process medians ranged from `8.394` to `18.434 ms`. Two later screens used
8 pinned CPU cores for q8 and recorded lower host load. Their q8 ratios were similar
(`1.77x` and `1.82x`). The DUCC times in these follow-ups are close to the
`27.862/4.147 ms` q1/q8 Python-plan values in the [October 12 milestone
record](../plans/05-validation-and-milestones.md#comparative-spin-1-analysis-profile-and-polar-cutoff-2026-10-12),
and differ from the `13.056/1.943 ms` values in the [October 14
thread-scaling report](spin1-analysis-thread-scaling-2026-10-14.md). The
cross-session difference remains unresolved; this refresh records the current
screen results and leaves the earlier report intact.

## Workload and method

The operation was f64 GL T511 STANDARD analysis on a `512x1025` grid with
`mmax=511`, `phi0=0.37`, and deterministic full-band map input. RustSHT used a
reused native plan and workspace from source commit `7ac443a40b034d5c52526d23ef30f6cef4b13d19`. DUCC used `ducc0==0.41.0`, source revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`, through the Python `analysis_2d`
call. Each process warmed once and recorded five inner samples; each DUCC
process used two five-sample batches. Five process pairs alternated RustSHT
and DUCC at each thread count. The initial corrected screen pinned q1 to one CPU core and q8 to 8 pinned CPU cores. Host-load variation was reviewed locally; exact transient load measurements remain untracked. The follow-ups pinned q1
to one CPU core and q8 to 8 pinned CPU cores. Load-average telemetry was retained only in local diagnostics. The CPUs were not isolated. A cgroup check after
the screens reported `cpu.max=max 100000`, so no CPU quota applied.

| Screen | Threads | RustSHT median (range) | DUCC median (range) | Paired ratio median (range) |
| --- | ---: | ---: | ---: | ---: |
| Initial corrected | 1 | `33.793 ms` (`33.641–34.833`) | `26.361 ms` (`25.824–27.597`) | `1.28x` (`1.26–1.31x`) |
| Initial corrected | 8 | `15.376 ms` (`8.394–18.434`) | `4.446 ms` (`4.135–5.251`) | `3.07x` (`2.02–4.15x`) |
| Follow-up 1 | 1 | `33.071 ms` (`32.818–33.695`) | `25.534 ms` (`25.368–32.366`) | `1.30x` (`1.01–1.32x`) |
| Follow-up 1 | 8 | `6.099 ms` (`5.958–9.533`) | `3.437 ms` (`3.350–3.850`) | `1.77x` (`1.76–2.48x`) |
| Follow-up 2 | 1 | `32.995 ms` (`32.795–34.023`) | `25.353 ms` (`25.230–25.560`) | `1.31x` (`1.29–1.33x`) |
| Follow-up 2 | 8 | `6.345 ms` (`5.953–6.612`) | `3.434 ms` (`3.373–3.639`) | `1.82x` (`1.73–1.89x`) |

The host was an x86-64 CPU running Linux. Rust was
`1.99.0` with LLVM `23.1.1`; release builds used portable Pulp runtime
dispatch and no `target-cpu=native`. Python was `3.11.15` with NumPy `2.4.6`. Raw inner samples, process timings, load snapshots, command lines, binary
hashes and the cgroup check are in [`the benchmark data`](data/spin1-analysis-ducc-refresh-2026-10-15/). The `preliminary-*` files record an earlier run whose native q1 arguments were
incorrect. The first corrected q8 screen had wide variation; the two
follow-ups used a separate CPU set and showed similar paired ratios. No Rust
source or transform behavior changed during these measurements. Existing
spin-1 analytic, DUCC, adjoint and high-bandlimit tests remain the numerical
evidence; this timing study did not rerun them. The follow-up ratios describe these CPU affinities and the native/Python call
boundaries. A future comparison should record both in the same screen before
revising the earlier thread-scaling report.
