# Git identity normalization — 2026-10-30

This archive records the owner-authorized identity-only history rewrite on `dev`.
The original remote `dev` tip was `97400e691c519b2d559e0ae91af4855c1127b503`. The pre-rewrite local tip was `22224a282590ca5f2f1621670e75a25c4b4c1d30`; the rewritten tip before this archive commit was `16772df244c8aba5876977e8ce8299fd5263ba8b`.

The audit found 32 commits with the diagnostic author or committer identity in `dev` and 40 across the selected local development branches/worktree snapshots. The rewrite corrected 80 identity fields. Propagated parent changes altered 51 commit IDs; all other mapped IDs are unchanged. It did not change trees, messages, parent ordering/topology, author or committer dates, or unrelated identities. No affected history was reachable from `main`, tags, or another remote branch. The remote-tracking `origin/dev` ref remained at the recorded original SHA through local verification.

`commit-map.tsv` maps every old commit reachable from the 21 selected local branch refs and detached worktree heads to its rewritten SHA. It contains 1995 rows (51 changed IDs). Its SHA-256 is `6213fa109f7bcd4a3304e1c2d36b0c97ca087be0c98479096665c64baad8ba90`. The callback source used for the rewrite had SHA-256 `fbe222dae07a1f84a4a67a77b14f39a116b795db1e65e4ed5bc1965c4b817db8`. The git-filter-repo version identifier was `a40bce548d2c`; the selected refs and callback are preserved in the local audit record, and `.git/filter-repo` retained the raw tool mapping.

All 21 affected local branch tips and 24 detached worktree HEADs were mapped. All 105 existing worktrees were checked after the rewrite; the 88 dirty worktrees retained identical porcelain status hashes. The pre-rewrite local audit and exact status hashes remain in the operator's external audit directory for recovery.

Historical benchmark source SHAs remain unchanged in experiment records: those identify the source revision used to build each measured binary. Use this map only to resolve the equivalent rewritten Git commit; it does not change benchmark provenance.
