// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * The f32 scalar recurrence follows DUCC's normalized scalar recurrence in
 * src/ducc0/sht/sht_inner_loop.h, revision
 * c1ed01b8fda69d2a378190f0915f1a19762dddbf.
 * Its binary scaling keeps recurrence values representable in f32 while
 * preserving the degree and lane traversal used by the f64 implementation.
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

use num_complex::Complex32;

use crate::error::ShtError;
use crate::legendre::LATITUDE_BLOCK_SIZE;

const BIG: f32 = f32::from_bits(167_u32 << 23);
const SMALL: f32 = f32::from_bits(87_u32 << 23);
const SCALE_UP: f32 = f32::from_bits(207_u32 << 23);
const SCALE_DOWN: f32 = f32::from_bits(47_u32 << 23);

#[derive(Clone, Copy)]
struct ScaledValue {
    mantissa: f32,
    exponent: i64,
}

pub(crate) fn sectoral_normalizations(mmax: usize) -> Result<Vec<f32>, ShtError> {
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
        let mf = m as f32;
        result.push(result[m - 1] * ((2.0 * mf + 1.0) / (2.0 * mf)).sqrt());
    }
    Ok(result)
}

pub(crate) fn polar_order_limit(lmax: usize, spin: usize, sin_theta: f32, cos_theta: f32) -> usize {
    let lmax_f = lmax as f32;
    let offset = (0.01 * lmax_f).max(100.0);
    let b = -2.0 * spin as f32 * cos_theta.abs();
    let t = lmax_f * sin_theta + offset;
    let c = (spin as f32).powi(2) - t * t;
    let discriminant = b * b - 4.0 * c;
    if discriminant <= 0.0 {
        return lmax;
    }
    (((-b + discriminant.sqrt()) * 0.5).min(lmax_f) + 0.5) as usize
}

pub(crate) fn fill_epsilon(m: usize, lmax: usize, output: &mut [f32]) {
    debug_assert!(m <= lmax);
    debug_assert_eq!(output.len(), lmax + 1);
    for (l, value) in output.iter_mut().enumerate().skip(m) {
        *value = epsilon(l, m);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The recurrence receives grid lanes and caller-owned recurrence scratch explicitly."
)]
pub(crate) fn for_each_y_lm_dispatched<F>(
    m: usize,
    lmax: usize,
    sin_theta: &[f32],
    cos_theta: &[f32],
    sectoral_norm: f32,
    epsilon: &[f32],
    previous: &mut [f32],
    current: &mut [f32],
    next: &mut [f32],
    exponents: &mut [i64],
    mut visit: F,
) -> Result<(), ShtError>
where
    F: FnMut(usize, usize, &[f32], &[i64]) -> Result<(), ShtError>,
{
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    debug_assert!(epsilon.len() > lmax);
    for ring_start in (0..sin_theta.len()).step_by(LATITUDE_BLOCK_SIZE) {
        let ring_end = (ring_start + LATITUDE_BLOCK_SIZE).min(sin_theta.len());
        let block_len = ring_end - ring_start;
        let mut previous = &mut previous[..block_len];
        let mut current = &mut current[..block_len];
        let mut next = &mut next[..block_len];
        let exponents = &mut exponents[..block_len];
        for lane in 0..block_len {
            let sectoral = scaled_power(sin_theta[ring_start + lane], m)?;
            let value = normalize_value(
                sectoral.mantissa * sectoral_norm * if m.is_multiple_of(2) { 1.0 } else { -1.0 },
                sectoral.exponent,
            )?;
            previous[lane] = 0.0;
            current[lane] = value.mantissa;
            exponents[lane] = value.exponent;
        }
        for l in m..=lmax {
            visit(l, ring_start, current, exponents)?;
            if l == lmax {
                break;
            }
            for lane in 0..block_len {
                let ring = ring_start + lane;
                next[lane] = (cos_theta[ring] * current[lane] - epsilon[l] * previous[lane])
                    / epsilon[l + 1];
            }
            normalize_pair_lanes(current, next, exponents)?;
            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
        }
    }
    Ok(())
}

pub(crate) fn multiply_complex_scaled(value: Complex32, factor: f32, exponent: i64) -> Complex32 {
    if factor.is_normal() {
        Complex32::new(
            multiply_component_scaled_with_normal_factor(value.re, factor, exponent),
            multiply_component_scaled_with_normal_factor(value.im, factor, exponent),
        )
    } else {
        Complex32::new(
            multiply_component_scaled(value.re, factor, exponent),
            multiply_component_scaled(value.im, factor, exponent),
        )
    }
}

fn multiply_component_scaled_with_normal_factor(value: f32, factor: f32, exponent: i64) -> f32 {
    if value.is_normal() {
        let product = value * factor;
        if product.is_normal() {
            let scaled = scale_binary(product, exponent);
            if scaled.is_normal() {
                return scaled;
            }
        }
    }
    if value == 0.0 || !value.is_finite() {
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

fn multiply_component_scaled(value: f32, factor: f32, exponent: i64) -> f32 {
    if value.is_normal() && factor.is_normal() {
        let product = value * factor;
        if product.is_normal() {
            let scaled = scale_binary(product, exponent);
            if scaled.is_normal() {
                return scaled;
            }
        }
    }
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

fn split_binary(value: f32) -> (f32, i64) {
    let bits = value.to_bits();
    let sign = bits & (1_u32 << 31);
    let raw_exponent = ((bits >> 23) & 0xff) as i64;
    let fraction = bits & ((1_u32 << 23) - 1);
    if raw_exponent == 0 {
        let scaled = value * 2.0_f32.powi(24);
        let (mantissa, exponent) = split_normal(scaled);
        (mantissa, exponent - 24)
    } else {
        let mantissa = f32::from_bits(sign | (126_u32 << 23) | fraction);
        (mantissa, raw_exponent - 126)
    }
}

fn split_normal(value: f32) -> (f32, i64) {
    let bits = value.to_bits();
    let sign = bits & (1_u32 << 31);
    let raw_exponent = ((bits >> 23) & 0xff) as i64;
    let fraction = bits & ((1_u32 << 23) - 1);
    (
        f32::from_bits(sign | (126_u32 << 23) | fraction),
        raw_exponent - 126,
    )
}

fn epsilon(l: usize, m: usize) -> f32 {
    if l == m {
        return 0.0;
    }
    let lf = l as f32;
    let mf = m as f32;
    (((lf - mf) * (lf + mf)) / ((2.0 * lf + 1.0) * (2.0 * lf - 1.0))).sqrt()
}

fn scaled_power(base: f32, mut exponent: usize) -> Result<ScaledValue, ShtError> {
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

fn normalize_value(mut value: f32, mut exponent: i64) -> Result<ScaledValue, ShtError> {
    while value.abs() > BIG {
        value *= SCALE_DOWN;
        exponent = exponent.checked_add(80).ok_or(ShtError::IntegerOverflow {
            quantity: "scalar recurrence scale",
        })?;
    }
    while value != 0.0 && value.abs() < SMALL {
        value *= SCALE_UP;
        exponent = exponent.checked_sub(80).ok_or(ShtError::IntegerOverflow {
            quantity: "scalar recurrence scale",
        })?;
    }
    Ok(ScaledValue {
        mantissa: value,
        exponent,
    })
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
                    quantity: "scalar recurrence scale",
                })?;
            maximum *= SCALE_DOWN;
        }
        while maximum != 0.0 && maximum < SMALL {
            previous[lane] *= SCALE_UP;
            current[lane] *= SCALE_UP;
            exponents[lane] = exponents[lane]
                .checked_sub(80)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "scalar recurrence scale",
                })?;
            maximum *= SCALE_UP;
        }
    }
    Ok(())
}

pub(crate) fn scale_binary(mut value: f32, mut exponent: i64) -> f32 {
    if value == 0.0 || !value.is_finite() || exponent == 0 {
        return value;
    }
    if exponent > 4096 {
        return value.signum() * f32::INFINITY;
    }
    if exponent < -4096 {
        return 0.0_f32.copysign(value);
    }
    let bits = value.to_bits();
    let raw_exponent = ((bits >> 23) & 0xff) as i64;
    let scaled_exponent = raw_exponent + exponent;
    if (1..0xff).contains(&raw_exponent) && (1..0xff).contains(&scaled_exponent) {
        let exponent_mask = 0xff_u32 << 23;
        return f32::from_bits((bits & !exponent_mask) | ((scaled_exponent as u32) << 23));
    }
    while exponent > 127 {
        value *= 2.0_f32.powi(127);
        exponent -= 127;
        if !value.is_finite() {
            return value;
        }
    }
    while exponent < -126 {
        value *= 2.0_f32.powi(-126);
        exponent += 126;
        if value == 0.0 {
            return value;
        }
    }
    value * 2.0_f32.powi(exponent as i32)
}
