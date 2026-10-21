# Plan 15 — CC spin-1 order-parallel resampling

## Result

At commit `a2dd56704f216728e9e878c8a851ba217d332d07`, f64 STANDARD CC spin-1
analysis parallelizes the existing meridional resampling by Fourier order
when the resampling is required and `(mmax + 1) * prepared_ntheta >= 65,536`.
It uses bounded 128-order tiles and reusable per-worker profile/FFT scratch;
each worker writes an order-major tile which is scattered back to the
existing ring-major spectrum. The q1, small-work, non-CC, and non-resampling
routes remain serial. Synthesis and the recurrence itself are unchanged.

The matched T720 q8 analysis A/B reduced Rust latency by 30–45% across full
and reduced order with both odd-minimum and even-smooth longitude counts.
T1023 q8 analysis improved 19–24%. At T255, the full-order case crosses the
work gate and improved about 36%; the reduced-order control stays below it and
changed by about 5%, within this short-call screen. The q1 aggregate controls
showed no repeatable route-level effect, and q8 synthesis paired ratios stayed
within roughly 1% of the baseline. These are comparisons of baseline source
`9f743c12fa9e20a821eb491360a44ec4bfcdd11d` to candidate `a2dd567…`; they do
not imply that an earlier source introduced a regression.

| CC analysis, q8 | Rust before | Rust after | DUCC after | Rust/DUCC before → after | Candidate/baseline | Relative L2 |
|---|---:|---:|---:|---:|---:|---:|
| T720, m180, nphi378 | 40.423 ms | 26.039 ms | 17.174 ms | 2.235× → 1.440× | 0.596× | 6.83e-14 |
| T720, m180, nphi361 | 41.310 ms | 27.803 ms | 17.708 ms | 2.405× → 1.597× | 0.703× | 6.75e-14 |
| T720, m720, nphi1458 | 122.718 ms | 66.679 ms | 35.059 ms | 3.431× → 1.902× | 0.554× | 6.10e-14 |
| T720, m720, nphi1441 | 121.508 ms | 69.324 ms | 40.012 ms | 3.051× → 1.708× | 0.575× | 6.11e-14 |
| T1023, m255, nphi512 | 73.049 ms | 59.571 ms | 22.830 ms | 3.280× → 2.644× | 0.809× | 9.18e-14 |
| T1023, m1023, nphi2048 | 189.047 ms | 146.946 ms | 57.013 ms | 3.268× → 2.602× | 0.762× | 7.51e-14 |

At T255, m255 q8 analysis moved from 9.737 to 6.121 ms (paired ratio
0.637; Rust/DUCC 2.616× to 1.735×). Its m63 control remained below the gate
and moved 3.123 to 2.967 ms (paired ratio 0.948; 1.107× to 1.089× DUCC).
All candidate A/B runner validations passed: six main geometries, 144 worker
records, 72 output comparisons, and zero failures; the separate T255 control
also had zero failures. The maximum candidate relative L2 error was
`1.354e-12` (on a synthesis control); the CC analysis cases in the table are
below `9.2e-14`. The direct serial-versus-parallel resampler test passed
repeated poisoned-workspace reuse for odd/even latitude counts, a 128-order
tile tail, subnormals and large finite cancellation inputs.

For the T1023 full-order q8 synthesis control, candidate first-call samples
were 234–240 ms with 38–43 ms reported setup; warmed batches were 229–237 ms.
The result JSON keeps these cold/setup measurements separate from warmed
latency.

## Remaining CC synthesis cost

The T1023 full-order q8 synthesis whole-call candidate result is 230.154 ms
versus DUCC 32.163 ms (6.889×, +197.992 ms). Its route is unchanged by this
analysis-only optimization. An earlier test-only profile reported about
125 ms because it created a default workspace without the production
`prepare_workspace(..., 8)` call. After correcting the profiling helper to
prepare q8 worker capacity, eight profile calls had a 235.7 ms median,
consistent with the matched native runner. The instrumented phase medians
were 2.61 ms for latitude/spectrum setup, 228.93 ms for order scheduling,
recurrence and spectral accumulation, and 4.24 ms for inverse FFT/writeback.
Thus the large Rust stage is recurrence/order execution, not the CC resampler
or inverse FFT. The internal counters are worker-time sums and overlap under
parallel execution: fused paired recurrence/accumulation was 137.1 ms,
paired coefficient preparation 25.0 ms, order factors 26.1 ms, and
scaled/per-degree fallback work 331.3 ms across workers. There were 9,487
successful fused blocks and 3,037 fallback blocks. Do not add these worker
times as wall-clock components. The same recurrence-stage pattern is already
present in GL, so this does not establish a CC-only synthesis fix.

## Conditions and reproduction

Rust baseline/candidate commits are listed above; both use Rust 1.99.1,
LLVM 23.1.1 and `-C target-cpu=x86-64-v3`. DUCC is 0.42.0 at
`42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2, with SHT sources
compiled using `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` and a
strict benchmark driver. The host was a Ryzen 9 5950X; q1 used one physical
core and q8 used eight physical cores sharing one L3 (exact CPU IDs are in
the result metadata). The main comparison used three interleaved process
pairs and 25 ms calibrated batches for short operations; long calls were
timed individually. Median q8 runqueue wait across both backends was 11.6%
in baseline and 12.9% in candidate (Rust 10.4%/9.3%, DUCC 14.2%/14.2%), so
absolute small-call differences remain host-sensitive.

Main six-case A/B reproduction (T720 full/reduced with odd/even sampling and
T1023 full/reduced even-smooth):

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  cargo build --release --example ducc_spin1_full_transform_parity --features parallel
RUSTSHT_BENCH_ROOT="$PWD" python3 "$BENCH_SCRIPT" \
  --ducc-src "$DUCC_SRC" --lmax 720 --lmax 1023 --grid CC \
  --process-pairs 3 --target-ms 25 --skip-build \
  --results-dir "$RESULT_DIR" --build-dir "$DUCC_BUILD"
```

`BENCH_SCRIPT` points to the committed subset wrapper. Run from each frozen
baseline/candidate worktree with distinct result directories and the same
DUCC build. The wrapper's `RUSTSHT_BENCH_ROOT`
selects that worktree's already-built Rust worker; the DUCC build and fixture
generation remain shared. The T255 threshold control uses the same command
with `--lmax 255` and a separate result directory. The q8 phase reproduction is the ignored
`spin_transform::tests::profile_spin1_standard_synthesis_phases` test with
`RUSTSHT_PROFILE_GRID=CC`, `RUSTSHT_PROFILE_LMAX=1023`,
`RUSTSHT_PROFILE_MMAX=1023`, `RUSTSHT_PROFILE_NTHETA=1025`,
`RUSTSHT_PROFILE_NPHI=2048`, `RUSTSHT_PROFILE_THREADS=8`,
`RUSTSHT_PROFILE_REPETITIONS=8`, Rust V3 flags, and CPU affinity 0–7. That
helper now calls `prepare_workspace` exactly as the native production worker
does.

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
RUSTSHT_PROFILE_GRID=CC RUSTSHT_PROFILE_LMAX=1023 RUSTSHT_PROFILE_MMAX=1023 \
RUSTSHT_PROFILE_NTHETA=1025 RUSTSHT_PROFILE_NPHI=2048 \
RUSTSHT_PROFILE_THREADS=8 RUSTSHT_PROFILE_REPETITIONS=8 \
taskset -c "$Q8_CPUS" cargo test --release --features parallel \
  spin_transform::tests::profile_spin1_standard_synthesis_phases \
  -- --ignored --exact --nocapture
```

Raw results, paired comparisons, first-call/setup values and batch samples
are in [`reports/data/plan15-spin1-cc-resampling-2026-10-21/`](data/plan15-spin1-cc-resampling-2026-10-21/).
The q8 synthesis phase log remains local because it contains per-worker
diagnostics; the reproducible setup and summarized measurements are recorded
here.

**Next:** CC analysis resampling is validated. The remaining CC synthesis
excess is attributed to the general spin-1 order/recurrence stage, with no
CC-specific source change justified by this profile. Proceed to F1 spin-1
preparation and phase attribution, preserving the CC measurements as controls.
