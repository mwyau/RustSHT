# Native-f32 spin-1 analysis adaptive reflected-range screen

## Scope and provenance

This isolated B3 candidate adds precision-specific seed-range detection and an adaptive reflected-ring block schedule to the shared spin-1 analysis route. It runs native-f32 recurrence, accumulation, spectra, FFT and map output. It does not promote transform inputs or recurrence state to f64. The base is `00f9770741a6e3e84646217d51374e35515daaa1`; the three-file source diff SHA-256 is `a6ba43101cdd1bb7c61c315b9100748954296a78a345ec197456a61c59017120`. The recoverable patch is [candidate.patch.gz](candidate.patch.gz).

The schedule uses an f32-specific tile of 32 for the scale-sensitive prefix. Once the first active sectoral seed is in the ordinary range, it returns to the 256-pair tile. The f64 monomorphization keeps the existing 256-pair path. Prefix blocks bypass the range cache so their entries cannot alias the suffix cache. The shared recurrence and accumulation kernel itself is unchanged.

## Matched GL T720 screen

Three interleaved process pairs measured the isolated candidate, its parent f32 route, matching-source f64 and pinned native-f32 DUCC 0.42.0. Inputs were the same deterministic f32 fixtures; the f64 worker widened those fixture components for diagnosis. Workers used Rust 1.99.1, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, AVX2/FMA, Ryzen 9 5950X, q1 CPU 0 and q8 CPUs 0–7. The exact patch, worker and fixture hashes and per-pair data are in [screen-3pair.json](screen-3pair.json).

| GL T720 cell | Candidate f32 ms | Matched f64 ms | f32/f64 | DUCC f32 ms | f32/DUCC | Candidate/parent | Status |
|---|---:|---:|---:|---:|---:|---:|---|
| Full q1 | 117.761 | 37.026 | 3.192x | 29.285 | 4.021x | 0.896x | GAP |
| Full q8 | 15.891 | 5.808 | 2.732x | 4.181 | 3.845x | 0.921x | GAP |
| Reduced q1 | 47.530 | 15.195 | 3.128x | 11.897 | 3.958x | 0.853x | GAP |
| Reduced q8 | 6.525 | 2.220 | 2.940x | 1.746 | 3.729x | 0.865x | GAP |

The candidate improves parent f32 time by about 8–15%, but misses the interim `<=2.0x` f64 gate in all four cells and remains 3.7–4.1x DUCC. A bounded tile-16/32/64 sensitivity screen found tile 32 best; tile 16 or 64 was neutral or slower. See [tile-sensitivity-3pair.json](tile-sensitivity-3pair.json). Three pairs are screening evidence, not a consequential performance claim.

The f64 worker rebuilt from the candidate source produced the same sampled output checksums as the base-source f64 worker in all four cells: `-0.00081491247818932` for full M720 and `0.0006040397208557` for reduced M180. Exact checksums and source hashes are in [f64-source-equivalence.json](f64-source-equivalence.json).

## Numerical check and validation

An independent binary128 direct-Fourier comparison used GL T720, `mmax=720`, `phi0=0`, 11 selected orders and `l=600..720`. The measured relative L2 errors were `4.9083e-5` for scalar-analysis coefficients, `4.8307e-5` for spin-analysis E and `5.3302e-5` for spin-analysis B; widened f64 controls were about `0.9–1.0e-13`. q1 and q8 outputs were byte-identical for these artifacts. This is a selected high-band check, not a full-array accuracy result or an application-tolerance PASS. The direct Fourier reference does not split recurrence, accumulation and FFT errors. The machine-readable measurements and source hashes are in [binary128-selected-band.json](binary128-selected-band.json); the reference source is [the archived cross-family harness](../native-f32-cross-family-accuracy-2026-10-30/phasec_fullorder_accuracy_reference.cpp).

`cargo fmt --all -- --check`, strict library Clippy, the focused spin-analysis unit tests (3 passed), and the f32 integration suite (14 passed, one opt-in T1023 test ignored) passed. See [validation.txt](validation.txt). No source change is promoted: all four interim cells remain GAP, and the selected high-band accuracy has no declared science tolerance.

## Decision

Adaptive scheduling reduces the cost of the scale-sensitive reflected prefix, but this block-size change alone does not reach the interim target. Tile 16 and 64 do not improve on tile 32. Keep the candidate isolated, preserve its numerical status as OPEN, and continue to the next family in the active owner order. Do not infer that f32 precision itself is the performance cause; this screen does not include suboperation profiling.

`SHA256SUMS` covers every file in this package except the manifest itself. The complete benchmark binaries and multi-megabyte output arrays remain in the local experiment worktree; the archived inputs, measurements, hashes, reference outputs and patch are sufficient to reproduce or verify the summarized result.
