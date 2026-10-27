# M6 triage evidence provenance

This directory preserves the bounded selection patch for the checked benchmark workers. The f64 selector derives from `scripts/benchmark_ducc_spin1_native.py` (original SHA-256 `e8364e3a18996bac98daa68cba348ccb0d6803576d26c167f63ce79d111ecabe`); the f32 selector derives from `scripts/benchmark_ducc_f32_native.py` (original SHA-256 `70033f7251358128783f35d7aefa527fa957abce5513571db1142e92e6e38ff0`). The zero-context patch SHA-256 is `63797a6c9566cdfae6d07284fd1401f001fbf177b6c66ccc9ed341ead64e563d`. The resulting bounded copies had SHA-256 values `a80b1c3a43a788978011d3fca19229d35247769c39d3ab8b5b91dcd011b921fd` and `98cfebe740a04d0506f01b9bde2a466e2f575594a14c0a98e0f31f6763702cd4`.

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
mkdir -p "$scratch/tmp"
git -C "$repo" worktree add --detach "$scratch/src" 26941cf45564a9637b72b0eca87ff880472c3505
git clone --quiet --no-hardlinks https://gitlab.mpcdf.mpg.de/mtr/ducc.git "$scratch/ducc-source"
git -C "$scratch/ducc-source" checkout --quiet --detach 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b
DUCC_SRC="$scratch/ducc-source"
cd "$scratch/src"
patch -p1 < "$repo/reports/data/ducc-parity-m6-2026-10-27/harness-selection.patch"

export CARGO_HOME="$scratch/cargo-home"
export CARGO_TARGET_DIR="$scratch/cargo-target"
export TMPDIR="$scratch/tmp"
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

Cleanup removed only exact scratch root `rustsht-m6-active-mask.7W3HgJPa` and its two registered worktrees after artifact verification. It contained 12,841 files (1,329,602,560 bytes allocated; 1,294,995,959 apparent). The root and registrations are absent; `/tmp` returned from 45 to 46 GiB available and free inodes increased by 12,430 to 949,018. No worker remained.

### Paired preweight and cosine-square fusion candidate — removed

This candidate combined local Q/U opposite-channel copy/preweight checks with per-lane cosine-square preparation in the shared spin-1 recurrence setup. It preserved the local transactional decline path but produced no supported whole-transform gain. The exact parent/candidate base was `85fd4998827619887c6d1642330ceb614b014935`; the measured source patch SHA is recorded separately from the recoverable zero-context patch hash.

| Artifact | SHA-256 | Contents |
|---|---|---|
| [Candidate patch](preweight-fusion-candidate.patch) | `fcfe352b69c12de8a24a7aec11ae86467d31eda05cf9c3da9d90fec4bce2a404` | Equivalent zero-context patch; apply with `git apply --unidiff-zero`. |
| [Triplet results JSON](preweight-fusion-triplets.json) | `a7e97193cbf32b3525c2578e5c08489a9170ff3e7641db0f4f87e7542a9c59c4` | Six q1/q8 triplets with medians, checks, numerical errors, output hashes, and build/fixture provenance; no worker or host snapshots. |
| [Triplet summary CSV](preweight-fusion-triplets.csv) | `776170071f3cc2688611366af66b317c19d5b58e7532c8bdfc79909ed1ebfeba` | Per-triplet medians, paired ratios, correctness, errors, and output hashes. |
| [Runner](preweight-fusion-triplet-runner.py) | `925a928a68f10797bf3be64dc22a989737e247043f9b25069c35c84d3493eaac` | Reused corrected GL T720 triplet runner from the earlier matched screen. |

See the [screen report](preweight-fusion-screen-2026-10-27.md) for the measured patch SHA, test result, and cleanup record.

Cleanup removed only exact scratch root `rustsht-m6-preweight-fusion.2ccEyRrg` and its two registered worktrees after artifact verification. It contained 12,850 files (1,329,721,344 bytes allocated; 1,295,118,456 apparent). The root and registrations are absent; `/tmp` returned from 45 to 46 GiB available and free inodes increased by 14,474 to 949,018. No worker remained.

### Checked-scaled prefix/update fusion candidate — screened and removed

The checked-scaled prefix/update fusion was based on exact parent `67eb85151d0e91cca0fcc3bc6b89372f67e64dcd` and rejected after three GL T720 full-mmax spin-1 analysis triplets at each q. It was slower in all six pairs; focused numerical tests, bitwise output comparisons, finite checks, and repeat/reset checks passed. See the [screen report](checked-prefix-fusion-screen-2026-10-27.md).

| Artifact | SHA-256 | Contents |
|---|---|---|
| [Candidate source patch](checked-prefix-fusion-candidate.patch) | `aa01403775a9113be444d693c659d825f673875f53e7a87b275c36734b95bed7` | Recoverable zero-context diff; not applied to production. |
| [Triplet summary JSON](checked-prefix-fusion-triplets.json) | `0233d11b231d35e9bc2828621f1500fa916a27841c5d1904664e36705545e7e8` | Six triplet medians, ratios, correctness/error summaries and source/binary/fixture/output provenance; no worker snapshots or batch-level samples. |
| [Triplet summary CSV](checked-prefix-fusion-triplets.csv) | `9919c378d47198f9d9fe37ec03f0b4fb4b0d143b10ffbf08a1ca3ed35dbaeda6` | One row per triplet with medians, paired ratios, checks, DUCC errors, wait and frequency summaries. |
| [Screen report](checked-prefix-fusion-screen-2026-10-27.md) | — | Method, result, validation, decision and limitations. |

The original runner capture is retained locally and untracked at SHA-256 `1efc83e1ec8ca4681c4fc8d1a8ebd9d7af0c7986b68daa5a30caa071bd411ad0`. After verifying the compact evidence, cleanup removed only the experiment-owned scratch root `rustsht-m6-checked-prefix-fusion.gMZMJ8I2` and its two worktrees. It held 9,007 files and 924,896,044 apparent bytes; `/tmp` available space rose by 930,566,144 bytes and free inodes increased by 10,380 to 949,018. No worker remained.
