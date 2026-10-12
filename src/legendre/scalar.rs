// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * This file contains an adaptation of the normalized scalar recurrence in
 * DUCC's sht_inner_loop.h.
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

use num_complex::Complex64;
use pulp::{Arch, Simd, WithSimd};

use crate::error::ShtError;

const BIG: f64 = f64::from_bits(1423_u64 << 52);
const SMALL: f64 = f64::from_bits(623_u64 << 52);
const SCALE_UP: f64 = f64::from_bits(1823_u64 << 52);
const SCALE_DOWN: f64 = f64::from_bits(223_u64 << 52);
pub(crate) const LATITUDE_BLOCK_SIZE: usize = 64;

#[derive(Clone, Copy)]
struct ScaledValue {
    mantissa: f64,
    exponent: i64,
}

/// Returns the scalar sectoral normalizations used by the DUCC recurrence.
pub(crate) fn sectoral_normalizations(mmax: usize) -> Result<Vec<f64>, ShtError> {
    let count = mmax.checked_add(1).ok_or(ShtError::IntegerOverflow {
        quantity: "scalar sectoral normalization count",
    })?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(count)
        .map_err(|_| ShtError::AllocationFailed {
            array: "scalar sectoral normalization",
        })?;
    result.push(1.0 / (4.0 * PI).sqrt());
    for m in 1..=mmax {
        let mf = m as f64;
        let previous = result[m - 1];
        result.push(previous * ((2.0 * mf + 1.0) / (2.0 * mf)).sqrt());
    }
    Ok(result)
}

/// Returns the polar order cutoff used by DUCC for a ring.
pub(crate) fn polar_order_limit(lmax: usize, spin: usize, sin_theta: f64, cos_theta: f64) -> usize {
    let lmax_f = lmax as f64;
    let offset = (0.01 * lmax_f).max(100.0);
    let b = -2.0 * spin as f64 * cos_theta.abs();
    let t = lmax_f * sin_theta + offset;
    let c = (spin as f64).powi(2) - t * t;
    let discriminant = b * b - 4.0 * c;
    if discriminant <= 0.0 {
        return lmax;
    }
    (((-b + discriminant.sqrt()) * 0.5).min(lmax_f) + 0.5) as usize
}

/// Fills the order-dependent scalar recurrence factors through `lmax`.
pub(crate) fn fill_epsilon(m: usize, lmax: usize, output: &mut [f64]) {
    debug_assert!(m <= lmax);
    debug_assert_eq!(output.len(), lmax + 1);
    for (l, value) in output.iter_mut().enumerate().skip(m) {
        *value = epsilon(l, m);
    }
}

/// Visits `Y_lm(theta)` for one order using shared binary scaling.
#[cfg(test)]
fn for_each_y_lm(
    m: usize,
    lmax: usize,
    sin_theta: f64,
    cos_theta: f64,
    sectoral_norm: f64,
    epsilon: &[f64],
    mut visit: impl FnMut(usize, f64, i64),
) -> Result<(), ShtError> {
    debug_assert!(m <= lmax);
    debug_assert!(epsilon.len() > lmax);

    let mut current = scaled_power(sin_theta, m)?;
    current = normalize_value(
        current.mantissa * sectoral_norm * if m.is_multiple_of(2) { 1.0 } else { -1.0 },
        current.exponent,
    )?;
    let mut previous = ScaledValue {
        mantissa: 0.0,
        exponent: current.exponent,
    };

    for l in m..=lmax {
        visit(l, current.mantissa, current.exponent);
        if l == lmax {
            break;
        }

        let next = (cos_theta * current.mantissa - epsilon[l] * previous.mantissa) / epsilon[l + 1];
        previous = current;
        current = ScaledValue {
            mantissa: next,
            exponent: previous.exponent,
        };
        normalize_pair(&mut previous, &mut current)?;
    }

    Ok(())
}

/// Visits one order over latitude lanes using the runtime-selected pulp backend.
#[expect(
    clippy::too_many_arguments,
    reason = "The kernel receives geometry, recurrence factors and caller-owned scratch explicitly."
)]
pub(crate) fn for_each_y_lm_dispatched(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    epsilon: &[f64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
    mut visit: impl FnMut(usize, usize, &[f64], &[i64]) -> Result<(), ShtError>,
) -> Result<(), ShtError> {
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    let scratch_len = sin_theta.len().min(LATITUDE_BLOCK_SIZE);
    debug_assert!(previous.len() >= scratch_len);
    debug_assert!(current.len() >= scratch_len);
    debug_assert!(next.len() >= scratch_len);
    debug_assert!(exponents.len() >= scratch_len);

    for ring_start in (0..sin_theta.len()).step_by(LATITUDE_BLOCK_SIZE) {
        let ring_end = (ring_start + LATITUDE_BLOCK_SIZE).min(sin_theta.len());
        let block_len = ring_end - ring_start;
        arch.dispatch(ForEachYlm {
            ring_start,
            m,
            lmax,
            sin_theta: &sin_theta[ring_start..ring_end],
            cos_theta: &cos_theta[ring_start..ring_end],
            sectoral_norm,
            epsilon,
            previous: &mut previous[..block_len],
            current: &mut current[..block_len],
            next: &mut next[..block_len],
            exponents: &mut exponents[..block_len],
            visit: &mut visit,
        })?;
    }
    Ok(())
}

struct ForEachYlm<'a, F> {
    ring_start: usize,
    m: usize,
    lmax: usize,
    sin_theta: &'a [f64],
    cos_theta: &'a [f64],
    sectoral_norm: f64,
    epsilon: &'a [f64],
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
    exponents: &'a mut [i64],
    visit: F,
}

impl<F> WithSimd for ForEachYlm<'_, F>
where
    F: FnMut(usize, usize, &[f64], &[i64]) -> Result<(), ShtError>,
{
    type Output = Result<(), ShtError>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let ForEachYlm {
            ring_start,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            epsilon,
            previous,
            current,
            next,
            exponents,
            mut visit,
        } = self;
        debug_assert!(m <= lmax);
        debug_assert_eq!(sin_theta.len(), cos_theta.len());
        debug_assert_eq!(sin_theta.len(), previous.len());
        debug_assert_eq!(sin_theta.len(), current.len());
        debug_assert_eq!(sin_theta.len(), next.len());
        debug_assert_eq!(sin_theta.len(), exponents.len());
        debug_assert!(epsilon.len() > lmax);

        let mut previous = previous;
        let mut current = current;
        let mut next = next;
        for ring in 0..sin_theta.len() {
            let sectoral = scaled_power(sin_theta[ring], m)?;
            let sectoral = normalize_value(
                sectoral.mantissa * sectoral_norm * if m.is_multiple_of(2) { 1.0 } else { -1.0 },
                sectoral.exponent,
            )?;
            previous[ring] = 0.0;
            current[ring] = sectoral.mantissa;
            exponents[ring] = sectoral.exponent;
        }

        for l in m..=lmax {
            visit(l, ring_start, current, exponents)?;
            if l == lmax {
                break;
            }

            let epsilon_l = simd.splat_f64s(epsilon[l]);
            let epsilon_next = simd.splat_f64s(epsilon[l + 1]);
            let (current_vectors, current_tail) = S::as_simd_f64s(current);
            let (previous_vectors, previous_tail) = S::as_simd_f64s(previous);
            let (cos_vectors, cos_tail) = S::as_simd_f64s(cos_theta);
            let (next_vectors, next_tail) = S::as_mut_simd_f64s(next);
            debug_assert_eq!(current_vectors.len(), previous_vectors.len());
            debug_assert_eq!(current_vectors.len(), cos_vectors.len());
            debug_assert_eq!(current_vectors.len(), next_vectors.len());
            for block in 0..current_vectors.len() {
                let degree_term = simd.mul_f64s(epsilon_l, previous_vectors[block]);
                let latitude_term = simd.mul_f64s(cos_vectors[block], current_vectors[block]);
                next_vectors[block] =
                    simd.div_f64s(simd.sub_f64s(latitude_term, degree_term), epsilon_next);
            }
            debug_assert_eq!(current_tail.len(), previous_tail.len());
            debug_assert_eq!(current_tail.len(), cos_tail.len());
            debug_assert_eq!(current_tail.len(), next_tail.len());
            for index in 0..current_tail.len() {
                next_tail[index] = (cos_tail[index] * current_tail[index]
                    - epsilon[l] * previous_tail[index])
                    / epsilon[l + 1];
            }

            normalize_pair_lanes(current, next, exponents)?;
            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
        }
        Ok(())
    }
}

fn normalize_pair_lanes(
    previous: &mut [f64],
    current: &mut [f64],
    exponents: &mut [i64],
) -> Result<(), ShtError> {
    debug_assert_eq!(previous.len(), current.len());
    debug_assert_eq!(previous.len(), exponents.len());
    for ring in 0..previous.len() {
        let mut previous_value = ScaledValue {
            mantissa: previous[ring],
            exponent: exponents[ring],
        };
        let mut current_value = ScaledValue {
            mantissa: current[ring],
            exponent: exponents[ring],
        };
        normalize_pair(&mut previous_value, &mut current_value)?;
        previous[ring] = previous_value.mantissa;
        current[ring] = current_value.mantissa;
        exponents[ring] = current_value.exponent;
    }
    Ok(())
}

/// Multiplies a complex value by a scaled recurrence value without overflow.
pub(crate) fn multiply_complex_scaled(value: Complex64, factor: f64, exponent: i64) -> Complex64 {
    Complex64::new(
        multiply_component_scaled(value.re, factor, exponent),
        multiply_component_scaled(value.im, factor, exponent),
    )
}

fn multiply_component_scaled(value: f64, factor: f64, exponent: i64) -> f64 {
    if value == 0.0 || factor == 0.0 || !value.is_finite() || !factor.is_finite() {
        return scale_binary(value * factor, exponent);
    }
    if value.is_normal() && factor.is_normal() {
        let product = value * factor;
        if product.is_normal() {
            let scaled = scale_binary(product, exponent);
            if scaled.is_normal() {
                return scaled;
            }
        }
    }
    let (value_mantissa, value_exponent) = split_binary(value);
    let (factor_mantissa, factor_exponent) = split_binary(factor);
    scale_binary(
        value_mantissa * factor_mantissa,
        exponent
            .saturating_add(value_exponent)
            .saturating_add(factor_exponent),
    )
}

fn split_binary(value: f64) -> (f64, i64) {
    let bits = value.to_bits();
    let sign = bits & (1_u64 << 63);
    let raw_exponent = ((bits >> 52) & 0x7ff) as i64;
    let fraction = bits & ((1_u64 << 52) - 1);
    if raw_exponent == 0 {
        let scaled = value * 2.0_f64.powi(54);
        let (mantissa, exponent) = split_normal(scaled);
        (mantissa, exponent - 54)
    } else {
        let mantissa = f64::from_bits(sign | (1022_u64 << 52) | fraction);
        (mantissa, raw_exponent - 1022)
    }
}

fn split_normal(value: f64) -> (f64, i64) {
    let bits = value.to_bits();
    let sign = bits & (1_u64 << 63);
    let raw_exponent = ((bits >> 52) & 0x7ff) as i64;
    let fraction = bits & ((1_u64 << 52) - 1);
    (
        f64::from_bits(sign | (1022_u64 << 52) | fraction),
        raw_exponent - 1022,
    )
}

fn epsilon(l: usize, m: usize) -> f64 {
    if l == m {
        return 0.0;
    }
    let lf = l as f64;
    let mf = m as f64;
    (((lf - mf) * (lf + mf)) / ((2.0 * lf + 1.0) * (2.0 * lf - 1.0))).sqrt()
}

fn scaled_power(base: f64, mut exponent: usize) -> Result<ScaledValue, ShtError> {
    let mut result = ScaledValue {
        mantissa: 1.0,
        exponent: 0,
    };
    let mut factor = normalize_value(base, 0)?;

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
            quantity: "scalar recurrence scale",
        })?;
    normalize_value(left.mantissa * right.mantissa, exponent)
}

fn normalize_value(mut value: f64, mut exponent: i64) -> Result<ScaledValue, ShtError> {
    while value.abs() > BIG {
        value *= SCALE_DOWN;
        exponent = exponent.checked_add(800).ok_or(ShtError::IntegerOverflow {
            quantity: "scalar recurrence scale",
        })?;
    }
    while value != 0.0 && value.abs() < SMALL {
        value *= SCALE_UP;
        exponent = exponent.checked_sub(800).ok_or(ShtError::IntegerOverflow {
            quantity: "scalar recurrence scale",
        })?;
    }
    Ok(ScaledValue {
        mantissa: value,
        exponent,
    })
}

fn normalize_pair(previous: &mut ScaledValue, current: &mut ScaledValue) -> Result<(), ShtError> {
    let mut maximum = previous.mantissa.abs().max(current.mantissa.abs());
    while maximum > BIG {
        previous.mantissa *= SCALE_DOWN;
        current.mantissa *= SCALE_DOWN;
        previous.exponent =
            previous
                .exponent
                .checked_add(800)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "scalar recurrence scale",
                })?;
        current.exponent = previous.exponent;
        maximum *= SCALE_DOWN;
    }
    if maximum != 0.0 {
        while maximum < SMALL {
            previous.mantissa *= SCALE_UP;
            current.mantissa *= SCALE_UP;
            previous.exponent =
                previous
                    .exponent
                    .checked_sub(800)
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "scalar recurrence scale",
                    })?;
            current.exponent = previous.exponent;
            maximum *= SCALE_UP;
        }
    }
    Ok(())
}

/// Applies a binary scale without forming an overflowing power-of-two factor.
pub(crate) fn scale_binary(mut value: f64, mut exponent: i64) -> f64 {
    if value == 0.0 || !value.is_finite() {
        return value;
    }
    if exponent == 0 {
        return value;
    }
    if exponent > 4096 {
        return value.signum() * f64::INFINITY;
    }
    if exponent < -4096 {
        return 0.0_f64.copysign(value);
    }

    let bits = value.to_bits();
    let raw_exponent = ((bits >> 52) & 0x7ff) as i64;
    let scaled_exponent = raw_exponent + exponent;
    // This exact path avoids powi for the normal values in each recurrence lane.
    if (1..0x7ff).contains(&raw_exponent) && (1..0x7ff).contains(&scaled_exponent) {
        let exponent_mask = 0x7ff_u64 << 52;
        return f64::from_bits((bits & !exponent_mask) | ((scaled_exponent as u64) << 52));
    }

    while exponent > 1023 {
        value *= 2.0_f64.powi(1023);
        exponent -= 1023;
        if !value.is_finite() {
            return value;
        }
    }
    while exponent < -1022 {
        value *= 2.0_f64.powi(-1022);
        exponent += 1022;
        if value == 0.0 {
            return value;
        }
    }
    value * 2.0_f64.powi(exponent as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_multiply_component_scaled(value: f64, factor: f64, exponent: i64) -> f64 {
        if value == 0.0 || factor == 0.0 || !value.is_finite() || !factor.is_finite() {
            return scale_binary(value * factor, exponent);
        }
        let (value_mantissa, value_exponent) = split_binary(value);
        let (factor_mantissa, factor_exponent) = split_binary(factor);
        scale_binary(
            value_mantissa * factor_mantissa,
            exponent
                .saturating_add(value_exponent)
                .saturating_add(factor_exponent),
        )
    }

    fn reference_scale_binary(mut value: f64, mut exponent: i64) -> f64 {
        if value == 0.0 || !value.is_finite() {
            return value;
        }
        if exponent > 4096 {
            return value.signum() * f64::INFINITY;
        }
        if exponent < -4096 {
            return 0.0_f64.copysign(value);
        }
        while exponent > 1023 {
            value *= 2.0_f64.powi(1023);
            exponent -= 1023;
            if !value.is_finite() {
                return value;
            }
        }
        while exponent < -1022 {
            value *= 2.0_f64.powi(-1022);
            exponent += 1022;
            if value == 0.0 {
                return value;
            }
        }
        value * 2.0_f64.powi(exponent as i32)
    }

    fn values(m: usize, lmax: usize, theta: f64) -> Vec<f64> {
        let norms = sectoral_normalizations(m).unwrap();
        let mut epsilon = vec![0.0; lmax + 1];
        fill_epsilon(m, lmax, &mut epsilon);
        let mut result = vec![0.0; lmax - m + 1];
        for_each_y_lm(
            m,
            lmax,
            theta.sin(),
            theta.cos(),
            norms[m],
            &epsilon,
            |l, mantissa, exponent| result[l - m] = scale_binary(mantissa, exponent),
        )
        .unwrap();
        result
    }

    fn values_over_rings(arch: Arch, m: usize, lmax: usize, theta: &[f64]) -> Vec<Vec<f64>> {
        let norms = sectoral_normalizations(m).unwrap();
        let sin_theta: Vec<_> = theta.iter().map(|theta| theta.sin()).collect();
        let cos_theta: Vec<_> = theta.iter().map(|theta| theta.cos()).collect();
        let mut epsilon = vec![0.0; lmax + 1];
        fill_epsilon(m, lmax, &mut epsilon);
        let scratch_len = theta.len().min(LATITUDE_BLOCK_SIZE);
        let mut previous = vec![0.0; scratch_len];
        let mut current = vec![0.0; scratch_len];
        let mut next = vec![0.0; scratch_len];
        let mut exponents = vec![0; scratch_len];
        let mut result = vec![vec![0.0; theta.len()]; lmax - m + 1];

        for_each_y_lm_dispatched(
            arch,
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            norms[m],
            &epsilon,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
            |l, ring_start, mantissa, exponents| {
                for lane in 0..mantissa.len() {
                    result[l - m][ring_start + lane] =
                        scale_binary(mantissa[lane], exponents[lane]);
                }
                Ok(())
            },
        )
        .unwrap();
        result
    }

    #[test]
    fn scalar_normalized_recurrence_matches_closed_forms() {
        let theta: f64 = 0.71;
        let (sin, cos) = theta.sin_cos();
        let y00 = 1.0 / (4.0 * PI).sqrt();
        assert!((values(0, 0, theta)[0] - y00).abs() < 2e-16);
        assert!((values(0, 1, theta)[1] - (3.0 / (4.0 * PI)).sqrt() * cos).abs() < 2e-16);
        assert!(
            (values(0, 2, theta)[2] - (5.0 / (16.0 * PI)).sqrt() * (3.0 * cos * cos - 1.0)).abs()
                < 3e-16
        );
        assert!((values(1, 2, theta)[1] + (15.0 / (8.0 * PI)).sqrt() * sin * cos).abs() < 3e-16);
        assert!((values(2, 2, theta)[0] - (15.0 / (32.0 * PI)).sqrt() * sin * sin).abs() < 3e-16);
    }

    #[test]
    fn dispatched_recurrence_matches_scalar_blocks_and_tails() {
        let mut theta = vec![
            0.013, 0.14, 0.33, 0.71, 1.1, 1.57, 2.03, 2.7, 3.0, 3.12, 1.6,
        ];
        theta.extend((0..120).map(|index| std::f64::consts::PI * (index as f64 + 0.5) / 120.0));
        for m in 0..=8 {
            let lmax = 32;
            let expected: Vec<Vec<f64>> =
                theta.iter().map(|&theta| values(m, lmax, theta)).collect();
            for arch in [Arch::Scalar, Arch::new()] {
                let actual = values_over_rings(arch, m, lmax, &theta);
                for (ring, expected_values) in expected.iter().enumerate() {
                    for l in m..=lmax {
                        let reference = expected_values[l - m];
                        let relative_error = if reference == 0.0 {
                            if actual[l - m][ring] == 0.0 {
                                0.0
                            } else {
                                f64::INFINITY
                            }
                        } else {
                            (actual[l - m][ring] - reference).abs() / reference.abs()
                        };
                        assert!(
                            relative_error <= 2e-13,
                            "m={m}, l={l}, ring={ring}, actual={:.16e}, reference={reference:.16e}, relative error={relative_error:.3e}",
                            actual[l - m][ring]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn lane_rescaling_matches_scalar_for_large_and_small_values() {
        let mut previous = [BIG * 2.0, SMALL / 2.0, 0.0, -BIG * 3.0];
        let mut current = [-BIG * 1.5, SMALL / 4.0, SMALL / 8.0, SMALL / 16.0];
        let mut exponents = [1, -3, 42, 500];
        let mut expected = Vec::new();

        for lane in 0..previous.len() {
            let mut previous_value = ScaledValue {
                mantissa: previous[lane],
                exponent: exponents[lane],
            };
            let mut current_value = ScaledValue {
                mantissa: current[lane],
                exponent: exponents[lane],
            };
            normalize_pair(&mut previous_value, &mut current_value).unwrap();
            expected.push((
                previous_value.mantissa,
                current_value.mantissa,
                current_value.exponent,
            ));
        }

        normalize_pair_lanes(&mut previous, &mut current, &mut exponents).unwrap();
        for lane in 0..previous.len() {
            assert_eq!(previous[lane], expected[lane].0);
            assert_eq!(current[lane], expected[lane].1);
            assert_eq!(exponents[lane], expected[lane].2);
        }
    }

    #[test]
    fn high_order_near_polar_recurrence_matches_scalar_across_tail_lanes() {
        let theta = [
            1e-12,
            1e-6,
            1e-3,
            0.04,
            0.2,
            0.8,
            1.4,
            1.57,
            2.2,
            3.13,
            std::f64::consts::PI - 1e-6,
        ];
        let m = 255;
        let lmax = 768;
        let expected: Vec<Vec<f64>> = theta.iter().map(|&value| values(m, lmax, value)).collect();

        for arch in [Arch::Scalar, Arch::new()] {
            let actual = values_over_rings(arch, m, lmax, &theta);
            for (ring, expected_values) in expected.iter().enumerate() {
                for l in m..=lmax {
                    let reference = expected_values[l - m];
                    let observed = actual[l - m][ring];
                    let relative_error = if reference == 0.0 {
                        if observed == 0.0 { 0.0 } else { f64::INFINITY }
                    } else {
                        (observed - reference).abs() / reference.abs()
                    };
                    assert!(
                        relative_error <= 2e-13,
                        "m={m}, l={l}, ring={ring}, observed={observed:.16e}, reference={reference:.16e}, relative error={relative_error:.3e}"
                    );
                }
            }
        }
    }

    #[test]
    fn scaled_sectoral_values_remain_finite_at_high_order() {
        let norm = sectoral_normalizations(255).unwrap()[255];
        let mut epsilon = vec![0.0; 256];
        fill_epsilon(255, 255, &mut epsilon);
        for theta in [1e-3, 0.37, std::f64::consts::FRAC_PI_2, 2.9] {
            let mut last = 0.0;
            for_each_y_lm(
                255,
                255,
                theta.sin(),
                theta.cos(),
                norm,
                &epsilon,
                |_, mantissa, exponent| last = scale_binary(mantissa, exponent),
            )
            .unwrap();
            assert!(last.is_finite());
        }
    }

    #[test]
    fn scaled_complex_multiply_avoids_intermediate_overflow() {
        let value = Complex64::new(1.0e300, -1.0e300);
        let actual = multiply_complex_scaled(value, BIG, -800);
        let expected_scale = 2.0_f64.powi(-400);
        assert!((actual.re / expected_scale - 1.0e300).abs() / 1.0e300 < 2e-15);
        assert!((actual.im / expected_scale + 1.0e300).abs() / 1.0e300 < 2e-15);
    }

    #[test]
    fn binary_scaling_matches_power_of_two_products() {
        let values = [
            0.0,
            -0.0,
            0.125,
            -0.75,
            1.0,
            -1.5,
            f64::MIN_POSITIVE,
            f64::from_bits(1),
        ];
        let exponents = [
            -4097, -2048, -1023, -1022, -801, -800, -401, -1, 0, 1, 401, 800, 1022, 1023, 2048,
            4097,
        ];

        for value in values {
            for exponent in exponents {
                let actual = scale_binary(value, exponent);
                let expected = reference_scale_binary(value, exponent);
                assert_eq!(
                    actual.to_bits(),
                    expected.to_bits(),
                    "value={value:e}, exponent={exponent}, actual={actual:e}, expected={expected:e}"
                );
            }
        }
    }

    #[test]
    fn scaled_multiply_matches_split_reference() {
        let values = [
            0.0,
            -0.0,
            0.125,
            -0.75,
            1.0,
            -1.5,
            f64::MIN_POSITIVE,
            -f64::MAX,
            f64::from_bits(1),
            f64::from_bits(0x6e46_ed13_0a30_155d),
        ];
        let factors = [
            0.5,
            -0.75,
            1.125,
            f64::MAX,
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            0.0,
            f64::INFINITY,
            f64::from_bits(0x21d3_bb30_e0c3_d752),
        ];
        let exponents = [
            -4097, -2048, -1285, -1023, -1022, -1, 0, 1, 1022, 1023, 2048, 4097,
        ];

        for value in values {
            for factor in factors {
                for exponent in exponents {
                    let actual = multiply_component_scaled(value, factor, exponent);
                    let expected = reference_multiply_component_scaled(value, factor, exponent);
                    assert_eq!(
                        actual.to_bits(),
                        expected.to_bits(),
                        "value={value:e}, factor={factor:e}, exponent={exponent}, actual={actual:e}, expected={expected:e}"
                    );
                }
            }
        }
    }

    #[test]
    fn polar_order_cutoff_matches_ducc_bounds() {
        assert_eq!(polar_order_limit(64, 0, 0.0, 1.0), 64);
        assert_eq!(polar_order_limit(255, 0, 0.0, 1.0), 100);
        assert_eq!(polar_order_limit(255, 0, 1.0, 0.0), 255);
    }

    #[test]
    fn cached_epsilon_values_match_recurrence_definition() {
        let mut values = vec![f64::NAN; 8];
        fill_epsilon(3, 7, &mut values);
        assert!(values[..3].iter().all(|value| value.is_nan()));
        for (l, &value) in values.iter().enumerate().skip(3) {
            assert_eq!(value, epsilon(l, 3));
        }
    }
}
