# Independent review — advisory, subordinate to owner instructions

**Inspected branch:** `opt/sht-performance-parity` at `08b938a1d9f89455b9c7a0b8bd81776988ea314a` (2026-10-22 UTC).  
**Severity:** Integration-provenance and scientific-compatibility risk; **not** evidence of a new numerical-kernel regression.  
**Disposition:** Owner decision required before relocating or announcing the compatibility implementation. Continue Plan 16 Milestone E independently.

## 1. Validate integration provenance for Plan 18

**Verified facts from the checked-in branch**

- `src/lib.rs` publicly exposes `pub mod pyspharm`, and `python/pyspharm-rs/spharm.py` contains the `spharm` compatibility wrapper. These files are present on the **pushed performance branch**. The addition was associated with [`811bccc0`](https://github.com/mwyau/RustSHT/commit/811bccc020b7dd6e7c7b9c9678021122864adde8) and [`4d28651f`](https://github.com/mwyau/RustSHT/commit/4d28651f1e1555136f70e064027dd958a39d6f52).
- [Plan 18](plans/18-pyspharm-and-native-ffi-compatibility.md) says the Python compatibility implementation was completed on isolated `feat/pyspharm-numerics`, that the performance branch remained untouched, and that the feature must not be merged, pushed or published without authorization. The **current branch contents** and that claimed separation are not consistent.
- Plan 18 already records a **scientific limitation** rather than full numerical parity: the tested regular-grid maximum-truncation cases differ from pyspharm-syl by relative L2 approximately `3.146e-2` at odd `nlat=7, T6` and `6.702e-2` at even `nlat=8, T7`. It also documents that the `pyspharm-rs` distribution does not automatically satisfy windspharm's `pyspharm>=1.0.9` dependency metadata.
- The **previous paired-odd-FFT integration discrepancy is resolved** by current [AGENTS.md](AGENTS.md) and [its production validation report](reports/paired-odd-longitude-fft-production-validation-2026-10-22.md). Its guarded source is explicitly acknowledged and validation was owner-authorized. **Do not reopen or undo this route.**

**Unknown:** whether the owner separately approved the Plan 18 code arriving on the performance branch. An apparent documentation contradiction does **not** authorize removing working code or reversing published commits.

### Goal for the existing Luna/Codex session

**Establish the provenance and owner-approved disposition of the already-pushed Plan 18 files without disrupting Plan 16 Milestone E.**

Constrained steps:

1. After the mandatory safe remote sync, inspect `git log --oneline --ancestry-path`, `git merge-base`, and the exact file histories for `src/pyspharm.rs` / `src/pyspharm/` as applicable, `src/lib.rs`, `python/pyspharm-rs/`, and `plans/18-pyspharm-and-native-ffi-compatibility.md`. Compare current `opt/sht-performance-parity` against `feat/pyspharm-numerics` and `main` without rewriting any branch. Record the actual introducing commits and whether this was a cherry-pick, merge, or independent implementation.
2. Check any later owner instruction explicitly authorizing integration. **Do not infer approval solely from the fact that files are pushed.** If approval is unclear, report the specific decision needed: keep the implementation on the performance branch versus isolate it for later integration. Continue Plan 16 Milestone E rather than blocking optimization.
3. If the owner confirms the current placement, reconcile only the stale Plan 18 provenance/status wording and add focused testing where independently justified. If the owner wants isolation, propose a non-history-rewriting follow-up and await authorization before removing files from the performance branch. Do not force-push, reset, revert, merge, publish wheels, or alter production kernels on this advisory alone.
4. Do not claim complete pyspharm numerical compatibility or drop documented regular-grid truncation limitations without independent reference tests. Keep windspharm metadata-resolution limitations explicit.

**Scope:** This is a provenance/coordination checkpoint, not a request to restart Plan 18, broaden FFT work, or change Plan 16/17 priorities.

## Review follow-up

An independent reviewer should update this file only when new source/benchmark evidence materially changes the next engineering decision; replace superseded advice rather than accumulating routine hourly notes. All updates remain subordinate to the owner's latest `AGENTS.md` and direct instructions.
