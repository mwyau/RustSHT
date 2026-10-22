# Fixed SIMD workspace field-order comparison

This is Step 2 of the cumulative scalar-analysis ablation. The experimental
`rustsht_scalar_analysis_hot_scratch_order` cfg moves the fixed `next`
recurrence array after the four spectrum arrays in `RecurrenceWorkspace` and
the isolated kernel fixture. It does not change capacity, alignment, total
workspace bytes, allocation count, or array initialization. The packed complex
reducer cfg was enabled in both A/B builds.

## Reproducibility

- Rust production source: commit `126fb2cbc2c858e87c5d22b68893b56a7cc0f0a6`;
  current branch commit `8b701cd83407123dff757715c68108dfa3923b9c` adds only
  instructions/plan documentation on top of that source.
- Rust 1.99.1, LLVM 23.1.1, Pulp 0.22.3, runtime `Arch::new()` selected V3
  (AVX2/FMA), `-C target-cpu=x86-64-v3`.
- DUCC `80388904b8cfc32400ba84af8a6a45aabfe60f86`, GCC 15.2, V3, DUCC source
  `-O3 -ffast-math -ffp-contract=fast`; driver compiled with `-fno-fast-math`.
- Host: AMD Ryzen 9 5950X; q1 pinned to one physical core, q8 to eight allowed
  CPUs on one L3 cache domain.
- Isolated kernel: ten interleaved process pairs, six calibrated 20–100 ms
  batches per process/backend. Full transform: three process pairs and ten
  calibrated approximately 25 ms batches per process/backend for T720 GL
  even-smooth analysis, `mmax=1,180,720`, q1/q8.
- Candidate and baseline target directories were separated. Baseline flags:
  `-C target-cpu=x86-64-v3 --cfg rustsht_packed_complex_pair_reduction`.
  Candidate adds `--cfg rustsht_scalar_analysis_hot_scratch_order`.

Raw process-level results and host logs remain local and untracked. The initial
full-transform pair is excluded: baseline q8 processes saw 11–12% runqueue
wait while candidate processes saw less than 1%. The reverse-order repeat had
similar median host telemetry, but q8 p90 runqueue wait was about 15% for both
builds, so small q8 differences remain inconclusive.

## Isolated kernel, q1

Values are medians across process medians in microseconds; parenthesized ranges
are p10–p90 of those process medians.

| Active lanes | Rust baseline | Rust reordered | DUCC baseline build | DUCC reordered build | Rust/DUCC baseline → candidate |
|---:|---:|---:|---:|---:|---:|
| 64 | 4.615 (4.597–4.667) | 4.621 (4.606–4.650) | 4.279 | 4.262 | 1.080× → 1.083× |
| 128 | 8.630 (8.582–8.688) | 8.592 (8.557–8.811) | 8.240 | 8.207 | 1.048× → 1.050× |
| 256 | 16.813 (16.741–16.898) | 16.824 (16.758–17.075) | 16.150 | 16.067 | 1.041× → 1.047× |

Timing ranges overlap and the differences are at most 0.44%. In the optimized
V3 kernel binaries, the `FusedScalarAnalysisPair<true,false,false,true,true>`
symbol remained 0x1375 bytes. The main recurrence-loop instruction-byte hash
was identical (`f51e9f07e3f04e40424c5a52d23b5971dc1ef7f3d647712f7934af2e6157407b`).
The field reorder did not alter the hot-loop instruction stream.

## Full-transform repeat

Warm Rust and paired Rust/DUCC medians, with Rust p10–p90 in parentheses:

| mmax | Threads | Rust baseline ms | Rust reordered ms | DUCC baseline / reordered ms | Rust/DUCC baseline → candidate |
|---:|---:|---:|---:|---:|---:|
| 1 | 1 | 0.0900 (0.0896–0.0909) | 0.0901 (0.0898–0.0914) | 0.1337 / 0.1350 | 0.673× → 0.667× |
| 1 | 8 | 0.0765 (0.0748–0.0828) | 0.0767 (0.0744–0.0783) | 0.1325 / 0.1301 | 0.578× → 0.590× |
| 180 | 1 | 6.160 (6.134–6.179) | 6.140 (6.072–6.207) | 5.876 / 5.911 | 1.049× → 1.039× |
| 180 | 8 | 0.919 (0.905–0.952) | 0.956 (0.909–1.101) | 0.925 / 0.926 | 0.993× → 1.032× |
| 720 | 1 | 15.323 (15.262–15.539) | 15.519 (15.297–15.941) | 14.429 / 14.547 | 1.062× → 1.067× |
| 720 | 8 | 2.336 (2.232–2.560) | 2.453 (2.200–2.599) | 2.123 / 2.183 | 1.101× → 1.124× |

The distributions overlap, especially for q8. The 0.1–5.0% candidate changes
do not support a workspace-layout speedup; this is a neutral candidate, not a
production improvement.

## Correctness and commands

Both repeated result sets cover all six cases, have empty failure lists, and
report `reset_consistent=true` for all 36 process samples. Zero, subnormal and
extreme-finite outputs stayed finite. Basis and roundtrip validation ran for
each case. Maximum Rust/DUCC relative L2 across case outputs was `6.12e-14`.

The full-transform runs used `scripts/benchmark_ducc_full_transform_parity.py`
with `--ducc-revision 80388904b8cfc32400ba84af8a6a45aabfe60f86 --lmax 720
--operation analysis --longitude-mode even_smooth --threads 1 --threads 8`;
the three mmax cases were `1`, `180`, and full (`720`). Use the baseline flags
above for one result directory, then build in a separate target directory
with the layout cfg added for the candidate. The calibrated isolated-kernel
comparison used `scripts/benchmark_scalar_analysis_kernel.py` and the 64,
128, and 256 active-lane fixtures.

Validation with both candidate cfgs:

- `cargo test --release --lib -- --test-threads=1`: 250 passed, 34 ignored.
- `cargo test --release --no-default-features --lib -- --test-threads=1`:
  202 passed, 17 ignored.
- `cargo clippy --release --all-targets --all-features -- -D warnings` and
  `cargo fmt --check`: passed.
