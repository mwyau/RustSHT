# GL T720 native-f32 / f64-internal diagnostic — 2026-10-28

**Status:** deterministic work census and assembly inspection complete; matched latency comparison is pending. No production source changed. This diagnostic does not promote a precision change or establish parity. The route interpretation below was written before a later real-state replay found that the exact-zero suffix candidate could underflow nonzero recurrence states; current-source f32-I/O/f64-internal timings are now the next diagnostic.

## Provenance and builds

- RustSHT source: `102675a171f59faeab93fece11817ab589bbacc8`; `src/`, `Cargo.toml`, and examples are unchanged from production source `061a0467c506ae1711d00b952dd000c48d5e9cab`.
- Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host: AMD Ryzen 9 5950X; rustc 1.99.1 / LLVM 23.1.1; GCC 15.2.0; x86-64-v3, AVX2 and FMA. Rust used `-C target-cpu=x86-64-v3`; DUCC used the pinned benchmark flags `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, with the driver compiled using `-fno-fast-math`.
- Fixture dimensions: GL T720, `ntheta=721`, `phi0=0.23`; full `mmax=720`, `nphi=1458`; reduced `mmax=180`, `nphi=378`. Input hashes are in [work-counts.json](work-counts.json).
- The f64-I/O diagnostic worker reads the exact f32 coefficient fixture, promotes each component to f64, runs `ShtPlan<f64>`, and casts the result back to f32. It was built in two routes: the normal f64 reflected/order-major route, and a forced shared paired route. Both are diagnostic controls only.
- The Rust and DUCC counter patches are preserved in [rust-instrumentation.patch](rust-instrumentation.patch) and [ducc-instrumentation.patch](ducc-instrumentation.patch). The profiling runner and raw first-call profiles are in [collect_profile.py](collect_profile.py) and [profile-counts-contended.json](profile-counts-contended.json).

## Work census

Rust's current native-f32 paired route processes both GL hemispheres. On the matched random fixtures, it has 161.20M / 81.21M active degree×latitude coefficient slots for full/reduced `mmax`, 80.70M / 40.64M paired accumulation slots, and 80.31M / 40.51M recurrence state transitions.

DUCC's `fill_a2m` / `extract_a2m` groups reflected ring partners and computes one representative hemisphere. It evaluates 80.73M / 40.66M degree×latitude terms for full/reduced `mmax`, then writes the reflected partner. DUCC's useful recurrence transitions are 40.32M / 20.31M, about half RustSHT's current count. The suffix vectors add only 0.60–0.66% padded recurrence lanes.

Across all active DUCC `calc_alm2map` blocks, the counters report zero skipped-prefix contributions and zero scaled-prefix contributions. Every counted contribution proceeds through `alm2map_kernel`; the normal random fixture does not exercise DUCC's scaled contribution loop. Thus the large disparity has at least two demonstrated sources: RustSHT computes both hemispheres, and its native-f32 path keeps doing checked packet/scalar scaled accumulation across degree pairs instead of handing off once to a bulk normal-range kernel. The prior suffix visitor did not produce a repeatable whole-transform gain, so these counts do not justify restoring it unchanged.

The previous report's 80.70M “degree–latitude contributions” figure was in paired-accumulation-slot units, with each slot covering up to two degree terms. This census records degree terms, pair slots, and recurrence updates separately to avoid conflating them.

### Packet-level transition census

A second isolated profile at the same production source inspected every AVX2 f32 packet in the existing scaled fallback. At each paired degree it checked both recurrence states in all eight lanes, applying the shared lane exponent by exact exponent-field adjustment and testing the same `[SMALL, BIG]` range as the existing checked fused kernel. It recorded the first safe pair, the individual coefficient-degree terms remaining, and whether any later pair left that state range. This is a deterministic route census, not a timed build. Its counters reconcile exactly with the prior q1/q8 useful-work totals.

| mmax / threads | Fallback packets with a safe transition | Remaining state-safe degree×latitude terms | Share of all active Rust degree×latitude terms | Packets later out of range | First safe pair index (mean / p50 / p90) |
|---|---:|---:|---:|---:|---:|
| 720 / q1 | 28,439 / 29,452 (96.6%) | 75,140,792 | 46.6% | 0 | 32.3 / 15 / 90 |
| 720 / q8 | 26,881 / 28,644 (93.8%) | 67,669,424 | 42.0% | 0 | 42.7 / 23 / 114 |
| 180 / q1 | 7,595 / 7,972 (95.3%) | 33,370,352 | 41.1% | 0 | 34.2 / 13 / 96 |
| 180 / q8 | 7,171 / 7,592 (94.5%) | 30,119,760 | 37.1% | 0 | 44.9 / 19 / 132 |

Every eligible packet's later recurrence states remained in the checked range through `lmax` on these random fixtures. The suffix count uses individual degree terms, including an odd final degree; a paired accumulation slot may cover two terms. It counts only the fallback packets entering this visitor. Polar-mixed blocks and non-vector tails remain on their existing fallback. The 32-ring q8 scheduling splits groups differently from q1, so its packet-route totals differ even though the mathematical transform work is identical.

This establishes substantial *recurrence-state* transition coverage for a packet-level handoff. It does not prove coefficient products or accumulated sums safe, and it does not measure candidate speed. A production candidate still needs dynamic guards and transactional restore to the original scaled route if the fused suffix declines, including subnormal, nonfinite and overflow cases. The census is preserved in [transition-profile.json](transition-profile.json), with the complete temporary instrumentation in [transition-instrumentation.patch](transition-instrumentation.patch).

### Work interpretation and candidate direction

DUCC's `make_ringdata` pairs reflected GL rings (`idx`/`midx`). One recurrence packet update maintains parity-separated accumulators, and `extract_a2m` reconstructs both output rings. Its 80.73M / 40.66M full/reduced degree updates therefore cover approximately twice as many output ring-degree slots. Rust updates the recurrence for all 721 rings separately. This is a second substantial algorithmic difference beside Rust's repeated scaled-contribution path; it is separate from instruction-codegen differences. The current-source Callgrind ratio is about 28× in instruction work per output degree slot, not an elapsed-time or cycle ratio.

At the time of this census, its state-range counts appeared to support one bounded packet-wise prefix-to-fused-suffix candidate. They were only a potential route count and did not prove that the scaled state remained representable after applying its exponent. A later real q1 packet replay at production source `54b166f2` found nonzero states with exponents `-160..-320` that scaled to zero while the scalar scaled contribution path still produced nonzero values. Under a diagnostic guard against that loss, no nonzero-exponent suffix packet completed. The prior block-wide and packet-suffix handoffs remain rejected; do not treat these state counts as proof of equivalent native-f32 execution. The f64-internal diagnostic now needs a current-source matched latency comparison to distinguish exponent-range/codegen cost from the remaining algorithmic work. Its earlier first-call stage durations remain exploratory and were collected under contention.

## DUCC path and emitted AVX2 code

DUCC's `init_lambda` seeds double recurrence values and a scale epoch. `iter_to_ieee` advances the prefix while all packed ring lanes remain below its cutoff. `calc_alm2map` checks the scale state, accumulates any still-scaled prefix with per-lane correction, rescales the two live states once, clears the epoch, and calls `alm2map_kernel` for the remaining degree range. On this fixture the scaled branch is not entered.

The executed `alm2map_kernel` binary uses four packed f64 lanes per AVX2 vector and unrolls the degree loop. The body has `vfmadd*pd` recurrence and accumulation operations, unit-stride state/output stores, and no helper call in the hot loop. The `calc_alm2map` prologue calls `iter_to_ieee` once, performs the packed scale scan, and branches to the suffix kernel.

Rust's hot fallback symbol `multiply_component_scaled_with_normal_factor_f32` contains scalar `vmulss`, exponent-field extraction, integer range checks, sign/mantissa reconstruction, multiple exceptional branches, and a `__powisf2` call on the fallback path. This matches the prior exact-source Callgrind attribution of 4.42B inclusive Ir under `multiply_complex_scaled_f32`; nested inclusive costs are not additive. The current-source-equivalent Callgrind counts and equivalent-output-work normalization are:

| GL T720 full synthesis | Rust Ir | DUCC Ir | Ir / required output degree slot, Rust | Ir / required output degree slot, DUCC |
|---|---:|---:|---:|---:|
| q1 | 7,389,581,004 | 260,814,851 | 45.84 | 1.62 |
| q8 | 6,928,340,193 | 260,387,345 | 42.98 | 1.62 |

The denominator is 161,199,393 full-output active degree×latitude slots. This is an instruction-work comparison, not a cycle or elapsed-time estimate. Pinned binary symbols and disassembly were inspected directly; the relevant source-level summary is preserved above, and full raw Callgrind/disassembly evidence remains in the linked 2026-10-28 handoff report.

## Stage profile and limits

The first-call profile binary records Rust ring-group jobs, per-worker elapsed group time, and useful work; DUCC records per-call prefix/suffix work and times `alm2leg` separately from the longitude FFT. The sampled q8 worker split confirms all Rust workers receive jobs, but this run is not suitable for utilization timing: an unrelated, unpinned host workload was consuming about four CPU cores across the benchmark CPU set. Profile-stage durations are retained in the JSON as exploratory values only and are **not** latency acceptance data.

A T7 smoke run confirmed the f32 and f64-internal workers write matching output sizes and pass finite/repeat/reset checks. Relative L2 versus DUCC was `2.76e-7` for native f32 and `3.18e-8` for f64-internal. No T720 accuracy, adjoint, polar/extreme, subnormal, overflow, or workspace-reset acceptance run has yet been made for the f64 diagnostic; it is not a production candidate.

## Decision and next measurement

DUCC enters its fused suffix immediately for all counted random-fixture contributions; it has no scaled prefix to optimize in these T720 cases. Rust's new packet-level census finds 37–47% of all active degree×latitude terms in state-safe fallback suffixes, with no later recurrence range exits. That supports testing the shared native-f32 suffix handoff. DUCC's reflected-ring pairing remains a larger independent work-reduction target after this candidate is evaluated.

The f64-internal diagnostic's earlier first-call stage durations were collected under host contention and are not latency evidence. The real-state replay at the newer production source shows native-f32 exponent folding can destroy nonzero recurrence states on the candidate handoff. Run a fresh f32-I/O/f64-internal comparison at current source with Legendre-stage and whole-transform q1 timings first for full and reduced `mmax`, then q8; report the diagnostic nature, per-worker jobs, and output errors. The prior profile is historical context only. All four native-f32 Rust/DUCC cells remain **GAP**.
