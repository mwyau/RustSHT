use super::*;
use pulp::Arch;

use crate::legendre::{LATITUDE_BLOCK_SIZE, scalar, spin, spin_factors};

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
            spin_factors::fill_spin_factors(1, m, lmax, &mut spin_alpha, &mut spin_a, &mut spin_b)
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
                let norm = spin_alpha[l] * spin_factors::spin_normalization::<f64>(1, l, false);
                for lane in 0..plus.len() {
                    let plus_value = spin::scaled_basis_value(plus[lane], plus_exp[lane], norm);
                    let minus_value = spin::scaled_basis_value(minus[lane], minus_exp[lane], norm);
                    let even = plus_value + minus_value;
                    let odd = plus_value - minus_value;
                    let ring = ring_start + lane;
                    spin_first_sum[ring] += e_coefficients[l - m] * even
                        + Complex::new(0.0, -1.0) * b_coefficients[l - m] * odd;
                    spin_second_sum[ring] += Complex::new(0.0, 1.0) * e_coefficients[l - m] * odd
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
            spin_factors::fill_spin_factors(2, m, lmax, &mut spin_alpha, &mut spin_a, &mut spin_b)
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
                let norm = spin_alpha[l] * spin_factors::spin_normalization::<f64>(2, l, false);
                for lane in 0..plus.len() {
                    let plus_value = spin::scaled_basis_value(plus[lane], plus_exp[lane], norm);
                    let minus_value = spin::scaled_basis_value(minus[lane], minus_exp[lane], norm);
                    let even = plus_value + minus_value;
                    let odd = plus_value - minus_value;
                    let ring = ring_start + lane;
                    spin_first_sum[ring] += e_coefficients[l - m] * even
                        + Complex::new(0.0, -1.0) * b_coefficients[l - m] * odd;
                    spin_second_sum[ring] += Complex::new(0.0, 1.0) * e_coefficients[l - m] * odd
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
