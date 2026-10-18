# T2047 scalar transition route profile

The test-only profile identifies the paired visitor as the largest measured
route cost in an isolated scalar transition source. It does not compare this
source with the clean baseline, DUCC, or SHTns and does not accept the
optimization.

## Source and configuration

- Clean branch base: `77cfda387f9bcbb0d334e185e5cc6a9355f90ea7`.
- Experimental source patch: [`scalar-transition-q1-q8-route-profile-v1.patch`](scalar-transition-q1-q8-route-profile-v1.patch), SHA-256 `fb0e5e53fb3f444fe570e3c7fe22f43e8f43f9b08a46f6d90c01f9752614d59f`.
- Release test binary SHA-256: `f91208ab2b053168b621a51f6c4b59ddccacb97332dbb9c35313bc3caf6d92dd`.
- Test: `legendre::route_profile::m6_scalar_synthesis_routes`, compiled with `--release`.
- Grid: Gauss–Legendre (GL), f64 scalar, `lmax=mmax=2047`, `ntheta=2048`, `nphi=4096`, `phi0=0.37`.
- The deterministic dense coefficients have magnitudes of order `1e-5`.
- q1 used one Rayon worker on one logical CPU. q8 used eight workers on eight logical CPUs.
- Hardware: AMD Ryzen 9 5950X, x86-64, 16 physical cores, 32 logical CPUs, AVX2 and fused multiply-add (FMA), 96 GiB RAM.
- Software: Ubuntu 26.04.1 LTS, Linux 7.0.14, Rust and Cargo 1.99.0, Rayon 1.12.0, and `pulp` 0.22.3.

The test ran one unmeasured synthesis, one timed synthesis with route counters
inactive, and one timed synthesis with counters active. It asserted exact map
equality between the timed calls. The q1 and q8 runs each contain one timing
sample, so they do not establish timing CV or drift. Pressure checks passed
below the warning band, and no competing build or test process was present.
Worker placement stayed inside each process's requested affinity mask. Exact
pressure, frequency, context-switch, process, and thread-placement captures
remain local and untracked. Peak RSS was not measured.

An initial q8 attempt failed the accounting assertion because the test added
the same whole-grid occupancy once for every worker. The run is preserved as
local diagnostic evidence and is excluded from the results below. The test
now records whole-grid occupancy once and asserts that workers observe the
same occupancy; this change affects test aggregation only.

## Route counts

The q1 and q8 runs counted the same work:

| Route | Lane-pairs | Fraction of active work |
|---|---:|---:|
| Ordinary fused path | 1,322,505,640 | 76.25% |
| Paired visitor, ordinary values | 100,471,286 | 5.79% |
| Paired visitor, scaled values | 37,458,250 | 2.16% |
| Successful transition prefix | 42,841,932 | 2.47% |
| Ordinary suffix after transition | 231,100,956 | 13.32% |

The grid contained 2,149,580,800 possible lane-pairs. The polar cutoff
excluded 415,202,736; the remaining 1,734,378,064 were each attributed to
one committed route. The transition ran 11,620 times: 8,612 completed and
3,008 declined. It discarded 131,220 prefix lane-pairs. No unpaired visitor
fallback or coefficient-preparation failure occurred for this input.

## Timing and worker distribution

| Workers | Whole synthesis, counters inactive (ms) | Counters active (ms) | Fused route phase sum (ms) | Visitor phase sum (ms) | Transition phase sum (ms) | Coefficient preparation (ms) | Factor preparation (ms) |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 2282.876 | 2290.775 | 362.234 | 1306.557 | 500.881 | 2.447 | 6.571 |
| 8 | 510.570 | 512.955 | 384.468 | 1345.626 | 517.278 | 18.375 | 54.464 |

The q1 and q8 wall times imply a descriptive 4.47x scaling ratio for these
single runs. The phase timers sum across workers, so their values can exceed
whole-transform latency. The visitor accounts for 60.0% of the summed route
phase time at q1 and 58.0% at q8, while it handles 7.95% of active lane-pairs.
These timings are mechanism diagnostics, not acceptance measurements.

| Rayon worker | Committed lane-pairs (millions) | Visitor phase (ms) | Summed route phases (ms) |
|---:|---:|---:|---:|
| 0 | 268.684 | 0.000 | 86.767 |
| 1 | 111.540 | 312.758 | 414.937 |
| 2 | 263.419 | 77.765 | 187.176 |
| 3 | 223.546 | 281.941 | 471.667 |
| 4 | 263.419 | 77.413 | 186.819 |
| 5 | 111.540 | 313.460 | 413.720 |
| 6 | 223.546 | 282.288 | 472.497 |
| 7 | 268.684 | 0.000 | 86.628 |

The maximum-to-minimum committed work ratio was 2.41. The summed route-phase
ratio was 5.45. This profile measures work imbalance as well as kernel cost;
it does not test a new scheduling policy.

## Interpretation

The paired visitor is the next measured synthesis bottleneck. Its callback
handles each degree pair and updates the spectrum for each lane, including
scaled complex products. The scalar transition source already accumulates the
ordinary suffix separately, but visitor work remains on the declined or
otherwise ineligible blocks. A bounded next step is to account for visitor
work by decline reason and source path, then test a native accumulator for
that route while keeping the scaled fallback intact.

The frozen 128-ring extension and its earlier q1 whole-transform matrix are a
separate experiment. This profile neither validates nor rejects that
candidate. The last matched T2047 baseline remains 9.06x slower than DUCC at
q1 and 13.97x at q8; these route diagnostics do not refresh those ratios.

## Follow-up: visitor source and transition-decline timing

The follow-up uses the v1 experimental source patch plus the test-only
instrumentation in
[`scalar-transition-visitor-source-profile-v2.patch`](scalar-transition-visitor-source-profile-v2.patch),
SHA-256 `0d2168f28fdc7c12ed434a89cf395210c502906a4a07e1b0929e8371133af54f`.
Applied in order to the clean branch base above, the two patches reconstruct
the measured source; apply the v2 zero-context diff with
`git apply --unidiff-zero`. Its SHA-256 is
`6eefe98cf3326b4e725bf5b0a4f126eb55b37a84b80f9cd673886af55b5f966c`. The
release test binary SHA-256 is
`f3b802befbbba495fe8a29868ced5fe7cdb05d1c6379b735a3f13cd457d7b8b4`.
The follow-up added source counters and timers only; it did not alter the
candidate's numerical path.

The same GL f64 scalar T2047 transform ran at q1 and q8. Each process used one
unmeasured warm-up, then one transform with counters inactive and one with
counters active. The map from the counter-active call was bitwise equal to the
counter-inactive map. Both route-accounting assertions passed. These single
calls are diagnostics, not performance acceptance samples; they do not
establish within-process CV or drift and do not compare the candidate with the
clean baseline, DUCC, or SHTns. Memory was not measured. The preflight and
postflight pressure checks stayed below the warning threshold, and monitoring
found no clear competing CPU workload. Sampled thread placement stayed within
the requested affinity masks. Detailed transient pressure, frequency,
context-switch, and process captures remain local and untracked.

The first follow-up q1 attempt completed both transforms with bitwise-equal
maps but failed a source-accounting assertion: 53.6 million visitor lane-pairs
came from the scaled-lane fallback inside the fused-block helper and had no
source label. That diagnostic was retained locally. The instrumentation then
added this route as a third visitor source and required every visitor
lane-pair to have exactly one label. The final q1 and q8 results below passed
that assertion. This correction changes accounting only.

### Visitor source work

| Visitor source | Calls | Lane-pairs | Fraction of active work | q1 worker-summed time (ms) | q8 worker-summed time (ms) |
|---|---:|---:|---:|---:|---:|
| Compact order block | 1,954 | 42,249,640 | 2.44% | 520.301 | 535.323 |
| Regular order block | 1,054 | 42,114,240 | 2.43% | 420.055 | 432.748 |
| Scaled-lane fallback after fused work | 3,290 | 53,565,656 | 3.09% | 413.445 | 446.288 |
| **Total paired visitor** | — | **137,929,536** | **7.95%** | **1,353.431** | **1,413.955** |

The compact-order visitor used 38.44% of q1 and 37.86% of q8 visitor time. The
regular-order visitor used 31.04% and 30.61%; the scaled-lane fallback used
30.55% and 31.56%. The first two sources are the visitor calls after declined
scalar transitions. The third source handles scaled lanes excluded from the
fused block's ordinary path.

The scalar transition was attempted 11,620 times: 8,612 completed and 3,008
declined. Thirty-two declines were geometry rejections; 2,976 occurred after
transition work. No coefficient or other pre-work rejection occurred. The
declined transitions discarded 131,220 prefix lane-pairs. The visitor
fallback accounts for the remaining recurrence work after those declines.

### Whole-call and route phase times

| Workers | Whole transform, counters inactive (ms) | Whole transform, counters active (ms) | Fused phase sum (ms) | Visitor phase sum (ms) | Transition phase sum (ms) | Coefficient preparation (ms) | Factor preparation (ms) |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 2,339.277 | 2,338.196 | 366.146 | 1,353.431 | 496.289 | 2.479 | 6.580 |
| 8 | 527.449 | 528.440 | 403.767 | 1,413.955 | 524.288 | 19.454 | 54.874 |

Route phase values sum across workers and can exceed whole-call latency. The
counter-active call changed the single measured wall time by −0.05% at q1 and
0.19% at q8; these differences are descriptive only. The fixed source used an
AMD Ryzen 9 5950X with AVX2/FMA, 16 physical and 32 logical CPUs, and 96 GiB
RAM. It used Ubuntu 26.04.1, Linux 7.0.14, Rust/Cargo 1.99.0, Rayon 1.12.0,
and `pulp` 0.22.3.

The compact-order visitor is the largest measured visitor subpath. It handles
only 2.44% of active lane-pairs but costs 520–535 ms in worker-summed time. A
native one-channel recurrence and accumulator was tested for the declined-
transition route. It committed only four calls; scaled-range checks caused
2,976 of 3,004 declines. The path therefore replaced only 0.014% of active
lane-pairs and is rejected as implemented. See the [decline
diagnostic](scalar-visitor-accumulator-decline-diagnostic-v1.md). The frozen
128-ring candidate remains a separate, unaccepted experiment.
