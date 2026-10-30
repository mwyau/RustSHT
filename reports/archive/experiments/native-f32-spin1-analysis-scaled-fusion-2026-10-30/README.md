# Native-f32 spin-1 analysis scaled-channel fusion screen

**Decision:** keep this patch isolated. It improves all four f32 times in a
three-pair screen, but every cell remains above the interim 2.0x matched-f64
gate. It is not a production or accuracy acceptance.

## Source and method

- Parent source: `9cb2788cd3f18c0f262e24dff6fac58236e5a18d`.
- Candidate base: `be39b56137f45ac9bfceda4ccc497109c70fd34d`, plus the isolated
  `src/legendre/scalar_paired/analysis.rs` patch. Parent and candidate base have
  identical tracked `src/` trees. The uncompressed patch SHA-256 is
  `06f0fa7cb578b1e2ff0a1a784ff476e758894c8126b5207dd764b6525fa8b434`.
- The f32 specialization fuses the same/opposite accumulations for both spin
  channels while sharing basis and cosine loads. The f64 trait default still
  calls the existing accumulator once per channel. Recurrence equations,
  coefficient order and public dispatch are unchanged.
- GL T720 full M720/nphi1458 and reduced M180/nphi378, q1/q8, three interleaved
  fresh-process pairs per cell, `phi0=0.23`. Fixtures come from
  `scripts/benchmark_ducc_f32_native.py`; their hashes are in the JSON.
- Rust 1.99.1, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, AMD Ryzen 9 5950X;
  q1 pinned to CPU 0 and q8 to CPUs 0–7. The candidate and DUCC workers use
  native f32. Matched Rust f64 workers widen those same f32 fixture values only
  for their diagnostic comparison. DUCC is pinned to
  `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Results

Times are medians in milliseconds. The f32/f64 ratios use the corresponding
same-source f64 worker; f32/DUCC is reported separately.

| Cell | Parent f32 | Parent f64 | Parent ratio | Fused f32 | Fused f64 | Fused ratio | Fused/parent f32 | DUCC f32 | Fused/DUCC |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Full q1 | 131.634 | 37.621 | 3.499x | 115.008 | 37.560 | 3.063x | 0.874x | 30.110 | 3.820x |
| Full q8 | 19.850 | 6.558 | 3.027x | 16.932 | 6.075 | 2.745x | 0.795x | 4.519 | 3.747x |
| Reduced q1 | 57.132 | 15.562 | 3.738x | 49.929 | 15.225 | 3.243x | 0.882x | 12.345 | 4.044x |
| Reduced q8 | 8.821 | 2.775 | 3.293x | 7.812 | 2.798 | 3.025x | 0.888x | 2.078 | 3.928x |

The f32 improvement over the paired parent is 11–21%, but the best fused cell
is still 2.745x f64. All four are timing GAPs; this is screening evidence, not
a 15-pair performance claim. The emitted V3 path executes eight-lane AVX2/FMA.
The inspected fused symbol has eight packed-f32 FMA accumulators and no YMM
stack spills, so this candidate's remaining gap is not caused by AVX2 register
spills.

The patch only removes duplicate channel traversal in the scaled accumulator.
For nonordinary sectoral seeds, `fused_scalar_analysis_pair_channels_block_native`
still runs `NativeScaledFusedAnalysisPairChannels`, which carries lane
exponents, prepares per-lane scale factors, handles zero-factor lanes through
the guarded scalar path, and advances the scaled paired recurrence for each
degree pair. This candidate does not remove that scale and fallback work. Its
whole-call screen misses the gate, while the precise time share of those
suboperations remains unmeasured. Stop this bounded fusion experiment rather
than extending the same accumulator microkernel.

## Numerical and functional checks

- `cargo fmt --all -- --check`: passed.
- `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' cargo test --release --features parallel --lib legendre::scalar_paired::analysis::tests`:
  36 passed, 8 ignored. This includes the native-f32 scaled-seed comparison
  against the checked recurrence.
- `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' cargo test --release --features parallel --test f32 f32_spin`:
  3 passed, including all-grid spin operation/adjoint checks and pinned T511
  DUCC analysis.
- Fused and parent f32 outputs are byte-identical for full q1 and reduced q8;
  output hashes are in `output-check.json`. The random fixture's f32-vs-DUCC relative L2 is
  `1.84695e-5` full and `2.17137e-5` reduced. These checks do not establish
  independent high-degree accuracy or a scientific tolerance.

No native-f32 accuracy PASS is assigned. The independent selected high-band
binary128 screen remains the one in the [cross-family accuracy package](../native-f32-cross-family-accuracy-2026-10-30/README.md).

## Reproduction artifacts

- `scaled-fusion.patch.gz`: recoverable source change; its uncompressed SHA is
  recorded above.
- `t720-paired-result.json`: per-process times, fixture and worker hashes,
  dispatch, DUCC comparison errors, compiler, flags and source provenance.
- `output-check.json`: bitwise parent/candidate output comparison for two
  representative geometry/thread cells.
- `run_fusion_paired.py`: screen driver. Set `RUSTSHT_M6_OUT`,
  `RUSTSHT_M6_PARENT_F32`, `RUSTSHT_M6_CANDIDATE_F32`,
  `RUSTSHT_M6_PARENT_F64`, `RUSTSHT_M6_CANDIDATE_F64`, and
  `RUSTSHT_M6_DUCC_F32` to the output directory and worker executables.
- `f64-worker/`: source and Cargo manifest template for the matched f64 worker.
- `AVX2_HOT_LOOP.txt`: compact disassembly excerpt and spill count.
- `SHA256SUMS`: checksums for the package artifacts.

The separate whole-transform timings were never taken from instrumented
binaries. No production code, benchmark fixture, or public API was changed by
this candidate.
