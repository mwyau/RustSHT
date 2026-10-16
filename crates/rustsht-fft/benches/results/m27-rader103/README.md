# M27: Rader-103 C2C pilot

The bounded Rader-103 route was rejected. On the N=1442 C2C workload, it was
37.7% slower for forward transforms and 34.9% slower for inverse transforms
than the M26 AVX2 direct-pass baseline, using the median of three paired
processes. The route also increased workspace storage from 26,368 to 30,176
bytes. The production change was removed after the pilot.

| Operation | Rader candidate | M26 baseline | Candidate / baseline |
|---|---:|---:|---:|
| C2C forward N=1442, 721 rows | 41.562 µs | 30.190 µs | 1.377× |
| C2C inverse N=1442, 721 rows | 43.552 µs | 32.294 µs | 1.349× |

Values are median per-transform times from three alternating process pairs.
Each process warmed and timed seven samples per operation. The release
candidate used Rust 1.99.0 stable, portable x86_64 settings and runtime AVX2
dispatch. The host was an AMD Ryzen 9 5950X; processes were pinned to logical
CPU 30. The M26 reference executable is RustSHT-FFT commit
`5d9264871972756a6cd7323e99e5a938b7e3040a`, binary SHA-256
`76f2404f5c454e32985520030bd12626c8f1979f96f12a2633515b0a3e9ca71b`. The
candidate binary SHA-256 is
`e19aa9384d89a83cf6639307f71e3127b959d36d604b6d8a5e88d5e44af88df0`; its
source patch SHA-256 is
`a224cad7a26f0f2dd13fec6f66e562f044cf30d61e619e9cd78577eca835dcf0`.

Focused direct-DFT tests passed for the radix-103 Rader convolution in both
directions. The end-to-end N=1442 independent-DFT test and route/range-guard
test also passed. This was a focused pilot rather than a full target-matrix
acceptance run. Raw process records and the source patch are retained in this
directory.

The N=1442 Rader candidate output was compared with `ducc0.fft.c2c` from DUCC
0.41.0 on `sin(0.173x) + i cos(0.113(x+1))`, for `x=1..1442`. The relative-L2
errors were `4.580e-16` forward and `4.361e-16` for normalized inverse; maximum
absolute errors were `2.305e-13` and `1.111e-16`, respectively. The Rust
driver, comparison script and saved output arrays are in
[`../m27-complex-kernel/`](../m27-complex-kernel/).
