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

## Provenance limitation

All 24 successful matrix preflights record ELF SHA-256
`2d5ddf6febd6ef9fd480a0c2cd97feb1f10f89ee0e003097cfa2b2be0ea6e4fb`. The
archived source snapshot is present, but the successful ELF and its build log
were not retained. A separate low-resolution smoke attempt failed with
`NonRealMZero { degree: 1 }`; it has no recorded ELF hash and is excluded from
the matrix. A post-screen replay used a different local ELF hash
`aaf5808ed463632ac0926360265b3411f557eb2f8226f49510245a42c3bb54b6` and failed
with the same error. That replay is also excluded. The raw matrix preflights
and both failed attempts are retained so they are not mistaken for accepted
measurements. The result is not tied conclusively to the archived source snapshot. Before a
performance or numerical acceptance decision, build the archived source in a
clean detached worktree with an isolated `CARGO_TARGET_DIR`, retain the build
log and ELF hash, then rerun the matrix and investigate any recurrence of the
spin-1 `m=0` failure. No acceptance is based on this screen.
