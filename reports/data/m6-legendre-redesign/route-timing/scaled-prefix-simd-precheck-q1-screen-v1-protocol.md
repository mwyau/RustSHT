# Scaled-prefix SIMD precheck q1 screen protocol v1

## Objective and fixed order

This screen evaluates the isolated SIMD precheck candidate against the clean
production baseline. The candidate source passed focused kernel tests and a
T2047 bitwise route check, but has no complete-transform timing result.

First run one T2047 baseline-versus-baseline diagnostic using the identical
clean baseline binary in two fresh processes. Each process runs one warm-up
and five timed transformations for RustSHT and DUCC. Continue only if every
backend series and both host-pressure checks pass. Do not replace or retry a
process. If the diagnostic fails, stop the q1 screen.

If the diagnostic passes, run one fresh baseline/candidate process pair at
T1023, baseline first, then one pair at T2047 in the same fixed order. Launch
each side as an independent fresh process and evaluate its full validity
before launching the next side. This enforces the stop-at-first-invalid-
process rule. Each process uses one warm-up and five timed complete plan-scope
transformations for each backend. Do not remove samples, retry, or change the
thresholds. These pairs are screening evidence only; a valid screen can
authorize the existing six-pair q1 matrix but cannot accept the candidate.

The baseline-versus-baseline diagnostic used the paired-process runner below.
For the four screen processes, invoke the unchanged harness directly with the
same environment and one-core placement. A reproducible command template is:

```sh
taskset -c "$BENCH_CPU" /usr/bin/time -v -o "$LOCAL_TIME" env \
  PYTHONPATH="$VARIANT_PYTHONPATH" RAYON_NUM_THREADS=1 \
  RUSTSHT_PERF_RAW=1 OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 \
  "$PYTHON" examples/python_perf_baseline.py \
  "$L" 0 1 GL 1 plan synthesis both STANDARD "$((2*(L+1)))" "$((L+1))"
```

The harness and process-pair runner hashes below remain frozen. The runner
hash documents the baseline stability diagnostic; the four candidate-screen
cells use the command template above so each side can be validated before
the next process starts.

## Frozen source and binaries

- Clean production baseline: source revision
  `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`; rebuilt with the screen Rust
  toolchain. Release extension SHA-256:
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- SIMD precheck candidate: the accumulated tile-128, scaled-prefix
  accumulator, no-op normalization and SIMD precheck source recorded by the
  [candidate report](scaled-prefix-simd-precheck-candidate-v1.md), which links
  the [no-op normalization source delta](scaled-mantissa-noop-screen-v1.patch)
  and [precheck source delta](scaled-prefix-simd-precheck-candidate-v1.patch).
  `scalar.rs` SHA-256:
  `cfe5f7ee31e75f70445fdca118f154cd39961ed5b0751f593e94c26794283a0a`;
  `synthesis_fast.rs` SHA-256:
  `5bd3af68311792091ff6dcc9628f9fed91ddcb240f54874b05c886fcc73430c3`.
  Release extension SHA-256:
  `08d049d340b43a9d80adbf164d6964aea4ecd1a196160efee643150729161d67`.
- Build both Rust extensions with `cargo build --release --manifest-path
  python/Cargo.toml`, Rust/Cargo 1.99.0, and default release settings. No
  global ISA flag or `target-cpu=native` is used.
- DUCC 0.42.0 source revision:
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`. The Python extension is built
  from that source revision with the default CMake project configuration;
  its SHA-256 is
  `07fe9333f8edc80dab0285d51debb5e45979d3754b4ac9033712712668e28468`.
- Benchmark harness: `examples/python_perf_baseline.py`, SHA-256
  `e7ffbe889922cc4da1b1ece63bc24bf264603c7373adb195abde5d705bb51c07`.
  Baseline-control fresh-process runner:
  `crates/rustsht-fft/benches/paired_processes.py`,
  SHA-256
  `430fe2ebb6cac4a6be30decf68687fb1fddf2f5d88dcaba00d79d266da940d7c`.
- Configuration: Python 3.11.15, NumPy 2.4.6, CMake 4.2.3, C++ compiler
  15.2.0. Host: AMD Ryzen 9 5950X, x86-64, AVX2/FMA, 16 physical cores, 32
  logical CPUs, 96 GiB RAM, Ubuntu 26.04.1 LTS.

## Workload and validity

Each cell uses GL f64 scalar `STANDARD` synthesis, full `mmax`,
`phi0=0.37`, one worker, and plan scope. T1023 is `1024x2048`; T2047 is
`2048x4096`. Each process runs the same numerical input with the same
configuration and compares its RustSHT output with DUCC.

- Reject preflight or postflight CPU PSI `some avg10` above 2.00%. Readings
  from 0.50% through 2.00% are warnings. Keep exact telemetry local.
- Require population CV at most 5% and first-three/final-two median drift
  at most 5% for every RustSHT and DUCC five-call series.
- Verify process and worker affinity, thread placement, CPU frequency,
  involuntary context switches, cgroup throttling, and concurrent
  CPU-intensive work. Clear interference makes a process inconclusive even
  below the PSI rejection threshold. Keep raw process and host-state captures
  local and untracked.
- Require relative-L2 error at most `2e-11` and maximum absolute error at
  most `2e-12` for both variants against DUCC.
- The one-pair candidate screen can proceed to the established six-pair q1
  matrix only if all series pass and candidate RustSHT median latency is at
  most `0.98` times baseline at one high bandlimit, with the other high
  bandlimit ratio at most `1.02`. Any invalid cell stops the screen. A valid
  screen does not clear the T511 control, the q8 gate, or the complete
  scalar/spin-1 regression matrix.

Commit only sanitized timing samples, gate outcomes, aggregate peak RSS,
numerical errors and conclusions. Do not commit paired runner JSONL, GNU time
output, preflight/postflight data, process listings, affinity IDs, frequency
snapshots, or raw command output.
