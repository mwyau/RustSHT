// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-FileCopyrightText: 2010-2026 Max-Planck-Society (DUCC FFT pass equations)
// SPDX-License-Identifier: BSD-3-Clause
//
// This kernel specializes the symmetric general-prime equations in DUCC
// fft1d_impl.h (`cfftpg::exec_`) at
// c1ed01b8fda69d2a378190f0915f1a19762dddbf.

use std::arch::x86_64::*;

use num_complex::Complex;

use crate::simd::ComplexGeneralPrimeIdo1Params;

const RADIX: usize = 19;
const HALF: usize = RADIX / 2;

/// Executes a radix-19 general-prime pass over pairs of outer groups.
///
/// # Safety
///
/// The caller must have detected AVX2 and provide disjoint input and output
/// slices of at least `19 * l1` complex values, `l1 >= 2`, a radix-19
/// phase table with at least 171 entries, and a roots slice containing every
/// indexed root. The inverse input must be in the deferred-normalization safety
/// range. The final odd group, if present, is left for the scalar caller.
#[target_feature(enable = "avx2")]
#[cold]
pub(super) unsafe fn execute(
    input: &[Complex<f64>],
    output: &mut [Complex<f64>],
    params: ComplexGeneralPrimeIdo1Params<'_, f64>,
) {
    let ComplexGeneralPrimeIdo1Params {
        l1,
        roots,
        root_stride,
        phase_indices,
        inverse,
        scale,
        ..
    } = params;
    debug_assert!(l1 >= 2);
    debug_assert!(root_stride > 0);
    debug_assert!(input.len() >= RADIX * l1);
    debug_assert!(output.len() >= RADIX * l1);
    debug_assert!(phase_indices.len() >= HALF * RADIX);
    debug_assert!(roots.len() > (RADIX - 1) * root_stride);

    let sign = if inverse { 1.0 } else { -1.0 };
    let mut cosines = [[0.0; HALF]; HALF];
    let mut sines = [[0.0; HALF]; HALF];
    for frequency in 1..=HALF {
        for branch in 1..=HALF {
            let phase = usize::from(phase_indices[(branch - 1) * RADIX + frequency]);
            let root = roots[phase * root_stride];
            cosines[frequency - 1][branch - 1] = root.re;
            sines[frequency - 1][branch - 1] = sign * root.im;
        }
    }

    let output_ptr = output.as_mut_ptr();
    let output_scale = _mm256_set1_pd(if inverse { scale } else { 1.0 });
    let imaginary_sign = _mm256_setr_pd(-1.0, 1.0, -1.0, 1.0);
    let vector_l1 = l1 - l1 % 2;

    for outer in (0..vector_l1).step_by(2) {
        let mut values = [_mm256_setzero_pd(); RADIX];
        for (branch, slot) in values.iter_mut().enumerate() {
            let first = input[outer * RADIX + branch];
            let second = input[(outer + 1) * RADIX + branch];
            *slot = _mm256_setr_pd(first.re, first.im, second.re, second.im);
        }

        let mut dc = values[0];
        let mut pairs = [_mm256_setzero_pd(); HALF];
        let mut differences = [_mm256_setzero_pd(); HALF];
        for branch in 1..=HALF {
            let pair = _mm256_add_pd(values[branch], values[RADIX - branch]);
            let difference = _mm256_sub_pd(values[branch], values[RADIX - branch]);
            dc = _mm256_add_pd(dc, pair);
            pairs[branch - 1] = pair;
            differences[branch - 1] = difference;
        }
        if inverse {
            dc = _mm256_mul_pd(dc, output_scale);
        }
        // SAFETY: the AVX2 caller validates the pair of output groups.
        unsafe { super::store(output_ptr, outer, dc) };

        for frequency in 1..=HALF {
            let mut positive = values[0];
            let mut negative = values[0];
            for branch in 1..=HALF {
                let coefficient = branch - 1;
                let cosine = _mm256_mul_pd(
                    pairs[coefficient],
                    _mm256_set1_pd(cosines[frequency - 1][coefficient]),
                );
                let imaginary_difference = _mm256_mul_pd(
                    _mm256_permute_pd(differences[coefficient], 0b0101),
                    imaginary_sign,
                );
                let sine = _mm256_mul_pd(
                    imaginary_difference,
                    _mm256_set1_pd(sines[frequency - 1][coefficient]),
                );
                positive = _mm256_add_pd(_mm256_add_pd(positive, cosine), sine);
                negative = _mm256_sub_pd(_mm256_add_pd(negative, cosine), sine);
            }
            if inverse {
                positive = _mm256_mul_pd(positive, output_scale);
                negative = _mm256_mul_pd(negative, output_scale);
            }
            // SAFETY: each pair occupies two adjacent outer output slots.
            unsafe {
                super::store(output_ptr, frequency * l1 + outer, positive);
                super::store(output_ptr, (RADIX - frequency) * l1 + outer, negative);
            }
        }
    }
}
