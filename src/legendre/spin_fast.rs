// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * These spin-1/scalar coefficient transforms adapt DUCC's spin1to0 and
 * spin0to1 routines in src/ducc0/sht/sht.cc at revision
 * c1ed01b8fda69d2a378190f0915f1a19762dddbf.
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
    b_coefficients: &[Complex<T>],
    scalar_first: &mut [Complex<T>],
    scalar_second: &mut [Complex<T>],
) {
    debug_assert_eq!(e_coefficients.len(), lmax - m + 1);
    debug_assert_eq!(b_coefficients.len(), lmax - m + 1);
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

        if l > 0 && l <= lmax {
            let derivative_factor = Complex::new(T::zero(), mf) / (lf * (lf + T::one())).sqrt();
            coefficient_first += derivative_factor * b_coefficients[l - m];
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
            coefficient_second -= b_coefficients[l + 1 - m] * next_factor;
        }

        if l <= lmax {
            last_first = e_coefficients[l - m];
            last_second = b_coefficients[l - m];
        } else {
            last_first = Complex::new(T::zero(), T::zero());
            last_second = Complex::new(T::zero(), T::zero());
        }
        scalar_first[l] = -coefficient_first;
        scalar_second[l] = -coefficient_second;
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
            let lm = T::from_usize(l - m).expect("degree difference converts");
            let lp = T::from_usize(l + m).expect("degree sum converts");
            let two_l = T::from_usize(2 * l).expect("twice degree converts");
            let previous_factor =
                (((lp * lm) / ((two_l + T::one()) * (two_l - T::one()))).sqrt()) * (-lf - T::one());
            coefficient_first += last_first * previous_factor;
            coefficient_second -= last_second * previous_factor;
        }
        let l_plus_one = lf + T::one();
        let next_factor = (((l_plus_one + mf) * (l_plus_one - mf)
            / (T::from_usize(2 * l + 3).expect("degree converts")
                * T::from_usize(2 * l + 1).expect("degree converts")))
        .sqrt())
            * lf;
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
                &b_coefficients,
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
                    let norm = spin_alpha[l] * spin::spin_normalization(1, l, false);
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
}
