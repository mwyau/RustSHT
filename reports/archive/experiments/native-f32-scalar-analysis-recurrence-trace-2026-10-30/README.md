# Native-f32 scalar-analysis recurrence trace

This diagnostic records the ordinary paired recurrence in the isolated B1
subnormal-prefix candidate. It checks the actual typed SIMD block and replays
its saved f32 operands with exact rational arithmetic. It is a recurrence
causality result, not an end-to-end transform accuracy result or a performance
measurement.

## Source and reproducibility

- Base source: `053c5668079fa89f37438a488b2545b5ceab1e69`.
- Candidate production diff: the B1 patch in
  `../native-f32-scalar-analysis-subnormal-prefix-2026-10-30/candidate.patch.gz`;
  uncompressed SHA-256 `645951812d6dc995c3921d16f32af4091f5d67ed46c2259fce897a0f934320c5`.
- Trace-test diff, applied after that candidate patch:
  `trace-test-after-b1.patch.gz`, SHA-256 recorded in `SHA256SUMS`.
- The candidate source patch is unchanged by instrumentation. The trace test
  calls `fused_scalar_analysis_pair_block_preweighted_typed` directly, with one
  nonzero spectral lane so its staged values expose the recurrence state.
- Reverification on 2026-10-30: the ignored trace test passed in a clean
  worktree based on the exact source and both generated CSV bytes matched
  `analysis-f32-f64-avx2-t720.csv.gz` exactly.

For a clean checkout at the base commit, apply the candidate patch first, then
run `gzip -dc trace-test-after-b1.patch.gz | git apply`. Run:

```sh
RUSTSHT_TRACE_CSV=/path/to/analysis.csv \
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
RAYON_NUM_THREADS=1 \
cargo test --release --features parallel --lib \
  trace_native_f32_analysis_recurrence -- --ignored --nocapture
```

The trace asserts the AVX2 widths are eight f32 lanes and four f64 lanes. The
bounded cases cover `m={0,1,2,5,8,16}` and north/equatorial rings selected from
`{0,1,5,360}`. The input to each recurrence block is synthetic and one-hot;
there is no longitude FFT or public transform call in this trace.

## Numerical result

For `m=0`, ring `0`, all saved f32 recurrence operands are finite and the scale
exponents remain zero. The first stored state that differs from exact replay
with the same prepared f32 operands is degree `4`, by one ULP. At degree `720`
the actual state differs by `0.0010084386591864658` absolute, `3.5731552e-2`
relative to that state, and `2.5570953e-2` under the local recurrence-state
scale used by the replay. Relative error and ULP counts become unstable near
state zeros, so the absolute and locally scaled values are more useful there.

Across all 22 selected `(m, ring)` cases, the first stored-state difference
from the exact same-f32-operand replay occurs in every case. For `m=0`, ring
`0`, the recurrence's leading-factor FMA and state-update FMA are correctly
rounded at every one of the 360 updates. The first leading-factor difference
between FMA and separately rounded multiply/add occurs at degree `0` and is
one ULP; it is a contraction difference, not a misrounded FMA. The exact replay
therefore points to the repeated rounding of stored f32 recurrence state as an
error source, rather than an AVX2-versus-scalar mismatch or a bad individual
FMA result. The measured trajectory is oscillatory; local error is amplified
as the recurrence evolves and can become large relative to near-zero states.

The selected recurrence cases do not exercise a nonzero scaling exponent, so
this trace does not diagnose scaled-prefix accuracy or the exceptional range.
It also does not compare DUCC, coefficient accumulation, the FFT, or final
maps. The complete bounded bit traces, exact-rational replay, summary script,
and AVX2 assembly are retained in this directory; the CSV is compressed to
avoid storing its multi-megabyte text form.

## Evidence files

- `analysis-f32-f64-avx2-t720.csv.gz`: recurrence inputs and before/after
  f32/f64 states, including bit patterns.
- `controlled-replay.json`: exact-rational same-operand replay and selected
  preparation sensitivity results.
- `summarize_trace.py`, `controlled_replay.py`: replay and summary scripts.
- `f32-avx2-recurrence.asm`: compiled f32 recurrence instructions.
- `trace-test-after-b1.patch.gz`: ignored test-only instrumentation patch.
- `SHA256SUMS`: checksums for every retained file.
