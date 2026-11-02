# B3 partial scaled-exception vector helper screen

**Disposition:** rejected at the helper gate; no whole-transform benchmark or
production change. The measured partial-lane AVX2 helper is substantially
slower than the existing scalar exceptional-lane path on the same inputs.

## Result

On pinned source `3d4c6150c50ff8a2b399eaa22c83f50406b83c20`, the test-only
microbenchmark compared three exceptional f64 lanes in one four-lane AVX2
vector, for both reflected complex components and two channels. The candidate
masks the ordinary lane and calls the existing checked scaled-vector helper;
the control calls `multiply_complex_scaled` and
`multiply_complex_scaled_product` for the same exceptional lanes.

Across two 9-pair runs, the paired geometric-mean vector/scalar ratio was
**2.835 [2.763, 2.908]** (95% t interval on log ratios); median paired ratio
was 2.872. Candidate and scalar outputs agreed within `1e-14` for the
deterministic single-call numerical check. The helper's current checked
products and exponent validation cost more than scalar recovery for this
three-of-four-lane pattern. Since the screen shows a resolved helper-level
regression, no whole-transform pairs were run and no code was promoted.

The first fixed-input probe reported 11.96× slowdown, but was discarded as a
decision measurement because invariant scalar products could be hoisted. The
preserved probe varies one basis value, one coefficient and the exponent each
iteration, performs two warmup batches, and runs pinned to CPU 0. The complete
paired values are in `data/screen.json`.

## Reproduction

Decompress and apply `patches/candidate-and-microbench.patch.gz` to the base
source above, then run:

```sh
gzip -dc patches/candidate-and-microbench.patch.gz | git apply
taskset -c "${CPU_ID}" env RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  CARGO_TARGET_DIR=target \
  cargo test --release --features parallel --lib \
  benchmark_b3_partial_scaled_exception_vector_vs_scalar_fallback \
  -- --ignored --nocapture
```

The run used Rust 1.99.1 / LLVM 23.1.1, `x86-64-v3`, one worker pinned to a
single core on an AMD Ryzen 9 5950X, and a four-lane f64 SIMD specialization.
The exact core identifier is omitted from this committed summary. The test
binary SHA-256 is recorded in `data/screen.json`. The patch includes both the
isolated candidate and ignored test-only benchmark; neither is in production.
The compressed and decompressed patch hashes are recorded in `SHA256SUMS`.

## Next

Do not route partial exception vectors through this helper. Return to the
remaining ranked B3 stage from the active plan: recurrence/normalization, using
the sampled profile and active AVX2 disassembly to form a distinct bounded
hypothesis before editing the kernel.
