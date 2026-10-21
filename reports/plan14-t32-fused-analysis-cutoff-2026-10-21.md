# Plan 14: T32 checked fused scalar analysis

**Implementation:** `de4c6569176387b7dc850e9cb083b715efbbc5a2`.
**Baseline code:** `c2f89215b24668503b3e1f4abf0e36ab26c3ac96`.
**DUCC:** 0.42.0, pinned at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Change and evidence

Plan 14's T32 full-order q1 analysis was about 9 µs slower than DUCC. Its
calibrated phase probe put 21.184 µs in recurrence and accumulation, making
the existing checked fused route a source-grounded candidate. Commit
`de4c6569` lowers only the f64 scalar analysis eligibility floor from T64
to T32. The existing checked SIMD route, scaled transition, range proof,
late validation and fallback are unchanged. f32 remains at T255; spin-1,
tables and other operators are unchanged.

## Matched full-transform results

The case set covers the T31/T32 cutoff, full and reduced `mmax`, and
odd-minimum/even-smooth longitude counts. Baseline code was `c2f89215`; the
candidate adds exactly the two-file cutoff/test diff later committed as
`de4c6569`. Rust 1.99.1 / LLVM 23.1.1 used release mode,
`-C target-cpu=x86-64-v3`, runtime Pulp V3; DUCC used GCC 15.2, pinned
0.42.0 source, x86-64-v3 with fast-math SHT compilation and a strict
driver. Each q1 case used three interleaved Rust/DUCC process pairs and 30
calibrated 20–100 ms batches per backend, with reusable plans/workspaces,
one physical CPU and numerical checks outside timing. The Ryzen 5950X was
contended: q1 runqueue-wait medians were about 12–16%, with p90 up to 32%.
The source rounds had similar CPU frequency and switch counts.

| Case | Rust p10 / median / p90 before → after (µs) | DUCC median before → after (µs) | Rust/DUCC before → after | Excess before → after (µs) | Rust change | Candidate relative L2 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| T31 m31, nphi=63 | 25.484 / 25.572 / 25.665 → 25.507 / 25.663 / 25.803 | 19.134 → 19.059 | 1.337x → 1.347x | +6.439 → +6.604 | +0.35% | `1.49e-15` |
| T31 m31, nphi=64 | 26.848 / 27.130 / 27.228 → 26.858 / 27.054 / 29.790 | 16.499 → 16.491 | 1.644x → 1.640x | +10.632 → +10.563 | −0.28% | `1.51e-15` |
| T32 m32, nphi=65 | 30.687 / 30.853 / 31.297 → 24.899 / 25.273 / 25.591 | 21.761 → 21.710 | 1.418x → 1.164x | +9.092 → +3.563 | **−18.08%** | `1.90e-15` |
| T32 m32, nphi=66 | 30.863 / 31.011 / 31.152 → 25.066 / 25.176 / 25.681 | 21.678 → 21.833 | 1.431x → 1.153x | +9.333 → +3.343 | **−18.81%** | `1.61e-15` |
| T32 m8, nphi=17 | 10.738 / 10.845 / 11.176 → 8.078 / 8.449 / 8.583 | 11.773 → 11.733 | 0.921x → 0.720x | −0.928 → −3.283 | **−22.09%** | `2.61e-15` |
| T32 m8, nphi=18 | 10.436 / 10.479 / 10.745 → 8.108 / 8.176 / 8.235 | 8.720 → 8.819 | 1.202x → 0.927x | +1.759 → −0.643 | **−21.98%** | `2.55e-15` |

T31 is the unchanged control; median movement stays within 0.4%. All four
T32 cases improve, including reduced `mmax`, and their candidate p10–p90
Rust ranges are below baseline. The table therefore supports the T32 floor
for the checked f64 route across these two longitude sizes. It does not
close the T31 even-smooth gap or establish parity across all low bands.

## Phase attribution and validation

For T32 full `mmax`, even `nphi=66`, the calibrated Rust stage probe moved
recurrence/accumulation from 21.184 to 16.558 µs (−4.626 µs). The full call
fell 5.835 µs; FFT moved only 6.014 to 5.951 µs, and spectrum/latitude
preparation was effectively unchanged. This ties most of the gain to the
fused recurrence/accumulation route. Stage values include test-only timers
and are diagnostics; full distributions are authoritative.

Across every case, maximum relative L2 was `2.62e-15` and maximum absolute
error was `2.67e-15`. Zero, subnormal, extreme-finite, analytic, adjoint,
polar, tail, f32 and spin regressions passed in the release suites: 233 unit,
4 extreme-scaling, 11 f32 and 1 workspace test with parallel features; 195
unit, 4 extreme-scaling, 11 f32 and 1 workspace test without default
features. Formatting and strict Clippy passed in both feature modes.

## Disposition and next step

Keep the f64 fused analysis floor at T32. It removes about 5.5–6.0 µs of
T32 full-order excess and improves the reduced-order cases without changing
T31. The remaining small-q1 lead is T31 full-order even-smooth analysis
(about 10.6 µs excess); profile and test the existing fused route there
before selecting any lower threshold. Then return to the largest current
q8 whole-transform excess.

Raw samples, checksums, dispatch, numerical outputs and host summaries are
in [`reports/data/plan14-t32-fused-analysis-cutoff-2026-10-21/`](data/plan14-t32-fused-analysis-cutoff-2026-10-21/).
The joined table is `comparison.csv`; stage details are in `phase-summary.csv`.
