# f32-to-f64 bridge screen provenance

This experiment is an unaccepted diagnostic screen. It does not change the
public f32 execution path.

## Source and tools

- RustSHT source revision: `490f71b6214f75dca23932b9f1089b816428fc25`.
- Archived benchmark source SHA-256:
  `eaeb8c3884e8ab490e49514a471fe9913d2d78e4a72e9868ed57ed80ad90cdd7`.
- Rust and Cargo: `1.99.0`.
- Host: x86-64 CPU, x86_64, AVX2 and FMA available.
- DUCC: `ducc0` 0.41.0, source revision
  `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.
- Benchmark source: `source/m6_f32_bridge_bench.rs`.
- DUCC comparison: `compare_ducc.py`.

## Configuration

The matrix covers GL STANDARD T127, T511 and T1023; scalar and spin-1;
synthesis and analysis; q1 and q8. Each grid has `ntheta=T+1`,
`nphi=2*(T+1)` and `phi0=0.37`. q1 was pinned to one CPU core and q8 to 8 pinned CPU cores. The Rust process compared native f32, per-call widening to f64 plus f64
transform and output downcast, and native f64 on the same f32-rounded inputs. Each cell used one warmup and five alternating within-process samples per
variant. DUCC f32 and f64 used separate q-specific processes, each with one
warmup and five timed calls. Process preflights, CPU pressure, elapsed time,
RSS and raw samples are retained under `raw/`. The bridge included allocation of f64 inputs and outputs in its timed region. The Rust process kept all three variants' plans, workspaces and output buffers
alive for comparison. GNU time RSS is therefore process-wide and cannot be
interpreted as bridge-only memory. The reported per-call temporary-array size
is computed from the f64 input/output payload sizes.

## Source-linked reproduction

The initial 24-cell matrix recorded ELF SHA-256
`2d5ddf6febd6ef9fd480a0c2cd97feb1f10f89ee0e003097cfa2b2be0ea6e4fb`, but its
build log and ELF were not retained. A clean detached worktree at revision
`73493cf410809f8bbe0455f252b201188d0fc6a5` rebuilt the archived benchmark
source in an isolated `CARGO_TARGET_DIR`. The Rust source at this revision is
unchanged from the original screen revision
`490f71b6214f75dca23932b9f1089b816428fc25`; the intervening commit records
documentation and evidence. Clippy and release build passed, and the rebuilt
binary reproduced the recorded ELF hash exactly. The source-linked T127 spin-1 q1 synthesis smoke passed. A fresh 24-cell matrix
then completed with exit status zero for every cell, no refused runs, and the
same ELF hash in all 24 preflights. Fresh DUCC f32/f64 full-array comparisons
used `ducc0` 0.41.0 at the pinned source revision. The current timings,
comparisons, build output, binary hash, matrix status, and per-cell records are
in [`replay-source-linked/`](replay-source-linked/). The initial-screen raw
files remain in `raw/`; its q1 DUCC medians differ in some cells from the
source-linked rerun, so the two timing tables are kept separate. The first smoke attempt has no recorded ELF hash and failed with
`NonRealMZero { degree: 1 }`. A separate post-screen replay using ELF hash
`aaf5808ed463632ac0926360265b3411f557eb2f8226f49510245a42c3bb54b6` failed
with the same error. Both are retained and excluded. The source-linked build
and fresh matrix passed these cells.

## Limits

This remains a screening experiment. Each Rust cell uses one process with five
alternating within-process samples, not independent paired processes. The
matrix covers even-longitude GL STANDARD synthesis and analysis; it does not
cover odd longitude counts, adjoints, extreme coefficients, or high-L f32
error envelopes. GNU time RSS includes all comparison plans, workspaces and
buffers. The f64 bridge remains slower than DUCC f32 by as much as 13.00x at
T1023 scalar q8 synthesis. No public f32 execution change is accepted.
