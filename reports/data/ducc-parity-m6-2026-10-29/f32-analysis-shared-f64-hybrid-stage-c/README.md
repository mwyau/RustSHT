# f32 GL T720 scalar analysis with the shared f64 recurrence

## Decision

The native-f32 FFT plus reusable shared-f64 analysis backend is a large
improvement over the native-f32 analysis parent, but it still misses the DUCC
target in all four cells. The 15-pair Rust/DUCC ratios are `1.129–1.368`; keep
this as an isolated diagnostic and continue Stage C. Do not promote the route
yet.

The follow-up that avoided full f32-spectrum promotion and cast f32 values on
demand also remained above DUCC in its 3-pair screen (`1.140–1.291`). It did
not justify further work on that dataflow. Its exact patch and screen are
retained here.

## Provenance and timing contract

- Source parent: `d32e7b4c7ba818f0a92ddf5068ba7bce3bd44699`.
- Baseline f32-I/O/f64-analysis source patch SHA-256:
  `0b7785c1ca43bed3f5acf233cd56ab0ec4bf9315d04f6842f2b74f646af24844`.
  It reuses `FusedScaledScalarAnalysisPair`, the paired recurrence visitor,
  and `F32SynthesisAccumulator`; no recurrence was copied.
- DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0).
- Rust: rustc 1.99.1 / LLVM 23.1.1, `--release --features parallel`,
  `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.
- DUCC: GCC 15.2, `-O3 -ffast-math -ffp-contract=fast
  -march=x86-64-v3`; the benchmark driver uses `-fno-fast-math`.
- Host: Ryzen 9 5950X with AVX2/FMA. q1 affinity `[0]`; q8 affinity
  `[0,2,4,6,8,10,12,14]`. Each of the four GL T720 spin-0 analysis cells
  used 15 interleaved fresh-process parent/candidate/DUCC comparisons with a
  25 ms warmed-call target. Plan/workspace setup was amortized. DUCC used its
  native-f32 API.
- Input fixtures and hashes are the Stage B catalog in
  [`fixtures-sha256.json`](../f32-io-f64-internal-stage-b/fixtures-sha256.json).
  The benchmark worker source is the repository's
  `examples/ducc_f32_full_transform_parity.rs`; this package retains the
  paired manifest, candidate patches, binary hashes, Callgrind profiles, and
  a portable runner snapshot for the direct-spectrum screen.

## Whole-transform measurements

Times are medians in milliseconds. Candidate/DUCC confidence intervals are
paired bootstrap 95% intervals. Candidate output error is against DUCC f32.

| Cell | Native f32 parent | f32 I/O / f64 analysis | DUCC f32 | Candidate/parent | Candidate/DUCC [95% CI] | Max rel-L2 | Status |
|---|---:|---:|---:|---:|---:|---:|---|
| Full q1 | 448.376 | 16.224 | 13.940 | 0.0362 | 1.1618 [1.1584, 1.1668] | 1.46e-7 | GAP |
| Reduced q1 | 138.119 | 6.422 | 5.697 | 0.0465 | 1.1288 [1.1242, 1.1341] | 1.34e-7 | GAP |
| Full q8 | 59.292 | 3.018 | 2.241 | 0.0509 | 1.3679 [1.3430, 1.3823] | 1.46e-7 | GAP |
| Reduced q8 | 19.055 | 1.264 | 0.987 | 0.0667 | 1.2745 [1.1966, 1.3365] | 1.34e-7 | GAP |

Candidate outputs were finite, repeat-consistent and reset-consistent. The
max absolute errors versus DUCC were `1.40e-9` full and `2.33e-9` reduced.

## Profile evidence and next hypothesis

The q1 full Callgrind run collected `2,442,717,071` instruction references
over eight public calls. The fused
`FusedScaledScalarAnalysisPair<true,false>` SIMD leaf accounted for
`658,537,208` self Ir (26.96%). The f64 `analyze_precomputed_spectra_into`
call tree accounted for `1,769,975,727` Ir inclusive (72.46%); this includes
the fused leaf and must not be added to its self cost. The f32 row FFT,
spectrum promotion, and input-range scan accounted for 13.31%, 6.47%, and
5.51% of collected Ir. DUCC's full worker collected `1,745,044,542` Ir
including setup; its `map2alm_kernel` accounted for `889,747,224` Ir
(50.99%). These are instruction references, not CPU cycles.

The shared f64 recurrence path already executes an AVX2 SIMD leaf. In the
reflected-spectrum preparation source, `ScalarAnalysisOrderSpectra::value`
repeats the same per-ring `ring_weight / nphi` operation for each order.
That repeated work is the next bounded hypothesis: cache the rounded
per-ring-per-plan quotient and reuse it without changing the arithmetic
order. The whole-call A/B result must establish whether this improves the
four analysis cells and leaves adjacent controls intact.

The direct-spectrum variant's 3-pair screen results are in
`rejected-direct-spectrum-3.tsv`; the baseline 15-pair records are in
`results-15.json.gz` and `results-15.tsv`. Callgrind profiles, runner source,
patches, executable hashes, and checksums are stored alongside this file.

## Validation actually run

- `cargo fmt --all -- --check` — passed.
- `cargo check --locked --release --features parallel` — passed.
- Focused internal f64 route / exceptional transactional fallback unit tests
  — 2 passed.
- T720 analytic scalar modes, adjoints, poisoned-workspace reset and q1/q8
  public-call checks — passed; analytic relative-L2 was `9.24–9.28e-8`.
- The four public-call performance cells used 15 interleaved pairs each.
  Protected f64 T1023 spin-1 synthesis and all adjacent spin performance
  controls were not rerun for this analysis-only diagnostic; do not infer
  their status from this report.
