# Native-f32 spin-1 analysis sequential-channel probe

## Change tested

Base source is `f01fb9b161769173397e50dae6c76aae97351c7f`; candidate source
diff SHA-256 is
`3baf60bc7efdcddc0fbeca27ea6aef455f5c545d8216ffd2419f46194f8853b5`.
The probe adds a compile-time f32 path that accumulates Q and U sequentially
within each degree pair, keeping four AVX2 vector sums live at a time rather
than eight. The recurrence remains shared and advances once per pair; f64
retains the fused implementation. The exact source patch is
[candidate.patch.gz](candidate.patch.gz).

## GL T720 results

Three fresh-process paired screens compare candidate f32, parent f32, a
matched f64 worker and pinned native-f32 DUCC 0.42.0. Geometry, deterministic
fixtures, q1/q8 affinity, compiler and whole-call protocol match the
[current-source baseline](../native-f32-breadth-port-2026-10-30/README.md).
The candidate output maps are byte-identical to the parent.

| Cell | Candidate f32 ms | Parent f32 ms | Matched f64 ms | DUCC f32 ms | Candidate/f64 | Candidate/DUCC | Candidate/parent |
|---|---:|---:|---:|---:|---:|---:|---:|
| Full q1 | 130.047 | 129.476 | 36.725 | 29.261 | 3.541x | 4.447x | 1.005x |
| Full q8 | 17.223 | 17.207 | 5.358 | 4.148 | 3.201x | 4.159x | 1.015x |
| Reduced q1 | 55.238 | 55.401 | 15.210 | 11.793 | 3.632x | 4.684x | 1.002x |
| Reduced q8 | 7.514 | 7.495 | 2.188 | 1.741 | 3.414x | 4.317x | 1.006x |

This probe does not improve whole-call performance. All four cells remain well
above the 2x f64 interim gate and are slower than the parent by 0.2–1.5%.

## Executed code and blocker

The measured binary SHA-256 is
`a501424e87ac776772899770d41e6d53ca2817b6aefad7fd38acfe8fb8466eed`; the
parent f32 binary is
`92262258bc39f87ad37495d4d85ec2b6cb98a4ec039d428715c5de9ce0b4f2d8`, and
DUCC is `a191b94696361fbbb2ca665715870024243c5fbc9577f6a13cccfe0206be18c8`.
The f64 candidate-source worker checksum matches the base worker for all four
cases; details are in [f64-source-equivalence.json](f64-source-equivalence.json).

The summarized AVX2 instruction and stack-operand counts are in
[assembly-summary.txt](assembly-summary.txt); raw host-specific disassembly
was excluded from the archive. The ordinary candidate kernel has no YMM stack operands; the
checked candidate path has three, versus four in the parent. The sequential
candidate grows the ordinary function from 73 to 99 YMM instructions and the
checked function from 162 to 188. The spill reduction is too small to affect
whole-call time. A previous unpinned phase profile on the same source family
put recurrence and accumulation at roughly 94–96% of the spin-analysis call;
that diagnostic used a different longitude length and is qualitative only.
The remaining gap needs an algorithmic reduction in recurrence/accumulation
work or scaled-path cost, not another register-count-only variation.

Candidate-vs-DUCC random-fixture relative L2 remains `1.84695e-5` full and
`2.17137e-5` reduced; this is not independent accuracy acceptance. Existing
f32 spin-operation, adjoint, T511 reference and workspace-reset tests pass.
This performance probe is rejected and its source remains isolated.

Sanitized pair data, retaining timings, outputs and code hashes, is in
[results-3pair.json](results-3pair.json).
