# T127 compact-table SIMD A/B data

The four `results.json`/`results.csv` pairs are the native full-transform
baseline and candidate captures. Each JSON contains its benchmark protocol,
all batch samples, fixture checksums, numerical checks and host summary.
`comparison.csv` joins the matched q1 and q8 case summaries. The q1 SVGs
show first-call and warmed Rust timings for each source. DUCC is pinned to
0.42.0 commit `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

The baseline source is clean commit `1bfede726e824d4ded35201308de6b1da70814af`.
The candidate was measured from that commit with exactly the four-file diff
later committed as `0484a876b76aaa5927f6be3924e227452c2d3ed8`; consequently
candidate JSON records `rust_worktree_clean: false`. No other working-tree
changes were present. See the parent report for compiler flags, affinity,
interleaving, uncertainty and limitations.

`phase-summary.csv` retains the smaller diagnostic table-versus-recurrence
and table-phase readings. Those measurements are supporting diagnostics;
the calibrated whole-transform A/B is the primary performance evidence.
