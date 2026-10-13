#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${DUCC_SRC:-}" ]]; then
    printf 'DUCC_SRC must name the pinned DUCC checkout\n' >&2
    exit 2
fi

expected_commit=c1ed01b8fda69d2a378190f0915f1a19762dddbf
actual_commit=$(git -C "$DUCC_SRC" rev-parse HEAD)
if [[ "$actual_commit" != "$expected_commit" ]]; then
    printf 'DUCC checkout is %s; expected %s\n' "$actual_commit" "$expected_commit" >&2
    exit 2
fi

build_dir=${RUSTSHT_FFT_BUILD_DIR:-/tmp/rustsht-fft-reference}
mkdir -p "$build_dir"
read -r -a compiler_flags <<<"${CXXFLAGS:--O3 -DNDEBUG -march=native}"

"${CXX:-g++}" -std=c++20 "${compiler_flags[@]}" \
    -I"$DUCC_SRC/src" \
    "$(dirname "$0")/ducc_fft.cpp" \
    "$DUCC_SRC/src/ducc0/infra/threading.cc" \
    "$DUCC_SRC/src/ducc0/infra/string_utils.cc" \
    -pthread -o "$build_dir/rustsht-fft-reference"

{
    printf 'DUCC revision: %s\n' "$actual_commit"
    printf 'C++ compiler command: %s\n' "${CXX:-g++}"
    printf 'C++ compiler version: %s\n' "$("${CXX:-g++}" --version | head -1)"
    printf 'C++ flags: -std=c++20 %s -pthread\n' "${CXXFLAGS:--O3 -DNDEBUG -march=native}"
} > "$build_dir/build-info.txt"

printf '%s\n' "$build_dir/rustsht-fft-reference"
