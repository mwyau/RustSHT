# Scaled-prefix SIMD precheck candidate

## Hypothesis and status

The isolated no-op normalization source still walks every active lane on each
scaled-prefix call. A prior route count found that all mantissas were already
in range on 51.78% of those calls. This candidate runs the existing SIMD
range predicate on each recurrence pair and calls the robust scalar
normalizer only when at least one active lane needs scaling. If the predicate
finds all mantissas in range, the route retains the current ordinary/scaled
state; it can enter the SIMD suffix only when every active exponent is zero.

The candidate passes focused recurrence, fallback, and T2047 route tests. It
has no complete-transform performance measurement and remains isolated.

## Source and validation

The source base is the isolated no-op normalization candidate. Its
`synthesis_fast.rs` SHA-256 was
`04aadcae00ab04feb18704e3f2d50687141ed088d32c7ce932222c4d186c308b`; the
candidate patch is
[`scaled-prefix-simd-precheck-candidate-v1.patch`](scaled-prefix-simd-precheck-candidate-v1.patch),
SHA-256
`d5ee9b8b094156bd41677758f52c76c01d5c52a0f959665fd3f49b8d47fc74a3`. The
resulting `synthesis_fast.rs` is
`5bd3af68311792091ff6dcc9628f9fed91ddcb240f54874b05c886fcc73430c3`;
`scalar.rs` remains
`cfe5f7ee31e75f70445fdca118f154cd39961ed5b0751f593e94c26794283a0a`.
The release route-test executable SHA-256 is
`7e27b4ffe0ecf61c43250df703d0a0382d8554a7cfcca469cc239a95f6e7b17e`.

Rust/Cargo 1.99.0 release tests passed for the full isolated synthesis-fast
unit-test module (6 passed, 1 ignored) and the scalar SIMD range guard (1
passed). These controls include paired-reference synthesis, transition,
unsafe-coefficient fallback, polar fallback, mixed polar tiles, NaN/infinity
range checks, and SIMD tails. The ignored T2047 q1 route test also passed and
required bitwise equality between its warm-up and profiled transform.

## Route counts

The T2047 profile used GL f64 scalar synthesis, full `mmax`, `phi0=0.37`,
`2048x4096`, and one worker. The candidate performed 1,043,227 SIMD range
probes; 499,011 found an out-of-range mantissa and entered the lane normalizer.
The prefix normalizer therefore handled 499,011 calls and 59,596,810 lane
visits. Against the no-op source route count, this removed 535,953 normalizer
calls and 49,894,046 lane visits. Recurrence work remained 110,444,000
prefix pair-lanes; the route also recorded the same 706,324 accumulator
rebases, 1,619,628,108 SIMD-suffix pair-lanes, 5,439,548 fallback pair-lanes,
and 262 fallback calls.

These are instrumented route counts, not speed or timing results. The probe
itself runs on every recurrence pair and has a cost that must be measured
against the saved scalar work. The candidate has not been built as a Python
extension, compared with DUCC or the clean production baseline, checked for
peak memory, or run through q8 and T511 regression controls. The preceding
no-op screen remains inconclusive; this candidate is a separate mechanism
and does not retry that screen.
