# T2047 f32 normalization-skip screen

The experiment compared the exact-match selector at commit
`2f36a9d540a0ede6ea93d0e7a10d3c71f5e7a968` with the isolated selector change
in [`candidate-selector.patch`](candidate-selector.patch). Both binaries
included the same benchmark-only sample-count option; the candidate additionally
enabled the existing normal-range normalization skip at exactly T2047. No
other runtime code changed.

Apply the candidate selector from the repository root with:

```sh
git apply --unidiff-zero reports/data/f32-scalar-normalization-skip-T2047-2026-10-16/candidate-selector.patch
```

The grid was GL `2048×4096`, full `mmax=2047`, `phi0=0.37`, and f32 STANDARD
transforms. Each benchmark process reused one plan and workspace and measured
one first call plus two warm samples at q1 and q8 for synthesis and analysis.
Processes were pinned to CPUs 0–7. The host was an AMD Ryzen 9 5950X; Rust was
`1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`. Binaries used the release
profile, the `parallel` feature, runtime Pulp dispatch, and no global
`target-cpu=native` flag.

The binary hashes were:

- Baseline: `e93fc28c08e84ee72cc7c7ff03128bff514de47d70a78603c151a04cd1888796`
- Candidate: `70fa90a93d2903707037f7521acdcfef7de5cd19757cfc0d76093411ad98ef51`

One process pair per spin completed in 18.4 minutes. Two more pairs at that
rate would have required about 37 additional minutes. The runner was
configured for three pairs but stopped after the first pair showed mixed
changes. A second pair started and stopped after 63.3 seconds; its interrupted
`time` output is retained. The first candidate scalar process coincided
with a brief RustFFT benchmark whose CPU placement was not captured at that
occurrence, so its timings are diagnostic. Later brief RustFFT runs were
observed on CPU 30, outside the RustSHT affinity. The CPU placement of the
other brief runs was not captured.

Raw process logs and `/usr/bin/time` outputs are in [`paired/`](paired/).
[`summarize-paired.py`](summarize-paired.py) recalculates the arithmetic median
from each pair of warm samples and writes
[`paired-summary.json`](paired-summary.json). The `*_median_ms` fields in the
original logs use the upper sample for an even count; the summary script uses
the conventional average of the two middle samples. The benchmark harness now
reports that median and accepts `RUSTSHT_M1_SAMPLES`; its default is five warm
samples.

The selector changes only the scalar f32 recurrence. Spin-1 STANDARD uses a
separate recurrence that is unchanged by this experiment, so spin-1 rows in
the paired summary are unaffected-path control variation, not selector
effects. The scalar numbers are one-pair diagnostic observations; the first
candidate scalar process overlapped an unrelated RustFFT process whose CPU
placement was not captured. The run did not capture complete outputs or
compare T2047 f32 outputs with DUCC. It establishes no T2047 f32 accuracy
threshold. The production selector uses the skip at T1023 and T1279; T2047
uses the established normalizer.

The paired runner accepts baseline and candidate binaries, output directory,
longitude count, pair count, and warm sample count:

```sh
reports/data/f32-scalar-normalization-skip-T2047-2026-10-16/run-paired.sh \
  BASELINE_BINARY CANDIDATE_BINARY OUTPUT_DIRECTORY 4096 3 2
```

## Validation after removing the T2047 selector

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `TMPDIR=target/tmp cargo test --workspace --all-features` passed: 188 unit
  tests and 11 f32 tests; 13 unit tests and one opt-in f32 test were ignored.
- `TMPDIR=target/tmp cargo test -p rustsht --no-default-features` passed: 150
  unit tests and 11 f32 tests; 7 unit tests and one opt-in f32 test were
  ignored.
- The following sample-count smoke run completed and reported two warm
  samples with the arithmetic median of those samples:

  ```sh
  RUSTSHT_M1_SAMPLES=2 cargo run --release --example m1_f32_atmospheric_bench \
    --features parallel -- 0 42 87 1
  ```

Python tests were not run because the bindings did not change. The T2047
candidate selector itself was not tested after its performance screen; it was
removed before the validation commands above.
