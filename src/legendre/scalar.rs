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
pub(crate) fn for_each_y_lm(
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

#[cfg(test)]
mod tests {
    use super::*;

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
