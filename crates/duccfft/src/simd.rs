// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause

use num_complex::Complex;
use num_traits::{Float, FromPrimitive};

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

    /// Executes the first fixed-radix pass over contiguous complex values.
    ///
    /// # Safety
    ///
    /// The caller must confirm AVX2 support and ensure the radix is 2, 4, or
    /// 8, `ido` is a positive multiple of the scalar's vector width, and the
    /// data and twiddle slices match the pass dimensions.
    unsafe fn avx2_first_pass(data: &mut [Complex<Self>], pass: &SimdPass<'_, Self>);

    /// Executes a fixed-radix out-of-place pass over contiguous complex values.
    ///
    /// # Safety
    ///
    /// The caller must confirm AVX2 support and ensure the radix is 2, 4, or
    /// 8, `ido` is a positive multiple of the scalar's vector width, the
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

    use super::{Complex, SimdPass, SimdScalar};

    static AVX2_AVAILABLE: OnceLock<bool> = OnceLock::new();

    pub(super) fn available() -> bool {
        *AVX2_AVAILABLE.get_or_init(|| std::is_x86_feature_detected!("avx2"))
    }

    impl SimdScalar for f64 {
        const AVX2_COMPLEX_LANES: usize = 2;

        fn avx2_available() -> bool {
            available()
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

        use super::super::SimdPass;
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
            for inner in (0..pass.ido).step_by(2) {
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

        #[target_feature(enable = "avx2")]
        unsafe fn pass_fixed<const RADIX: usize>(
            input: &[Complex<f64>],
            output: &mut [Complex<f64>],
            pass: &SimdPass<'_, f64>,
        ) {
            let input_ptr = input.as_ptr();
            let output_ptr = output.as_mut_ptr();
            for outer in 0..pass.l1 {
                for inner in (0..pass.ido).step_by(2) {
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
            for inner in (0..pass.ido).step_by(4) {
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
            for outer in 0..pass.l1 {
                for inner in (0..pass.ido).step_by(4) {
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

    use super::{SimdPass, SimdScalar};

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
        let ido = 4;
        for radix in [2, 4, 8] {
            let len = radix * ido;
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

                for inner in 0..ido {
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
            }
        }
    }

    #[test]
    fn f64_out_of_place_pass_matches_scalar_pass_equations() {
        if !<f64 as SimdScalar>::avx2_available() {
            return;
        }
        let l1 = 3;
        let ido = 4;
        for radix in [2, 4, 8] {
            let len = radix * l1 * ido;
            for inverse in [false, true] {
                let input: Vec<_> = (0..len).map(signal).collect();
                let mut actual = vec![Complex::new(0.0, 0.0); len];
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
                    for inner in 0..ido {
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
            }
        }
    }

    #[test]
    fn f32_vector_passes_match_scalar_pass_equations() {
        if !<f32 as SimdScalar>::avx2_available() {
            return;
        }
        let ido = 4;
        let l1 = 3;
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
                for inner in 0..ido {
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

                let input: Vec<_> = (0..len).map(signal32).collect();
                let mut actual = vec![Complex::new(0.0, 0.0); len];
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
                    for inner in 0..ido {
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
            }
        }
    }
}
