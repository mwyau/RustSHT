#!/usr/bin/env bash
set -euo pipefail

crate_dir=$(cd "$(dirname "$0")/.." && pwd)
reference_dir=${RUSTSHT_FFT_BUILD_DIR:-/tmp/rustsht-fft-reference}
reference_binary=$reference_dir/rustsht-fft-reference

if [[ ! -x "$reference_binary" ]]; then
    if [[ -z "${DUCC_SRC:-}" ]]; then
        printf 'DUCC_SRC is required when the reference binary has not been built\n' >&2
        exit 2
    fi
    reference_binary=$(DUCC_SRC="$DUCC_SRC" RUSTSHT_FFT_BUILD_DIR="$reference_dir" \
        "$crate_dir/benches/reference/build.sh")
fi

printf '%s\n' '== Benchmark environment =='
if [[ -f "${reference_binary%/*}/build-info.txt" ]]; then
    cat "${reference_binary%/*}/build-info.txt"
else
    printf 'DUCC revision: %s\n' "$(git -C "${DUCC_SRC:-/tmp/rustsht-fft-ducc}" rev-parse HEAD)"
fi
printf 'Rust compiler: %s\n' "$(rustc --version)"
printf 'Rust benchmark profile: cargo bench release defaults (opt-level=3)\n'
printf 'Host: %s; CPU: %s\n' "$(uname -sm)" "$(awk -F: '/model name/ {gsub(/^ +/, "", $2); print $2; exit}' /proc/cpuinfo)"
printf 'DUCC vectorize planner flag: %s\n' "${RUSTSHT_FFT_DUCC_VECTORIZE:-0}"
printf 'DUCC thread count per transform: 1; batch rows: 1 and 32 (4 above 10000)\n'
printf 'Benchmark affinity: %s\n' "${RUSTSHT_FFT_CPU:-$(taskset -pc $$ 2>&1 | sed 's/.*affinity list: //')}"
printf 'Load average: %s\n' "$(cut -d' ' -f1-3 /proc/loadavg)"

run_affined() {
    if [[ -n "${RUSTSHT_FFT_CPU:-}" ]]; then
        taskset -c "$RUSTSHT_FFT_CPU" "$@"
    else
        "$@"
    fi
}

if [[ "${1:-}" == "--sht" ]]; then
    printf '%s\n' '== Direct DUCC 1D plans at RustSHT longitude row counts =='
    run_affined "$reference_binary" bench-sht
    printf '%s\n' '== RustFFT at RustSHT longitude row counts =='
    run_affined cargo bench --manifest-path "$crate_dir/Cargo.toml" --bench rustfft -- bench-sht
else
    printf '%s\n' '== Direct DUCC 1D plans =='
    run_affined "$reference_binary" bench "$@"

    printf '%s\n' '== RustFFT with reusable plans and scratch =='
    run_affined cargo bench --manifest-path "$crate_dir/Cargo.toml" --bench rustfft -- bench "$@"
fi
