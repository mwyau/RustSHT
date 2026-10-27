# Native-f32 GL T720 scalar synthesis: scoped q8 task grain

## Decision and current status

Promote the f32-GL-scalar-synthesis-only scheduling policy in RustSHT source
`9dc4516080eb0b25711ae74825c634ca71166ffa`. It lowers the Rayon minimum
groups per task from two to one only when native f32 GL scalar synthesis has
more ring groups than execution threads. The shared f32/f64 recurrence,
coefficient accumulation, polar cutoff, and scaled fallback are unchanged.

The change materially improves both q8 cells. The q1 cells are repeatably
slower by 0.7% and 1.3%; this tradeoff is recorded rather than hidden. Adjacent
f32 paths and the protected f64 synthesis controls show no material regression.
The four scalar-synthesis cells remain far from DUCC parity, so this is an
accepted scheduling improvement, not goal completion.

## Matched whole-transform measurements

Each cell used 15 randomized, interleaved, fresh-process parent/candidate
pairs, with pinned DUCC 0.42.0 measured in the same screening campaign. Times
are warm whole-transform medians in milliseconds. `C/P` and `C/D` are medians
of per-pair ratios; the interval is the paired 95% bootstrap interval for
candidate/parent. q1 used CPU 0; q8 used CPUs 0–7 on an AMD Ryzen 9 5950X.

| GL T720 f32 scalar synthesis | Parent ms | Candidate ms | DUCC ms | C/P (95% interval) | C/D | Candidate faster |
|---|---:|---:|---:|---:|---:|---:|
| Full `mmax=720`, q1 | 717.367 | 722.448 | 18.914 | 1.0072 `[1.0067, 1.0088]` | 38.21× | 1/15 |
| Full `mmax=720`, q8 | 322.205 | 141.205 | 2.675 | 0.4365 `[0.4270, 0.4403]` | 52.62× | 15/15 |
| Reduced `mmax=180`, q1 | 217.487 | 220.628 | 8.334 | 1.0128 `[1.0116, 1.0178]` | 26.39× | 0/15 |
| Reduced `mmax=180`, q8 | 111.686 | 90.635 | 1.214 | 0.8116 `[0.8096, 0.8170]` | 74.70× | 15/15 |

Rust parent: `0677b5bfe98117c1d1a5d5204b0a710828fcb9c8`; the production
`src/` tree at that exact parent matched the source at the start of this
experiment. Promoted source: `9dc4516080eb0b25711ae74825c634ca71166ffa`.
The patch used to build the timed v1 candidate is
[`candidate.patch.gz`](candidate.patch.gz), with uncompressed SHA-256
`c7c0a9c87e4002ae7c4643086df0970721c825d0a7abb9aadf9ecc556969e0e9`. An exact
parent-to-promoted-source patch is in
[`promoted-source.patch.gz`](promoted-source.patch.gz).
Pinned DUCC 0.42.0: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

Both Rust binaries used release mode, `--features parallel`, Rust 1.99.1 / LLVM
23.1.1, and `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. DUCC numerical
sources used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; its driver
used `-fno-fast-math -ffp-contract=fast`. All variants used the same GL
geometry (`ntheta=721`, `phi0=0.23`), coefficient fixtures, output length,
warm-call boundary, and requested thread count. Full mmax used `nphi=1458`;
reduced mmax used `nphi=378`. The fixture, executable, and output hashes plus
all per-pair samples are retained in [`timings.json`](timings.json).

Candidate and parent outputs were bitwise equal in all 60 primary paired
comparisons. Every worker reported finite output and passed repeat and poisoned
workspace-reset checks. Candidate/DUCC relative L2 and maximum absolute error
were the same as the parent: full mmax `1.72343e-4` and `9.64404e-4`; reduced
mmax `2.72546e-4` and `1.54787e-3`.

Adjacent f32 controls covered full-mmax scalar analysis and spin-1 analysis and
synthesis at q1/q8 (three pairs each). Candidate/parent ratios were
`0.9662`/`0.9604` for scalar analysis, `0.9999`/`0.9994` for spin-1 analysis,
and `0.9828`/`0.9755` for spin-1 synthesis. Protected f64 controls covered
T720 scalar synthesis and T1023 spin-1 synthesis at q1/q8 (three pairs each);
ratios were `0.9957`, `0.9999`, `0.9983`, and `0.9979`. The complete
per-worker records and accuracy comparisons are in `timings.json`.

## Exact fallback route census

Temporary thread-safe counters were enabled only in an isolated instrumented
build. Counts below were identical across three runs and between q1 and q8 for
the same mmax; the production benchmark binaries contained no counters.

The source trace classifies about 79.8% of contribution states as normal-range,
but only 37.6% of contribution work lies in blocks eligible for fusion at the
initial seed. Fusion eligibility is a block-level all-lane check: one
polar-excluded lane or one scaled/unsafe seed sends the whole block to the
fallback. A later normal contribution inside that block does not satisfy the
initial fused-entry proof.

| Route per scalar order block | Full `mmax=720` | Reduced `mmax=180` |
|---|---:|---:|
| Total blocks | 8,652 | 2,172 |
| Polar-inactive / excluded | 1,937 | 28 |
| Mixed polar fallback | 1,216 | 152 |
| Initially normal and fully fused | 1,744 | 922 |
| Initially scaled, scalar recurrence fallback | 3,755 | 1,070 |
| Fused declines / transactional replay | 0 / 0 | 0 / 0 |
| Spectrum lanes staged on the prepared path | 347,095 | 122,647 |

The mixed adapter declines before touching spectra when polar-excluded lanes
are present, compacts the active contiguous span, and accumulates through the
scaled callback. The prepared all-active path stages prior spectrum values,
initializes its recurrence, and fuses only when every seed lane is initially
normal. If the seed is scaled, it runs the scalar recurrence once and commits
once. No block transitioned from scaled to fused, and no retry/replay occurred
in this census.

| Fallback work | Full `mmax=720` | Reduced `mmax=180` |
|---|---:|---:|
| Recurrence callbacks | 937,595 | 377,037 |
| Degree–latitude contribution visits | 52,699,590 | 21,457,620 |
| Useful recurrence updates | 52,423,706 | 21,387,896 |
| Exponent-zero visits | 39,534,760 | 17,483,500 |
| Nonzero-exponent visits | 13,164,830 | 3,974,120 |
| Scaled helper calls in the mixed route | 12,587,348 | 3,376,880 |
| `multiply_complex_scaled_f32` calls on nonzero exponents in the all-active route | 7,891,996 | 2,698,785 |

Full-mmax mixed blocks account for 6,293,674 of the contribution visits and
6,253,504 useful recurrence updates; 40,170 final-pair visits do not advance a
recurrence. Reduced mixed blocks account for 1,688,440 visits and 1,682,590
updates. The all-active scaled route accounts for the remaining contribution
work. Thus retry work is zero; the dominant avoidable cost is per-contribution
scalar fallback recurrence, accumulation, and scaling across millions of
degree–latitude visits, not eligibility scanning or replay.

The mixed active-lane scan/copy timer sums to about 0.12 ms for full mmax and
0.016 ms for reduced mmax. The broader fallback timer, which includes the
recurrence and its accumulation callback, sums across worker jobs to about
780/795 ms full q1/q8 and 247/248 ms reduced q1/q8. These are instrumented
thread-work sums, not transform wall time; this instrumentation did not isolate
coefficient accumulation, recurrence arithmetic, and scaling into independent
elapsed-time timers. The Callgrind attribution below provides instruction
costs for the helper/callback path, with nested inclusive counts kept distinct.

The mixed-span screen also tried to count safe contiguous normal runs against
the existing width-eight fused leaf. Its optimistic disjoint partition could
cover only 12.9% of mixed full-mmax pair/lane contributions, before setup or
declines. That bound is not a basis for another mixed-lane or per-vector retry
path; see the [lane-level profile](../f32-mixed-span-route-profile-2026-10-27/README.md).

## q8 scheduling profile and next bottleneck

At the exact parent, 12 64-ring groups were batched with
`min_groups_per_task=2`; only four of eight workers received work, with
7.35–7.48× max/min summed group-time skew. The candidate changes only the
minimum to one for f32 GL scalar synthesis. It schedules 12 groups across all
eight workers; full-mmax max/min worker group-time skew fell to 1.93–2.92×.

The instrumented reduced-mmax candidate run 0 shows where q8 time goes:

| Worker | Jobs | Summed group time |
|---:|---:|---:|
| 0 | 1 | 98.98 ms |
| 1 | 2 | 65.94 ms |
| 2 | 1 | 11.02 ms |
| 3 | 1 | 25.97 ms |
| 4 | 1 | 22.87 ms |
| 5 | 3 | 19.72 ms |
| 6 | 1 | 17.18 ms |
| 7 | 2 | 36.13 ms |

These per-worker job counts and elapsed sums are instrumentation diagnostics,
not wall-time partitions; the full three-run worker records are in
`route-summary.json` and the ring-indexed group CSVs are in
`route-profile.tar.gz`.

Reduced mmax still has severe ring-dependent job cost. The three instrumented
q8 runs repeatedly measured these 64-ring jobs (approximate elapsed ranges):

| Ring range | Group time |
|---|---:|
| 0–64 | 99–100 ms |
| 64–128 | 26 ms |
| 128–192 | 17 ms |
| 192–256 | 11 ms |
| 256–384 | 1.6–1.9 ms per 64 rings |
| 384–448 | 1.8–1.9 ms |
| 448–512 | 8 ms |
| 512–576 | 16 ms |
| 576–640 | 23 ms |
| 640–704 | 64–65 ms |
| 704–721 | 28 ms |

The very different work by latitude explains why finer task scheduling alone
does not balance reduced mmax. The next bounded screen is to split the 64-ring
jobs into 32-ring jobs only for f32 GL scalar synthesis, keeping the recurrence
and fallback unchanged. Record ring-indexed worker times, scratch growth, and
all four q1/q8 cells before considering promotion. The fallback arithmetic
remains the following measured target.

## DUCC algorithm and emitted AVX2 comparison

The matched operation is the same GL scalar synthesis over the same `lmax`,
`mmax`, latitude/longitude geometry, coefficients, and output values. RustSHT
uses native f32 and eight AVX2 lanes. Its fused path is an eight-lane
`vfmadd*ps` recurrence/accumulation loop with range checks, but only the
initially all-normal blocks enter it. Scaled and polar-mixed work remains in
per-contribution callbacks; exponent-zero visits still perform accumulation,
while nonzero exponents call scaling helpers.

Pinned DUCC's f32 adapter promotes recurrence and accumulation to f64. Its
paired recurrence advances through a scaled prefix and then passes a normal
suffix to a dynamically scheduled, latitude-batched `alm2map_kernel`. The
inspected AVX2 kernel uses four double lanes and twelve vector FMAs per
four-pair body, unit-stride memory operations, and no hot-loop spills. This
algorithmic partition and job scheduling differ from RustSHT's block-level
initial eligibility and latitude grouping.

The exact-parent/current-source Callgrind capture for full-mmax q8 reports
8,366,416,331 Rust Ir and 252,802,007 DUCC Ir (about 33.1× as many Rust
instructions). This is an instruction-reference ratio, not a time ratio. The
Rust paired fallback iterator has 4.193B inclusive Ir; its callback has 1.443B
self Ir. The all-active nonzero-exponent helper was called 7,891,996 times and
has 1.657B inclusive Ir; the mixed helper call site has 12,587,348 calls and
3.067B inclusive Ir. These nested inclusive values overlap and must not be
added. The fused leaf ran 1,744 times and accounts for 161.3M inclusive Ir.
The same whole-transform timing cell remains 52.62× DUCC, demonstrating that
Ir and elapsed time answer different questions.

The current-source profile records the emitted Rust f32 leaf and the matched
DUCC source comparison; the isolated route and timing evidence are summarized
in [`f32-current-source-profile-3189fe8e`](../f32-current-source-profile-3189fe8e/README.md),
the [safe-suffix analysis](../f32-safe-suffix-handoff-2026-10-27/README.md),
and [`route-summary.json`](route-summary.json). There is no new f32/f64
algorithm or exceptional-path change in this candidate.

## Rejected alternative

An out-of-line `#[inline(never)]` policy helper made q1 appear neutral but
regressed adjacent f32 GL T720 spin-1 analysis: candidate/parent was 1.0076 at
q1 and 1.0198 at q8 in all three pairs. The helper variant is rejected and
retained only as
[`rejected-helper-variant.patch.gz`](rejected-helper-variant.patch.gz)
and [`rejected-helper-variant.json`](rejected-helper-variant.json). Its
primary q8 gains were comparable; the inline policy used by the promoted
candidate avoids this neighboring-path regression.

## Numerical and validation checks

- Focused paired-scalar tests: 34 passed, 8 ignored. They cover polar/mixed
  lanes, scaled recurrence and SIMD tails, unsafe overflow, subnormal range
  declines, nonfinite coefficient rejection, and scalar fallback behavior.
- f32 integration tests: 11 passed, 1 ignored. These cover independent DUCC
  references, analytic fields, poles, repeated workspace use, scalar and
  spin adjoints, and high-degree values.
- T720 low-mode analytic relative L2 / max absolute error:
  `7.2320e-8` / `5.9605e-8`; recovered coefficient relative L2 / max absolute
  error: `4.4978e-5` / `4.5896e-6`; adjoint relative error: `2.1052e-5`.
- Sectoral degree-720 analytic relative L2 / max absolute error:
  `1.0714e-5` / `5.8889e-5`.
- Minimum normal, smallest subnormal, large finite, and maximum finite
  coefficient cases all produced finite output. Repeated output was bitwise
  identical. Existing paired-synthesis tests preserve overflow and nonfinite
  transactional declines; no tolerance was changed.
- At the promoted source, `cargo fmt --all --check`, `git diff --check`,
  `cargo test --locked --offline --features parallel --lib
  legendre::scalar_paired` and `cargo test --locked --offline --features
  parallel --test f32` passed.

Machine-readable per-pair timing, fixture, binary, output-hash and error data
are in [`timings.json`](timings.json); route counters and per-worker/group
profiles are in [`route-summary.json`](route-summary.json) and
[`route-profile.tar.gz`](route-profile.tar.gz). The isolated instrumentation
diff is [`instrumentation.patch.gz`](instrumentation.patch.gz). The exact
interleaving, adjacent-control, and analytic-check drivers are in
[`runner-source/`](runner-source/); they use the checked project benchmark
helpers and contain no compiled binaries. Checksums are listed in
[`checksums.sha256`](checksums.sha256).
