#!/usr/bin/env bash
set -euo pipefail

: "${DUCC_SRC:?set DUCC_SRC to the pinned DUCC checkout}"
expected_commit=42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b
actual_commit=$(git -C "$DUCC_SRC" rev-parse HEAD)
if [[ "$actual_commit" != "$expected_commit" ]]; then
  printf 'DUCC checkout is %s; expected %s\n' "$actual_commit" "$expected_commit" >&2
  exit 2
fi

compiler=${CXX:-g++}
build_dir=${RUSTSHT_PLAN14_DUCC_BUILD_DIR:-${TMPDIR:-/tmp}/plan14-ducc-q8-odd-inverse}
mkdir -p "$build_dir"
base_flags=(-std=c++17 -O3 -march=x86-64-v3 -pthread -I"$DUCC_SRC/src")
fast_flags=(-ffast-math -ffp-contract=fast)
strict_flags=(-fno-fast-math -ffp-contract=off)

"$compiler" "${base_flags[@]}" "${fast_flags[@]}" -c ducc/kernel.cpp -o "$build_dir/kernel.o"
"$compiler" "${base_flags[@]}" "${strict_flags[@]}" -c ducc/driver.cpp -o "$build_dir/driver.o"
"$compiler" "${base_flags[@]}" "${fast_flags[@]}" -c "$DUCC_SRC/src/ducc0/infra/threading.cc" -o "$build_dir/threading.o"
"$compiler" "${base_flags[@]}" "${fast_flags[@]}" -c "$DUCC_SRC/src/ducc0/infra/string_utils.cc" -o "$build_dir/string_utils.o"
"$compiler" -O3 -march=x86-64-v3 -pthread "$build_dir/driver.o" "$build_dir/kernel.o" "$build_dir/threading.o" "$build_dir/string_utils.o" -o "$build_dir/ducc-q8-odd-inverse"

printf 'DUCC revision: %s\n' "$actual_commit"
printf 'Compiler: %s\n' "$($compiler --version | head -1)"
printf 'Kernel flags: -std=c++17 -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3\n'
printf 'Driver flags: -std=c++17 -O3 -fno-fast-math -ffp-contract=off -march=x86-64-v3\n'
printf 'Thread support flags: same optimized x86-64-v3 flags as the kernel\n'
printf 'Binary: %s\n' "$build_dir/ducc-q8-odd-inverse"
