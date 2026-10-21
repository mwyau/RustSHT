# Plan 14: f64 scalar analysis fused-route floor at T16

**Baseline source:** `2dadb0c07d776e0f7a6679af8c28bb908082c443`.
**Candidate source:** `830ecc63eb9df829bebefb7c7a908bba4e20242e`.
**DUCC:** 0.42.0, pinned at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Change

The production f64 scalar-analysis checked fused-route floor is now T16.
Only the dispatch threshold and its route-selection test changed. T15 keeps
the callback route; T16, T31 and larger bands can use the existing checked
fused route. The scaled transition, range proof/cache, product checks, late
validation and fallback are unchanged. f32 remains at T255, and scalar
tables, synthesis, spin-1, other grids and other operators are unchanged.

The motivating T31 full-order even-smooth control had remained about 10.6 µs
slower than DUCC after the T32 change. Three calibrated q1 phase-profile
processes put the T31 callback route at 19.451 µs median for recurrence and
accumulation, versus 4.775 µs for the forward FFT. A checked fused-route
profile reduced recurrence/accumulation to 14.515 µs while FFT and preparation
stayed flat. Those stage intervals include the test-only observer and are
diagnostic; full-transform timing is authoritative.

## Matched full-transform comparison

Both source builds used Rust 1.99.1 / LLVM 23.1.1, release mode,
`-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, and runtime Pulp V3. DUCC used
GCC 15.2 and the pinned source; SHT translation units used
`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, while the native
benchmark driver used strict FP. The host was a Ryzen 9 5950X. q1 workers
were pinned to physical CPU 0. The same native harness, DUCC binary, geometry,
fixtures and output/reset rules were used. Each source/case used three
interleaved Rust/DUCC process pairs and 30 calibrated 75 ms-target batches
per backend; actual batches ranged 42.47–71.94 ms. All six cases are full
`mmax`, GL, `ntheta=lmax+1`. Baseline/candidate source rounds were separate,
but the harness alternated Rust and DUCC process order within each round.

| Case | Rust p10 / median / p90 before → after (µs) | DUCC median before → after (µs) | Rust change | Rust/DUCC before → after | Candidate excess | Candidate relative L2 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| T15, `nphi=31` | 8.535 / 8.586 / 8.632 → 8.488 / 8.547 / 8.587 | 10.504 → 10.400 | +0.5% | 0.817x → 0.822x | −1.853 µs | `9.04e-16` |
| T15, `nphi=32` | 7.120 / 7.145 / 7.527 → 7.132 / 7.180 / 7.256 | 6.042 → 6.033 | −0.5% | 1.183x → 1.190x | +1.147 µs | `1.00e-15` |
| T16, `nphi=33` | 8.277 / 8.317 / 8.349 → 7.507 / 7.536 / 7.596 | 8.125 → 8.144 | **+9.4%** | 1.024x → 0.925x | −0.608 µs | `1.36e-15` |
| T16, `nphi=34` | 8.528 / 8.587 / 8.665 → 7.800 / 7.831 / 7.903 | 6.839 → 6.890 | **+8.8%** | 1.256x → 1.136x | +0.940 µs | `1.39e-15` |
| T31, `nphi=63` | 25.424 / 25.800 / 26.177 → 19.924 / 20.044 / 20.278 | 19.021 → 19.004 | **+22.3%** | 1.356x → 1.055x | +1.040 µs | `1.52e-15` |
| T31, `nphi=64` | 26.963 / 27.051 / 27.185 → 21.323 / 21.559 / 21.737 | 16.520 → 16.325 | **+20.3%** | 1.638x → 1.321x | +5.234 µs | `1.54e-15` |

The T15 control is stable within 0.5%. Rust T16 latency falls by 0.78–0.77
µs, and T31 falls by 5.49–5.76 µs. On T31 the stage profile attributes
4.94 µs of the reduction to recurrence/accumulation; FFT moved by only
0.03 µs and order-spectrum preparation by about 0.01 µs. The T31 even
longitude case still has a measured +5.23 µs whole-transform excess, so this
change narrows the gap rather than closing it.

Frequency medians were effectively identical (4.691 GHz baseline and
candidate). Runqueue-wait medians were 5.4% and 5.8%, with p90 values of
11.7% and 14.2%. Candidate and baseline Rust p10–p90 ranges for T16/T31 were
separated; T15 ranges overlapped. Every recorded backend batch met the
20–100 ms timing contract. First-call/setup data are preserved separately.

## First-call behavior and numerical validation

A focused T15/T16 even-smooth follow-up collected ten process pairs and 100
timed batches per backend/case. At T16, Rust's warm median changed from 8.611
to 7.836 µs; first-call median changed from 26.230 to 30.898 µs. T15's first
call also moved from 23.350 to 25.644 µs, and the T15/T16 first-call
p10–p90 ranges overlap. This does not establish a clean first-call
regression, but it leaves a possible one-time T16 proof/dispatch cost. If
the full 4.67 µs median shift is real, the warm saving repays it after about
six further T16 analyses. Plan/workspace setup remained around 0.09 ms in
both builds. Do not present warm latency as a first-call improvement.

The native harness checked deterministic full coefficient inputs against
DUCC, repeated-call reset identity, selected basis modes, zero, extreme finite
and subnormal inputs, roundtrip and both adjoint residuals. Maximum candidate
relative L2 over the main matrix was `1.54e-15`; maximum absolute error was
`8.31e-16`. The release regression suites passed with all features (233 unit,
4 extreme-scaling, 11 f32, 1 workspace) and without default features (195
unit, 4 extreme-scaling, 11 f32, 1 workspace). Formatting and strict Clippy
passed with all features and no default features. No numerical tolerance or
safety check changed.

## Disposition and next work

Keep the T16 f64 floor for the current reusable-plan throughput path: T16 and
T31 warm full transforms improve, T15 is unchanged, the production route
tests confirm the intended boundary, and the safety/regression suites pass.
Track the first-call T16 caveat rather than hiding it. T31 even-smooth still
has a +5.23 µs excess. The next Plan 14 task is to re-rank the remaining q8
whole-transform excess from this source, not another low-band cutoff sweep.

Compact machine-readable timing summaries, binary hashes and reproduction
instructions are in
[`reports/data/plan14-t16-analysis-cutoff-2026-10-21/`](data/plan14-t16-analysis-cutoff-2026-10-21/).
Per-process raw batches and host diagnostics remain local and untracked.
