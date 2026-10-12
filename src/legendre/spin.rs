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

use std::f64::consts::PI;

use pulp::{Arch, Simd, WithSimd};

use crate::error::ShtError;

const BIG: f64 = f64::from_bits(1423_u64 << 52);
const SMALL: f64 = f64::from_bits(623_u64 << 52);
const SCALE_UP: f64 = f64::from_bits(1823_u64 << 52);
const SCALE_DOWN: f64 = f64::from_bits(223_u64 << 52);

#[derive(Clone, Copy)]
struct ScaledValue {
    mantissa: f64,
    exponent: i64,
}

pub(crate) struct SpinInitial {
    pub(crate) start_degree: usize,
    cos_power: usize,
    sin_power: usize,
    plus_sign: f64,
    minus_sign: f64,
    prefactor: ScaledValue,
}

/// Fills DUCC's order-dependent spin recurrence coefficients.
pub(crate) fn fill_spin_factors(
    spin: usize,
    m: usize,
    lmax: usize,
    alpha: &mut [f64],
    coefficient_a: &mut [f64],
    coefficient_b: &mut [f64],
) -> Result<SpinInitial, ShtError> {
    debug_assert!(spin > 0 && spin <= lmax && m <= lmax);
    debug_assert!(alpha.len() > lmax + 1);
    debug_assert!(coefficient_a.len() > lmax + 1);
    debug_assert!(coefficient_b.len() > lmax + 1);

    let mlo = spin.min(m);
    let mhi = spin.max(m);
    alpha.fill(0.0);
    coefficient_a.fill(0.0);
    coefficient_b.fill(0.0);
    alpha[mhi] = 1.0;

    for l in mhi..=lmax {
        let lf = l as f64;
        let mf = m as f64;
        let sf = spin as f64;
        let inv_product = ((lf + mf + 1.0).recip()
            * (lf - mf + 1.0).recip()
            * (lf + sf + 1.0).recip()
            * (lf - sf + 1.0).recip())
        .sqrt();
        let leading = (lf + 1.0) * (2.0 * lf + 1.0) * inv_product;
        let coupling = if l == 0 {
            0.0
        } else {
            mf * sf / (lf * (lf + 1.0))
        };

        if l > mhi {
            let ratio = (((lf + mf) * (lf - mf) * (lf + sf) * (lf - sf))
                / ((lf + mf + 1.0) * (lf - mf + 1.0) * (lf + sf + 1.0) * (lf - sf + 1.0)))
                .sqrt()
                * ((lf + 1.0) / lf);
            alpha[l + 1] = alpha[l - 1] * ratio;
        } else {
            alpha[l + 1] = 1.0;
        }

        let a = leading * alpha[l] / alpha[l + 1];
        coefficient_a[l + 1] = a;
        coefficient_b[l + 1] = coupling * a;
    }

    let mut prefactor = ScaledValue {
        mantissa: 1.0,
        exponent: 0,
    };
    let binomial_lower = mhi - mlo;
    for i in 1..=binomial_lower {
        let numerator = mhi
            .checked_add(mlo)
            .and_then(|value| value.checked_add(i))
            .ok_or(ShtError::IntegerOverflow {
                quantity: "spin recurrence binomial factor",
            })?;
        prefactor = normalize(
            prefactor.mantissa * ((numerator as f64) / (i as f64)).sqrt(),
            prefactor.exponent,
        )?;
    }

    let (cos_power, sin_power, plus_sign, minus_sign) = if mhi == m {
        (
            mhi.checked_add(spin).ok_or(ShtError::IntegerOverflow {
                quantity: "spin recurrence half-angle power",
            })?,
            mhi - spin,
            parity_sign(m),
            parity_sign(mhi - spin),
        )
    } else {
        (
            mhi.checked_add(m).ok_or(ShtError::IntegerOverflow {
                quantity: "spin recurrence half-angle power",
            })?,
            mhi - m,
            parity_sign(spin),
            parity_sign(mhi.checked_add(m).ok_or(ShtError::IntegerOverflow {
                quantity: "spin recurrence half-angle power",
            })?),
        )
    };

    Ok(SpinInitial {
        start_degree: mhi,
        cos_power,
        sin_power,
        plus_sign,
        minus_sign,
        prefactor,
    })
}

const fn parity_sign(value: usize) -> f64 {
    if value.is_multiple_of(2) { 1.0 } else { -1.0 }
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
    for index in 0..current_vectors.len() {
        let leading = simd.sub_f64s(
            simd.mul_f64s(simd.splat_f64s(coefficient_a), cosine_vectors[index]),
            simd.splat_f64s(coefficient_b),
        );
        next_vectors[index] = simd.sub_f64s(
            simd.mul_f64s(leading, current_vectors[index]),
            previous_vectors[index],
        );
    }
    for index in 0..current_tail.len() {
        let leading = coefficient_a * cosine_tail[index] - coefficient_b;
        next_tail[index] = leading * current_tail[index] - previous_tail[index];
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
                    .checked_add(800)
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
                    .checked_sub(800)
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "spin recurrence exponent",
                    })?;
            value *= SCALE_UP;
        }
    }
    Ok(())
}

fn power(base: f64, mut exponent: usize) -> Result<ScaledValue, ShtError> {
    let mut result = ScaledValue {
        mantissa: 1.0,
        exponent: 0,
    };
    let mut factor = normalize(base, 0)?;
    while exponent != 0 {
        if !exponent.is_multiple_of(2) {
            result = multiply(result, factor)?;
        }
        exponent /= 2;
        if exponent != 0 {
            factor = multiply(factor, factor)?;
        }
    }
    Ok(result)
}

fn multiply(left: ScaledValue, right: ScaledValue) -> Result<ScaledValue, ShtError> {
    let exponent = left
        .exponent
        .checked_add(right.exponent)
        .ok_or(ShtError::IntegerOverflow {
            quantity: "spin recurrence exponent",
        })?;
    normalize(left.mantissa * right.mantissa, exponent)
}

fn normalize(mut value: f64, mut exponent: i64) -> Result<ScaledValue, ShtError> {
    while value.abs() > BIG {
        value *= SCALE_DOWN;
        exponent = exponent.checked_add(800).ok_or(ShtError::IntegerOverflow {
            quantity: "spin recurrence exponent",
        })?;
    }
    while value != 0.0 && value.abs() < SMALL {
        value *= SCALE_UP;
        exponent = exponent.checked_sub(800).ok_or(ShtError::IntegerOverflow {
            quantity: "spin recurrence exponent",
        })?;
    }
    Ok(ScaledValue {
        mantissa: value,
        exponent,
    })
}

/// Converts one scaled recurrence value to a real spin basis value.
pub(crate) fn scaled_basis_value(value: f64, exponent: i64, alpha_norm: f64) -> f64 {
    if value == 0.0 || alpha_norm == 0.0 {
        return 0.0;
    }
    let product = value * alpha_norm;
    crate::legendre::scale_binary(product, exponent)
}

/// Returns the normalization applied to one spin recurrence degree.
pub(crate) fn spin_normalization(spin: usize, degree: usize, derivative: bool) -> f64 {
    if derivative {
        if degree == 0 {
            return 0.0;
        }
        let degree = degree as f64;
        0.5 * (degree * (degree + 1.0) * (2.0 * degree + 1.0) / (4.0 * PI)).sqrt()
    } else {
        let sign = if spin.is_multiple_of(2) { -1.0 } else { 1.0 };
        sign * 0.5 * ((2 * degree + 1) as f64 / (4.0 * PI)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
