# M32: radix-19 at N=722

## Accepted change

The planner now selects a private AVX2 radix-19 kernel for the second N=722
pass, `(l1, ido) = (38, 1)`. The first pass, `(2, 19)`, remains on the scalar
general-prime kernel. The AVX2 kernel computes symmetric output-frequency pairs
and vectors two independent outer groups. It preserves a scalar tail, runtime
AVX2 dispatch, the scalar f32 path, inverse normalization, and the range-safe
inverse fallback. The equations are adapted from DUCC `fft1d_impl.h`
`cfftpg::exec_`, pinned at `c1ed01b8fda69d2a378190f0915f1a19762dddbf`; source
notices are retained in the kernel.

## Acceptance capture

`m32-cold-layout-full-six-pairs-config.json` and its JSONL/CSV contain the
final six alternating process pairs over the established 34-cell matrix, with
seven warmed samples, one transform thread, stable Rust 1.99.0, portable
x86_64 flags and runtime AVX2. The process was pinned to one CPU core on an x86-64 CPU. The candidate binary SHA-256 is
`a1036e527f31637d8f0fd9212d1401c2d568155cd1e8dc0faff785aa05333a8c`. The baseline binary is the retained M30 benchmark build; the M31 starting
checkout reported 11.11 us forward and 13.88 us inverse.

| N=722 operation | Candidate | Baseline | Speedup | Paired bootstrap 95% interval |
| --- | ---: | ---: | ---: | ---: |
| C2C forward | 7.258 us | 11.135 us | 1.533x | 1.524–1.611x |
| C2C inverse | 9.310 us | 13.801 us | 1.487x | 1.473–1.495x |

The full matrix shows no broad regression. Notable small changes include C2C
forward N=300 at 0.985x (95% interval 0.983–0.989), N=1024 at 0.984x
(0.962–0.992), C2C inverse N=512 at 0.973x (0.965–0.978), and real inverse
N=4097 at 0.990x (0.982–0.992). N=723 C2R is at parity, 0.998x
(0.983–1.008); N=257 C2R is 0.977x (0.972–1.000). Workspace bytes are
unchanged across all rows; the capture also records plan and workspace creation
times. The small unrelated shifts are disclosed rather than attributed to
instruction-count changes.

## Correctness and profiling

Independent direct DFT checks cover both transform directions, N=722's full
two-pass execution, vector-kernel layouts with even and odd `l1`, scalar odd
tails, guard regions, and extreme finite inputs. Planner-route tests confirm
runtime AVX2 selection for f64 and scalar routing for f32 and unsafe inverse
inputs. Native and portable DUCC comparisons are retained in this directory. Before the final documentation-only safety-contract correction, debug and
release all-feature suites each passed 111 tests, and Clippy with warnings
denied passed. After the correction, `cargo fmt --all -- --check` and the
focused N=722 dispatch/fallback test passed. The comment correction does not
change the measured binary.

attribution, not elapsed-cycle measurements. Hardware performance counters

## Rejected candidates

The AVX2 `(2, 19)` stage candidate reduced N=722 time but regressed N=257 C2R
retained. A shared stage-loop variant slowed N=722 inverse by about 4% and
N=723 C2R by about 9%. A non-cold reconstruction caused material N=257 and
N=723 C2R regressions. These candidates were not accepted. Their source has
been removed; their captures remain in this directory.
