# Independent review — advisory, subordinate to owner instructions

**Status (2026-10-22): RESOLVED — no active steering request.**  
**Prior review:** `08b938a1d9f89455b9c7a0b8bd81776988ea314a`.  
**Owner clarification:** the pyspharm numerical and Python-compatibility code on `opt/sht-performance-parity` was **intentionally integrated**. That overrides the previous independent review's inference from Plan 18's earlier isolation wording.

## Resolved provenance concern

The previous note incorrectly requested a new owner decision about whether to keep the pyspharm files on the performance branch. **Do not act on that request, remove or relocate the integration, or block Plan 16.** The integration decision is confirmed. Earlier [Plan 18](plans/18-pyspharm-and-native-ffi-compatibility.md) statements that the performance branch was untouched refer to the prior isolated development stage and need not be interpreted as a current prohibition.

The separate scientific compatibility limitations recorded in Plan 18 remain relevant **unless new measurements supersede them**: regular-grid maximum-truncation parity is not established (small-grid examples report relative L2 `3.146e-2` at `nlat=7, T6` and `6.702e-2` at `nlat=8, T7`); the `pyspharm-rs` distribution does not automatically satisfy windspharm's `pyspharm>=1.0.9` installation metadata. These are **documented limitations, not a request to reopen Plan 18 immediately**. No PyPI publication was authorized by the clarification.

The guarded paired odd-length FFT is likewise intentionally integrated. [Current validation](reports/paired-odd-longitude-fft-production-validation-2026-10-22.md) reports paired/independent whole-transform geometric mean latency ratios for full-order odd-longitude cases of 0.963/0.943 (analysis q1/q8) and 0.905/0.945 (synthesis q1/q8). Its numerical fallback guards do not imply that the entire fast path is slower. Some low-`nphi`, reduced-order q1 analysis cases were 5–11% slower. Those measurements are a bounded screen with scheduling and run-order limitations. **No new FFT eligibility or guard change is authorized.**

## Current direction

No independent redirection is warranted from these previously reviewed findings. Plan 16 and the owner-directed scalar dispatch/paired-FFT/full-sweep checkpoint are complete; continue Plan 17 as directed in [AGENTS.md](AGENTS.md).

Future independent reviewers should replace this resolved status only for a fresh, material, evidence-backed finding; otherwise leave it unchanged. This note is advisory and always subordinate to explicit owner instructions.
