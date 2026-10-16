# M27: N=1442 radix-103 kernel experiments

The compact paired-outer AVX2 layout reduced N=1442 C2C latency, but each
version caused repeatable slowdowns in other FFT-target cells. The candidate
was removed. M26 is the current production path.

## Whole-matrix captures

Six alternating process pairs compared each candidate with the committed M26
source, RustSHT-FFT commit
`5d9264871972756a6cd7323e99e5a938b7e3040a`, binary SHA-256
`76f2404f5c454e32985520030bd12626c8f1979f96f12a2633515b0a3e9ca71b`.
Captures used Rust 1.99.0 stable, the optimized bench profile, portable x86_64
settings, runtime AVX2 dispatch, an AMD Ryzen 9 5950X pinned to logical CPU 30,
one transform thread and seven warmed samples per operation. Each capture
covered all 34 C2C and real SHT-target cells. Candidate and reference processes
alternated order; all exited successfully.

| Candidate layout | N=1442 forward speedup | N=1442 inverse speedup | Worst non-target cell |
|---|---:|---:|---|
| Paired outer, inline | 1.211× [1.201, 1.215] | 1.198× [1.189, 1.204] | C2C inverse N=146, 73 rows: 0.925× |
| Paired outer, isolated helper | 1.214× [1.208, 1.217] | 1.211× [1.203, 1.235] | C2C inverse N=146, 73 rows: 0.924× |
| Compact helper with scalar odd-group tail | 1.222× [1.220, 1.224] | 1.201× [1.196, 1.209] | C2R N=723, 361 rows: 0.904× |

Speedup is M26 time divided by candidate time; values below 1 indicate a
candidate regression. The first two versions repeatedly slowed N=146 inverse
by about 8% and N=722 inverse by about 6%. The compact version removed those
regressions but slowed N=723 C2R by about 11%. Moving the helper module and
adding cold-code annotations changed which unrelated cells regressed, but
did not remove the regressions. The Linux text-subsection attempt left the
helper symbols at the same linked addresses as the prior build and was not
benchmarked.

Raw process records, summaries, configurations and source patches for each
version are stored in this directory. The M26 baseline binary remains in
`/dev/shm/rustsht-fft-m26-isolated-target` for reproducing the captures while
that temporary filesystem persists.

## Numerical checks

The radix-103 AVX2 direct-DFT test passed in both directions for group counts
1, 14 and 15, including output guard regions. The end-to-end N=1442 independent
DFT and route/range-guard tests passed during development. The N=1442 Rader
alternative was measured separately and rejected; its results are in
[`../m27-rader103/`](../m27-rader103/).

The compact paired-outer output and the Rader-103 output were also compared
with `ducc0.fft.c2c` from DUCC 0.41.0. The deterministic N=1442 input was
`sin(0.173x) + i cos(0.113(x+1))`, for `x=1..1442`; the inverse comparison
used DUCC `inorm=2` to match RustSHT-FFT's normalized inverse. Both f64
candidates were built in release mode with Rust 1.99.0. The paired-outer
source patch SHA-256 is
`626756dea4582fcc4645aa26b7c100bd2fb2721f9b96a939807a19cc1a5cd917`; the
Rader source patch SHA-256 is
`a224cad7a26f0f2dd13fec6f66e562f044cf30d61e619e9cd78577eca835dcf0`. The
comparison source and output arrays are stored alongside the benchmark
records.

| Candidate | Direction | Relative L2 error | Maximum absolute error |
|---|---|---:|---:|
| Paired outer | Forward | 7.855e-16 | 9.097e-13 |
| Paired outer | Normalized inverse | 4.448e-16 | 2.289e-16 |
| Rader-103 | Forward | 4.580e-16 | 2.305e-13 |
| Rader-103 | Normalized inverse | 4.361e-16 | 1.111e-16 |

The comparison script is [`m27-ducc-crosscheck.py`](m27-ducc-crosscheck.py),
and the Rust candidate driver is [`m27-ducc-crosscheck.rs`](m27-ducc-crosscheck.rs).
