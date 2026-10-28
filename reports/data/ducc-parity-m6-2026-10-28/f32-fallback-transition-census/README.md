# Native-f32 scalar synthesis fallback census — 2026-10-28

**Decision:** do not implement deferred scale across stable-exponent fallback
runs or an all-lane normal exponent-adjust packet from this evidence. The
first would cover under 1% of current scalar-fallback slots after exact route
intersection; the second is safe for only two full-mmax packets and no reduced
packets. No production source changed. All four GL T720 f32 scalar-synthesis
cells remain **GAP** against pinned DUCC.

## Source and measurement

- Repository HEAD: `bb53b86dc4ed47a9fb2cc01fd06b4e5d8626a7f6`.
- The measured production transform code is unchanged from
  `54b166f2c0698f1c70e88f8679d12fa4376704fe`.
- Pinned DUCC: 0.42.0,
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host/compiler: AMD Ryzen 9 5950X, rustc 1.99.1 / LLVM 23.1.1. Rust used
  `--locked --release`, `parallel`, the census-only feature and
  `-C target-cpu=x86-64-v3`.
- Fixtures: GL T720, `ntheta=721`, `phi0=0.23`; full
  `mmax=720, nphi=1458`; reduced `mmax=180, nphi=378`. Input hashes are in
  [`fixture-provenance.json`](fixture-provenance.json).
- One instrumented first transform per cell; q1 pinned to CPU 3 and q8 to
  CPUs 8–15. No timed candidate, whole-transform timing or Callgrind run was
  made. The counters duplicate route checks and are not performance
  measurements. Build and scratch-only patch hashes are in
  [`provenance.json`](provenance.json) and
  [`instrumentation.patch.gz`](instrumentation.patch.gz).

The census re-evaluates the existing checked packet's exponent range,
coefficient, basis, product, scaled contribution and next-sum conditions. It
counts full SIMD packets, so one unsafe lane sends the whole packet through
the current scalar callback. Stable-exponent runs are counted separately;
those totals include terms already accepted by the existing SIMD packet leaf
and must not be interpreted as removable fallback work.

## Exact scalar-fallback overlap

Counts are paired-degree / latitude accumulator slots. “Deferred safe” means
every term in a contiguous scalar-fallback run had normal coefficient, basis,
product and unscaled running sum, and applying the exponent once to the
aggregate produced normal values and finite staged sums. This is a conservative
counter-level screen, not a candidate correctness proof.

| GL T720 f32 scalar synthesis | Pair slots | Scalar-fallback slots | Fallback slots in packets with an exponent outside `[-126,127]` | Deferred-safe slots | Safe segments | Safe deferred share of fallback | Normal exponent-adjust packets | Underflow-range fallback packet terms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Full q1 | 80.70M | 3.730M | 3.570M | 24,896 | 944 | 0.668% | 2 (16 slots) | 466,210 |
| Full q8 | 80.70M | 5.203M | 4.996M | 35,056 | 1,302 | 0.674% | 2 (16 slots) | 650,333 |
| Reduced q1 | 40.64M | 1.338M | 1.283M | 12,968 | 372 | 0.969% | 0 | 167,308 |
| Reduced q8 | 40.64M | 1.661M | 1.594M | 16,120 | 467 | 0.970% | 0 | 207,655 |

More than eight fallback terms occurred in 15,223 / 20,397 full and 5,369 /
6,305 reduced q1/q8 runs. Those runs still failed at least one safety check;
the safe subset remains below 1% of scalar-fallback slots. The broad census
also found 46.37M / 45.13M full and 19.74M / 18.71M reduced q1/q8 slots in
exponent-stable runs across all routes. These totals include work already
handled by the checked SIMD leaf. A corrected odd/even final-degree condition
is included in the preserved instrumentation; earlier 27–28M all-run totals
missed final runs when the last visited degree was `lmax-1`.

Every counted scalar-fallback packet had normal coefficients, basis values and
products. All but two full-mmax packets then put at least one scaled result
below the normal exponent range (subnormal or zero); no packet put a result
above the normal range. Only two full-mmax packets met the stricter all-lane
normal exponent-adjust check, and neither reduced cell had one. A normal-range
vector leaf would therefore leave nearly all of the measured scaled fallback
work on the existing exceptional path. Deferred aggregation has the same
coverage problem: its longer raw runs generally fail when applying the scale.
Neither route justifies an implementation. Polar, synthetic subnormal,
nonfinite and overflow controls were not part of this random-fixture census;
the production exact fallback remains responsible for them.

## q8 work distribution

All eight workers ran the 23 scheduled jobs. Job counts ranged from 2 to 5
per worker full and 1 to 6 reduced. Instrumented
paired-accumulation work ranged from 4.69M to 20.40M slots per worker full
and 1.54M to 10.97M reduced. Worker nanoseconds are retained in
[`q8-worker-profile.csv`](q8-worker-profile.csv), but include the added route
classifier and atomic counters; do not use them to infer production load
imbalance. The accepted 32-ring schedule is unchanged.

The existing matched source profile remains the timing reference: f32 scalar
synthesis is `36.591×` / `40.257×` DUCC full q1/q8 and `25.432×` / `45.090×`
reduced q1/q8; the scaled-component helper accounts for 2.726B self Ir in the
matched q1 profile. Those are from the earlier exact-source timed/profile
screens, not this instrumentation run. See the
[single active ledger](../../../DUCC_PARITY_M6.md), the
[packet-suffix report](../f32-packet-suffix-candidate/README.md), and the
[f32-I/O / f64-internal diagnostic](../f32-internal-double-current-source/README.md).

## Next measured hypothesis

The current scalar-fallback terms cannot safely enter the native-f32 normal
suffix in useful numbers. The larger remaining measured algorithmic difference
is recurrence work: Rust updates about 80.31M / 40.51M paired states full /
reduced, while pinned DUCC updates about 40.32M / 20.31M northern states and
reconstructs the reflected GL outputs. A reflected f32 synthesis candidate
previously cut whole-call time substantially but regressed protected reduced
f32 scalar-analysis q8 by 4.3%; do not repeat it unchanged. Continue with the
other current-source native-f32 scalar/spin cells, then investigate a synthesis
route that preserves analysis behavior before returning to smaller f64 gaps.

The compact route table is in
[`fallback-route-summary.csv`](fallback-route-summary.csv), per-worker rows in
[`q8-worker-profile.csv`](q8-worker-profile.csv), all 51 raw counters per cell
in [`raw-census.json`](raw-census.json), and compressed original logs under
[`raw/`](raw/). `artifact-checksums.json` verifies the saved evidence.
