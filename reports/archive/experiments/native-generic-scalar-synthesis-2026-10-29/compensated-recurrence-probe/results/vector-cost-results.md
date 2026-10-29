# AVX2 vector cost screen

Status: passed test-only timing probe. This is a kernel screen, not whole-transform evidence.

- Native f32 AVX2 width: 8 lanes; 16 orders; 353–360 degree pairs per order; one equatorial input repeated across lanes.
- 15 interleaved single-thread pairs, 16 sweeps per path. CPU affinity was not pinned.
- Baseline median: `0.098 ms`; compensated median: `1.677 ms`; paired median ratio: `17.23983144x` (rounded `17.240x`).
- Sorted paired ratios: `14.95345272, 16.11453876, 16.49152444, 16.85338010, 17.10918643, 17.13168651, 17.13507713, 17.23983144, 17.27296453, 17.32769013, 17.33770335, 17.34008857, 17.51724030, 17.59365690, 18.00225365`.
- The generated vector callback used a 96-byte stack frame. The compensated one-pair loop had 116 instructions (40 packed adds, 48 packed subtracts, 9 packed multiplies, 4 packed FMA residual operations); the ordinary loop had 19 instructions, including 5 packed FMAs.

The result reflects repeated product-residual recovery and renormalization for the leading contraction and state update, plus double-single sums. It does not measure full-transform time or q8 scaling. Production dispatch remains unchanged; a cheaper residual correction must first preserve accuracy and reduce vector cost.
