// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * The f32 spin recurrence follows DUCC's normalized spin recurrence in
 * src/ducc0/sht/sht_inner_loop.h, revision
 * c1ed01b8fda69d2a378190f0915f1a19762dddbf.
 * Its binary scaling keeps recurrence values representable in f32 while
 * preserving the positive and negative spin sequences.
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

use std::f32::consts::PI;

use pulp::{Arch, Simd, WithSimd};

use crate::error::ShtError;
use crate::legendre::{LATITUDE_BLOCK_SIZE, scalar_f32::scale_binary};

const BIG: f32 = f32::from_bits(167_u32 << 23);
const SMALL: f32 = f32::from_bits(87_u32 << 23);
const SCALE_UP: f32 = f32::from_bits(207_u32 << 23);
const SCALE_DOWN: f32 = f32::from_bits(47_u32 << 23);

#[derive(Clone, Copy)]
struct ScaledValue {
    mantissa: f32,
    exponent: i64,
}

pub(crate) struct SpinInitial32 {
    pub(crate) start_degree: usize,
    cos_power: usize,
    sin_power: usize,
    plus_sign: f32,
    minus_sign: f32,
    prefactor: ScaledValue,
}

pub(crate) fn fill_spin_factors(
    spin: usize,
    m: usize,
    lmax: usize,
    alpha: &mut [f32],
    coefficient_a: &mut [f32],
    coefficient_b: &mut [f32],
) -> Result<SpinInitial32, ShtError> {
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
        let lf = l as f32;
        let mf = m as f32;
        let sf = spin as f32;
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
            prefactor.mantissa * ((numerator as f32) / (i as f32)).sqrt(),
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
        let combined = mhi.checked_add(m).ok_or(ShtError::IntegerOverflow {
            quantity: "spin recurrence half-angle power",
        })?;
        (combined, mhi - m, parity_sign(spin), parity_sign(combined))
    };

    Ok(SpinInitial32 {
        start_degree: mhi,
        cos_power,
        sin_power,
        plus_sign,
        minus_sign,
        prefactor,
    })
}

const fn parity_sign(value: usize) -> f32 {
    if value.is_multiple_of(2) { 1.0 } else { -1.0 }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The recurrence receives its plan factors and caller-owned lane scratch explicitly."
)]
pub(crate) fn for_each_spin_lm_dispatched<F>(
    arch: Arch,
    initial: &SpinInitial32,
    lmax: usize,
    sin_theta: &[f32],
    cos_theta: &[f32],
    coefficient_a: &[f32],
    coefficient_b: &[f32],
    previous_plus: &mut [f32],
    current_plus: &mut [f32],
    next_plus: &mut [f32],
    exponent_plus: &mut [i64],
    previous_minus: &mut [f32],
    current_minus: &mut [f32],
    next_minus: &mut [f32],
    exponent_minus: &mut [i64],
    mut visit: F,
) -> Result<(), ShtError>
where
    F: FnMut(usize, usize, &[f32], &[i64], &[f32], &[i64]) -> Result<(), ShtError>,
{
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    let scratch_len = sin_theta.len().min(LATITUDE_BLOCK_SIZE);
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

    for ring_start in (0..sin_theta.len()).step_by(LATITUDE_BLOCK_SIZE) {
        let ring_end = (ring_start + LATITUDE_BLOCK_SIZE).min(sin_theta.len());
        let block_len = ring_end - ring_start;
        arch.dispatch(ForEachSpin32 {
            ring_start,
            initial,
            lmax,
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

struct ForEachSpin32<'a, F> {
    ring_start: usize,
    initial: &'a SpinInitial32,
    lmax: usize,
    cos_theta: &'a [f32],
    coefficient_a: &'a [f32],
    coefficient_b: &'a [f32],
    previous_plus: &'a mut [f32],
    current_plus: &'a mut [f32],
    next_plus: &'a mut [f32],
    exponent_plus: &'a mut [i64],
    previous_minus: &'a mut [f32],
    current_minus: &'a mut [f32],
    next_minus: &'a mut [f32],
    exponent_minus: &'a mut [i64],
    visit: F,
}

impl<F> WithSimd for ForEachSpin32<'_, F>
where
    F: FnMut(usize, usize, &[f32], &[i64], &[f32], &[i64]) -> Result<(), ShtError>,
{
    type Output = Result<(), ShtError>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let ForEachSpin32 {
            ring_start,
            initial,
            lmax,
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
        debug_assert_eq!(cos_theta.len(), previous_plus.len());
        debug_assert_eq!(cos_theta.len(), current_plus.len());
        debug_assert_eq!(cos_theta.len(), next_plus.len());
        debug_assert_eq!(cos_theta.len(), exponent_plus.len());
        debug_assert_eq!(cos_theta.len(), previous_minus.len());
        debug_assert_eq!(cos_theta.len(), current_minus.len());
        debug_assert_eq!(cos_theta.len(), next_minus.len());
        debug_assert_eq!(cos_theta.len(), exponent_minus.len());

        for lane in 0..cos_theta.len() {
            let half_cosine = ((1.0 + cos_theta[lane]) * 0.5).sqrt().max(1e-15);
            let half_sine = ((1.0 - cos_theta[lane]) * 0.5).sqrt().max(1e-15);
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
            previous_plus[lane] = 0.0;
            current_plus[lane] = initial.plus_sign * plus.mantissa;
            exponent_plus[lane] = plus.exponent;
            previous_minus[lane] = 0.0;
            current_minus[lane] = initial.minus_sign * minus.mantissa;
            exponent_minus[lane] = minus.exponent;
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
    reason = "The recurrence step receives lane vectors and independent spin coefficients explicitly."
)]
fn recurrence_step<S: Simd>(
    simd: S,
    cosine: &[f32],
    coefficient_a: f32,
    coefficient_b: f32,
    current: &[f32],
    previous: &[f32],
    next: &mut [f32],
    add_coupling: bool,
) {
    let coefficient_b = if add_coupling {
        -coefficient_b
    } else {
        coefficient_b
    };
    let (current_vectors, current_tail) = S::as_simd_f32s(current);
    let (previous_vectors, previous_tail) = S::as_simd_f32s(previous);
    let (cosine_vectors, cosine_tail) = S::as_simd_f32s(cosine);
    let (next_vectors, next_tail) = S::as_mut_simd_f32s(next);
    let coupling = simd.splat_f32s(-coefficient_b);
    for index in 0..current_vectors.len() {
        let leading = simd.mul_add_f32s(
            simd.splat_f32s(coefficient_a),
            cosine_vectors[index],
            coupling,
        );
        next_vectors[index] = simd.mul_add_f32s(
            leading,
            current_vectors[index],
            simd.neg_f32s(previous_vectors[index]),
        );
    }
    for index in 0..current_tail.len() {
        let leading = coefficient_a.mul_add(cosine_tail[index], -coefficient_b);
        next_tail[index] = leading.mul_add(current_tail[index], -previous_tail[index]);
    }
}

fn normalize_pair_lanes(
    previous: &mut [f32],
    current: &mut [f32],
    exponents: &mut [i64],
) -> Result<(), ShtError> {
    for lane in 0..previous.len() {
        let mut maximum = previous[lane].abs().max(current[lane].abs());
        while maximum > BIG {
            previous[lane] *= SCALE_DOWN;
            current[lane] *= SCALE_DOWN;
            exponents[lane] = exponents[lane]
                .checked_add(80)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "spin recurrence exponent",
                })?;
            maximum *= SCALE_DOWN;
        }
        while maximum != 0.0 && maximum < SMALL {
            previous[lane] *= SCALE_UP;
            current[lane] *= SCALE_UP;
            exponents[lane] = exponents[lane]
                .checked_sub(80)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "spin recurrence exponent",
                })?;
            maximum *= SCALE_UP;
        }
    }
    Ok(())
}

fn power(base: f32, mut exponent: usize) -> Result<ScaledValue, ShtError> {
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

fn normalize(mut value: f32, mut exponent: i64) -> Result<ScaledValue, ShtError> {
    while value.abs() > BIG {
        value *= SCALE_DOWN;
        exponent = exponent.checked_add(80).ok_or(ShtError::IntegerOverflow {
            quantity: "spin recurrence exponent",
        })?;
    }
    while value != 0.0 && value.abs() < SMALL {
        value *= SCALE_UP;
        exponent = exponent.checked_sub(80).ok_or(ShtError::IntegerOverflow {
            quantity: "spin recurrence exponent",
        })?;
    }
    Ok(ScaledValue {
        mantissa: value,
        exponent,
    })
}

pub(crate) fn scaled_basis_value(value: f32, exponent: i64, alpha_norm: f32) -> f32 {
    if value == 0.0 || alpha_norm == 0.0 {
        return 0.0;
    }
    scale_binary(value * alpha_norm, exponent)
}

pub(crate) fn spin_normalization(spin: usize, degree: usize, derivative: bool) -> f32 {
    if derivative {
        if degree == 0 {
            return 0.0;
        }
        let degree = degree as f32;
        0.5 * (degree * (degree + 1.0) * (2.0 * degree + 1.0) / (4.0 * PI)).sqrt()
    } else {
        let sign = if spin.is_multiple_of(2) { -1.0 } else { 1.0 };
        sign * 0.5 * ((2 * degree + 1) as f32 / (4.0 * PI)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values_over_rings(
        arch: Arch,
        spin: usize,
        m: usize,
        lmax: usize,
        theta: &[f32],
    ) -> (Vec<Vec<f32>>, Vec<Vec<f32>>) {
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
        let scratch_len = theta.len().min(LATITUDE_BLOCK_SIZE);
        let mut previous_plus = vec![0.0; scratch_len];
        let mut current_plus = vec![0.0; scratch_len];
        let mut next_plus = vec![0.0; scratch_len];
        let mut exponent_plus = vec![0; scratch_len];
        let mut previous_minus = vec![0.0; scratch_len];
        let mut current_minus = vec![0.0; scratch_len];
        let mut next_minus = vec![0.0; scratch_len];
        let mut exponent_minus = vec![0; scratch_len];
        let degree_count = lmax - initial.start_degree + 1;
        let mut plus_values = vec![vec![0.0; theta.len()]; degree_count];
        let mut minus_values = vec![vec![0.0; theta.len()]; degree_count];
        for_each_spin_lm_dispatched(
            arch,
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
            |degree, ring_start, plus, plus_exponents, minus, minus_exponents| {
                let factor = spin_normalization(spin, degree, false) * alpha[degree];
                for lane in 0..plus.len() {
                    plus_values[degree - initial.start_degree][ring_start + lane] =
                        scaled_basis_value(plus[lane], plus_exponents[lane], factor);
                    minus_values[degree - initial.start_degree][ring_start + lane] =
                        scaled_basis_value(minus[lane], minus_exponents[lane], factor);
                }
                Ok(())
            },
        )
        .unwrap();
        (plus_values, minus_values)
    }

    #[test]
    fn runtime_simd_recurrence_matches_scalar_fallback_across_blocks_and_tails() {
        let mut theta = (0..129)
            .map(|ring| PI * (ring as f32 + 0.5) / 129.0)
            .collect::<Vec<_>>();
        theta[0] = 1.0e-6;
        theta[1] = 1.0e-3;
        theta[127] = PI - 1.0e-3;
        theta[128] = PI - 1.0e-6;

        for spin in 1..=4 {
            for m in [0, spin, 96] {
                let expected = values_over_rings(Arch::Scalar, spin, m, 96, &theta);
                let actual = values_over_rings(Arch::new(), spin, m, 96, &theta);
                for (sign, expected_values, actual_values) in [
                    ("plus", &expected.0, &actual.0),
                    ("minus", &expected.1, &actual.1),
                ] {
                    for (degree_offset, (expected_rings, actual_rings)) in
                        expected_values.iter().zip(actual_values).enumerate()
                    {
                        for (ring, (&expected, &actual)) in
                            expected_rings.iter().zip(actual_rings).enumerate()
                        {
                            assert!(
                                actual.is_finite(),
                                "spin={spin}, m={m}, sign={sign}, l={}, ring={ring}: non-finite result {actual}",
                                spin.max(m) + degree_offset
                            );
                            assert!(
                                (actual - expected).abs() <= 5.0e-6,
                                "spin={spin}, m={m}, sign={sign}, l={}, ring={ring}: actual {actual:.9e}, scalar {expected:.9e}",
                                spin.max(m) + degree_offset
                            );
                        }
                    }
                }
            }
        }
    }
}
