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
