# Plan 15 native spin-1 benchmark adapter

`ducc_spin1_full_transform_parity.cpp` and
`examples/ducc_spin1_full_transform_parity.rs` expose matched native spin-1
STANDARD E/B analysis and synthesis workers for GL, CC and F1. The Python
runner calibrates repeated calls, pins q1/q8 to the same allowed physical CPU
sets, interleaves native Rust and DUCC processes, and compares both channels.
It records plan/setup, first-call, warm-call and batch timings separately.

The initial smoke was run on Rust source `341661a0256ecab329e5de65ad286f96a8a73914`
against DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, with GCC
15.2 and `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` for DUCC's
numerical translation units. The driver overrides fast math with
`-fno-fast-math`. It covered six T4 cases (GL/CC/F1 × even-smooth/odd-minimum
longitude), A/S, q1/q8, and one interleaved process pair. All 24 output
comparisons and repeated/stale-destination checks passed; maximum relative L2
error was `6.19e-16`, and maximum absolute error was `3.56e-15`.

This is only an adapter smoke, not the Plan 15 performance baseline. The
full representative matrix has not been run. Reproduce the smoke with:

```sh
python3 scripts/benchmark_ducc_spin1_native.py \
  --ducc-src "${DUCC_SRC:?set pinned DUCC checkout}" \
  --smoke --process-pairs 1 --target-ms 25 \
  --results-dir "${TMPDIR:-.}/plan15-spin1-smoke"
```
