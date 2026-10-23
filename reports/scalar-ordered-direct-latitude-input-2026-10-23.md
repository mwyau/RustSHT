# Direct latitude inputs for ordered scalar synthesis — 2026-10-23

## Change

The ordered recurrence previously copied sine, cosine and north-ring indices into worker scratch for every order and latitude batch. It now reads sine/cosine from the existing per-call geometry arrays, derives each north-ring index from the batch start, and stages only an incomplete SIMD sine tail. The active recurrence and scaled fallback are unchanged. This removes repeated geometry packing from the hot loop.

## Validation

- `cargo test --release --lib scalar_ordered -- --test-threads=1`: 18 passed, 2 ignored.
- `cargo test --release --lib -- --test-threads=1`: 276 passed, 36 ignored.
- `cargo test --release --no-default-features --lib -- --test-threads=1`: 224 passed, 17 ignored.
- `cargo clippy --all-targets --all-features -- -D warnings`, formatting and `git diff --check`: passed.

The existing ordered CC/F1 tests cover exact grids, parallel q1/q8 agreement, reflected latitude tails, poisoned workspace reuse and extreme/subnormal fallback. The focused DUCC comparison had zero failure records and accepted reset checks. Its broader analytic/random sidecar is marked incomplete; the runner's random-output relative L2 was `6.45e-14` for CC, `3.87e-14` for F1 and `4.41e-14` for GL.

## Matched T720 full-order synthesis comparison

Cases used `lmax=mmax=720`, `phi0=0.23`, `nphi=1458`, even-smooth longitudes, and exact latitude counts (CC 722; F1/GL 721). The baseline executable is the geometry-cache source at commit `7e29b79e`, SHA-256 `f46f7045761a3194f73e523567ac5c1bb31fb58bc325c6bba57704d0f71d1be7`. The candidate is that source plus this change, SHA-256 `e158df8ee7283321f6e2946dc5cba5d5c6475d4273ec3308f6811ff9fc1a661c`. Both use the same pinned DUCC executable (`db1f703671b712a8b8b45ab71fbb550c6884879a0808a3db70608db03ef6ec6c`, source `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`). Rust uses `-C target-cpu=x86-64-v3`; DUCC uses the pinned x86-64-v3 build.

Each row has 30 calibrated batches per backend across three process pairs, q1 on one physical core and q8 on the same eight-core set. Every timed batch met the 20–100 ms target. Source runs were separate (the F1 order was reversed); treat before/after changes as directional. Values below are warm milliseconds and Rust/DUCC ratios.

| Grid | Threads | Rust ms, baseline → candidate | DUCC ms, baseline → candidate | Ratio, baseline → candidate | Paired-ratio p10–p90, baseline → candidate |
|---|---:|---:|---:|---:|---:|
| CC | 1 | 20.314 → 20.127 | 19.112 → 19.234 | 1.068 → 1.043 | 1.049–1.076 → 1.000–1.060 |
| CC | 8 | 2.779 → 2.714 | 2.700 → 2.684 | 1.028 → 1.014 | 1.026–1.029 → 1.002–1.017 |
| F1 | 1 | 20.030 → 19.248 | 19.155 → 19.101 | 1.044 → 1.011 | 1.040–1.051 → 1.000–1.018 |
| F1 | 8 | 2.753 → 2.752 | 2.692 → 2.691 | 1.024 → 1.025 | 1.016–1.037 → 1.012–1.025 |
| GL | 1 | 19.861 → 19.203 | 19.215 → 18.980 | 1.028 → 1.016 | 1.028–1.886 → 1.005–1.862 |
| GL | 8 | 2.799 → 2.724 | 2.696 → 2.692 | 1.036 → 1.011 | 1.025–1.065 → 1.002–1.014 |

The clearest signals are F1 q1 (`-3.9%` Rust time; nonoverlapping paired-ratio ranges), CC q8 (`-2.3%`; nonoverlapping ranges) and the GL q8 comparison (`-2.7%`; nonoverlapping ranges). CC q1 and F1 q8 ranges overlap; GL q1 has a large process outlier. Selected CC/F1 synthesis now ranges from `1.011x` to `1.043x` DUCC at T720, so this does not establish broad parity or remove the remaining q1 CC excess.

The benchmark used `scripts/benchmark_ducc_full_transform_parity.py` at harness SHA-256 `4d753f915f80bcc4476cdaeb3b563c8c83fddc933f3ec8ed7073d0fb1c8ca97a`, with `--skip-build`, `--lmax 720 --full-mmax-only --longitude-mode even_smooth --operation synthesis --threads 1 --threads 8`. Raw results and local CPU/runqueue telemetry remain untracked locally.

## T255 full-order holdout

The source pair was also checked at `lmax=mmax=255`, `nphi=512`, with exact CC (`ntheta=257`) and F1 (`ntheta=256`) grids. Each row again used 30 batches per backend across three process pairs. The runner reported maximum relative L2 `5.01e-15` for CC and `5.64e-15` for F1, with output-reset checks accepted. Its broader validation sidecar remains incomplete.

| Grid | Threads | Rust ms, baseline → candidate | DUCC ms, baseline → candidate | Ratio, baseline → candidate | Paired-ratio p10–p90, baseline → candidate |
|---|---:|---:|---:|---:|---:|
| CC | 1 | 1.529 → 1.448 | 1.416 → 1.418 | 1.082 → 1.024 | 1.066–1.114 → 1.019–1.026 |
| CC | 8 | 0.237 → 0.228 | 0.235 → 0.234 | 1.003 → 0.972 | 1.002–1.118 → 0.966–0.977 |
| F1 | 1 | 1.559 → 1.412 | 1.380 → 1.376 | 1.097 → 1.030 | 1.091–1.130 → 1.017–1.062 |
| F1 | 8 | 0.228 → 0.220 | 0.230 → 0.228 | 0.995 → 0.964 | 0.984–0.996 → 0.957–0.975 |

The q1 paired-ratio ranges do not overlap for either grid, and the candidate q8 medians are below DUCC. The host was contended: runqueue-wait p90 was about `20.8%` across these source runs, and the baseline CC q8 row had batches outside the target window. Treat this lower-degree set as directional. Together with T720, it shows no clear low-degree regression and places selected CC/F1 full-order synthesis within about 3% of DUCC or faster; CC T720 q1 remains the largest measured excess.
