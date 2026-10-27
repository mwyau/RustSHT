# Paired Q/U safety scan — f64 triage screen

**Decision:** reject the candidate; retain the checked production path. The three-pair screen found no supported whole-transform gain at either thread count. No production source was changed.

**Parent:** d3aeb208222d4142217f4c59b83f712b0945d4a6 (dev). The archived candidate patch was prepared from the same production source at 469f175a905df34b4514a9f21db391a424adbe15; the intervening parent commit changed reports only.
**Candidate patch SHA-256:** c71b5e36e5c953dbfbfcafdc389226ab045d242292cd17dafb76318093757071.
**DUCC:** 0.42.0, 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b.
**Case:** GL T720 spin-1 analysis, full mmax=720, ntheta=721, nphi=1458, phi0=0.23, q1/q8.
**Host/build:** AMD Ryzen 9 5950X; rustc 1.99.1 / LLVM 23.1.1, release -C target-cpu=x86-64-v3 -Aunexpected_cfgs; GCC 15.2.0. DUCC SHT sources used -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; its driver added -fno-fast-math -ffp-contract=fast.
**Fixture:** seed 1416973757; map and coefficient hashes are in the JSON record.

The candidate combines Q/U input and post-compaction safety scans while reusing cosine validation. Checked-scaled recurrence, range cache, and fallback routing are unchanged. The patch does not remain in production.

Each q used three fresh-process parent/candidate/DUCC triplets in balanced rotating order. Workers pinned q1 to CPU 0 and q8 to CPUs 0–7, with a monitor on CPU 8. Calls were warmed; each process recorded ten batches targeting 25 ms. Timing covers reusable-plan whole-transform calls, excluding setup, fixture I/O, and output checks. The triplets are a triage screen, not a confidence interval.

| q | Parent / candidate / DUCC median of process medians (ms) | Paired C/P median | Faster C/P pairs | Parent/DUCC; candidate/DUCC paired-ratio medians |
|---:|---:|---:|---:|---:|
| 1 | 36.411 / 36.380 / 29.463 | 0.999166 | 2/3 | 1.254876; 1.239101 |
| 8 | 5.340 / 5.230 / 4.235 | 0.998194 | 2/3 | 1.265562; 1.234792 |

Parent and candidate outputs were bitwise identical in all six triplets. All outputs were finite, and all workers passed repeat/reset checks. Parent and candidate each had maximum DUCC relative-L2 error 4.760590e-14 and maximum absolute error 1.295838e-15. Per-pair process medians, output hashes, and build provenance are in the [JSON](pair-safety-triplets.json); the [CSV](pair-safety-triplets.csv) also records summarized wait and frequency.

At q1, parent and candidate workers each saw about 6.5–6.9% scheduler wait. At q8, two pairs were essentially neutral; the remaining parent sample was 6.003 ms versus its candidate at 5.230 ms and had 2.46% parent wait versus 0.55% candidate wait. Keep this sample; it explains the spread but does not turn the near-parity medians into evidence of a repeatable win. The candidate still ran about 1.24× DUCC in both q1 and q8 paired-ratio medians, so this screen does not close the M6 gap.

Candidate focused validation was completed before timing: paired-analysis tests (35 passed, 8 ignored), spin-1 tests (49 passed, 4 ignored), formatting and diff checks. No expanded test suite or warning-denied Clippy was run for this rejected candidate.

**Disposition:** do not extend this candidate to 15 pairs or merge its additional scan-sharing control flow. Preserve the patch and measurements as negative evidence. A new optimization needs a distinct causal hypothesis; this experiment neither establishes parity nor changes the production baseline.

Cleanup: removed only scratch root rustsht-m6-pair-safety-final.65icJ7y4 and its two registered parent/candidate worktrees after verifying the durable evidence. The scratch contained 55,863 files (1,756,628 KiB allocated; 1,620,623 KiB apparent). The exact root and worktree registrations are absent. Tmpfs available space increased by 1,798,787,072 bytes and free inodes by 65,223; no experiment worker remained.

The executed runner SHA-256 is 925a928a68f10797bf3be64dc22a989737e247043f9b25069c35c84d3493eaac; its portable copy is [pair-safety-triplet-runner.py](pair-safety-triplet-runner.py). The recoverable source change is [pair-safety-candidate.patch](pair-safety-candidate.patch).
