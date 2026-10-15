#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    printf 'usage: %s fftw|admiral SOURCE_DIR OUTPUT_BINARY\n' "$0" >&2
    exit 2
fi

backend=$1
if [[ "$2" == "system" ]]; then
    source_dir=system
else
    source_dir=$(cd "$2" && pwd)
fi
output=$(mkdir -p "$(dirname "$3")" && cd "$(dirname "$3")" && pwd)/$(basename "$3")
root=$(cd "$(dirname "$0")/../.." && pwd)
build_root=${RUSTSHT_FFT_REFERENCE_BUILD_DIR:-${RUSTSHT_TMP_DIR:-${TMPDIR:-$(python3 -c 'import tempfile;print(tempfile.gettempdir())')}}/rustsht-fft-external}

case "$backend" in
    fftw)
        if [[ "$2" == "system" ]]; then
            version=$(pkg-config --modversion fftw3)
            library=$(ldconfig -p | awk '/libfftw3\.so\.3 / { print $NF; exit }')
            "${CXX:-g++}" -std=c++20 -O3 -DNDEBUG -DBENCH_FFTW \
                "$root/benches/reference/external_fft.cpp" -lfftw3 -o "$output"
            {
                printf 'Backend: FFTW system package\nVersion: %s\nLibrary: %s\n' "$version" "$library"
                printf 'Compiler: %s\n' "$(${CXX:-g++} --version | head -1)"
                printf 'Build flags: package build configuration not recorded; harness -O3 -DNDEBUG\n'
                printf 'Planning: FFTW_ESTIMATE | FFTW_PRESERVE_INPUT\n'
            } > "${output}.build-info.txt"
        else
            expected_commit=40e620ec4441d0502e55d73744ef5d29a340a288
            build_dir="$build_root/fftw-$expected_commit"
            actual_commit=$(git -C "$source_dir" rev-parse HEAD)
            if [[ "$actual_commit" != "$expected_commit" ]]; then
                printf 'FFTW checkout is %s; expected %s\n' "$actual_commit" "$expected_commit" >&2
                exit 2
            fi
            cmake -S "$source_dir" -B "$build_dir" \
                -DBUILD_TESTS=OFF -DENABLE_THREADS=OFF -DENABLE_AVX2=ON \
                -DCMAKE_BUILD_TYPE=Release
            cmake --build "$build_dir" --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-16}"
            "${CXX:-g++}" -std=c++20 -O3 -DNDEBUG -DBENCH_FFTW \
                -I"$source_dir/api" "$root/benches/reference/external_fft.cpp" \
                -L"$build_dir" -Wl,-rpath,"$build_dir" -lfftw3 -o "$output"
            {
                printf 'Backend: FFTW\nRevision: %s\n' "$actual_commit"
                printf 'Compiler: %s\n' "$(${CXX:-g++} --version | head -1)"
                printf 'Flags: CMake Release, ENABLE_AVX2=ON, ENABLE_THREADS=OFF; harness -O3 -DNDEBUG\n'
                printf 'Planning: FFTW_ESTIMATE | FFTW_PRESERVE_INPUT\n'
            } > "${output}.build-info.txt"
        fi
        ;;
    admiral)
        expected_commit=ce67bc00ff9e6de96bf5b75f7a789581105ddb58
        build_dir="$build_root/admiral-$expected_commit"
        actual_commit=$(git -C "$source_dir" rev-parse HEAD)
        if [[ "$actual_commit" != "$expected_commit" ]]; then
            printf 'Admiral checkout is %s; expected %s\n' "$actual_commit" "$expected_commit" >&2
            exit 2
        fi
        cmake -S "$source_dir" -B "$build_dir" \
            -DCMAKE_BUILD_TYPE=Release -DADM_BUILD_TESTS=OFF \
            -DADM_BUILD_BENCHMARKS=OFF -DADM_BUILD_EXAMPLES=OFF \
            -DADM_ENABLE_THREADS=OFF -DADM_USE_FAST_MATH=OFF \
            -DADM_PIN_PLAN_ESTIMATE=ON -DADM_TARGET_ARCH=native
        cmake --build "$build_dir" --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-16}" --target admiral
        "${CXX:-g++}" -std=c++20 -O3 -DNDEBUG -DBENCH_ADMIRAL \
            -I"$source_dir/include" "$root/benches/reference/external_fft.cpp" \
            -L"$build_dir/src" -Wl,-rpath,"$build_dir/src" -ladmiral -o "$output"
        {
            printf 'Backend: Admiral\nRevision: %s\n' "$actual_commit"
            printf 'Compiler: %s\n' "$(${CXX:-g++} --version | head -1)"
            printf 'Flags: CMake Release, ADM_TARGET_ARCH=native, ADM_USE_FAST_MATH=OFF, ADM_ENABLE_THREADS=OFF; harness -O3 -DNDEBUG\n'
            printf 'Planning: admiral::effort::estimate with ADM_PIN_PLAN_ESTIMATE=ON\n'
        } > "${output}.build-info.txt"
        ;;
    *)
        printf 'unknown backend: %s\n' "$backend" >&2
        exit 2
        ;;
esac

printf '%s\n' "$output"
