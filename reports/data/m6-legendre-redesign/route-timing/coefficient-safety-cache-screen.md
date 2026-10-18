# Per-order coefficient-safety cache screen

## Hypothesis and source

The tile-128 synthesis kernel checks the same prepared coefficient arrays once per active latitude tile, although all tiles for one harmonic order use identical arrays. The isolated patch caches that boolean result lazily on the first tile containing ordinary lanes. It leaves recurrence arithmetic, transition checks, output staging, unsafe-coefficient fallback and polar handling unchanged.

The frozen pre-cache source hash is `0c0ee956205f782626e6ab0ffc6914cfa9f0d1e20caf5075111a53a9c9449b25`; its native `m1b_bench` binary hash is `cf46dc4ce6bf4bf389f5e90b62d22e90b21c697e1f1189dda1bb0f6e43ddb26b`. The cached-source patch is [`coefficient-safety-cache.patch`](coefficient-safety-cache.patch), applied to that source. The final isolated source hash is `da5fe000c53cc2d5a28aea4de79fc770709de1cde7d26671dec70aa1ecfc2c7e`, and the rebuilt release `m1b_bench` hash is `c4dc9af2ab5ce97b6d6e837bed961c1c4af4527c71ee5851774e674765535bc5`.

The first candidate acquisition used binary `00b47102fb6ce6a85f9b6094aa2f25f23b16ae99cc36f3125a9a7ceb2f60e777`, built from the same runtime code before adding lint-only annotations and a test type alias. It produced no complete timing series.

## Correctness and build checks

A focused test uses 257 ordinary latitude lanes, forming three tiles, and a subnormal prepared coefficient. It verifies one coefficient-safety scan for the order, the unsafe route declines on each tile, scaled fallback remains active, and the staged spectrum matches the paired scaled reference.

`cargo fmt --all -- --check` passed. `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed after adding narrow lint annotations to the isolated prototype. `cargo test --workspace --all-features` passed: 201 library tests, four extreme-FFT tests, 11 f32 tests, and one workspace-compatibility test; 14 library tests and one f32 test were ignored. This is validation of the isolated candidate source, not acceptance of the tile-128 route.

## Bounded performance screen

The predeclared screen compared the cached and frozen tile-128 binaries at q1, T2047 GL f64 scalar STANDARD synthesis, full `mmax`, `phi0=0.37`, `2048 x 4096`, one warm-up and five timed transformations per independent process. Six paired processes were planned in alternating order. Each process had to meet the 5% CV and first-three/final-two drift limits; pre/post CPU PSI had to remain at or below 2.00%, and interference review was required. Retries and sample removal were prohibited.

The first pre-cache process passed its per-process timing check: samples were `1743.589, 1746.299, 1745.218, 1749.076, 1754.631 ms`, median `1746.299 ms`, CV `0.2215%`, and drift `0.3800%`. Peak RSS was 331,828 KiB. This single process is unpaired and is not a performance comparison.

The process passed the pre/post pressure gate; a during-run reading entered the warning band but stayed below the rejection threshold. The runner was pinned to one logical CPU, and the process monitor observed it on that CPU. No competing build/benchmark process or cgroup throttling was detected. Because the runner failed during its second process, complete thread-placement review across both variants was not available.

The candidate runner failed in telemetry before producing a complete timed series: process discovery also matched a transient telemetry command, and the monitor exited while the benchmark was active. The fixed screen stopped without retrying. There is no candidate ratio, no evidence of a speedup, and no performance acceptance. No q8, T511, or broader regression measurements were launched. The tile-128 kernel remains isolated and unaccepted.

Raw host and process captures remain local and untracked. The candidate's performance effect remains unknown. The previously measured bottleneck is still the scaled prefix; scalar accumulation was the largest of its sampled internal components, while repeated coefficient-safety scans accounted for 69.235 ms in the separate T2047 route diagnostic. The next kernel change requires a valid paired whole-transform screen before further redesign.
