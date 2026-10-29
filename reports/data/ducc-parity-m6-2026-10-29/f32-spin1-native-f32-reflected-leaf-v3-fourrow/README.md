# Fused four-row scaled packet for reflected f32 spin synthesis — rejected

V3 fused the four scaled coefficient rows in the reflected spin-1 leaf, sharing the cosine and basis loads, SIMD range checks, and packet setup. It staged all eight output rows and committed them only after all accumulations were finite. The existing exact scalar path still handled partial tails, and unsafe full packets declined transactionally.

## Provenance and method

- Exact source parent: `def4bbee4071f8e77e0387c7f88424431430d4fc`.
- Complete candidate patch SHA256: `e19044a3a932f493e87086ba65848268b17e6ea0a7b7eb167e653150e057f5c6`; `git apply --check` passed against the parent. It contains V1, V2, and V3 changes, with no production code promoted.
- Pinned DUCC: 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
- Rust: rustc 1.99.1 / LLVM 23.1.1, release `parallel`, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; C++: g++ 15.2.0, SHT sources `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`, driver `-fno-fast-math`.
- Host: AMD Ryzen 9 5950X x86_64, Pulp V3, eight f32 lanes. q1 used one pinned logical CPU; q8 used eight pinned distinct physical cores. CPU identifiers are omitted from the committed record.
- Three interleaved parent/candidate/native-f32 DUCC pairs per cell, 25 ms warm target, setup-amortized plans/workspaces, complete public f32 calls. Fixtures used `benchmark_ducc_f32_native.py::make_fixture`, phi0=0.23; hashes and pair telemetry are preserved in `fixture-hashes.json` and compressed `results.json.gz`.
- Binary hashes: parent `8e462e4f...`, candidate `78ceab2e...`, DUCC `fcfbdf59...`; complete values are in `binary-hashes.txt`.

## Whole-call screen

Times are medians in milliseconds. Candidate/parent and candidate/DUCC ratios are paired medians; ranges are observed three-pair min/max.

| GL T720 spin-1 synthesis | Parent / candidate / DUCC ms | Candidate/parent [range] | Candidate/DUCC [range] | Max rel-L2 vs DUCC | Status |
|---|---:|---:|---:|---:|---|
| Full M720 q1 | 1501.926 / 1515.830 / 40.846 | 1.009 [0.994, 1.014] | 37.224 [35.977, 37.415] | 2.136e-4 | GAP; no material gain |
| Reduced M180 q1 | 607.184 / 580.345 / 17.668 | 0.957 [0.955, 0.958] | 32.898 [32.844, 33.022] | 1.949e-4 | GAP; small gain |
| Full M720 q8 | 273.307 / 545.595 / 5.905 | 1.996 [1.985, 2.023] | 92.398 [71.762, 92.878] | 2.140e-4 | GAP; rejected |
| Reduced M180 q8 | 122.073 / 315.508 / 2.935 | 2.585 [2.569, 3.045] | 107.492 [86.268, 114.215] | 1.958e-4 | GAP; rejected |

All workers were finite, repeat-consistent, and consistent after poisoned-output reset. Candidate relative-L2 error remained worse than parent in all four cells (`1.949e-4–2.140e-4` versus `1.415e-4–1.426e-4`). The q8 regressions are decisive after three pairs, so V3 was not extended to 15 pairs or promoted. Candidate-vs-DUCC error is not an independent correctness reference.

## Route and decision

Test-only counters confirm the fused scaled-packet operation ran on 425,242/524,402 full-mmax q1/q8 packets and 62,185/146,254 reduced q1/q8 packets; every packet succeeded, with no leaf or packet declines. Exact scalar comparison, overflow transactionality, and unsafe-packet decline tests passed. The focused route, formatting, and all timed worker reset checks passed.

V1 Callgrind identified scalar scaling as a large cost; V2 and V3 attempted row-wise then fused packet handling. Despite all measured packets taking the new fused path, q8 remained about twice as slow as parent, and V3 did not improve q8 versus V2 in this separate three-pair screen. No V3 Callgrind was run, so the remaining q8 mechanism is unresolved. Full candidate patch, per-pair results, and all route counts are preserved here; prior profiles remain in the [V1 report](../f32-spin1-native-f32-reflected-leaf-v1/README.md). M6 remains open.

## Next experiment

Stop native-f32 reflected-leaf variants. Use the already measured reusable f64-internal spin-1 routes as the next bounded candidate. Start from the current production f32 plan/workspace and existing f64 plan/workspace ownership; include every f32/f64 conversion and writeback, retain guarded native-f32 fallback, and compare all four T720 synthesis cells to native-f32 DUCC. The measured Stage B route is near parity but misses q8 and reduced q1; profile transform-stage cost rather than repeating conversion micro-optimizations.
