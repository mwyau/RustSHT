# Tile-128 T511 q1 synthesis screen

## Result and stop condition

CPU-pressure validity was assessed under the declared protocol; raw readings remain local.

## T511 synthesis measurements

| Version | Rust samples (ms) | Rust median | CV | first3/final2 drift | DUCC median (CV) | SHTns adapter median (CV) |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Baseline | 40.061, 40.074, 39.885, 39.944, 40.426 | 40.061 ms | 0.47% | 0.31% | 7.639 ms (1.83%) | 4.635 ms (0.50%) |
| Candidate | 32.249, 32.164, 32.258, 32.163, 32.212 | 32.212 ms | 0.13% | 0.19% | 7.883 ms (1.61%) | 4.650 ms (0.49%) |

CPU-pressure validity was assessed under the declared protocol; raw readings remain local.

## Evidence

`provenance.txt`, `candidate-source.patch`, and the summaries record source, compiler/ISA, reference, workload, timing samples, pressure-gate outcomes, and RSS details. `run-one.sh`, `run-exactly.sh`, and `check-stability.py` preserve the original protocol. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Raw stdout, time, preflight, postflight, and command captures are local and untracked. This is a historical v1 result and is not reclassified under protocol v2. No production source or Git changes were made by this screen.
