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

#[cfg(test)]
use std::f32::consts::PI;

use num_complex::Complex32;
use pulp::{Arch, Simd, WithSimd};

use crate::error::ShtError;
use crate::legendre::{LATITUDE_BLOCK_SIZE, LegendrePrecision, ScaledValue as BinaryScaledValue};

type ScaledValue = BinaryScaledValue<f32>;

const BIG: f32 = <f32 as LegendrePrecision>::BIG;
const SMALL: f32 = <f32 as LegendrePrecision>::SMALL;
const SCALE_UP: f32 = <f32 as LegendrePrecision>::SCALE_UP;
const SCALE_DOWN: f32 = <f32 as LegendrePrecision>::SCALE_DOWN;
const SCALE_EXPONENT: i64 = <f32 as LegendrePrecision>::SCALE_EXPONENT;
// Paired screens support the skip at T1023 and T1279. Keep other truncations
// on the original normalizer; an exact-match selector avoids T720 overhead.
const NORMAL_RANGE_SKIP_LMAX_T1023: usize = 1023;
const NORMAL_RANGE_SKIP_LMAX_T1279: usize = 1279;

pub(crate) fn sectoral_normalizations(mmax: usize) -> Result<Vec<f32>, ShtError> {
    super::numeric::sectoral_normalizations(mmax)
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
    super::numeric::fill_epsilon(m, lmax, output);
}

#[expect(
    clippy::too_many_arguments,
    reason = "The recurrence receives grid lanes and caller-owned recurrence scratch explicitly."
)]
pub(crate) fn for_each_y_lm_dispatched<F>(
    arch: Arch,
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
    visit: F,
) -> Result<(), ShtError>
where
    F: FnMut(usize, usize, &[f32], &[i64]) -> Result<(), ShtError>,
{
    if matches!(
        lmax,
        NORMAL_RANGE_SKIP_LMAX_T1023 | NORMAL_RANGE_SKIP_LMAX_T1279
    ) {
        for_each_y_lm_dispatched_impl::<F, true>(
            arch,
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
            visit,
        )
    } else {
        for_each_y_lm_dispatched_impl::<F, false>(
            arch,
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
            visit,
        )
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The recurrence receives grid lanes and caller-owned recurrence scratch explicitly."
)]
fn for_each_y_lm_dispatched_impl<F, const ALLOW_NORMAL_RANGE_SKIP: bool>(
    arch: Arch,
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
    let scratch_len = sin_theta.len().min(LATITUDE_BLOCK_SIZE);
    debug_assert!(previous.len() >= scratch_len);
    debug_assert!(current.len() >= scratch_len);
    debug_assert!(next.len() >= scratch_len);
    debug_assert!(exponents.len() >= scratch_len);

    for ring_start in (0..sin_theta.len()).step_by(LATITUDE_BLOCK_SIZE) {
        let ring_end = (ring_start + LATITUDE_BLOCK_SIZE).min(sin_theta.len());
        let block_len = ring_end - ring_start;
        arch.dispatch(ForEachYlm32::<_, ALLOW_NORMAL_RANGE_SKIP> {
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

struct ForEachYlm32<'a, F, const ALLOW_NORMAL_RANGE_SKIP: bool> {
    ring_start: usize,
    m: usize,
    lmax: usize,
    sin_theta: &'a [f32],
    cos_theta: &'a [f32],
    sectoral_norm: f32,
    epsilon: &'a [f32],
    previous: &'a mut [f32],
    current: &'a mut [f32],
    next: &'a mut [f32],
    exponents: &'a mut [i64],
    visit: F,
}

impl<F, const ALLOW_NORMAL_RANGE_SKIP: bool> WithSimd
    for ForEachYlm32<'_, F, ALLOW_NORMAL_RANGE_SKIP>
where
    F: FnMut(usize, usize, &[f32], &[i64]) -> Result<(), ShtError>,
{
    type Output = Result<(), ShtError>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let ForEachYlm32 {
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
        let mut normal_range = false;
        for lane in 0..sin_theta.len() {
            let sectoral = scaled_power(sin_theta[lane], m)?;
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

            let epsilon_l = simd.splat_f32s(epsilon[l]);
            let epsilon_next = simd.splat_f32s(epsilon[l + 1]);
            let (current_vectors, current_tail) = S::as_simd_f32s(current);
            let (previous_vectors, previous_tail) = S::as_simd_f32s(previous);
            let (cos_vectors, cos_tail) = S::as_simd_f32s(cos_theta);
            let (next_vectors, next_tail) = S::as_mut_simd_f32s(next);
            debug_assert_eq!(current_vectors.len(), previous_vectors.len());
            debug_assert_eq!(current_vectors.len(), cos_vectors.len());
            debug_assert_eq!(current_vectors.len(), next_vectors.len());
            for block in 0..current_vectors.len() {
                let degree_term = simd.mul_f32s(epsilon_l, previous_vectors[block]);
                let latitude_term = simd.mul_f32s(cos_vectors[block], current_vectors[block]);
                next_vectors[block] =
                    simd.div_f32s(simd.sub_f32s(latitude_term, degree_term), epsilon_next);
            }
            debug_assert_eq!(current_tail.len(), previous_tail.len());
            debug_assert_eq!(current_tail.len(), cos_tail.len());
            debug_assert_eq!(current_tail.len(), next_tail.len());
            for index in 0..current_tail.len() {
                next_tail[index] = (cos_tail[index] * current_tail[index]
                    - epsilon[l] * previous_tail[index])
                    / epsilon[l + 1];
            }

            if ALLOW_NORMAL_RANGE_SKIP {
                if normal_range {
                    if pair_lanes_need_scaling(simd, current, next) {
                        normal_range = normalize_pair_lanes_with_state(current, next, exponents)?;
                    }
                } else {
                    normal_range = normalize_pair_lanes_with_state(current, next, exponents)?;
                }
            } else {
                normalize_pair_lanes(current, next, exponents)?;
            }
            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
        }
        Ok(())
    }
}

pub(crate) fn multiply_complex_scaled(value: Complex32, factor: f32, exponent: i64) -> Complex32 {
    if factor.is_normal() {
        if value.re.is_normal() && value.im.is_normal() {
            let product = Complex32::new(value.re * factor, value.im * factor);
            if product.re.is_normal()
                && product.im.is_normal()
                && let Some(scaled) = scale_complex_normal_binary(product, exponent)
            {
                return scaled;
            }
        }
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

fn scale_complex_normal_binary(value: Complex32, exponent: i64) -> Option<Complex32> {
    if exponent == 0 {
        return Some(value);
    }
    if !(-512..=512).contains(&exponent) {
        return None;
    }

    let real_bits = value.re.to_bits();
    let imag_bits = value.im.to_bits();
    let real_exponent = ((real_bits >> 23) & 0xff) as i64 + exponent;
    let imag_exponent = ((imag_bits >> 23) & 0xff) as i64 + exponent;
    if !(1..0xff).contains(&real_exponent) || !(1..0xff).contains(&imag_exponent) {
        return None;
    }

    let exponent_mask = 0xff_u32 << 23;
    Some(Complex32::new(
        f32::from_bits((real_bits & !exponent_mask) | ((real_exponent as u32) << 23)),
        f32::from_bits((imag_bits & !exponent_mask) | ((imag_exponent as u32) << 23)),
    ))
}

fn multiply_component_scaled_with_normal_factor(value: f32, factor: f32, exponent: i64) -> f32 {
    if value.is_normal() {
        let product = value * factor;
        if product.is_normal()
            && let Some(scaled) = scale_normal_binary(product, exponent)
        {
            return scaled;
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
        if product.is_normal()
            && let Some(scaled) = scale_normal_binary(product, exponent)
        {
            return scaled;
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
        exponent = exponent
            .checked_add(SCALE_EXPONENT)
            .ok_or(ShtError::IntegerOverflow {
                quantity: "scalar recurrence scale",
            })?;
    }
    while value != 0.0 && value.abs() < SMALL {
        value *= SCALE_UP;
        exponent = exponent
            .checked_sub(SCALE_EXPONENT)
            .ok_or(ShtError::IntegerOverflow {
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
        let previous_bits = previous[lane].to_bits() & 0x7fff_ffff;
        let current_bits = current[lane].to_bits() & 0x7fff_ffff;
        debug_assert!(previous_bits < 0x7f80_0000 && current_bits < 0x7f80_0000);
        let mut maximum_bits = previous_bits.max(current_bits);
        let mut maximum = f32::from_bits(maximum_bits);
        while maximum_bits > BIG.to_bits() {
            previous[lane] *= SCALE_DOWN;
            current[lane] *= SCALE_DOWN;
            exponents[lane] =
                exponents[lane]
                    .checked_add(SCALE_EXPONENT)
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "scalar recurrence scale",
                    })?;
            maximum *= SCALE_DOWN;
            maximum_bits = maximum.to_bits() & 0x7fff_ffff;
        }
        while maximum_bits != 0 && maximum_bits < SMALL.to_bits() {
            previous[lane] *= SCALE_UP;
            current[lane] *= SCALE_UP;
            exponents[lane] =
                exponents[lane]
                    .checked_sub(SCALE_EXPONENT)
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "scalar recurrence scale",
                    })?;
            maximum *= SCALE_UP;
            maximum_bits = maximum.to_bits() & 0x7fff_ffff;
        }
    }
    Ok(())
}

#[inline(always)]
fn normalize_pair_lanes_with_state(
    previous: &mut [f32],
    current: &mut [f32],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    let mut normal_range = true;
    for lane in 0..previous.len() {
        let previous_bits = previous[lane].to_bits() & 0x7fff_ffff;
        let current_bits = current[lane].to_bits() & 0x7fff_ffff;
        debug_assert!(previous_bits < 0x7f80_0000 && current_bits < 0x7f80_0000);
        let mut maximum_bits = previous_bits.max(current_bits);
        let mut maximum = f32::from_bits(maximum_bits);
        while maximum_bits > BIG.to_bits() {
            previous[lane] *= SCALE_DOWN;
            current[lane] *= SCALE_DOWN;
            exponents[lane] =
                exponents[lane]
                    .checked_add(SCALE_EXPONENT)
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "scalar recurrence scale",
                    })?;
            maximum *= SCALE_DOWN;
            maximum_bits = maximum.to_bits() & 0x7fff_ffff;
        }
        while maximum_bits != 0 && maximum_bits < SMALL.to_bits() {
            previous[lane] *= SCALE_UP;
            current[lane] *= SCALE_UP;
            exponents[lane] =
                exponents[lane]
                    .checked_sub(SCALE_EXPONENT)
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "scalar recurrence scale",
                    })?;
            maximum *= SCALE_UP;
            maximum_bits = maximum.to_bits() & 0x7fff_ffff;
        }
        normal_range &= exponents[lane] == 0
            && maximum_bits <= BIG.to_bits()
            && (maximum_bits == 0 || maximum_bits >= SMALL.to_bits());
    }
    Ok(normal_range)
}

#[inline(always)]
fn pair_is_in_normal_range(previous: f32, current: f32) -> bool {
    let maximum = previous.abs().max(current.abs());
    maximum <= BIG && (maximum == 0.0 || maximum >= SMALL)
}

#[inline(always)]
fn pair_lanes_need_scaling<S: Simd>(simd: S, previous: &[f32], current: &[f32]) -> bool {
    debug_assert_eq!(previous.len(), current.len());
    let (previous_vectors, previous_tail) = S::as_simd_f32s(previous);
    let (current_vectors, current_tail) = S::as_simd_f32s(current);
    debug_assert_eq!(previous_vectors.len(), current_vectors.len());
    debug_assert_eq!(previous_tail.len(), current_tail.len());

    let big = simd.splat_f32s(BIG);
    let zeros = simd.splat_f32s(0.0);
    for index in 0..previous_vectors.len() {
        let previous = simd.abs_f32s(previous_vectors[index]);
        let current = simd.abs_f32s(current_vectors[index]);
        let previous_ordered = simd.equal_f32s(previous, previous);
        let current_ordered = simd.equal_f32s(current, current);
        let maximum = simd.max_f32s(previous, current);
        let maximum = simd.select_f32s(current_ordered, maximum, previous);
        let maximum = simd.select_f32s(previous_ordered, maximum, current);
        let below_upper_bound = simd.less_than_or_equal_f32s(maximum, big);
        let is_zero = simd.equal_f32s(maximum, zeros);
        let lower_check = simd.select_f32s(is_zero, big, maximum);
        let in_range = simd.select_f32s(below_upper_bound, lower_check, zeros);
        if simd.reduce_min_f32s(in_range) < SMALL {
            return true;
        }
    }

    previous_tail
        .iter()
        .zip(current_tail)
        .any(|(&previous, &current)| !pair_is_in_normal_range(previous, current))
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

fn scale_normal_binary(value: f32, exponent: i64) -> Option<f32> {
    debug_assert!(value.is_normal());
    if exponent == 0 {
        return Some(value);
    }
    if exponent > 4096 {
        return None;
    }
    if exponent < -4096 {
        return None;
    }

    let bits = value.to_bits();
    let raw_exponent = ((bits >> 23) & 0xff) as i64;
    let scaled_exponent = raw_exponent + exponent;
    if (1..0xff).contains(&scaled_exponent) {
        let exponent_mask = 0xff_u32 << 23;
        return Some(f32::from_bits(
            (bits & !exponent_mask) | ((scaled_exponent as u32) << 23),
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PairRangeCheck<'a> {
        previous: &'a [f32],
        current: &'a [f32],
    }

    impl WithSimd for PairRangeCheck<'_> {
        type Output = bool;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            pair_lanes_need_scaling(simd, self.previous, self.current)
        }
    }

    fn reference_multiply_component_scaled(value: f32, factor: f32, exponent: i64) -> f32 {
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

    fn reference_scale_binary(mut value: f32, mut exponent: i64) -> f32 {
        if value == 0.0 || !value.is_finite() {
            return value;
        }
        if exponent > 4096 {
            return value.signum() * f32::INFINITY;
        }
        if exponent < -4096 {
            return 0.0_f32.copysign(value);
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

    fn reference_normalize_pair_lanes(
        previous: &mut [f32],
        current: &mut [f32],
        exponents: &mut [i64],
    ) {
        for lane in 0..previous.len() {
            let mut maximum = previous[lane].abs().max(current[lane].abs());
            while maximum > BIG {
                previous[lane] *= SCALE_DOWN;
                current[lane] *= SCALE_DOWN;
                exponents[lane] += 80;
                maximum *= SCALE_DOWN;
            }
            while maximum != 0.0 && maximum < SMALL {
                previous[lane] *= SCALE_UP;
                current[lane] *= SCALE_UP;
                exponents[lane] -= 80;
                maximum *= SCALE_UP;
            }
        }
    }

    #[test]
    fn f32_pair_normalization_matches_float_magnitude_reference() {
        let initial_previous = [
            0.0,
            f32::from_bits(1),
            f32::from_bits(SMALL.to_bits() - 1),
            -f32::from_bits(BIG.to_bits() + 1),
            f32::MAX,
            f32::MIN_POSITIVE,
            -0.5,
            0.0,
        ];
        let initial_current = [
            -0.0,
            0.0,
            0.0,
            BIG,
            f32::MIN_POSITIVE,
            -f32::from_bits(1),
            0.25,
            SMALL,
        ];
        let mut actual_previous = initial_previous;
        let mut actual_current = initial_current;
        let mut actual_exponents = [0; 8];
        let mut expected_previous = initial_previous;
        let mut expected_current = initial_current;
        let mut expected_exponents = [0; 8];

        normalize_pair_lanes(
            &mut actual_previous,
            &mut actual_current,
            &mut actual_exponents,
        )
        .unwrap();
        reference_normalize_pair_lanes(
            &mut expected_previous,
            &mut expected_current,
            &mut expected_exponents,
        );

        for lane in 0..initial_previous.len() {
            assert_eq!(
                actual_previous[lane].to_bits(),
                expected_previous[lane].to_bits()
            );
            assert_eq!(
                actual_current[lane].to_bits(),
                expected_current[lane].to_bits()
            );
            assert_eq!(actual_exponents[lane], expected_exponents[lane]);
        }
    }

    #[test]
    fn f32_normal_range_check_covers_simd_lanes_and_tail() {
        for (previous_value, current_value) in [
            (0.0, 0.0),
            (0.0, SMALL / 2.0),
            (BIG, SMALL / 2.0),
            (BIG * 2.0, 0.0),
            (SMALL, BIG),
            (f32::NAN, BIG),
            (BIG, f32::NAN),
            (f32::NAN, SMALL / 2.0),
            (f32::NAN, f32::NAN),
            (f32::INFINITY, f32::NAN),
        ] {
            let previous = [previous_value; 17];
            let current = [current_value; 17];
            let expected = !pair_is_in_normal_range(previous_value, current_value);
            for arch in [Arch::Scalar, Arch::new()] {
                let actual = arch.dispatch(PairRangeCheck {
                    previous: &previous,
                    current: &current,
                });
                assert_eq!(actual, expected, "{previous_value:?}, {current_value:?}");
            }
        }

        let mut previous = vec![0.25_f32; 17];
        let mut current = vec![0.5_f32; 17];
        let mut exponents = vec![0_i64; 17];
        previous[0] = 0.0;
        current[0] = -0.0;
        previous[1] = SMALL;
        current[1] = 0.0;
        previous[2] = BIG;

        for arch in [Arch::Scalar, Arch::new()] {
            let needs_scaling = |previous: &[f32], current: &[f32]| {
                arch.dispatch(PairRangeCheck { previous, current })
            };
            assert!(!needs_scaling(&previous, &current));

            let mut normal_previous = previous.clone();
            let mut normal_current = current.clone();
            let mut normal_exponents = exponents.clone();
            assert!(
                normalize_pair_lanes_with_state(
                    &mut normal_previous,
                    &mut normal_current,
                    &mut normal_exponents,
                )
                .unwrap()
            );
            assert_eq!(normal_previous, previous);
            assert_eq!(normal_current, current);

            let mut below = current.clone();
            below[3] = f32::from_bits(SMALL.to_bits() - 1);
            let mut below_previous = previous.clone();
            below_previous[3] = 0.0;
            assert!(needs_scaling(&below_previous, &below));

            let mut above = current.clone();
            above[16] = f32::from_bits(BIG.to_bits() + 1);
            assert!(needs_scaling(&previous, &above));
        }

        exponents[8] = 1;
        assert!(
            !normalize_pair_lanes_with_state(&mut previous, &mut current, &mut exponents).unwrap()
        );
    }

    #[test]
    fn f32_binary_scaling_matches_staged_products() {
        let values = [
            0.0,
            -0.0,
            0.125,
            -0.75,
            1.0,
            -1.5,
            f32::MIN_POSITIVE,
            f32::from_bits(0x00ff_ffff),
            f32::from_bits(1),
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
        ];
        let exponents = [
            -4097, -512, -254, -127, -126, -81, -1, 0, 1, 81, 126, 127, 512, 4097,
        ];
        for value in values {
            for exponent in exponents {
                let actual = scale_binary(value, exponent);
                let expected = reference_scale_binary(value, exponent);
                assert_eq!(actual.to_bits(), expected.to_bits());
                if value.is_normal() {
                    let scaled = scale_normal_binary(value, exponent);
                    let raw_exponent = i64::from((value.to_bits() >> 23) & 0xff);
                    let scaled_exponent = raw_exponent + exponent;
                    let should_use_fast_path = (1..0xff).contains(&scaled_exponent);
                    assert_eq!(scaled.is_some(), should_use_fast_path);
                    if let Some(scaled) = scaled {
                        assert_eq!(scaled.to_bits(), expected.to_bits());
                    }
                }
            }
        }
    }

    #[test]
    fn f32_scaled_complex_multiply_matches_split_reference() {
        let values = [
            0.0,
            -0.0,
            0.125,
            -0.75,
            1.0,
            -1.5,
            f32::MIN_POSITIVE,
            -f32::MAX,
            f32::from_bits(0x017f_ffff),
            f32::from_bits(1),
            f32::from_bits(0x6e46_ed13),
        ];
        let factors = [
            0.5,
            -0.75,
            1.125,
            f32::MAX,
            f32::MIN_POSITIVE,
            f32::from_bits(1),
            0.0,
            f32::INFINITY,
            f32::from_bits(0x21d3_bb30),
        ];
        let exponents = [-4097, -512, -255, -127, -126, -1, 0, 1, 126, 127, 512, 4097];

        let boundary_value = f32::from_bits(0x017f_ffff);
        assert_eq!((boundary_value * 0.5).to_bits(), 0x00ff_ffff);
        let boundary_actual =
            multiply_complex_scaled(Complex32::new(boundary_value, -boundary_value), 0.5, -1);
        let boundary_expected = Complex32::new(
            reference_multiply_component_scaled(boundary_value, 0.5, -1),
            reference_multiply_component_scaled(-boundary_value, 0.5, -1),
        );
        assert_eq!(boundary_actual.re.to_bits(), boundary_expected.re.to_bits());
        assert_eq!(boundary_actual.im.to_bits(), boundary_expected.im.to_bits());

        for value in values {
            for factor in factors {
                for exponent in exponents {
                    let actual = multiply_component_scaled(value, factor, exponent);
                    let expected = reference_multiply_component_scaled(value, factor, exponent);
                    assert_eq!(
                        actual.to_bits(),
                        expected.to_bits(),
                        "value={value:e}, factor={factor:e}, exponent={exponent}"
                    );
                    let complex = Complex32::new(value, -value);
                    let actual = multiply_complex_scaled(complex, factor, exponent);
                    let expected = Complex32::new(
                        reference_multiply_component_scaled(value, factor, exponent),
                        reference_multiply_component_scaled(-value, factor, exponent),
                    );
                    assert_eq!(actual.re.to_bits(), expected.re.to_bits());
                    assert_eq!(actual.im.to_bits(), expected.im.to_bits());
                }
            }
        }

        for (complex, factor, exponent) in [
            (Complex32::new(1.5, 0.25), 0.75, 2),
            (Complex32::new(f32::MIN_POSITIVE, -1.5), 0.5, -1),
            (Complex32::new(f32::MAX, 0.25), 2.0, 0),
        ] {
            let actual = multiply_complex_scaled(complex, factor, exponent);
            let expected = Complex32::new(
                reference_multiply_component_scaled(complex.re, factor, exponent),
                reference_multiply_component_scaled(complex.im, factor, exponent),
            );
            assert_eq!(actual.re.to_bits(), expected.re.to_bits());
            assert_eq!(actual.im.to_bits(), expected.im.to_bits());
        }
    }

    fn values_over_rings(arch: Arch, m: usize, lmax: usize, theta: &[f32]) -> Vec<Vec<f32>> {
        let sin_theta = theta.iter().map(|value| value.sin()).collect::<Vec<_>>();
        let cos_theta = theta.iter().map(|value| value.cos()).collect::<Vec<_>>();
        let mut epsilon = vec![0.0; lmax + 1];
        fill_epsilon(m, lmax, &mut epsilon);
        let scratch_len = theta.len().min(LATITUDE_BLOCK_SIZE);
        let mut previous = vec![0.0; scratch_len];
        let mut current = vec![0.0; scratch_len];
        let mut next = vec![0.0; scratch_len];
        let mut exponents = vec![0; scratch_len];
        let mut result = vec![vec![0.0; theta.len()]; lmax - m + 1];
        let sectoral_norm = sectoral_normalizations(m).unwrap()[m];
        for_each_y_lm_dispatched(
            arch,
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &epsilon,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
            |degree, ring_start, values, exponents| {
                for lane in 0..values.len() {
                    result[degree - m][ring_start + lane] =
                        scale_binary(values[lane], exponents[lane]);
                }
                Ok(())
            },
        )
        .unwrap();
        result
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

        for m in [0, 1, 2, 31, 96] {
            let expected = values_over_rings(Arch::Scalar, m, 127, &theta);
            let actual = values_over_rings(Arch::new(), m, 127, &theta);
            for (degree_offset, (expected_values, actual_values)) in
                expected.iter().zip(&actual).enumerate()
            {
                for (ring, (&expected, &actual)) in
                    expected_values.iter().zip(actual_values).enumerate()
                {
                    assert!(
                        actual.is_finite(),
                        "m={m}, l={}, ring={ring}: non-finite result {actual}",
                        m + degree_offset
                    );
                    assert!(
                        (actual - expected).abs() <= 5.0e-6,
                        "m={m}, l={}, ring={ring}: actual {actual:.9e}, scalar {expected:.9e}",
                        m + degree_offset
                    );
                }
            }
        }
    }
}
