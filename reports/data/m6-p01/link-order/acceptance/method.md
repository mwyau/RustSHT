# P01 ordered-binary acceptance matrix

This matrix compares the retained pre-P01 baseline with the already-built
ordered P01 binary. No compilation, source edit, or link-option change is part
of this measurement.

## Binaries and build evidence

- Baseline: `target/m6-p01-baseline-bench`, SHA-256
  `4fd894d46b7195d8a7a4c24cc52f3f04a85ccd1c213d5a2c896f7c828156ccd0`.
- Ordered candidate: `target/m6-p01-link-order-bench`, SHA-256
  `22cb00437aa6cbe33222fc1a8140694c69ef8ff63694fbaa7310f50561b968f2`.
- P01-only source patch SHA-256:
  `fff74a3059aec58741072b3eb1a96421c006ddcc4ba6847949098d17266faecf`.
- The candidate was built from the saved P01 source patch and excludes P02. The first `--section-ordering-file` attempt failed. The later successful
  build log omits its linker command; `symbol-order.txt` and the observed
  symbol movement are consistent with a `--symbol-ordering-file` link, but do
  not identify the exact successful option or file. Attempt logs and symbol
  addresses are in the parent `link-order/` directory.
- Toolchain: rustc 1.99.0 (`b940084d7`, LLVM 23.1.1), x86_64 Linux.
- Host: x86-64 CPU, host topology omitted. q1 is pinned to one CPU core;
  q8 is 8 pinned CPU cores.

## Workload and order

Each fresh process runs the M1b whole-transform harness in f64 with GL,
STANDARD mode, full `mmax=lmax`, `ntheta=lmax+1`, `nphi=2*(lmax+1)`,
`phi0=0.37`, `both` operations, and `requested-only` output. The six cells are
T511 scalar q1, T511 spin1 q1, T1023 scalar q1/q8, and T2047 scalar q1/q8. Each cell has six fresh-process pairs; odd pairs run baseline then ordered,
even pairs run ordered then baseline. Every process retains all five timing
samples for synthesis and analysis. `/usr/bin/time -v` records peak RSS. `run-one.sh` records a process/PSI snapshot before every process. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. A refused attempt is retained in
`preflight-processes.log`; it creates no timing result and can be retried
after two clean preflights. Completed process outputs are never overwritten.

## Statistics and gate

For each operation separately, the paired ratio is the ordered-process median
divided by the baseline-process median within each pair. The summary reports
the median of six paired ratios, their range, and a deterministic percentile
95% bootstrap interval using 50,000 resamples of the six pair ratios (seed
20261017). It also reports all raw five-sample ranges and paired peak-RSS
deltas. The predeclared control gate is a median ratio below 1.02 for every
operation/control; T1023 and T2047 scalar synthesis must also retain a
material speedup. This report records evidence only and does not determine
acceptance. Raw stdout, `/usr/bin/time` output, and per-process preflight evidence are in
this directory. `summarize.py` validates the complete matrix and writes
`summary.csv` and `process-pairs.csv` after all cells finish.
