# M6 triage evidence provenance

This directory preserves the bounded selection patch for the checked benchmark workers. The f64 selector derives from `scripts/benchmark_ducc_spin1_native.py` (original SHA-256 `e8364e3a18996bac98daa68cba348ccb0d6803576d26c167f63ce79d111ecabe`); the f32 selector derives from `scripts/benchmark_ducc_f32_native.py` (original SHA-256 `70033f7251358128783f35d7aefa527fa957abce5513571db1142e92e6e38ff0`). The zero-context patch SHA-256 is `63797a6c9566cdfae6d07284fd1401f001fbf177b6c66ccc9ed341ead64e563d`. The resulting bounded copies had SHA-256 values `a80b1c3a43a788978011d3fca19229d35247769c39d3ab8b5b91dcd011b921fd` and `98cfebe740a04d0506f01b9bde2a466e2f575594a14c0a98e0f31f6763702cd4`.

The patch was tested by copying the checked scripts into a clean directory and applying it with `patch -p1`; both resulting files matched the recorded hashes. It limits the f64 run to six spin-1 cells and makes `CARGO_TARGET_DIR` selectable. It limits f32 to eight full-mmax T720 cells and adds worker-level scheduler/frequency capture and output hashes. The checked repository scripts remain unchanged.

The Phase B profile also used temporary, test-only work counters and a dedicated Callgrind example. Their exact source diff from RustSHT `feb46e5f` is preserved in [profile-instrumentation.patch](profile-instrumentation.patch), SHA-256 `ef6991cc0f736971642f0c7f34cdf28a13287a813bdf0a0d199cc8ee826d3c92`. It includes the profile-only edits to `Cargo.toml`, spin-analysis diagnostics/tests/workspace counters, and the temporary `m6_spin1_analysis_callgrind` example. It is a zero-context patch: apply with `git apply --unidiff-zero` (or `patch -p1`). Forward application was checked against the current source tree and reverse application against the instrumented worktree; none of it is production code. The pinned DUCC checkout was clean at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Whole-call profile hashes and measured values are in [the Phase B report](../../DUCC_PARITY_M6.md) and [machine-readable profile CSV](../ducc-parity-m6-2026-10-27-profile.csv).

## Reproduction outline

Use a fresh scratch root, current source `26941cf45564a9637b72b0eca87ff880472c3505`, and pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Copy the two checked benchmark scripts into the clean source worktree, then apply `harness-selection.patch` from the repository root:

```sh
patch -p1 < reports/data/ducc-parity-m6-2026-10-27/harness-selection.patch
```

Build the Rust examples in an experiment-local Cargo target with `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'` and the `parallel` feature. Build the f64 and f32 DUCC adapters from `reports/data/plan15-spin1-native-baseline-2026-10-21` and `reports/data/plan17-f32-native-baseline-2026-10-22` with CMake Release and the pinned DUCC checkout. Use three interleaved process pairs; f64 target batch time is 25 ms and f32 target batch time is 10 ms. Pin q1 to CPU 0 and q8 to eight distinct physical cores. The ledger records binary hashes, flags, fixtures, pair medians, errors, output hashes (where emitted), wait fractions, and frequency summaries.

The following recreates the bounded triage from a fresh scratch root. Check free bytes and inodes on the selected transient filesystem before the build. `repo` names the primary checkout; all temporary source, build, cache, fixture, and result data stays under `scratch`.

```sh
repo="$(git rev-parse --show-toplevel)"
scratch="$(mktemp -d -t rustsht-m6-triage.XXXXXXXX)"
df -h "$(dirname "$scratch")"
df -i "$(dirname "$scratch")"
mkdir -p "$scratch/tmp"
git -C "$repo" worktree add --detach "$scratch/src" 26941cf45564a9637b72b0eca87ff880472c3505
git clone --quiet --no-hardlinks https://gitlab.mpcdf.mpg.de/mtr/ducc.git "$scratch/ducc-source"
git -C "$scratch/ducc-source" checkout --quiet --detach 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b
DUCC_SRC="$scratch/ducc-source"
cd "$scratch/src"
patch -p1 < "$repo/reports/data/ducc-parity-m6-2026-10-27/harness-selection.patch"

export CARGO_HOME="$scratch/cargo-home"
export CARGO_TARGET_DIR="$scratch/cargo-target"
export TMPDIR="$scratch/tmp"
export RUSTSHT_SPIN1_RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'

python3 scripts/benchmark_ducc_spin1_native.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --results-dir "$scratch/results/f64-spin1" \
  --build-dir "$scratch/build/f64-spin1" \
  --lmax 720 --grid GL --process-pairs 3 --target-ms 25

RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  cargo build --locked --release --example ducc_f32_full_transform_parity --features parallel
cmake -S "$scratch/src/reports/data/plan17-f32-native-baseline-2026-10-22" \
  -B "$scratch/build/f32-ducc" -DDUCC_SRC="$DUCC_SRC" \
  -DCMAKE_BUILD_TYPE=Release -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -DCMAKE_CXX_COMPILER=g++
cmake --build "$scratch/build/f32-ducc" --parallel 4

python3 scripts/benchmark_ducc_f32_native.py \
  --rust-binary "$CARGO_TARGET_DIR/release/examples/ducc_f32_full_transform_parity" \
  --ducc-binary "$scratch/build/f32-ducc/ducc_f32_full_transform_parity" \
  --outdir "$scratch/results/f32" --scratch-dir "$scratch/scratch/f32" \
  --process-pairs 3 --target-ms 10 --q1-cpu 0 \
  --q8-cpus 0,2,4,6,8,10,12,14
```

The q8 f32 scalar cases had elevated runqueue wait. Preserve that telemetry when comparing later runs; three pairs are a triage screen, not a confidence interval.
