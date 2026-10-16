# f32 scalar recurrence range-skip data

These files record a scalar f32 Legendre-recurrence optimization for the GL
T1023 workload. The paired benchmark compares RustSHT STANDARD synthesis and
analysis with a reused plan and workspace. Positive candidate changes in the
JSON summaries mean longer elapsed time; negative changes mean shorter elapsed
time.

## Workload and build

- Baseline source: `2aea96bdb23b24b762dc9c83102773cd51d12a37` on
  `opt/sht-kernel-performance`.
- CPU: AMD Ryzen 9 5950X, 16 cores, 32 logical CPUs, AVX2 and FMA; no AVX-512.
- Compiler: rustc `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`.
- Build: Cargo release profile with `--all-features`, portable Pulp runtime
  dispatch, and no global `target-cpu=native` flag.
- Timing affinity: CPUs 0–7. The host is shared; the cgroup has no CPU quota.
- DUCC reference: `ducc0==0.41.0`, source revision
  `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.
- T1023 grid: GL `1024×2048`, full `mmax`, `phi0=0.37`, f32 maps and complex64
  coefficients.
- T720 control grid: GL `721×1458`, full `mmax`, `phi0=0.37`, f32 maps and
  complex64 coefficients.
- Each process records plan and workspace setup, one warm-up, five samples per
  operation, q1 and q8 timings, and peak RSS. The paired scripts alternate
  process order and use identical inputs for the two binaries.

## Candidate versions

| Binary | SHA-256 | Role |
| --- | --- | --- |
| Baseline | `aecb8aa5fd88d874bad71fdf8fe284187df0cb518d65fb2a0ef125610d691cf0` | Commit `2aea96b` reference |
| First candidate | `e206d0fa4e90a6d8cd624b82b3e424c3d50bba9d422bc3c4e1c527909c9b0fbf` | Ungated screening binary; discovery data only |
| Gated candidate | `b2848d9a0b4d62586b42c07077b7fa4a4295835d61c290666543795dd80e80dc` | Intermediate runtime-gated version; T720 regression, rejected |
| Specialized candidate | `785db7f0e599df71f8788d93d1422eb2c17df51635e60ba087537f396831c237` | Compile-time fallback specialization, enabled only at T1023 |

The specialized source patch atop the baseline commit has SHA-256
`aac86900b02ec4c830ee22de68f5bed3452f3b69fc1952989de8271b376b313c`.

## Final paired results

The table reports paired median change and range for the specialized
candidate. All changes use candidate time relative to baseline time.

| Truncation | Operation | q1 change | q8 change |
| --- | --- | ---: | ---: |
| T1023 | Synthesis | −4.39% (−4.76% to −4.32%) | −2.95% (−3.80% to −1.67%) |
| T1023 | Analysis | −4.49% (−4.50% to −4.44%) | −1.10% (−1.72% to −0.88%) |
| T720 | Synthesis | −1.04% (−1.34% to −0.81%) | −0.36% (−0.46% to −0.15%) |
| T720 | Analysis | −0.36% (−1.10% to −0.11%) | −0.23% (−0.38% to −0.08%) |

The T720 path is disabled. Its small measured differences are a selector
control, not a skip-path speedup. The process medians, paired ranges, and peak
RSS values are in `summary-specialized-T1023.json` and
`summary-specialized-T720.json`.

The T1023 candidate's six full outputs match baseline bytes exactly. The
DUCC 0.41.0 comparison reports relative-L2 errors from `1.67e-5` to
`4.87e-5`; the complete metrics are in `final-ducc-comparison.json`.

The T1023 five-pair screen and the first T720 three-pair screen use the first
candidate. The `paired-gated-clean-T1023` and `paired-gated-clean-T720`
directories use the gated candidate. They are retained as intermediate results
and do not describe the specialized candidate. Final pairs are in
`paired-specialized-T1023` and `paired-specialized-T720`.

The `paired-final-T1023` and `paired-gated-T1023` directories contain partial
runs that overlapped another benchmark. Their README files mark the unusable
samples. No timing summary uses those runs.

## Scripts and comparisons

- `run-paired.sh` runs five T1023 pairs for the first candidate.
- `run-t720-paired.sh` runs three T720 pairs for the first candidate.
- `run-selected-paired.sh` accepts binaries, output directory, `lmax`, `nphi`,
  and pair count for later candidates.
- `summarize.py` calculates process medians, paired changes and ranges.
- `compare-baseline-outputs.py` compares all dumped RustSHT arrays by bytes,
  relative L2 error and maximum absolute difference.
- The JSON files record full-array comparisons against the baseline and DUCC.
- `validation/` contains the passing formatting, Clippy, all-feature,
  no-default-feature, and T1023 DUCC test logs.

Generated full-size arrays and benchmark binaries live under ignored `target/`;
the scripts, process logs, timing files and comparison summaries are committed.
