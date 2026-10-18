# Range-precheck T2047 q1 baseline/candidate screen protocol v1

## Purpose and stop rule

Run one complete-transform T2047 q1 screen of the isolated tile-128
range-precheck source against clean production. The fixed order is clean
baseline first, candidate second. If the baseline process fails any validity
or correctness gate, stop before starting the candidate. If either process
fails, do not retry, replace, or omit it. This screen does not accept the
candidate. A valid candidate RustSHT median at most 0.98 times the baseline
median authorizes the frozen six-pair T2047 q1 matrix only.

The earlier T1023 v2 matrix stopped at baseline process six because the SHTns
series exceeded its fixed CV limit. Its samples remain diagnostic evidence;
they are not reclassified or included here.

## Frozen sources and harness

- Clean production source revision:
  `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`; extension SHA-256
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- Isolated candidate `src/legendre/synthesis_fast.rs` SHA-256:
  `07c41fe4ad285e59d7ed180931d145f72841d7adc9fe889a20a755837bf48b06`.
  Candidate `src/legendre/scalar.rs` SHA-256:
  `961da754257fcc4126e553700f5c32a7ac66b8889a66f986ec60b1e842fff559`.
  Candidate extension SHA-256:
  `58c029273cb3aedd0bd1350fe2dde22fe703559c819ace8e90ce52d06e25d562`.
- Harness base SHA-256:
  `1f1bdfa8f2cf886db3874b2ae9160d9c68f89f0a4026a19bc9ada47948606fb8`.
  Apply [`prefix-fusion-explicit-warmup-v2.patch`](prefix-fusion-explicit-warmup-v2.patch),
  SHA-256 `dba74d632d2abdcc20c73e90a8b2538efbd0f298d797488dde6d8bf427e6dc45`.
  Patched harness SHA-256:
  `61b640657990bebf80a1d19a1cd4cd0e99e2adcb7234161ea9b984b6971669ab`.
- DUCC 0.42.0, source revision
  `80388904b8cfc32400ba84af8a6a45aabfe60f86`, extension SHA-256
  `1218502fa03116cd14578fe73f716f5a1d88db74d296359cc9e8c27e04047d1e`.
- SHTns 3.7.5, source revision
  `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`, library SHA-256
  `641195ac574ba02078f3238b357caecf0e268b3f4ef7eba1d87898197fcc888f`.
- Build configuration: Rust/Cargo 1.99.0, release profile, all features.
  Runtime environment: Python 3.11.15, NumPy 2.4.6. The host used for this
  screen has an AMD Ryzen 9 5950X, x86-64, AVX2 and FMA, 16 physical and 32
  logical cores, and 96 GiB installed RAM; Ubuntu 26.04.1. These hardware
  specifications are recorded for reproducibility. Host identifiers and
  transient state are excluded.

## Fixed workload and acquisition

Use GL f64 scalar `STANDARD` synthesis, `lmax=mmax=2047`, a `2048x4096`
grid, `phi0=0.37`, q1, `scope=into`, `operation=synthesis`, `backend=all`, and
`runs=1`. Use one fixed allowed CPU for both processes, with one Rust Rayon,
DUCC, SHTns, and BLAS thread. The command arguments are:

```text
2047 0 1 GL 1 into synthesis all STANDARD 4096 2048
```

Set `RUSTSHT_PERF_RAW=1` and `RUSTSHT_BENCH_EXTRA_WARMUPS=3`. For each backend,
the harness performs its fixed correctness call, three additional untimed
warm-ups, its existing untimed warm-up, then five timed complete transforms.
The timing interval covers the existing plan's synthesis into caller-owned
output. Plan creation, coefficient preparation, and map allocation are outside
that interval. The harness reports RustSHT and SHTns numerical errors against
DUCC before timing. Use the same configuration and harness for both builds.

Build and verify both extensions before acquisition. Do not compile or run
another performance workload while either process is active. Capture raw
stdout, GNU-time data, PSI, process/thread placement, frequency, affinity,
context switches, and cgroup state outside the repository. Record exact
preflight and postflight PSI locally. PSI `some avg10` above 2.00% rejects a
process; readings from 0.50% through 2.00% are warnings. Clear evidence of
interference invalidates a process even below the PSI threshold.

## Fixed validity and numerical gates

- Each RustSHT, DUCC, and SHTns five-sample series must have population CV at
  most 5% and first-three/final-two median drift at most 5%.
- Require candidate and baseline RustSHT results, and SHTns, to agree with
  DUCC within relative-L2 `2e-11` and maximum absolute error `2e-12`.
- Retain every timed sample. Do not retry, select, or remove samples, and do
  not change these gates after observing a result.
- Report per-backend median, five timing samples, CV, drift, numerical errors,
  aggregate peak RSS, hardware/software configuration, PSI warning/gate
  outcome, and interference conclusion. Keep raw telemetry and command output
  local and untracked.

The candidate remains isolated after this screen. Passing only authorizes the
six-pair q1 T2047 matrix; it does not establish T1023/T511 behavior, q8 scaling,
spin-1 regressions, or production acceptance.
