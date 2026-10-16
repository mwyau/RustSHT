// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-FileCopyrightText: 2010-2026 Max-Planck-Society (DUCC FFT pass equations)
// SPDX-License-Identifier: BSD-3-Clause
//
// This AVX2 tail kernel uses the radix-eleven equations in DUCC
// fft1d_impl.h at c1ed01b8fda69d2a378190f0915f1a19762dddbf.

use std::arch::x86_64::*;

use num_complex::Complex;

use crate::simd::SimdPass;

const RADIX: usize = 11;

/// Executes the odd final `ido` value for pairs of outer groups.
///
/// # Safety
///
/// The caller must have detected AVX2 and provide disjoint radix-11 buffers
/// with odd `ido >= 3`, `l1 >= 2`, and lengths of at least `11 * l1 * ido`
/// complex values. The twiddle slice must contain at least `11 * ido`
/// complex values. This function writes the final inner value for the largest
/// even prefix of outer groups.
#[target_feature(enable = "avx2")]
#[inline(never)]
pub(super) unsafe fn execute(
    input: &[Complex<f64>],
    output: &mut [Complex<f64>],
    pass: &SimdPass<'_, f64>,
) {
    debug_assert_eq!(pass.radix, RADIX);
    debug_assert!(pass.ido >= 3 && !pass.ido.is_multiple_of(2));
    debug_assert!(pass.l1 >= 2);
    debug_assert!(input.len() >= RADIX * pass.l1 * pass.ido);
    debug_assert!(output.len() >= RADIX * pass.l1 * pass.ido);
    debug_assert!(pass.twiddles.len() >= RADIX * pass.ido);

    let input_ptr = input.as_ptr();
    let output_ptr = output.as_mut_ptr();
    let inner = pass.ido - 1;
    let vector_l1 = pass.l1 - pass.l1 % 2;
    let scale = _mm256_set1_pd(if pass.inverse {
        pass.inverse_scale
    } else {
        1.0
    });

    for outer in (0..vector_l1).step_by(2) {
        let mut values = [_mm256_setzero_pd(); RADIX];
        for (branch, slot) in values.iter_mut().enumerate() {
            let first_index = inner + pass.ido * (branch + RADIX * outer);
            let second_index = first_index + pass.ido * RADIX;
            // SAFETY: both indices are the final inner value for two adjacent
            // outer groups within the validated input slice.
            let first = unsafe { *input_ptr.add(first_index) };
            // SAFETY: the paired outer index is within the even prefix.
            let second = unsafe { *input_ptr.add(second_index) };
            *slot = _mm256_setr_pd(first.re, first.im, second.re, second.im);
        }

        // SAFETY: AVX2 was checked by the caller, and each vector lane holds
        // one independent radix-eleven input group.
        let transformed = unsafe { super::radix11_fixed(&values, pass.inverse) };
        for (branch, &result) in transformed.iter().enumerate() {
            let mut value = result;
            if branch != 0 {
                let twiddle = pass.twiddles[branch * pass.ido + inner];
                let twiddle = _mm256_setr_pd(twiddle.re, twiddle.im, twiddle.re, twiddle.im);
                // SAFETY: the AVX2 caller guarantees the target feature.
                value = unsafe { super::complex_mul(value, twiddle, pass.inverse) };
            }
            if pass.inverse {
                value = _mm256_mul_pd(value, scale);
            }
            let first_index = inner + pass.ido * (outer + pass.l1 * branch);
            // SAFETY: the two halves are the final inner value for adjacent
            // outer groups; validated dimensions keep both destinations in
            // the output slice.
            unsafe {
                _mm_storeu_pd(
                    output_ptr.add(first_index).cast::<f64>(),
                    _mm256_castpd256_pd128(value),
                );
                _mm_storeu_pd(
                    output_ptr.add(first_index + pass.ido).cast::<f64>(),
                    _mm256_extractf128_pd(value, 1),
                );
            }
        }
    }
}
