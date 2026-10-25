# Performance checkpoints — rolling, five meaningful entries maximum

- **2026-10-25 20:15 UTC — VALIDATED: B30 rejected for production; CI green on `dbb4bacd`.** Local workflow checks passed: formatting, workspace/all-targets/all-features check, strict Clippy, Rust tests (**290 unit + 4 extreme + 11 f32 + 1 workspace passed; 37 ignored**) and Python (**20 passed**). GitHub Actions run [40229003400](https://github.com/mwyau/RustSHT/actions/runs/40229003400) completed successfully on Ubuntu, Windows, macOS, and Python. For the corrected opt-in B30 candidate `c52d4118` against exact parent `95125789`, Callgrind whole-program Ir rises **17.8% q1 / 17.7% q8** (5.737→6.757B / 5.803→6.827B), despite a **44.8%** reduction in eight-vector reduction groups per channel. Three interleaved parent/candidate pairs show q1 median ratios **1.142× T511, 1.186× T720, 0.996× T1023** and q8 **1.117× T511, 1.230× T720, 1.301× T1023**; runqueue contention makes T1023 q1 and the q8 estimates noisy, but T720 slows in every pair and Ir increases materially. Numerical checks passed (candidate/parent T720 relative L2 `1.06e-15`; candidate/DUCC `4.76e-14`; T511/T1023 q1/q8 remained within existing tolerances). **Decision: do not promote B30; keep the implementation explicitly opt-in and retain its block-shape/fallback tests and exact source in Git for reproducibility.** Next: return to the ranked T720 analysis instruction attribution; no performance-status snapshot update because no gain was accepted.







**Only recent execution notes.** Accepted values live in [PERFORMANCE_STATUS.md](../PERFORMANCE_STATUS.md); T1023 q1 attribution lives in [T1023_Q1_ROOT_CAUSE.md](T1023_Q1_ROOT_CAUSE.md). Older entries remain in Git history.
