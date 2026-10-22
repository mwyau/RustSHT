# Original `ce710` analysis-kernel baseline

The data covers the actual fused scalar-analysis two-pair kernel at T720,
order `m=1`, with 64, 128 and 256 f64 latitude lanes. RustSHT production
source is `ce710d3622e3ca37783e0666b8bb800426fd72e0`; the DUCC kernel source
is revision `80388904b8cfc32400ba84af8a6a45aabfe60f86`.

Rust 1.99.1 / LLVM 23.1.1 used Pulp runtime V3 (AVX2/FMA), strict FP and
`-C target-cpu=x86-64-v3`. DUCC used GCC 15.2 and
`-O3 -DNDEBUG -ffast-math -ffp-contract=fast -march=x86-64-v3`; the driver
used `-fno-fast-math`. q1 was pinned to one physical CPU. Each lane size has ten
interleaved process pairs and three calibrated batches per backend and
process; batches were calibrated to 20–100 ms. The timed metric subtracts
an identically repeated reset-only loop; the JSON also retains reset-inclusive
wall timings. The batch result is consumed and checked outside each timed
batch. Sanitized summary distributions are in `summary.json`.

| Lanes | Rust kernel median (p10–p90), µs | DUCC kernel median (p10–p90), µs | Paired ratio | Excess, µs | Output relative L2 | Max absolute error |
|---:|---:|---:|---:|---:|---:|---:|
| 64 | 7.502 (7.464–7.662) | 4.087 (4.069–4.141) | 1.832× | 3.395 | `6.67e-14` | `5.01e-12` |
| 128 | 13.034 (12.973–13.451) | 8.029 (7.967–8.095) | 1.620× | 4.985 | `1.63e-13` | `1.90e-11` |
| 256 | 24.601 (24.533–24.743) | 15.751 (15.715–15.852) | 1.561× | 8.824 | `5.93e-14` | `2.56e-11` |

DUCC's driver confirmed that `map2alm_kernel` executed with four f64 lanes.
The lane-count changes compare more recurrence work, so the lower ratio at
larger sizes is not a speedup comparison between tile sizes. Raw per-process
samples and logs remain local. Fixture inputs and coefficient outputs are
retained here.

The original temporary Rust benchmark test was based on the same fused
kernel at `ce710` and used reset subtraction only as a separate diagnostic.
The exact current harness files and DUCC C++ source checksums are described
in the adjacent whole-transform README and the cumulative report.
