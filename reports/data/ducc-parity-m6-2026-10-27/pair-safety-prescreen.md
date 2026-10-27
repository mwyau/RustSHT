# Spin-1 paired safety scan — pre-screen checkpoint

**Parent source:** `469f175a905df34b4514a9f21db391a424adbe15`

**Candidate patch SHA-256:** `c71b5e36e5c953dbfbfcafdc389226ab045d242292cd17dafb76318093757071`

**Patch:** [pair-safety-candidate.patch](pair-safety-candidate.patch)

The candidate combines Q/U input and post-compaction safety scans so each SIMD pass validates the shared cosine lanes once while preserving a separate safety result for each channel. The checked-scaled recurrence, range cache, and fallback routing are unchanged.

Pre-screen validation on the candidate passed `cargo fmt --all -- --check`, `git diff --check`, and `cargo test --locked --all-features --lib legendre::scalar_paired::analysis` (35 passed, 8 ignored) plus `cargo test --locked --all-features --lib spin1` (49 passed, 4 ignored). The release Rust parent and candidate workers built with rustc 1.99.1 / LLVM 23.1.1 and `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. The pinned DUCC 0.42.0 worker also built from `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` with GCC 15.2 and the recorded M6 flags.

**No timing result was retained.** The initial runner stopped during Python module initialization: its `dict.get` fallback eagerly evaluated `Path(__file__).resolve().parents[3]` for the scratch runner path, raising `IndexError` even though `RUSTSHT_REPO` was set. No fixture was created.

The fresh-root retry fixed that path selection and created the deterministic GL T720 fixture (seed `1416973757`; map SHA-256 values `e3379278…d7abba` and `dc3a1322…d5cc`, coefficient SHA-256 values `f4e07903…bb96b8` and `148b93ae…f9174fd`). The first parent q1 worker then ran through its analysis, warm-up, and timed batches, but failed when serializing output because the runner had not created its results directory. The worker writes outputs before printing its timing JSON, so this acquisition retained neither output nor timing values. Candidate and DUCC workers were not started. Binary/source hashes and the exact failure state are in [the machine-readable record](pair-safety-prescreen.json). No performance conclusion follows.

After saving and verifying the patch, fixture hashes, and failure record, removed exact owned scratch root label `rustsht-m6-pair-safety-retry.DYDnowSr` and both registered worktrees. The root held 55,820 files and 1,610,060 KiB allocated according to `du`; it is absent and no worker remains. The transient filesystem free-inode count rose from 883,839 to 949,018 (+65,179); available space moved from 45 GiB to 46 GiB in the host's rounded `df` display. No other parked worktree or shared cache was touched.
