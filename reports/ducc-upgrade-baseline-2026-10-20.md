# Historical RustSHT/DUCC toolchain and kernel baseline — archived

**Status: HISTORICAL EVIDENCE, NOT CURRENT SOURCE OR AN ACTIVE TASK.**
The [complete post-upgrade benchmark and disassembly report](archive/experiments/ducc-upgrade-baseline-2026-10-20.md)
preserves source versions, GCC/Clang compiler flags, numerical
conditions and distributions. The original [structured baseline
data](data/) remain unchanged.

This was the **Rust 1.99.1/LLVM 23.1.1** and **DUCC 0.42.0**
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`
rebaseline. DUCC SHT compilation uses
`-O3 -ffast-math -march=x86-64-v3` in upstream multiarch;
the standalone SHT kernel explicitly uses
`-ffp-contract=fast`, while its numeric validation driver uses
`-fno-fast-math`. Rust remains strict with V3 AVX2/FMA.
Compiler-dependent timing differences persisted under
high scheduling interference; do not assign them a clean
algorithmic cause without quiet-host matched reruns.

The most recent **T2047/2048 prefix-route cutoff** and T720
production/candidate dispositions are in
[current status](../PERFORMANCE_STATUS.md) and
[the concise current result index](scalar-analysis-cumulative-2026-10-20.md).

## Full-transform results

Read [original full-transform result tables](archive/experiments/ducc-upgrade-baseline-2026-10-20.md#full-transform-results).

## Matched ordinary-kernel results

Read the [archived matched kernel comparison](archive/experiments/ducc-upgrade-baseline-2026-10-20.md#matched-ordinary-kernel-results).

### Flag confirmation rerun at `6682095c`

Read the [original flag verification and timing caveats](archive/experiments/ducc-upgrade-baseline-2026-10-20.md#flag-confirmation-rerun-at-6682095c).

### Executed AVX2 loop and GCC tuning control

Read the [archived disassembly and compiler study](archive/experiments/ducc-upgrade-baseline-2026-10-20.md#executed-avx2-loop-and-gcc-tuning-control).
