# P01 default versus ordered link screen

This screen compares two retained P01 binaries without rebuilding or editing
source. The unordered candidate is `target/m6-p01-candidate-bench`
(SHA-256 `3388c0219a9d9ba214241763bd8749a35ef6e60bbe685ec779b842291ff6da2e`). The ordered candidate is `target/m6-p01-link-order-bench` (SHA-256
`22cb00437aa6cbe33222fc1a8140694c69ef8ff63694fbaa7310f50561b968f2`). Both
are identified as rustc 1.99.0 / GCC 15.2.0 / LLD 23.1.1 PIE executables. The retained candidate source patch and the ordered matrix's P01 source patch
have the same SHA-256, `fff74a3059aec58741072b3eb1a96421c006ddcc4ba6847949098d17266faecf`. Cargo release profile/features and rustflags in the ordered target fingerprint
match the release configuration. P02 is excluded. The default and ordered symbol address lists show different final text
placement. The successful ordered build's exact linker command was not
retained; an earlier `--section-ordering-file` attempt failed. Thus this
experiment compares the retained default and ordered final links, while the
exact option/file used for the successful ordering remains unverified. Each cell has six alternating fresh-process pairs on one CPU core: T511 spin-1 q1
synthesis and T1023 scalar q1 synthesis. Every process uses GL STANDARD f64,
`ntheta=L+1`, `nphi=2*(L+1)`, full `mmax=L`, `phi0=0.37`, and requested-only
output. Each process retains all five warmed synthesis samples and
`/usr/bin/time -v` peak RSS. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. The first two stability
checks are in `preflight-stability.txt`. The first T1023 pair's ordered
CPU-pressure validity was assessed under the declared protocol; raw readings remain local.
`preflight-processes.log`. Two subsequent low-pressure checks were recorded
before rerunning both sides of that pair. The original baseline side is
preserved in `superseded-after-refusal/`. Pair ratios are ordered-link process median divided by default-link process
median. The summary reports the median of six pair ratios, a deterministic
95% bootstrap interval (50,000 resamples, seed 20261017), ranges, and RSS
changes. The T511 spin-1 cell is the control; T1023 scalar synthesis is the
gain control. The results support a final-link layout effect on the spin-1
control while preserving the scalar gain. Several sample-level high values
remain in the summaries; no accepted process had a compiler or benchmark
workload in its preflight. This is a bounded link-layout screen, not P01
acceptance.
