# Native-f32 scalar and spin-1 analysis rebaseline — 2026-10-27

This screen updates the GL T720 f32 analysis baseline after the exponent-zero fast path in multiply_complex_scaled_f32. The exact comparison parent is 8c4dba98fbc4cf7fe3499c7353cde6c18b7d457f; the tested production source is 3189fe8ee49f125d0d4569549fb880a6b6468bb8 (built from a report-only descendant). The pinned DUCC 0.42.0 source is 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b.

The source delta also includes scalar-synthesis code, which these analysis calls do not execute. This rebaseline measures the current analysis behavior; it does not attribute every timing change beyond the helper fast path.

## Matched GL T720 results

Three interleaved fresh-process parent/current/DUCC triplets ran for each scalar and spin-1 analysis cell. Full and reduced mmax were 720 and 180; q1 used CPU 0 and q8 used CPUs 0–7. Rust workers were built in release mode with native f32 and -C target-cpu=x86-64-v3 -Aunexpected_cfgs. DUCC used the pinned C++ worker and matching x86-64-v3 numerical flags. The ratio is the median of the three per-triplet current/parent ratios.

| Analysis | Parent ms | Current ms | DUCC ms | Current/parent (range) | Current/DUCC |
|---|---:|---:|---:|---:|---:|
| Scalar M720 q1 | 605.127 | 540.744 | 14.049 | 0.89360 (0.88134–0.90289) | 38.50× |
| Scalar M720 q8 | 79.085 | 71.614 | 2.057 | 0.90446 (0.90433–0.90853) | 34.86× |
| Scalar M180 q1 | 225.908 | 188.653 | 5.774 | 0.84907 (0.83093–0.89418) | 32.89× |
| Scalar M180 q8 | 29.867 | 25.496 | 0.931 | 0.85367 (0.84588–0.86876) | 27.38× |
| Spin-1 M720 q1 | 1035.894 | 1041.688 | 28.768 | 1.01212 (1.00127–1.01555) | 36.57× |
| Spin-1 M720 q8 | 136.005 | 136.606 | 4.144 | 1.00442 (1.00360–1.01536) | 33.31× |
| Spin-1 M180 q1 | 370.826 | 376.515 | 11.728 | 1.01466 (0.99668–1.02783) | 32.08× |
| Spin-1 M180 q8 | 50.010 | 50.269 | 1.761 | 1.00752 (0.99411–1.01695) | 28.81× |

The current scalar-analysis route is 9.6–15.1% faster across the four cells. The spin-1 cells are near parity, with median changes from 0.4–1.5% slower. This is a three-triplet triage screen, not a confidence interval. All current outputs were bitwise identical to the parent in all 24 triplets. Current/DUCC relative-L2 errors were unchanged from parent: 1.82936e-5 / 4.29514e-7 max absolute at full scalar mmax, 2.20526e-5 / 9.05711e-7 at reduced scalar mmax, 1.56935e-5 / 2.23052e-7 at full spin-1, and 1.17229e-5 / 3.50410e-7 at reduced spin-1.

All 72 workers passed finite-output, repeated-call, and workspace-reset checks. Per-worker warm batch samples, fixture/output hashes, exact worker/build hashes, and triplet results are preserved in triplets.json, triplets.csv, and provenance.json. screen-runner.py contains the reusable runner; its repository and scratch roots are supplied through environment variables.

## Scope

This updates the eight scalar/spin-1 analysis cells only. The four scalar-synthesis cells were separately measured at current source in the vector-fusion screen; the f32 spin-1 synthesis code paths were unchanged from the earlier broad baseline. The remaining DUCC gaps are substantial, and these T720 numerical errors do not establish new acceptance tolerances.

## Scratch cleanup

After verifying the report hashes, removed only the owned experiment scratch (measured as 1.6 GiB by du) and its clean registered parent worktree. The scratch and worktree registration are absent, and no experiment workers remain. Available temporary space increased by 1,616,261,120 bytes and free inodes increased by 61,475.
