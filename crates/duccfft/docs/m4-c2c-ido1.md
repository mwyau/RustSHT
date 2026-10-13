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
used runtime AVX2 dispatch on an x86-64 CPU, pinned to logical one CPU core.

length 4,096 for each direction. Every iteration restored the same finite
input before calling the public transform. The reported instruction totals
include plan and workspace construction, input setup, the 103 input resets,
and the transform calls. The profile command and invocation are:

```sh
  target/release/deps/rustfft-50234de64dcf3139 \
  bench-profile-complex 4096 forward 100
```

compressed profiles are

## Attribution

Length 4,096 factorizes into four radix-eight passes. The pass dimensions are
`(l1, ido) = (1, 512), (8, 64), (64, 8), (512, 1)`. The AVX2 out-of-place
kernel requires `ido` to contain its two-complex f64 vector width. Thus the
first three passes can use AVX2, while the final `(512, 1)` pass uses the
three out-of-place passes per transform reached the AVX2 pass kernel.

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
they do not predict its latency effect. The profile includes two large `memcpy` groups per transform: restoring the
benchmark input and copying the final workspace result back to the public
buffer. Their inclusive instruction counts are each about 6.75 million over
103 calls. A separate earlier wall-time profile measured the 4,096-point
copyback at about 0.91 µs, or 2.3% of the whole C2C forward time. Instruction
share and wall-time share are different measurements.

## Prototype constraints

The candidate may process two adjacent `outer` butterflies in each AVX2
vector. For `ido=1`, the input contains eight contiguous complex branches per
butterfly. Pairing the corresponding two-complex loads with
`_mm256_permute2f128_pd` immediates `0x20` and `0x31` forms one vector per
branch, with one butterfly in each 128-bit lane. The vector radix-eight
equations then compute both butterflies in parallel. Each branch's two
outputs are contiguous in the pass output. The kernel must be limited to radix eight, `ido=1`, even `l1 >= 2`, and the
f64 AVX2 dispatch. It must keep the existing inverse eligibility gate. An
inverse that needs prescaling and residual recovery must continue through the
scalar path. The existing `ido >= 2` vector contract and all f32 dispatch
conditions remain unchanged. The candidate requires direct pass-equation and
public-transform checks, plus a paired timing comparison before it can be
retained. R32's read-only review confirmed the lane order, bounds, output
layout, and profile attribution. It approved the bounded experiment and
required an explicit unsafe contract for the low-`ido` entry, full-output
forward and inverse checks, and verification that the residual fallback is
unchanged. The reviewer did not run builds or tests.
