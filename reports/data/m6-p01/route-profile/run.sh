#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(git rev-parse --show-toplevel)
out_dir="$repo_dir/reports/data/m6-p01/route-profile"
log="$out_dir/candidate-test.log"
if [[ -e "$log" || -e "$out_dir/source.patch" ]]; then
  printf 'route-profile evidence already exists; refusing to overwrite\n' >&2
  exit 2
fi

cd "$repo_dir"
some_avg10=$(awk '/some/ {for (i=1; i<=NF; i++) if ($i ~ /^avg10=/) {sub("avg10=", "", $i); print $i}}' /proc/pressure/cpu)
busy=$(ps -eo pid,pcpu,stat,comm,args --sort=-pcpu | awk -v self="$$" '$1 != self && (($2 + 0) >= 8) {print}')
if awk -v value="$some_avg10" 'BEGIN {exit !(value >= 8.0)}' || [[ -n "$busy" ]]; then
  printf 'preflight refused: cpu_some_avg10=%s; competing=%s\n' "$some_avg10" "$busy" >&2
  exit 75
fi

{
  printf 'timestamp=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'branch_head=%s\n' "$(git rev-parse HEAD)"
  printf 'rustc='; rustc --version
  printf 'cargo='; cargo --version
  printf 'cpu_some_avg10=%s\n' "$some_avg10"
  cat /proc/pressure/cpu
  ps -eo pid,ppid,psr,pcpu,stat,comm,args --sort=-pcpu | head -n 20
} > "$out_dir/provenance.txt"

git diff HEAD -- src/legendre/mod.rs src/legendre/scalar.rs \
  src/legendre/scalar_paired.rs src/transform.rs > "$out_dir/source.patch"
cp src/legendre/route_profile.rs "$out_dir/route_profile.rs"
sha256sum src/legendre/mod.rs src/legendre/scalar.rs \
  src/legendre/scalar_paired.rs src/transform.rs \
  src/legendre/route_profile.rs > "$out_dir/source-sha256.txt"

TMPDIR="$repo_dir/target/tmp" \
RUSTSHT_ROUTE_LMAX=1023 \
RUSTSHT_ROUTE_THREADS=1 \
cargo test --release -p rustsht --features parallel --lib \
  legendre::route_profile::m6_scalar_synthesis_routes \
  -- --ignored --exact --nocapture > "$log" 2>&1

test_executable=$(sed -nE 's/^ *Running unittests src\/lib\.rs \(([^)]*)\)$/\1/p' "$log" | head -n 1)
if [[ -z "$test_executable" ]]; then
  printf 'could not extract test executable from %s\n' "$log" >&2
  exit 1
fi
sha256sum "$test_executable" > "$out_dir/test-binary-sha256.txt"
printf '%s\n' "$test_executable" > "$out_dir/test-binary-path.txt"
