# Native-f32 GL T720 scalar-synthesis baseline

## Exact source and result

The production scalar-synthesis implementation is exact source
`092fe72f322bd482dac73b020785b096772bc901`; current `dev` is
`023ba6f3172bac76ae558c44b359309f417d45b8`. The intervening commit changes
documentation only. `src/legendre/scalar_paired/synthesis.rs` and
`src/transform/scalar.rs` are unchanged at the current head. Pinned DUCC 0.42.0
is `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

Both scalar-synthesis files have identical Git blob IDs at this parent and
suffix-screen parent `746c7e60`: `295e862e9002fe91a00bdc1dd2caf8f40c3f9385`
for paired synthesis and `1334031f957c227dd7abf2f690a90926db0bd12f` for the
scalar transform. The transition candidate and route counts below therefore
exercise the same production implementation as this baseline.

The warm benchmark used three interleaved Rust/DUCC pairs per cell, GL
`lmax=720`, `ntheta=721`, `phi0=0.23`, full `mmax=720, nphi=1458`, reduced
`mmax=180, nphi=378`, and even longitude counts. Rust used rustc 1.99.1,
`-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, Pulp V3/eight f32 lanes. DUCC used
GCC 15.2 with `-O3 -ffast-math -ffp-contract=fast
-march=x86-64-v3`; its driver used `-fno-fast-math`. q1 was pinned to CPU 3 and
q8 to CPUs 8–15. Workspaces were reused; worker binaries selected repeated
warm batches targeting 35 ms.

| Cell | Rust ms | DUCC ms | Paired Rust/DUCC | Status |
|---|---:|---:|---:|---:|---|
| Full q1 | 642.768 | 20.015 | 32.08× | **GAP** |
| Full q8 | 97.791 | 2.657 | 36.94× | **GAP** |
| Reduced q1 | 191.050 | 8.389 | 22.77× | **GAP** |
| Reduced q8 | 51.287 | 1.205 | 42.43× | **GAP** |

The candidate binary was built for a separate f32 spin-1 dispatch screen. Its
only source change is bypassed for scalar spin 0, so its small timing spread is
not an implementation candidate and is excluded from the baseline decision.
All scalar parent/candidate outputs in that screen were bitwise equal. The
workers passed finite-output, repeat-call, and workspace-reset checks. Parent
versus DUCC relative-L2 / maximum absolute error was `1.72343e-4` /
`9.64403e-4` full and `2.72546e-4` / `1.54787e-3` reduced. This random-fixture
baseline does not replace analytic-mode or exceptional-value validation.

## Stage and work attribution

A scratch-only test probe on exact parent `092fe72f` measured 30 warmed calls
per cell, with phase timers enabled only in the test build. The median
Legendre recurrence and spectral-accumulation stage used 99.68–99.82% of the
whole call. The inverse FFT/writeback took 1.593 ms full q1, 0.236 ms full q8,
0.326 ms reduced q1, and 0.076 ms reduced q8. Full medians were 645.116 ms
Legendre / 646.750 ms whole q1 and 93.295 / 93.597 ms q8. Reduced medians were
192.839 / 193.179 ms q1 and 50.902 / 50.996 ms q8. Per-cell p10/p90 phase
values are in `baseline-results.json`; the isolated test helper patch is
`phase-profile-test.patch.gz`.

The current-source work census at `49295a89f2d13d8b02c2cbfa8566a8c5c21c6a80`
has byte-identical paired-synthesis and scalar-transform source. Rust visits
161,199,393 full and 81,210,177 reduced degree–latitude terms. The equivalent
DUCC GL route advances 80,729,837 full and 40,662,194 reduced northern-ring
terms, then reflects the outputs. The roughly 2× useful recurrence-visit
difference is an algorithmic opportunity, not a cycle estimate; DUCC's padded
SIMD work and Rust's executed work must be kept separate.

An isolated suffix-candidate route probe on that same synthesis source counted
1,744/4,337 initial all-normal fused blocks and 3,657/7,112 non-polar
fallback blocks for full q1/q8; reduced counted 922/2,036 fused and 972/1,849
fallback blocks. The probe did not count polar-excluded or mixed blocks. Its
candidate-only suffix counters and declines are preserved in the linked
[suffix-transition report](../f32-prefix-suffix-transition-49295a89/README.md)
and are not production behavior. That suffix candidate was rejected. Its
current-source AVX2 disassembly shows the existing native-f32 kernel already
uses packed YMM FMA, while retaining per-pair checks; DUCC's executed kernel
uses packed YMM double FMA and an unrolled suffix. The existing matched q1
Callgrind sample at this synthesis source measured 6.610B Rust versus 260.8M
DUCC Ir, or 41.01 versus 1.62 Ir per required degree–latitude output slot.
Ir are instruction counts, not elapsed time.

The q8 ring census has 23 group jobs across eight workers. Contributions per
worker ranged from 3.12M to 56.47M full and 3.09M to 18.21M reduced. The
test-only summed group nanoseconds were counter-instrumented and are not
reliable worker makespan or scheduling timings. This is evidence of uneven
work assignment, but q1 remains 22.77–32.08× DUCC, so scheduling does not
explain the main gap.

## Scaled-prefix transition result and decision

The paired-synthesis and scalar-transform source blobs at this baseline are
identical to exact suffix-screen parent `746c7e60`. That isolated candidate
corrected both recurrence states once at a checked transition and then called
the existing fused f32 suffix. It completed a substantial guarded suffix, but
whole-transform latency regressed in every cell. The detailed candidate
results and emitted DUCC assembly are in the [suffix-screen report](../f32-scaled-prefix-suffix-746c7e60/README.md).

| Cell | Completed blocks | Declined | No transition | Tail | Prefix contributions | Guarded fused suffix contributions |
|---|---:|---:|---:|---:|---:|---:|
| Full q1 | 3,114 | 445 | 98 | 98 | 8.134M | 33.598M |
| Full q8 | 6,041 | 940 | 131 | 98 | 7.328M | 33.048M |
| Reduced q1 | 720 | 159 | 93 | 98 | 3.282M | 13.695M |
| Reduced q8 | 1,453 | 303 | 93 | 98 | 2.395M | 13.736M |

Transition-degree min/median/max was `5/399/720` and `5/392/720` for full
q1/q8, and `5/130/715` and `5/132/715` for reduced q1/q8. Recurrence states
did not become unsafe after a successful handoff. All later declines came from
coefficient, product or accumulated-output checks; each declined block and
each no-transition block replayed through the existing scaled route. The
candidate slowed the whole transform by 8.2%, 15.5%, 16.0% and 27.0% for full
q1/q8 and reduced q1/q8. Its Legendre stage slowed by 7.1%, 25.3%, 14.2% and
26.9%. This strategy is a measured blocker; do not repeat the same guarded
block-check/replay implementation.

The DUCC path keeps recurrence state in f64 with a per-lane scale through
`init_lambda`/`iter_to_ieee`; `calc_alm2map` applies the residual scale once,
then `alm2map_kernel` runs an unrolled double-FMA suffix. Real-state replay
shows why native-f32 correction must retain the original fallback: exponents
from `-160` to `-320` can turn a nonzero recurrence state into zero while its
scaled coefficient product remains representable.

The separate f32-I/O/f64-internal diagnostic at transform source `54b166f2`
measured the existing reflected f64 route at `0.746–0.779×` DUCC, while
forcing the same f64 implementation through all-ring paired traversal cost
`3.714–13.442×` DUCC. The major remaining cause is therefore algorithmic
dataflow as well as native-f32 exponent range. After this documented blocker,
continue to the remaining native-f32 scalar/spin families, then revisit
reflected-ring scalar synthesis. No production code changed, and all four
native-f32 scalar-synthesis cells remain GAP.

## Reproduction and provenance

`baseline-results.json` contains source revisions, binary hashes, fixtures,
per-pair warm timings, phase summaries, route/work counts, and limits.
`paired-screen-runner.py` imports the checked benchmark helper and accepts
`RUSTSHT_M6_BUILD_ROOT` and `RUSTSHT_M6_RESULTS_DIR`; its per-run telemetry is
not intended for committing. Set up `cargo-parent`, `cargo-candidate`, and
`ducc-build` beneath the experiment build root, then run the script from the
repository root. The exact current Rust CI run on `023ba6f3` passed all Rust
format/check/lint/test jobs and the Python job:
[GitHub Actions run 40753398083](https://github.com/mwyau/RustSHT/actions/runs/40753398083).
