# Spin-1 fused Q/U visitor screen

## Status

The six-pair preliminary screen passed its declared primary and q1 control
criteria. The extended synthesis matrix is incomplete and inconclusive. The
candidate is **unaccepted**; no production source was changed. The q8 scalar T511 control produced severe within-process timing
nonstationarity even though its fixed preflight PSI checks passed. The
bootstrap interval alone is not a usable pass. The next spin-1 q8 cell has
CPU-pressure validity was assessed under the declared protocol; raw readings remain local.
fixed `0.50` limit. The matrix was stopped without relaxing that threshold. All valid samples, refusals, retries, and orphan process outputs are retained;
no timing sample was discarded.

## Candidate and build provenance

The isolated candidate starts from c0e11ad plus the source snapshot patch
listed in `provenance.txt`; P01 and P02 are excluded. Its only source changes
are in `src/legendre/mod.rs`, `src/legendre/scalar.rs`, and
`src/spin_transform.rs`. It fuses the Q and U ordinary visitor prologue where
they share recurrence basis and inverse-sine work. Per-channel coefficient
and exponent handling, multiply order, safety checks, and the existing scaled
fallback remain separate. The change is not a SIMD kernel. `candidate.patch` preserves the exact source delta. `candidate-source.tar.gz`
archives the complete isolated source tree except its generated `reports/`
directory, compiled extension, and caches. SHA-256 files accompany the patch
and archive. The candidate source tree hash is
`a423b2794a712f8a84efb3f08aaa43d48fbfbe3424c62c455eb8ffe732d16474`; the
candidate extension hash is
`1e29d9c2f0a4901fe81dd676a6ea6548261963e0ff404812cf6d79bf3e37974c`. The
baseline source tree and extension hashes, Rust/compiler details, runner
hashes, and exact inputs are in `provenance.txt`,
`source-and-binary-sha256.txt`, and `runner-and-harness-sha256.txt`. The host was an x86-64 CPU; Rust was 1.99.0 (b940084d7, LLVM 23.1.1). References use DUCC 0.42.0 at revision
`80388904b8cfc32400ba84af8a6a45aabfe60f86`.

## Correctness checks

The direct paired-helper checks and spin-1 GL/CC/F1 analytic and DUCC tests
passed. At T2047 q8 and T511 q1, candidate output maps were bitwise identical
to the same-source baseline. Relative L2 errors against DUCC were
`3.707345e-13` and `6.699090e-14`, respectively. Raw test output and map
comparisons are retained in `test-*.log` and `map-equivalence.log`.

## Preliminary paired screen

The primary was T2047 spin-1 q8 synthesis: six alternating fresh-process
pairs, candidate/baseline median `0.948295`, paired bootstrap 95% interval
`[0.945511, 0.960977]`, range `[0.944022, 0.963544]`. This met the declared
primary screen (`median <= 0.98`, interval upper `< 1.00`). The T511 spin-1 q1
control median was `1.006177`, interval `[1.002531, 1.018994]`, meeting its
`upper <= 1.02` screen. Full per-pair values and sample logs are in
`initial-screen-summary.md`, `initial-screen-pairs.csv`, and the paired raw
files.

## Extended matrix

The frozen matrix targeted twelve alternating process pairs per synthesis
cell across scalar/spin-1, T511/T1023/T2047, and q1/q8. Only three cells were
completed:

| Cell | Median candidate/baseline | Paired 95% interval | Status |
|---|---:|---:|---|
| Scalar T511 q1 | 1.002761 | [0.996376, 1.005369] | Pass |
| Spin-1 T511 q1 | 0.999635 | [0.992825, 1.002275] | Pass |
| Scalar T511 q8 | 0.891984 | [0.776221, 0.998472] | Inconclusive: severe timing nonstationarity |

For scalar T511 q8, paired ratios ranged from `0.628262` to `1.488840`; the
median within-process sample CV was 40.1%, and measured samples consistently
fell during a process. No valid sample was removed. See
`matrix/spin0-t511-q8-audit.md` and its linked CSVs for every sample, timing,
context-switch count, PSI snapshot, and process record. The spin-1 T511 q8
cell was not completed; two pairs are retained and a subsequent preflight
CPU-pressure validity was assessed under the declared protocol; raw readings remain local.
artifacts are under `matrix/`. Because one control cell is inconclusive and the remaining matrix was not
run, these data do not establish the candidate's full performance gate.

### Version split of the T511 q8 timing audit

The selected 12 pairs contain 12 baseline and 12 candidate process traces. Using the audit's within-process CV definition (sample standard deviation
normalized by that process median), baseline median/mean CV was
37.42%/41.24%; candidate was 43.08%/49.41%. Across processes, the median of
the first-three-call medians was 18.964 ms for baseline and 20.965 ms for
candidate; the median of the final-two-call medians was 9.500 ms and 9.555 ms,
respectively. Both versions therefore fell to nearly the same final-call
range, while candidate traces had somewhat greater dispersion. Pair order was
balanced: six baseline-then-candidate and six candidate-then-baseline pairs,
so each version ran first six times and second six times. This shows the early
decline in both versions but does not identify its cause or change the
inconclusive gate. All selected raw samples remain preserved and none was
discarded. The version-split calculation uses the process records in
`matrix/spin0-t511-q8-audit-processes.csv` and their selected raw outputs.

### Exact-harness baseline stability validation

Four fresh baseline-only processes ran in q8, q1, q1, q8 order with the
archived `python_perf_baseline.py` harness unchanged (`backend=both`, scope
`into`, synthesis, STANDARD, `runs=1`). The extension hash was
`a8641456e1fcdf8d37ce3dd583dd1b7cf224306ca792d38c2b75f3bfcac63847`; the
archived harness hash was
`72cd1986a220573c1a445bd6a6aa96b9df272a6bff1dc833843ee3f274c77739`. The
process configuration was GL scalar f64 T511, 512x1024, full mmax, phi0=.37. Each process performed the existing Rust-vs-DUCC synthesis correctness call,
then one untimed warm-up and five timed Rust synthesis calls.

| Process | Five timed Rust calls (ms) | CV | First 3 median | Final 2 median |
| --- | --- | ---: | ---: | ---: |
| q8-01 | 24.938, 20.991, 14.161, 9.736, 9.289 | 48.96% | 20.991 | 9.512 |
| q1-02 | 39.646, 39.516, 39.475, 39.645, 39.560 | .19% | 39.516 | 39.603 |
| q1-03 | 39.787, 39.963, 39.534, 39.745, 39.827 | .39% | 39.787 | 39.786 |
| q8-04 | 21.038, 15.110, 17.596, 9.379, 9.433 | 33.84% | 17.596 | 9.406 |

Each preflight confirmed the requested affinity and listed no competing
build or benchmark process. All preflight PSI values met the .50 limit. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. `/usr/bin/time -v` reports context-switch
totals for the entire process. The unchanged harness does not expose
per-call involuntary context-switch deltas. Preflight CPU frequency snapshots
and the full raw logs are in `t511-q8-instability-diagnostic/`. A
contemporaneous root-side cgroup check reported unlimited `cpu.max` and zero
throttling. Thermal measurements were unavailable. The cause of the timing
drift remains unresolved. Under the prospective rule, both exact-harness q1 processes pass the CV and
drift checks: their first-three/final-two drift is 0.22% and 0.003%. Both q8
processes fail both checks: drift is 81.1% and 54.2%, alongside CVs of 48.96%
and 33.84%. The baseline q8 process drift appeared in both fresh q8 runs,
while both q1 controls were stable. Therefore the visitor candidate remains **unaccepted
and deferred** because the q8 host-stability control prevents its gate from
being evaluated; this is not a performance rejection of the candidate. Do
not run its acceptance matrix under the current measurement method. For any future acceptance sampling, use this prospective stability rule with
the exact harness sequence above: keep the correctness call and one existing
untimed warm-up, then retain all five timed Rust calls. Require both
`sample_sd(samples) / median(samples) <= 5%` and
`abs(median(first3) - median(final2)) / median(all5) <= 5%`. Keep preflight
PSI `some avg10 <= .50` and require no competing build or benchmark. If any
process fails either stability condition, retain it and stop the cell or
matrix as inconclusive; do not delete samples or retry until a process
passes. This proposed rule is not applied retroactively. The clean standalone
trace had q8/q1 CVs of 2.37%/.50%, and the clean exact-harness q1 controls had
.19%/.39%; the exact-harness q8 controls failed with 48.96%/33.84%. The
earlier standalone trace and every historical timing sample remain unchanged.
