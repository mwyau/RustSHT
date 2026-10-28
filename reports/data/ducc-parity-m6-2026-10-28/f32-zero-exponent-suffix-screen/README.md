# Native-f32 zero-exponent suffix screen — rejected

**Decision:** do not promote the guarded scaled-prefix-to-fused-suffix
candidate. Its first three-pair screen regressed three of four GL T720
whole-transform cells. The reduced-q8 result is favorable but noisy, and the
candidate added 4.77% q1 full-mmax Callgrind instruction work. The existing
scaled packet leaf remains the production path.

## Provenance and measurement

- Exact Rust parent: `ef11ce0a1043d8acb049ce9178513e4cde058789`.
- Candidate: isolated patch SHA-256
  `74d9bfcfcae98e57e87944058baea44afcaaf1c3a78782780dbfd47af6278666`; it was
  never applied to the branch.
- Pinned DUCC 0.42.0:
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Host: AMD Ryzen 9 5950X; Rust 1.99.1 / LLVM 23.1.1; GCC 15.2.0. Rust and
  DUCC used x86-64-v3 AVX2/FMA builds. q1 used one fixed physical core; q8
  used eight distinct fixed physical cores. Host-local CPU IDs are omitted.
- Fixtures: GL T720 (`ntheta=721`, `phi0=0.23`), `lmax=720`; full
  `mmax=720`, `nphi=1458`; reduced `mmax=180`, `nphi=378`. Parent, candidate,
  and DUCC used the same f32 inputs. Fixture hashes are in
  [fixture-input-sha256.json](fixture-input-sha256.json).
- Three interleaved fresh-process parent/candidate/DUCC pairs per cell. Each
  worker checked finite output, repeat consistency, and workspace-reset
  consistency. Whole-transform measurements were warm and uninstrumented.
  Separate Legendre-stage results are medians of seven warm calls.

## Whole-transform results

Times are medians in milliseconds; Rust/DUCC and candidate/parent values are
medians of paired ratios. The screen stopped at three pairs because the main
cells regressed.

| GL T720 f32 scalar synthesis | Parent ms | Candidate ms | DUCC ms | Candidate/parent | Parent/DUCC | Candidate/DUCC | Status |
|---|---:|---:|---:|---:|---:|---:|---|
| Full q1 | 670.311 | 688.252 | 19.362 | 1.0274 | 34.44× | 35.32× | **GAP; reject** |
| Full q8 | 101.033 | 104.519 | 4.972 | 1.0327 | 19.90× | 21.02× | **GAP; reject** |
| Reduced q1 | 199.219 | 208.068 | 8.489 | 1.0401 | 23.59× | 24.57× | **GAP; reject** |
| Reduced q8 | 57.761 | 54.572 | 1.430 | 0.9448 | 39.89× | 39.27× | **GAP; inconclusive** |

All three full-q1/full-q8/reduced-q1 pairs favored the parent. Reduced q8
favored the candidate in two of three pairs; no extension was justified after
the other cells regressed. These are initial screens, not 15-pair acceptance
results.

The Legendre-stage median changed by `+3.76%` full q1, `+5.32%` full q8,
`+2.22%` reduced q1, and `-5.98%` reduced q8. The stage and whole-transform
directions agree for three cells; reduced q8 is the exception.

## Route and instruction evidence

The candidate split each existing 32-ring fallback block into separate
8-lane packet traversals so a packet could stop after its safe suffix. In the
full-q1 Callgrind profile the candidate entered 3,755 fallback-block
traversals; the fused normal block was entered 1,744 times. The candidate did
not add dynamic counters for completed/declined suffixes, so the earlier
37.62M safe-suffix-slot census is route opportunity from the exact-parent
state replay, not a claim about candidate-completed work. See the
[real-state transition replay](../f32-exact-zero-operand-suffix-rejection/README.md).

One warmed full-q1 `synthesis_into` call collected `6,610,413,955` parent Ir
and `6,925,618,870` candidate Ir (`+4.77%`). Against the pinned DUCC
same-geometry reference of `260,801,202` Ir, the equivalent work was
`41.01` parent, `42.96` candidate, and `1.62` DUCC Ir per required
degree-latitude slot (`161,199,393` slots). DUCC's Ir reference comes from the
same-shape pinned profile in the accepted packet-vector screen; it was not
reprofiled for this candidate. Ir are instruction references, not cycles or
elapsed-time estimates. The machine-readable counts are in
[callgrind-summary.json](callgrind-summary.json); compressed profiles are
provided for the parent and candidate.

The emitted V3 code shows the normal `FusedSynthesisBlock<f32, true, false>`
symbol growing from `0x6ec` (1,772 bytes) to `0x130f` (4,879 bytes). The
candidate also adds a 10,012-byte `F32SynthesisFallback` symbol. Both fused
versions use AVX2 YMM `vfmadd*ps`; the candidate's fallback symbol contains
the added transition and suffix path. This code-size growth and four-way
packet traversal are concrete costs; the measurements do not isolate how
much each caused the instruction increase. See the compressed matched V3
assembly.

## Correctness and validation

Candidate vs parent maximum relative-L2 error was below `1.2e-7`; maximum
absolute error was below `2.1e-7`. Candidate vs DUCC remained at relative-L2
`1.7234e-4` full and `2.7255e-4` reduced, with maximum absolute error
`9.6441e-4` and `1.5479e-3`. All primary workers were finite, repeat-consistent,
and reset-consistent.

The isolated candidate passed seven scalar-paired synthesis module tests and
the f32 integration suite (12 passed, 1 ignored), including analytic,
adjoint, polar, and reset cases. The suffix-specific tests cover a zero-
exponent handoff and transactional overflow decline. The broad exceptional
matrix and protected f64 timing controls were not run because the primary
screen rejected the candidate; no production code changed.

## Next measured hypothesis

The large remaining work difference is reflected-ring reuse. At the prior
same-geometry DUCC/Rust work census, Rust computed `161.20M` degree-latitude
slots and `80.31M` useful paired-state updates over both hemispheres; DUCC
computed both outputs from `80.73M` northern-hemisphere degree terms with
`40.32M` useful recurrence updates (plus `40.56M` padded lane updates). The
f32-I/O/f64-internal diagnostic also showed that the reflected order-major
route is near DUCC while forcing the all-ring paired route remains several
times slower. See the [work-count diagnostic](../f32-internal-double-diagnostic/README.md).

Next, measure a **synthesis-only** shared-recurrence reflected-ring route.
Keep scalar analysis and f64 dispatch untouched, and do not restore the prior
duplicated reflected f32 implementation, which regressed reduced scalar
analysis q8. Count useful recurrence work and stage time before coding the
candidate. M6 remains open.

## Artifacts

- [Paired results](paired-results.json)
- [Legendre-stage samples](stage-profile.json)
- [Numerical summary](numerical-summary.json)
- [Callgrind counts](callgrind-summary.json)
- `candidate.patch.gz` is the exact rejected source/test patch.
- `stage-profile-harness.patch.gz` reproduces the separate stage timing hook.
- The two compressed Callgrind files and three compressed V3 assembly files
  preserve the focused attribution evidence.
