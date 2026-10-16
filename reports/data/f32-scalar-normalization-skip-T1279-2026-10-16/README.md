# T1279 scalar f32 normalization-skip screen

The paired release processes compare the fallback at T1279 with the same
normal-range skip enabled for T1023 and T1279. They use f32 scalar STANDARD
synthesis and analysis on GL `1280×2560`, full mmax, and `phi0=0.37`.

## Method and artifacts

- `paired/` and `paired-T720-control/`: initial array-search-gate screen,
  rejected after a repeatable T720 regression. Its summary is
  `summary-array-search-T1279-rejected.json`; the T720 paired values are in
  `summary-T720-control.json`.
- `paired-T1279-match/`: three alternating process pairs for the final
  exact-match selector; see `summary-T1279-match.json`.
- `paired-T720-match-control/`: three pairs confirming no disabled-path
  regression with the final selector; see `summary-T720-match-control.json`.
- Each process measures a first call and five warm samples at q1 and q8.
- `output-baseline/` and `output-candidate/`: complete scalar f32 synthesis
  and analysis arrays captured from q1 execution.
- `output-comparison.json`: byte and relative-L2 comparison of the complete
  arrays; both are bitwise identical.
- `ducc-comparison.json`: full-array comparison with DUCC 0.41.0 using source
  revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.
- `validation/`: final exact-match formatting, Clippy, all-feature, no-default,
  and opt-in T1023 DUCC logs.
- `validation-array-search-gate/`: earlier equivalent checks for the rejected
  array-search selector, including the original `/tmp`-full failure.
- `compare-output-arrays.py` and `compare-ducc.py`: reproducible comparisons.

The baseline timing executable SHA-256 is
`785db7f0e599df71f8788d93d1422eb2c17df51635e60ba087537f396831c237`; the
rejected array-search candidate is
`5f97eeb730cd1a14810b8b490f1e100ebfa4b13f77f6a29f427a43102f4f8176`; the
final exact-match candidate is
`7811fccf53f914389a689fb6b87f7ca775321fa4d9a5e7b3c6a33a4719ddb2a5`. All
timing binaries used the root release profile and runtime Pulp CPU dispatch.
The comparison capture binaries included optional post-timing output dumps and
have hashes `b28183963c5972dbd149c916d82f5b36ba9402239a6f90917fa94061d1f0721f`
for baseline and
`77e0e762803078100dd86c38e98dd3285249230b001cb2052208b099901735f1` for
candidate.

To reproduce the paired screen from the repository root:

```sh
reports/data/f32-scalar-normalization-skip-2026-10-16/run-selected-paired.sh \
  BASELINE_BINARY CANDIDATE_BINARY \
  reports/data/f32-scalar-normalization-skip-T1279-2026-10-16/paired \
  1279 2560 3
```

The runner pins q1 and q8 execution to CPUs 0–7. The rejected T720 control used
the same binaries with arguments `720 1458 3`. Reproducing the full-output
capture requires building each selector version with the optional
`RUSTSHT_F32_DUMP_DIR` output enabled in `examples/m1_f32_atmospheric_bench.rs`,
then invoking it with `0 1279 2560 1`.
