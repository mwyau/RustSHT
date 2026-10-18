# T511 q8 timing instability diagnostic

This is diagnostic-only data, not acceptance evidence. Existing raw samples remain unchanged. The run used the retained baseline extension only; no source or production binary was built.

## Timing sequence and setup

The archived `examples/python_perf_baseline.py` harness performs one untimed warm-up call in `time_runs`, then five individually timed calls (`runs=1`). In the `backend=both` path, the Rust-vs-DUCC synthesis comparison runs before that timed group and includes an additional Rust synthesis and a DUCC synthesis. The diagnostic trace used the same Rust extension, deterministic ALM, GL grid and synthesis call, with one Rust correctness-call equivalent, one untimed warm-up, then 20 individually recorded calls. It did not run the intervening DUCC correctness call, so it is a focused baseline trace rather than a bit-for-bit reproduction of the complete harness sequence.


## Results

| Measured calls | Median | Mean | CV | Range |
| ---: | ---: | ---: | ---: | ---: |
| 20 | 9.486 ms | 9.436 ms | 2.37% | 9.041–9.869 ms |
| one pinned CPU core | 39.834 ms | 39.878 ms | 0.50% | 39.599–40.154 ms |

CPU-pressure validity was assessed under the declared protocol; raw readings remain local.


## Interpretation and limits

The old q8 within-process decline is not reproduced in this short low-PSI trace. One correctness call plus the harness's warm-up cannot alone explain the historical behavior under these conditions. The trace does not identify the cause: it is one q8 and one q1 process, omits the harness's intervening DUCC correctness call, and cannot exclude transient host or frequency effects. No revised acceptance or warm-up protocol is defined here; the evidence is being handed back for review before that decision. The numeric timing sequence is retained in the adjacent structured sample file. Per-call raw stdout, GNU-time, affinity and host-state captures remain local and untracked. `call-trace.py` is the diagnostic-only recorder.
