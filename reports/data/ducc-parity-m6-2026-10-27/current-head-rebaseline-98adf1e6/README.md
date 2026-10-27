# M6 exact current-head rebaseline — 2026-10-27

## Scope

This is a fresh matched warm whole-transform baseline at RustSHT `98adf1e6b5334021bfe6ae9abb618c6be167a822` (`dev`) against pinned DUCC 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. It records 22 cells: six selected f64 spin-1 controls and the full/reduced native-f32 GL T720 scalar/spin-1 A/S q1/q8 matrix. It is a baseline, not a parent/candidate comparison.

Each cell used three interleaved fresh-process Rust/DUCC pairs on the same deterministic fixture. The worker times warm calls using reusable Rust plans/workspaces; process startup, setup, and fixture I/O are outside the timed batches. The f64 target was 25 ms per batch and f32 10 ms; both use 10 calibrated batches below 100 ms and 5 otherwise. Three pairs give a triage range, not a confidence interval. q1 pinned to one physical core; q8 used eight distinct physical cores. The f64 monitor frequency samples come from a separate monitor CPU; f32 frequency samples come from worker-affined CPUs.

Rust used rustc/Cargo 1.99.1 and LLVM 23.1.1. The f64 worker used release + `parallel` and `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. The f32 worker used release + `parallel` with `RUSTFLAGS` unset; it reported Pulp V3 and eight f32 lanes. DUCC SHT sources used `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; drivers used `-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3`. Host: AMD Ryzen 9 5950X; GCC 15.2.0.

The bounded selector and output-hash instrumentation patch is preserved in gzip form at [`bounded-harness.patch.gz`](bounded-harness.patch.gz); its decompressed SHA-256 is `d0409db8337f476ddfb629c1de1f31957f5716ef2a05ee69e8486877f0f919fb`. Executed f64/f32 runner SHA-256 values are `7db304a9004f4c31604b861454699d9b2a496f73cea6577165467b34ce6f6d02` and `bd8d919161c227670755e4829172dacc029c8fb63f98e3dcb9009f9d8fb8ed31`; common runner SHA-256 is `8a364bd8f6caf1eb5c98f832cd5a28aa7446145b1a3455996cda5f35bcd86843`.

## f64 cells

| Workload | Rust ms | DUCC ms | Rust/DUCC | Pair range | Max relative L2 | Status |
|---|---:|---:|---:|---:|---:|---|
| CC T720 m180 spin-1 synthesis q1 | 30.010 | 18.644 | 1.608× | 1.600–1.704× | 3.62e-13 | **GAP** |
| GL T1023 m1023 spin-1 synthesis q1 | 87.990 | 121.330 | 0.734× | 0.721–0.745× | 2.60e-13 | **PASS** |
| GL T1023 m1023 spin-1 synthesis q8 | 14.080 | 19.775 | 0.715× | 0.696–0.716× | 2.60e-13 | **PASS** |
| GL T720 m180 spin-1 synthesis q8 | 4.255 | 4.677 | 0.911× | 0.902–0.912× | 2.27e-13 | **PASS** |
| GL T720 m720 spin-1 analysis q1 | 36.569 | 29.234 | 1.249× | 1.234–1.271× | 4.76e-14 | **GAP** |
| GL T720 m720 spin-1 analysis q8 | 5.372 | 4.260 | 1.298× | 1.220–1.307× | 4.76e-14 | **GAP** |

All six f64 cells passed finite, repeat, and reset checks. The current GL T720 full-mmax spin-1 analysis gap is 1.249× q1 and 1.298× q8; all observed pair ratios exceed 1. The q1 Rust scheduler wait was 6.4–6.7%, while DUCC wait was below 0.1%; q8 Rust wait was 0.4–1.1%. The T1023 synthesis control remains faster than DUCC at both thread counts. The reduced over-resolved GL synthesis cell remains faster; CC reduced synthesis q1 remains a gap and its Rust workers saw about 9.8–10.3% scheduler wait.

The Phase B GL T720 analysis Callgrind/source attribution applies to this source: `git diff 26941cf4..98adf1e6` contains no `src/`, `Cargo.toml`, or `Cargo.lock` changes. Its q1/q8 whole-call counts were 1.4899×/1.4990× DUCC Ir, with the reflected fused recurrence/accumulation as the dominant Rust stage. See the [M6 report](../../../DUCC_PARITY_M6.md#phase-b-profile--current-source-feb46e5f) and [profile CSV](../../ducc-parity-m6-2026-10-27-profile.csv).

## Native f32 cells

| Workload | Rust ms | DUCC ms | Rust/DUCC | Pair range | Max relative L2 | Status |
|---|---:|---:|---:|---:|---:|---|
| GL T720 m720 spin-0 analysis q1 | 605.946 | 14.084 | 42.622× | 40.394–43.812× | 1.83e-05 | **GAP** |
| GL T720 m720 spin-0 analysis q8 | 79.290 | 2.078 | 37.945× | 37.814–38.393× | 1.83e-05 | **GAP** |
| GL T720 m720 spin-0 synthesis q1 | 1066.473 | 19.079 | 55.222× | 54.355–56.667× | 1.72e-04 | **GAP** |
| GL T720 m720 spin-0 synthesis q8 | 373.030 | 2.660 | 139.613× | 139.422–140.555× | 1.72e-04 | **GAP** |
| GL T720 m720 spin-1 analysis q1 | 1098.267 | 28.707 | 38.237× | 38.219–38.429× | 1.57e-05 | **GAP** |
| GL T720 m720 spin-1 analysis q8 | 147.914 | 4.091 | 35.509× | 35.215–35.683× | 1.57e-05 | **GAP** |
| GL T720 m720 spin-1 synthesis q1 | 1508.019 | 40.544 | 37.260× | 36.196–37.343× | 1.43e-04 | **GAP** |
| GL T720 m720 spin-1 synthesis q8 | 274.675 | 5.642 | 48.588× | 48.051–48.773× | 1.43e-04 | **GAP** |
| GL T720 m180 spin-0 analysis q1 | 227.693 | 5.699 | 40.033× | 39.348–40.147× | 2.21e-05 | **GAP** |
| GL T720 m180 spin-0 analysis q8 | 29.945 | 0.894 | 33.575× | 33.245–34.667× | 2.21e-05 | **GAP** |
| GL T720 m180 spin-0 synthesis q1 | 410.869 | 8.305 | 49.399× | 49.113–50.467× | 2.73e-04 | **GAP** |
| GL T720 m180 spin-0 synthesis q8 | 150.199 | 1.213 | 123.947× | 123.561–124.539× | 2.73e-04 | **GAP** |
| GL T720 m180 spin-1 analysis q1 | 406.805 | 11.763 | 34.502× | 34.281–35.058× | 1.17e-05 | **GAP** |
| GL T720 m180 spin-1 analysis q8 | 53.927 | 1.770 | 30.391× | 29.830–31.242× | 1.17e-05 | **GAP** |
| GL T720 m180 spin-1 synthesis q1 | 614.114 | 17.687 | 34.572× | 34.400–34.797× | 1.41e-04 | **GAP** |
| GL T720 m180 spin-1 synthesis q8 | 123.567 | 2.509 | 49.104× | 40.041–49.384× | 1.41e-04 | **GAP** |

All 16 f32 cells passed finite, repeat, and reset checks. Ratios span 30.391×–139.613×; every cell is a clear performance GAP. The largest q8 case is full-mmax scalar synthesis (139.613×), and reduced-mmax scalar synthesis q8 is 123.947×. The observed DUCC-relative L2 errors span 1.17e-5–2.73e-4; the benchmark reports these values but has no T720-specific error threshold.

Independent existing f32 integration coverage passed with `cargo test --locked --release --all-features --test f32` (11 passed, 1 ignored, 0 failed). Those tests cover all-grid scalar references and adjoints, spin 1–4 operations/adjoints, GL spin-1 synthesis, and T511 analysis. The opt-in T1023 comparison is ignored. This is broad reference coverage, not a T720-specific numerical acceptance claim.

## Evidence

- [`current-head-rebaseline.json`](current-head-rebaseline.json) — machine-readable 22-cell results, pair timings, hashes, errors, and worker diagnostics.
- [`../ducc-parity-m6-2026-10-27.csv`](../../ducc-parity-m6-2026-10-27.csv) — persistent M6 per-cell ledger; these rows are keyed by current source SHA.
- [`bounded-harness.patch.gz`](bounded-harness.patch.gz) — gzip-preserved scratch-only selector and output-hash patch; decompresses to the SHA-256 recorded above.
- No production source changed in this baseline. The immediate GL T720 f64 analysis target and all measured f32 cells remain open performance gaps; M6 remains in progress.

## Experiment cleanup

The experiment-owned scratch directory was removed after the report, machine-readable results, ledger rows, and harness patch were verified. The measured `/tmp` free-space change was +990,162,944 bytes and +6,930 inodes; no benchmark worker or temporary worktree registration remained.
