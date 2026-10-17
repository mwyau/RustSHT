# M2 general scalar visitor screen

## Decision

Reject the guarded ordinary-product visitor candidate. The paired q1 screen
measured a 6.98% scalar synthesis regression at T511 and a 3.45% regression at
T1023. Both paired 95% bootstrap intervals are entirely above the 2% guard. The T511 spin-1 synthesis control was neutral. The candidate did not proceed
to the full operation matrix or matched DUCC/SHTns timing. The candidate is preserved as a patch and isolated source snapshot. No
production source was changed or committed.

## Candidate

The clean base was production commit a50f70051049e37fa71898d5ec0aecc56ff388b3. The source inspection in [mechanism.md](mechanism.md) identified repeated
coefficient and product classification inside the normal scalar visitor. The candidate used the visitor's normal-range status and a conservative
coefficient/product bound to classify ordinary products once per
degree-pair/block, while retaining the existing scaled helpers for unsafe
products and scaled visits. It did not change coefficient preparation,
recurrence updates, degree accumulation order, or reuse coefficients across
blocks. The complete source diff is [rejected-candidate.patch](rejected-candidate.patch);
its SHA-256 is in [evidence-sha256.txt](evidence-sha256.txt). Source file
hashes are in [candidate-source-sha256.txt](candidate-source-sha256.txt). The candidate and baseline ELF hashes are in [binary-sha256.txt](binary-sha256.txt). The release build logs are retained alongside the raw benchmark output.

## Numerical checks

Formatting passed. Focused debug checks passed for the coefficient exponent
edges and intermediate product fallbacks; scalar synthesis against pinned
DUCC 0.41.0 at low degree and T255; T511 scalar GL analytic/DUCC sectoral
samples; T255 spin-1 GL DUCC samples; CC and F1 scalar DUCC and adjoint
identities; and f32 scalar S/A against DUCC 0.41.0 on GL, CC and F1. Exact
commands and output are in [focused-tests.log](focused-tests.log). These checks validate the isolated candidate on the listed cases. The full
workspace suites were not run because the initial performance screen rejected
the candidate.

## Paired whole-transform screen

Rust 1.99.0 release binaries were built from the same base and Cargo lockfile. RUSTFLAGS and CARGO_ENCODED_RUSTFLAGS were empty. The host was an x86-64 CPU. Each fresh process used one CPU core, q1, GL STANDARD f64, full mmax,
phi0=0.37, ntheta=lmax+1, nphi=2(lmax+1), requested-only synthesis, one
warmup and five measured synthesis samples. Six alternating baseline/candidate
process pairs were run per cell. Peak RSS and wall-time output are in the
paired .time and .stdout files; scripts are run-one.sh and run-screen.sh.

| T | Spin | Baseline median (ms) | Candidate median (ms) | Candidate / baseline | Paired bootstrap 95% interval | Result |
|---:|---:|---:|---:|---:|---:|---|
| 511 | 0 | 40.346 | 43.153 | 1.06980 | [1.06676, 1.07341] | Reject |
| 511 | 1 | 42.709 | 42.831 | 1.00604 | [0.99565, 1.01099] | Control passes |
| 1023 | 0 | 295.113 | 305.729 | 1.03446 | [1.02346, 1.03882] | Reject |

All six T511 scalar pair ratios exceed 1.02. Five of six T1023 scalar pairs
exceed 1.02, and its bootstrap interval is wholly above 1.02. The median
paired peak-RSS change was negligible at both scalar targets. Raw pair rows
and bootstrap summary are in [screen-pairs.csv](screen-pairs.csv) and
[screen-summary.csv](screen-summary.csv). The preflight log records 36 benchmark process snapshots and one partial
snapshot from a failed launcher probe. No run was refused; no compiler or
benchmark workload was observed during accepted process preflights. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The initial
taskset affinity probe had invalid arguments and stopped before launching a
benchmark; the probe was corrected before the paired screen. The raw process
timings show consistent scalar slowdown in both alternating orders. Treat
this as a rejection screen, not a general host-stability claim.

## Status

The M1 route profile remains the reason to target visitor work: its q1 visitor
timer sums were 26.408 ms at T511 and 191.910 ms at T1023. This candidate did
not improve those whole-transform targets. P01 remains shelved and
unaccepted; P02 remains rejected. Further visitor work needs a different
mechanism supported by the profile and must pass its own regression screen.
