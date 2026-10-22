# Plan 16 parity-separated synthesis experiment

This is the bounded Milestone B T720 CC A/B data. `paired-batches.csv` holds
every timed batch for the production baseline and one-channel experimental
kernel; `distribution-summary.csv` has backend distributions; `candidate-baseline.csv`
contains paired source ratios; `validation.csv` retains per-process numerical,
repeat and output-reset checks; `metadata.csv` records binaries, source flags,
compiler, DUCC, affinity and CPU topology. `parity-candidates.patch` preserves
the temporary two-channel and one-channel implementations for source review.

The deterministic full/reduced fixtures were shared across source variants.
Three independent process invocations were collected for each source, with
10 calibrated batches per short call (about 25 ms each); long T720 q1 analysis
and synthesis calls were sampled individually. Source order was
baseline, one-channel, one-channel, baseline, baseline, one-channel. The
one-pair helper used Rust then DUCC inside each invocation; source variants
were interleaved, but backend order was not reversed. Per-process runqueue
wait and sampled frequency are retained with each batch.

The one-channel native binary was compiled from the source represented by
`parity-candidates.patch` using the `rustsht_spin1_parity_separated_one_channel`
cfg. Build flags and executable hashes are in `metadata.csv`. The pinned DUCC
SHT object uses `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the
benchmark driver overrides fast math. The numerical suite used the same Rust
cfg and `-C target-cpu=x86-64-v3`.

For a reproduction, apply `parity-candidates.patch` to the recorded baseline
commit, build baseline and candidate with separate `CARGO_TARGET_DIR` values,
and copy each executable to the worker path used by the native harness. The
case selector used here is the tracked
`reports/data/plan15-integrated-t720-current-2026-10-21/benchmark_plan15_integrated_t720.py`;
run it with `--skip-build --ducc-src <pinned-ducc-checkout> --build-dir
<ducc-build> --lmax 720 --grid CC --process-pairs 3 --target-ms 25`. For the
candidate build, set `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs
--cfg rustsht_spin1_parity_separated_one_channel'`; for baseline omit the final
cfg. Keep the build toolchain and DUCC flags from `metadata.csv`. This dataset
records the original timings; it is not combined with the earlier
non-interleaved two-channel q8 screen.
