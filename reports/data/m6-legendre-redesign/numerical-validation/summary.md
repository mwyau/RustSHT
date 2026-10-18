# Tile-128 prototype numerical test checks

Both Rust test configurations passed against the isolated candidate at source
commit `739275d20987fec35506916cb682f961e8a7ffaf`, plus the uncommitted
tile-128 candidate patch. The tested `src/legendre/synthesis_fast.rs` SHA-256
was `0c0ee956205f782626e6ab0ffc6914cfa9f0d1e20caf5075111a53a9c9449b25`;
the frozen release extension SHA-256 was
`5eeff4dd2087238f18278928ad2af84437d21b7caa38eb360783f9b627cf8620`.

| Command | Passed | Ignored | Failed | Wall time | Peak RSS |
|---|---:|---:|---:|---:|---:|
| `cargo test --workspace --all-features` | 216 | 15 | 0 | 4:37.66 | 446.3 MiB |
| `cargo test --no-default-features` | 177 | 8 | 0 | 6:14.23 | 1.28 GiB |

The totals include the root library, FFT integration, f32 integration, and
workspace-compatibility test targets. The test totals, wall times, and peak RSS are summarized above. Raw build/test
logs, GNU-time files, and host snapshots are local and untracked. Both commands
used Rust 1.99.0, a local temporary directory, and `RUST_TEST_THREADS=1`. No benchmark ran concurrently with either test
suite. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The checks cover the candidate's focused tile, polar-lane, mixed-fallback and
scaled-to-ordinary transition tests, plus workspace compatibility and the
existing DUCC comparisons. They do not include an external DUCC comparison
for the complete T1023 or T2047 candidate transform. The paired benchmark
harness checks candidate maps against DUCC on each process, and the candidate
remains isolated pending the complete timing and regression controls.
