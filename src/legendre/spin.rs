// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * This file adapts the normalized spin recurrence in DUCC's
 * sht_inner_loop.h to caller-owned latitude lanes and binary-scaled f64
 * values. The recurrence coefficients and half-angle initialization follow
 * the cited DUCC implementation; lane dispatch and scratch ownership use
 * RustSHT's workspace model.
 *
 * This code is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This code is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * Copyright (C) 2020-2025 Max-Planck-Society
 * Author: Martin Reinecke
 * Source revision: c1ed01b8fda69d2a378190f0915f1a19762dddbf.
 */

#[cfg(test)]
use std::f64::consts::PI;

use num_complex::Complex64;
use pulp::{Arch, Simd, WithSimd};

use crate::error::ShtError;
use crate::legendre::{LegendrePrecision, define_f64_scaled_arithmetic};

const BIG: f64 = <f64 as LegendrePrecision>::BIG;
const SMALL: f64 = <f64 as LegendrePrecision>::SMALL;
const SCALE_UP: f64 = <f64 as LegendrePrecision>::SCALE_UP;
const SCALE_DOWN: f64 = <f64 as LegendrePrecision>::SCALE_DOWN;
const SCALE_EXPONENT: i64 = <f64 as LegendrePrecision>::SCALE_EXPONENT;

pub(crate) type SpinInitial = super::spin_factors::SpinInitial<f64>;

/// Fills DUCC's order-dependent spin recurrence coefficients.
pub(crate) fn fill_spin_factors(
    spin: usize,
    m: usize,
    lmax: usize,
    alpha: &mut [f64],
    coefficient_a: &mut [f64],
    coefficient_b: &mut [f64],
) -> Result<SpinInitial, ShtError> {
    super::spin_factors::fill_spin_factors(spin, m, lmax, alpha, coefficient_a, coefficient_b)
}

/// Visits the two normalized spin recurrence sequences over latitude lanes.
#[expect(
    clippy::too_many_arguments,
    reason = "The recurrence receives its plan factors and caller-owned lane scratch explicitly."
)]
pub(crate) fn for_each_spin_lm_dispatched(
    arch: Arch,
    initial: &SpinInitial,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    previous_plus: &mut [f64],
    current_plus: &mut [f64],
    next_plus: &mut [f64],
    exponent_plus: &mut [i64],
    previous_minus: &mut [f64],
    current_minus: &mut [f64],
    next_minus: &mut [f64],
    exponent_minus: &mut [i64],
    mut visit: impl FnMut(usize, usize, &[f64], &[i64], &[f64], &[i64]) -> Result<(), ShtError>,
) -> Result<(), ShtError> {
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    let scratch_len = sin_theta.len().min(crate::legendre::LATITUDE_BLOCK_SIZE);
    for values in [
        &*previous_plus,
        &*current_plus,
        &*next_plus,
        &*previous_minus,
        &*current_minus,
        &*next_minus,
    ] {
        debug_assert!(values.len() >= scratch_len);
    }
    debug_assert!(exponent_plus.len() >= scratch_len);
    debug_assert!(exponent_minus.len() >= scratch_len);

    for ring_start in (0..sin_theta.len()).step_by(crate::legendre::LATITUDE_BLOCK_SIZE) {
        let ring_end = (ring_start + crate::legendre::LATITUDE_BLOCK_SIZE).min(sin_theta.len());
        let block_len = ring_end - ring_start;
        arch.dispatch(ForEachSpin {
            ring_start,
            initial,
            lmax,
            sin_theta: &sin_theta[ring_start..ring_end],
            cos_theta: &cos_theta[ring_start..ring_end],
            coefficient_a,
            coefficient_b,
            previous_plus: &mut previous_plus[..block_len],
            current_plus: &mut current_plus[..block_len],
            next_plus: &mut next_plus[..block_len],
            exponent_plus: &mut exponent_plus[..block_len],
            previous_minus: &mut previous_minus[..block_len],
            current_minus: &mut current_minus[..block_len],
            next_minus: &mut next_minus[..block_len],
            exponent_minus: &mut exponent_minus[..block_len],
            visit: &mut visit,
        })?;
    }
    Ok(())
}

struct ForEachSpin<'a, F> {
    ring_start: usize,
    initial: &'a SpinInitial,
    lmax: usize,
    sin_theta: &'a [f64],
    cos_theta: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    previous_plus: &'a mut [f64],
    current_plus: &'a mut [f64],
    next_plus: &'a mut [f64],
    exponent_plus: &'a mut [i64],
    previous_minus: &'a mut [f64],
    current_minus: &'a mut [f64],
    next_minus: &'a mut [f64],
    exponent_minus: &'a mut [i64],
    visit: F,
}

impl<F> WithSimd for ForEachSpin<'_, F>
where
    F: FnMut(usize, usize, &[f64], &[i64], &[f64], &[i64]) -> Result<(), ShtError>,
{
    type Output = Result<(), ShtError>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let ForEachSpin {
            ring_start,
            initial,
            lmax,
            sin_theta,
            cos_theta,
            coefficient_a,
            coefficient_b,
            previous_plus,
            current_plus,
            next_plus,
            exponent_plus,
            previous_minus,
            current_minus,
            next_minus,
            exponent_minus,
            mut visit,
        } = self;

        for ring in 0..sin_theta.len() {
            let half_cosine = ((1.0 + cos_theta[ring]) * 0.5).sqrt().max(1e-15);
            let half_sine = ((1.0 - cos_theta[ring]) * 0.5).sqrt().max(1e-15);
            let plus = multiply(
                initial.prefactor,
                multiply(
                    power(half_cosine, initial.cos_power)?,
                    power(half_sine, initial.sin_power)?,
                )?,
            )?;
            let minus = multiply(
                initial.prefactor,
                multiply(
                    power(half_cosine, initial.sin_power)?,
                    power(half_sine, initial.cos_power)?,
                )?,
            )?;
            previous_plus[ring] = 0.0;
            current_plus[ring] = initial.plus_sign * plus.mantissa;
            exponent_plus[ring] = plus.exponent;
            previous_minus[ring] = 0.0;
            current_minus[ring] = initial.minus_sign * minus.mantissa;
            exponent_minus[ring] = minus.exponent;
        }

        let mut previous_plus = previous_plus;
        let mut current_plus = current_plus;
        let mut next_plus = next_plus;
        let mut previous_minus = previous_minus;
        let mut current_minus = current_minus;
        let mut next_minus = next_minus;

        for l in initial.start_degree..=lmax {
            visit(
                l,
                ring_start,
                current_plus,
                exponent_plus,
                current_minus,
                exponent_minus,
            )?;
            if l == lmax {
                break;
            }

            recurrence_step(
                simd,
                cos_theta,
                coefficient_a[l + 1],
                coefficient_b[l + 1],
                current_plus,
                previous_plus,
                next_plus,
                false,
            );
            recurrence_step(
                simd,
                cos_theta,
                coefficient_a[l + 1],
                coefficient_b[l + 1],
                current_minus,
                previous_minus,
                next_minus,
                true,
            );
            normalize_pair_lanes(current_plus, next_plus, exponent_plus)?;
            normalize_pair_lanes(current_minus, next_minus, exponent_minus)?;
            std::mem::swap(&mut previous_plus, &mut current_plus);
            std::mem::swap(&mut current_plus, &mut next_plus);
            std::mem::swap(&mut previous_minus, &mut current_minus);
            std::mem::swap(&mut current_minus, &mut next_minus);
        }
        Ok(())
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The SIMD recurrence step receives vector lanes and independent positive/negative spin coefficients."
)]
fn recurrence_step<S: Simd>(
    simd: S,
    cosine: &[f64],
    coefficient_a: f64,
    coefficient_b: f64,
    current: &[f64],
    previous: &[f64],
    next: &mut [f64],
    add_coupling: bool,
) {
    let coefficient_b = if add_coupling {
        -coefficient_b
    } else {
        coefficient_b
    };
    let (current_vectors, current_tail) = S::as_simd_f64s(current);
    let (previous_vectors, previous_tail) = S::as_simd_f64s(previous);
    let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cosine);
    let (next_vectors, next_tail) = S::as_mut_simd_f64s(next);
    let coupling = simd.splat_f64s(-coefficient_b);
    for index in 0..current_vectors.len() {
        let leading = simd.mul_add_f64s(
            simd.splat_f64s(coefficient_a),
            cosine_vectors[index],
            coupling,
        );
        next_vectors[index] = simd.mul_add_f64s(
            leading,
            current_vectors[index],
            simd.neg_f64s(previous_vectors[index]),
        );
    }
    for index in 0..current_tail.len() {
        let leading = coefficient_a.mul_add(cosine_tail[index], -coefficient_b);
        next_tail[index] = leading.mul_add(current_tail[index], -previous_tail[index]);
    }
}

fn normalize_pair_lanes(
    previous: &mut [f64],
    current: &mut [f64],
    exponents: &mut [i64],
) -> Result<(), ShtError> {
    for ring in 0..previous.len() {
        let maximum = previous[ring].abs().max(current[ring].abs());
        let mut value = maximum;
        while value > BIG {
            previous[ring] *= SCALE_DOWN;
            current[ring] *= SCALE_DOWN;
            exponents[ring] =
                exponents[ring]
                    .checked_add(SCALE_EXPONENT)
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "spin recurrence exponent",
                    })?;
            value *= SCALE_DOWN;
        }
        while value != 0.0 && value < SMALL {
            previous[ring] *= SCALE_UP;
            current[ring] *= SCALE_UP;
            exponents[ring] =
                exponents[ring]
                    .checked_sub(SCALE_EXPONENT)
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "spin recurrence exponent",
                    })?;
            value *= SCALE_UP;
        }
    }
    Ok(())
}

/// Accumulates one spin-analysis degree over latitude lanes.
///
/// Ordinary lanes use Pulp vector arithmetic and a horizontal reduction.
/// Scaled, inactive, or exceptional lanes use the scalar recurrence path.
#[expect(
    clippy::too_many_arguments,
    reason = "The accumulation receives both Fourier components, spin bases and polar cutoffs."
)]
pub(crate) fn accumulate_analysis_dispatched(
    arch: Arch,
    vectorizable_inputs: bool,
    m: usize,
    q_real: &[f64],
    q_imag: &[f64],
    u_real: &[f64],
    u_imag: &[f64],
    plus: &[f64],
    plus_exponents: &[i64],
    minus: &[f64],
    minus_exponents: &[i64],
    alpha_norm: f64,
    ring_order_limits: &[usize],
    include_b: bool,
) -> (Complex64, Complex64) {
    debug_assert_eq!(q_real.len(), plus.len());
    debug_assert_eq!(q_imag.len(), plus.len());
    debug_assert_eq!(u_real.len(), plus.len());
    debug_assert_eq!(u_imag.len(), plus.len());
    debug_assert_eq!(plus_exponents.len(), plus.len());
    debug_assert_eq!(minus.len(), plus.len());
    debug_assert_eq!(minus_exponents.len(), plus.len());
    debug_assert_eq!(ring_order_limits.len(), plus.len());

    let vectorizable = vectorizable_inputs
        && alpha_norm.is_normal()
        && plus_exponents.iter().all(|&exponent| exponent == 0)
        && minus_exponents.iter().all(|&exponent| exponent == 0)
        && plus.iter().all(|value| value.is_normal())
        && minus.iter().all(|value| value.is_normal());
    if vectorizable {
        return arch.dispatch(SpinAnalysisAccumulation {
            q_real,
            q_imag,
            u_real,
            u_imag,
            plus,
            minus,
            alpha_norm,
            include_b,
        });
    }

    super::accumulate_spin_analysis_scalar_fallback::<f64>(
        m,
        q_real,
        q_imag,
        u_real,
        u_imag,
        plus,
        plus_exponents,
        minus,
        minus_exponents,
        alpha_norm,
        ring_order_limits,
        include_b,
    )
}

pub(crate) fn analysis_inputs_vectorizable(
    m: usize,
    q_real: &[f64],
    q_imag: &[f64],
    u_real: &[f64],
    u_imag: &[f64],
    ring_order_limits: &[usize],
) -> bool {
    debug_assert_eq!(q_real.len(), q_imag.len());
    debug_assert_eq!(q_real.len(), u_real.len());
    debug_assert_eq!(q_real.len(), u_imag.len());
    debug_assert_eq!(q_real.len(), ring_order_limits.len());
    ring_order_limits.iter().all(|&limit| m <= limit)
        && q_real.iter().all(|value| value.is_normal())
        && q_imag.iter().all(|value| value.is_normal())
        && u_real.iter().all(|value| value.is_normal())
        && u_imag.iter().all(|value| value.is_normal())
}

struct SpinAnalysisAccumulation<'a> {
    q_real: &'a [f64],
    q_imag: &'a [f64],
    u_real: &'a [f64],
    u_imag: &'a [f64],
    plus: &'a [f64],
    minus: &'a [f64],
    alpha_norm: f64,
    include_b: bool,
}

impl WithSimd for SpinAnalysisAccumulation<'_> {
    type Output = (Complex64, Complex64);

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self {
            q_real,
            q_imag,
            u_real,
            u_imag,
            plus,
            minus,
            alpha_norm,
            include_b,
        } = self;
        let (q_real_vectors, q_real_tail) = S::as_simd_f64s(q_real);
        let (q_imag_vectors, q_imag_tail) = S::as_simd_f64s(q_imag);
        let (u_real_vectors, u_real_tail) = S::as_simd_f64s(u_real);
        let (u_imag_vectors, u_imag_tail) = S::as_simd_f64s(u_imag);
        let (plus_vectors, plus_tail) = S::as_simd_f64s(plus);
        let (minus_vectors, minus_tail) = S::as_simd_f64s(minus);
        let vector_count = plus_vectors.len();
        debug_assert_eq!(q_real_vectors.len(), vector_count);
        debug_assert_eq!(q_imag_vectors.len(), vector_count);
        debug_assert_eq!(u_real_vectors.len(), vector_count);
        debug_assert_eq!(u_imag_vectors.len(), vector_count);
        debug_assert_eq!(minus_vectors.len(), vector_count);

        let alpha = simd.splat_f64s(alpha_norm);
        let mut e_real_sum = simd.splat_f64s(0.0);
        let mut e_imag_sum = simd.splat_f64s(0.0);
        let mut b_real_sum = simd.splat_f64s(0.0);
        let mut b_imag_sum = simd.splat_f64s(0.0);
        for index in 0..vector_count {
            let plus_value = simd.mul_f64s(plus_vectors[index], alpha);
            let minus_value = simd.mul_f64s(minus_vectors[index], alpha);
            let even_basis = simd.add_f64s(plus_value, minus_value);
            let odd_basis = simd.sub_f64s(plus_value, minus_value);
            e_real_sum = simd.add_f64s(
                e_real_sum,
                simd.add_f64s(
                    simd.mul_f64s(q_real_vectors[index], even_basis),
                    simd.mul_f64s(u_imag_vectors[index], odd_basis),
                ),
            );
            e_imag_sum = simd.add_f64s(
                e_imag_sum,
                simd.sub_f64s(
                    simd.mul_f64s(q_imag_vectors[index], even_basis),
                    simd.mul_f64s(u_real_vectors[index], odd_basis),
                ),
            );
            if include_b {
                b_real_sum = simd.add_f64s(
                    b_real_sum,
                    simd.sub_f64s(
                        simd.mul_f64s(u_real_vectors[index], even_basis),
                        simd.mul_f64s(q_imag_vectors[index], odd_basis),
                    ),
                );
                b_imag_sum = simd.add_f64s(
                    b_imag_sum,
                    simd.add_f64s(
                        simd.mul_f64s(u_imag_vectors[index], even_basis),
                        simd.mul_f64s(q_real_vectors[index], odd_basis),
                    ),
                );
            }
        }

        let mut e_real_tail_sum = 0.0;
        let mut e_imag_tail_sum = 0.0;
        let mut b_real_tail_sum = 0.0;
        let mut b_imag_tail_sum = 0.0;
        for lane in 0..plus_tail.len() {
            let plus_value = plus_tail[lane] * alpha_norm;
            let minus_value = minus_tail[lane] * alpha_norm;
            let even_basis = plus_value + minus_value;
            let odd_basis = plus_value - minus_value;
            e_real_tail_sum += q_real_tail[lane] * even_basis + u_imag_tail[lane] * odd_basis;
            e_imag_tail_sum += q_imag_tail[lane] * even_basis - u_real_tail[lane] * odd_basis;
            if include_b {
                b_real_tail_sum += u_real_tail[lane] * even_basis - q_imag_tail[lane] * odd_basis;
                b_imag_tail_sum += u_imag_tail[lane] * even_basis + q_real_tail[lane] * odd_basis;
            }
        }

        (
            Complex64::new(
                simd.reduce_sum_f64s(e_real_sum) + e_real_tail_sum,
                simd.reduce_sum_f64s(e_imag_sum) + e_imag_tail_sum,
            ),
            Complex64::new(
                simd.reduce_sum_f64s(b_real_sum) + b_real_tail_sum,
                simd.reduce_sum_f64s(b_imag_sum) + b_imag_tail_sum,
            ),
        )
    }
}

// Keep the f64 recurrence leaf specialized; B32's f64 timing evidence does
// not justify routing this protected transform through the generic helpers.
define_f64_scaled_arithmetic!(power, multiply, normalize, "spin recurrence exponent");

/// Converts one scaled recurrence value to a real spin basis value.
pub(crate) fn scaled_basis_value(value: f64, exponent: i64, alpha_norm: f64) -> f64 {
    if value == 0.0 || alpha_norm == 0.0 {
        return 0.0;
    }
    let product = value * alpha_norm;
    if exponent == 0 {
        return product;
    }
    crate::legendre::scale_binary(product, exponent)
}

/// Returns the normalization applied to one spin recurrence degree.
#[inline]
pub(crate) fn spin_normalization(spin: usize, degree: usize, derivative: bool) -> f64 {
    super::spin_factors::spin_normalization(spin, degree, derivative)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AnalysisLanes {
        q_real: Vec<f64>,
        q_imag: Vec<f64>,
        u_real: Vec<f64>,
        u_imag: Vec<f64>,
        plus: Vec<f64>,
        plus_exponents: Vec<i64>,
        minus: Vec<f64>,
        minus_exponents: Vec<i64>,
        limits: Vec<usize>,
    }

    impl AnalysisLanes {
        fn new(count: usize) -> Self {
            Self {
                q_real: (0..count).map(|lane| 0.25 + lane as f64 * 0.007).collect(),
                q_imag: (0..count).map(|lane| -0.4 + lane as f64 * 0.003).collect(),
                u_real: (0..count).map(|lane| 0.15 - lane as f64 * 0.002).collect(),
                u_imag: (0..count).map(|lane| 0.55 + lane as f64 * 0.004).collect(),
                plus: (0..count).map(|lane| 0.31 + lane as f64 * 0.001).collect(),
                plus_exponents: vec![0; count],
                minus: (0..count).map(|lane| -0.17 + lane as f64 * 0.002).collect(),
                minus_exponents: vec![0; count],
                limits: vec![64; count],
            }
        }

        fn accumulate(
            &self,
            arch: Arch,
            m: usize,
            alpha_norm: f64,
            include_b: bool,
        ) -> (Complex64, Complex64) {
            let vectorizable_inputs = analysis_inputs_vectorizable(
                m,
                &self.q_real,
                &self.q_imag,
                &self.u_real,
                &self.u_imag,
                &self.limits,
            );
            accumulate_analysis_dispatched(
                arch,
                vectorizable_inputs,
                m,
                &self.q_real,
                &self.q_imag,
                &self.u_real,
                &self.u_imag,
                &self.plus,
                &self.plus_exponents,
                &self.minus,
                &self.minus_exponents,
                alpha_norm,
                &self.limits,
                include_b,
            )
        }

        fn scalar_reference(
            &self,
            m: usize,
            alpha_norm: f64,
            include_b: bool,
        ) -> (Complex64, Complex64) {
            let mut e_sum = Complex64::new(0.0, 0.0);
            let mut b_sum = Complex64::new(0.0, 0.0);
            for lane in 0..self.plus.len() {
                if m > self.limits[lane] {
                    continue;
                }
                let plus_value =
                    scaled_basis_value(self.plus[lane], self.plus_exponents[lane], alpha_norm);
                let minus_value =
                    scaled_basis_value(self.minus[lane], self.minus_exponents[lane], alpha_norm);
                let even_basis = plus_value + minus_value;
                let odd_basis = plus_value - minus_value;
                let q = Complex64::new(self.q_real[lane], self.q_imag[lane]);
                let u = Complex64::new(self.u_real[lane], self.u_imag[lane]);
                e_sum += q * even_basis + Complex64::new(0.0, -1.0) * u * odd_basis;
                if include_b {
                    b_sum += Complex64::new(0.0, 1.0) * q * odd_basis + u * even_basis;
                }
            }
            (e_sum, b_sum)
        }
    }

    #[test]
    fn spin_analysis_simd_accumulation_matches_scalar_blocks_and_tails() {
        for count in [64, 61] {
            let lanes = AnalysisLanes::new(count);
            let expected = lanes.scalar_reference(4, 0.75, true);
            for arch in [Arch::Scalar, Arch::new()] {
                let actual = lanes.accumulate(arch, 4, 0.75, true);
                assert!((actual.0 - expected.0).norm() < 2e-14);
                assert!((actual.1 - expected.1).norm() < 2e-14);
            }
        }
    }

    #[test]
    fn spin_analysis_scaled_and_inactive_lanes_use_scalar_accumulation() {
        let mut lanes = AnalysisLanes::new(64);
        lanes.plus_exponents[11] = -800;
        lanes.limits[7] = 3;
        let expected = lanes.scalar_reference(4, 0.75, true);
        for arch in [Arch::Scalar, Arch::new()] {
            let actual = lanes.accumulate(arch, 4, 0.75, true);
            assert_eq!(actual.0, expected.0);
            assert_eq!(actual.1, expected.1);
        }
    }

    #[test]
    fn scaled_spin_basis_matches_binary_scaling_reference() {
        let values = [
            0.0,
            -0.0,
            0.125,
            -0.75,
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            f64::INFINITY,
            f64::NAN,
        ];
        let normalizations = [
            0.0,
            -0.0,
            0.5,
            -1.25,
            f64::MIN_POSITIVE,
            f64::INFINITY,
            f64::NAN,
        ];
        let exponents = [-4097, -1022, -1, 0, 1, 1023, 4097];

        for value in values {
            for alpha_norm in normalizations {
                for exponent in exponents {
                    let expected = if value == 0.0 || alpha_norm == 0.0 {
                        0.0
                    } else {
                        crate::legendre::scale_binary(value * alpha_norm, exponent)
                    };
                    let actual = scaled_basis_value(value, exponent, alpha_norm);
                    assert_eq!(actual.to_bits(), expected.to_bits());
                }
            }
        }
    }

    fn evaluate(
        spin: usize,
        m: usize,
        lmax: usize,
        theta: &[f64],
    ) -> Vec<(usize, Vec<f64>, Vec<f64>)> {
        let count = lmax + 2;
        let mut alpha = vec![0.0; count];
        let mut coefficient_a = vec![0.0; count];
        let mut coefficient_b = vec![0.0; count];
        let initial = fill_spin_factors(
            spin,
            m,
            lmax,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        )
        .unwrap();
        let sin_theta = theta.iter().map(|value| value.sin()).collect::<Vec<_>>();
        let cos_theta = theta.iter().map(|value| value.cos()).collect::<Vec<_>>();
        let scratch_len = theta.len();
        let mut previous_plus = vec![0.0; scratch_len];
        let mut current_plus = vec![0.0; scratch_len];
        let mut next_plus = vec![0.0; scratch_len];
        let mut exponent_plus = vec![0; scratch_len];
        let mut previous_minus = vec![0.0; scratch_len];
        let mut current_minus = vec![0.0; scratch_len];
        let mut next_minus = vec![0.0; scratch_len];
        let mut exponent_minus = vec![0; scratch_len];
        let mut result = Vec::new();
        for_each_spin_lm_dispatched(
            Arch::new(),
            &initial,
            lmax,
            &sin_theta,
            &cos_theta,
            &coefficient_a,
            &coefficient_b,
            &mut previous_plus,
            &mut current_plus,
            &mut next_plus,
            &mut exponent_plus,
            &mut previous_minus,
            &mut current_minus,
            &mut next_minus,
            &mut exponent_minus,
            |degree, _, plus, plus_exp, minus, minus_exp| {
                let norm = spin_normalization(spin, degree, false) * alpha[degree];
                result.push((
                    degree,
                    plus.iter()
                        .zip(plus_exp)
                        .map(|(&value, &exponent)| scaled_basis_value(value, exponent, norm))
                        .collect(),
                    minus
                        .iter()
                        .zip(minus_exp)
                        .map(|(&value, &exponent)| scaled_basis_value(value, exponent, norm))
                        .collect(),
                ));
                Ok(())
            },
        )
        .unwrap();
        result
    }

    #[test]
    fn low_degree_spin_bases_match_closed_forms() {
        let theta = [0.31, 0.77, 1.43, 2.81];
        let spin1_m0 = evaluate(1, 0, 1, &theta);
        let expected_norm1 = (3.0 / (8.0 * PI)).sqrt();
        for (ring, &angle) in theta.iter().enumerate() {
            let expected = -expected_norm1 * angle.sin();
            assert!((spin1_m0[0].1[ring] - expected * 0.5).abs() < 2e-15);
            assert!((spin1_m0[0].2[ring] - expected * 0.5).abs() < 2e-15);
        }

        let spin1_m1 = evaluate(1, 1, 1, &theta);
        for (ring, &angle) in theta.iter().enumerate() {
            let norm = 0.5 * (3.0 / (4.0 * PI)).sqrt();
            let cosine = angle.cos();
            assert!((spin1_m1[0].1[ring] + norm * (1.0 + cosine) * 0.5).abs() < 2e-15);
            assert!((spin1_m1[0].2[ring] - norm * (1.0 - cosine) * 0.5).abs() < 2e-15);
        }

        let spin2_m0 = evaluate(2, 0, 2, &theta);
        let expected_norm2 = -(30.0 / (64.0 * PI)).sqrt();
        for (ring, &angle) in theta.iter().enumerate() {
            let expected = expected_norm2 * angle.sin().powi(2);
            assert!((spin2_m0[0].1[ring] - expected * 0.5).abs() < 2e-15);
            assert!((spin2_m0[0].2[ring] - expected * 0.5).abs() < 2e-15);
        }

        for spin in [3, 4] {
            let basis = evaluate(spin, 0, spin, &theta);
            let central_binomial = (1..=spin)
                .map(|index| (spin + index) as f64 / index as f64)
                .product::<f64>();
            for (ring, &angle) in theta.iter().enumerate() {
                let half_angle_product = (angle / 2.0).sin() * (angle / 2.0).cos();
                let expected = -0.5
                    * (((2 * spin + 1) as f64 * central_binomial / (4.0 * PI)).sqrt())
                    * half_angle_product.powi(spin as i32);
                assert!((basis[0].1[ring] - expected).abs() < 3e-15);
                assert!((basis[0].2[ring] - expected).abs() < 3e-15);
            }
        }
    }
}
