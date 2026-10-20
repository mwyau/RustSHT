# Native validation against the current DUCC reference

Reference: DUCC 0.42.0, commit
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. The Rust worker used Rust 1.99.1
with runtime Pulp V3; the native DUCC worker used GCC 15.2.0 and fixed
x86-64-v3 AVX2/FMA. Both workers reported q1 and q8 pool sizes matching the
requested counts.

The T255 GL check covered `mmax=1`, `63`, and `255`, both odd-minimum and
even-smooth longitude counts, analysis and synthesis, and q1/q8. Six geometry
configurations produced 24 timed rows and 72 random-output comparisons. The
maximum Rust/DUCC relative L2 error was `8.976e-14`.

Twelve additional out-of-timing suites covered selected zonal, intermediate
and sectoral modes; roundtrips; exact zero, subnormal and extreme-finite
inputs; and analysis/synthesis adjoint identities. All outputs were finite,
all repeated calls reset consistently, and both backends passed. Maximum
roundtrip relative L2 was `8.841e-14`, selected-basis recovery error was
`1.664e-13`, and adjoint residual was `9.100e-14`.

The check used the same harness as the published historical report, with the
new immutable revision and a temporary result directory:

```sh
python3 scripts/benchmark_ducc_full_transform_parity.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --results-dir "${RESULTS_DIR:-validation-output}" \
  --lmax 255 --no-report
```

These measurements establish numerical compatibility only. The updated
Rust/DUCC performance baseline is recorded separately after the multiarch
build and compiler controls.
