# CC T720 reduced synthesis q1 route profile

## Scope and provenance

This diagnostic profiles the current M6 gap cell: f64 spin-1 synthesis on exact CC geometry with `lmax=720`, `mmax=180`, `ntheta=722`, `nphi=378`, and `phi0=0.23`. The Rust source was clean at `f2f813fd51c58b52a5991e666c92d83b3286bf7f`; the production file `src/transform/spin/synthesis.rs` has SHA-256 `8ef50c59d134f8c816793d8567198ff4120919503f9c243a8984ce6d13fef4e1`.

It ran the existing ignored test helper `profile_spin1_standard_synthesis_phases` with Rust 1.99.1 / LLVM 23.1.1, release `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, all features, and one Rayon thread. The helper generated deterministic coefficients from its built-in seed, warmed three calls, then recorded three profile calls. Test binary SHA-256: `6bde446901d25c045c09f3e050e68b565144436e0487d7b2bb2f05b358143ba7`. The selected helper passed (one test; 336 filtered out).

The executed command was `cargo test --release --locked --all-features --lib profile_spin1_standard_synthesis_phases -- --ignored --nocapture --test-threads=1` with the profile geometry and `RUSTFLAGS` listed above. The temporary full log SHA-256 was `ab24833e22631e192b0b7f7df413201b201a90c4f3e3e72f62fcdba9f7b48ad9`; it was removed after the compact CSV and report matched the recorded output.

This is source-route timing with test instrumentation. It does not use the M6 fixture, compare outputs with DUCC, collect scheduler-wait telemetry, or establish a new whole-transform timing. The current matched M6 benchmark remains Rust 30.006 ms / DUCC 18.593 ms, ratio 1.616×, with about 10% Rust scheduler wait and a three-pair range of 1.591–1.658×.

## Phase observations

| Repetition | Total ms | Latitude and setup ms | Order recurrence and spectral accumulation ms | Two inverse FFTs and writeback ms |
|---:|---:|---:|---:|---:|
| 0 | 28.191018 | 0.067869 | 27.047794 | 1.054526 |
| 1 | 29.344230 | 0.066145 | 28.218420 | 1.028906 |
| 2 | 28.038709 | 0.073930 | 26.966942 | 0.967620 |

Phase medians are 28.191018 ms total, 0.067869 ms setup, 27.047794 ms recurrence/spectral accumulation, and 1.028906 ms inverse FFT/writeback. Instrumented totals are descriptive and must not be compared directly with the M6 benchmark timings.

## Routes and counters

The route counts were stable across all three calls:

- 1,784 blocks were eligible for the paired kernel; 1,736 completed the ordinary fused path.
- There were 305 fallback entries: 191 staged transition successes and 114 per-degree blocks.
- All 114 per-degree blocks attempted and passed the polar split; there were no polar split declines.
- The profile recorded 36,172,480 eligible lane-pair positions and 35,263,168 ordinary fused lane-pair positions.
- The serial route allocated no scalar coefficient cache and no order-major scratch.

Worker-sum inner timers per call were 0.546–0.551 ms for order factors, 0.253–0.263 ms for paired coefficient preparation, 11.600–12.074 ms for fused recurrence/accumulation, 10.495–11.193 ms for scaled/per-degree fallback, and 0.119–0.123 ms for coefficient finite scans. These timers measure selected subregions inside the outer phase; they do not partition the whole phase, so do not sum them with its timing.

The measured phase is recurrence and spectral accumulation. Both inverse longitude FFTs and map writeback take about 1 ms together. CC resampling is not used for synthesis on this path. Therefore, another CC resampling, FFT, or coefficient-scan experiment is not supported by this profile.

The earlier reduced reflected scalar component route is not a valid retry unchanged: its reduced-order T720 version failed an existing test. Its full-order CC q8 screen had no repeatable end-to-end gain. A next production candidate needs a distinct recurrence/dataflow mechanism, exact-parent q1/q8 timing, numerical comparisons, and fallback/adjoint checks.

Compact per-call output is in [the route-profile CSV](cc-t720-reduced-q1-route-profile.csv). After verifying the report and CSV, cleanup removed only scratch root `rustsht-m6-cc-q1-profile.lI3xYv`: 1,245 files, 120,484 KiB allocated and 120,073,510 apparent bytes. The selected temporary filesystem gained 123,375,616 available bytes and 1,519 free inodes, reaching 949,016. The raw profile log and experiment-local Cargo/build artifacts were removed; no worker or worktree remained.
