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

`provenance.txt`, `build.log`, and `candidate-source.patch` record source, compiler, CPU/ISA, reference, import, and artifact details. `run-one.sh`, `run-exactly.sh`, and `check-stability.py` preserve the protocol. Each process has `.command`, `.stdout`, `.time`, `.preflight`, `.postflight`, and `.stability` files; the candidate also has a `.postflight-failed` record. No production source or Git changes were made by this screen.
