# Plan 16: executed spin-1 synthesis crosswalk

**Milestone A complete.** This note records the production paths and a focused
stage profile before changing the paired recurrence. It is not a new full
benchmark campaign.

## Matched workload and sources

- RustSHT source: `6381edbc3f8c4ffb33579f115de195c8e8e2f00f`; the spin-transform
  implementation is byte-identical to the Plan 15 closeout source
  `ae78e18044fea2d65316d230dd434e4b4f8222be`.
- DUCC: 0.42.0, pinned source
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust 1.99.1 / LLVM 23.1.1, runtime Pulp V3 (four f64 lanes); DUCC GCC 15.2,
  SHT numerical sources `-O3 -ffast-math -ffp-contract=fast
  -march=x86-64-v3`, driver strict FP.
- Exact T720 CC: `ntheta=722`, `phi0=0.23`; full order uses `nphi=1458`,
  `mmax=180` uses `nphi=378`. The previous test helper's defaults used
  `nphi=1442` even for reduced order. Those mismatched runs were discarded.

## Executed algorithm paths

Rust's `synthesis_spin_core_with_hemisphere` uses ring groups. At q1 it converts
E/B to two scalar rows per order in worker scratch. At q8 it prepares and
finite-checks the converted rows once in the workspace, then dynamically
queues 64-ring groups (12 groups at T720 CC) across workers. Each order/group
fills recurrence factors; each at-most-64-ring block prepares even/odd complex
coefficient streams for both scalar rows and calls
`FusedPreparedSynthesisPairBlock`. That kernel keeps latitude lanes outermost,
advances one shared paired recurrence, and forms `even + cos(theta)*odd` for
each channel inside every degree-pair step. The caller then applies `1/sin(theta)`
and writes ring-major spectra. Near-polar or order-excluded blocks use the
checked general spin recurrence. Two inverse row FFT passes produce Q and U.

DUCC `synthesis()` calls `alm2leg` followed by `leg2map`; exact CC sampling is
not downsampled. For T720 STANDARD spin 1, `alm2leg` sorts rings and splits at
`|sin(theta)|=1e-4`: ordinary rings are converted by `spin1to0`, normalized,
and passed through `inner_loop_a2m` twice with scalar `s0data` SIMD lanes; the
two exact polar rings take DUCC's direct-spin path. `fill_a2m` prepares
`cos(theta)^2` and separates the two parity channels in its lane data;
`calc_alm2map` advances the paired scalar recurrence and accumulates the
channels independently before extraction. `leg2map` performs the longitude
FFT. This is a scalar-converted bulk path plus a small direct-spin polar path,
not a single direct-spin kernel over all rings.

The remaining candidate mechanism is therefore specific: Rust recomputes the
cosine-weighted coefficient combination at every degree pair, while DUCC keeps
the two parity accumulators separate through the recurrence. Reversing the
latitude/degree loop is not part of this candidate; that earlier variant
spilled state and regressed.

## Focused Rust stage profile

The ignored test-only phase probe used three warmups and five measured calls,
pinned to one physical core for q1 and eight physical cores in one L3 domain
for q8. Stage values below are medians in
milliseconds. The phase probe adds instrumentation and is used for boundaries
and route attribution, not as the native benchmark's performance claim.

| T720 CC | q | setup | converted-row cache | order / recurrence / spectrum | inverse rows / writeback |
|---|---:|---:|---:|---:|---:|
| full, `mmax=720`, `nphi=1458` | 1 | 0.514 | n/a | 95.299 | 4.756 |
| full, `mmax=720`, `nphi=1458` | 8 | 0.634 | 2.050 | 21.041 | 1.190 |
| reduced, `mmax=180`, `nphi=378` | 1 | 0.069 | n/a | 45.819 | 1.073 |
| reduced, `mmax=180`, `nphi=378` | 8 | 0.103 | 0.623 | 17.541 | 0.191 |

For the full q8 profile, each measured call had 5,296 paired-eligible blocks,
5,028 successful paired blocks and 1,589 general-fallback blocks. Reduced
q8 had 1,787 eligible, 1,738 successful and 306 fallback blocks. These are
route counts, not latency fractions. Worker-summed inner timers overlap across
q8 workers and are not interpreted as wall time. The q8 end-to-end stage
boundary still places most Rust time in order recurrence/spectrum rather than
the inverse rows; the q1 profile shows the same ordering.

The Plan 15 matched end-to-end reference at this source boundary measured
q8 Rust/DUCC `23.596/6.087 ms` full and `17.179/2.647 ms` reduced. Those
measurements and the new instrumented phase profile have different boundaries
and are not combined into a synthetic ratio. Existing numerical checks for
this source family passed with maximum DUCC relative L2 error `3.64e-13` in
the CC coefficient-reuse comparison. The next change is the bounded
parity-separated two-channel candidate; no production default changes in
Milestone A.

## Reproduction

```bash
# Set Q8_CPUS to eight physical CPUs in one L3 domain on the test host.
RUSTSHT_PROFILE_GRID=CC RUSTSHT_PROFILE_LMAX=720 RUSTSHT_PROFILE_MMAX=720 \
RUSTSHT_PROFILE_NPHI=1458 RUSTSHT_PROFILE_THREADS=8 \
RUSTSHT_PROFILE_REPETITIONS=5 taskset -c "$Q8_CPUS" \
cargo test --release --lib --features parallel \
  profile_spin1_standard_synthesis_phases -- --ignored --nocapture
```

Use `mmax=180`, `nphi=378` for the reduced control, and `THREADS=1` with a
single physical CPU affinity for q1. Pinned DUCC source crosswalk: `sht.cc::synthesis`,
`sht.cc::alm2leg`, `sht.cc::spin1to0`, `sht_inner_loop.h::inner_loop_a2m`,
`fill_a2m`, `calc_alm2map`, and `sht.cc::leg2map`.
