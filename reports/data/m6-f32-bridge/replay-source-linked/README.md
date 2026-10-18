# Source-linked f32 bridge replay

This is the current f32 bridge timing screen. It rebuilds the archived example
from RustSHT revision `73493cf410809f8bbe0455f252b201188d0fc6a5` in an
isolated target directory. That revision is a documentation-only descendant
of the original benchmark revision `490f71b6214f75dca23932b9f1089b816428fc25`. The example source hash is
`eaeb8c3884e8ab490e49514a471fe9913d2d78e4a72e9868ed57ed80ad90cdd7`. The clean release binary hash is
`2d5ddf6febd6ef9fd480a0c2cd97feb1f10f89ee0e003097cfa2b2be0ea6e4fb`, matching
all 24 successful Rust cell preflights. The committed report retains structured per-cell timing samples, numerical errors,
source and binary hashes, and the matrix validity summary. Raw build/test output,
process output, host provenance and GNU-time captures remain local and untracked. The fresh replay measured native f32, f32 inputs through the f64 plan, native
f64 and DUCC f32/f64 on GL STANDARD T127/T511/T1023; scalar and spin-1;
synthesis and analysis; q1 and q8. Maximum full-array relative-L2 error
against DUCC f32 was `4.873e-5` for native f32 and `5.474e-8` for the bridge. The bridge ran 3.49x–31.74x faster than native f32. It remained up to 13.00x
slower than DUCC f32 for T1023 scalar q8 synthesis. See `summary.csv` and
`summary.md` for the full table. DUCC comparisons used version 0.41.0 at source revision
`c1ed01b8fda69d2a378190f0915f1a19762dddbf`, from the project Python 3.11
environment. q1 ran on one CPU core; q8 ran on 8 pinned CPU cores. Each Rust cell used one
warmup and five within-process alternating samples. Each DUCC cell used one
warmup and five timed calls. The two q-specific DUCC processes ran
sequentially. The full comparison generated 108 output dumps totalling 364.7 MiB. They are
not stored in Git. `dumps-sha256.manifest` records their hashes; deterministic
input generation and the comparison program are in the parent directory. `ducc-q1-samples.json` and `ducc-q8-samples.json` retain structured full-array
error metrics and timing samples.

## Reproduction

Start from a clean worktree and use a separate Cargo target directory. Copy
the archived source and scripts into that worktree, then build before running
any benchmarks:

```sh
 : "${RUSTSHT_LOCAL_DIAG_DIR:?set a directory outside Git worktrees}"
: "${RUSTSHT_SOURCE_ARCHIVE:?set the local source archive directory}"
: "${RUSTSHT_PYTHON_BIN:?set the Python 3.11 executable}"
worktree="$RUSTSHT_LOCAL_DIAG_DIR/worktree"
target_dir="$RUSTSHT_LOCAL_DIAG_DIR/target"
evidence="$RUSTSHT_SOURCE_ARCHIVE"
python_bin="$RUSTSHT_PYTHON_BIN"
mkdir -p "$RUSTSHT_LOCAL_DIAG_DIR" "$target_dir/tmp"
git worktree add --detach "$worktree" 490f71b6214f75dca23932b9f1089b816428fc25
mkdir -p "$worktree/examples" "$worktree/reports/data/m6-f32-bridge"
cp "$evidence/source/m6_f32_bridge_bench.rs" "$worktree/examples/"
cp "$evidence/run-one.sh" "$evidence/run-matrix.sh" "$evidence/compare_ducc.py" "$evidence/summarize.py" "$worktree/reports/data/m6-f32-bridge/"
CARGO_TARGET_DIR="$target_dir" TMPDIR="$target_dir/tmp" cargo clippy --manifest-path "$worktree/Cargo.toml" --release -p rustsht --example m6_f32_bridge_bench --all-features -- -D warnings > "$RUSTSHT_LOCAL_DIAG_DIR/rebuild.log" 2>&1
CARGO_TARGET_DIR="$target_dir" TMPDIR="$target_dir/tmp" cargo build --manifest-path "$worktree/Cargo.toml" --release -p rustsht --example m6_f32_bridge_bench >> "$RUSTSHT_LOCAL_DIAG_DIR/rebuild.log" 2>&1
sha256sum "$target_dir/release/examples/m6_f32_bridge_bench" > "$RUSTSHT_LOCAL_DIAG_DIR/binary.sha256"
bash "$worktree/reports/data/m6-f32-bridge/run-matrix.sh" "$worktree" "$target_dir/release/examples/m6_f32_bridge_bench" "$RUSTSHT_LOCAL_DIAG_DIR/replay-output"
taskset -c "${RUSTSHT_CPUSET_Q1:?set local affinity for one worker}" /usr/bin/time -v -o "$RUSTSHT_LOCAL_DIAG_DIR/ducc-q1.time" "$python_bin" "$worktree/reports/data/m6-f32-bridge/compare_ducc.py" "$RUSTSHT_LOCAL_DIAG_DIR/replay-output/dumps" 1 > "$RUSTSHT_LOCAL_DIAG_DIR/ducc-q1.json"
taskset -c "${RUSTSHT_CPUSET_Q8:?set local affinity for eight workers}" /usr/bin/time -v -o "$RUSTSHT_LOCAL_DIAG_DIR/ducc-q8.time" "$python_bin" "$worktree/reports/data/m6-f32-bridge/compare_ducc.py" "$RUSTSHT_LOCAL_DIAG_DIR/replay-output/dumps" 8 > "$RUSTSHT_LOCAL_DIAG_DIR/ducc-q8.json"
RUSTSHT_M6_F32_SOURCE_LINKED=1 python3 "$worktree/reports/data/m6-f32-bridge/summarize.py"
```

The commands build before running the serial Rust matrix, then run DUCC q1 and
q8 comparisons sequentially. Keep build output, ELF hashes, raw cell captures
and DUCC process output in `RUSTSHT_LOCAL_DIAG_DIR`; convert timing output to
structured samples and retain numerical summaries before committing evidence. The runner refuses cells when it observes a heavy competing
workload or CPU PSI above its configured threshold. This remains a screen: it has no independent process-pair repetitions, odd
longitude counts, adjoint operations or extreme-value tests. The f32 bridge is
not accepted as a public execution path.
