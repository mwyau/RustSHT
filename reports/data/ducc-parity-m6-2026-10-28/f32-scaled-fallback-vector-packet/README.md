# Native-f32 scaled-fallback packet SIMD candidate

**Decision:** accept this bounded synthesis-only improvement as an interim
step. It removes repeated scalar complex-scaling helper work for checked full
f32 packets, while retaining the shared recurrence, existing scalar fallback,
and the checked normal-range packet path. The four GL T720 scalar-synthesis
cells improve by 5.8–12.7% against exact parent source `33c2faba`; the result
is material but remains 23.1–35.5× pinned DUCC. No DUCC parity is claimed.

The scaled-prefix-to-fused-recurrence suffix candidate was separately measured
and rejected. The transition census found almost every scalar fallback packet
enters the subnormal/zero exponent range, and exact one-time correction can
erase nonzero recurrence state. This candidate therefore vectorizes the
contribution arithmetic inside the existing scaled path; it does not add a
second recurrence or a new recurrence suffix handoff.

## Source and method

- Rust parent and candidate base: `33c2faba2f9be2f890b27543bd3f887c715f7a25`.
- Candidate patch: [`candidate.patch.gz`](candidate.patch.gz), SHA-256
  `ee09fd5bb51dc69cf1740461f12bbe765e09f2b4e8bcc05de1a37d1cb430277e`.
- Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host: AMD Ryzen 9 5950X; rustc/Cargo 1.99.1, LLVM 23.1.1; GCC 15.2.0.
- Rust release workers used `-C target-cpu=x86-64-v3`, Pulp V3, and eight
  f32 lanes. DUCC used `-O3 -ffast-math -ffp-contract=fast
  -march=x86-64-v3`, with a non-fast-math driver. q1 was pinned to one fixed physical CPU;
  q8 used eight distinct fixed physical CPUs. Host-local CPU IDs are omitted.
- Cases used GL T720 (`ntheta=721`, `phi0=0.23`), full `mmax=720`,
  `nphi=1458`, and reduced `mmax=180`, `nphi=378`. Parent/candidate used 15
  randomized interleaved fresh-process pairs per cell, identical warm-call
  boundaries, reusable plans/workspaces, and finite/repeat/reset checks.
  DUCC medians are from the initial three interleaved parent/candidate/DUCC
  triads; the `C/D` ratios below are reference comparisons, not 15-pair
  confidence intervals. Fixture, binary and per-pair data are preserved in
  the JSON files alongside this report.

## Whole-transform results

| GL T720 f32 scalar synthesis | Parent ms | Candidate ms | DUCC ms | Candidate/parent [95% CI] | Faster pairs | Candidate/DUCC |
|---|---:|---:|---:|---:|---:|---:|
| Full q1 | 731.604 | 672.131 | 19.719 | 0.92124 [0.91203, 0.92996] | 15/15 | 34.14× |
| Full q8 | 118.428 | 103.919 | 3.410 | 0.87294 [0.82945, 0.89726] | 15/15 | 30.65× |
| Reduced q1 | 218.893 | 198.619 | 8.575 | 0.90868 [0.90233, 0.92115] | 15/15 | 23.11× |
| Reduced q8 | 59.042 | 54.896 | 1.592 | 0.94219 [0.90064, 0.98313] | 13/15 | 35.47× |

Separate seven-call Legendre-stage samples gave parent→candidate medians of
720.061→666.910 ms full q1, 120.358→98.970 ms full q8,
216.530→200.047 ms reduced q1, and 56.967→52.002 ms reduced q8. These stage
measurements exclude FFT and writeback; the table above remains the whole-call
acceptance result. The change does not modify scheduling; the accepted
32-ring q8 worker distribution and workspace costs remain in the
[schedule report](../../ducc-parity-m6-2026-10-27/f32-32-ring-schedule-2026-10-27/README.md).
The exact temporary stage-profiling test is preserved as
[`stage-profile-harness.patch.gz`](callgrind/stage-profile-harness.patch.gz).

## Fallback work and instruction attribution

The parent-source route census counted 3.730M/5.203M full and 1.338M/1.661M
reduced paired fallback slots at q1/q8. Those packets had normal coefficients,
basis values, and unscaled products; almost all became subnormal or zero only
after the recurrence exponent was applied. A conventional normal suffix was
safe for fewer than 1% of fallback slots, and the separate packet handoff was
not promoted. The new packet leaf vectorizes coefficient/product arithmetic,
splits each normal product into a mantissa and exponent per lane, performs the
normal scaling and accumulation with f32 SIMD, and delegates subnormal
rounding to the existing exact binary scaler. Its full-packet checks decline
nonfinite, subnormal-input, possible-recovered-zero, overflow, and unsafe-sum
cases before modifying staged output; partial vectors still use the scalar
path.

At q1, whole-call Callgrind totals fell from 7,386,721,582 to 6,610,414,010
Ir full (−10.51%) and from 2,386,505,645 to 2,124,301,940 Ir reduced
(−10.99%). The source-attributed call count for each real/imaginary
`multiply_component_scaled_with_normal_factor_f32` helper fell from
10,443,214 to 7,237,292 full (−30.7%) and from 2,893,846 to 1,823,007 reduced
(−37.0%). This is direct evidence of less scaled-helper work; no new route
counters were inserted in the candidate, so these counts are not presented as
an exact accepted-packet census.

Normalized by 161,199,393 full and 81,210,177 reduced required
degree–latitude output slots, Rust changed from 45.82→41.01 and 29.39→26.16
Ir/slot. The prior same-geometry pinned-DUCC q1 profile is 1.618/1.338
Ir/slot, leaving instruction ratios of 25.35× full and 19.55× reduced for
the candidate. Callgrind instruction references are not cycles or elapsed
time; component-helper inclusive totals overlap their caller. Raw compressed
profiles and the derived summary are in [`callgrind/`](callgrind/).

The executed Pulp V3 loop disassembly contains packed AVX2 `vmulps`, `vaddps`,
and `vfmaddps` operations for coefficient/product/accumulator lanes. It still
contains scalar exponent decomposition and safety branches, which is why its
instruction work remains far above DUCC. The [candidate visitor disassembly](callgrind/candidate-fallback-loop.disasm.txt)
and before/after lint-fix visitor extracts are preserved under
[`callgrind/`](callgrind/). DUCC's pinned f32-I/O path uses f64
for Legendre work, transitions from its scaled prefix into an unrolled
four-lane AVX2/FMA recurrence-accumulation suffix, and pairs reflected GL
rings. The earlier [packet suffix report](../f32-packet-suffix-candidate/README.md)
contains the DUCC kernel source/assembly comparison and the failed Rust suffix
handoff's route counts and dynamic declines.

## Numerical and protected controls

All 60 parent/candidate primary workers produced bitwise-identical output and
passed finite, repeat, and workspace-reset checks. Candidate/DUCC maximum
relative-L2 / max-absolute errors were `1.72343e-4 / 9.64403e-4` full and
`2.72546e-4 / 1.54787e-3` reduced. The new independent T720 low-mode analytic
test measured relative-L2 / max-absolute errors `1.36819e-7 / 2.98023e-7`
reduced and `1.51022e-7 / 3.57628e-7` full. The scalar adjoint relative
identity errors were `1.06850e-6` reduced and `8.07212e-6` full. Existing
packet tests cover subnormals, extreme exponents, nonfinite/overflow decline,
zero products that could recover under positive exponents, and SIMD tails.

An initial three-pair adjacent screen covered full/reduced f32 scalar analysis
and spin-1 analysis/synthesis q1/q8; all parent/candidate outputs were bitwise
equal. Its per-cell results are in
[`adjacent-f32-initial-triads.json`](adjacent-f32-initial-triads.json). The
15-pair extensions were also bitwise equal to parent. Reduced scalar analysis
q8 was neutral (`0.99599`, 95% interval `[0.93718,1.06449]`). Full
spin-1 analysis q1 measured a repeatable 1.27% slowdown (`1.01274`, interval
`[1.00860,1.01694]`); full q8 spin-1 analysis was inconclusive (`1.00860`,
`[0.96640,1.09326]`), as was full q8 spin-1 synthesis (`1.01992`,
`[0.98070,1.04594]`). The q1 analysis effect is small relative to the
5.8–12.7% synthesis gains, but is retained as a disclosed collateral result;
its cause is not established. Protected f64 GL T1023 spin-1 synthesis remained
bitwise equal, with candidate/parent `1.00076` q1
(`[0.99739,1.00978]`) and `0.99678` q8 (`[0.98366,1.02005]`).

Candidate validation passed `cargo fmt --all -- --check`,
`cargo check --workspace --all-targets --all-features`, warning-denied
workspace Clippy, and `cargo test --workspace --all-features` (307 library,
4 extreme-FFT, 12 f32 integration, and 1 workspace-compatibility tests passed;
37 tests were ignored: 36 library and 1 f32 integration). GitHub's latest
pre-candidate Rust CI run on parent
`33c2faba` was green; the candidate's workspace commands above also pass
locally. The temporary Callgrind example and stage test were excluded from the
validation checkout and are not production files. After screening, Clippy's
`question_mark` lint prompted two equivalent early-return syntax changes. A
release rebuild showed the 2,662-instruction paired visitor sequence was
identical after normalizing RIP-relative link offsets; see
[`visitor-codegen-compare.json`](visitor-codegen-compare.json).

## Next measured direction

Keep M6 active. This change lowers scalar-synthesis fallback cost but does not
implement a recurrence suffix, and the four synthesis cells remain **GAP**.
The corrected transition route count admits only suffixes whose recurrence
exponents stay zero in every lane; these cover 41.10–46.62% of active paired
slots, but are not timed or numerically accepted. Next test a synthesis-only
guarded handoff using the existing shared recurrence and native-f32 fused leaf,
with exact transactional fallback on any nonzero exponent, later decline,
polar/mixed block, exceptional value or tail. Do not restore the rejected
shared early-stop visitor or f32-only reflected recurrence unchanged.
