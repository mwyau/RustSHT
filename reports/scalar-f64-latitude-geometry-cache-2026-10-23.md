# f64 scalar latitude geometry cache — 2026-10-23

## Change

`ShtPlan` now precomputes `sin(theta)`, `cos(theta)` and scalar polar order limits for f64 scalar plans. CC/F1 plans also cache the prepared-grid values. Transform calls copy these immutable values into their existing workspace instead of repeating trigonometry and cutoff calculation. f32 and non-scalar plans keep the existing path. The new test checks exact cached values on GL, CC and F1, including prepared CC/F1 geometry.

## Matched T720 full-order comparison

Baseline source: `66f70711b01c9f2e2de72fae4a13b039b940bcf6`. Candidate: that source plus the change above. Cases used `lmax=mmax=720`, `phi0=0.23`, `nphi=1458` and `even_smooth` longitudes; CC used `ntheta=722`, GL/F1 used `ntheta=721`. Both Rust builds used `-C target-cpu=x86-64-v3`; the pinned DUCC reference was commit `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, built with `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` (driver `-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3`). Rust binary SHA-256 was `5005b3d07e4537c2d38924f8732dfb2db17b5cec91d36bd5b272dd7b38c865e6` at baseline and `f46f7045761a3194f73e523567ac5c1bb31fb58bc325c6bba57704d0f71d1be7` for the candidate; the DUCC binary was identical across runs (`db1f703671b712a8b8b45ab71fbb550c6884879a0808a3db70608db03ef6ec6`).

The benchmark used `scripts/benchmark_ducc_full_transform_parity.py` (harness SHA-256 `4d753f915f80bcc4476cdaeb3b563c8c83fddc933f3ec8ed7073d0fb1c8ca97a`) with `--lmax 720 --full-mmax-only --longitude-mode even_smooth --operation analysis --operation synthesis --threads 1 --threads 8`, once per grid. The runner calibrated short calls to 20–100 ms and collected 10 batches per backend per process pair, across three independent process pairs. q1 was pinned to one physical core and q8 to the same eight physical cores. All rows were within the batch-duration target; numerical and reset checks passed. Maximum relative L2 error was `6.45e-14` for CC, `4.63e-14` for GL and `5.98e-14` for F1.

The table reports warm latency in milliseconds and Rust/DUCC ratios. Source versions were measured in separate runs, not interleaved, so differences are directional.

| Grid | Operation | Threads | Rust ms, baseline → candidate | DUCC ms, baseline → candidate | Rust/DUCC, baseline → candidate |
|---|---|---:|---:|---:|---:|
| CC | Analysis | 1 | 39.342 → 39.705 | 56.870 → 57.298 | 0.685 → 0.691 |
| CC | Analysis | 8 | 6.543 → 6.492 | 10.310 → 10.277 | 0.643 → 0.637 |
| CC | Synthesis | 1 | 20.619 → 19.964 | 19.185 → 19.186 | 1.073 → 1.043 |
| CC | Synthesis | 8 | 2.812 → 2.773 | 2.696 → 2.685 | 1.040 → 1.030 |
| GL | Analysis | 1 | 14.963 → 14.878 | 14.631 → 14.656 | 1.023 → 1.019 |
| GL | Analysis | 8 | 2.245 → 2.200 | 2.139 → 2.122 | 1.037 → 1.033 |
| GL | Synthesis | 1 | 20.167 → 20.174 | 19.103 → 19.227 | 1.060 → 1.046 |
| GL | Synthesis | 8 | 2.805 → 2.758 | 2.693 → 2.687 | 1.042 → 1.029 |
| F1 | Analysis | 1 | 40.186 → 39.440 | 58.431 → 57.474 | 0.686 → 0.683 |
| F1 | Analysis | 8 | 7.226 → 6.883 | 10.482 → 10.392 | 0.689 → 0.659 |
| F1 | Synthesis | 1 | 20.116 → 19.910 | 19.424 → 19.141 | 1.040 → 1.034 |
| F1 | Synthesis | 8 | 2.836 → 2.770 | 2.702 → 2.689 | 1.051 → 1.028 |

The clearest result is CC q1 synthesis: Rust fell 3.2% with nearly unchanged DUCC latency, and the per-run Rust/DUCC ratio ranges did not overlap (`1.055–1.080` before, `1.028–1.047` after). F1 q8 synthesis fell 2.3%, with ratio ranges `1.046–1.058` and `1.026–1.034`. Other changes are small or have overlapping run ranges. CC q8 synthesis remains about 3.0% slower than DUCC; F1 q1 synthesis remains about 3.4% slower. This selected T720 result does not establish broad scalar parity.

## Validation

- `cargo test --release --lib -- --test-threads=1`: 276 passed, 36 ignored.
- `cargo test --release --no-default-features --lib -- --test-threads=1`: 224 passed, 17 ignored.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
