# T511 q8 baseline stability diagnostic

This short diagnostic repeated the same clean baseline binary in two fresh
processes to investigate the previously observed T511 scalar q8 timing drop.
It did not measure a candidate and does not accept or reject the tile-128
kernel.

## Configuration

- Clean baseline source: `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`.
- Baseline executable SHA-256: `58060a3a7b92f0330c173ba691626b5f501a72bda2d23dde8dd7ea022c1de4cc`.
- Harness: `examples/m1b_bench.rs`, SHA-256 `79d58ef6cd8c8b1e4c3a5c3cb54ff47b6cea0e8f9db78d1c5ea780b5c6153fc4`.
- Rust: `1.99.0`; release build on x86-64 AMD Ryzen 9 5950X, 16 physical and 32 logical cores, AVX2 and FMA, with 96 GiB installed RAM.
- Workload: f64 scalar GL `STANDARD` synthesis, T511, `mmax=511`, 512x1024, `phi0=0.37`, eight Rayon workers, `requested-only`.
- Each process used the same eight-CPU affinity width and one untimed transform followed by five timed transforms. The command template was `taskset -c <eight allowed CPUs> env RUSTSHT_BENCH_PHI0=0.37 <m1b_bench> 0 511 8 synthesis GL STANDARD 512 1024 511 requested-only`.

The predeclared process gate remains population CV at most 5% and first-three
versus final-two median drift at most 5%. Every timed sample is retained and
included; no result was removed or retried.

## Results

| Fresh process | Five synthesis samples (ms) | Median (ms) | CV | Drift | Timing gate |
|---|---|---:|---:|---:|---|
| Baseline 1 | 9.573, 9.166, 9.222, 9.104, 9.151 | 9.166 | 1.83% | 1.02% | Pass |
| Baseline 2 | 13.144, 13.825, 9.200, 9.456, 9.083 | 9.456 | 19.11% | 29.48% | Fail |

The second process had two slower early calls followed by three calls near
9.1–9.5 ms. One untimed transform did not produce a stable series in that
process. The first process was stable under the same protocol.

## Validity observations

CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The benchmark used eight workers with placement checked against the configured worker count. The local interference review found no competing build or benchmark workload. Context-switch and frequency telemetry remain local; their sampling resolution was too coarse to explain changes between individual transform calls. Peak process RSS
was 33.8 MiB and 33.2 MiB, respectively.

The diagnostic confirms that q8 timing can fail the existing stability gate
without elevated PSI or an observed competing benchmark. It does not isolate
whether the transient comes from worker startup, workspace initialization,
frequency changes, or another source. The M1 harness creates its Rayon pool
and workspace before timing but does not call `prepare_workspace`; its single
untimed transform is the only transform warm-up.

The q8 candidate matrix remains stopped. Do not change the stability limits
or classify the prior q8 samples as valid. If q8 work resumes, first define a
fixed warm-up protocol and verify it on q1 and q8 baseline controls before
rebuilding the frozen candidate. Raw process, frequency, affinity, context
switch, PSI, and console captures remain local and untracked.
