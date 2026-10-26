// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * The spin-1 coefficient transforms and spin-2-to-scalar transform adapt
 * DUCC routines in src/ducc0/sht/sht.cc at revision
 * c1ed01b8fda69d2a378190f0915f1a19762dddbf. The spin-2 scalar-to-spin
 * transform is the exact transpose of the Rust synthesis conversion.
 *
 * This code is free software; you can redistribute it and/or modify it under
 * the terms of the GNU General Public License as published by the Free
 * Software Foundation, version 2 or (at your option) any later version. It is
 * distributed without any warranty; see the GNU General Public License for
 * more details.
 *
 * Copyright (C) 2020-2025 Max-Planck-Society
 * Author: Martin Reinecke
 */

use num_complex::Complex;
use rustfft::num_traits::{Float, FromPrimitive};

/// Converts spin-1 E/B order coefficients into two scalar coefficient rows.
pub(crate) fn spin1_to_scalar<
    T: Float
        + FromPrimitive
        + std::ops::AddAssign
        + std::ops::SubAssign
        + std::ops::MulAssign
        + std::ops::DivAssign
        + std::ops::RemAssign,
>(
    m: usize,
    lmax: usize,
    e_coefficients: &[Complex<T>],
    b_coefficients: Option<&[Complex<T>]>,
    scalar_first: &mut [Complex<T>],
    scalar_second: &mut [Complex<T>],
) {
    spin1_to_scalar_with_visit(
        m,
        lmax,
        e_coefficients,
        b_coefficients,
        scalar_first,
        scalar_second,
        |_, _, _| {},
    );
}

/// Converts spin-1 rows and visits each emitted degree without changing the
/// recurrence order. The visitor lets order-major synthesis prepare paired
/// coefficients while retaining the complete rows for its checked fallback.
pub(crate) fn spin1_to_scalar_with_visit<
    T: Float
        + FromPrimitive
        + std::ops::AddAssign
        + std::ops::SubAssign
        + std::ops::MulAssign
        + std::ops::DivAssign
        + std::ops::RemAssign,
    F: FnMut(usize, Complex<T>, Complex<T>),
>(
    m: usize,
    lmax: usize,
    e_coefficients: &[Complex<T>],
    b_coefficients: Option<&[Complex<T>]>,
    scalar_first: &mut [Complex<T>],
    scalar_second: &mut [Complex<T>],
    mut visit: F,
) {
    debug_assert_eq!(e_coefficients.len(), lmax - m + 1);
    debug_assert!(b_coefficients.is_none_or(|values| values.len() == lmax - m + 1));
    debug_assert!(scalar_first.len() >= lmax + 2);
    debug_assert!(scalar_second.len() >= lmax + 2);
    scalar_first.fill(Complex::new(T::zero(), T::zero()));
    scalar_second.fill(Complex::new(T::zero(), T::zero()));

    let mf = T::from_usize(m).expect("order converts");
    let mut last_first = Complex::new(T::zero(), T::zero());
    let mut last_second = Complex::new(T::zero(), T::zero());
    for l in m..=lmax + 1 {
        let lf = T::from_usize(l).expect("degree converts");
        let mut coefficient_first = Complex::new(T::zero(), T::zero());
        let mut coefficient_second = Complex::new(T::zero(), T::zero());
        let b_coefficient = if l <= lmax {
            b_coefficients.map_or_else(
                || Complex::new(T::zero(), T::zero()),
                |values| values[l - m],
            )
        } else {
            Complex::new(T::zero(), T::zero())
        };

        if l > 0 && l <= lmax {
            let derivative_factor = Complex::new(T::zero(), mf) / (lf * (lf + T::one())).sqrt();
            coefficient_first += derivative_factor * b_coefficient;
            coefficient_second -= derivative_factor * e_coefficients[l - m];
        }
        if l > m && l > 1 {
            let lm = T::from_usize(l - m).expect("degree difference converts");
            let lp = T::from_usize(l + m).expect("degree sum converts");
            let two_l = T::from_usize(2 * l).expect("twice degree converts");
            let previous_factor = (((lp * lm) / ((two_l + T::one()) * (two_l - T::one()))).sqrt()
                * (lf - T::one()))
                / (lf * (lf - T::one())).sqrt();
            coefficient_first -= last_first * previous_factor;
            coefficient_second -= last_second * previous_factor;
        }
        if l < lmax {
            let l_plus_one = lf + T::one();
            let next_factor = (((l_plus_one + mf) * (l_plus_one - mf)
                / (T::from_usize(2 * l + 3).expect("degree converts")
                    * T::from_usize(2 * l + 1).expect("degree converts")))
            .sqrt()
                * (-lf - T::from_f64(2.0).expect("two converts")))
                / ((l_plus_one * (lf + T::from_f64(2.0).expect("two converts"))).sqrt());
            coefficient_first -= e_coefficients[l + 1 - m] * next_factor;
            coefficient_second -= b_coefficients.map_or_else(
                || Complex::new(T::zero(), T::zero()),
                |values| values[l + 1 - m],
            ) * next_factor;
        }

        if l <= lmax {
            last_first = e_coefficients[l - m];
            last_second = b_coefficient;
        } else {
            last_first = Complex::new(T::zero(), T::zero());
            last_second = Complex::new(T::zero(), T::zero());
        }
        scalar_first[l] = -coefficient_first;
        scalar_second[l] = -coefficient_second;
        visit(l, scalar_first[l], scalar_second[l]);
    }
}

/// Applies the transpose spin-1/scalar coefficient transform to one order.
pub(crate) fn scalar_to_spin1<
    T: Float
        + FromPrimitive
        + std::ops::AddAssign
        + std::ops::SubAssign
        + std::ops::MulAssign
        + std::ops::DivAssign
        + std::ops::RemAssign,
>(
    m: usize,
    lmax: usize,
    scalar_first: &[Complex<T>],
    scalar_second: &[Complex<T>],
    e_coefficients: &mut [Complex<T>],
    b_coefficients: &mut [Complex<T>],
) {
    scalar_to_spin1_inner::<false, T>(
        m,
        lmax,
        scalar_first,
        scalar_second,
        &[],
        e_coefficients,
        b_coefficients,
    );
}

/// Applies the transposed conversion using the scalar recurrence factors that
/// were already prepared for this order.
pub(crate) fn scalar_to_spin1_with_epsilon<
    T: Float
        + FromPrimitive
        + std::ops::AddAssign
        + std::ops::SubAssign
        + std::ops::MulAssign
        + std::ops::DivAssign
        + std::ops::RemAssign,
>(
    m: usize,
    lmax: usize,
    scalar_first: &[Complex<T>],
    scalar_second: &[Complex<T>],
    epsilon: &[T],
    e_coefficients: &mut [Complex<T>],
    b_coefficients: &mut [Complex<T>],
) {
    debug_assert!(epsilon.len() > lmax + 1);
    scalar_to_spin1_inner::<true, T>(
        m,
        lmax,
        scalar_first,
        scalar_second,
        epsilon,
        e_coefficients,
        b_coefficients,
    );
}

#[inline(always)]
fn scalar_to_spin1_inner<
    const USE_PREPARED_EPSILON: bool,
    T: Float
        + FromPrimitive
        + std::ops::AddAssign
        + std::ops::SubAssign
        + std::ops::MulAssign
        + std::ops::DivAssign
        + std::ops::RemAssign,
>(
    m: usize,
    lmax: usize,
    scalar_first: &[Complex<T>],
    scalar_second: &[Complex<T>],
    epsilon: &[T],
    e_coefficients: &mut [Complex<T>],
    b_coefficients: &mut [Complex<T>],
) {
    debug_assert!(scalar_first.len() >= lmax + 2);
    debug_assert!(scalar_second.len() >= lmax + 2);
    debug_assert_eq!(e_coefficients.len(), lmax - m + 1);
    debug_assert_eq!(b_coefficients.len(), lmax - m + 1);

    let mf = T::from_usize(m).expect("order converts");
    let mut last_first = Complex::new(T::zero(), T::zero());
    let mut last_second = Complex::new(T::zero(), T::zero());
    for l in m..=lmax {
        let lf = T::from_usize(l).expect("degree converts");
        let mut coefficient_first = Complex::new(T::zero(), T::zero());
        let mut coefficient_second = Complex::new(T::zero(), T::zero());

        if l > 0 {
            let factor = Complex::new(T::zero(), mf);
            coefficient_first -= factor * scalar_second[l];
            coefficient_second -= factor * scalar_first[l];
        }
        if l > m {
            let previous_factor = if USE_PREPARED_EPSILON {
                epsilon[l] * (-lf - T::one())
            } else {
                let lm = T::from_usize(l - m).expect("degree difference converts");
                let lp = T::from_usize(l + m).expect("degree sum converts");
                let two_l = T::from_usize(2 * l).expect("twice degree converts");
                (((lp * lm) / ((two_l + T::one()) * (two_l - T::one()))).sqrt()) * (-lf - T::one())
            };
            coefficient_first += last_first * previous_factor;
            coefficient_second -= last_second * previous_factor;
        }
        let l_plus_one = lf + T::one();
        let next_factor = if USE_PREPARED_EPSILON {
            epsilon[l + 1] * lf
        } else {
            (((l_plus_one + mf) * (l_plus_one - mf)
                / (T::from_usize(2 * l + 3).expect("degree converts")
                    * T::from_usize(2 * l + 1).expect("degree converts")))
            .sqrt())
                * lf
        };
        coefficient_first += scalar_first[l + 1] * next_factor;
        coefficient_second -= scalar_second[l + 1] * next_factor;

        last_first = scalar_first[l];
        last_second = scalar_second[l];
        let norm = if l > 0 {
            T::one() / (lf * (lf + T::one())).sqrt()
        } else {
            T::zero()
        };
        e_coefficients[l - m] = coefficient_first * norm;
        b_coefficients[l - m] = -coefficient_second * norm;
    }
}

/// Fills DUCC's degree factors used by the spin-2 scalar conversions.
pub(crate) fn fill_spin2_factors<T: Float + FromPrimitive>(
    lmax: usize,
    f1: &mut [T],
    f2: &mut [T],
) {
    debug_assert!(f1.len() > lmax);
    debug_assert!(f2.len() > lmax);
    f1.fill(T::zero());
    f2.fill(T::zero());

    let one = T::one();
    let two = T::from_f64(2.0).expect("two is representable");
    let three = T::from_f64(3.0).expect("three is representable");
    for (degree, factor) in f1.iter_mut().enumerate().take(lmax + 1).skip(1) {
        let l = T::from_usize(degree).expect("degree converts");
        *factor = ((two * l + three) / (two * l + one)) / (l * (l + one) * (l + two) * (l + three));
    }
    for (degree, factor) in f2.iter_mut().enumerate().take(lmax + 1).skip(2) {
        let l = T::from_usize(degree).expect("degree converts");
        *factor = (one / ((l - one) * l * (l + one) * (l + two))).sqrt();
    }
}

/// Converts spin-2 E/B coefficients into two scalar coefficient sequences.
#[expect(
    clippy::too_many_arguments,
    reason = "The conversion reuses the plan factors and worker-owned coefficient buffers."
)]
pub(crate) fn spin2_to_scalar<
    T: Float
        + FromPrimitive
        + std::ops::AddAssign
        + std::ops::SubAssign
        + std::ops::MulAssign
        + std::ops::DivAssign
        + std::ops::RemAssign,
>(
    m: usize,
    lmax: usize,
    e_coefficients: &[Complex<T>],
    b_coefficients: &[Complex<T>],
    scalar_first: &mut [Complex<T>],
    scalar_second: &mut [Complex<T>],
    glm_first: &mut [Complex<T>],
    glm_second: &mut [Complex<T>],
    f1: &[T],
    f2: &[T],
) {
    debug_assert_eq!(e_coefficients.len(), lmax - m + 1);
    debug_assert_eq!(b_coefficients.len(), lmax - m + 1);
    debug_assert!(scalar_first.len() >= lmax + 3);
    debug_assert!(scalar_second.len() >= lmax + 3);
    debug_assert!(glm_first.len() > lmax);
    debug_assert!(glm_second.len() > lmax);
    debug_assert!(f1.len() > lmax);
    debug_assert!(f2.len() > lmax);

    let zero = Complex::new(T::zero(), T::zero());
    let imaginary = Complex::new(T::zero(), T::one());
    scalar_first.fill(zero);
    scalar_second.fill(zero);
    glm_first.fill(zero);
    glm_second.fill(zero);
    for l in m..=lmax {
        let offset = l - m;
        glm_first[l] = -e_coefficients[offset] - imaginary * b_coefficients[offset];
        glm_second[l] = -e_coefficients[offset] + imaginary * b_coefficients[offset];
    }

    let mf = T::from_usize(m).expect("order converts");
    for l in m..=lmax {
        let lf = T::from_usize(l).expect("degree converts");
        let scale = f2[l] * lf * (lf - T::one());
        scalar_first[l] = glm_first[l] * scale;
        scalar_second[l] = glm_second[l] * scale;
    }
    multiply_by_cosine(m, lmax, scalar_first, scalar_second);

    for l in m..=lmax {
        let lf = T::from_usize(l).expect("degree converts");
        let same_degree_scale = f2[l] * two::<T>() * mf * (lf - T::one());
        scalar_first[l] -= glm_first[l] * same_degree_scale;
        scalar_second[l] += glm_second[l] * same_degree_scale;
        if l < lmax {
            let lp1 = lf + T::one();
            let next_scale = two::<T>() * (f1[l] * (lp1 * lp1 - mf * mf)).sqrt();
            scalar_first[l] += glm_first[l + 1] * next_scale;
            scalar_second[l] += glm_second[l + 1] * next_scale;
        }
    }
    multiply_by_cosine(m, lmax + 1, scalar_first, scalar_second);

    for l in m..=lmax {
        let lf = T::from_usize(l).expect("degree converts");
        let same_degree_scale = f2[l] * (two::<T>() * mf * mf - lf * (lf + T::one()));
        scalar_first[l] += glm_first[l] * same_degree_scale;
        scalar_second[l] += glm_second[l] * same_degree_scale;
        if l < lmax {
            let lp1 = lf + T::one();
            let next_scale = two::<T>() * mf * (f1[l] * (lp1 * lp1 - mf * mf)).sqrt();
            scalar_first[l] += glm_first[l + 1] * next_scale;
            scalar_second[l] -= glm_second[l + 1] * next_scale;
        }
    }

    for l in m..=lmax + 2 {
        let first = scalar_first[l];
        let second = scalar_second[l];
        scalar_first[l] = (second + first) * T::from_f64(0.5).expect("one half is representable");
        scalar_second[l] =
            imaginary * (second - first) * T::from_f64(0.5).expect("one half is representable");
    }
}

/// Applies the exact transpose of the spin-2/scalar synthesis conversion.
#[expect(
    clippy::too_many_arguments,
    reason = "The transpose operates in place on worker scratch and reuses plan factors."
)]
pub(crate) fn scalar_to_spin2<
    T: Float
        + FromPrimitive
        + std::ops::AddAssign
        + std::ops::SubAssign
        + std::ops::MulAssign
        + std::ops::DivAssign
        + std::ops::RemAssign,
>(
    m: usize,
    lmax: usize,
    scalar_first: &mut [Complex<T>],
    scalar_second: &mut [Complex<T>],
    glm_first: &mut [Complex<T>],
    glm_second: &mut [Complex<T>],
    f1: &[T],
    f2: &[T],
) {
    debug_assert!(scalar_first.len() >= lmax + 3);
    debug_assert!(scalar_second.len() >= lmax + 3);
    debug_assert!(glm_first.len() >= lmax + 5);
    debug_assert!(glm_second.len() >= lmax + 5);
    debug_assert!(f1.len() > lmax);
    debug_assert!(f2.len() > lmax);

    let zero = Complex::new(T::zero(), T::zero());
    let imaginary = Complex::new(T::zero(), T::one());
    let half = T::from_f64(0.5).expect("one half is representable");
    glm_first.fill(zero);
    glm_second.fill(zero);

    // Transpose the final E/B-to-scalar row mixing.
    for l in m..=lmax + 2 {
        let first = scalar_first[l];
        let second = scalar_second[l];
        scalar_first[l] = (first + imaginary * second) * half;
        scalar_second[l] = (first - imaginary * second) * half;
    }

    let mf = T::from_usize(m).expect("order converts");

    // Transpose the final diagonal and next-degree terms in spin2_to_scalar.
    for l in m..=lmax {
        let lf = T::from_usize(l).expect("degree converts");
        let diagonal_scale = f2[l] * (two::<T>() * mf * mf - lf * (lf + T::one()));
        glm_first[l] += scalar_first[l] * diagonal_scale;
        glm_second[l] += scalar_second[l] * diagonal_scale;
        if l < lmax {
            let lp1 = lf + T::one();
            let next_scale = two::<T>() * mf * (f1[l] * (lp1 * lp1 - mf * mf)).sqrt();
            glm_first[l + 1] += scalar_first[l] * next_scale;
            glm_second[l + 1] -= scalar_second[l] * next_scale;
        }
    }

    // Transpose the second cosine multiplication. Its synthesis output is
    // consumed only through degree lmax, while its input extends through lmax+1.
    transpose_multiply_by_cosine_in_place(m, lmax + 1, scalar_first, scalar_second);

    // Transpose the middle diagonal and next-degree terms.
    for l in m..=lmax {
        let lf = T::from_usize(l).expect("degree converts");
        let diagonal_scale = -two::<T>() * mf * (lf - T::one()) * f2[l];
        glm_first[l] += scalar_first[l] * diagonal_scale;
        glm_second[l] -= scalar_second[l] * diagonal_scale;
        if l < lmax {
            let lp1 = lf + T::one();
            let next_scale = two::<T>() * (f1[l] * (lp1 * lp1 - mf * mf)).sqrt();
            glm_first[l + 1] += scalar_first[l] * next_scale;
            glm_second[l + 1] += scalar_second[l] * next_scale;
        }
    }

    // Transpose the first cosine multiplication, then the initial diagonal.
    transpose_multiply_by_cosine_in_place(m, lmax, scalar_first, scalar_second);
    for l in m..=lmax {
        let lf = T::from_usize(l).expect("degree converts");
        let initial_scale = f2[l] * lf * (lf - T::one());
        glm_first[l] += scalar_first[l] * initial_scale;
        glm_second[l] += scalar_second[l] * initial_scale;
    }

    // Transpose glm0=-E-iB and glm1=-E+iB.
    for l in m..=lmax {
        scalar_first[l] = -(glm_first[l] + glm_second[l]);
        scalar_second[l] = imaginary * (glm_first[l] - glm_second[l]);
    }
}

fn transpose_multiply_by_cosine_in_place<T: Float + FromPrimitive>(
    m: usize,
    input_max: usize,
    first: &mut [Complex<T>],
    second: &mut [Complex<T>],
) {
    debug_assert!(first.len() > input_max + 1);
    debug_assert!(second.len() > input_max + 1);
    let mf = T::from_usize(m).expect("order converts");
    let one = T::one();
    let two = two::<T>();
    let mut previous_first = Complex::new(T::zero(), T::zero());
    let mut previous_second = Complex::new(T::zero(), T::zero());
    let mut current_first = first[m];
    let mut current_second = second[m];
    let mut next_first = first[m + 1];
    let mut next_second = second[m + 1];

    for l in m..=input_max {
        let lf = T::from_usize(l).expect("degree converts");
        let mut result_first = Complex::new(T::zero(), T::zero());
        let mut result_second = Complex::new(T::zero(), T::zero());
        if l > m {
            let lower = ((lf + mf) * (lf - mf) / ((two * lf + one) * (two * lf - one))).sqrt();
            result_first = result_first + previous_first * lower;
            result_second = result_second + previous_second * lower;
        }
        let lp1 = lf + one;
        let upper = ((lp1 + mf) * (lp1 - mf) / ((two * lp1 + one) * (two * lp1 - one))).sqrt();
        result_first = result_first + next_first * upper;
        result_second = result_second + next_second * upper;

        first[l] = result_first;
        second[l] = result_second;
        previous_first = current_first;
        previous_second = current_second;
        current_first = next_first;
        current_second = next_second;
        next_first = if l + 2 < first.len() {
            first[l + 2]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        next_second = if l + 2 < second.len() {
            second[l + 2]
        } else {
            Complex::new(T::zero(), T::zero())
        };
    }
}

fn multiply_by_cosine<T: Float + FromPrimitive>(
    m: usize,
    lmax: usize,
    first: &mut [Complex<T>],
    second: &mut [Complex<T>],
) {
    debug_assert!(first.len() > lmax + 1);
    debug_assert!(second.len() > lmax + 1);
    let mf = T::from_usize(m).expect("order converts");
    let one = T::one();
    let two = two::<T>();
    let mut previous_first = Complex::new(T::zero(), T::zero());
    let mut previous_second = Complex::new(T::zero(), T::zero());
    for l in m..=lmax + 1 {
        let lf = T::from_usize(l).expect("degree converts");
        let mut next_first = Complex::new(T::zero(), T::zero());
        let mut next_second = Complex::new(T::zero(), T::zero());
        if l > m {
            let factor = ((lf + mf) * (lf - mf) / ((two * lf + one) * (two * lf - one))).sqrt();
            next_first = next_first + previous_first * factor;
            next_second = next_second + previous_second * factor;
        }
        if l < lmax {
            let lp1 = lf + one;
            let factor = ((lp1 + mf) * (lp1 - mf) / ((two * lp1 + one) * (two * lp1 - one))).sqrt();
            next_first = next_first + first[l + 1] * factor;
            next_second = next_second + second[l + 1] * factor;
        }
        previous_first = first[l];
        previous_second = second[l];
        first[l] = next_first;
        second[l] = next_second;
    }
}

fn two<T: FromPrimitive>() -> T {
    T::from_f64(2.0).expect("two is representable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulp::Arch;

    use crate::legendre::{LATITUDE_BLOCK_SIZE, scalar, spin};

    #[test]
    fn spin1_scalar_conversion_matches_the_spin_recurrence() {
        let lmax = 16;
        let theta = [0.11, 0.43, 1.17, 1.57, 2.61, 3.02];
        let sin_theta: Vec<_> = theta.iter().map(|angle| angle.sin()).collect();
        let cos_theta: Vec<_> = theta.iter().map(|angle| angle.cos()).collect();

        for m in 0..=lmax {
            let degree_count = lmax - m + 1;
            let e_coefficients: Vec<_> = (m..=lmax)
                .map(|l| {
                    if l == 0 {
                        Complex::new(0.0, 0.0)
                    } else {
                        Complex::new(
                            ((l * 7 + m * 3) % 17) as f64 * 0.031,
                            if m == 0 {
                                0.0
                            } else {
                                ((l * 5 + m * 11) % 13) as f64 * 0.023
                            },
                        )
                    }
                })
                .collect();
            let b_coefficients: Vec<_> = (m..=lmax)
                .map(|l| {
                    if l == 0 {
                        Complex::new(0.0, 0.0)
                    } else {
                        Complex::new(
                            ((l * 3 + m * 5) % 19) as f64 * 0.027,
                            if m == 0 {
                                0.0
                            } else {
                                ((l * 13 + m * 7) % 11) as f64 * 0.019
                            },
                        )
                    }
                })
                .collect();
            let mut scalar_first = vec![Complex::new(0.0, 0.0); lmax + 2];
            let mut scalar_second = vec![Complex::new(0.0, 0.0); lmax + 2];
            spin1_to_scalar(
                m,
                lmax,
                &e_coefficients,
                Some(&b_coefficients),
                &mut scalar_first,
                &mut scalar_second,
            );

            let mut epsilon = vec![0.0; lmax + 3];
            scalar::fill_epsilon(m, lmax + 1, &mut epsilon);
            let norms = scalar::sectoral_normalizations(m).unwrap();
            let mut scalar_previous = vec![0.0; LATITUDE_BLOCK_SIZE];
            let mut scalar_current = vec![0.0; LATITUDE_BLOCK_SIZE];
            let mut scalar_next = vec![0.0; LATITUDE_BLOCK_SIZE];
            let mut scalar_exponents = vec![0; LATITUDE_BLOCK_SIZE];

            let mut spin_alpha = vec![0.0; lmax + 2];
            let mut spin_a = vec![0.0; lmax + 2];
            let mut spin_b = vec![0.0; lmax + 2];
            let spin_initial =
                spin::fill_spin_factors(1, m, lmax, &mut spin_alpha, &mut spin_a, &mut spin_b)
                    .unwrap();
            let mut spin_previous_plus = vec![0.0; theta.len()];
            let mut spin_current_plus = vec![0.0; theta.len()];
            let mut spin_next_plus = vec![0.0; theta.len()];
            let mut spin_exponents_plus = vec![0; theta.len()];
            let mut spin_previous_minus = vec![0.0; theta.len()];
            let mut spin_current_minus = vec![0.0; theta.len()];
            let mut spin_next_minus = vec![0.0; theta.len()];
            let mut spin_exponents_minus = vec![0; theta.len()];
            let mut scalar_first_sum = vec![Complex::new(0.0, 0.0); theta.len()];
            let mut scalar_second_sum = vec![Complex::new(0.0, 0.0); theta.len()];
            let mut spin_first_sum = vec![Complex::new(0.0, 0.0); theta.len()];
            let mut spin_second_sum = vec![Complex::new(0.0, 0.0); theta.len()];

            scalar::for_each_y_lm_dispatched(
                Arch::Scalar,
                m,
                lmax + 1,
                &sin_theta,
                &cos_theta,
                norms[m],
                &epsilon,
                &mut scalar_previous,
                &mut scalar_current,
                &mut scalar_next,
                &mut scalar_exponents,
                |l, ring_start, basis, exponents| {
                    for lane in 0..basis.len() {
                        scalar_first_sum[ring_start + lane] += scalar::multiply_complex_scaled(
                            scalar_first[l],
                            basis[lane],
                            exponents[lane],
                        );
                        scalar_second_sum[ring_start + lane] += scalar::multiply_complex_scaled(
                            scalar_second[l],
                            basis[lane],
                            exponents[lane],
                        );
                    }
                    Ok(())
                },
            )
            .unwrap();
            spin::for_each_spin_lm_dispatched(
                Arch::Scalar,
                &spin_initial,
                lmax,
                &sin_theta,
                &cos_theta,
                &spin_a,
                &spin_b,
                &mut spin_previous_plus,
                &mut spin_current_plus,
                &mut spin_next_plus,
                &mut spin_exponents_plus,
                &mut spin_previous_minus,
                &mut spin_current_minus,
                &mut spin_next_minus,
                &mut spin_exponents_minus,
                |l, ring_start, plus, plus_exp, minus, minus_exp| {
                    let norm = spin_alpha[l]
                        * crate::legendre::spin_factors::spin_normalization::<f64>(1, l, false);
                    for lane in 0..plus.len() {
                        let plus_value = spin::scaled_basis_value(plus[lane], plus_exp[lane], norm);
                        let minus_value =
                            spin::scaled_basis_value(minus[lane], minus_exp[lane], norm);
                        let even = plus_value + minus_value;
                        let odd = plus_value - minus_value;
                        let ring = ring_start + lane;
                        spin_first_sum[ring] += e_coefficients[l - m] * even
                            + Complex::new(0.0, -1.0) * b_coefficients[l - m] * odd;
                        spin_second_sum[ring] +=
                            Complex::new(0.0, 1.0) * e_coefficients[l - m] * odd
                                + b_coefficients[l - m] * even;
                    }
                    Ok(())
                },
            )
            .unwrap();

            for ring in 0..theta.len() {
                let fast_first = scalar_first_sum[ring] / sin_theta[ring];
                let fast_second = scalar_second_sum[ring] / sin_theta[ring];
                let first_error = (fast_first - spin_first_sum[ring]).norm();
                let second_error = (fast_second - spin_second_sum[ring]).norm();
                assert!(
                    first_error <= 3e-12,
                    "m={m}, ring={ring}, first scalar={fast_first:?}, spin={:?}, error={first_error:.3e}",
                    spin_first_sum[ring]
                );
                assert!(
                    second_error <= 3e-12,
                    "m={m}, ring={ring}, second scalar={fast_second:?}, spin={:?}, error={second_error:.3e}",
                    spin_second_sum[ring]
                );
            }

            let scalar_test_first: Vec<_> = (0..=lmax + 1)
                .map(|l| Complex::new(l as f64 * 0.013 - 0.04, l as f64 * 0.017 + 0.03))
                .collect();
            let scalar_test_second: Vec<_> = (0..=lmax + 1)
                .map(|l| Complex::new(l as f64 * -0.019 + 0.02, l as f64 * 0.011 - 0.01))
                .collect();
            let mut adjoint_e = vec![Complex::new(0.0, 0.0); degree_count];
            let mut adjoint_b = vec![Complex::new(0.0, 0.0); degree_count];
            scalar_to_spin1(
                m,
                lmax,
                &scalar_test_first,
                &scalar_test_second,
                &mut adjoint_e,
                &mut adjoint_b,
            );
            let mut prepared_e = vec![Complex::new(0.0, 0.0); degree_count];
            let mut prepared_b = vec![Complex::new(0.0, 0.0); degree_count];
            scalar_to_spin1_with_epsilon(
                m,
                lmax,
                &scalar_test_first,
                &scalar_test_second,
                &epsilon,
                &mut prepared_e,
                &mut prepared_b,
            );
            for (prepared, reference) in prepared_e
                .iter()
                .chain(&prepared_b)
                .zip(adjoint_e.iter().chain(&adjoint_b))
            {
                assert!((*prepared - *reference).norm() <= 2.0e-14);
            }
            let mut scalar_inner_product = Complex::new(0.0, 0.0);
            for l in m..=lmax + 1 {
                scalar_inner_product += scalar_first[l].conj() * scalar_test_first[l];
                scalar_inner_product += scalar_second[l].conj() * scalar_test_second[l];
            }
            let mut spin_inner_product = Complex::new(0.0, 0.0);
            for l in 0..degree_count {
                spin_inner_product += e_coefficients[l].conj() * adjoint_e[l];
                spin_inner_product += b_coefficients[l].conj() * adjoint_b[l];
            }
            assert!(
                (scalar_inner_product - spin_inner_product).norm() < 1e-11,
                "spin/scalar coefficient transforms are not adjoints for m={m}: {scalar_inner_product:?} != {spin_inner_product:?}"
            );
        }
    }

    #[test]
    fn spin1_e_only_conversion_matches_zero_b_at_low_degree_boundaries() {
        for lmax in 1..=16 {
            for m in 0..=lmax {
                let e_coefficients: Vec<_> = (m..=lmax)
                    .map(|l| {
                        if l == 0 {
                            Complex::new(0.0, 0.0)
                        } else {
                            Complex::new(
                                (l * 7 + m * 3) as f64 * 0.013,
                                if m == 0 {
                                    0.0
                                } else {
                                    (l * 5 + m) as f64 * 0.017
                                },
                            )
                        }
                    })
                    .collect();
                let zero_b = vec![Complex::new(0.0, 0.0); lmax - m + 1];
                let mut actual_first = vec![Complex::new(0.0, 0.0); lmax + 2];
                let mut actual_second = actual_first.clone();
                let mut expected_first = actual_first.clone();
                let mut expected_second = actual_first.clone();

                spin1_to_scalar(
                    m,
                    lmax,
                    &e_coefficients,
                    None,
                    &mut actual_first,
                    &mut actual_second,
                );
                spin1_to_scalar(
                    m,
                    lmax,
                    &e_coefficients,
                    Some(&zero_b),
                    &mut expected_first,
                    &mut expected_second,
                );

                assert_eq!(
                    actual_first, expected_first,
                    "first row for lmax={lmax}, m={m}"
                );
                assert_eq!(
                    actual_second, expected_second,
                    "second row for lmax={lmax}, m={m}"
                );
            }
        }
    }

    #[test]
    fn spin2_scalar_conversion_matches_the_spin_recurrence_and_adjoint() {
        let lmax = 16;
        let scalar_lmax = lmax + 2;
        let theta = [0.11, 0.43, 1.17, 1.57, 2.61, 3.02];
        let sin_theta: Vec<_> = theta.iter().map(|angle| angle.sin()).collect();
        let cos_theta: Vec<_> = theta.iter().map(|angle| angle.cos()).collect();
        let norms = scalar::sectoral_normalizations(scalar_lmax).unwrap();
        let mut f1 = vec![0.0; lmax + 1];
        let mut f2 = vec![0.0; lmax + 1];
        fill_spin2_factors(lmax, &mut f1, &mut f2);

        for m in 0..=lmax {
            let degree_count = lmax - m + 1;
            let e_coefficients: Vec<_> = (m..=lmax)
                .map(|l| {
                    if l < 2 {
                        Complex::new(0.0, 0.0)
                    } else {
                        Complex::new(
                            ((l * 7 + m * 3) % 17) as f64 * 0.031,
                            if m == 0 {
                                0.0
                            } else {
                                ((l * 5 + m * 11) % 13) as f64 * 0.023
                            },
                        )
                    }
                })
                .collect();
            let b_coefficients: Vec<_> = (m..=lmax)
                .map(|l| {
                    if l < 2 {
                        Complex::new(0.0, 0.0)
                    } else {
                        Complex::new(
                            ((l * 3 + m * 5) % 19) as f64 * 0.027,
                            if m == 0 {
                                0.0
                            } else {
                                ((l * 13 + m * 7) % 11) as f64 * 0.019
                            },
                        )
                    }
                })
                .collect();
            let mut scalar_first = vec![Complex::new(0.0, 0.0); scalar_lmax + 1];
            let mut scalar_second = vec![Complex::new(0.0, 0.0); scalar_lmax + 1];
            let mut glm_first = vec![Complex::new(0.0, 0.0); lmax + 5];
            let mut glm_second = vec![Complex::new(0.0, 0.0); lmax + 5];
            spin2_to_scalar(
                m,
                lmax,
                &e_coefficients,
                &b_coefficients,
                &mut scalar_first,
                &mut scalar_second,
                &mut glm_first,
                &mut glm_second,
                &f1,
                &f2,
            );

            let mut epsilon = vec![0.0; scalar_lmax + 4];
            scalar::fill_epsilon(m, scalar_lmax, &mut epsilon);
            let mut scalar_previous = vec![0.0; LATITUDE_BLOCK_SIZE];
            let mut scalar_current = vec![0.0; LATITUDE_BLOCK_SIZE];
            let mut scalar_next = vec![0.0; LATITUDE_BLOCK_SIZE];
            let mut scalar_exponents = vec![0; LATITUDE_BLOCK_SIZE];
            let mut scalar_first_sum = vec![Complex::new(0.0, 0.0); theta.len()];
            let mut scalar_second_sum = vec![Complex::new(0.0, 0.0); theta.len()];
            scalar::for_each_y_lm_dispatched(
                Arch::Scalar,
                m,
                scalar_lmax,
                &sin_theta,
                &cos_theta,
                norms[m],
                &epsilon,
                &mut scalar_previous,
                &mut scalar_current,
                &mut scalar_next,
                &mut scalar_exponents,
                |l, ring_start, basis, exponents| {
                    for lane in 0..basis.len() {
                        scalar_first_sum[ring_start + lane] += scalar::multiply_complex_scaled(
                            scalar_first[l],
                            basis[lane],
                            exponents[lane],
                        );
                        scalar_second_sum[ring_start + lane] += scalar::multiply_complex_scaled(
                            scalar_second[l],
                            basis[lane],
                            exponents[lane],
                        );
                    }
                    Ok(())
                },
            )
            .unwrap();

            let mut spin_alpha = vec![0.0; lmax + 2];
            let mut spin_a = vec![0.0; lmax + 2];
            let mut spin_b = vec![0.0; lmax + 2];
            let spin_initial =
                spin::fill_spin_factors(2, m, lmax, &mut spin_alpha, &mut spin_a, &mut spin_b)
                    .unwrap();
            let mut spin_previous_plus = vec![0.0; theta.len()];
            let mut spin_current_plus = vec![0.0; theta.len()];
            let mut spin_next_plus = vec![0.0; theta.len()];
            let mut spin_exponents_plus = vec![0; theta.len()];
            let mut spin_previous_minus = vec![0.0; theta.len()];
            let mut spin_current_minus = vec![0.0; theta.len()];
            let mut spin_next_minus = vec![0.0; theta.len()];
            let mut spin_exponents_minus = vec![0; theta.len()];
            let mut spin_first_sum = vec![Complex::new(0.0, 0.0); theta.len()];
            let mut spin_second_sum = vec![Complex::new(0.0, 0.0); theta.len()];
            spin::for_each_spin_lm_dispatched(
                Arch::Scalar,
                &spin_initial,
                lmax,
                &sin_theta,
                &cos_theta,
                &spin_a,
                &spin_b,
                &mut spin_previous_plus,
                &mut spin_current_plus,
                &mut spin_next_plus,
                &mut spin_exponents_plus,
                &mut spin_previous_minus,
                &mut spin_current_minus,
                &mut spin_next_minus,
                &mut spin_exponents_minus,
                |l, ring_start, plus, plus_exp, minus, minus_exp| {
                    let norm = spin_alpha[l]
                        * crate::legendre::spin_factors::spin_normalization::<f64>(2, l, false);
                    for lane in 0..plus.len() {
                        let plus_value = spin::scaled_basis_value(plus[lane], plus_exp[lane], norm);
                        let minus_value =
                            spin::scaled_basis_value(minus[lane], minus_exp[lane], norm);
                        let even = plus_value + minus_value;
                        let odd = plus_value - minus_value;
                        let ring = ring_start + lane;
                        spin_first_sum[ring] += e_coefficients[l - m] * even
                            + Complex::new(0.0, -1.0) * b_coefficients[l - m] * odd;
                        spin_second_sum[ring] +=
                            Complex::new(0.0, 1.0) * e_coefficients[l - m] * odd
                                + b_coefficients[l - m] * even;
                    }
                    Ok(())
                },
            )
            .unwrap();

            for ring in 0..theta.len() {
                let inverse_sine_squared = 1.0 / (sin_theta[ring] * sin_theta[ring]);
                let fast_first = scalar_first_sum[ring] * inverse_sine_squared;
                let fast_second = scalar_second_sum[ring] * inverse_sine_squared;
                let first_error = (fast_first - spin_first_sum[ring]).norm();
                let second_error = (fast_second - spin_second_sum[ring]).norm();
                assert!(
                    first_error <= 3e-11,
                    "m={m}, ring={ring}, first scalar={fast_first:?}, spin={:?}, error={first_error:.3e}",
                    spin_first_sum[ring]
                );
                assert!(
                    second_error <= 3e-11,
                    "m={m}, ring={ring}, second scalar={fast_second:?}, spin={:?}, error={second_error:.3e}",
                    spin_second_sum[ring]
                );
            }

            let scalar_test_first: Vec<_> = (0..=scalar_lmax)
                .map(|l| {
                    Complex::new(
                        l as f64 * 0.013 - 0.04,
                        if m == 0 { 0.0 } else { l as f64 * 0.017 + 0.03 },
                    )
                })
                .collect();
            let scalar_test_second: Vec<_> = (0..=scalar_lmax)
                .map(|l| {
                    Complex::new(
                        l as f64 * -0.019 + 0.02,
                        if m == 0 { 0.0 } else { l as f64 * 0.011 - 0.01 },
                    )
                })
                .collect();
            let mut adjoint_first = scalar_test_first.clone();
            let mut adjoint_second = scalar_test_second.clone();
            scalar_to_spin2(
                m,
                lmax,
                &mut adjoint_first,
                &mut adjoint_second,
                &mut glm_first,
                &mut glm_second,
                &f1,
                &f2,
            );
            for l in m..=lmax {
                let index = l - m;
                let mut basis_e = vec![Complex::new(0.0, 0.0); degree_count];
                let basis_b = vec![Complex::new(0.0, 0.0); degree_count];
                basis_e[index] = Complex::new(1.0, 0.0);
                let mut basis_first = vec![Complex::new(0.0, 0.0); scalar_lmax + 1];
                let mut basis_second = vec![Complex::new(0.0, 0.0); scalar_lmax + 1];
                let mut basis_glm_first = vec![Complex::new(0.0, 0.0); lmax + 5];
                let mut basis_glm_second = vec![Complex::new(0.0, 0.0); lmax + 5];
                spin2_to_scalar(
                    m,
                    lmax,
                    &basis_e,
                    &basis_b,
                    &mut basis_first,
                    &mut basis_second,
                    &mut basis_glm_first,
                    &mut basis_glm_second,
                    &f1,
                    &f2,
                );
                let expected_e: Complex<f64> = (m..=scalar_lmax)
                    .map(|degree| {
                        basis_first[degree].conj() * scalar_test_first[degree]
                            + basis_second[degree].conj() * scalar_test_second[degree]
                    })
                    .sum();
                assert!(
                    (adjoint_first[l] - expected_e).norm() < 3e-10,
                    "transpose E mismatch m={m}, l={l}: actual={:?}, expected={expected_e:?}",
                    adjoint_first[l]
                );

                let basis_e = vec![Complex::new(0.0, 0.0); degree_count];
                let mut basis_b = vec![Complex::new(0.0, 0.0); degree_count];
                basis_b[index] = Complex::new(1.0, 0.0);
                let mut basis_first = vec![Complex::new(0.0, 0.0); scalar_lmax + 1];
                let mut basis_second = vec![Complex::new(0.0, 0.0); scalar_lmax + 1];
                let mut basis_glm_first = vec![Complex::new(0.0, 0.0); lmax + 5];
                let mut basis_glm_second = vec![Complex::new(0.0, 0.0); lmax + 5];
                spin2_to_scalar(
                    m,
                    lmax,
                    &basis_e,
                    &basis_b,
                    &mut basis_first,
                    &mut basis_second,
                    &mut basis_glm_first,
                    &mut basis_glm_second,
                    &f1,
                    &f2,
                );
                let expected_b: Complex<f64> = (m..=scalar_lmax)
                    .map(|degree| {
                        basis_first[degree].conj() * scalar_test_first[degree]
                            + basis_second[degree].conj() * scalar_test_second[degree]
                    })
                    .sum();
                assert!(
                    (adjoint_second[l] - expected_b).norm() < 3e-10,
                    "transpose B mismatch m={m}, l={l}: actual={:?}, expected={expected_b:?}",
                    adjoint_second[l]
                );
            }
            let mut scalar_inner_product = Complex::new(0.0, 0.0);
            for l in m..=scalar_lmax {
                scalar_inner_product += scalar_first[l].conj() * scalar_test_first[l];
                scalar_inner_product += scalar_second[l].conj() * scalar_test_second[l];
            }
            let mut spin_inner_product = Complex::new(0.0, 0.0);
            for l in m..=lmax {
                let index = l - m;
                spin_inner_product += e_coefficients[index].conj() * adjoint_first[l];
                spin_inner_product += b_coefficients[index].conj() * adjoint_second[l];
            }
            assert!(
                (scalar_inner_product - spin_inner_product).norm() < 3e-10,
                "spin/scalar coefficient transforms are not adjoints for m={m}: {scalar_inner_product:?} != {spin_inner_product:?}"
            );
        }
    }
}
