# M6 triage evidence provenance

Latest accepted source slice: [shared f32/f64 scalar synthesis recurrence](f32-fused-synthesis-2026-10-27/README.md), RustSHT `a30537bd`. The focused q8 cells remain 140.90×/117.24× DUCC at full/reduced mmax; this is not parity or a complete matrix refresh.

The current-source GL T720 q8 f32 scalar synthesis instruction profile is preserved in [the focused Callgrind evidence](f32-callgrind-q8-2026-10-27/README.md). It compares exact parent `972bacda` and pinned DUCC; Ir reductions are attribution evidence, not timing claims.

This directory preserves the bounded selection patch for the checked benchmark workers. Raw Callgrind captures from other campaigns remain local and untracked; compact committed summaries and reports preserve their measurements. The f64 selector derives from `scripts/benchmark_ducc_spin1_native.py` (original SHA-256 `e8364e3a18996bac98daa68cba348ccb0d6803576d26c167f63ce79d111ecabe`); the f32 selector derives from `scripts/benchmark_ducc_f32_native.py` (original SHA-256 `70033f7251358128783f35d7aefa527fa957abce5513571db1142e92e6e38ff0`). The zero-context patch SHA-256 is `63797a6c9566cdfae6d07284fd1401f001fbf177b6c66ccc9ed341ead64e563d`. The resulting bounded copies had SHA-256 values `a80b1c3a43a788978011d3fca19229d35247769c39d3ab8b5b91dcd011b921fd` and `98cfebe740a04d0506f01b9bde2a466e2f575594a14c0a98e0f31f6763702cd4`.

The patch was tested by copying the checked scripts into a clean directory and applying it with `patch -p1`; both resulting files matched the recorded hashes. It limits the f64 run to six spin-1 cells and makes `CARGO_TARGET_DIR` selectable. It limits f32 to eight full-mmax T720 cells and adds worker-level scheduler/frequency capture and output hashes. The checked repository scripts remain unchanged.

The Phase B profile also used temporary, test-only work counters and a dedicated Callgrind example. Their exact source diff from RustSHT `feb46e5f` is preserved in [profile-instrumentation.patch](profile-instrumentation.patch), SHA-256 `ef6991cc0f736971642f0c7f34cdf28a13287a813bdf0a0d199cc8ee826d3c92`. It includes the profile-only edits to `Cargo.toml`, spin-analysis diagnostics/tests/workspace counters, and the temporary `m6_spin1_analysis_callgrind` example. It is a zero-context patch: apply with `git apply --unidiff-zero` (or `patch -p1`). Forward application was checked against the current source tree and reverse application against the instrumented worktree; none of it is production code. The pinned DUCC checkout was clean at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Whole-call profile hashes and measured values are in [the Phase B report](../../DUCC_PARITY_M6.md) and [machine-readable profile CSV](../ducc-parity-m6-2026-10-27-profile.csv).

The exact-source q1 source-line attribution follow-up collected `639,748,794` Ir and reproduced the prior whole-call total within 0.002%. It confirms that the reflected-order fused recurrence and accumulation dominate; it did not select a candidate or change production code. See the [focused report](source-line-attribution-q1-2026-10-27.md), [CSV](source-line-attribution-q1-2026-10-27.csv), and the [full M6 discussion](../../DUCC_PARITY_M6.md#q1-source-line-attribution-follow-up--2026-10-27).

### Phase C indexed-loop candidate

The checked-length unchecked-index variant was tested against exact parent `1af82adb5190264e89586219c50d61e30b223822` and pinned DUCC at q1/q8, 15 interleaved triplets per thread count. It had bitwise-identical Rust outputs but no supported timing gain and was removed; see the [Phase C decision and focused tests](../../DUCC_PARITY_M6.md#phase-c-indexed-loop-screen--candidate-removed).

| Artifact | SHA-256 | Contents |
|---|---|---|
| [Candidate source patch](indexed-loop-candidate.patch) | `1c5a867b641dec243b3754d03a69d8964123d675e8ce3f1ad43609e7f588cce0` | Exact 15-line source change based on parent `1af82adb`. |
| [Triplet results JSON](indexed-loop-triplets.json) | `5902d6fef2a478fa08753bf193697dd8207f064aba2b279cfdd92d6dd4bf6acf` | Compact per-triplet medians, validation, numerical errors, and per-backend output hashes, plus source/build/fixture provenance. |
| [Triplet summary CSV](indexed-loop-triplets.csv) | `bd587ecd970a804c1a12e9c61b186dc70de54886556a1a58d70db1f3f9a381b3` | One row per q1/q8 triplet with paired ratios, median latencies, and summarized wait/frequency diagnostics. |
| [Triplet runner](indexed-loop-runner.py) | `84bc7ac6c14e500482fbcd2e737ba5c78cbb2b4a838dee33e37dee742f00374f` | Portable version using the checked RustSHT worker-command, fixture and host-diagnostic helpers. |

The executed scratch runner SHA-256 is retained in the JSON. The archived runner resolves the repository root through `RUSTSHT_REPO` or its location in the evidence tree; its measurement logic matches the executed copy. The checked harness source hashes are `e8364e3a18996bac98daa68cba348ccb0d6803576d26c167f63ce79d111ecabe` for `benchmark_ducc_spin1_native.py` and `8a364bd8f6caf1eb5c98f832cd5a28aa7446145b1a3455996cda5f35bcd86843` for `benchmark_ducc_full_transform_parity.py`.

### Paired Q/U safety scan candidate — pre-screen

| Artifact | SHA-256 | Contents |
|---|---|---|
| [Candidate source patch](pair-safety-candidate.patch) | `c71b5e36e5c953dbfbfcafdc389226ab045d242292cd17dafb76318093757071` | Shared-cosine two-channel safety scan, zero-context patch based on exact parent `469f175a`; apply with `git apply --unidiff-zero`. |
| [Pre-screen checkpoint](pair-safety-prescreen.md) | — | Focused validation passed. Two runner setup failures produced no retained timing data; the second ran one parent q1 worker through its batches before output serialization failed. Exact scratch and worktrees were removed. |
| [Pre-screen details JSON](pair-safety-prescreen.json) | — | Compact build, fixture, attempt, and cleanup provenance; no worker snapshots or local paths. |

The pre-screen record is retained as setup history. The corrected matched screen completed; see the final result section below.

## Reproduction outline

Use a fresh scratch root, current source `26941cf45564a9637b72b0eca87ff880472c3505`, and pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. Copy the two checked benchmark scripts into the clean source worktree, then apply `harness-selection.patch` from the repository root:

```sh
patch -p1 < reports/data/ducc-parity-m6-2026-10-27/harness-selection.patch
```

Build the Rust examples in an experiment-local Cargo target with `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'` and the `parallel` feature. Build the f64 and f32 DUCC adapters from `reports/data/plan15-spin1-native-baseline-2026-10-21` and `reports/data/plan17-f32-native-baseline-2026-10-22` with CMake Release and the pinned DUCC checkout. Use three interleaved process pairs; f64 target batch time is 25 ms and f32 target batch time is 10 ms. Pin q1 to CPU 0 and q8 to eight distinct physical cores. The ledger records binary hashes, flags, fixtures, pair medians, errors, output hashes (where emitted), wait fractions, and frequency summaries.

The following recreates the bounded triage from a fresh scratch root. Check free bytes and inodes on the selected transient filesystem before the build. `repo` names the primary checkout; all temporary source, build, cache, fixture, and result data stays under `scratch`.

```sh
repo="$(git rev-parse --show-toplevel)"
scratch="$(mktemp -d -t rustsht-m6-triage.XXXXXXXX)"
df -h "$(dirname "$scratch")"
df -i "$(dirname "$scratch")"
mkdir -p "$scratchtemporary filesystem"
git -C "$repo" worktree add --detach "$scratch/src" 26941cf45564a9637b72b0eca87ff880472c3505
git clone --quiet --no-hardlinks https://gitlab.mpcdf.mpg.de/mtr/ducc.git "$scratch/ducc-source"
git -C "$scratch/ducc-source" checkout --quiet --detach 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b
DUCC_SRC="$scratch/ducc-source"
cd "$scratch/src"
patch -p1 < "$repo/reports/data/ducc-parity-m6-2026-10-27/harness-selection.patch"

export CARGO_HOME="$scratch/cargo-home"
export CARGO_TARGET_DIR="$scratch/cargo-target"
export TMPDIR="$scratchtemporary filesystem"
export RUSTSHT_SPIN1_RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'

python3 scripts/benchmark_ducc_spin1_native.py \
  --ducc-src "$DUCC_SRC" \
  --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b \
  --results-dir "$scratch/results/f64-spin1" \
  --build-dir "$scratch/build/f64-spin1" \
  --lmax 720 --grid GL --process-pairs 3 --target-ms 25

RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' \
  cargo build --locked --release --example ducc_f32_full_transform_parity --features parallel
cmake -S "$scratch/src/reports/data/plan17-f32-native-baseline-2026-10-22" \
  -B "$scratch/build/f32-ducc" -DDUCC_SRC="$DUCC_SRC" \
  -DCMAKE_BUILD_TYPE=Release -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -DCMAKE_CXX_COMPILER=g++
cmake --build "$scratch/build/f32-ducc" --parallel 4

python3 scripts/benchmark_ducc_f32_native.py \
  --rust-binary "$CARGO_TARGET_DIR/release/examples/ducc_f32_full_transform_parity" \
  --ducc-binary "$scratch/build/f32-ducc/ducc_f32_full_transform_parity" \
  --outdir "$scratch/results/f32" --scratch-dir "$scratch/scratch/f32" \
  --process-pairs 3 --target-ms 10 --q1-cpu 0 \
  --q8-cpus 0,2,4,6,8,10,12,14
```

The q8 f32 scalar cases had elevated runqueue wait. Preserve that telemetry when comparing later runs; three pairs are a triage screen, not a confidence interval.


### Paired Q/U safety scan candidate — final matched screen

The corrected runner completed three balanced parent/candidate/DUCC triplets at q1 and q8. The candidate was rejected after near-parity medians and no supported repeatable gain. Exact source patch SHA-256 is c71b5e36e5c953dbfbfcafdc389226ab045d242292cd17dafb76318093757071. The measured parent was d3aeb208222d4142217f4c59b83f712b0945d4a6; its production source matches 469f175a905df34b4514a9f21db391a424adbe15, the patch's originating parent. Full method, decision and limitations are in the [screen report](pair-safety-screen-2026-10-27.md).

| Artifact | SHA-256 | Contents |
|---|---|---|
| [Triplet results JSON](pair-safety-triplets.json) | 64f1efbb34ac173c894b8480dd9d6c62839e90cb032f52e6f1fbb3fde5bd015f | Six triplets of process medians, correctness, numerical errors, output hashes and build provenance; host and worker snapshots omitted. |
| [Triplet summary CSV](pair-safety-triplets.csv) | 7172c3431e6f19607fce88fbebf3969f1fcc22774c0c7181aefc43d188e0ad63 | One row per q1/q8 triplet with timings, ratios, checks, errors, wait, frequency and output hashes. |
| [Triplet runner](pair-safety-triplet-runner.py) | 925a928a68f10797bf3be64dc22a989737e247043f9b25069c35c84d3493eaac | Corrected portable runner used for the final screen; creates its results directory before workers start. |
| [Candidate source patch](pair-safety-candidate.patch) | c71b5e36e5c953dbfbfcafdc389226ab045d242292cd17dafb76318093757071 | Recoverable zero-context source diff; not applied to production. |

The run used the GL T720 full-mmax spin-1 analysis case and pinned DUCC 0.42.0. Rust parent/candidate outputs were bitwise identical in all six triplets. The M6 timing gap remains open.

Cleanup removed only scratch root rustsht-m6-pair-safety-final.65icJ7y4 and its two registered worktrees after evidence verification. It contained 55,863 files (1,756,628 KiB allocated; 1,620,623 KiB apparent). The root and worktree registrations are absent; tmpfs available space rose by 1,798,787,072 bytes and free inodes by 65,223. No experiment worker remained.

### Reflected active-lane mask candidate — removed

The candidate reused the staging pass's active-lane decisions during reflected-block compaction. It removed the second activity predicate and its repeated ring-limit reads but added mask recording and bit tests. Parent/candidate source base was `914e68fbdf4385b3a7e74304e2bcdc77cfd6e4c1`; the three-triplet-per-q GL T720 full-mmax screen showed paired median C/P `0.992072` q1 and `1.008278` q8, with no repeatable gain. All six outputs were bitwise equal, and correctness/repeat/reset checks passed. See the [screen report](active-mask-screen-2026-10-27.md).

| Artifact | SHA-256 | Contents |
|---|---|---|
| [Candidate patch](active-mask-candidate.patch) | `1b3d8012c73752f0ea84e27a0368e5edd599b0bfccd5d67f4ad48a70d9b6bf08` | Zero-context recoverable source diff (`git apply --unidiff-zero`); the measured scratch patch SHA was `a2a399159a825272db5b04cad6d3e41fdd0fe93f7a478cf13edd4dceb47ffa04`. |
| [Triplet results JSON](active-mask-triplets.json) | `064898abfb4e5d2fba954dbb8663a9b0819158e6aea0d6dbcc0d47f1d9d19592` | Six q1/q8 triplets with process-median timings, correctness, numerical errors, output hashes, and binary/fixture provenance; no worker or host snapshots. |
| [Triplet summary CSV](active-mask-triplets.csv) | `0fa98fac94f29683d64a5b4ccbdd8551a98feae2f7e4866caaa0af98985e61e5` | Per-triplet medians, paired ratios, correctness, errors, and output hashes. |
| [Runner](active-mask-triplet-runner.py) | `925a928a68f10797bf3be64dc22a989737e247043f9b25069c35c84d3493eaac` | Reused corrected GL T720 triplet runner from the Q/U screen; a scratch patch-name alias supplied the candidate patch. |

The candidate passed `cargo test --release --locked --all-features --lib spin1` (49 passed, 4 ignored), format, and diff checks. q1 runqueue wait was 6.6–6.9%; q8 frequency samples included 1.746 GHz readings. See the [screen report](active-mask-screen-2026-10-27.md) for timing and cleanup details.

Cleanup removed only exact scratch root `rustsht-m6-active-mask.7W3HgJPa` and its two registered worktrees after artifact verification. It contained 12,841 files (1,329,602,560 bytes allocated; 1,294,995,959 apparent). The root and registrations are absent; `temporary filesystem` returned from 45 to 46 GiB available and free inodes increased by 12,430 to 949,018. No worker remained.

### Paired preweight and cosine-square fusion candidate — removed

This candidate combined local Q/U opposite-channel copy/preweight checks with per-lane cosine-square preparation in the shared spin-1 recurrence setup. It preserved the local transactional decline path but produced no supported whole-transform gain. The exact parent/candidate base was `85fd4998827619887c6d1642330ceb614b014935`; the measured source patch SHA is recorded separately from the recoverable zero-context patch hash.

| Artifact | SHA-256 | Contents |
|---|---|---|
| [Candidate patch](preweight-fusion-candidate.patch) | `fcfe352b69c12de8a24a7aec11ae86467d31eda05cf9c3da9d90fec4bce2a404` | Equivalent zero-context patch; apply with `git apply --unidiff-zero`. |
| [Triplet results JSON](preweight-fusion-triplets.json) | `a7e97193cbf32b3525c2578e5c08489a9170ff3e7641db0f4f87e7542a9c59c4` | Six q1/q8 triplets with medians, checks, numerical errors, output hashes, and build/fixture provenance; no worker or host snapshots. |
| [Triplet summary CSV](preweight-fusion-triplets.csv) | `776170071f3cc2688611366af66b317c19d5b58e7532c8bdfc79909ed1ebfeba` | Per-triplet medians, paired ratios, correctness, errors, and output hashes. |
| [Runner](preweight-fusion-triplet-runner.py) | `925a928a68f10797bf3be64dc22a989737e247043f9b25069c35c84d3493eaac` | Reused corrected GL T720 triplet runner from the earlier matched screen. |

See the [screen report](preweight-fusion-screen-2026-10-27.md) for the measured patch SHA, test result, and cleanup record.

Cleanup removed only exact scratch root `rustsht-m6-preweight-fusion.2ccEyRrg` and its two registered worktrees after artifact verification. It contained 12,850 files (1,329,721,344 bytes allocated; 1,295,118,456 apparent). The root and registrations are absent; `temporary filesystem` returned from 45 to 46 GiB available and free inodes increased by 14,474 to 949,018. No worker remained.

### Checked-scaled prefix/update fusion candidate — screened and removed

The checked-scaled prefix/update fusion was based on exact parent `67eb85151d0e91cca0fcc3bc6b89372f67e64dcd` and rejected after three GL T720 full-mmax spin-1 analysis triplets at each q. It was slower in all six pairs; focused numerical tests, bitwise output comparisons, finite checks, and repeat/reset checks passed. See the [screen report](checked-prefix-fusion-screen-2026-10-27.md).

| Artifact | SHA-256 | Contents |
|---|---|---|
| [Candidate source patch](checked-prefix-fusion-candidate.patch) | `aa01403775a9113be444d693c659d825f673875f53e7a87b275c36734b95bed7` | Recoverable zero-context diff; not applied to production. |
| [Triplet summary JSON](checked-prefix-fusion-triplets.json) | `0233d11b231d35e9bc2828621f1500fa916a27841c5d1904664e36705545e7e8` | Six triplet medians, ratios, correctness/error summaries and source/binary/fixture/output provenance; no worker snapshots or batch-level samples. |
| [Triplet summary CSV](checked-prefix-fusion-triplets.csv) | `9919c378d47198f9d9fe37ec03f0b4fb4b0d143b10ffbf08a1ca3ed35dbaeda6` | One row per triplet with medians, paired ratios, checks, DUCC errors, wait and frequency summaries. |
| [Screen report](checked-prefix-fusion-screen-2026-10-27.md) | — | Method, result, validation, decision and limitations. |

The original runner capture is retained locally and untracked at SHA-256 `1efc83e1ec8ca4681c4fc8d1a8ebd9d7af0c7986b68daa5a30caa071bd411ad0`. After verifying the compact evidence, cleanup removed only the experiment-owned scratch root `rustsht-m6-checked-prefix-fusion.gMZMJ8I2` and its two worktrees. It held 9,007 files and 924,896,044 apparent bytes; `temporary filesystem` available space rose by 930,566,144 bytes and free inodes increased by 10,380 to 949,018. No worker remained.

### Two-vector recurrence pipeline candidate — screened and removed

The two-vector pipeline candidate followed exact parent `30a17ee82154d761ccbbb40a256e27a97d3c2946` and was rejected after three GL T720 full-mmax spin-1 analysis triplets at each q. It was slower in all six pairs despite bitwise-equal parent/candidate outputs, finite results, and passing focused tests. See the [screen report](two-vector-pipeline-screen-2026-10-27.md).

| Artifact | SHA-256 | Contents |
|---|---|---|
| [Candidate source patch](two-vector-pipeline-candidate.patch) | `f786de45bb2e09854146ad1259324e89619b4278b0fdb423351802f8e69f18a6` | Recoverable zero-context diff; not applied to production. |
| [Triplet summary JSON](two-vector-pipeline-triplets.json) | `332bc3b5061099d9ee16a4f6fac031123bd38d2126e085a84ea48e91c76f5f8c` | Six triplet medians, ratios, correctness/error summaries and source/binary/fixture/output provenance; no worker snapshots or batch-level samples. |
| [Triplet summary CSV](two-vector-pipeline-triplets.csv) | `392ecf7328a0677bc550fac3dc0ffa18ba9d0991e06b3306ebb2aa432f5b6d20` | One row per triplet with medians, paired ratios, checks, DUCC errors and summarized scheduler-wait diagnostics. |
| [Screen report](two-vector-pipeline-screen-2026-10-27.md) | — | Method, result, validation, decision and limitations. |

The original runner capture is retained locally and untracked at SHA-256 `95312da1c6130384f5d055584c048de1727c461861f2df705753b3f9b4f0f060`. After verifying the compact evidence, cleanup removed only the experiment-owned scratch root `rustsht-m6-two-vector-pipeline.6ZaMVKzS` and its two worktrees. It held 9,557 files and 1,697,144,643 apparent bytes; `temporary filesystem` available space rose by 1,667,055,616 bytes and free inodes increased by 11,004 to 949,016. No worker remained.

### CC T720 reduced-synthesis q1 route profile

The current M6 CC reduced-order q1 synthesis gap was profiled with the existing ignored phase helper at source `f2f813fd`. The measured order-recurrence/spectral-accumulation phase was 26.967–28.218 ms; both inverse longitude FFTs and map writeback took 0.968–1.055 ms. The profile recorded 1,736 ordinary fused paired-kernel successes and 305 fallback entries per call. This is route evidence only, not a matched timing or numerical comparison. See the [focused report](cc-t720-reduced-q1-route-profile-2026-10-27.md) and [compact CSV](cc-t720-reduced-q1-route-profile.csv). Cleanup removed only scratch root `rustsht-m6-cc-q1-profile.lI3xYv` after verifying the compact evidence; its 1,245 files occupied 120,484 KiB and `temporary filesystem` available space rose by 123,375,616 bytes, with 1,519 inodes reclaimed.

### Forced native-spin analysis route — rejected

The candidate bypassed the fast GL scalar-conversion route and forced existing direct native-spin recurrence for GL T720 full-mmax spin-1 analysis. Three parent/candidate/DUCC triplets per q showed the route was about 26× slower at q1 and 23× slower at q8. Candidate output matched DUCC numerically, but the current scalar-conversion path remains essential for performance. The [screen report](native-spin-route-screen-2026-10-27.md), [recoverable candidate patch](native-spin-route-candidate.patch), [triplet CSV](native-spin-route-triplets-2026-10-27.csv), and [triplet JSON](native-spin-route-triplets-2026-10-27.json) preserve the evidence. The exact scratch and two worktrees were removed after checks; 941,285,376 bytes and 10,408 inodes were reclaimed.

### Shared phase-scale and FMA rotation — rejected

The candidate combined the per-ring scale with the phase factor shared by Q and U, then used explicit FMA complex rotation in the reflected spin-1 analysis route. Fifteen matched GL T720 full-mmax parent/candidate/DUCC triplets per q showed median C/P `0.995288` q1 and `0.998563` q8; both bootstrap intervals included 1, so the small lead is inconclusive. Numerical differences remained at roundoff scale and spin-1 validation passed. See the [screen report](phase-scale-screen-2026-10-27.md), [candidate patch](phase-scale-candidate.patch), [triplet CSV](phase-scale-triplets-2026-10-27.csv), [triplet JSON](phase-scale-triplets-2026-10-27.json), and [runner](phase-scale-triplet-runner.py). Cleanup removed only owned scratch root `rustsht-m6-phase-scale.9C0pr88s` and its two registered worktrees after evidence verification; it held 9,453 files and 1,615,943,895 apparent bytes. Available temporary filesystem space rose by 1,639,022,592 bytes and free inodes increased by 10,890. The scratch root and registrations are absent.


### Fused inverse-sine scale in paired synthesis — rejected

The candidate moved the final per-ring scale into the four SIMD recurrence sums in `FusedPreparedSynthesisPairBlock`. On CC T720 reduced-mmax synthesis, q1 was 2.4–3.4% slower in all three pairs; q8 was noisy and inconclusive. Outputs were bitwise equal to the parent and the focused spin-1 suite passed. The candidate was not promoted. See the [screen report](simd-scale-screen-2026-10-27.md).

| Artifact | SHA-256 | Contents |
|---|---|---|
| [Candidate source patch](simd-scale-candidate.patch) | `e0725761add848c53cf4032d0b68f3865aa6141703b9758cda5e338d78e4c6fe` | Isolated source diff; not applied to production. |
| [Triplet summary JSON](simd-scale-triplets.json) | `9eac3742879b444042bf84ae9c94920e5085c46a72ad3e14478bf1ab2cc9091d` | Six triplet records, all per-worker batches/diagnostics, correctness/error checks and provenance. |
| [Triplet summary CSV](simd-scale-triplets.csv) | `08ebe895b5f7751ee1f026727c84a54085893967f452b261a85c67d5b92b9bd5` | One row per triplet with paired ratios, errors and scheduler wait. |
| [Screen report](simd-scale-screen-2026-10-27.md) | `cccfe49f29acd2b4a99d9904b33686dd3c27655a4057320698fc79e8c4838dad` | Method, source/toolchain provenance, validation, results and cleanup. |
| [Runner](simd-scale-triplet-runner.py) | `03b21ce64647ec8e6b40f8a0b997f780cd54d6a93cfb4693c1d9f850c54b9084` | Reproducible runner expecting binaries and worktrees under `RUSTSHT_M6_SCRATCH`. |

Cleanup removed only owned scratch root `rustsht-m6-simd-scale.BfvcbdyW` and its two registered worktrees after verifying all durable artifacts. It held 853,016,576 apparent bytes across 9,034 files. `temporary filesystem` available space rose by 853,016,576 bytes and free inodes increased by 10,408; the root and registrations are absent and no worker remains.


### Direct writes to cleared paired-synthesis spectra — rejected

The candidate replaced Q/U read/add updates with direct stores in `synthesize_prepared_order_pair_block_dispatched`; the per-order output slots are cleared and owned by one recurrence block. Fifteen CC T720 reduced-mmax synthesis triplets per q showed no supported gain: q1 paired C/P `0.997470 [0.992019,1.002035]`, q8 `1.008259 [0.982173,1.010737]`. Outputs were bitwise equal and validation passed. See the [screen report](direct-store-screen-2026-10-27.md).

| Artifact | SHA-256 | Contents |
|---|---|---|
| [Candidate source patch](direct-store-candidate.patch) | `de5863798bf82495d2a235c77ea4d9e7e75682019e26c57b8e5383d2d888a037` | Isolated diff; not applied to production. |
| [Triplet JSON](direct-store-triplets.json) | `8906fe0f64e4017d422d053056d21232d3f1e2d2a93064f6ee844e9ed642e8aa` | Thirty per-q triplets, 90 worker records and full provenance. |
| [Triplet CSV](direct-store-triplets.csv) | `df58553a367d46a4e81a052a294fb488208136be267c0cba6e748f143a686bec` | One row per triplet with paired ratios, errors and scheduler wait. |
| [Screen report](direct-store-screen-2026-10-27.md) | `dad89b3441dbba20ce91f850d98fa8758ef8385819448d1027aeb62ac2e6fe60` | Method, validation, results, decision and cleanup. |
| [15-pair runner](direct-store-triplet-runner.py) | `0f1144b7081e1c809fc90c06be2e00309690f563f5726b0adcb4a08dc9841044` | Reproducible paired runner; expects binaries/worktrees under `RUSTSHT_M6_SCRATCH`. |

Cleanup removed only owned scratch root `rustsht-m6-direct-store.fCds2w2R` and its two registered worktrees after verifying the durable artifacts. It held 1,168,195,584 apparent bytes across 9,177 files. `temporary filesystem` available space rose by 1,168,195,584 bytes and free inodes increased by 10,550; the root and registrations are absent and no worker remains.

### CC T720 reduced synthesis q1 source attribution

The exact-source q1 Callgrind run collected 613,272,869 Ir around one warmed transform. The prepared fused pair route accounted for 47.26% inclusive Ir; direct source attribution mapped 10.06% of total Ir to its nested zip iterator. The indexed-read candidate was screened and rejected: it reduced matched Ir only 0.12%, while q8 whole-transform ratios favored the parent. Checked transition and fallback regions remain the next source area to inspect. See the [source-profile report](cc-t720-reduced-q1-source-attribution-2026-10-27.md) and [indexed-pair screen](indexed-pair-screen-2026-10-27.md).

| Artifact | SHA-256 | Contents |
|---|---|---|
| [Source attribution report](cc-t720-reduced-q1-source-attribution-2026-10-27.md) | `1c575480e555a2f25e268cc1d184c4a3965d8097c02dc923cb86448a54005706` | Scope, method, line-level interpretation, candidate hypothesis, and cleanup record. |
| [Callgrind summary CSV](cc-t720-reduced-q1-source-attribution.csv) | `4d9900039aa034d1c09b8410f492d9ab25d779242c8af6143a9bc3d4482d686d` | Inclusive function and direct source-attribution Ir counts; overlapping inclusive rows are labeled. |
| Raw Callgrind profile (local, untracked) | `1848049f5f35426fc9efd6a06a9d447e6dbb9aa16798117b263bb556314c8ef2` | 204 KiB Valgrind 3.26.0 profile for the single measured transform. |
| [Rust harness](cc-t720-reduced-q1-callgrind-harness.rs) | `6bcf5df94445389db5dfc4821681f5cede0cc61bfe12ebcc5562b05416cc4446` | Scratch-only public-API harness with deterministic coefficients, warm/repeat check, and Callgrind bracketing. |
| [C shim](cc-t720-reduced-q1-callgrind-shim.c) | `6caa936803f86a346742c49fc205e4cc2c63053e1bd6dda70d478abba8e282ee` | Callgrind zero/start/stop macros linked by the scratch build. |

Cleanup removed the exact 506,452,604-byte scratch root and its one registered worktree after evidence verification; `temporary filesystem` free space rose by 518,352,896 bytes and free inodes by 5,598. The root/registration are absent and no worker remains.


### Prepared paired-synthesis indexed traversal — rejected

The candidate replaced the nested zip/enumerate traversal with ordered index reads in the prepared two-channel synthesis SIMD vector and scalar-tail loops. It preserved coefficient and FMA order. Matched CC T720 reduced synthesis and GL T1023 f64 synthesis runs at q1/q8 found bitwise-equal parent/candidate outputs, finite/repeat/reset success, and no supported whole-transform win. The matched q1 CC Callgrind total fell only 0.1182%; q8 candidate/parent medians were 1.0143 (CC, 0/3 faster) and 1.0110 (GL, 1/3 faster). See the [screen report](indexed-pair-screen-2026-10-27.md).

| Artifact | SHA-256 | Contents |
|---|---|---|
| [Screen report](indexed-pair-screen-2026-10-27.md) | `ba1cbb8921a442ffcb6806b92fd16582ce36c8349865e85d8079257834518d58` | Candidate, validation, per-cell timing, instruction attribution, provenance, and cleanup. |
| [Candidate patch](indexed-pair-candidate.patch) | `007238b0fc312d2dfd885cfd03ec8ec21ccf5e019b7d482ba1259d9e24f730cb` | Recoverable isolated source diff; never applied to production. |
| [Triplet runner](indexed-pair-triplet-runner.py) | `924ca5903415bbf5b9042efd1e275825160351df1e1c7b921a92bce083ee84cd` | Checked matched parent/candidate/DUCC worker protocol, parameterized for CC and GL controls. |
| [CC triplet JSON](indexed-pair-cc-triplets.json) | `6de873fc936efd673a69ce26d202f071165772ef440e4a88e73edf41f40d0cb5` | Three triplets per q for CC T720 reduced mmax. |
| [CC triplet CSV](indexed-pair-cc-triplets.csv) | `40d0756dc0da38097ea4911837559ac6b86ca7a874b2afc62f11db43ff384154` | Per-triplet timing, correctness, error, and wait diagnostics. |
| [GL triplet JSON](indexed-pair-gl-triplets.json) | `b0e6c39e90afb1b86a9785fd6e94fbba0b5b172d1a65a0b58e4ce1ca859ae2dc` | Three triplets per q for GL T1023 full mmax. |
| [GL triplet CSV](indexed-pair-gl-triplets.csv) | `ed5ec9d668a302ba74188cbf2269680e0a246b9278e2e9641899cfe34173b38c` | Per-triplet timing, correctness, error, and wait diagnostics. |
| [Callgrind comparison CSV](indexed-pair-callgrind.csv) | `d35a60d357965ce372a73c5a6eb4fb08e082388e803262a68aebf9dc1734a169` | Matched parent/candidate Ir and the direct source cost shift. |
| Parent Callgrind profile (local, untracked) | `6405e466e013c4e8c6ba955a501ab551f8ce3d8b1794104c607de629e4864846` | One warmed CC T720 reduced q1 transform. |
| Candidate Callgrind profile (local, untracked) | `c72c6f4d8cb9a0d6b25a61b221cd62245178ba3130b2e8d1cc2edff4cfafb91d` | Same harness and inputs as parent. |

Cleanup removed the exact 2,113,777,626-byte scratch root and its two worktrees after verification; `temporary filesystem` available space rose by 2,138,640,384 bytes and free inodes by 11,546. No worker remains.


### Prepared synthesis safety-check cache — rejected

The isolated candidate memoized the prepared-pair coefficient safety scan across adjacent latitude blocks, evaluating it only after the ordinary fused route declined. It passed focused transition tests and the release spin-1 suite (49 passed, 4 ignored); all 12 CC/GL q1/q8 triplets were bitwise equal to the parent and passed finite, repeat, and reset checks. CC median candidate/parent ratios were 1.001844 q1 and 1.000910 q8. GL T1023 q1 had a small unconfirmed lead (0.991833), which did not repeat at q8 (1.000487). **Rejected; no production source changed.** See the [screen report](prepared-safety-cache-screen-2026-10-27.md).

| Artifact | SHA-256 | Contents |
|---|---|---|
| [Candidate source patch](prepared-safety-cache-candidate.patch) | `36553db57b80244ac39325f8049097fa5d9c405b8a9449412c8c3b6380f3c341` | Exact-parent isolated diff; not applied to production. |
| [CC triplet CSV](prepared-safety-cache-cc-triplets.csv) | `bcb2216aa7395fbd19eadd17d56c67c6c2609a3df1310ef708829874f2d6e869` | Three q1/q8 triplets for CC T720 reduced synthesis, with timing and diagnostics. |
| [CC triplet JSON](prepared-safety-cache-cc-triplets.json) | `3e71a8d6e3083bc07630380090120d1ebc50ea6e3bc8df400c50e5a69f1f3d44` | Sanitized source, build, fixture, binary, output, and per-worker provenance. |
| [GL triplet CSV](prepared-safety-cache-gl-triplets.csv) | `a1650c23fc984152d6b3c5595bb9dbfeb795b04aa1ed0c1ba0a1bbeabba00e64` | Three q1/q8 triplets for GL T1023 full synthesis, with timing and diagnostics. |
| [GL triplet JSON](prepared-safety-cache-gl-triplets.json) | `26324165ccfd669b632f62c5dc0d16cb44fab77cf38588f0c836d1224c83b619` | Sanitized source, build, fixture, binary, output, and per-worker provenance. |
| [Screen report](prepared-safety-cache-screen-2026-10-27.md) | `1a41d06250a070ecc84de1600b1e61c11fc69b6a1fac620523f812dfe6311074` | Candidate, validation, measurements, decision, provenance, and exact cleanup record. |

Cleanup removed only the owned experiment scratch and its two worktrees after verifying the durable evidence. The scratch contained 10,791 paths and 1,568,382,753 apparent bytes; temporary-filesystem available space increased by 1,591,410,688 bytes and free inodes by 10,791. No experiment process or worktree remains.
