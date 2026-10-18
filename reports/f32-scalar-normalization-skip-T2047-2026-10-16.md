# T2047 scalar f32 normalization-skip screen

## Result

The existing scalar f32 normal-range normalization skip was screened at T2047
on GL `2048×4096`, full `mmax=2047`, and `phi0=0.37`. One baseline/candidate
process pair per spin completed. The candidate changed only the exact-match
gate that is enabled at T1023 and T1279. The one-pair changes were mixed: the
scalar observations showed analysis 2.2–3.1% faster while synthesis stayed
within 0.7% of baseline. These are single-pair observations, not established
gains; the candidate scalar process briefly overlapped an unrelated RustFFT
benchmark whose CPU placement was not captured. The selector changes only the
scalar f32 recurrence. Spin-1 STANDARD uses a separate, unchanged recurrence,
so every spin-1 row below is control variation between processes, not an effect
of the selector.

| Operation | Threads | Baseline | Candidate | Observed difference |
| --- | ---: | ---: | ---: | ---: |
| Scalar synthesis | 1 | 30,182.640 ms | 29,968.931 ms | −0.71% |
| Scalar synthesis | 8 | 5,381.048 ms | 5,392.851 ms | +0.22% |
| Scalar analysis | 1 | 14,568.565 ms | 14,115.711 ms | −3.11% |
| Scalar analysis | 8 | 2,666.338 ms | 2,606.896 ms | −2.23% |
| Spin-1 synthesis | 1 | 62,398.649 ms | 61,044.844 ms | −2.17% |
| Spin-1 synthesis | 8 | 8,731.077 ms | 9,245.186 ms | +5.89% |
| Spin-1 analysis | 1 | 51,695.003 ms | 49,120.769 ms | −4.98% |
| Spin-1 analysis | 8 | 10,277.382 ms | 9,607.876 ms | −6.51% |

Times are arithmetic medians of two warm samples in each process. The scalar
candidate process overlapped briefly with a separate RustFFT benchmark whose
CPU placement was not captured, so the scalar differences are diagnostic. During the spin-1 candidate process, another short RustFFT run was observed on
one CPU core while RustSHT was 8 pinned CPU cores. The spin-1 q8 synthesis samples
were `9,920.050` and `8,570.322` ms; that spread does not support a stable
estimate. All spin-1 measurements are unaffected-path control variation and
do not indicate a selector benefit or regression. None of these results are
repeated performance acceptance evidence. The benchmark reported plan construction between 0.217 and 0.574 ms and
workspace allocation between 24.716 and 27.133 ms. Peak process RSS ranged
from 199,460 to 199,604 KiB. A first-use call and all raw warm samples are
preserved in the [data directory](data/f32-scalar-normalization-skip-T2047-2026-10-16/). The production selector uses the skip at T1023 and T1279; T2047 uses the
established normalizer. The experiment stopped after one process pair per spin because the scalar
observations did not establish a repeatable benefit and were mixed across
operations. The spin-1 results are controls, not candidate effects. The pair
took 18.4 minutes; two more pairs at the observed rate would have taken about
37 additional minutes. The incomplete next process is retained in the raw
data. Candidate outputs were not captured or compared with DUCC, so the screen
gives no T2047 f32 numerical acceptance evidence and sets no T2047 f32
accuracy threshold.

## Measurement record

The baseline source revision was
`2f36a9d540a0ede6ea93d0e7a10d3c71f5e7a968`. Both binaries used
`cargo build --release --example m1_f32_atmospheric_bench --features parallel`,
the root release profile, runtime Pulp dispatch, and no global
`target-cpu=native` flag. The baseline binary SHA-256 was
`e93fc28c08e84ee72cc7c7ff03128bff514de47d70a78603c151a04cd1888796`; the
candidate binary SHA-256 was
`70fa90a93d2903707037f7521acdcfef7de5cd19757cfc0d76093411ad98ef51`. The
host was an x86-64 CPU; Rust was `1.99.0 (b940084d7 2026-09-28)` and
LLVM was `23.1.1`. Load-average telemetry was retained only in local diagnostics. Each process used one reused plan and workspace, one first call, two timed
warm samples per operation at q1 and q8, and CPU affinity sized for the worker count. The paired
runner was configured for three process pairs but stopped after one completed
pair per spin. It was interrupted 63.3 seconds into the next candidate scalar
process; its partial timing record remains in the data directory. Python
comparisons and Rust test suites were not run for the temporary selector. The
T2047 candidate was removed from production code before final validation.
