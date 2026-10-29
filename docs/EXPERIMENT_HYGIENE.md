# Experiment and scratch hygiene

These rules apply to builds, benchmarks, profiles, temporary reference
environments and isolated worktrees. The [active M6 plan](../plans/m6-native-generic-ducc-parity.md)
defines the experiment sequence. The primary session is the only worker; do
not delegate.

## Before an experiment

1. Pin the parent source, candidate patch, DUCC revision, fixture hashes,
   compiler/ISA flags, operation, warm-call boundary and q1/q8 thread counts.
   Define the minimum evidence to preserve before building.
2. Inspect Git status, worktrees, relevant processes, and available bytes and
   inodes (`df -h` and `df -i`) on the intended scratch filesystem.
3. Create a unique scratch root owned by this experiment. Keep its worktree,
   `CARGO_TARGET_DIR`, reference build, fixtures, binaries, logs, profiles and
   temporary environments under that root. Do not use the checkout's normal
   `target/` directory.
4. Build and benchmark sequentially with bounded resource use. Reuse source
   provenance and compact results, not long-lived experiment build trees.

## Preserve and clean after each experiment

1. Stop and join only processes started for this experiment.
2. Save durable evidence before deleting scratch: exact source/reference
   revisions, flags, fixture hashes, source patch or commit, correctness and
   reset results, paired whole-transform timings, dispersion and limitations.
   Store compact reports and data under `reports/`; larger unique evidence may
   use a documented persistent location. A `/tmp` path alone is not durable.
3. Verify saved files can be read. Then remove only this experiment's exact
   inactive scratch root and verify it is gone.
4. Preserve useful failed-candidate patches and conclusions. Do not retain
   disposable build products in the repository or clean shared caches.
5. If ownership or uniqueness is uncertain, leave the data untouched. Never
   remove another worktree, untracked user work, shared toolchains or the only
   copy of benchmark evidence. Do not run broad deletion or worktree-prune
   commands.

Do not report cleanup unless reclaimed paths and space were checked. If
resources are insufficient, record the exact capacity issue and stop adding
build environments. On interruption, leave exact source SHAs, completed
validation, risks and the next action in the live report.
