# M1 T2047 q8 scheduling-granularity screen (partial)

## Question and candidate

The M1 q8 T2047 route profile showed an active-worker timer ratio of 9.53,
with eight contiguous 256-ring groups for eight Rayon workers. This isolated
candidate changes only that exact GL/full-mmax scalar scheduling path to
sixteen independent 128-ring groups, allocates one recurrence scratch entry
per group, and sets Rayon minimum grain to one. The recurrence, floating-point
operation order, route selection and safety guards are unchanged. The intent
was to make another group available for stealing when a worker takes the
scaled visitor fallback. This is a scheduling hypothesis, not a measured
explanation of the imbalance. The source-matched baseline is commit `197c3d3b9536d8f308d6dc0fae13c806e3d92970`
(tree `aa09a0b93fb1e201a2a9e8c914d942312110733f`, C02+C03 production, P01/P02
excluded). The isolated candidate patch is
[`candidate-source.patch`](candidate-source.patch), SHA-256
`8c1a2e6f7a56fb2fe60f511216f229b4abcc220b03aca4a9f1c62288870f01d2`; its
only source paths are `src/plan.rs` and `src/transform.rs`. No shared
production source was edited.

## Build and numerical provenance

Both isolated Python extension builds used `cargo build --release --locked -p
rustsht-python`, Rust/Cargo 1.99.0, and requested one CPU core pinned CPU cores, one build
at a time. Baseline extension SHA-256 is
`69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`; candidate
extension SHA-256 is
`bffc5de5101c9689694e2cfcf316c51931f890f2863b46990a21fa57afa4620c`. The
build logs, `/usr/bin/time` records, source copies and host/compiler snapshot
are retained beside this summary. The harness SHA-256 is
`72cd1986a220573c1a445bd6a6aa96b9df272a6bff1dc833843ee3f274c77739`. Focused output checks used scalar GL STANDARD f64, T2047, 2048x4096, full
mmax and `phi0=0.37`, against the pinned DUCC 0.42.0 build at source revision
`80388904b8cfc32400ba84af8a6a45aabfe60f86`. Baseline and candidate maps were
bitwise equal (SHA-256
`8433a17c70adf2ca7909d82233f8347189a09034d988859e318099b010bdcda3`). Both
had relative L2 error `3.01496781410830626e-13` versus DUCC, with maximum
absolute error `1.11038955807885031e-12`. The numerical comparison results and output hashes are summarized in `map-comparison.txt` and `map-sha256.txt`; raw build and test output remains local.

## Timed partial screen

The predeclared screen is eight alternating fresh-process pairs of T2047
scalar synthesis, GL STANDARD f64, 2048x4096, full mmax, `phi0=0.37`, eight
Rayon threads, requested 8 pinned CPU cores. Each process runs the retained
`python_perf_baseline.py 2047 0 8 GL 1 into synthesis both STANDARD 4096 2048`
harness command (`backend=both`, `scope=into`) and records five timed Rust
samples. The paired estimate uses the median of those five samples per
process, then candidate/baseline within each pair. Structured timing samples, paired results and aggregate RSS are retained. Raw process and host captures remain local.

| Pair | Order | Baseline median (ms) | Candidate median (ms) | Candidate/baseline | Peak RSS baseline/candidate (MiB) |
|---:|---|---:|---:|---:|---:|
| 1 | baseline, candidate | 804.035 | 669.505 | 0.83268 | 614.3 / 615.7 |
| 2 (retry) | candidate, baseline | 693.819 | 804.081 | 1.15892 | 616.9 / 615.2 |
| 3 | baseline, candidate | 801.265 | 677.556 | 0.84561 | 615.4 / 616.5 |
| 4 (retry) | candidate, baseline | 677.730 | 802.293 | 1.18379 | 616.2 / 614.4 |

Across these four complete pairs, the exploratory median ratio is `1.00226`
(range `0.83268–1.18379`). A deterministic percentile bootstrap over paired
ratios (100,000 resamples, seed 2047) gives `0.83268–1.18379` at 95%. With
only four pairs this interval is descriptive. The raw medians have a strong
run-order pattern: the second process is faster in each pair, whichever binary
it is. Therefore this partial sample does not identify a scheduler effect. Peak RSS changes are within about 2 MiB and show no material memory increase. All eight completed-process preflights were permitted and showed no matching
build/benchmark process or heavy process. CPU-pressure validity was reviewed under the declared protocol; exact transient readings remain local. Pair 2's original candidate-only process remains
preserved as an unmatched orphan and is excluded; the complete pair 2 retry
uses fresh candidate and baseline processes. Pair 4's initial candidate
preflight was refused, then the complete pair 4 was rerun in alternating
order after clean checks. The CPU-pressure check failed the declared gate; exact transient readings remain local. Pairs 6–8 were not run. The fixed refusal rule remains
`some avg10<=0.5`, `some avg60<=1.0`, `full avg10<=0.05`, with no matching
build/benchmark or process using at least 15% CPU. The eight-pair screen is incomplete and the exploratory point estimate does
not meet the predeclared primary requirement (median ratio at most 0.98 and
paired-bootstrap 95% upper bound below 1.00). No controls or route-profile
rerun were started. This candidate is not accepted. A quieter window could
technically finish the same binary/settings screen, but the consistent
second-process advantage across the retained pairs is a run-order concern;
those four pairs should not be presented as evidence of a scheduling gain. Any continuation or restart should retain the same fixed preflight gate and
all prior raw records.
