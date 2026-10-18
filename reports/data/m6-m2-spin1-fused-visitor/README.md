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
