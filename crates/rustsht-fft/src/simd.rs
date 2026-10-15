// SPDX-FileCopyrightText: 2010-2026 Max-Planck-Society (DUCC FFT pass equations)
// SPDX-FileCopyrightText: 2019 Peter Bell (DUCC FFT pass equations)
// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause
//
// The packed-real radix-four equations follow DUCC fft1d_impl.h at
// c1ed01b8fda69d2a378190f0915f1a19762dddbf.

use num_complex::Complex;
use num_traits::{Float, FromPrimitive};

pub(crate) type RealRadix4InteriorKernel<T> = unsafe fn(&[T], [&mut [T]; 4], usize, &[T]) -> usize;
pub(crate) type RealRadix2InteriorKernel<T> =
    unsafe fn([&[T]; 2], [&mut [T]; 2], usize, &[T]) -> usize;
pub(crate) type RealRadix2ForwardInteriorKernel<T> =
    unsafe fn([&[T]; 2], [&mut [T]; 2], usize, &[T]) -> usize;
pub(crate) type RealRadix4ForwardInteriorKernel<T> =
    unsafe fn([&[T]; 4], [&mut [T]; 4], usize, &[T]) -> usize;
#[derive(Clone, Copy)]
pub(crate) struct RealGeneralPrimeIdo1Params<'a, T> {
    pub(crate) l1: usize,
    pub(crate) radix: usize,
    pub(crate) cosines: &'a [T],
    pub(crate) sines: &'a [T],
    pub(crate) inverse: bool,
    pub(crate) scale: T,
}
pub(crate) type RealGeneralPrimeIdo1Kernel<T> =
    for<'a> unsafe fn(&[T], &mut [T], RealGeneralPrimeIdo1Params<'a, T>);
pub(crate) struct ComplexGeneralPrimeIdo1Params<'a, T> {
    pub(crate) l1: usize,
    pub(crate) radix: usize,
    pub(crate) roots: &'a [Complex<T>],
    pub(crate) root_stride: usize,
    pub(crate) phase_indices: &'a [u8],
    pub(crate) inverse: bool,
    pub(crate) scale: T,
}
pub(crate) type ComplexGeneralPrimeIdo1Kernel<T> =
    for<'a> unsafe fn(&[Complex<T>], &mut [Complex<T>], ComplexGeneralPrimeIdo1Params<'a, T>);
/// A fixed-radix AVX2 complex pass function pointer.
///
/// The ordinary pass pointer follows `avx2_pass`'s vector-width contract. A
/// low-`ido` pointer returned separately may use its own narrower contract.
pub(crate) type ComplexPassKernel<T> =
    unsafe fn(&[Complex<T>], &mut [Complex<T>], &SimdPass<'_, T>);

pub(crate) struct SimdPass<'a, T> {
    pub(crate) radix: usize,
    pub(crate) l1: usize,
    pub(crate) ido: usize,
    pub(crate) twiddles: &'a [Complex<T>],
    pub(crate) inverse: bool,
    pub(crate) inverse_scale: T,
}

/// Scalar types supported by the private runtime-dispatched FFT kernels.
pub(crate) trait SimdScalar: Float + FromPrimitive {
    const AVX2_COMPLEX_LANES: usize;

    fn avx2_available() -> bool;

    /// Returns the f64 radix-eight pass kernel for `ido == 1`, if supported.
    ///
    /// A returned pointer may be called only after runtime AVX2 detection and
    /// when `radix == 8`, `ido == 1`, and `l1 >= 2` is even. Its input and
    /// output must be disjoint and each contain at least `8 * l1` complex
    /// values. This contract is separate from the ordinary pass kernel's
    /// vector-prefix contract.
    fn avx2_ido1_radix8_pass_kernel() -> Option<ComplexPassKernel<Self>> {
        None
    }

    /// Returns the f64 radix-four pass kernel for `ido == 1`, if supported.
    ///
    /// The kernel operates on two adjacent outer groups and leaves an odd
    /// final group untouched for the scalar caller.
    fn avx2_ido1_radix4_pass_kernel() -> Option<ComplexPassKernel<Self>> {
        None
    }

    /// Returns the f64 radix-31 general-prime kernel for `ido == 1`.
    ///
    /// The caller must check the returned kernel's radix, pass layout, input
    /// safety range, and runtime AVX2 availability before calling it.
    fn avx2_general_prime_ido1_kernel() -> Option<ComplexGeneralPrimeIdo1Kernel<Self>> {
        None
    }

    /// Returns the inverse radix-four interior kernel when AVX2 is available.
    fn real_radix4_interior_kernel() -> Option<RealRadix4InteriorKernel<Self>> {
        None
    }

    /// Returns the real general-prime `ido == 1` kernel when supported.
    fn real_general_prime_ido1_kernel() -> Option<RealGeneralPrimeIdo1Kernel<Self>> {
        None
    }

    /// Returns the forward radix-four interior kernel when AVX2 is available.
    fn real_radix4_forward_interior_kernel() -> Option<RealRadix4ForwardInteriorKernel<Self>> {
        None
    }

    /// Returns the inverse radix-two interior kernel when AVX2 is available.
    fn real_radix2_interior_kernel() -> Option<RealRadix2InteriorKernel<Self>> {
        None
    }

    /// Returns the forward radix-two interior kernel when AVX2 is available.
    fn real_radix2_forward_interior_kernel() -> Option<RealRadix2ForwardInteriorKernel<Self>> {
        None
    }

    /// Executes the first fixed-radix pass over contiguous complex values.
    ///
    /// # Safety
    ///
    /// The caller must confirm AVX2 support and ensure the radix is 2, 4, or
    /// 8, `ido` is at least the scalar's vector width, and the
    /// data and twiddle slices match the pass dimensions.
    unsafe fn avx2_first_pass(data: &mut [Complex<Self>], pass: &SimdPass<'_, Self>);

    /// Executes a fixed-radix out-of-place pass over contiguous complex values.
    ///
    /// # Safety
    ///
    /// The caller must confirm AVX2 support and ensure the radix is 2, 4, or
    /// 8, `ido` is at least the scalar's vector width, the
    /// slices are disjoint, and the slices match the pass dimensions.
    unsafe fn avx2_pass(
        input: &[Complex<Self>],
        output: &mut [Complex<Self>],
        pass: &SimdPass<'_, Self>,
    );
}

#[cfg(target_arch = "x86_64")]
mod x86 {
    use std::sync::OnceLock;

    use super::{
        Complex, ComplexGeneralPrimeIdo1Kernel, ComplexPassKernel, RealGeneralPrimeIdo1Kernel,
        RealRadix2ForwardInteriorKernel, RealRadix2InteriorKernel, RealRadix4ForwardInteriorKernel,
        RealRadix4InteriorKernel, SimdPass, SimdScalar,
    };

    static AVX2_AVAILABLE: OnceLock<bool> = OnceLock::new();

    pub(super) fn available() -> bool {
        *AVX2_AVAILABLE.get_or_init(|| std::is_x86_feature_detected!("avx2"))
    }

    impl SimdScalar for f64 {
        const AVX2_COMPLEX_LANES: usize = 2;

        fn avx2_available() -> bool {
            available()
        }

        fn avx2_ido1_radix8_pass_kernel() -> Option<ComplexPassKernel<Self>> {
            Some(f64_kernel::pass_ido1_radix8)
        }

        fn avx2_ido1_radix4_pass_kernel() -> Option<ComplexPassKernel<Self>> {
            Some(f64_kernel::pass_ido1_radix4)
        }

        fn avx2_general_prime_ido1_kernel() -> Option<ComplexGeneralPrimeIdo1Kernel<Self>> {
            if available() {
                Some(f64_kernel::general_prime_ido1_radix31)
            } else {
                None
            }
        }

        fn real_radix4_interior_kernel() -> Option<RealRadix4InteriorKernel<Self>> {
            if available() {
                Some(f64_real_kernel::inverse_radix4_interior)
            } else {
                None
            }
        }

        fn real_general_prime_ido1_kernel() -> Option<RealGeneralPrimeIdo1Kernel<Self>> {
            if available() {
                Some(f64_real_kernel::general_prime_ido1)
            } else {
                None
            }
        }

        fn real_radix4_forward_interior_kernel() -> Option<RealRadix4ForwardInteriorKernel<Self>> {
            if available() {
                Some(f64_real_kernel::forward_radix4_interior)
            } else {
                None
            }
        }

        fn real_radix2_interior_kernel() -> Option<RealRadix2InteriorKernel<Self>> {
            if available() {
                Some(f64_real_kernel::inverse_radix2_interior)
            } else {
                None
            }
        }

        fn real_radix2_forward_interior_kernel() -> Option<RealRadix2ForwardInteriorKernel<Self>> {
            if available() {
                Some(f64_real_kernel::forward_radix2_interior)
            } else {
                None
            }
        }

        unsafe fn avx2_first_pass(data: &mut [Complex<Self>], pass: &SimdPass<'_, Self>) {
            // SAFETY: the caller upholds the trait's AVX2 and layout contract.
            unsafe { f64_kernel::first_pass(data, pass) }
        }

        unsafe fn avx2_pass(
            input: &[Complex<Self>],
            output: &mut [Complex<Self>],
            pass: &SimdPass<'_, Self>,
        ) {
            // SAFETY: the caller upholds the trait's AVX2 and layout contract.
            unsafe { f64_kernel::pass(input, output, pass) }
        }
    }

    impl SimdScalar for f32 {
        const AVX2_COMPLEX_LANES: usize = 4;

        fn avx2_available() -> bool {
            available()
        }

        fn real_radix4_interior_kernel() -> Option<RealRadix4InteriorKernel<Self>> {
            if available() {
                Some(f32_real_kernel::inverse_radix4_interior)
            } else {
                None
            }
        }

        fn real_radix4_forward_interior_kernel() -> Option<RealRadix4ForwardInteriorKernel<Self>> {
            if available() {
                Some(f32_real_kernel::forward_radix4_interior)
            } else {
                None
            }
        }

        fn real_radix2_interior_kernel() -> Option<RealRadix2InteriorKernel<Self>> {
            if available() {
                Some(f32_real_kernel::inverse_radix2_interior)
            } else {
                None
            }
        }

        fn real_radix2_forward_interior_kernel() -> Option<RealRadix2ForwardInteriorKernel<Self>> {
            if available() {
                Some(f32_real_kernel::forward_radix2_interior)
            } else {
                None
            }
        }

        unsafe fn avx2_first_pass(data: &mut [Complex<Self>], pass: &SimdPass<'_, Self>) {
            // SAFETY: the caller upholds the trait's AVX2 and layout contract.
            unsafe { f32_kernel::first_pass(data, pass) }
        }

        unsafe fn avx2_pass(
            input: &[Complex<Self>],
            output: &mut [Complex<Self>],
            pass: &SimdPass<'_, Self>,
        ) {
            // SAFETY: the caller upholds the trait's AVX2 and layout contract.
            unsafe { f32_kernel::pass(input, output, pass) }
        }
    }

    pub(super) mod f64_kernel {
        use std::arch::x86_64::*;

        use super::super::{ComplexGeneralPrimeIdo1Params, SimdPass};
        use num_complex::Complex;

        type V = __m256d;

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn load(values: *const Complex<f64>, index: usize) -> V {
            // SAFETY: Complex<f64> is repr(C); callers validate vector bounds.
            unsafe { _mm256_loadu_pd(values.add(index).cast::<f64>()) }
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn store(values: *mut Complex<f64>, index: usize, value: V) {
            // SAFETY: Complex<f64> is repr(C); callers validate vector bounds.
            unsafe { _mm256_storeu_pd(values.add(index).cast::<f64>(), value) }
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn complex_mul(left: V, right: V, inverse: bool) -> V {
            let right = if inverse {
                right
            } else {
                _mm256_mul_pd(right, _mm256_setr_pd(1.0, -1.0, 1.0, -1.0))
            };
            let swapped = _mm256_permute_pd(left, 0b0101);
            let real = _mm256_movedup_pd(right);
            let imaginary = _mm256_permute_pd(right, 0b1111);
            _mm256_addsub_pd(_mm256_mul_pd(left, real), _mm256_mul_pd(swapped, imaginary))
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn rotate_quarter(value: V, inverse: bool) -> V {
            let swapped = _mm256_permute_pd(value, 0b0101);
            let signs = if inverse {
                _mm256_setr_pd(-1.0, 1.0, -1.0, 1.0)
            } else {
                _mm256_setr_pd(1.0, -1.0, 1.0, -1.0)
            };
            _mm256_mul_pd(swapped, signs)
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn rotate_eighth(value: V, inverse: bool) -> V {
            let real = _mm256_movedup_pd(value);
            let imaginary = _mm256_permute_pd(value, 0b1111);
            let sign = if inverse { 1.0 } else { -1.0 };
            let rotated_real = _mm256_sub_pd(real, _mm256_mul_pd(_mm256_set1_pd(sign), imaginary));
            let rotated_imag = _mm256_add_pd(_mm256_mul_pd(_mm256_set1_pd(sign), real), imaginary);
            _mm256_mul_pd(
                _mm256_blend_pd(rotated_real, rotated_imag, 0b1010),
                _mm256_set1_pd(std::f64::consts::FRAC_1_SQRT_2),
            )
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn rotate_eighth_three(value: V, inverse: bool) -> V {
            let real = _mm256_movedup_pd(value);
            let imaginary = _mm256_permute_pd(value, 0b1111);
            let (rotated_real, rotated_imag) = if inverse {
                (
                    _mm256_sub_pd(_mm256_setzero_pd(), _mm256_add_pd(real, imaginary)),
                    _mm256_sub_pd(real, imaginary),
                )
            } else {
                (
                    _mm256_sub_pd(imaginary, real),
                    _mm256_sub_pd(_mm256_setzero_pd(), _mm256_add_pd(real, imaginary)),
                )
            };
            _mm256_mul_pd(
                _mm256_blend_pd(rotated_real, rotated_imag, 0b1010),
                _mm256_set1_pd(std::f64::consts::FRAC_1_SQRT_2),
            )
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn radix2(input: &[V; 8]) -> [V; 8] {
            let mut output = [_mm256_setzero_pd(); 8];
            output[0] = _mm256_add_pd(input[0], input[1]);
            output[1] = _mm256_sub_pd(input[0], input[1]);
            output
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn radix4(input: &[V; 8], inverse: bool) -> [V; 8] {
            let mut output = [_mm256_setzero_pd(); 8];
            let even_sum = _mm256_add_pd(input[0], input[2]);
            let even_difference = _mm256_sub_pd(input[0], input[2]);
            let odd_sum = _mm256_add_pd(input[1], input[3]);
            let odd_difference = _mm256_sub_pd(input[1], input[3]);
            let turn = unsafe { rotate_quarter(odd_difference, inverse) };
            output[0] = _mm256_add_pd(even_sum, odd_sum);
            output[2] = _mm256_sub_pd(even_sum, odd_sum);
            output[1] = _mm256_add_pd(even_difference, turn);
            output[3] = _mm256_sub_pd(even_difference, turn);
            output
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn radix8(input: &[V; 8], inverse: bool) -> [V; 8] {
            let mut output = [_mm256_setzero_pd(); 8];
            let mut a1 = _mm256_add_pd(input[1], input[5]);
            let mut a5 = _mm256_sub_pd(input[1], input[5]);
            let mut a3 = _mm256_add_pd(input[3], input[7]);
            let mut a7 = _mm256_sub_pd(input[3], input[7]);
            let old_a1 = a1;
            a1 = _mm256_add_pd(a1, a3);
            a3 = unsafe { rotate_quarter(_mm256_sub_pd(old_a1, a3), inverse) };
            a7 = unsafe { rotate_quarter(a7, inverse) };
            let old_a5 = a5;
            a5 = _mm256_add_pd(a5, a7);
            a7 = unsafe { rotate_eighth_three(_mm256_sub_pd(old_a5, a7), inverse) };
            a5 = unsafe { rotate_eighth(a5, inverse) };

            let a0 = _mm256_add_pd(input[0], input[4]);
            let a4 = _mm256_sub_pd(input[0], input[4]);
            let a2 = _mm256_add_pd(input[2], input[6]);
            let mut a6 = _mm256_sub_pd(input[2], input[6]);
            let even_sum = _mm256_add_pd(a0, a2);
            let even_difference = _mm256_sub_pd(a0, a2);
            output[0] = _mm256_add_pd(even_sum, a1);
            output[4] = _mm256_sub_pd(even_sum, a1);
            output[2] = _mm256_add_pd(even_difference, a3);
            output[6] = _mm256_sub_pd(even_difference, a3);
            a6 = unsafe { rotate_quarter(a6, inverse) };
            let odd_sum = _mm256_add_pd(a4, a6);
            output[1] = _mm256_add_pd(odd_sum, a5);
            output[5] = _mm256_sub_pd(odd_sum, a5);
            output[3] = _mm256_add_pd(_mm256_sub_pd(a4, a6), a7);
            output[7] = _mm256_sub_pd(_mm256_sub_pd(a4, a6), a7);
            output
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn transform<const RADIX: usize>(input: &[V; 8], inverse: bool) -> [V; 8] {
            match RADIX {
                2 => unsafe { radix2(input) },
                4 => unsafe { radix4(input, inverse) },
                8 => unsafe { radix8(input, inverse) },
                _ => unreachable!("AVX2 kernel supports radices 2, 4, and 8"),
            }
        }

        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn first_pass(data: &mut [Complex<f64>], pass: &SimdPass<'_, f64>) {
            match pass.radix {
                2 => unsafe { first_pass_fixed::<2>(data, pass) },
                4 => unsafe { first_pass_fixed::<4>(data, pass) },
                8 => unsafe { first_pass_fixed::<8>(data, pass) },
                _ => unreachable!("AVX2 kernel supports radices 2, 4, and 8"),
            }
        }

        #[target_feature(enable = "avx2")]
        unsafe fn first_pass_fixed<const RADIX: usize>(
            data: &mut [Complex<f64>],
            pass: &SimdPass<'_, f64>,
        ) {
            let data_ptr = data.as_mut_ptr();
            let vector_ido = pass.ido - pass.ido % 2;
            for inner in (0..vector_ido).step_by(2) {
                let mut input = [_mm256_setzero_pd(); 8];
                for (branch, slot) in input.iter_mut().enumerate().take(RADIX) {
                    *slot = unsafe { load(data_ptr, inner + pass.ido * branch) };
                }
                let transformed = unsafe { transform::<RADIX>(&input, pass.inverse) };
                for (branch, &result) in transformed.iter().enumerate().take(RADIX) {
                    let mut value = result;
                    if branch != 0 && pass.ido > 1 {
                        value = unsafe {
                            complex_mul(
                                value,
                                load(pass.twiddles.as_ptr(), branch * pass.ido + inner),
                                pass.inverse,
                            )
                        };
                    }
                    if pass.inverse {
                        value = _mm256_mul_pd(value, _mm256_set1_pd(pass.inverse_scale));
                    }
                    unsafe { store(data_ptr, inner + pass.ido * branch, value) };
                }
            }
        }

        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn pass(
            input: &[Complex<f64>],
            output: &mut [Complex<f64>],
            pass: &SimdPass<'_, f64>,
        ) {
            match pass.radix {
                2 => unsafe { pass_fixed::<2>(input, output, pass) },
                4 => unsafe { pass_fixed::<4>(input, output, pass) },
                8 => unsafe { pass_fixed::<8>(input, output, pass) },
                _ => unreachable!("AVX2 kernel supports radices 2, 4, and 8"),
            }
        }

        /// # Safety
        ///
        /// The caller must have detected AVX2 and provide a disjoint
        /// radix-eight pass with `ido == 1`, even `l1 >= 2`, and slices of at
        /// least `8 * l1` complex elements.
        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn pass_ido1_radix8(
            input: &[Complex<f64>],
            output: &mut [Complex<f64>],
            pass: &SimdPass<'_, f64>,
        ) {
            debug_assert_eq!(pass.radix, 8);
            debug_assert_eq!(pass.ido, 1);
            debug_assert!(pass.l1 >= 2 && pass.l1.is_multiple_of(2));
            debug_assert!(input.len() >= 8 * pass.l1);
            debug_assert!(output.len() >= 8 * pass.l1);

            let input_ptr = input.as_ptr();
            let output_ptr = output.as_mut_ptr();
            let scale = _mm256_set1_pd(if pass.inverse {
                pass.inverse_scale
            } else {
                1.0
            });
            for outer in (0..pass.l1).step_by(2) {
                let first_base = 8 * outer;
                let second_base = first_base + 8;
                let mut values = [_mm256_setzero_pd(); 8];
                for pair in 0..4 {
                    let first = unsafe { load(input_ptr, first_base + 2 * pair) };
                    let second = unsafe { load(input_ptr, second_base + 2 * pair) };
                    values[2 * pair] = _mm256_permute2f128_pd::<0x20>(first, second);
                    values[2 * pair + 1] = _mm256_permute2f128_pd::<0x31>(first, second);
                }

                let transformed = unsafe { radix8(&values, pass.inverse) };
                for (branch, &result) in transformed.iter().enumerate() {
                    let value = if pass.inverse {
                        _mm256_mul_pd(result, scale)
                    } else {
                        result
                    };
                    unsafe { store(output_ptr, branch * pass.l1 + outer, value) };
                }
            }
        }

        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn pass_ido1_radix4(
            input: &[Complex<f64>],
            output: &mut [Complex<f64>],
            pass: &SimdPass<'_, f64>,
        ) {
            debug_assert_eq!(pass.radix, 4);
            debug_assert_eq!(pass.ido, 1);
            debug_assert!(pass.l1 >= 2);
            // SAFETY: this function has the same target-feature and slice
            // contract as the fixed radix-four helper.
            unsafe { pass_ido1_fixed::<4>(input, output, pass) }
        }

        /// Executes a radix-31 general-prime pass with `ido == 1`, processing
        /// pairs of output frequencies in AVX2 lanes.
        ///
        /// # Safety
        ///
        /// The caller must have detected AVX2 and provide disjoint input and
        /// output slices of at least `31 * l1` complex values, `l1 > 0`, a
        /// radix-31 phase table with at least 465 entries, and a roots slice
        /// containing every indexed root. The inverse input must be in the
        /// deferred-normalization safety range.
        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn general_prime_ido1_radix31(
            input: &[Complex<f64>],
            output: &mut [Complex<f64>],
            params: ComplexGeneralPrimeIdo1Params<'_, f64>,
        ) {
            let ComplexGeneralPrimeIdo1Params {
                l1,
                radix,
                roots,
                root_stride,
                phase_indices,
                inverse,
                scale,
            } = params;
            let half = radix / 2;
            debug_assert_eq!(radix, 31);
            debug_assert!(l1 > 0);
            debug_assert!(root_stride > 0);
            debug_assert!(input.len() >= radix * l1);
            debug_assert!(output.len() >= radix * l1);
            debug_assert!(phase_indices.len() >= half * radix);
            debug_assert!(roots.len() > (radix - 1) * root_stride);

            let output_ptr = output.as_mut_ptr();
            let imaginary_signs = _mm256_setr_pd(-1.0, 1.0, -1.0, 1.0);
            let output_scale = _mm256_set1_pd(scale);

            for outer in 0..l1 {
                let input_zero = input[radix * outer];
                let mut dc = input_zero;
                for branch in 1..=half {
                    let pair =
                        input[radix * outer + branch] + input[radix * outer + (radix - branch)];
                    dc += pair;
                }
                if inverse {
                    dc *= scale;
                }
                output[outer] = dc;

                let mut frequency = 1;
                while frequency < half {
                    let initial =
                        _mm256_setr_pd(input_zero.re, input_zero.im, input_zero.re, input_zero.im);
                    let mut positive = initial;
                    let mut negative = initial;

                    for branch in 1..=half {
                        let left = input[radix * outer + branch];
                        let right = input[radix * outer + (radix - branch)];
                        let pair = left + right;
                        let difference = left - right;
                        let pair = _mm256_setr_pd(pair.re, pair.im, pair.re, pair.im);
                        let difference = _mm256_setr_pd(
                            difference.re,
                            difference.im,
                            difference.re,
                            difference.im,
                        );
                        let imaginary_difference =
                            _mm256_mul_pd(_mm256_permute_pd(difference, 0b0101), imaginary_signs);

                        let phase_first =
                            usize::from(phase_indices[(branch - 1) * radix + frequency]);
                        let phase_second =
                            usize::from(phase_indices[(branch - 1) * radix + frequency + 1]);
                        let root_first = roots[phase_first * root_stride];
                        let root_second = roots[phase_second * root_stride];
                        let sign = if inverse { 1.0 } else { -1.0 };
                        let cosine = _mm256_setr_pd(
                            root_first.re,
                            root_first.re,
                            root_second.re,
                            root_second.re,
                        );
                        let sine = _mm256_setr_pd(
                            root_first.im * sign,
                            root_first.im * sign,
                            root_second.im * sign,
                            root_second.im * sign,
                        );
                        let cosine_term = _mm256_mul_pd(pair, cosine);
                        let sine_term = _mm256_mul_pd(imaginary_difference, sine);
                        positive = _mm256_add_pd(_mm256_add_pd(positive, cosine_term), sine_term);
                        negative = _mm256_sub_pd(_mm256_add_pd(negative, cosine_term), sine_term);
                    }

                    if inverse {
                        positive = _mm256_mul_pd(positive, output_scale);
                        negative = _mm256_mul_pd(negative, output_scale);
                    }
                    // SAFETY: `frequency` and `outer` stay within the
                    // validated radix-by-l1 output layout.
                    unsafe {
                        _mm_storeu_pd(
                            output_ptr.add(frequency * l1 + outer).cast::<f64>(),
                            _mm256_castpd256_pd128(positive),
                        );
                        _mm_storeu_pd(
                            output_ptr.add((frequency + 1) * l1 + outer).cast::<f64>(),
                            _mm256_extractf128_pd(positive, 1),
                        );
                        _mm_storeu_pd(
                            output_ptr
                                .add((radix - frequency) * l1 + outer)
                                .cast::<f64>(),
                            _mm256_castpd256_pd128(negative),
                        );
                        _mm_storeu_pd(
                            output_ptr
                                .add((radix - frequency - 1) * l1 + outer)
                                .cast::<f64>(),
                            _mm256_extractf128_pd(negative, 1),
                        );
                    }
                    frequency += 2;
                }

                if frequency <= half {
                    let mut positive = input_zero;
                    let mut negative = input_zero;
                    for branch in 1..=half {
                        let left = input[radix * outer + branch];
                        let right = input[radix * outer + (radix - branch)];
                        let pair = left + right;
                        let difference = left - right;
                        let phase = usize::from(phase_indices[(branch - 1) * radix + frequency]);
                        let root = roots[phase * root_stride];
                        let root_im = if inverse { root.im } else { -root.im };
                        let cosine = pair * root.re;
                        let sine = Complex::new(-difference.im, difference.re) * root_im;
                        positive = positive + cosine + sine;
                        negative = negative + cosine - sine;
                    }
                    if inverse {
                        positive *= scale;
                        negative *= scale;
                    }
                    output[frequency * l1 + outer] = positive;
                    output[(radix - frequency) * l1 + outer] = negative;
                }
            }
        }

        #[target_feature(enable = "avx2")]
        unsafe fn pass_ido1_fixed<const RADIX: usize>(
            input: &[Complex<f64>],
            output: &mut [Complex<f64>],
            pass: &SimdPass<'_, f64>,
        ) {
            debug_assert_eq!(pass.radix, RADIX);
            debug_assert_eq!(pass.ido, 1);
            debug_assert!(pass.l1 >= 2);
            debug_assert!(input.len() >= RADIX * pass.l1);
            debug_assert!(output.len() >= RADIX * pass.l1);

            let input_ptr = input.as_ptr();
            let output_ptr = output.as_mut_ptr();
            let vector_l1 = pass.l1 - pass.l1 % 2;
            let scale = _mm256_set1_pd(if pass.inverse {
                pass.inverse_scale
            } else {
                1.0
            });
            for outer in (0..vector_l1).step_by(2) {
                let first_base = RADIX * outer;
                let second_base = first_base + RADIX;
                let mut values = [_mm256_setzero_pd(); 8];
                for pair in 0..RADIX / 2 {
                    let first = unsafe { load(input_ptr, first_base + 2 * pair) };
                    let second = unsafe { load(input_ptr, second_base + 2 * pair) };
                    values[2 * pair] = _mm256_permute2f128_pd::<0x20>(first, second);
                    values[2 * pair + 1] = _mm256_permute2f128_pd::<0x31>(first, second);
                }

                let transformed = unsafe { transform::<RADIX>(&values, pass.inverse) };
                for (branch, &result) in transformed.iter().enumerate().take(RADIX) {
                    let value = if pass.inverse {
                        _mm256_mul_pd(result, scale)
                    } else {
                        result
                    };
                    unsafe { store(output_ptr, branch * pass.l1 + outer, value) };
                }
            }
        }

        #[target_feature(enable = "avx2")]
        unsafe fn pass_fixed<const RADIX: usize>(
            input: &[Complex<f64>],
            output: &mut [Complex<f64>],
            pass: &SimdPass<'_, f64>,
        ) {
            let input_ptr = input.as_ptr();
            let output_ptr = output.as_mut_ptr();
            let vector_ido = pass.ido - pass.ido % 2;
            for outer in 0..pass.l1 {
                for inner in (0..vector_ido).step_by(2) {
                    let mut values = [_mm256_setzero_pd(); 8];
                    for (branch, slot) in values.iter_mut().enumerate().take(RADIX) {
                        *slot =
                            unsafe { load(input_ptr, inner + pass.ido * (branch + RADIX * outer)) };
                    }
                    let transformed = unsafe { transform::<RADIX>(&values, pass.inverse) };
                    for (branch, &result) in transformed.iter().enumerate().take(RADIX) {
                        let mut value = result;
                        if branch != 0 && pass.ido > 1 {
                            value = unsafe {
                                complex_mul(
                                    value,
                                    load(pass.twiddles.as_ptr(), branch * pass.ido + inner),
                                    pass.inverse,
                                )
                            };
                        }
                        if pass.inverse {
                            value = _mm256_mul_pd(value, _mm256_set1_pd(pass.inverse_scale));
                        }
                        unsafe {
                            store(
                                output_ptr,
                                inner + pass.ido * (outer + pass.l1 * branch),
                                value,
                            )
                        };
                    }
                }
            }
        }
    }

    mod f32_kernel {
        use std::arch::x86_64::*;

        use super::super::SimdPass;
        use num_complex::Complex;

        type V = __m256;

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn load(values: *const Complex<f32>, index: usize) -> V {
            // SAFETY: Complex<f32> is repr(C); callers validate vector bounds.
            unsafe { _mm256_loadu_ps(values.add(index).cast::<f32>()) }
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn store(values: *mut Complex<f32>, index: usize, value: V) {
            // SAFETY: Complex<f32> is repr(C); callers validate vector bounds.
            unsafe { _mm256_storeu_ps(values.add(index).cast::<f32>(), value) }
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn complex_mul(left: V, right: V, inverse: bool) -> V {
            let right = if inverse {
                right
            } else {
                _mm256_mul_ps(
                    right,
                    _mm256_setr_ps(1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0),
                )
            };
            let swapped = _mm256_permute_ps(left, 0b1011_0001);
            let real = _mm256_moveldup_ps(right);
            let imaginary = _mm256_movehdup_ps(right);
            _mm256_addsub_ps(_mm256_mul_ps(left, real), _mm256_mul_ps(swapped, imaginary))
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn rotate_quarter(value: V, inverse: bool) -> V {
            let swapped = _mm256_permute_ps(value, 0b1011_0001);
            let signs = if inverse {
                _mm256_setr_ps(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0)
            } else {
                _mm256_setr_ps(1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0)
            };
            _mm256_mul_ps(swapped, signs)
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn rotate_eighth(value: V, inverse: bool) -> V {
            let real = _mm256_moveldup_ps(value);
            let imaginary = _mm256_movehdup_ps(value);
            let sign = if inverse { 1.0 } else { -1.0 };
            let rotated_real = _mm256_sub_ps(real, _mm256_mul_ps(_mm256_set1_ps(sign), imaginary));
            let rotated_imag = _mm256_add_ps(_mm256_mul_ps(_mm256_set1_ps(sign), real), imaginary);
            _mm256_mul_ps(
                _mm256_blend_ps(rotated_real, rotated_imag, 0b1010_1010),
                _mm256_set1_ps(std::f32::consts::FRAC_1_SQRT_2),
            )
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn rotate_eighth_three(value: V, inverse: bool) -> V {
            let real = _mm256_moveldup_ps(value);
            let imaginary = _mm256_movehdup_ps(value);
            let (rotated_real, rotated_imag) = if inverse {
                (
                    _mm256_sub_ps(_mm256_setzero_ps(), _mm256_add_ps(real, imaginary)),
                    _mm256_sub_ps(real, imaginary),
                )
            } else {
                (
                    _mm256_sub_ps(imaginary, real),
                    _mm256_sub_ps(_mm256_setzero_ps(), _mm256_add_ps(real, imaginary)),
                )
            };
            _mm256_mul_ps(
                _mm256_blend_ps(rotated_real, rotated_imag, 0b1010_1010),
                _mm256_set1_ps(std::f32::consts::FRAC_1_SQRT_2),
            )
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn radix2(input: &[V; 8]) -> [V; 8] {
            let mut output = [_mm256_setzero_ps(); 8];
            output[0] = _mm256_add_ps(input[0], input[1]);
            output[1] = _mm256_sub_ps(input[0], input[1]);
            output
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn radix4(input: &[V; 8], inverse: bool) -> [V; 8] {
            let mut output = [_mm256_setzero_ps(); 8];
            let even_sum = _mm256_add_ps(input[0], input[2]);
            let even_difference = _mm256_sub_ps(input[0], input[2]);
            let odd_sum = _mm256_add_ps(input[1], input[3]);
            let odd_difference = _mm256_sub_ps(input[1], input[3]);
            let turn = unsafe { rotate_quarter(odd_difference, inverse) };
            output[0] = _mm256_add_ps(even_sum, odd_sum);
            output[2] = _mm256_sub_ps(even_sum, odd_sum);
            output[1] = _mm256_add_ps(even_difference, turn);
            output[3] = _mm256_sub_ps(even_difference, turn);
            output
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn radix8(input: &[V; 8], inverse: bool) -> [V; 8] {
            let mut output = [_mm256_setzero_ps(); 8];
            let mut a1 = _mm256_add_ps(input[1], input[5]);
            let mut a5 = _mm256_sub_ps(input[1], input[5]);
            let mut a3 = _mm256_add_ps(input[3], input[7]);
            let mut a7 = _mm256_sub_ps(input[3], input[7]);
            let old_a1 = a1;
            a1 = _mm256_add_ps(a1, a3);
            a3 = unsafe { rotate_quarter(_mm256_sub_ps(old_a1, a3), inverse) };
            a7 = unsafe { rotate_quarter(a7, inverse) };
            let old_a5 = a5;
            a5 = _mm256_add_ps(a5, a7);
            a7 = unsafe { rotate_eighth_three(_mm256_sub_ps(old_a5, a7), inverse) };
            a5 = unsafe { rotate_eighth(a5, inverse) };

            let a0 = _mm256_add_ps(input[0], input[4]);
            let a4 = _mm256_sub_ps(input[0], input[4]);
            let a2 = _mm256_add_ps(input[2], input[6]);
            let mut a6 = _mm256_sub_ps(input[2], input[6]);
            let even_sum = _mm256_add_ps(a0, a2);
            let even_difference = _mm256_sub_ps(a0, a2);
            output[0] = _mm256_add_ps(even_sum, a1);
            output[4] = _mm256_sub_ps(even_sum, a1);
            output[2] = _mm256_add_ps(even_difference, a3);
            output[6] = _mm256_sub_ps(even_difference, a3);
            a6 = unsafe { rotate_quarter(a6, inverse) };
            let odd_sum = _mm256_add_ps(a4, a6);
            output[1] = _mm256_add_ps(odd_sum, a5);
            output[5] = _mm256_sub_ps(odd_sum, a5);
            output[3] = _mm256_add_ps(_mm256_sub_ps(a4, a6), a7);
            output[7] = _mm256_sub_ps(_mm256_sub_ps(a4, a6), a7);
            output
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn transform<const RADIX: usize>(input: &[V; 8], inverse: bool) -> [V; 8] {
            match RADIX {
                2 => unsafe { radix2(input) },
                4 => unsafe { radix4(input, inverse) },
                8 => unsafe { radix8(input, inverse) },
                _ => unreachable!("AVX2 kernel supports radices 2, 4, and 8"),
            }
        }

        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn first_pass(data: &mut [Complex<f32>], pass: &SimdPass<'_, f32>) {
            match pass.radix {
                2 => unsafe { first_pass_fixed::<2>(data, pass) },
                4 => unsafe { first_pass_fixed::<4>(data, pass) },
                8 => unsafe { first_pass_fixed::<8>(data, pass) },
                _ => unreachable!("AVX2 kernel supports radices 2, 4, and 8"),
            }
        }

        #[target_feature(enable = "avx2")]
        unsafe fn first_pass_fixed<const RADIX: usize>(
            data: &mut [Complex<f32>],
            pass: &SimdPass<'_, f32>,
        ) {
            let data_ptr = data.as_mut_ptr();
            let vector_ido = pass.ido - pass.ido % 4;
            for inner in (0..vector_ido).step_by(4) {
                let mut input = [_mm256_setzero_ps(); 8];
                for (branch, slot) in input.iter_mut().enumerate().take(RADIX) {
                    *slot = unsafe { load(data_ptr, inner + pass.ido * branch) };
                }
                let transformed = unsafe { transform::<RADIX>(&input, pass.inverse) };
                for (branch, &result) in transformed.iter().enumerate().take(RADIX) {
                    let mut value = result;
                    if branch != 0 && pass.ido > 1 {
                        value = unsafe {
                            complex_mul(
                                value,
                                load(pass.twiddles.as_ptr(), branch * pass.ido + inner),
                                pass.inverse,
                            )
                        };
                    }
                    if pass.inverse {
                        value = _mm256_mul_ps(value, _mm256_set1_ps(pass.inverse_scale));
                    }
                    unsafe { store(data_ptr, inner + pass.ido * branch, value) };
                }
            }
        }

        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn pass(
            input: &[Complex<f32>],
            output: &mut [Complex<f32>],
            pass: &SimdPass<'_, f32>,
        ) {
            match pass.radix {
                2 => unsafe { pass_fixed::<2>(input, output, pass) },
                4 => unsafe { pass_fixed::<4>(input, output, pass) },
                8 => unsafe { pass_fixed::<8>(input, output, pass) },
                _ => unreachable!("AVX2 kernel supports radices 2, 4, and 8"),
            }
        }

        #[target_feature(enable = "avx2")]
        unsafe fn pass_fixed<const RADIX: usize>(
            input: &[Complex<f32>],
            output: &mut [Complex<f32>],
            pass: &SimdPass<'_, f32>,
        ) {
            let input_ptr = input.as_ptr();
            let output_ptr = output.as_mut_ptr();
            let vector_ido = pass.ido - pass.ido % 4;
            for outer in 0..pass.l1 {
                for inner in (0..vector_ido).step_by(4) {
                    let mut values = [_mm256_setzero_ps(); 8];
                    for (branch, slot) in values.iter_mut().enumerate().take(RADIX) {
                        *slot =
                            unsafe { load(input_ptr, inner + pass.ido * (branch + RADIX * outer)) };
                    }
                    let transformed = unsafe { transform::<RADIX>(&values, pass.inverse) };
                    for (branch, &result) in transformed.iter().enumerate().take(RADIX) {
                        let mut value = result;
                        if branch != 0 && pass.ido > 1 {
                            value = unsafe {
                                complex_mul(
                                    value,
                                    load(pass.twiddles.as_ptr(), branch * pass.ido + inner),
                                    pass.inverse,
                                )
                            };
                        }
                        if pass.inverse {
                            value = _mm256_mul_ps(value, _mm256_set1_ps(pass.inverse_scale));
                        }
                        unsafe {
                            store(
                                output_ptr,
                                inner + pass.ido * (outer + pass.l1 * branch),
                                value,
                            )
                        };
                    }
                }
            }
        }
    }

    mod f64_real_kernel {
        use std::arch::x86_64::*;

        use super::super::RealGeneralPrimeIdo1Params;

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn multiply_pm(roots: __m256d, values: __m256d) -> __m256d {
            let even_values = _mm256_movedup_pd(values);
            let odd_values = _mm256_permute_pd(values, 0b1111);
            let roots_swapped = _mm256_permute_pd(roots, 0b0101);
            let even_products = _mm256_mul_pd(roots, even_values);
            let odd_products = _mm256_mul_pd(roots_swapped, odd_values);
            let sums = _mm256_add_pd(even_products, odd_products);
            let differences = _mm256_sub_pd(odd_products, even_products);
            _mm256_blend_pd(sums, differences, 0b1010)
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn pair_re(value: __m256d) -> __m256d {
            _mm256_permute_pd(value, 0b0000)
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn pair_im(value: __m256d) -> __m256d {
            _mm256_permute_pd(value, 0b1111)
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn pack_pair(real: __m256d, imag: __m256d) -> __m256d {
            _mm256_unpacklo_pd(real, imag)
        }

        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn forward_radix2_interior(
            input: [&[f64]; 2],
            output: [&mut [f64]; 2],
            ido: usize,
            twiddles: &[f64],
        ) -> usize {
            debug_assert!(ido > 2);
            debug_assert!(ido.is_multiple_of(2));
            debug_assert!(input.iter().all(|branch| branch.len() == ido));
            debug_assert!(output.iter().all(|branch| branch.len() == ido));
            debug_assert!(twiddles.len() >= ido - 1);

            let input0 = input[0].as_ptr();
            let input1 = input[1].as_ptr();
            let twiddle = twiddles.as_ptr();
            let output0 = output[0].as_mut_ptr();
            let output1 = output[1].as_mut_ptr();

            let mut i = 2;
            while i + 2 < ido {
                let x0 = unsafe { _mm256_loadu_pd(input0.add(i - 1)) };
                let x1 = unsafe { _mm256_loadu_pd(input1.add(i - 1)) };
                let roots = unsafe { _mm256_loadu_pd(twiddle.add(i - 2)) };
                let twiddled = unsafe { multiply_pm(roots, x1) };
                let sum = _mm256_add_pd(x0, twiddled);
                let difference = _mm256_sub_pd(x0, twiddled);
                let reverse_imaginary = _mm256_sub_pd(twiddled, x0);
                let branch1 = _mm256_blend_pd(difference, reverse_imaginary, 0b1010);
                let branch1 = _mm256_permute4x64_pd(branch1, 0b0100_1110);
                unsafe {
                    _mm256_storeu_pd(output0.add(i - 1), sum);
                    _mm256_storeu_pd(output1.add(ido - i - 3), branch1);
                }
                i += 4;
            }
            i
        }

        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn inverse_radix2_interior(
            input: [&[f64]; 2],
            output: [&mut [f64]; 2],
            ido: usize,
            twiddles: &[f64],
        ) -> usize {
            debug_assert!(ido > 2);
            debug_assert!(ido.is_multiple_of(2));
            debug_assert!(input.iter().all(|branch| branch.len() == ido));
            debug_assert!(output.iter().all(|branch| branch.len() == ido));
            debug_assert!(twiddles.len() >= ido - 1);

            let input0 = input[0].as_ptr();
            let input1 = input[1].as_ptr();
            let twiddle = twiddles.as_ptr();
            let output0 = output[0].as_mut_ptr();
            let output1 = output[1].as_mut_ptr();

            let mut i = 2;
            while i + 2 < ido {
                let branch0 = unsafe { _mm256_loadu_pd(input0.add(i - 1)) };
                let branch1 = unsafe { _mm256_loadu_pd(input1.add(ido - i - 3)) };
                let branch1 = _mm256_permute4x64_pd(branch1, 0b0100_1110);
                let sum = _mm256_add_pd(branch0, branch1);
                let difference = _mm256_sub_pd(branch0, branch1);

                let output_branch0 = _mm256_blend_pd(sum, difference, 0b1010);
                let values = _mm256_blend_pd(
                    _mm256_permute_pd(difference, 0b0101),
                    _mm256_permute_pd(sum, 0b0101),
                    0b0101,
                );
                let roots = unsafe { _mm256_loadu_pd(twiddle.add(i - 2)) };
                let output_branch1 = unsafe { multiply_pm(roots, values) };
                let output_branch1 = _mm256_permute_pd(output_branch1, 0b0101);
                unsafe {
                    _mm256_storeu_pd(output0.add(i - 1), output_branch0);
                    _mm256_storeu_pd(output1.add(i - 1), output_branch1);
                }
                i += 4;
            }
            i
        }

        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn forward_radix4_interior(
            input: [&[f64]; 4],
            output: [&mut [f64]; 4],
            ido: usize,
            twiddles: &[f64],
        ) -> usize {
            debug_assert!(ido > 2);
            debug_assert!(ido.is_multiple_of(2));
            debug_assert!(input.iter().all(|branch| branch.len() == ido));
            debug_assert!(output.iter().all(|branch| branch.len() == ido));
            debug_assert!(twiddles.len() >= 3 * (ido - 1));

            let input0 = input[0].as_ptr();
            let input1 = input[1].as_ptr();
            let input2 = input[2].as_ptr();
            let input3 = input[3].as_ptr();
            let twiddle1 = twiddles.as_ptr();
            let twiddle2 = unsafe { twiddle1.add(ido - 1) };
            let twiddle3 = unsafe { twiddle2.add(ido - 1) };
            let output0 = output[0].as_mut_ptr();
            let output1 = output[1].as_mut_ptr();
            let output2 = output[2].as_mut_ptr();
            let output3 = output[3].as_mut_ptr();

            let mut i = 2;
            while i + 2 < ido {
                let x0 = unsafe { _mm256_loadu_pd(input0.add(i - 1)) };
                let x1 = unsafe { _mm256_loadu_pd(input1.add(i - 1)) };
                let x2 = unsafe { _mm256_loadu_pd(input2.add(i - 1)) };
                let x3 = unsafe { _mm256_loadu_pd(input3.add(i - 1)) };
                let roots1 = unsafe { _mm256_loadu_pd(twiddle1.add(i - 2)) };
                let roots2 = unsafe { _mm256_loadu_pd(twiddle2.add(i - 2)) };
                let roots3 = unsafe { _mm256_loadu_pd(twiddle3.add(i - 2)) };
                let x1 = unsafe { multiply_pm(roots1, x1) };
                let x2 = unsafe { multiply_pm(roots2, x2) };
                let x3 = unsafe { multiply_pm(roots3, x3) };

                let x0r = unsafe { pair_re(x0) };
                let x0i = unsafe { pair_im(x0) };
                let cr2 = unsafe { pair_re(x1) };
                let ci2 = unsafe { pair_im(x1) };
                let cr3 = unsafe { pair_re(x2) };
                let ci3 = unsafe { pair_im(x2) };
                let cr4 = unsafe { pair_re(x3) };
                let ci4 = unsafe { pair_im(x3) };

                let tr1 = _mm256_add_pd(cr4, cr2);
                let tr4 = _mm256_sub_pd(cr4, cr2);
                let ti1 = _mm256_add_pd(ci2, ci4);
                let ti4 = _mm256_sub_pd(ci2, ci4);
                let tr2 = _mm256_add_pd(x0r, cr3);
                let tr3 = _mm256_sub_pd(x0r, cr3);
                let ti2 = _mm256_add_pd(x0i, ci3);
                let ti3 = _mm256_sub_pd(x0i, ci3);

                let branch0 =
                    unsafe { pack_pair(_mm256_add_pd(tr2, tr1), _mm256_add_pd(ti1, ti2)) };
                let branch3 =
                    unsafe { pack_pair(_mm256_sub_pd(tr2, tr1), _mm256_sub_pd(ti1, ti2)) };
                let branch2 =
                    unsafe { pack_pair(_mm256_add_pd(tr3, ti4), _mm256_add_pd(tr4, ti3)) };
                let branch1 =
                    unsafe { pack_pair(_mm256_sub_pd(tr3, ti4), _mm256_sub_pd(tr4, ti3)) };
                let mirror = ido - i - 3;
                unsafe {
                    _mm256_storeu_pd(output0.add(i - 1), branch0);
                    _mm256_storeu_pd(output2.add(i - 1), branch2);
                    _mm256_storeu_pd(
                        output1.add(mirror),
                        _mm256_permute4x64_pd(branch1, 0b0100_1110),
                    );
                    _mm256_storeu_pd(
                        output3.add(mirror),
                        _mm256_permute4x64_pd(branch3, 0b0100_1110),
                    );
                }
                i += 4;
            }
            i
        }

        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn inverse_radix4_interior(
            input: &[f64],
            output: [&mut [f64]; 4],
            ido: usize,
            twiddles: &[f64],
        ) -> usize {
            debug_assert!(ido > 2);
            debug_assert!(ido.is_multiple_of(2));
            debug_assert_eq!(input.len(), 4 * ido);
            debug_assert!(output.iter().all(|branch| branch.len() == ido));
            debug_assert!(twiddles.len() >= 3 * (ido - 1));

            let input0 = input.as_ptr();
            let input1 = unsafe { input0.add(ido) };
            let input2 = unsafe { input1.add(ido) };
            let input3 = unsafe { input2.add(ido) };
            let twiddle1 = twiddles.as_ptr();
            let twiddle2 = unsafe { twiddle1.add(ido - 1) };
            let twiddle3 = unsafe { twiddle2.add(ido - 1) };
            let output0 = output[0].as_mut_ptr();
            let output1 = output[1].as_mut_ptr();
            let output2 = output[2].as_mut_ptr();
            let output3 = output[3].as_mut_ptr();

            let mut i = 2;
            while i + 2 < ido {
                let ic = ido - i;
                let x0 = unsafe { _mm256_loadu_pd(input0.add(i - 1)) };
                let x3 = unsafe { _mm256_loadu_pd(input3.add(ic - 3)) };
                let x3 = _mm256_permute4x64_pd(x3, 0b0100_1110);
                let x2 = unsafe { _mm256_loadu_pd(input2.add(i - 1)) };
                let x1 = unsafe { _mm256_loadu_pd(input1.add(ic - 3)) };
                let x1 = _mm256_permute4x64_pd(x1, 0b0100_1110);

                let sum03 = _mm256_add_pd(x0, x3);
                let diff03 = _mm256_sub_pd(x0, x3);
                let sum21 = _mm256_add_pd(x2, x1);
                let diff21 = _mm256_sub_pd(x2, x1);

                let out0_real = _mm256_add_pd(
                    _mm256_permute_pd(sum03, 0b0000),
                    _mm256_permute_pd(sum21, 0b0000),
                );
                let out0_imag = _mm256_add_pd(
                    _mm256_permute_pd(diff03, 0b1111),
                    _mm256_permute_pd(diff21, 0b1111),
                );
                let out0 = _mm256_unpacklo_pd(out0_real, out0_imag);

                let cr2 = _mm256_sub_pd(
                    _mm256_permute_pd(diff03, 0b0000),
                    _mm256_permute_pd(sum21, 0b1111),
                );
                let ci2 = _mm256_add_pd(
                    _mm256_permute_pd(sum03, 0b1111),
                    _mm256_permute_pd(diff21, 0b0000),
                );
                let values1 = _mm256_unpacklo_pd(ci2, cr2);

                let cr3 = _mm256_sub_pd(
                    _mm256_permute_pd(sum03, 0b0000),
                    _mm256_permute_pd(sum21, 0b0000),
                );
                let ci3 = _mm256_sub_pd(
                    _mm256_permute_pd(diff03, 0b1111),
                    _mm256_permute_pd(diff21, 0b1111),
                );
                let values2 = _mm256_unpacklo_pd(ci3, cr3);

                let cr4 = _mm256_add_pd(
                    _mm256_permute_pd(diff03, 0b0000),
                    _mm256_permute_pd(sum21, 0b1111),
                );
                let ci4 = _mm256_sub_pd(
                    _mm256_permute_pd(sum03, 0b1111),
                    _mm256_permute_pd(diff21, 0b0000),
                );
                let values3 = _mm256_unpacklo_pd(ci4, cr4);

                let roots1 = unsafe { _mm256_loadu_pd(twiddle1.add(i - 2)) };
                let roots2 = unsafe { _mm256_loadu_pd(twiddle2.add(i - 2)) };
                let roots3 = unsafe { _mm256_loadu_pd(twiddle3.add(i - 2)) };
                let result1 = unsafe { multiply_pm(roots1, values1) };
                let result2 = unsafe { multiply_pm(roots2, values2) };
                let result3 = unsafe { multiply_pm(roots3, values3) };

                unsafe {
                    _mm256_storeu_pd(output0.add(i - 1), out0);
                    _mm256_storeu_pd(output1.add(i - 1), _mm256_permute_pd(result1, 0b0101));
                    _mm256_storeu_pd(output2.add(i - 1), _mm256_permute_pd(result2, 0b0101));
                    _mm256_storeu_pd(output3.add(i - 1), _mm256_permute_pd(result3, 0b0101));
                }
                i += 4;
            }
            i
        }

        /// Executes an `ido == 1` real general-prime pass, vectorizing across
        /// consecutive frequencies in the packed real spectrum.
        ///
        /// # Safety
        ///
        /// The caller must have detected AVX2 support. `radix` must be odd
        /// and at least 7, `l1` must be positive, `input` and `output` must
        /// each contain at least `radix * l1` values, the coefficient slices
        /// must contain at least `(radix / 2)^2` values, and input/output
        /// storage must not overlap.
        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn general_prime_ido1(
            input: &[f64],
            output: &mut [f64],
            params: RealGeneralPrimeIdo1Params<'_, f64>,
        ) {
            let RealGeneralPrimeIdo1Params {
                l1,
                radix,
                cosines,
                sines,
                inverse,
                scale,
            } = params;
            let half = radix / 2;
            let coefficient_count = half * half;
            debug_assert!(radix >= 7 && radix % 2 == 1);
            debug_assert!(l1 > 0);
            debug_assert!(input.len() >= radix * l1);
            debug_assert!(output.len() >= radix * l1);
            debug_assert!(cosines.len() >= coefficient_count);
            debug_assert!(sines.len() >= coefficient_count);

            if !inverse {
                let zero = _mm_setzero_pd();
                for group in 0..l1 {
                    let input0 = input[group];
                    let mut dc = input0;
                    for branch in 1..=half {
                        dc += input[branch * l1 + group] + input[(radix - branch) * l1 + group];
                    }
                    let output_start = group * radix;
                    output[output_start] = dc;

                    let mut frequency_index = 0;
                    while frequency_index + 1 < half {
                        let mut sum_re = _mm_set1_pd(input0);
                        let mut sum_im = zero;
                        for branch in 1..=half {
                            let left = input[branch * l1 + group];
                            let right = input[(radix - branch) * l1 + group];
                            let pair = _mm_set1_pd(left + right);
                            let difference = _mm_set1_pd(left - right);
                            let coefficient_start = (branch - 1) * half + frequency_index;
                            let cosine =
                                unsafe { _mm_loadu_pd(cosines.as_ptr().add(coefficient_start)) };
                            let sine =
                                unsafe { _mm_loadu_pd(sines.as_ptr().add(coefficient_start)) };
                            sum_re = _mm_add_pd(sum_re, _mm_mul_pd(pair, cosine));
                            sum_im = _mm_sub_pd(sum_im, _mm_mul_pd(difference, sine));
                        }

                        let output_pair = unsafe {
                            output
                                .as_mut_ptr()
                                .add(output_start + 1 + 2 * frequency_index)
                        };
                        unsafe {
                            _mm_storeu_pd(output_pair, _mm_unpacklo_pd(sum_re, sum_im));
                            _mm_storeu_pd(output_pair.add(2), _mm_unpackhi_pd(sum_re, sum_im));
                        }
                        frequency_index += 2;
                    }

                    if frequency_index < half {
                        let frequency = frequency_index + 1;
                        let mut sum_re = input0;
                        let mut sum_im = 0.0;
                        for branch in 1..=half {
                            let left = input[branch * l1 + group];
                            let right = input[(radix - branch) * l1 + group];
                            let coefficient = (branch - 1) * half + frequency_index;
                            sum_re += (left + right) * cosines[coefficient];
                            sum_im -= (left - right) * sines[coefficient];
                        }
                        let output_start = output_start + 2 * frequency - 1;
                        output[output_start] = sum_re;
                        output[output_start + 1] = sum_im;
                    }
                }
                return;
            }

            let two = _mm_set1_pd(2.0);
            for group in 0..l1 {
                let input_start = group * radix;
                let dc = input[input_start];
                let mut zero_frequency_sample = dc;
                for frequency in 1..=half {
                    zero_frequency_sample += 2.0 * input[input_start + 2 * frequency - 1];
                }
                output[group] = zero_frequency_sample * scale;

                for sample in 1..=half {
                    let coefficient_start = (sample - 1) * half;
                    let mut positive = _mm_setzero_pd();
                    let mut negative = _mm_setzero_pd();
                    let mut frequency_index = 0;
                    while frequency_index + 1 < half {
                        let packed = unsafe {
                            _mm256_loadu_pd(
                                input.as_ptr().add(input_start + 1 + 2 * frequency_index),
                            )
                        };
                        let low = _mm256_castpd256_pd128(packed);
                        let high = _mm256_extractf128_pd(packed, 1);
                        let real = _mm_unpacklo_pd(low, high);
                        let imag = _mm_unpackhi_pd(low, high);
                        let cosine = unsafe {
                            _mm_loadu_pd(cosines.as_ptr().add(coefficient_start + frequency_index))
                        };
                        let sine = unsafe {
                            _mm_loadu_pd(sines.as_ptr().add(coefficient_start + frequency_index))
                        };
                        let real_cosine = _mm_mul_pd(real, cosine);
                        let imaginary_sine = _mm_mul_pd(imag, sine);
                        positive = _mm_add_pd(
                            positive,
                            _mm_mul_pd(two, _mm_sub_pd(real_cosine, imaginary_sine)),
                        );
                        negative = _mm_add_pd(
                            negative,
                            _mm_mul_pd(two, _mm_add_pd(real_cosine, imaginary_sine)),
                        );
                        frequency_index += 2;
                    }

                    let mut positive_parts = [0.0; 2];
                    let mut negative_parts = [0.0; 2];
                    unsafe {
                        _mm_storeu_pd(positive_parts.as_mut_ptr(), positive);
                        _mm_storeu_pd(negative_parts.as_mut_ptr(), negative);
                    }
                    let mut positive_sum = dc + positive_parts[0] + positive_parts[1];
                    let mut negative_sum = dc + negative_parts[0] + negative_parts[1];
                    if frequency_index < half {
                        let frequency = frequency_index + 1;
                        let coefficient = coefficient_start + frequency_index;
                        let real = input[input_start + 2 * frequency - 1];
                        let imag = input[input_start + 2 * frequency];
                        let real_cosine = real * cosines[coefficient];
                        let imaginary_sine = imag * sines[coefficient];
                        positive_sum += 2.0 * (real_cosine - imaginary_sine);
                        negative_sum += 2.0 * (real_cosine + imaginary_sine);
                    }

                    output[sample * l1 + group] = positive_sum * scale;
                    output[(radix - sample) * l1 + group] = negative_sum * scale;
                }
            }
        }
    }

    mod f32_real_kernel {
        use std::arch::x86_64::*;

        type V = __m256;

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn multiply_pm(roots: V, values: V) -> V {
            let even_values = _mm256_moveldup_ps(values);
            let odd_values = _mm256_movehdup_ps(values);
            let roots_swapped = _mm256_permute_ps(roots, 0b1011_0001);
            let even_products = _mm256_mul_ps(roots, even_values);
            let odd_products = _mm256_mul_ps(roots_swapped, odd_values);
            let sums = _mm256_add_ps(even_products, odd_products);
            let differences = _mm256_sub_ps(odd_products, even_products);
            _mm256_blend_ps(sums, differences, 0b1010_1010)
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn swap_pair(value: V) -> V {
            _mm256_permute_ps(value, 0b1011_0001)
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn reverse_complex_order(value: V) -> V {
            let order = _mm256_setr_epi32(6, 7, 4, 5, 2, 3, 0, 1);
            _mm256_permutevar8x32_ps(value, order)
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn real(value: V) -> V {
            _mm256_moveldup_ps(value)
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn imag(value: V) -> V {
            _mm256_movehdup_ps(value)
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        unsafe fn pack(real: V, imag: V) -> V {
            _mm256_blend_ps(real, imag, 0b1010_1010)
        }

        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn forward_radix2_interior(
            input: [&[f32]; 2],
            output: [&mut [f32]; 2],
            ido: usize,
            twiddles: &[f32],
        ) -> usize {
            debug_assert!(ido > 2);
            debug_assert!(ido.is_multiple_of(2));
            debug_assert!(input.iter().all(|branch| branch.len() == ido));
            debug_assert!(output.iter().all(|branch| branch.len() == ido));
            debug_assert!(twiddles.len() >= ido - 1);

            let input0 = input[0].as_ptr();
            let input1 = input[1].as_ptr();
            let twiddle = twiddles.as_ptr();
            let output0 = output[0].as_mut_ptr();
            let output1 = output[1].as_mut_ptr();

            let mut i = 2;
            while i + 6 < ido {
                let x0 = unsafe { _mm256_loadu_ps(input0.add(i - 1)) };
                let x1 = unsafe { _mm256_loadu_ps(input1.add(i - 1)) };
                let roots = unsafe { _mm256_loadu_ps(twiddle.add(i - 2)) };
                let twiddled = unsafe { multiply_pm(roots, x1) };
                let sum = _mm256_add_ps(x0, twiddled);
                let difference = _mm256_sub_ps(x0, twiddled);
                let reverse_imaginary = _mm256_sub_ps(twiddled, x0);
                let branch1 = _mm256_blend_ps(difference, reverse_imaginary, 0b1010_1010);
                let branch1 = unsafe { reverse_complex_order(branch1) };
                unsafe {
                    _mm256_storeu_ps(output0.add(i - 1), sum);
                    _mm256_storeu_ps(output1.add(ido - i - 7), branch1);
                }
                i += 8;
            }
            i
        }

        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn inverse_radix2_interior(
            input: [&[f32]; 2],
            output: [&mut [f32]; 2],
            ido: usize,
            twiddles: &[f32],
        ) -> usize {
            debug_assert!(ido > 2);
            debug_assert!(ido.is_multiple_of(2));
            debug_assert!(input.iter().all(|branch| branch.len() == ido));
            debug_assert!(output.iter().all(|branch| branch.len() == ido));
            debug_assert!(twiddles.len() >= ido - 1);

            let input0 = input[0].as_ptr();
            let input1 = input[1].as_ptr();
            let twiddle = twiddles.as_ptr();
            let output0 = output[0].as_mut_ptr();
            let output1 = output[1].as_mut_ptr();

            let mut i = 2;
            while i + 7 < ido {
                let branch0 = unsafe { _mm256_loadu_ps(input0.add(i - 1)) };
                let branch1 = unsafe { _mm256_loadu_ps(input1.add(ido - i - 7)) };
                let branch1 = unsafe { reverse_complex_order(branch1) };
                let sum = _mm256_add_ps(branch0, branch1);
                let difference = _mm256_sub_ps(branch0, branch1);

                let output_branch0 = _mm256_blend_ps(sum, difference, 0b1010_1010);
                let values = _mm256_blend_ps(
                    unsafe { swap_pair(difference) },
                    unsafe { swap_pair(sum) },
                    0b0101_0101,
                );
                let roots = unsafe { _mm256_loadu_ps(twiddle.add(i - 2)) };
                let output_branch1 = unsafe { multiply_pm(roots, values) };
                let output_branch1 = unsafe { swap_pair(output_branch1) };
                unsafe {
                    _mm256_storeu_ps(output0.add(i - 1), output_branch0);
                    _mm256_storeu_ps(output1.add(i - 1), output_branch1);
                }
                i += 8;
            }
            i
        }

        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn forward_radix4_interior(
            input: [&[f32]; 4],
            output: [&mut [f32]; 4],
            ido: usize,
            twiddles: &[f32],
        ) -> usize {
            debug_assert!(ido > 2);
            debug_assert!(ido.is_multiple_of(2));
            debug_assert!(input.iter().all(|branch| branch.len() == ido));
            debug_assert!(output.iter().all(|branch| branch.len() == ido));
            debug_assert!(twiddles.len() >= 3 * (ido - 1));

            let input0 = input[0].as_ptr();
            let input1 = input[1].as_ptr();
            let input2 = input[2].as_ptr();
            let input3 = input[3].as_ptr();
            let twiddle1 = twiddles.as_ptr();
            let twiddle2 = unsafe { twiddle1.add(ido - 1) };
            let twiddle3 = unsafe { twiddle2.add(ido - 1) };
            let output0 = output[0].as_mut_ptr();
            let output1 = output[1].as_mut_ptr();
            let output2 = output[2].as_mut_ptr();
            let output3 = output[3].as_mut_ptr();

            let mut i = 2;
            while i + 6 < ido {
                let x0 = unsafe { _mm256_loadu_ps(input0.add(i - 1)) };
                let x1 = unsafe { _mm256_loadu_ps(input1.add(i - 1)) };
                let x2 = unsafe { _mm256_loadu_ps(input2.add(i - 1)) };
                let x3 = unsafe { _mm256_loadu_ps(input3.add(i - 1)) };
                let roots1 = unsafe { _mm256_loadu_ps(twiddle1.add(i - 2)) };
                let roots2 = unsafe { _mm256_loadu_ps(twiddle2.add(i - 2)) };
                let roots3 = unsafe { _mm256_loadu_ps(twiddle3.add(i - 2)) };
                let x1 = unsafe { multiply_pm(roots1, x1) };
                let x2 = unsafe { multiply_pm(roots2, x2) };
                let x3 = unsafe { multiply_pm(roots3, x3) };

                let tr1 = _mm256_add_ps(unsafe { real(x3) }, unsafe { real(x1) });
                let tr4 = _mm256_sub_ps(unsafe { real(x3) }, unsafe { real(x1) });
                let ti1 = _mm256_add_ps(unsafe { imag(x1) }, unsafe { imag(x3) });
                let ti4 = _mm256_sub_ps(unsafe { imag(x1) }, unsafe { imag(x3) });
                let tr2 = _mm256_add_ps(unsafe { real(x0) }, unsafe { real(x2) });
                let tr3 = _mm256_sub_ps(unsafe { real(x0) }, unsafe { real(x2) });
                let ti2 = _mm256_add_ps(unsafe { imag(x0) }, unsafe { imag(x2) });
                let ti3 = _mm256_sub_ps(unsafe { imag(x0) }, unsafe { imag(x2) });

                let branch0 = unsafe { pack(_mm256_add_ps(tr2, tr1), _mm256_add_ps(ti1, ti2)) };
                let branch3 = unsafe { pack(_mm256_sub_ps(tr2, tr1), _mm256_sub_ps(ti1, ti2)) };
                let branch2 = unsafe { pack(_mm256_add_ps(tr3, ti4), _mm256_add_ps(tr4, ti3)) };
                let branch1 = unsafe { pack(_mm256_sub_ps(tr3, ti4), _mm256_sub_ps(tr4, ti3)) };
                let mirror = ido - i - 7;
                unsafe {
                    _mm256_storeu_ps(output0.add(i - 1), branch0);
                    _mm256_storeu_ps(output2.add(i - 1), branch2);
                    _mm256_storeu_ps(output1.add(mirror), reverse_complex_order(branch1));
                    _mm256_storeu_ps(output3.add(mirror), reverse_complex_order(branch3));
                }
                i += 8;
            }
            i
        }

        #[target_feature(enable = "avx2")]
        pub(super) unsafe fn inverse_radix4_interior(
            input: &[f32],
            output: [&mut [f32]; 4],
            ido: usize,
            twiddles: &[f32],
        ) -> usize {
            debug_assert!(ido > 2);
            debug_assert!(ido.is_multiple_of(2));
            debug_assert_eq!(input.len(), 4 * ido);
            debug_assert!(output.iter().all(|branch| branch.len() == ido));
            debug_assert!(twiddles.len() >= 3 * (ido - 1));

            let input0 = input.as_ptr();
            let input1 = unsafe { input0.add(ido) };
            let input2 = unsafe { input1.add(ido) };
            let input3 = unsafe { input2.add(ido) };
            let twiddle1 = twiddles.as_ptr();
            let twiddle2 = unsafe { twiddle1.add(ido - 1) };
            let twiddle3 = unsafe { twiddle2.add(ido - 1) };
            let output0 = output[0].as_mut_ptr();
            let output1 = output[1].as_mut_ptr();
            let output2 = output[2].as_mut_ptr();
            let output3 = output[3].as_mut_ptr();

            let mut i = 2;
            while i + 6 < ido {
                let ic = ido - i;
                let x0 = unsafe { _mm256_loadu_ps(input0.add(i - 1)) };
                let x3 = unsafe { _mm256_loadu_ps(input3.add(ic - 7)) };
                let x3 = unsafe { reverse_complex_order(x3) };
                let x2 = unsafe { _mm256_loadu_ps(input2.add(i - 1)) };
                let x1 = unsafe { _mm256_loadu_ps(input1.add(ic - 7)) };
                let x1 = unsafe { reverse_complex_order(x1) };

                let sum03 = _mm256_add_ps(x0, x3);
                let diff03 = _mm256_sub_ps(x0, x3);
                let sum21 = _mm256_add_ps(x2, x1);
                let diff21 = _mm256_sub_ps(x2, x1);

                let out0 = unsafe {
                    pack(
                        _mm256_add_ps(real(sum03), real(sum21)),
                        _mm256_add_ps(imag(diff03), imag(diff21)),
                    )
                };
                let values1 = unsafe {
                    pack(
                        _mm256_add_ps(imag(sum03), real(diff21)),
                        _mm256_sub_ps(real(diff03), imag(sum21)),
                    )
                };
                let values2 = unsafe {
                    pack(
                        _mm256_sub_ps(imag(diff03), imag(diff21)),
                        _mm256_sub_ps(real(sum03), real(sum21)),
                    )
                };
                let values3 = unsafe {
                    pack(
                        _mm256_sub_ps(imag(sum03), real(diff21)),
                        _mm256_add_ps(real(diff03), imag(sum21)),
                    )
                };

                let roots1 = unsafe { _mm256_loadu_ps(twiddle1.add(i - 2)) };
                let roots2 = unsafe { _mm256_loadu_ps(twiddle2.add(i - 2)) };
                let roots3 = unsafe { _mm256_loadu_ps(twiddle3.add(i - 2)) };
                let result1 = unsafe { multiply_pm(roots1, values1) };
                let result2 = unsafe { multiply_pm(roots2, values2) };
                let result3 = unsafe { multiply_pm(roots3, values3) };

                unsafe {
                    _mm256_storeu_ps(output0.add(i - 1), out0);
                    _mm256_storeu_ps(output1.add(i - 1), swap_pair(result1));
                    _mm256_storeu_ps(output2.add(i - 1), swap_pair(result2));
                    _mm256_storeu_ps(output3.add(i - 1), swap_pair(result3));
                }
                i += 8;
            }
            i
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
impl SimdScalar for f64 {
    const AVX2_COMPLEX_LANES: usize = 2;
    fn avx2_available() -> bool {
        false
    }
    unsafe fn avx2_first_pass(_: &mut [Complex<Self>], _: &SimdPass<'_, Self>) {
        unreachable!("AVX2 is unavailable on this target")
    }
    unsafe fn avx2_pass(_: &[Complex<Self>], _: &mut [Complex<Self>], _: &SimdPass<'_, Self>) {
        unreachable!("AVX2 is unavailable on this target")
    }
}

#[cfg(not(target_arch = "x86_64"))]
impl SimdScalar for f32 {
    const AVX2_COMPLEX_LANES: usize = 4;
    fn avx2_available() -> bool {
        false
    }
    unsafe fn avx2_first_pass(_: &mut [Complex<Self>], _: &SimdPass<'_, Self>) {
        unreachable!("AVX2 is unavailable on this target")
    }
    unsafe fn avx2_pass(_: &[Complex<Self>], _: &mut [Complex<Self>], _: &SimdPass<'_, Self>) {
        unreachable!("AVX2 is unavailable on this target")
    }
}

#[cfg(all(test, target_arch = "x86_64"))]
mod tests {
    use num_complex::Complex;

    use super::{ComplexGeneralPrimeIdo1Params, RealGeneralPrimeIdo1Params, SimdPass, SimdScalar};

    fn root(index: usize, len: usize, inverse: bool) -> Complex<f64> {
        let sign = if inverse { 1.0 } else { -1.0 };
        let angle = sign * std::f64::consts::TAU * index as f64 / len as f64;
        Complex::new(angle.cos(), angle.sin())
    }

    fn signal(index: usize) -> Complex<f64> {
        let value = index as f64;
        Complex::new((value * 0.37).sin(), (value * 0.19).cos())
    }

    fn assert_close(actual: Complex<f64>, expected: Complex<f64>, context: &str) {
        assert!(
            (actual.re - expected.re).abs() < 2.0e-12,
            "{context}: {actual} != {expected}"
        );
        assert!(
            (actual.im - expected.im).abs() < 2.0e-12,
            "{context}: {actual} != {expected}"
        );
    }

    #[test]
    fn f64_complex_general_prime_radix31_ido1_matches_direct_dfts() {
        if !<f64 as SimdScalar>::avx2_available() {
            return;
        }
        let Some(kernel) = <f64 as SimdScalar>::avx2_general_prime_ido1_kernel() else {
            panic!("AVX2 complex general-prime kernel should be available");
        };
        let radix = 31;
        let half = radix / 2;

        for l1 in [1, 2, 3, 22, 23] {
            let len = radix * l1;
            let roots: Vec<_> = (0..len).map(|index| root(index, len, true)).collect();
            let mut phase_indices = vec![0_u8; half * radix];
            for branch in 1..=half {
                for frequency in 0..radix {
                    phase_indices[(branch - 1) * radix + frequency] =
                        ((branch * frequency) % radix) as u8;
                }
            }
            let original: Vec<_> = (0..len).map(|index| signal(index + l1 * 7)).collect();

            for inverse in [false, true] {
                let scale = if inverse { 0.25 } else { 1.0 };
                let mut storage = vec![Complex::new(-91.0, 37.0); len + 2];
                let (start_guard, tail) = storage.split_at_mut(1);
                let (output, end_guard) = tail.split_at_mut(len);
                // SAFETY: the test checked runtime AVX2, supplies disjoint
                // exact-sized slices, and constructs a complete radix-31
                // phase table and root array.
                unsafe {
                    kernel(
                        &original,
                        output,
                        ComplexGeneralPrimeIdo1Params {
                            l1,
                            radix,
                            roots: &roots,
                            root_stride: l1,
                            phase_indices: &phase_indices,
                            inverse,
                            scale,
                        },
                    )
                }
                assert_eq!(start_guard[0], Complex::new(-91.0, 37.0));
                assert_eq!(end_guard[0], Complex::new(-91.0, 37.0));

                for group in 0..l1 {
                    for frequency in 0..radix {
                        let sign = if inverse { 1.0 } else { -1.0 };
                        let mut expected = Complex::new(0.0, 0.0);
                        for sample in 0..radix {
                            let phase = sample * frequency % radix;
                            let angle = sign * std::f64::consts::TAU * phase as f64 / radix as f64;
                            expected += original[radix * group + sample]
                                * Complex::new(angle.cos(), angle.sin());
                        }
                        expected *= scale;
                        let context = format!(
                            "radix={radix}, l1={l1}, inverse={inverse}, frequency={frequency}, group={group}"
                        );
                        assert_close(output[frequency * l1 + group], expected, &context);
                    }
                }
            }
        }
    }

    #[test]
    fn f64_real_general_prime_ido1_matches_direct_dfts_and_preserves_guards() {
        if !<f64 as SimdScalar>::avx2_available() {
            return;
        }
        let Some(kernel) = <f64 as SimdScalar>::real_general_prime_ido1_kernel() else {
            panic!("AVX2 general-prime kernel should be available");
        };

        for radix in [17, 37, 41, 73, 103, 131] {
            let half = radix / 2;
            let mut cosines = vec![0.0; half * half];
            let mut sines = vec![0.0; half * half];
            for multiplier in 1..=half {
                for frequency in 1..=half {
                    let angle = std::f64::consts::TAU * (multiplier * frequency % radix) as f64
                        / radix as f64;
                    let index = (multiplier - 1) * half + frequency - 1;
                    cosines[index] = angle.cos();
                    sines[index] = angle.sin();
                }
            }

            for l1 in [1, 2, 3, 7] {
                let input: Vec<_> = (0..radix * l1)
                    .map(|index| (index as f64 * 0.173).sin() + (index as f64 * 0.071).cos())
                    .collect();
                let mut forward_storage = vec![1234.5; radix * l1 + 2];
                let (forward_guard, forward_tail) = forward_storage.split_at_mut(1);
                let (forward, forward_end_guard) = forward_tail.split_at_mut(radix * l1);
                // SAFETY: this test runs only after AVX2 detection and provides
                // disjoint, correctly sized input/output and coefficient slices.
                unsafe {
                    kernel(
                        &input,
                        forward,
                        RealGeneralPrimeIdo1Params {
                            l1,
                            radix,
                            cosines: &cosines,
                            sines: &sines,
                            inverse: false,
                            scale: 1.0,
                        },
                    )
                }
                assert_eq!(forward_guard[0], 1234.5);
                assert_eq!(forward_end_guard[0], 1234.5);
                for group in 0..l1 {
                    for frequency in 0..=half {
                        let value = (0..radix).fold(Complex::new(0.0, 0.0), |sum, sample| {
                            let angle = std::f64::consts::TAU * (sample * frequency % radix) as f64
                                / radix as f64;
                            sum + input[sample * l1 + group]
                                * Complex::new(angle.cos(), -angle.sin())
                        });
                        let (actual_re, actual_im) = if frequency == 0 {
                            (forward[group * radix], 0.0)
                        } else {
                            (
                                forward[group * radix + 2 * frequency - 1],
                                forward[group * radix + 2 * frequency],
                            )
                        };
                        let context = format!("forward radix={radix}, l1={l1}, f={frequency}");
                        assert!((actual_re - value.re).abs() < 3.0e-11, "{context}");
                        assert!((actual_im - value.im).abs() < 3.0e-11, "{context}");
                    }
                }

                let mut packed = vec![0.0; radix * l1];
                for group in 0..l1 {
                    packed[group * radix] = (group as f64 * 0.31).sin();
                    for frequency in 1..=half {
                        packed[group * radix + 2 * frequency - 1] =
                            (group as f64 * 0.17 + frequency as f64 * 0.21).cos();
                        packed[group * radix + 2 * frequency] =
                            (group as f64 * 0.13 - frequency as f64 * 0.19).sin();
                    }
                }
                let scale = 0.25;
                let mut inverse_storage = vec![-4321.0; radix * l1 + 2];
                let (inverse_guard, inverse_tail) = inverse_storage.split_at_mut(1);
                let (inverse, inverse_end_guard) = inverse_tail.split_at_mut(radix * l1);
                // SAFETY: this test runs only after AVX2 detection and provides
                // disjoint, correctly sized input/output and coefficient slices.
                unsafe {
                    kernel(
                        &packed,
                        inverse,
                        RealGeneralPrimeIdo1Params {
                            l1,
                            radix,
                            cosines: &cosines,
                            sines: &sines,
                            inverse: true,
                            scale,
                        },
                    )
                }
                assert_eq!(inverse_guard[0], -4321.0);
                assert_eq!(inverse_end_guard[0], -4321.0);
                for group in 0..l1 {
                    for sample in 0..radix {
                        let value = (0..radix).fold(Complex::new(0.0, 0.0), |sum, frequency| {
                            let bin = if frequency <= half {
                                let index = group * radix;
                                if frequency == 0 {
                                    Complex::new(packed[index], 0.0)
                                } else {
                                    Complex::new(
                                        packed[index + 2 * frequency - 1],
                                        packed[index + 2 * frequency],
                                    )
                                }
                            } else {
                                let mirror = radix - frequency;
                                let index = group * radix;
                                Complex::new(
                                    packed[index + 2 * mirror - 1],
                                    -packed[index + 2 * mirror],
                                )
                            };
                            let angle = std::f64::consts::TAU * (sample * frequency % radix) as f64
                                / radix as f64;
                            sum + bin * Complex::new(angle.cos(), angle.sin())
                        });
                        let actual = inverse[sample * l1 + group];
                        let expected = value.re * scale;
                        let context = format!("inverse radix={radix}, l1={l1}, n={sample}");
                        assert!((actual - expected).abs() < 5.0e-11, "{context}");
                    }
                }
            }
        }
    }

    fn root32(index: usize, len: usize, inverse: bool) -> Complex<f32> {
        let sign = if inverse { 1.0 } else { -1.0 };
        let angle = sign * std::f64::consts::TAU * index as f64 / len as f64;
        Complex::new(angle.cos() as f32, angle.sin() as f32)
    }

    fn signal32(index: usize) -> Complex<f32> {
        let value = index as f32;
        Complex::new((value * 0.37).sin(), (value * 0.19).cos())
    }

    fn assert_close32(actual: Complex<f32>, expected: Complex<f32>, context: &str) {
        assert!(
            (actual.re - expected.re).abs() < 3.0e-5,
            "{context}: {actual} != {expected}"
        );
        assert!(
            (actual.im - expected.im).abs() < 3.0e-5,
            "{context}: {actual} != {expected}"
        );
    }

    #[test]
    fn f64_first_pass_matches_scalar_pass_equations() {
        if !<f64 as SimdScalar>::avx2_available() {
            return;
        }
        for radix in [2, 4, 8] {
            for ido in [3, 4, 5] {
                let len = radix * ido;
                let vector_ido = ido - ido % 2;
                for inverse in [false, true] {
                    let mut actual: Vec<_> = (0..len).map(signal).collect();
                    let original = actual.clone();
                    let mut twiddles = vec![Complex::new(0.0, 0.0); radix * ido];
                    for branch in 0..radix {
                        for inner in 0..ido {
                            twiddles[branch * ido + inner] = root(branch * inner, len, true);
                        }
                    }
                    let pass = SimdPass {
                        radix,
                        l1: 1,
                        ido,
                        twiddles: &twiddles,
                        inverse,
                        inverse_scale: 1.0,
                    };
                    unsafe {
                        <f64 as SimdScalar>::avx2_first_pass(&mut actual, &pass);
                    }

                    for inner in 0..vector_ido {
                        for frequency in 0..radix {
                            let mut expected = Complex::new(0.0, 0.0);
                            for branch in 0..radix {
                                expected += original[inner + ido * branch]
                                    * root(branch * frequency * ido, len, inverse);
                            }
                            expected *= root(frequency * inner, len, inverse);
                            assert_close(
                                actual[inner + ido * frequency],
                                expected,
                                &format!(
                                    "radix {radix}, inverse {inverse}, inner {inner}, frequency {frequency}"
                                ),
                            );
                        }
                    }
                    for inner in vector_ido..ido {
                        for frequency in 0..radix {
                            assert_eq!(
                                actual[inner + ido * frequency],
                                original[inner + ido * frequency],
                                "first-pass tail changed: radix={radix}, ido={ido}, inner={inner}, frequency={frequency}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn f64_out_of_place_pass_matches_scalar_pass_equations() {
        if !<f64 as SimdScalar>::avx2_available() {
            return;
        }
        for radix in [2, 4, 8] {
            for ido in [3, 4, 5] {
                let l1 = 3;
                let len = radix * l1 * ido;
                let vector_ido = ido - ido % 2;
                for inverse in [false, true] {
                    let input: Vec<_> = (0..len).map(signal).collect();
                    let mut actual = vec![Complex::new(f64::NAN, f64::NAN); len];
                    let mut twiddles = vec![Complex::new(0.0, 0.0); radix * ido];
                    for branch in 0..radix {
                        for inner in 0..ido {
                            twiddles[branch * ido + inner] = root(branch * l1 * inner, len, true);
                        }
                    }
                    let pass = SimdPass {
                        radix,
                        l1,
                        ido,
                        twiddles: &twiddles,
                        inverse,
                        inverse_scale: 1.0,
                    };
                    unsafe {
                        <f64 as SimdScalar>::avx2_pass(&input, &mut actual, &pass);
                    }

                    for outer in 0..l1 {
                        for inner in 0..vector_ido {
                            for frequency in 0..radix {
                                let mut expected = Complex::new(0.0, 0.0);
                                for branch in 0..radix {
                                    expected += input[inner + ido * (branch + radix * outer)]
                                        * root(branch * frequency * l1 * ido, len, inverse);
                                }
                                expected *= root(frequency * l1 * inner, len, inverse);
                                let output_index = inner + ido * (outer + l1 * frequency);
                                assert_close(
                                    actual[output_index],
                                    expected,
                                    &format!(
                                        "radix {radix}, inverse {inverse}, outer {outer}, inner {inner}, frequency {frequency}"
                                    ),
                                );
                            }
                        }
                    }
                    for outer in 0..l1 {
                        for inner in vector_ido..ido {
                            for frequency in 0..radix {
                                let output_index = inner + ido * (outer + l1 * frequency);
                                assert!(
                                    actual[output_index].re.is_nan()
                                        && actual[output_index].im.is_nan()
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn f64_ido1_radix4_pass_matches_scalar_equations_and_preserves_odd_tail() {
        let Some(kernel) = <f64 as SimdScalar>::avx2_ido1_radix4_pass_kernel() else {
            return;
        };
        if !<f64 as SimdScalar>::avx2_available() {
            return;
        }

        for radix in [4] {
            for l1 in [2, 3, 8, 9] {
                let len = radix * l1;
                let vector_l1 = l1 - l1 % 2;
                let input: Vec<_> = (0..len).map(signal).collect();
                for inverse in [false, true] {
                    let inverse_scale = if inverse { 1.0 / len as f64 } else { 1.0 };
                    let twiddles = vec![Complex::new(1.0, 0.0); radix];
                    let pass = SimdPass {
                        radix,
                        l1,
                        ido: 1,
                        twiddles: &twiddles,
                        inverse,
                        inverse_scale,
                    };
                    let mut actual = vec![Complex::new(f64::NAN, f64::NAN); len];
                    // SAFETY: the kernel is returned only for x86 AVX2 builds,
                    // runtime support is checked above, and the disjoint slices
                    // have the dimensions required for this pass.
                    unsafe { kernel(&input, &mut actual, &pass) };

                    for outer in 0..vector_l1 {
                        for frequency in 0..radix {
                            let mut expected = Complex::new(0.0, 0.0);
                            for branch in 0..radix {
                                expected += input[branch + radix * outer]
                                    * root(branch * frequency * l1, len, inverse);
                            }
                            expected *= inverse_scale;
                            let output_index = outer + l1 * frequency;
                            assert_close(
                                actual[output_index],
                                expected,
                                &format!(
                                    "radix={radix}, l1={l1}, inverse={inverse}, outer={outer}, frequency={frequency}"
                                ),
                            );
                        }
                    }
                    if vector_l1 < l1 {
                        for frequency in 0..radix {
                            assert!(actual[l1 * frequency + vector_l1].re.is_nan());
                            assert!(actual[l1 * frequency + vector_l1].im.is_nan());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn f64_ido1_radix8_pass_matches_scalar_equations() {
        let Some(kernel) = <f64 as SimdScalar>::avx2_ido1_radix8_pass_kernel() else {
            return;
        };
        if !<f64 as SimdScalar>::avx2_available() {
            return;
        }

        for l1 in [2, 8] {
            let radix = 8;
            let len = radix * l1;
            let input: Vec<_> = (0..len).map(signal).collect();
            for inverse in [false, true] {
                let inverse_scale = if inverse { 1.0 / len as f64 } else { 1.0 };
                let twiddles = vec![Complex::new(1.0, 0.0); radix];
                let pass = SimdPass {
                    radix,
                    l1,
                    ido: 1,
                    twiddles: &twiddles,
                    inverse,
                    inverse_scale,
                };
                let mut actual = vec![Complex::new(f64::NAN, f64::NAN); len];
                // SAFETY: runtime AVX2 support and the even l1 contract are
                // checked above; input and output are disjoint.
                unsafe { kernel(&input, &mut actual, &pass) };

                for outer in 0..l1 {
                    for frequency in 0..radix {
                        let expected = (0..radix).fold(Complex::new(0.0, 0.0), |sum, branch| {
                            sum + input[branch + radix * outer]
                                * root(branch * frequency * l1, len, inverse)
                        }) * inverse_scale;
                        assert_close(
                            actual[outer + l1 * frequency],
                            expected,
                            &format!("l1={l1}, inverse={inverse}, frequency={frequency}"),
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn f64_real_forward_radix4_interior_matches_scalar_equations() {
        let Some(kernel) = <f64 as SimdScalar>::real_radix4_forward_interior_kernel() else {
            return;
        };
        for ido in [4, 6, 8, 10, 12, 16, 32, 64] {
            let inputs: [Vec<f64>; 4] = std::array::from_fn(|branch| {
                (0..ido)
                    .map(|index| {
                        let x = (branch * ido + index) as f64;
                        (x * 0.37).sin() + 0.25 * (x * 0.19).cos()
                    })
                    .collect()
            });
            let twiddles: Vec<_> = (0..3 * (ido - 1))
                .map(|index| {
                    let x = index as f64;
                    (x * 0.23).cos() + 0.5 * (x * 0.41).sin()
                })
                .collect();
            let sentinel = f64::NAN;
            let mut actual0 = vec![sentinel; ido];
            let mut actual1 = vec![sentinel; ido];
            let mut actual2 = vec![sentinel; ido];
            let mut actual3 = vec![sentinel; ido];
            let start = unsafe {
                kernel(
                    [&inputs[0], &inputs[1], &inputs[2], &inputs[3]],
                    [&mut actual0, &mut actual1, &mut actual2, &mut actual3],
                    ido,
                    &twiddles,
                )
            };
            let expected_start = 2 + 4 * ((ido - 2) / 4);
            assert_eq!(start, expected_start, "ido={ido}");
            let actual = [actual0, actual1, actual2, actual3];
            let mut expected = [
                vec![sentinel; ido],
                vec![sentinel; ido],
                vec![sentinel; ido],
                vec![sentinel; ido],
            ];

            let multiply = |branch: usize, component: usize, e: f64, f: f64| {
                let offset = (branch - 1) * (ido - 1) + component;
                let c = twiddles[offset];
                let d = twiddles[offset + 1];
                (c * e + d * f, c * f - d * e)
            };
            for i in (2..start).step_by(2) {
                let ic = ido - i;
                let (cr2, ci2) = multiply(1, i - 2, inputs[1][i - 1], inputs[1][i]);
                let (cr3, ci3) = multiply(2, i - 2, inputs[2][i - 1], inputs[2][i]);
                let (cr4, ci4) = multiply(3, i - 2, inputs[3][i - 1], inputs[3][i]);
                let (tr1, tr4) = (cr4 + cr2, cr4 - cr2);
                let (ti1, ti4) = (ci2 + ci4, ci2 - ci4);
                let (tr2, tr3) = (inputs[0][i - 1] + cr3, inputs[0][i - 1] - cr3);
                let (ti2, ti3) = (inputs[0][i] + ci3, inputs[0][i] - ci3);
                expected[0][i - 1] = tr2 + tr1;
                expected[3][ic - 1] = tr2 - tr1;
                expected[0][i] = ti1 + ti2;
                expected[3][ic] = ti1 - ti2;
                expected[2][i - 1] = tr3 + ti4;
                expected[1][ic - 1] = tr3 - ti4;
                expected[2][i] = tr4 + ti3;
                expected[1][ic] = tr4 - ti3;
            }

            for branch in 0..4 {
                for index in 0..ido {
                    if expected[branch][index].is_nan() {
                        assert!(
                            actual[branch][index].is_nan(),
                            "ido={ido}, branch={branch}, untouched index={index} was overwritten"
                        );
                    } else {
                        assert!(
                            (actual[branch][index] - expected[branch][index]).abs() < 3.0e-12,
                            "ido={ido}, branch={branch}, index={index}: {} != {}",
                            actual[branch][index],
                            expected[branch][index]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn f64_real_inverse_radix4_interior_matches_scalar_equations() {
        let Some(kernel) = <f64 as SimdScalar>::real_radix4_interior_kernel() else {
            return;
        };
        for ido in [4, 6, 8, 10, 12, 16, 32, 64] {
            let input: Vec<_> = (0..4 * ido)
                .map(|index| {
                    let x = index as f64;
                    (x * 0.37).sin() + 0.25 * (x * 0.19).cos()
                })
                .collect();
            let twiddles: Vec<_> = (0..3 * (ido - 1))
                .map(|index| {
                    let x = index as f64;
                    (x * 0.23).cos() + 0.5 * (x * 0.41).sin()
                })
                .collect();
            let sentinel = f64::NAN;
            let mut actual0 = vec![sentinel; ido];
            let mut actual1 = vec![sentinel; ido];
            let mut actual2 = vec![sentinel; ido];
            let mut actual3 = vec![sentinel; ido];
            let start = unsafe {
                kernel(
                    &input,
                    [&mut actual0, &mut actual1, &mut actual2, &mut actual3],
                    ido,
                    &twiddles,
                )
            };
            let expected_start = 2 + 4 * ((ido - 2) / 4);
            assert_eq!(start, expected_start, "ido={ido}");
            let actual = [actual0, actual1, actual2, actual3];

            for i in (2..start).step_by(4) {
                let ic = ido - i;
                let input0 = &input[0..ido];
                let input1 = &input[ido..2 * ido];
                let input2 = &input[2 * ido..3 * ido];
                let input3 = &input[3 * ido..4 * ido];
                let (tr2, tr1) = (
                    input0[i - 1] + input3[ic - 1],
                    input0[i - 1] - input3[ic - 1],
                );
                let (ti1, ti2) = (input0[i] + input3[ic], input0[i] - input3[ic]);
                let (tr4, ti3) = (input2[i] + input1[ic], input2[i] - input1[ic]);
                let (tr3, ti4) = (
                    input2[i - 1] + input1[ic - 1],
                    input2[i - 1] - input1[ic - 1],
                );
                let (x0r, cr3) = (tr2 + tr3, tr2 - tr3);
                let (x0i, ci3) = (ti2 + ti3, ti2 - ti3);
                let (cr4, cr2) = (tr1 + tr4, tr1 - tr4);
                let (ci2, ci4) = (ti1 + ti4, ti1 - ti4);
                let multiply = |branch: usize, component: usize, e: f64, f: f64| {
                    let offset = (branch - 1) * (ido - 1) + component;
                    let c = twiddles[offset];
                    let d = twiddles[offset + 1];
                    (c * e + d * f, c * f - d * e)
                };
                let (y1i, y1r) = multiply(1, i - 2, ci2, cr2);
                let (y2i, y2r) = multiply(2, i - 2, ci3, cr3);
                let (y3i, y3r) = multiply(3, i - 2, ci4, cr4);
                let expected = [(x0r, x0i), (y1r, y1i), (y2r, y2i), (y3r, y3i)];
                let next_ic = ido - i - 2;
                let (next_tr2, next_tr1) = (
                    input0[i + 1] + input3[next_ic - 1],
                    input0[i + 1] - input3[next_ic - 1],
                );
                let (next_ti1, next_ti2) = (
                    input0[i + 2] + input3[next_ic],
                    input0[i + 2] - input3[next_ic],
                );
                let (next_tr4, next_ti3) = (
                    input2[i + 2] + input1[next_ic],
                    input2[i + 2] - input1[next_ic],
                );
                let (next_tr3, next_ti4) = (
                    input2[i + 1] + input1[next_ic - 1],
                    input2[i + 1] - input1[next_ic - 1],
                );
                let (next_x0r, next_cr3) = (next_tr2 + next_tr3, next_tr2 - next_tr3);
                let (next_x0i, next_ci3) = (next_ti2 + next_ti3, next_ti2 - next_ti3);
                let (next_cr4, next_cr2) = (next_tr1 + next_tr4, next_tr1 - next_tr4);
                let (next_ci2, next_ci4) = (next_ti1 + next_ti4, next_ti1 - next_ti4);
                let (next_y1i, next_y1r) = multiply(1, i, next_ci2, next_cr2);
                let (next_y2i, next_y2r) = multiply(2, i, next_ci3, next_cr3);
                let (next_y3i, next_y3r) = multiply(3, i, next_ci4, next_cr4);
                let next_expected = [
                    (next_x0r, next_x0i),
                    (next_y1r, next_y1i),
                    (next_y2r, next_y2i),
                    (next_y3r, next_y3i),
                ];
                for branch in 0..4 {
                    for (offset, expected_value) in [expected[branch].0, expected[branch].1]
                        .into_iter()
                        .enumerate()
                    {
                        let observed = actual[branch][i - 1 + offset];
                        assert!(
                            (observed - expected_value).abs() < 3.0e-12,
                            "ido={ido}, i={i}, branch={branch}: {observed} != {expected_value}"
                        );
                    }
                    for (offset, expected_value) in
                        [next_expected[branch].0, next_expected[branch].1]
                            .into_iter()
                            .enumerate()
                    {
                        let observed = actual[branch][i + 1 + offset];
                        assert!(
                            (observed - expected_value).abs() < 3.0e-12,
                            "ido={ido}, i={}, branch={branch}: {observed} != {expected_value}",
                            i + 2
                        );
                    }
                }
            }
            for branch in &actual {
                assert!(branch[0].is_nan(), "ido={ido}, index=0 was overwritten");
                for (index, &value) in branch.iter().enumerate().take(start - 1).skip(1) {
                    assert!(
                        value.is_finite(),
                        "ido={ido}, index={index} was not written"
                    );
                }
                for (index, &value) in branch.iter().enumerate().skip(start - 1) {
                    assert!(
                        value.is_nan(),
                        "ido={ido}, untouched index={index} was overwritten"
                    );
                }
            }
        }
    }

    #[test]
    fn f64_real_inverse_radix2_interior_matches_scalar_equations() {
        let Some(kernel) = <f64 as SimdScalar>::real_radix2_interior_kernel() else {
            return;
        };
        for ido in [4, 6, 8, 10, 12, 16, 32, 64] {
            let inputs: [Vec<f64>; 2] = std::array::from_fn(|branch| {
                (0..ido)
                    .map(|index| {
                        let x = (branch * ido + index) as f64;
                        (x * 0.37).sin() + 0.25 * (x * 0.19).cos()
                    })
                    .collect()
            });
            let twiddles: Vec<_> = (0..ido - 1)
                .map(|index| {
                    let x = index as f64;
                    (x * 0.23).cos() + 0.5 * (x * 0.41).sin()
                })
                .collect();
            let sentinel = f64::NAN;
            let mut actual0 = vec![sentinel; ido];
            let mut actual1 = vec![sentinel; ido];
            let start = unsafe {
                kernel(
                    [&inputs[0], &inputs[1]],
                    [&mut actual0, &mut actual1],
                    ido,
                    &twiddles,
                )
            };
            let expected_start = 2 + 4 * ((ido - 2) / 4);
            assert_eq!(start, expected_start, "ido={ido}");
            let actual = [actual0, actual1];
            let mut expected = [vec![sentinel; ido], vec![sentinel; ido]];

            for i in (2..start).step_by(2) {
                let ic = ido - i;
                let (a, b, c, d) = (
                    inputs[0][i - 1],
                    inputs[1][ic - 1],
                    inputs[0][i],
                    inputs[1][ic],
                );
                let (x0, tr2) = (a + b, a - b);
                let (ti2, x1) = (c + d, c - d);
                let root_re = twiddles[i - 2];
                let root_im = twiddles[i - 1];
                let (y_i, y_im1) = (root_re * ti2 + root_im * tr2, root_re * tr2 - root_im * ti2);
                expected[0][i - 1] = x0;
                expected[0][i] = x1;
                expected[1][i - 1] = y_im1;
                expected[1][i] = y_i;
            }

            for branch in 0..2 {
                for index in 0..ido {
                    if expected[branch][index].is_nan() {
                        assert!(
                            actual[branch][index].is_nan(),
                            "ido={ido}, branch={branch}, untouched index={index} was overwritten"
                        );
                    } else {
                        assert!(
                            (actual[branch][index] - expected[branch][index]).abs() < 3.0e-12,
                            "ido={ido}, branch={branch}, index={index}: {} != {}",
                            actual[branch][index],
                            expected[branch][index]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn f64_real_forward_radix2_interior_matches_scalar_equations() {
        let Some(kernel) = <f64 as SimdScalar>::real_radix2_forward_interior_kernel() else {
            return;
        };
        for ido in [4, 6, 8, 10, 12, 14, 16, 18, 20, 32, 64] {
            let inputs: [Vec<f64>; 2] = std::array::from_fn(|branch| {
                (0..ido)
                    .map(|index| {
                        let x = (branch * ido + index) as f64;
                        (x * 0.37).sin() + 0.25 * (x * 0.19).cos()
                    })
                    .collect()
            });
            let twiddles: Vec<_> = (0..ido - 1)
                .map(|index| {
                    let x = index as f64;
                    (x * 0.23).cos() + 0.5 * (x * 0.41).sin()
                })
                .collect();
            let sentinel = f64::NAN;
            let mut actual0 = vec![sentinel; ido];
            let mut actual1 = vec![sentinel; ido];
            let start = unsafe {
                kernel(
                    [&inputs[0], &inputs[1]],
                    [&mut actual0, &mut actual1],
                    ido,
                    &twiddles,
                )
            };
            let expected_start = 2 + 4 * ((ido - 2) / 4);
            assert_eq!(start, expected_start, "ido={ido}");
            let actual = [actual0, actual1];
            let mut expected = [vec![sentinel; ido], vec![sentinel; ido]];

            for i in (2..start).step_by(2) {
                let ic = ido - i;
                let (twiddled_re, twiddled_im) = (
                    twiddles[i - 2] * inputs[1][i - 1] + twiddles[i - 1] * inputs[1][i],
                    twiddles[i - 2] * inputs[1][i] - twiddles[i - 1] * inputs[1][i - 1],
                );
                expected[0][i - 1] = inputs[0][i - 1] + twiddled_re;
                expected[0][i] = twiddled_im + inputs[0][i];
                expected[1][ic - 1] = inputs[0][i - 1] - twiddled_re;
                expected[1][ic] = twiddled_im - inputs[0][i];
            }

            for branch in 0..2 {
                for index in 0..ido {
                    if expected[branch][index].is_nan() {
                        assert!(
                            actual[branch][index].is_nan(),
                            "ido={ido}, branch={branch}, untouched index={index} was overwritten"
                        );
                    } else {
                        assert!(
                            (actual[branch][index] - expected[branch][index]).abs() < 3.0e-12,
                            "ido={ido}, branch={branch}, index={index}: {} != {}",
                            actual[branch][index],
                            expected[branch][index]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn f32_real_forward_radix2_interior_matches_scalar_equations() {
        let Some(kernel) = <f32 as SimdScalar>::real_radix2_forward_interior_kernel() else {
            return;
        };
        for ido in [4, 6, 8, 10, 12, 14, 16, 18, 24, 32, 64] {
            let inputs: [Vec<f32>; 2] = std::array::from_fn(|branch| {
                (0..ido)
                    .map(|index| {
                        let x = (branch * ido + index) as f32;
                        (x * 0.37).sin() + 0.25 * (x * 0.19).cos()
                    })
                    .collect()
            });
            let twiddles: Vec<_> = (0..ido - 1)
                .map(|index| {
                    let x = index as f32;
                    (x * 0.23).cos() + 0.5 * (x * 0.41).sin()
                })
                .collect();
            let sentinel = f32::NAN;
            let mut actual0 = vec![sentinel; ido];
            let mut actual1 = vec![sentinel; ido];
            let start = unsafe {
                kernel(
                    [&inputs[0], &inputs[1]],
                    [&mut actual0, &mut actual1],
                    ido,
                    &twiddles,
                )
            };
            let expected_start = 2 + 8 * ((ido - 2) / 8);
            assert_eq!(start, expected_start, "ido={ido}");
            let actual = [actual0, actual1];
            let mut expected = [vec![sentinel; ido], vec![sentinel; ido]];

            for i in (2..start).step_by(2) {
                let ic = ido - i;
                let (twiddled_re, twiddled_im) = (
                    twiddles[i - 2] * inputs[1][i - 1] + twiddles[i - 1] * inputs[1][i],
                    twiddles[i - 2] * inputs[1][i] - twiddles[i - 1] * inputs[1][i - 1],
                );
                expected[0][i - 1] = inputs[0][i - 1] + twiddled_re;
                expected[0][i] = twiddled_im + inputs[0][i];
                expected[1][ic - 1] = inputs[0][i - 1] - twiddled_re;
                expected[1][ic] = twiddled_im - inputs[0][i];
            }

            for branch in 0..2 {
                for index in 0..ido {
                    if expected[branch][index].is_nan() {
                        assert!(
                            actual[branch][index].is_nan(),
                            "ido={ido}, branch={branch}, untouched index={index} was overwritten"
                        );
                    } else {
                        assert!(
                            (actual[branch][index] - expected[branch][index]).abs() < 2.0e-5,
                            "ido={ido}, branch={branch}, index={index}: {} != {}",
                            actual[branch][index],
                            expected[branch][index]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn f32_real_inverse_radix2_interior_matches_scalar_equations() {
        let Some(kernel) = <f32 as SimdScalar>::real_radix2_interior_kernel() else {
            return;
        };
        for ido in [4, 6, 8, 10, 12, 14, 16, 18, 24, 32, 64] {
            let inputs: [Vec<f32>; 2] = std::array::from_fn(|branch| {
                (0..ido)
                    .map(|index| {
                        let x = (branch * ido + index) as f32;
                        (x * 0.37).sin() + 0.25 * (x * 0.19).cos()
                    })
                    .collect()
            });
            let twiddles: Vec<_> = (0..ido - 1)
                .map(|index| {
                    let x = index as f32;
                    (x * 0.23).cos() + 0.5 * (x * 0.41).sin()
                })
                .collect();
            let sentinel = f32::NAN;
            let mut actual0 = vec![sentinel; ido];
            let mut actual1 = vec![sentinel; ido];
            let start = unsafe {
                kernel(
                    [&inputs[0], &inputs[1]],
                    [&mut actual0, &mut actual1],
                    ido,
                    &twiddles,
                )
            };
            let expected_start = 2 + 8 * ((ido - 2) / 8);
            assert_eq!(start, expected_start, "ido={ido}");
            let actual = [actual0, actual1];
            let mut expected = [vec![sentinel; ido], vec![sentinel; ido]];

            for i in (2..start).step_by(2) {
                let ic = ido - i;
                let (a, b, c, d) = (
                    inputs[0][i - 1],
                    inputs[1][ic - 1],
                    inputs[0][i],
                    inputs[1][ic],
                );
                let (x0, tr2) = (a + b, a - b);
                let (ti2, x1) = (c + d, c - d);
                let root_re = twiddles[i - 2];
                let root_im = twiddles[i - 1];
                let (y_i, y_im1) = (root_re * ti2 + root_im * tr2, root_re * tr2 - root_im * ti2);
                expected[0][i - 1] = x0;
                expected[0][i] = x1;
                expected[1][i - 1] = y_im1;
                expected[1][i] = y_i;
            }

            for branch in 0..2 {
                for index in 0..ido {
                    if expected[branch][index].is_nan() {
                        assert!(
                            actual[branch][index].is_nan(),
                            "ido={ido}, branch={branch}, untouched index={index} was overwritten"
                        );
                    } else {
                        assert!(
                            (actual[branch][index] - expected[branch][index]).abs() < 3.0e-5,
                            "ido={ido}, branch={branch}, index={index}: {} != {}",
                            actual[branch][index],
                            expected[branch][index]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn f32_vector_passes_match_scalar_pass_equations() {
        if !<f32 as SimdScalar>::avx2_available() {
            return;
        }
        for ido in [4, 5, 7] {
            let l1 = 3;
            let vector_ido = ido - ido % 4;
            for radix in [2, 4, 8] {
                let len = radix * l1 * ido;
                for inverse in [false, true] {
                    let first_len = radix * ido;
                    let mut first_actual: Vec<_> = (0..first_len).map(signal32).collect();
                    let first_original = first_actual.clone();
                    let mut first_twiddles = vec![Complex::new(0.0, 0.0); radix * ido];
                    for branch in 0..radix {
                        for inner in 0..ido {
                            first_twiddles[branch * ido + inner] =
                                root32(branch * inner, first_len, true);
                        }
                    }
                    let first_pass = SimdPass {
                        radix,
                        l1: 1,
                        ido,
                        twiddles: &first_twiddles,
                        inverse,
                        inverse_scale: 1.0,
                    };
                    unsafe {
                        <f32 as SimdScalar>::avx2_first_pass(&mut first_actual, &first_pass);
                    }
                    for inner in 0..vector_ido {
                        for frequency in 0..radix {
                            let mut expected = Complex::new(0.0, 0.0);
                            for branch in 0..radix {
                                expected += first_original[inner + ido * branch]
                                    * root32(branch * frequency * ido, first_len, inverse);
                            }
                            expected *= root32(frequency * inner, first_len, inverse);
                            assert_close32(
                                first_actual[inner + ido * frequency],
                                expected,
                                &format!("first pass radix {radix}, inverse {inverse}"),
                            );
                        }
                    }
                    for inner in vector_ido..ido {
                        for frequency in 0..radix {
                            assert_eq!(
                                first_actual[inner + ido * frequency],
                                first_original[inner + ido * frequency]
                            );
                        }
                    }

                    let input: Vec<_> = (0..len).map(signal32).collect();
                    let mut actual = vec![Complex::new(f32::NAN, f32::NAN); len];
                    let mut twiddles = vec![Complex::new(0.0, 0.0); radix * ido];
                    for branch in 0..radix {
                        for inner in 0..ido {
                            twiddles[branch * ido + inner] = root32(branch * l1 * inner, len, true);
                        }
                    }
                    let pass = SimdPass {
                        radix,
                        l1,
                        ido,
                        twiddles: &twiddles,
                        inverse,
                        inverse_scale: 1.0,
                    };
                    unsafe {
                        <f32 as SimdScalar>::avx2_pass(&input, &mut actual, &pass);
                    }
                    for outer in 0..l1 {
                        for inner in 0..vector_ido {
                            for frequency in 0..radix {
                                let mut expected = Complex::new(0.0, 0.0);
                                for branch in 0..radix {
                                    expected += input[inner + ido * (branch + radix * outer)]
                                        * root32(branch * frequency * l1 * ido, len, inverse);
                                }
                                expected *= root32(frequency * l1 * inner, len, inverse);
                                let output_index = inner + ido * (outer + l1 * frequency);
                                assert_close32(
                                    actual[output_index],
                                    expected,
                                    &format!(
                                        "out pass radix {radix}, inverse {inverse}, outer {outer}, inner {inner}, frequency {frequency}"
                                    ),
                                );
                            }
                        }
                    }
                    for outer in 0..l1 {
                        for inner in vector_ido..ido {
                            for frequency in 0..radix {
                                let output_index = inner + ido * (outer + l1 * frequency);
                                assert!(
                                    actual[output_index].re.is_nan()
                                        && actual[output_index].im.is_nan()
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
