# Plan 14 T16 analysis cutoff data

This directory keeps compact, derived timing tables and build provenance for six f64 GL q1 analysis cases: T15, T16 and T31 at odd-minimum and even-smooth longitude lengths. Each case used three interleaved Rust/DUCC process pairs and 30 timed batches per backend. The 75 ms target produced backend batches of 42.47–71.94 ms (baseline) and 42.47–68.53 ms (candidate). Plans/workspaces were reused; setup and first-call fields were separate. Inputs were deterministic, reset consistency and output checks passed, and q1 was pinned to physical CPU 0. Per-process raw captures and host diagnostics remain local and are not part of this compact dataset.

Rust 1.99.1 / LLVM 23.1.1 used strict FP, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, and runtime Pulp V3. DUCC 0.42.0 is pinned at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, GCC 15.2, with SHT sources built `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` and the benchmark driver strict. The same DUCC binary and fixture formulas were used in both runs.

Baseline Rust binary was built from clean commit `2dadb0c07d776e0f7a6679af8c28bb908082c443`; candidate binary contains only the threshold/test changes in source commit `830ecc63eb9df829bebefb7c7a908bba4e20242e`. SHA-256 values are in `build-manifest.json`. The raw benchmark JSON says `rust_worktree_clean=false` for both because the shared checkout already held the candidate source when the saved baseline binary was run for the 75 ms capture; use the manifest’s independently recorded binary/source provenance. The baseline binary had been produced from the clean checkout before that source edit.

`comparison.csv` joins the six cases, `cold-followup-summary.csv` contains the ten-pair first-call/warm summary, and `phase-summary.csv` gives the three-process Rust stage profile distributions. The 50 ms phase target produced 30 batches per process with actual batches above 20 ms. Raw phase and host logs remain local and untracked.

Reproduction: build Rust release example binaries at baseline and candidate commits with `RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs'`, saving each binary separately; build the pinned DUCC driver using `reports/data/ducc-full-transform-parity-2026-10-20/CMakeLists.txt`. Run `scripts/benchmark_ducc_full_transform_parity.py` with `--ducc-src <pinned-checkout> --lmax 15 --lmax 16 --lmax 31 --full-mmax-only --longitude-mode odd_minimum --longitude-mode even_smooth --operation analysis --threads 1 --no-report --skip-build`, selecting the saved binary via `CARGO_TARGET_DIR`. Set the in-memory runner `TARGET_BATCH_MS=75.0` before `main()` (the normal CLI default is 25 ms). For phase profiles, run the ignored `plan14_analysis_phase_batch_profile` release test with `RUSTSHT_PROFILE_Q=1`, `LMAX=31`, `MMAX=31`, `NTHETA=32`, `NPHI=64`, `BATCHES=30`, `BATCH_MS=50`, pinned to CPU 0.

`cold-followup/` adds a 10-process-pair T15/T16 even-smooth cold/warm check.
Each Rust and DUCC process actually emitted 10 timed batches, so this follow-up
has 100 batches per backend/case; every observed batch was 27.3–67.5 ms.
The copied JSON `protocol.batches_per_process` field says 3 because the
Python in-memory setting changed only that metadata field, not the example
worker's hardcoded 10 timed batches. The `samples[*].batch_ms` arrays and
`cold-followup-summary.csv` are authoritative for counts and timings.
