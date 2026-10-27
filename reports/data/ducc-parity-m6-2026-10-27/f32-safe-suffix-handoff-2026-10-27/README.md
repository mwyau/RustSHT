# Native-f32 safe-suffix handoff screen — 2026-10-27

## Result

The candidate stopped scalar accumulation in an all-active paired recurrence block at its first all-lane normal state, then sent that state and the remaining degree suffix through the existing native-f32 `FusedSynthesisBlock`. It retained the scaled scalar path for the prefix and the polar/mixed fallback, and restored/replayed the full scalar path if a suffix range or finite check declined. It changed no f64 recurrence arithmetic, but it changed the shared paired traversal interface.

The candidate improved the four GL T720 f32 scalar-synthesis cells in three interleaved parent/candidate/DUCC triplets each. The gain was modest, and an adjacent protected f64 cell regressed repeatably: f64 spin-1 analysis q8 was slower in all three pairs (median candidate/parent `1.0668`). The current patch is **not promoted**. It remains available as a recoverable patch while the early-stop interface is revised and the mixed polar route is profiled at lane granularity.

| Cell | Parent ms | Candidate ms | DUCC ms | Candidate/parent | Parent/DUCC | Candidate/DUCC | Candidate/parent pairs faster |
|---|---:|---:|---:|---:|---:|---:|---:|
| f32 scalar synthesis, full M720, q1 | 722.690 | 683.807 | 19.003 | 0.9453 | 38.08× | 36.00× | 3/3 |
| f32 scalar synthesis, full M720, q8 | 319.383 | 312.291 | 2.679 | 0.9778 | 119.21× | 116.56× | 3/3 |
| f32 scalar synthesis, reduced M180, q1 | 220.633 | 204.462 | 8.315 | 0.9284 | 26.53× | 24.58× | 3/3 |
| f32 scalar synthesis, reduced M180, q8 | 110.464 | 103.861 | 1.215 | 0.9427 | 90.89× | 85.65× | 3/3 |

These are triage screens, not confidence intervals. Full-mmax q8 remains over 116× DUCC; the candidate did not close the algorithmic/runtime gap. It reduced full q8 by about 2.2% and reduced q8 by about 5.7%.

## Correctness and protected controls

Across these workers, outputs were finite and repeat/reset consistent. Candidate/parent f32 scalar-synthesis relative L2 was `1.7903e-7` (full) and `1.5850e-7` (reduced); max absolute difference was `2.3842e-7` and `1.7881e-7`. Candidate/DUCC errors matched the parent result: relative L2 was `1.7234e-4` full and `2.7254e-4` reduced. No T720-specific error tolerance is established, so these measurements are comparisons rather than a new numerical acceptance threshold.

The adjacent f32 full-mmax scalar-analysis and spin-1 analysis/synthesis q1/q8 cells are included in the JSON. Parent/candidate outputs were bitwise equal for those controls. All eight f64 GL T720 scalar S/A and spin-1 S/A q1/q8 controls are also recorded. Parent/candidate f64 outputs were bitwise equal. Scalar synthesis remained within 1.2% of parent in this screen. The exception is f64 spin-1 analysis q8: candidate/parent ratios were `1.0668`, `1.0393`, and `1.1267`; all three were slower. This is a protected-cell regression for the candidate, regardless of unchanged numerical output, so the current shared early-stop visitor cannot be promoted. Per-pair runtimes and numerical comparisons are preserved in the sanitized `triad-results.json`; raw worker telemetry is not published.

Candidate validation on the isolated worktree:

- Focused paired f32 unit tests: 4 passed.
- `cargo test --locked --offline --features parallel --lib f32_`: 18 passed, 1 ignored.
- `cargo test --locked --offline --features parallel --test f32`: 11 passed, 1 ignored. These cover GL/CC/F1 analytic/reference cases, scalar and spin adjoints, Nyquist handling, high-degree finite values, and workspace reuse.
- The paired unit tests include scaled-product references and overflow decline; the transform worker checked repeat and poisoned-output reset.
- `cargo fmt --all` and `git diff --check` passed.

The candidate did not receive a dedicated prepared-suffix subnormal test because it failed the protected performance gate and was not promoted. The next candidate must prove that a subnormal/range decline restores the exact fallback before acceptance.

## Fallback route evidence

The current exact parent is RustSHT `0677b5bfe98117c1d1a5d5204b0a710828fcb9c8`; pinned DUCC is `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. A one-warm-call GL T720 full-mmax f32 q8 Callgrind run collected **8,366,310,958 Rust Ir** and **252,802,007 DUCC Ir** (about **33.1×** as many Rust instructions). The Rust thread-attributed annotation is dominated by Rayon `wait_until_cold`, so thread-local Callgrind attribution is not used as a worker split. DUCC `alm2map_kernel` self attribution is 150,669,396 Ir (59.6% of the DUCC total). These are instruction counts, not elapsed time.

Temporary route counters on the candidate full-mmax q8 transform separate the paths:

- 8,652 paired blocks: 1,937 inactive; 6,715 active partition checks; 1,216 blocks declined the no-polar-lane adapter and used the mixed polar path.
- The 1,216 mixed blocks compacted 40,170 active latitude lanes into one contiguous active span per block. Every active seed lane was scaled. The all-lane range status never became normal during the mixed recurrence: 188,465 callbacks and 6,293,674 pair/lane visits remained on fallback; 1,020,840 exponent-zero and 5,272,834 nonzero-exponent visits generated 12,587,348 scaled helper calls. **Per-lane safe state and monotone contiguous safe spans inside these mixed blocks were not measured**, so this does not rule out a profitable contiguous-span route.
- The prepared all-active path staged, seeded, and committed 347,095 lanes. Of 3,755 scaled-seed fallback blocks, 3,205 later reached an all-lane normal state with no observed re-entry. The candidate attempted 3,205 fused suffixes: 3,158 succeeded and 47 declined the range check. Those 47 restored staged sums and replayed 862,036 pair/lane visits; no decline was due to a nonfinite fused sum. The candidate fused 31,379,485 suffix pair/lane visits. Its remaining prepared scalar fallback made 8,049,835 scaled-helper calls.
- The run also counted 122,339 inactive-scan predicates, 367,793 partition predicates, 20,698 mixed-detection predicates, 75,474 compact-scan lanes, and 188,465 coefficient-pair builds. Staging and final commit each traversed 347,095 lanes. These are operation counts, not timing shares.

The route counters are in `route-counters.json`; output and fixture hashes, per-pair timings, numerical differences, and repeat/reset checks are in the sanitized `triad-results.json`. Raw host and worker telemetry is excluded from the committed summary.

## q8 schedule and DUCC/assembly comparison

The baseline timing-only group profile used 12 latitude groups on q8 with `min_groups_per_task=2`. Only four Rayon worker IDs received the groups, three consecutive groups each. Their summed observed group durations were approximately 320 ms, 44 ms, 90 ms, and 261 ms. The slowest worker accumulated over seven times the group time of the lightest worker. These group timers include scheduling and overlap effects; they are not compute-only costs or total-transform time. Still, they show a strong q8 utilization/load-balance lead. Raw group records are in `group-timings.csv`.

Pinned DUCC 0.42.0's `calc_alm2map`/`iter_to_ieee` advances the scaled paired recurrence through its prefix, then hands a normal suffix to a vector `alm2map_kernel`; its order loop schedules work dynamically by `m` and compacts eligible latitudes into batches. The f32 DUCC adapter promotes the numerical transform to f64. Its AVX2 kernel uses four double lanes and 12 vector FMAs in the four-pair loop, with unit-stride memory operations and no hot-loop stack spills in the inspected assembly. The raw assembly capture is not published.

RustSHT uses native f32 and eight AVX2 lanes. Its existing V3 fused loop has the recurrence and two-channel accumulation FMAs, plus repeated vector range comparisons and branches. At the outer level, Rust partitions by latitude groups; mixed polar blocks are compacted but still accumulate each scaled contribution through the callback. The existing f32 kernel from the exact parent was disassembled, not the new candidate-specific clone; the candidate did not receive a fresh instruction profile or disassembly. The assembly observations above describe the parent hot loop and are not proof of candidate codegen. The raw capture is not published.

## Provenance and next action

The candidate patch is `candidate-production.patch` (SHA-256 `c05e0e349169804e1a9f1aae01b6538e8f2b11538f0f0240f09f9c254a2cfd61`); it changes the shared paired traversal API, extends the existing fused block, and adds one state-rotation unit test. The full temporary instrumentation patch is not part of it. Build flags were Rust release `-C target-cpu=x86-64-v3 -Aunexpected_cfgs` with runtime Pulp dispatch; DUCC numerical sources used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, with `-fno-fast-math` on the benchmark driver. The sanitized `triad-results.json` records source, DUCC, fixture, output, and benchmark-binary hashes plus compact numerical results. `artifact-checksums.json` lists hashes for published report artifacts. Raw Callgrind and assembly captures were analyzed locally; their instruction totals, route counts, and codegen findings are summarized above but the raw files are not published.

Next, profile lane-level pair-state safety and monotone contiguous spans inside the 1,216 mixed blocks, while measuring q8 task distribution on the exact baseline. Rework the early-stop traversal so ordinary shared f64 visitors do not pay the new stop-control cost. Only implement a mixed safe-span path if the measured contiguous-run lengths and remaining pair work justify its staging; preserve all exceptional/polar lanes on the scaled fallback. The M6 performance goal remains open.
