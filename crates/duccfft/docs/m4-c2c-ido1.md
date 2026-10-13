# C2C low-`ido` pass profile

This profile examines the f64 AVX2 C2C implementation before a targeted
radix-eight low-`ido` kernel experiment. It does not establish a wall-time
gain.

## Method

The FFT source was commit `402056f`. The harness addition is in
`benches/rustfft.rs`, SHA-256
`ca57805ec9fdff65f52a058f326dc25d848a36f6cc2086cf7cae2670e75fbb73`. Rust
1.99.0 (`b940084d7`, LLVM 23.1.1) built the optimized Cargo `bench` profile
with no custom `RUSTFLAGS`, LTO, or `target-cpu` setting. The profiled binary
SHA-256 is
`1ed1851043f07280aeeab9ec14046140889eb3a1d25e9157bd14b0853c7c1157`. It
used runtime AVX2 dispatch on an AMD Ryzen 9 5950X, pinned to logical CPU 30.

Callgrind 3.26.0 ran three warm-up and 100 measured public transforms at
length 4,096 for each direction. Every iteration restored the same finite
input before calling the public transform. The reported instruction totals
include plan and workspace construction, input setup, the 103 input resets,
and the transform calls. The profile command and invocation are:

```sh
taskset -c 30 valgrind --tool=callgrind \
  --callgrind-out-file=/tmp/c2c-forward.callgrind \
  target/release/deps/rustfft-50234de64dcf3139 \
  bench-profile-complex 4096 forward 100
```

The inverse command substitutes `c2c-inverse.callgrind` and `inverse`. The
compressed profiles are
[`forward`](../benches/results/m4-c2c-ido1-forward-callgrind.out.gz) and
[`inverse`](../benches/results/m4-c2c-ido1-inverse-callgrind.out.gz).

## Attribution

Length 4,096 factorizes into four radix-eight passes. The pass dimensions are
`(l1, ido) = (1, 512), (8, 64), (64, 8), (512, 1)`. The AVX2 out-of-place
kernel requires `ido` to contain its two-complex f64 vector width. Thus the
first three passes can use AVX2, while the final `(512, 1)` pass uses the
scalar radix-eight loop. Callgrind's call counts confirm that two of the
three out-of-place passes per transform reached the AVX2 pass kernel.

| Callgrind measurement | Forward | Inverse |
|---|---:|---:|
| Program instruction references | 47,690,848 | 61,642,908 |
| Public `ComplexFftPlan` operation, inclusive | 38,622,521 | 52,574,695 |
| `execute_pass`, inclusive; 309 calls | 27,703,395 | 28,494,126 |
| AVX2 out-of-place pass kernel, inclusive; 206 calls | 8,953,584 | 9,111,792 |
| AVX2 first-pass kernel; 103 calls | 4,120,721 | 4,199,722 |
| `execute_pass` self cost, overlapping child rows excluded | 18,749,193 | 19,381,716 |

Each transform executes three out-of-place pass calls. Two use AVX2; the
remaining call is the low-`ido` scalar pass. The `execute_pass` self cost also
includes dispatch and loop work from the vectorized calls, so it does not
isolate the final scalar pass. These counts identify a suitable experiment;
they do not predict its latency effect.

The profile includes two large `memcpy` groups per transform: restoring the
benchmark input and copying the final workspace result back to the public
buffer. Their inclusive instruction counts are each about 6.75 million over
103 calls. A separate earlier wall-time profile measured the 4,096-point
copyback at about 0.91 µs, or 2.3% of the whole C2C forward time. Instruction
share and wall-time share are different measurements.

## Prototype design

The candidate may process two adjacent `outer` butterflies in each AVX2
vector. For `ido=1`, the input contains eight contiguous complex branches per
butterfly. Pairing the corresponding two-complex loads with
`_mm256_permute2f128_pd` immediates `0x20` and `0x31` forms one vector per
branch, with one butterfly in each 128-bit lane. The vector radix-eight
equations then compute both butterflies in parallel. Each branch's two
outputs are contiguous in the pass output.

The implemented kernel is limited to radix eight, `ido=1`, even `l1 >= 2`,
and f64 AVX2 dispatch. It keeps the existing inverse eligibility gate. An
inverse that needs prescaling and residual recovery continues through the
scalar path. The existing `ido >= 2` vector contract and all f32 dispatch
conditions remain unchanged. The initial R32 design review confirmed the lane
order, bounds, output layout, and profile attribution. It approved the bounded
experiment and required an explicit unsafe contract for the low-`ido` entry,
full-output forward and inverse checks, and verification of the residual
fallback. The reviewer did not run builds or tests.

## Candidate result

Commit `509cab9` adds the f64 AVX2 radix-eight `ido=1` pass. The dispatcher
constructs the specialized pass descriptor only for eligible passes; regular
AVX2 passes retain the `ido` vector-width check. Inverse execution still
requires `inverse_normalization_is_safe`, so high-range inputs use the scalar
prescale and residual-correction path.

The specialized kernel pairs two adjacent outer butterflies in AVX2 lanes.
Its disassembly uses `vperm2f128`, packed additions, subtractions and
multiplications, and unaligned vector loads and stores. The 608-byte function
has no function calls or stack spills. The disassembly is archived in
[`m4-c2c-ido1-candidate-disassembly.txt`](../benches/results/m4-c2c-ido1-candidate-disassembly.txt).

### Paired wall-time samples

The baseline source was commit `15e75f1`; the candidate source was commit
`509cab9`. Both used the same `benches/rustfft.rs` source (SHA-256
`ca57805ec9fdff65f52a058f326dc25d848a36f6cc2086cf7cae2670e75fbb73`) and
optimized Cargo bench profile. Rust was 1.99.0 (`b940084d7`, LLVM 23.1.1),
with `-C opt-level=3 -C embed-bitcode=no -C strip=debuginfo`; there were no
custom `RUSTFLAGS`, LTO, or `target-cpu` settings. Runtime dispatch selected
AVX2 on an AMD Ryzen 9 5950X; each process was pinned to logical CPU 30. The
host also ran other workloads, so the timing evidence remains provisional.

The harness warmed each plan, restored identical finite input before every
timed public transform, and collected seven interleaved process pairs for
each length and direction. Each process timed 5,000 transforms per sample.
Therefore, the public-operation time includes the input reset and transform,
while plan and workspace setup remain outside the wall-time interval. The
factorization column shows whether the final low-`ido` radix-eight kernel can
run; the final factor must be eight, with `ido=1` and even `l1 >= 2`.

| Length | Pass factorization | Direction | Baseline median (ns) | Candidate median (ns) | Ratio | Paired latency reduction | Candidate wins |
| ---: | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 64 | `[8,8]` | Forward | 238.685 | 122.445 | 1.949× | 51.42% | 7/7 |
| 64 | `[8,8]` | Inverse | 303.130 | 193.506 | 1.567× | 33.77% | 7/7 |
| 128 | `[2,8,8]` | Forward | 426.289 | 224.461 | 1.899× | 46.59% | 7/7 |
| 128 | `[2,8,8]` | Inverse | 597.882 | 394.814 | 1.514× | 33.95% | 7/7 |
| 256 | `[8,8,4]` | Forward | 836.746 | 819.732 | 1.021× | 0.32% | 4/7 |
| 256 | `[8,8,4]` | Inverse | 1,137.168 | 1,123.823 | 1.012× | 1.09% | 5/7 |
| 512 | `[8,8,8]` | Forward | 1,712.639 | 885.433 | 1.934× | 48.98% | 7/7 |
| 512 | `[8,8,8]` | Inverse | 2,460.427 | 1,645.587 | 1.495× | 33.73% | 7/7 |
| 1,024 | `[2,8,8,8]` | Forward | 3,908.541 | 2,250.462 | 1.737× | 42.12% | 7/7 |
| 1,024 | `[2,8,8,8]` | Inverse | 5,419.239 | 3,721.103 | 1.456× | 31.23% | 7/7 |
| 2,048 | `[8,8,8,4]` | Forward | 8,393.888 | 8,412.987 | 0.998× | 0.72% | 4/7 |
| 2,048 | `[8,8,8,4]` | Inverse | 11,388.895 | 11,337.344 | 1.005× | −0.52% | 2/7 |
| 4,096 | `[8,8,8,8]` | Forward | 18,575.360 | 11,145.883 | 1.667× | 39.80% | 7/7 |
| 4,096 | `[8,8,8,8]` | Inverse | 24,411.266 | 17,209.056 | 1.419× | 29.77% | 7/7 |

The ratio is the ratio of the two median times. The latency reduction is the
median of the seven paired percentage reductions, so it need not equal the
percentage calculated from the two medians. All 196 raw rows and displayed
checksums are recorded in
[`m4-c2c-ido1-process-samples.csv`](../benches/results/m4-c2c-ido1-process-samples.csv);
each baseline/candidate pair has a matching checksum. The kernel is eligible
for lengths 64, 128, 512, 1,024, and 4,096; it is not selected at 256 or 2,048.
Both noneligible controls show mixed, sub-1.1% paired outcomes. The eligible
lengths favor the candidate in all 70 paired forward and inverse samples.

### Instruction profile

Callgrind 3.26.0 profiled the optimized portable-dispatch binaries on CPU 30.
Each process included plan and workspace construction, input creation, three
warm-up transforms and 100 measured public transforms. Input was reset before
each measured transform. The candidate binaries have SHA-256
`1ed1851043f07280aeeab9ec14046140889eb3a1d25e9157bd14b0853c7c1157` for the
baseline and
`6cf78b4afd6f90d556c13de6121673d2433c14a608df5d5f7597d1d33f3b20dc` for the
candidate. The candidate profiles are archived as
[`forward`](../benches/results/m4-c2c-ido1-candidate-forward-callgrind.out.gz)
and [`inverse`](../benches/results/m4-c2c-ido1-candidate-inverse-callgrind.out.gz).

| Callgrind Ir | Baseline forward | Candidate forward | Baseline inverse | Candidate inverse |
| --- | ---: | ---: | ---: | ---: |
| Whole process | 47,690,848 | 31,284,928 | 61,642,908 | 44,867,636 |
| `execute_pass`, inclusive | 27,703,395 | 11,300,130 | 28,494,126 | 11,721,503 |
| Low-`ido` radix-eight kernel, inclusive | — | 2,322,547 | — | 2,586,021 |
| AVX2 out-of-place pass kernel, inclusive | 8,953,584 | 8,953,584 | 9,111,792 | 9,111,792 |
| `execute_pass` self cost | 18,749,193 | 23,793 | 19,381,716 | 23,484 |

The low-`ido` kernel ran 103 times in each profile; the ordinary AVX2 pass ran
206 times. The self-cost reduction reflects the scalar terminal-pass work
moving out of `execute_pass`; inclusive rows overlap and instruction counts do
not directly represent elapsed time. Whole-process instruction references
fell by 34.40% forward and 27.21% inverse.

### Numerical checks and review

The direct pass test checks every output for `l1=2` and `l1=8`, in both
directions. A public 4,096-point test compares forward and independent inverse
outputs against RustFFT. A maximum-finite DC spectrum test confirms that a
4,096-point inverse remains finite and correctly scaled through the scalar
residual path. The existing high-range and subnormal tests remain unchanged.
`cargo fmt --all -- --check`, `cargo test --all-features`, and
`cargo clippy --all-targets --all-features -- -D warnings` pass for the
standalone crate.

R32's read-only review accepted the kernel and verified the expanded sweep.
It confirmed the lane mapping, bounds, disjoint buffers, one-time inverse
scaling, runtime AVX2 dispatch, f32 fallback, and high-range scalar fallback.
It reproduced all 196 timing records, table statistics, factorization labels,
checksums, Callgrind totals, and packed AVX2 codegen; it found no actionable
implementation issue. All 70 paired samples at the five eligible lengths
favor the candidate. At 4,096, the ratios of medians are 1.667× forward and
1.419× inverse, while the medians of paired latency reductions are 39.80% and
29.77%. At the ineligible 2,048 control, the ratios are 0.998× forward and
1.005× inverse, while the paired reductions are 0.72% and −0.52%; the control
results are mixed and small. The targeted kernel reduces scalar terminal-pass
cost at the eligible lengths but does not resolve the remaining complex/real
gaps.
