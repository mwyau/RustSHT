# DUCC multiarch build and SHT inspection

Reference: DUCC 0.42.0, commit
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Builds used the upstream CMake
option `DUCC0_OPTIMIZATION=multiarch` on Linux x86-64, with CMake 4.2.3,
GCC 15.2.0 and Clang 21.1.8. The host is an AMD Ryzen 9 5950X. It supports
AVX2/FMA and x86-64-v3, but not AVX-512/x86-64-v4.

## Reproduction

Set `DUCC_SRC` to a clean checkout at the pinned commit and `PYTHON` to a
Python with development headers and pybind11 available. CMake's multiarch
configuration creates separate `ducc0_lib_v1`, `ducc0_lib_v3` and
`ducc0_lib_v4` object targets. Run once with each compiler:

```sh
for compiler in g++ clang++-21; do
  build="${DUCC_BUILD_ROOT:?}/${compiler}"
  DUCC0_OPTIMIZATION=multiarch cmake -S "$DUCC_SRC" -B "$build" \
    -DCMAKE_CXX_COMPILER="$compiler" -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_EXPORT_COMPILE_COMMANDS=ON \
    -DPython_EXECUTABLE="$PYTHON" \
    -DSKBUILD_PROJECT_NAME=ducc0 -DSKBUILD_PROJECT_VERSION=0.42.0
  DUCC0_OPTIMIZATION=multiarch cmake --build "$build" --parallel 6
done
```

The SHT translation unit is compiled with `DUCC0_MULTIARCH` and a profile
namespace. CMake's optimized flags include `-O3 -ffast-math`; its object
targets add `-march=x86-64`, `-march=x86-64-v3`, or `-march=x86-64-v4` for v1,
v3, and v4. The standalone full-transform benchmark uses its own explicit
flags (`-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`); those builds
must be identified separately from these CMake builds.

Check runtime selection using the Python extension built by each compiler:

```sh
PYTHONPATH="$build" "$PYTHON" -c \
  'import ducc0; print(ducc0.misc.cpu_info())'
```

Both builds reported `multiarch=true`, compiled profiles v1/v3/v4, available
profiles v1/v3, and `active_profile=x86-64-v3`. A warmed T255 GL scalar
analysis smoke call through the public `ducc0.sht.analysis_2d` API returned
finite, repeat-identical coefficients under this dispatch. Its five-call
single-process timing screen varied substantially (q1 median 1.606 ms, q8
median 0.316 ms), so it is recorded only as a dispatch smoke check; the
calibrated cross-library baseline is a separate Phase 4 measurement.

## SHT machine-code findings

The inspection targets the actual `sht.cc.o` profile objects, not generic
SIMD symbols from unrelated DUCC components. `nm -C` finds f64
`detail_sht::analysis_2d` and the internal
`detail_sht_inner_loop::map2alm_kernel` in both v3 and v4 namespaces. The
symbols are defined in `ducc0_v3`/`ducc0_v4` and their instructions come from
the profile-specific build of `sht.cc`.

For GCC 15.2, the f64 `map2alm_kernel` object symbol is 722 bytes in v3 and
1,062 bytes in v4. Clang 21.1.8 emits 716 bytes in v3 and 1,229 bytes in v4.
The exact ranges were disassembled with `objdump -dC -Mintel`.

- GCC and Clang v3 loops use YMM packed-double operations: four latitude lanes
  per vector, `vbroadcastsd`, and `vfmadd132pd`/`213pd`/`231pd`. The recurrence
  and spectrum accumulation state remains in vector registers through the
  degree loop. The ordinary path has eight partial spectrum accumulators.
  Horizontal completion uses `vhaddpd` and cross-128-bit-lane shuffles/adds.
- GCC and Clang v4 SHT symbols contain ZMM packed-double loads, broadcasts and
  FMA operations across eight lanes. Their final reductions explicitly
  extract and combine 256-bit and 128-bit pieces. This confirms that the v4
  code belongs to SHT, rather than an unrelated AVX-512 object.
- No YMM/ZMM spill or reload appears in the hot recurrence loops inspected;
  the prologue saves general-purpose callee-saved registers. GCC and Clang
  choose different recurrence scheduling, FMA forms and code sizes even with
  the same profile and `-ffast-math`; static instruction counts alone do not
  establish which compiler is faster.

The v4 SHT routines were compiled and disassembled only; none was called or
executed on this AVX2-only host. Runtime and numerical comparisons use v3.
The next step is the matched Rust 1.99.1 versus DUCC GCC/Clang v3 baseline.
