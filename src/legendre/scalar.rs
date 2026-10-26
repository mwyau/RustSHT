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

#[cfg(test)]
use std::f64::consts::PI;

use num_complex::Complex64;
use pulp::{Arch, Simd, WithSimd};

use crate::error::ShtError;
use crate::legendre::{
    LegendrePrecision, ScaledValue as BinaryScaledValue, define_scale_binary,
    scalar_recurrence_step,
};

type ScaledValue = BinaryScaledValue<f64>;

pub(super) const BIG: f64 = <f64 as LegendrePrecision>::BIG;
pub(super) const SMALL: f64 = <f64 as LegendrePrecision>::SMALL;
const SCALE_UP: f64 = <f64 as LegendrePrecision>::SCALE_UP;
const SCALE_DOWN: f64 = <f64 as LegendrePrecision>::SCALE_DOWN;
const SCALE_EXPONENT: i64 = <f64 as LegendrePrecision>::SCALE_EXPONENT;
pub(crate) const LATITUDE_BLOCK_SIZE: usize = 64;

#[inline]
pub(super) fn scalar_value_in_normal_range(value: f64) -> bool {
    super::numeric::value_in_normal_range(value)
}

#[inline(always)]
pub(super) fn simd_values_in_normal_range<S: Simd>(simd: S, values: S::f64s) -> bool {
    let absolute = simd.abs_f64s(values);
    let upper_mask = simd.less_than_or_equal_f64s(absolute, simd.splat_f64s(BIG));
    let upper_ok = simd.select_f64s(upper_mask, simd.splat_f64s(1.0), simd.splat_f64s(0.0));
    if simd.reduce_min_f64s(upper_ok) != 1.0 {
        return false;
    }
    let zero_mask = simd.equal_f64s(absolute, simd.splat_f64s(0.0));
    let lower_values = simd.select_f64s(zero_mask, simd.splat_f64s(BIG), absolute);
    simd.reduce_min_f64s(lower_values) >= SMALL
}

#[inline]
pub(super) fn lane_values_in_normal_range(values: &[f64]) -> bool {
    values
        .iter()
        .all(|&value| scalar_value_in_normal_range(value))
}

/// Returns the scalar sectoral normalizations used by the DUCC recurrence.
pub(crate) fn sectoral_normalizations(mmax: usize) -> Result<Vec<f64>, ShtError> {
    super::numeric::sectoral_normalizations(mmax)
}

/// Fills the order-dependent scalar recurrence factors through `lmax`.
pub(crate) fn fill_epsilon(m: usize, lmax: usize, output: &mut [f64]) {
    super::numeric::fill_epsilon(m, lmax, output);
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
        let mut normal_range = false;
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

            scalar_recurrence_step!(
                simd,
                S,
                epsilon,
                l,
                current,
                previous,
                cos_theta,
                next,
                splat_f64s,
                as_simd_f64s,
                as_mut_simd_f64s,
                mul_f64s,
                sub_f64s,
                div_f64s
            );

            if normal_range {
                if pair_lanes_need_scaling(simd, current, next) {
                    normal_range = normalize_pair_lanes(current, next, exponents)?;
                }
            } else {
                normal_range = normalize_pair_lanes(current, next, exponents)?;
            }
            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
        }
        Ok(())
    }
}

pub(super) fn normalize_pair_lanes(
    previous: &mut [f64],
    current: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    debug_assert_eq!(previous.len(), current.len());
    debug_assert_eq!(previous.len(), exponents.len());
    let mut normal_range = true;
    for ring in 0..previous.len() {
        if exponents[ring] == 0 && pair_is_in_normal_range(previous[ring], current[ring]) {
            continue;
        }
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
        normal_range &=
            exponents[ring] == 0 && pair_is_in_normal_range(previous[ring], current[ring]);
    }
    Ok(normal_range)
}

/// Normalizes paired recurrence lanes with SIMD masks and exact checked
/// exponent updates. `None` asks the caller to use the robust scalar route.
#[inline(always)]
pub(super) fn normalize_pair_lanes_simd<S: Simd>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    exponents: &mut [i64],
) -> Option<bool> {
    if previous.len() != current.len() || current.len() != exponents.len() {
        return None;
    }

    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
    let (exponent_vectors, exponent_tail) = S::as_mut_simd_i64s(exponents);
    let mut all_normal = true;

    for index in 0..previous_vectors.len() {
        let mut previous_value = previous_vectors[index];
        let mut current_value = current_vectors[index];
        let mut exponent = exponent_vectors[index];
        all_normal &= normalize_pair_vector_simd(
            simd,
            &mut previous_value,
            &mut current_value,
            &mut exponent,
        )?;
        previous_vectors[index] = previous_value;
        current_vectors[index] = current_value;
        exponent_vectors[index] = exponent;
    }

    for lane in 0..previous_tail.len() {
        if !previous_tail[lane].is_finite() || !current_tail[lane].is_finite() {
            return None;
        }
        let normalized = normalize_pair_lanes(
            &mut previous_tail[lane..lane + 1],
            &mut current_tail[lane..lane + 1],
            &mut exponent_tail[lane..lane + 1],
        )
        .ok()?;
        all_normal &= normalized;
    }
    Some(all_normal)
}

/// Applies the checked exponent normalization for one SIMD vector. Keeping
/// this operation separate lets recurrence kernels fuse it into their vector
/// update loop without changing the checks or scaling order.
#[inline(always)]
pub(super) fn normalize_pair_vector_simd<S: Simd>(
    simd: S,
    previous_value: &mut S::f64s,
    current_value: &mut S::f64s,
    exponent: &mut S::i64s,
) -> Option<bool> {
    let maximum = simd.splat_f64s(BIG);
    let minimum = simd.splat_f64s(SMALL);
    let zero_f64 = simd.splat_f64s(0.0);
    let maximum_finite = simd.splat_f64s(f64::MAX);
    let scale_up = simd.splat_f64s(SCALE_UP);
    let scale_down = simd.splat_f64s(SCALE_DOWN);
    let exponent_step = simd.splat_i64s(SCALE_EXPONENT);
    let zero_i64 = simd.splat_i64s(0);

    let previous_magnitude = simd.abs_f64s(*previous_value);
    let current_magnitude = simd.abs_f64s(*current_value);
    let finite = simd.and_m64s(
        simd.less_than_or_equal_f64s(previous_magnitude, maximum_finite),
        simd.less_than_or_equal_f64s(current_magnitude, maximum_finite),
    );
    if !mask_all_f64(simd, finite) {
        return None;
    }

    let mut magnitude = simd.max_f64s(previous_magnitude, current_magnitude);
    let in_range = pair_magnitudes_in_range(simd, magnitude, minimum, maximum, zero_f64);
    if mask_all_f64(simd, in_range) {
        return Some(mask_all_i64(simd, simd.equal_i64s(*exponent, zero_i64)));
    }

    loop {
        let too_large = simd.greater_than_f64s(magnitude, maximum);
        if !mask_any_f64(simd, too_large) {
            break;
        }
        let next_exponent = simd.add_i64s(*exponent, exponent_step);
        let overflow = simd.and_m64s(too_large, simd.less_than_i64s(next_exponent, *exponent));
        if mask_any_f64(simd, overflow) {
            return None;
        }
        *previous_value = simd.select_f64s(
            too_large,
            simd.mul_f64s(*previous_value, scale_down),
            *previous_value,
        );
        *current_value = simd.select_f64s(
            too_large,
            simd.mul_f64s(*current_value, scale_down),
            *current_value,
        );
        *exponent = simd.select_i64s(too_large, next_exponent, *exponent);
        magnitude = simd.max_f64s(
            simd.abs_f64s(*previous_value),
            simd.abs_f64s(*current_value),
        );
    }

    loop {
        let nonzero = simd.not_m64s(simd.equal_f64s(magnitude, zero_f64));
        let too_small = simd.and_m64s(nonzero, simd.less_than_f64s(magnitude, minimum));
        if !mask_any_f64(simd, too_small) {
            break;
        }
        let next_exponent = simd.sub_i64s(*exponent, exponent_step);
        let overflow = simd.and_m64s(too_small, simd.greater_than_i64s(next_exponent, *exponent));
        if mask_any_f64(simd, overflow) {
            return None;
        }
        *previous_value = simd.select_f64s(
            too_small,
            simd.mul_f64s(*previous_value, scale_up),
            *previous_value,
        );
        *current_value = simd.select_f64s(
            too_small,
            simd.mul_f64s(*current_value, scale_up),
            *current_value,
        );
        *exponent = simd.select_i64s(too_small, next_exponent, *exponent);
        magnitude = simd.max_f64s(
            simd.abs_f64s(*previous_value),
            simd.abs_f64s(*current_value),
        );
    }

    let in_range = pair_magnitudes_in_range(simd, magnitude, minimum, maximum, zero_f64);
    Some(mask_all_i64(simd, simd.equal_i64s(*exponent, zero_i64)) && mask_all_f64(simd, in_range))
}

#[inline(always)]
fn pair_magnitudes_in_range<S: Simd>(
    simd: S,
    magnitude: S::f64s,
    minimum: S::f64s,
    maximum: S::f64s,
    zero: S::f64s,
) -> S::m64s {
    simd.and_m64s(
        simd.less_than_or_equal_f64s(magnitude, maximum),
        simd.or_m64s(
            simd.equal_f64s(magnitude, zero),
            simd.greater_than_or_equal_f64s(magnitude, minimum),
        ),
    )
}

#[inline(always)]
fn mask_any_f64<S: Simd>(simd: S, mask: S::m64s) -> bool {
    let selected = simd.select_f64s(mask, simd.splat_f64s(1.0), simd.splat_f64s(0.0));
    simd.reduce_max_f64s(selected) > 0.0
}

#[inline(always)]
fn mask_all_f64<S: Simd>(simd: S, mask: S::m64s) -> bool {
    let selected = simd.select_f64s(mask, simd.splat_f64s(1.0), simd.splat_f64s(0.0));
    simd.reduce_min_f64s(selected) > 0.0
}

#[inline(always)]
fn mask_all_i64<S: Simd>(simd: S, mask: S::m64s) -> bool {
    let selected = simd.select_f64s(mask, simd.splat_f64s(1.0), simd.splat_f64s(0.0));
    simd.reduce_min_f64s(selected) > 0.0
}

#[cfg(test)]
#[inline(always)]
pub(super) fn normalize_pair_lane(
    previous: &mut f64,
    current: &mut f64,
    exponent: &mut i64,
) -> Result<bool, ShtError> {
    if *exponent == 0 && pair_is_in_normal_range(*previous, *current) {
        return Ok(true);
    }
    let mut previous_value = ScaledValue {
        mantissa: *previous,
        exponent: *exponent,
    };
    let mut current_value = ScaledValue {
        mantissa: *current,
        exponent: *exponent,
    };
    normalize_pair(&mut previous_value, &mut current_value)?;
    *previous = previous_value.mantissa;
    *current = current_value.mantissa;
    *exponent = current_value.exponent;
    Ok(*exponent == 0 && pair_is_in_normal_range(*previous, *current))
}

pub(super) fn sectoral_value(
    sin_theta: f64,
    m: usize,
    sectoral_norm: f64,
) -> Result<(f64, i64), ShtError> {
    let value = scaled_power(sin_theta, m)?;
    let value = normalize_value(
        value.mantissa * sectoral_norm * if m.is_multiple_of(2) { 1.0 } else { -1.0 },
        value.exponent,
    )?;
    Ok((value.mantissa, value.exponent))
}

#[inline(always)]
fn pair_is_in_normal_range(previous: f64, current: f64) -> bool {
    let maximum = previous.abs().max(current.abs());
    maximum <= BIG && (maximum == 0.0 || maximum >= SMALL)
}

#[inline(always)]
pub(super) fn pair_lanes_need_scaling<S: Simd>(simd: S, previous: &[f64], current: &[f64]) -> bool {
    debug_assert_eq!(previous.len(), current.len());
    let (previous_vectors, previous_tail) = S::as_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_simd_f64s(current);
    debug_assert_eq!(previous_vectors.len(), current_vectors.len());
    debug_assert_eq!(previous_tail.len(), current_tail.len());

    let big = simd.splat_f64s(BIG);
    let zeros = simd.splat_f64s(0.0);
    for index in 0..previous_vectors.len() {
        let previous = simd.abs_f64s(previous_vectors[index]);
        let current = simd.abs_f64s(current_vectors[index]);
        let previous_ordered = simd.equal_f64s(previous, previous);
        let current_ordered = simd.equal_f64s(current, current);
        let maximum = simd.max_f64s(previous, current);
        let maximum = simd.select_f64s(current_ordered, maximum, previous);
        let maximum = simd.select_f64s(previous_ordered, maximum, current);
        let below_upper_bound = simd.less_than_or_equal_f64s(maximum, big);
        let is_zero = simd.equal_f64s(maximum, zeros);
        let lower_check = simd.select_f64s(is_zero, big, maximum);
        let in_range = simd.select_f64s(below_upper_bound, lower_check, zeros);
        if simd.reduce_min_f64s(in_range) < SMALL {
            return true;
        }
    }

    previous_tail
        .iter()
        .zip(current_tail)
        .any(|(&previous, &current)| !pair_is_in_normal_range(previous, current))
}

/// Multiplies a complex value by a scaled recurrence value without overflow.
pub(crate) fn multiply_complex_scaled(value: Complex64, factor: f64, exponent: i64) -> Complex64 {
    if factor.is_normal() {
        if value.re.is_normal() && value.im.is_normal() {
            let product = Complex64::new(value.re * factor, value.im * factor);
            if product.re.is_normal() && product.im.is_normal() {
                if exponent == 0 {
                    return product;
                }
                if let Some(scaled) = scale_complex_normal_binary(product, exponent) {
                    return scaled;
                }
            }
        }
        return Complex64::new(
            multiply_component_scaled_with_normal_factor(value.re, factor, exponent),
            multiply_component_scaled_with_normal_factor(value.im, factor, exponent),
        );
    }
    Complex64::new(
        multiply_component_scaled(value.re, factor, exponent),
        multiply_component_scaled(value.im, factor, exponent),
    )
}

pub(crate) fn multiply_complex_scaled_product(
    value: Complex64,
    factor: f64,
    additional_factor: f64,
    exponent: i64,
) -> Complex64 {
    if factor.is_normal()
        && additional_factor.is_normal()
        && value.re.is_normal()
        && value.im.is_normal()
    {
        let product = Complex64::new(
            (value.re * factor) * additional_factor,
            (value.im * factor) * additional_factor,
        );
        if product.re.is_normal() && product.im.is_normal() {
            if exponent == 0 {
                return product;
            }
            if let Some(scaled) = scale_complex_normal_binary(product, exponent) {
                return scaled;
            }
        }
    }
    Complex64::new(
        multiply_component_scaled_product(value.re, factor, additional_factor, exponent),
        multiply_component_scaled_product(value.im, factor, additional_factor, exponent),
    )
}

fn scale_complex_normal_binary(value: Complex64, exponent: i64) -> Option<Complex64> {
    if exponent == 0 {
        return Some(value);
    }
    if !(-4096..=4096).contains(&exponent) {
        return None;
    }

    let real_bits = value.re.to_bits();
    let imag_bits = value.im.to_bits();
    let real_exponent = ((real_bits >> 52) & 0x7ff) as i64 + exponent;
    let imag_exponent = ((imag_bits >> 52) & 0x7ff) as i64 + exponent;
    if !(1..0x7ff).contains(&real_exponent) || !(1..0x7ff).contains(&imag_exponent) {
        return None;
    }

    let exponent_mask = 0x7ff_u64 << 52;
    Some(Complex64::new(
        f64::from_bits((real_bits & !exponent_mask) | ((real_exponent as u64) << 52)),
        f64::from_bits((imag_bits & !exponent_mask) | ((imag_exponent as u64) << 52)),
    ))
}

fn multiply_component_scaled_with_normal_factor(value: f64, factor: f64, exponent: i64) -> f64 {
    if value.is_normal() {
        let product = value * factor;
        if product.is_normal() {
            let scaled = scale_normal_binary(product, exponent);
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

fn multiply_component_scaled(value: f64, factor: f64, exponent: i64) -> f64 {
    if value.is_normal() && factor.is_normal() {
        let product = value * factor;
        if product.is_normal() {
            let scaled = scale_normal_binary(product, exponent);
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

fn multiply_component_scaled_product(
    value: f64,
    factor: f64,
    additional_factor: f64,
    exponent: i64,
) -> f64 {
    if value == 0.0
        || factor == 0.0
        || additional_factor == 0.0
        || !value.is_finite()
        || !factor.is_finite()
        || !additional_factor.is_finite()
    {
        return scale_binary((value * factor) * additional_factor, exponent);
    }
    let (value_mantissa, value_exponent) = split_binary(value);
    let (factor_mantissa, factor_exponent) = split_binary(factor);
    let (additional_mantissa, additional_exponent) = split_binary(additional_factor);
    scale_binary(
        (value_mantissa * factor_mantissa) * additional_mantissa,
        exponent
            .saturating_add(value_exponent)
            .saturating_add(factor_exponent)
            .saturating_add(additional_exponent),
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

#[cfg(test)]
fn epsilon(l: usize, m: usize) -> f64 {
    super::numeric::epsilon(l, m)
}

// Keep the f64 transform leaf local: the shared generic form regressed the
// measured T720 scalar synthesis q8 route.
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

fn normalize_pair(previous: &mut ScaledValue, current: &mut ScaledValue) -> Result<(), ShtError> {
    let mut maximum = previous.mantissa.abs().max(current.mantissa.abs());
    while maximum > BIG {
        previous.mantissa *= SCALE_DOWN;
        current.mantissa *= SCALE_DOWN;
        previous.exponent =
            previous
                .exponent
                .checked_add(SCALE_EXPONENT)
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
                    .checked_sub(SCALE_EXPONENT)
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "scalar recurrence scale",
                    })?;
            current.exponent = previous.exponent;
            maximum *= SCALE_UP;
        }
    }
    Ok(())
}

define_scale_binary! {
    pub(crate) fn scale_binary {
        float: f64,
        bits: u64,
        mantissa_bits: 52,
        exponent_field_mask: 0x7ff,
        maximum_exponent: 0x7ff,
        upper_chunk: 1023,
        lower_chunk: -1022,
    }
}

fn scale_normal_binary(value: f64, exponent: i64) -> f64 {
    debug_assert!(value.is_normal());
    if exponent == 0 {
        return value;
    }
    if exponent > 4096 {
        return f64::INFINITY.copysign(value);
    }
    if exponent < -4096 {
        return 0.0_f64.copysign(value);
    }

    let bits = value.to_bits();
    let raw_exponent = ((bits >> 52) & 0x7ff) as i64;
    let scaled_exponent = raw_exponent + exponent;
    if (1..0x7ff).contains(&scaled_exponent) {
        let exponent_mask = 0x7ff_u64 << 52;
        return f64::from_bits((bits & !exponent_mask) | ((scaled_exponent as u64) << 52));
    }
    scale_binary(value, exponent)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PairLaneNormalization<'a> {
        previous: &'a mut [f64],
        current: &'a mut [f64],
        exponents: &'a mut [i64],
    }

    impl WithSimd for PairLaneNormalization<'_> {
        type Output = Option<bool>;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            normalize_pair_lanes_simd(simd, self.previous, self.current, self.exponents)
        }
    }

    struct PairRangeCheck<'a> {
        previous: &'a [f64],
        current: &'a [f64],
    }

    impl WithSimd for PairRangeCheck<'_> {
        type Output = bool;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            pair_lanes_need_scaling(simd, self.previous, self.current)
        }
    }

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
    fn paired_range_guard_matches_scalar_reference_on_vectors_and_tails() {
        let cases = [
            (0.0, 0.0),
            (0.0, SMALL / 2.0),
            (BIG, SMALL / 2.0),
            (BIG * 2.0, 0.0),
            (SMALL, BIG),
            (f64::NAN, BIG),
            (BIG, f64::NAN),
            (f64::NAN, SMALL / 2.0),
            (f64::NAN, f64::NAN),
            (f64::INFINITY, f64::NAN),
        ];

        for (previous_value, current_value) in cases {
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

        for failing_lane in [3, 16] {
            let mut previous = [BIG; 17];
            let mut current = [SMALL; 17];
            previous[failing_lane] = 0.0;
            current[failing_lane] = SMALL / 2.0;
            let expected = previous
                .iter()
                .zip(current)
                .any(|(&previous, current)| !pair_is_in_normal_range(previous, current));
            for arch in [Arch::Scalar, Arch::new()] {
                let actual = arch.dispatch(PairRangeCheck {
                    previous: &previous,
                    current: &current,
                });
                assert!(actual, "failing lane {failing_lane}");
                assert_eq!(actual, expected, "failing lane {failing_lane}");
            }
        }
    }

    #[test]
    fn ducc_paired_degree_recurrence_preserves_coefficient_transform_and_adjoint() {
        let lmax = 128;
        let theta = [0.003, 0.37, 1.1, 1.57, std::f64::consts::PI - 0.003];
        let sin_theta: Vec<_> = theta.iter().map(|value| value.sin()).collect();
        let cos_theta: Vec<_> = theta.iter().map(|value| value.cos()).collect();
        let mut epsilon = vec![0.0; lmax + 4];
        for m in 0..=lmax {
            fill_epsilon(m, lmax, &mut epsilon[..=lmax]);
            for (l, value) in epsilon.iter_mut().enumerate().skip(lmax + 1) {
                *value = super::epsilon(l, m);
            }

            let degrees = lmax - m + 1;
            let coefficients: Vec<_> = (m..=lmax)
                .map(|l| ((l as f64 * 1.618 + m as f64).sin()) / (l - m + 1) as f64)
                .collect();
            let factor_count = lmax / 2 + 2;
            let mut alpha = vec![0.0; factor_count];
            let mut coefficient_a = vec![0.0; factor_count];
            let mut coefficient_b = vec![0.0; factor_count];
            super::super::scalar_paired::fill_factors(
                m,
                lmax,
                &epsilon,
                &mut alpha,
                &mut coefficient_a,
                &mut coefficient_b,
            );
            let pair_count = (lmax - m + 2) / 2;
            let mut paired_by_arch = Vec::new();
            for arch in [Arch::Scalar, Arch::new()] {
                let mut previous = vec![0.0; theta.len()];
                let mut current = vec![0.0; theta.len()];
                let mut next = vec![0.0; theta.len()];
                let mut exponents = vec![0; theta.len()];
                let mut paired = vec![vec![0.0; theta.len()]; pair_count];
                super::super::scalar_paired::for_each_y_lm_paired_status_dispatched(
                    arch,
                    m,
                    lmax,
                    &sin_theta,
                    &cos_theta,
                    sectoral_normalizations(m).unwrap()[m],
                    &coefficient_a,
                    &coefficient_b,
                    &mut previous,
                    &mut current,
                    &mut next,
                    &mut exponents,
                    |l, ring_start, values, exponents, normal_range| {
                        let pair = (l - m) / 2;
                        if normal_range {
                            assert!(exponents.iter().all(|&exponent| exponent == 0));
                        }
                        for lane in 0..values.len() {
                            paired[pair][ring_start + lane] =
                                scale_binary(values[lane], exponents[lane]);
                        }
                        Ok(())
                    },
                )
                .unwrap();
                paired_by_arch.push(paired);
            }
            let paired = &paired_by_arch[1];
            for (pair, pair_values) in paired.iter().enumerate() {
                for (ring, &actual) in pair_values.iter().enumerate() {
                    let reference = paired_by_arch[0][pair][ring];
                    let error = (actual - reference).abs();
                    assert!(
                        error <= 2e-12 * reference.abs().max(1.0),
                        "m={m}, pair={pair}, ring={ring}, SIMD={actual:.16e}, scalar={reference:.16e}"
                    );
                }
            }

            for (ring, &theta_value) in theta.iter().enumerate() {
                let cos = cos_theta[ring];
                let direct = values(m, lmax, theta_value);
                let paired: Vec<_> = paired.iter().map(|pair| pair[ring]).collect();

                let mut synthesis_direct = 0.0;
                let mut synthesis_paired = 0.0;
                let mut analysis_even = vec![0.0; paired.len()];
                let mut analysis_odd = vec![0.0; paired.len()];
                for (pair, l) in (m..=lmax).step_by(2).enumerate() {
                    let offset = l - m;
                    synthesis_direct += coefficients[offset] * direct[offset];
                    let next_even = if l + 2 <= lmax {
                        coefficients[offset + 2]
                    } else {
                        0.0
                    };
                    let transformed_even = alpha[pair]
                        * (epsilon[l + 1] * coefficients[offset] + epsilon[l + 2] * next_even);
                    let transformed_odd = if l < lmax {
                        coefficients[offset + 1]
                    } else {
                        0.0
                    };
                    synthesis_paired +=
                        paired[pair] * (transformed_even + cos * alpha[pair] * transformed_odd);
                    analysis_even[pair] = paired[pair] * 0.73;
                    analysis_odd[pair] = cos * paired[pair] * 0.73;
                    if l < lmax {
                        synthesis_direct += coefficients[offset + 1] * direct[offset + 1];
                    }
                }

                assert!(
                    (synthesis_paired - synthesis_direct).abs()
                        <= 2e-12 * synthesis_direct.abs().max(1.0),
                    "m={m}, theta={theta_value}, paired={synthesis_paired:.16e}, direct={synthesis_direct:.16e}"
                );

                let mut analysis_recovered = vec![0.0; degrees];
                for (pair, l) in (m..=lmax).step_by(2).enumerate() {
                    let previous = if pair == 0 {
                        0.0
                    } else {
                        alpha[pair - 1] * epsilon[l] * analysis_even[pair - 1]
                    };
                    analysis_recovered[l - m] =
                        alpha[pair] * epsilon[l + 1] * analysis_even[pair] + previous;
                    if l < lmax {
                        analysis_recovered[l + 1 - m] = alpha[pair] * analysis_odd[pair];
                    }
                }
                for (degree, (&actual, &expected)) in
                    analysis_recovered.iter().zip(&direct).enumerate()
                {
                    assert!(
                        (actual - 0.73 * expected).abs()
                            <= 2e-12 * (0.73 * expected).abs().max(1.0),
                        "m={m}, theta={theta_value}, l={}, recovered={actual:.16e}, reference={expected:.16e}",
                        m + degree
                    );
                }
            }
        }
    }

    #[test]
    fn fused_paired_synthesis_matches_scaled_reference_with_vector_tails() {
        let lmax = 95;
        let norms = sectoral_normalizations(lmax).unwrap();
        for lane_count in [64, 61] {
            let theta: Vec<_> = (0..lane_count)
                .map(|ring| 0.19 + 2.76 * ring as f64 / (lane_count - 1) as f64)
                .collect();
            let sin_theta: Vec<_> = theta.iter().map(|value| value.sin()).collect();
            let cos_theta: Vec<_> = theta.iter().map(|value| value.cos()).collect();
            let ring_order_limits = vec![lmax; lane_count];

            for m in [0, 1, 17, 78, 79] {
                let mut epsilon = vec![0.0; lmax + 4];
                for (degree, value) in epsilon.iter_mut().enumerate().skip(m) {
                    *value = super::epsilon(degree, m);
                }
                let factor_count = lmax / 2 + 2;
                let mut alpha = vec![0.0; factor_count];
                let mut coefficient_a = vec![0.0; factor_count];
                let mut coefficient_b = vec![0.0; factor_count];
                super::super::scalar_paired::fill_factors(
                    m,
                    lmax,
                    &epsilon,
                    &mut alpha,
                    &mut coefficient_a,
                    &mut coefficient_b,
                );
                let coefficients: Vec<_> = (m..=lmax)
                    .map(|degree| {
                        Complex64::new(
                            (0.31 * degree as f64 + 0.73 * m as f64).sin(),
                            (0.19 * degree as f64 - 0.41 * m as f64).cos(),
                        ) / (degree - m + 1) as f64
                    })
                    .collect();
                let second_coefficients: Vec<_> = (m..=lmax)
                    .map(|degree| {
                        Complex64::new(
                            (0.23 * degree as f64 - 0.37 * m as f64).cos(),
                            (0.29 * degree as f64 + 0.17 * m as f64).sin(),
                        ) / (degree - m + 2) as f64
                    })
                    .collect();

                for arch in [Arch::Scalar, Arch::new()] {
                    let spectrum_len = lmax + 1;
                    let mut actual = vec![Complex64::new(0.0, 0.0); lane_count * spectrum_len];
                    let mut previous = vec![0.0; lane_count];
                    let mut current = vec![0.0; lane_count];
                    let mut next = vec![0.0; lane_count];
                    let mut exponents = vec![0; lane_count];
                    let used = super::super::scalar_paired::synthesize_order_block_dispatched(
                        arch,
                        m,
                        lmax,
                        &sin_theta,
                        &cos_theta,
                        norms[m],
                        &epsilon,
                        &alpha,
                        &coefficient_a,
                        &coefficient_b,
                        &coefficients,
                        &ring_order_limits,
                        spectrum_len,
                        &mut actual,
                        &mut previous,
                        &mut current,
                        &mut next,
                        &mut exponents,
                    )
                    .unwrap();
                    assert!(used, "lane_count={lane_count}, m={m}, arch={arch:?}");

                    let mut dual_first = vec![Complex64::new(0.0, 0.0); lane_count * spectrum_len];
                    let mut dual_second = dual_first.clone();
                    let mut first_sum_re = vec![0.0; lane_count];
                    let mut first_sum_im = vec![0.0; lane_count];
                    let mut second_sum_re = vec![0.0; lane_count];
                    let mut second_sum_im = vec![0.0; lane_count];
                    let dual_used =
                        super::super::scalar_paired::synthesize_order_pair_block_dispatched(
                            arch,
                            m,
                            lmax,
                            &sin_theta,
                            &cos_theta,
                            norms[m],
                            &epsilon,
                            &alpha,
                            &coefficient_a,
                            &coefficient_b,
                            &coefficients,
                            &second_coefficients,
                            &ring_order_limits,
                            spectrum_len,
                            &mut dual_first,
                            &mut dual_second,
                            &mut previous,
                            &mut current,
                            &mut next,
                            &mut exponents,
                            &mut first_sum_re,
                            &mut first_sum_im,
                            &mut second_sum_re,
                            &mut second_sum_im,
                        )
                        .unwrap();
                    assert!(dual_used, "lane_count={lane_count}, m={m}, arch={arch:?}");
                    let mut separate_second =
                        vec![Complex64::new(0.0, 0.0); lane_count * spectrum_len];
                    let second_used =
                        super::super::scalar_paired::synthesize_order_block_dispatched(
                            arch,
                            m,
                            lmax,
                            &sin_theta,
                            &cos_theta,
                            norms[m],
                            &epsilon,
                            &alpha,
                            &coefficient_a,
                            &coefficient_b,
                            &second_coefficients,
                            &ring_order_limits,
                            spectrum_len,
                            &mut separate_second,
                            &mut previous,
                            &mut current,
                            &mut next,
                            &mut exponents,
                        )
                        .unwrap();
                    assert!(
                        second_used,
                        "second channel should use the ordinary fast path"
                    );
                    for ring in 0..lane_count {
                        let first_error = (dual_first[ring * spectrum_len + m]
                            - actual[ring * spectrum_len + m])
                            .norm();
                        let second_error = (dual_second[ring * spectrum_len + m]
                            - separate_second[ring * spectrum_len + m])
                            .norm();
                        assert!(first_error <= 3e-14, "first channel error={first_error:e}");
                        assert!(
                            second_error <= 3e-14,
                            "second channel error={second_error:e}"
                        );
                    }

                    let mut expected = vec![Complex64::new(0.0, 0.0); lane_count];
                    super::super::scalar_paired::for_each_y_lm_paired_dispatched(
                        arch,
                        m,
                        lmax,
                        &sin_theta,
                        &cos_theta,
                        norms[m],
                        &coefficient_a,
                        &coefficient_b,
                        &mut previous,
                        &mut current,
                        &mut next,
                        &mut exponents,
                        |degree, ring_start, basis, basis_exponents| {
                            let pair = (degree - m) / 2;
                            let offset = degree - m;
                            let next_even = if degree + 2 <= lmax {
                                coefficients[offset + 2]
                            } else {
                                Complex64::new(0.0, 0.0)
                            };
                            let even = (coefficients[offset] * epsilon[degree + 1]
                                + next_even * epsilon[degree + 2])
                                * alpha[pair];
                            let odd = if degree < lmax {
                                coefficients[offset + 1] * alpha[pair]
                            } else {
                                Complex64::new(0.0, 0.0)
                            };
                            for lane in 0..basis.len() {
                                expected[ring_start + lane] += super::multiply_complex_scaled(
                                    even,
                                    basis[lane],
                                    basis_exponents[lane],
                                )
                                    + super::multiply_complex_scaled_product(
                                        odd,
                                        basis[lane],
                                        cos_theta[ring_start + lane],
                                        basis_exponents[lane],
                                    );
                            }
                            Ok(())
                        },
                    )
                    .unwrap();

                    for ring in 0..lane_count {
                        let observed = actual[ring * spectrum_len + m];
                        let error = (observed - expected[ring]).norm();
                        assert!(
                            error <= 2e-12 * expected[ring].norm().max(1.0),
                            "lane_count={lane_count}, m={m}, ring={ring}, fused={observed:?}, reference={:?}, error={error:.3e}",
                            expected[ring]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn fused_paired_synthesis_declines_inactive_and_scaled_blocks() {
        let lmax = 32;
        let m = 3;
        let mut epsilon = vec![0.0; lmax + 4];
        for (degree, value) in epsilon.iter_mut().enumerate().skip(m) {
            *value = super::epsilon(degree, m);
        }
        let factor_count = lmax / 2 + 2;
        let mut alpha = vec![0.0; factor_count];
        let mut coefficient_a = vec![0.0; factor_count];
        let mut coefficient_b = vec![0.0; factor_count];
        super::super::scalar_paired::fill_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        );
        let coefficients = vec![Complex64::new(0.25, -0.5); lmax - m + 1];
        let norms = sectoral_normalizations(lmax).unwrap();
        let spectrum_len = lmax + 1;
        for (sin_theta, ring_order_limits, expected_used) in [
            (vec![0.5; 4], vec![lmax, lmax, m - 1, lmax], false),
            (vec![1e-200; 4], vec![lmax; 4], false),
        ] {
            let cos_theta = vec![0.8; sin_theta.len()];
            let mut spectra = vec![Complex64::new(0.0, 0.0); sin_theta.len() * spectrum_len];
            let mut previous = vec![0.0; sin_theta.len()];
            let mut current = vec![0.0; sin_theta.len()];
            let mut next = vec![0.0; sin_theta.len()];
            let mut exponents = vec![0; sin_theta.len()];
            let used = super::super::scalar_paired::synthesize_order_block_dispatched(
                Arch::new(),
                m,
                lmax,
                &sin_theta,
                &cos_theta,
                norms[m],
                &epsilon,
                &alpha,
                &coefficient_a,
                &coefficient_b,
                &coefficients,
                &ring_order_limits,
                spectrum_len,
                &mut spectra,
                &mut previous,
                &mut current,
                &mut next,
                &mut exponents,
            )
            .unwrap();
            assert_eq!(used, expected_used);
            assert!(
                spectra
                    .iter()
                    .all(|value| *value == Complex64::new(0.0, 0.0))
            );
        }
    }

    #[test]
    fn fused_paired_synthesis_matches_scaled_reference_at_high_order() {
        let lmax = 768;
        let m = 255;
        let norms = sectoral_normalizations(lmax).unwrap();
        let theta: Vec<_> = (0..64)
            .map(|ring| 0.42 + 2.30 * ring as f64 / 63.0)
            .collect();
        let sin_theta: Vec<_> = theta.iter().map(|value| value.sin()).collect();
        let cos_theta: Vec<_> = theta.iter().map(|value| value.cos()).collect();
        let ring_order_limits = vec![lmax; theta.len()];
        let mut epsilon = vec![0.0; lmax + 4];
        for (degree, value) in epsilon.iter_mut().enumerate().skip(m) {
            *value = super::epsilon(degree, m);
        }
        let factor_count = lmax / 2 + 2;
        let mut alpha = vec![0.0; factor_count];
        let mut coefficient_a = vec![0.0; factor_count];
        let mut coefficient_b = vec![0.0; factor_count];
        super::super::scalar_paired::fill_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        );
        let coefficients: Vec<_> = (m..=lmax)
            .map(|degree| {
                Complex64::new(
                    (0.31 * degree as f64 + 0.73 * m as f64).sin(),
                    (0.19 * degree as f64 - 0.41 * m as f64).cos(),
                ) / ((degree - m + 1) as f64).sqrt()
            })
            .collect();
        let spectrum_len = lmax + 1;
        let mut actual = vec![Complex64::new(0.0, 0.0); theta.len() * spectrum_len];
        let mut previous = vec![0.0; theta.len()];
        let mut current = vec![0.0; theta.len()];
        let mut next = vec![0.0; theta.len()];
        let mut exponents = vec![0; theta.len()];
        assert!(
            super::super::scalar_paired::synthesize_order_block_dispatched(
                Arch::new(),
                m,
                lmax,
                &sin_theta,
                &cos_theta,
                norms[m],
                &epsilon,
                &alpha,
                &coefficient_a,
                &coefficient_b,
                &coefficients,
                &ring_order_limits,
                spectrum_len,
                &mut actual,
                &mut previous,
                &mut current,
                &mut next,
                &mut exponents,
            )
            .unwrap()
        );

        let mut expected = vec![Complex64::new(0.0, 0.0); theta.len()];
        super::super::scalar_paired::for_each_y_lm_paired_dispatched(
            Arch::new(),
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            norms[m],
            &coefficient_a,
            &coefficient_b,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
            |degree, ring_start, basis, basis_exponents| {
                let pair = (degree - m) / 2;
                let offset = degree - m;
                let next_even = if degree + 2 <= lmax {
                    coefficients[offset + 2]
                } else {
                    Complex64::new(0.0, 0.0)
                };
                let even = (coefficients[offset] * epsilon[degree + 1]
                    + next_even * epsilon[degree + 2])
                    * alpha[pair];
                let odd = if degree < lmax {
                    coefficients[offset + 1] * alpha[pair]
                } else {
                    Complex64::new(0.0, 0.0)
                };
                for lane in 0..basis.len() {
                    expected[ring_start + lane] +=
                        super::multiply_complex_scaled(even, basis[lane], basis_exponents[lane])
                            + super::multiply_complex_scaled_product(
                                odd,
                                basis[lane],
                                cos_theta[ring_start + lane],
                                basis_exponents[lane],
                            );
                }
                Ok(())
            },
        )
        .unwrap();

        for ring in 0..theta.len() {
            let observed = actual[ring * spectrum_len + m];
            let error = (observed - expected[ring]).norm();
            assert!(
                error <= 5e-12 * expected[ring].norm().max(1.0),
                "ring={ring}, fused={observed:?}, reference={:?}, error={error:.3e}",
                expected[ring]
            );
        }
    }

    #[test]
    fn paired_analysis_simd_accumulation_matches_scaled_lane_reference() {
        for lane_count in [64, 61] {
            let same_real: Vec<_> = (0..lane_count)
                .map(|lane| (0.17 * lane as f64).sin())
                .collect();
            let same_imag: Vec<_> = (0..lane_count)
                .map(|lane| (0.11 * lane as f64).cos())
                .collect();
            let opposite_real: Vec<_> = (0..lane_count)
                .map(|lane| (0.07 * lane as f64 + 0.3).cos())
                .collect();
            let opposite_imag: Vec<_> = (0..lane_count)
                .map(|lane| (0.23 * lane as f64 - 0.2).sin())
                .collect();
            let basis: Vec<_> = (0..lane_count)
                .map(|lane| (0.13 * lane as f64 + 0.1).sin())
                .collect();
            let exponents = vec![0; lane_count];
            let cos_theta: Vec<_> = (0..lane_count)
                .map(|lane| (0.31 * lane as f64 + 0.2).cos())
                .collect();
            let products_safe = super::super::scalar_paired::analysis_products_safe(
                &same_real,
                &same_imag,
                &opposite_real,
                &opposite_imag,
                &cos_theta,
            );
            assert!(products_safe);

            let actual = super::super::scalar_paired::accumulate_analysis_pair_dispatched(
                Arch::new(),
                products_safe,
                true,
                &same_real,
                &same_imag,
                &opposite_real,
                &opposite_imag,
                &basis,
                &exponents,
                &cos_theta,
            );
            let mut expected_same = Complex64::new(0.0, 0.0);
            let mut expected_opposite = Complex64::new(0.0, 0.0);
            for lane in 0..lane_count {
                expected_same += multiply_complex_scaled(
                    Complex64::new(same_real[lane], same_imag[lane]),
                    basis[lane],
                    0,
                );
                expected_opposite += multiply_complex_scaled_product(
                    Complex64::new(opposite_real[lane], opposite_imag[lane]),
                    basis[lane],
                    cos_theta[lane],
                    0,
                );
            }

            for (actual, expected) in [actual.0, actual.1]
                .into_iter()
                .zip([expected_same, expected_opposite])
            {
                assert!(
                    (actual - expected).norm() < 2e-13 * expected.norm().max(1.0),
                    "lanes={lane_count}, actual={actual:?}, expected={expected:?}"
                );
            }
        }
    }

    #[test]
    fn paired_analysis_extreme_finite_products_use_scaled_fallback() {
        let same_real = [0.25];
        let same_imag = [-0.5];
        let opposite_real = [1.0e308];
        let opposite_imag = [-1.0e308];
        let basis = [2.0];
        let exponents = [0];
        let cos_theta = [0.5];
        let products_safe = super::super::scalar_paired::analysis_products_safe(
            &same_real,
            &same_imag,
            &opposite_real,
            &opposite_imag,
            &cos_theta,
        );
        assert!(!products_safe);

        let actual = super::super::scalar_paired::accumulate_analysis_pair_dispatched(
            Arch::new(),
            products_safe,
            true,
            &same_real,
            &same_imag,
            &opposite_real,
            &opposite_imag,
            &basis,
            &exponents,
            &cos_theta,
        );
        let expected_same =
            multiply_complex_scaled(Complex64::new(same_real[0], same_imag[0]), basis[0], 0);
        let expected_opposite = multiply_complex_scaled_product(
            Complex64::new(opposite_real[0], opposite_imag[0]),
            basis[0],
            cos_theta[0],
            0,
        );

        assert!(actual.1.re.is_finite() && actual.1.im.is_finite());
        assert_eq!(actual.0.re.to_bits(), expected_same.re.to_bits());
        assert_eq!(actual.0.im.to_bits(), expected_same.im.to_bits());
        assert_eq!(actual.1.re.to_bits(), expected_opposite.re.to_bits());
        assert_eq!(actual.1.im.to_bits(), expected_opposite.im.to_bits());
        assert_eq!(actual.1.re, 1.0e308);
        assert_eq!(actual.1.im, -1.0e308);
    }

    #[test]
    fn paired_analysis_scaled_lanes_use_the_general_multiply_path() {
        let same_real = [0.75, -0.25, 0.5];
        let same_imag = [-0.5, 0.875, -0.125];
        let opposite_real = [0.625, -0.75, 0.375];
        let opposite_imag = [0.25, 0.5, -0.625];
        let basis = [0.5, -0.75, 0.25];
        let exponents = [-1074, 900, -1075];
        let cos_theta = [0.25, -0.5, 0.75];
        let actual = super::super::scalar_paired::accumulate_analysis_pair_dispatched(
            Arch::new(),
            true,
            false,
            &same_real,
            &same_imag,
            &opposite_real,
            &opposite_imag,
            &basis,
            &exponents,
            &cos_theta,
        );
        let mut expected_same = Complex64::new(0.0, 0.0);
        let mut expected_opposite = Complex64::new(0.0, 0.0);
        for lane in 0..basis.len() {
            expected_same += multiply_complex_scaled(
                Complex64::new(same_real[lane], same_imag[lane]),
                basis[lane],
                exponents[lane],
            );
            expected_opposite += multiply_complex_scaled_product(
                Complex64::new(opposite_real[lane], opposite_imag[lane]),
                basis[lane],
                cos_theta[lane],
                exponents[lane],
            );
        }
        assert_eq!(actual.0.re.to_bits(), expected_same.re.to_bits());
        assert_eq!(actual.0.im.to_bits(), expected_same.im.to_bits());
        assert_eq!(actual.1.re.to_bits(), expected_opposite.re.to_bits());
        assert_eq!(actual.1.im.to_bits(), expected_opposite.im.to_bits());
    }

    #[test]
    fn paired_analysis_mixed_normal_and_scaled_lanes_match_lane_reference() {
        for lane_count in [64, 61] {
            let same_real: Vec<_> = (0..lane_count)
                .map(|lane| (0.17 * lane as f64).sin() * 0.5)
                .collect();
            let same_imag: Vec<_> = (0..lane_count)
                .map(|lane| (0.13 * lane as f64).cos() * 0.4)
                .collect();
            let opposite_real: Vec<_> = (0..lane_count)
                .map(|lane| (0.11 * lane as f64 + 0.3).cos() * 0.6)
                .collect();
            let opposite_imag: Vec<_> = (0..lane_count)
                .map(|lane| (0.07 * lane as f64 - 0.2).sin() * 0.3)
                .collect();
            let basis: Vec<_> = (0..lane_count)
                .map(|lane| (0.09 * lane as f64 + 0.1).sin())
                .collect();
            let exponents: Vec<_> = (0..lane_count)
                .map(|lane| match lane % 3 {
                    0 => 0,
                    1 => -12,
                    _ => 7,
                })
                .collect();
            let cos_theta: Vec<_> = (0..lane_count)
                .map(|lane| (0.05 * lane as f64 + 0.2).cos())
                .collect();
            let products_safe = super::super::scalar_paired::analysis_products_safe(
                &same_real,
                &same_imag,
                &opposite_real,
                &opposite_imag,
                &cos_theta,
            );
            assert!(products_safe);

            for arch in [Arch::Scalar, Arch::new()] {
                let actual = super::super::scalar_paired::accumulate_analysis_pair_dispatched(
                    arch,
                    products_safe,
                    false,
                    &same_real,
                    &same_imag,
                    &opposite_real,
                    &opposite_imag,
                    &basis,
                    &exponents,
                    &cos_theta,
                );
                let mut expected_same = Complex64::new(0.0, 0.0);
                let mut expected_opposite = Complex64::new(0.0, 0.0);
                for lane in 0..lane_count {
                    expected_same += multiply_complex_scaled(
                        Complex64::new(same_real[lane], same_imag[lane]),
                        basis[lane],
                        exponents[lane],
                    );
                    expected_opposite += multiply_complex_scaled_product(
                        Complex64::new(opposite_real[lane], opposite_imag[lane]),
                        basis[lane],
                        cos_theta[lane],
                        exponents[lane],
                    );
                }

                for (actual, expected) in [actual.0, actual.1]
                    .into_iter()
                    .zip([expected_same, expected_opposite])
                {
                    assert!(
                        (actual - expected).norm() <= 2e-13 * expected.norm().max(1.0),
                        "lanes={lane_count}, actual={actual:?}, expected={expected:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn paired_analysis_mixed_lanes_keep_subnormal_basis_on_scaled_path() {
        let lane_count = 17;
        let mut same_real = vec![0.0; lane_count];
        let mut same_imag = vec![0.0; lane_count];
        let mut opposite_real = vec![0.0; lane_count];
        let mut opposite_imag = vec![0.0; lane_count];
        let mut basis = vec![0.5; lane_count];
        let mut exponents = vec![-10; lane_count];
        let cos_theta = vec![1.0; lane_count];
        for lane in [0, lane_count - 1] {
            same_real[lane] = 1.0;
            same_imag[lane] = -1.0;
            opposite_real[lane] = 0.5;
            opposite_imag[lane] = -0.5;
            basis[lane] = f64::from_bits(1);
            exponents[lane] = 0;
        }
        let products_safe = super::super::scalar_paired::analysis_products_safe(
            &same_real,
            &same_imag,
            &opposite_real,
            &opposite_imag,
            &cos_theta,
        );
        assert!(products_safe);

        for arch in [Arch::Scalar, Arch::new()] {
            let actual = super::super::scalar_paired::accumulate_analysis_pair_dispatched(
                arch,
                products_safe,
                false,
                &same_real,
                &same_imag,
                &opposite_real,
                &opposite_imag,
                &basis,
                &exponents,
                &cos_theta,
            );
            let mut expected_same = Complex64::new(0.0, 0.0);
            let mut expected_opposite = Complex64::new(0.0, 0.0);
            for lane in 0..lane_count {
                expected_same += multiply_complex_scaled(
                    Complex64::new(same_real[lane], same_imag[lane]),
                    basis[lane],
                    exponents[lane],
                );
                expected_opposite += multiply_complex_scaled_product(
                    Complex64::new(opposite_real[lane], opposite_imag[lane]),
                    basis[lane],
                    cos_theta[lane],
                    exponents[lane],
                );
            }
            assert_eq!(actual.0.re.to_bits(), expected_same.re.to_bits());
            assert_eq!(actual.0.im.to_bits(), expected_same.im.to_bits());
            assert_eq!(actual.1.re.to_bits(), expected_opposite.re.to_bits());
            assert_eq!(actual.1.im.to_bits(), expected_opposite.im.to_bits());
        }
    }

    #[test]
    fn ducc_paired_recurrence_scaled_to_normal_transition_matches_reference() {
        let m = 255;
        let lmax = 768;
        let theta: [f64; 4] = [0.15, 0.2, 0.3, 0.8];
        let sin_theta: Vec<_> = theta.iter().map(|value| value.sin()).collect();
        let cos_theta: Vec<_> = theta.iter().map(|value| value.cos()).collect();
        let mut epsilon = vec![0.0; lmax + 4];
        fill_epsilon(m, lmax, &mut epsilon);
        let factor_count = lmax / 2 + 2;
        let mut alpha = vec![0.0; factor_count];
        let mut coefficient_a = vec![0.0; factor_count];
        let mut coefficient_b = vec![0.0; factor_count];
        super::super::scalar_paired::fill_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        );
        let coefficients: Vec<_> = (m..=lmax).map(|l| 1.0 / (l - m + 1) as f64).collect();
        let expected: Vec<_> = theta
            .iter()
            .map(|&angle| {
                values(m, lmax, angle)
                    .iter()
                    .zip(&coefficients)
                    .map(|(basis, coefficient)| basis * coefficient)
                    .sum::<f64>()
            })
            .collect();
        let mut previous = vec![0.0; theta.len()];
        let mut current = vec![0.0; theta.len()];
        let mut next = vec![0.0; theta.len()];
        let mut exponents = vec![0; theta.len()];
        let mut observed = vec![0.0; theta.len()];
        let mut saw_scaled = [false; 4];
        let mut saw_normal_after_scaled = [false; 4];

        super::super::scalar_paired::for_each_y_lm_paired_dispatched(
            Arch::new(),
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_normalizations(m).unwrap()[m],
            &coefficient_a,
            &coefficient_b,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
            |l, _, paired_basis, exponents| {
                let pair_index = (l - m) / 2;
                let offset = l - m;
                let next_even = if l + 2 <= lmax {
                    coefficients[offset + 2]
                } else {
                    0.0
                };
                let transformed_even = alpha[pair_index]
                    * (epsilon[l + 1] * coefficients[offset] + epsilon[l + 2] * next_even);
                let transformed_odd = if l < lmax {
                    alpha[pair_index] * coefficients[offset + 1]
                } else {
                    0.0
                };
                for lane in 0..paired_basis.len() {
                    let combined = transformed_even + cos_theta[lane] * transformed_odd;
                    observed[lane] += multiply_complex_scaled(
                        Complex64::new(combined, 0.0),
                        paired_basis[lane],
                        exponents[lane],
                    )
                    .re;
                    if exponents[lane] < 0 {
                        saw_scaled[lane] = true;
                    } else if saw_scaled[lane] {
                        saw_normal_after_scaled[lane] = true;
                    }
                }
                Ok(())
            },
        )
        .unwrap();

        for lane in 0..theta.len() {
            let error = (observed[lane] - expected[lane]).abs();
            assert!(
                error <= 2e-11 * expected[lane].abs().max(1.0),
                "m={m}, theta={}, paired={:.16e}, reference={:.16e}, error={error:.3e}",
                theta[lane],
                observed[lane],
                expected[lane]
            );
            if lane < 3 {
                assert!(saw_scaled[lane], "lane {lane} never used binary scaling");
                assert!(
                    saw_normal_after_scaled[lane],
                    "lane {lane} did not enter normal arithmetic after scaling"
                );
            }
        }
        assert!(!saw_scaled[3], "interior lane unexpectedly used scaling");
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
    fn scaled_to_normal_transition_matches_scalar_reference() {
        let m = 255;
        let lmax = 768;
        let theta: [f64; 4] = [0.15, 0.2, 0.3, 0.8];
        let sin_theta: Vec<_> = theta.iter().map(|theta| theta.sin()).collect();
        let cos_theta: Vec<_> = theta.iter().map(|theta| theta.cos()).collect();
        let norms = sectoral_normalizations(m).unwrap();
        let mut epsilon = vec![0.0; lmax + 1];
        fill_epsilon(m, lmax, &mut epsilon);
        let mut previous = vec![0.0; theta.len()];
        let mut current = vec![0.0; theta.len()];
        let mut next = vec![0.0; theta.len()];
        let mut exponents = vec![0; theta.len()];
        let expected: Vec<_> = theta.iter().map(|&theta| values(m, lmax, theta)).collect();
        let mut saw_scaled = [false; 4];
        let mut saw_normal_after_scaled = [false; 4];

        for_each_y_lm_dispatched(
            Arch::new(),
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
            |l, _, mantissas, exponents| {
                for lane in 0..mantissas.len() {
                    let observed = scale_binary(mantissas[lane], exponents[lane]);
                    let reference = expected[lane][l - m];
                    let relative_error = if reference == 0.0 {
                        if observed == 0.0 { 0.0 } else { f64::INFINITY }
                    } else {
                        (observed - reference).abs() / reference.abs()
                    };
                    assert!(
                        relative_error <= 2e-13,
                        "m={m}, l={l}, ring={lane}, observed={observed:.16e}, reference={reference:.16e}, relative error={relative_error:.3e}"
                    );
                    if exponents[lane] < 0 {
                        saw_scaled[lane] = true;
                    } else if saw_scaled[lane] {
                        saw_normal_after_scaled[lane] = true;
                    }
                }
                Ok(())
            },
        )
        .unwrap();

        assert!(saw_scaled[0]);
        assert!(saw_normal_after_scaled[0]);
        assert!(!saw_scaled[3]);
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
    fn simd_pair_normalization_matches_scalar_with_tails_and_mixed_scales() {
        for lane_count in [4, 9, 17] {
            let mut previous: Vec<_> = (0..lane_count)
                .map(|lane| match lane % 5 {
                    0 => BIG * 2.0,
                    1 => SMALL / 2.0,
                    2 => -BIG * 3.0,
                    3 => SMALL / 16.0,
                    _ => 0.375,
                })
                .collect();
            let mut current: Vec<_> = (0..lane_count)
                .map(|lane| match lane % 5 {
                    0 => -BIG * 1.5,
                    1 => SMALL / 4.0,
                    2 => SMALL / 8.0,
                    3 => BIG / 2.0,
                    _ => -0.625,
                })
                .collect();
            let mut exponents: Vec<_> = (0..lane_count)
                .map(|lane| lane as i64 * 800 - 1600)
                .collect();
            let mut expected_previous = previous.clone();
            let mut expected_current = current.clone();
            let mut expected_exponents = exponents.clone();
            let mut expected_normal = true;
            for lane in 0..lane_count {
                expected_normal &= normalize_pair_lanes(
                    &mut expected_previous[lane..lane + 1],
                    &mut expected_current[lane..lane + 1],
                    &mut expected_exponents[lane..lane + 1],
                )
                .unwrap();
            }

            let actual_normal = Arch::new().dispatch(PairLaneNormalization {
                previous: &mut previous,
                current: &mut current,
                exponents: &mut exponents,
            });
            assert_eq!(actual_normal, Some(expected_normal));
            assert_eq!(previous, expected_previous);
            assert_eq!(current, expected_current);
            assert_eq!(exponents, expected_exponents);
        }
    }

    #[test]
    fn simd_pair_normalization_declines_nonfinite_and_overflowing_lanes() {
        let mut previous = [BIG * 2.0, 0.0, 0.0, 0.0];
        let mut current = [0.5, 0.5, 0.5, 0.5];
        let mut exponents = [i64::MAX, 0, 0, 0];
        assert_eq!(
            Arch::new().dispatch(PairLaneNormalization {
                previous: &mut previous,
                current: &mut current,
                exponents: &mut exponents,
            }),
            None
        );

        let mut previous = [f64::INFINITY, 0.5, 0.5, 0.5];
        let mut current = [0.5; 4];
        let mut exponents = [0; 4];
        assert_eq!(
            Arch::new().dispatch(PairLaneNormalization {
                previous: &mut previous,
                current: &mut current,
                exponents: &mut exponents,
            }),
            None
        );
    }

    #[test]
    fn single_lane_normalization_matches_slice_normalization() {
        for (previous_value, current_value, exponent) in [
            (0.0, 0.0, 0),
            (0.5, -0.75, 800),
            (BIG * 2.0, -BIG * 1.5, 1),
            (SMALL / 4.0, -SMALL / 8.0, -3),
            (BIG * 2.0, -BIG, i64::MAX),
        ] {
            let mut previous = [previous_value];
            let mut current = [current_value];
            let mut exponents = [exponent];
            let mut single_previous = previous_value;
            let mut single_current = current_value;
            let mut single_exponent = exponent;
            let array_result = normalize_pair_lanes(&mut previous, &mut current, &mut exponents);
            let single_result = normalize_pair_lane(
                &mut single_previous,
                &mut single_current,
                &mut single_exponent,
            );
            assert_eq!(single_result, array_result);
            assert_eq!(single_previous.to_bits(), previous[0].to_bits());
            assert_eq!(single_current.to_bits(), current[0].to_bits());
            assert_eq!(single_exponent, exponents[0]);
        }
    }

    #[test]
    fn normalize_pair_lanes_preserves_ordinary_lanes_in_mixed_blocks() {
        let mut previous = [0.25, -BIG * 1.5, SMALL / 4.0, 0.0, 1.0];
        let mut current = [-0.5, BIG / 2.0, SMALL / 8.0, 0.0, f64::MIN_POSITIVE];
        let mut exponents = [0, 0, -7, 0, 0];
        let original = (previous, current, exponents);

        assert!(!normalize_pair_lanes(&mut previous, &mut current, &mut exponents).unwrap());
        for lane in [0, 3, 4] {
            assert_eq!(previous[lane].to_bits(), original.0[lane].to_bits());
            assert_eq!(current[lane].to_bits(), original.1[lane].to_bits());
            assert_eq!(exponents[lane], original.2[lane]);
        }
        assert_eq!(exponents[1], 800);
        assert_eq!(exponents[2], -807);
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
    fn normal_complex_products_skip_identity_binary_scaling() {
        let value = Complex64::new(0.75, -1.25);
        let factor = 1.125;
        let additional_factor = -0.875;
        let expected = Complex64::new(
            (value.re * factor) * additional_factor,
            (value.im * factor) * additional_factor,
        );

        let actual = multiply_complex_scaled_product(value, factor, additional_factor, 0);
        assert_eq!(actual.re.to_bits(), expected.re.to_bits());
        assert_eq!(actual.im.to_bits(), expected.im.to_bits());
    }

    #[test]
    fn complex_product_keeps_fallback_when_first_product_underflows() {
        let value = Complex64::new(1.0e-200, -2.0e-200);
        let factor = 1.0e-200;
        let additional_factor = 1.0e300;

        let actual = multiply_complex_scaled_product(value, factor, additional_factor, 0);
        let expected = Complex64::new(
            multiply_component_scaled_product(value.re, factor, additional_factor, 0),
            multiply_component_scaled_product(value.im, factor, additional_factor, 0),
        );
        assert_eq!(actual.re.to_bits(), expected.re.to_bits());
        assert_eq!(actual.im.to_bits(), expected.im.to_bits());
        assert!(actual.re.is_normal() && actual.im.is_normal());
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
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
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
                if value.is_normal() {
                    assert_eq!(
                        scale_normal_binary(value, exponent).to_bits(),
                        expected.to_bits(),
                        "normal value={value:e}, exponent={exponent}"
                    );
                }
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

                    let complex = Complex64::new(value, -value);
                    let actual = multiply_complex_scaled(complex, factor, exponent);
                    let expected = Complex64::new(
                        reference_multiply_component_scaled(value, factor, exponent),
                        reference_multiply_component_scaled(-value, factor, exponent),
                    );
                    assert_eq!(
                        actual.re.to_bits(),
                        expected.re.to_bits(),
                        "complex real value={value:e}, factor={factor:e}, exponent={exponent}"
                    );
                    assert_eq!(
                        actual.im.to_bits(),
                        expected.im.to_bits(),
                        "complex imaginary value={value:e}, factor={factor:e}, exponent={exponent}"
                    );
                }
            }
        }

        let complex = Complex64::new(1.25, f64::from_bits(1));
        let factor = 0.75;
        let exponent = 2;
        let actual = multiply_complex_scaled(complex, factor, exponent);
        let expected = Complex64::new(
            reference_multiply_component_scaled(complex.re, factor, exponent),
            reference_multiply_component_scaled(complex.im, factor, exponent),
        );
        assert_eq!(actual.re.to_bits(), expected.re.to_bits());
        assert_eq!(actual.im.to_bits(), expected.im.to_bits());

        for (complex, factor, exponent) in [
            (Complex64::new(1.5, 0.25), 0.75, 2),
            (Complex64::new(f64::MIN_POSITIVE, -1.5), 0.5, -1),
            (Complex64::new(f64::MAX, 0.25), 2.0, 0),
        ] {
            let actual = multiply_complex_scaled(complex, factor, exponent);
            let expected = Complex64::new(
                reference_multiply_component_scaled(complex.re, factor, exponent),
                reference_multiply_component_scaled(complex.im, factor, exponent),
            );
            assert_eq!(actual.re.to_bits(), expected.re.to_bits());
            assert_eq!(actual.im.to_bits(), expected.im.to_bits());
        }
    }

    #[test]
    fn paired_synthesis_transition_matches_scaled_reference_with_tail() {
        let m = 64;
        let lmax = 511;
        let lane_count = 63;
        let sin_theta: Vec<_> = (0..lane_count)
            .map(|lane| 0.01 + lane as f64 * 0.0001)
            .collect();
        let cos_theta: Vec<_> = sin_theta
            .iter()
            .map(|sine| (1.0 - sine * sine).sqrt())
            .collect();
        let sectoral_norm = sectoral_normalizations(m).unwrap()[m];
        let mut previous = vec![0.0; lane_count];
        let mut current = vec![0.0; lane_count];
        let mut exponents = vec![0; lane_count];
        for lane in 0..lane_count {
            let (value, exponent) = sectoral_value(sin_theta[lane], m, sectoral_norm).unwrap();
            previous[lane] = 0.0;
            current[lane] = value;
            exponents[lane] = exponent;
        }
        assert!(!normalize_pair_lanes(&mut previous, &mut current, &mut exponents).unwrap());

        let mut epsilon = vec![0.0; lmax + 4];
        fill_epsilon(m, lmax, &mut epsilon);
        let factor_count = lmax / 2 + 2;
        let mut alpha = vec![0.0; factor_count];
        let mut coefficient_a = vec![0.0; factor_count];
        let mut coefficient_b = vec![0.0; factor_count];
        super::super::scalar_paired::fill_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        );
        let first_coefficients: Vec<_> = (m..=lmax)
            .map(|degree| {
                Complex64::new(
                    ((degree * 7 + 3) % 19 + 1) as f64 * 1e-6,
                    ((degree * 11 + 5) % 23 + 1) as f64 * 1e-6,
                )
            })
            .collect();
        let second_coefficients: Vec<_> = (m..=lmax)
            .map(|degree| {
                Complex64::new(
                    ((degree * 13 + 7) % 17 + 1) as f64 * 1e-6,
                    -(((degree * 5 + 9) % 29 + 1) as f64) * 1e-6,
                )
            })
            .collect();

        for arch in [Arch::Scalar, Arch::new()] {
            let mut previous = vec![0.0; lane_count];
            let mut current = vec![0.0; lane_count];
            let mut next = vec![0.0; lane_count];
            let mut exponents = vec![0; lane_count];
            let mut first_sum_re = vec![0.0; lane_count];
            let mut first_sum_im = vec![0.0; lane_count];
            let mut second_sum_re = vec![0.0; lane_count];
            let mut second_sum_im = vec![0.0; lane_count];
            let staged =
                super::super::scalar_paired::try_synthesize_order_pair_transition_dispatched(
                    arch,
                    m,
                    lmax,
                    &sin_theta,
                    &cos_theta,
                    sectoral_norm,
                    &epsilon,
                    &alpha,
                    &coefficient_a,
                    &coefficient_b,
                    &first_coefficients,
                    &second_coefficients,
                    &mut previous,
                    &mut current,
                    &mut next,
                    &mut exponents,
                    &mut first_sum_re,
                    &mut first_sum_im,
                    &mut second_sum_re,
                    &mut second_sum_im,
                )
                .unwrap();
            assert!(staged, "the scaled recurrence should enter the normal path");

            let mut reference_previous = vec![0.0; lane_count];
            let mut reference_current = vec![0.0; lane_count];
            let mut reference_next = vec![0.0; lane_count];
            let mut reference_exponents = vec![0; lane_count];
            let mut reference_first = vec![Complex64::new(0.0, 0.0); lane_count];
            let mut reference_second = vec![Complex64::new(0.0, 0.0); lane_count];
            super::super::scalar_paired::for_each_y_lm_paired_dispatched(
                arch,
                m,
                lmax,
                &sin_theta,
                &cos_theta,
                sectoral_norm,
                &coefficient_a,
                &coefficient_b,
                &mut reference_previous,
                &mut reference_current,
                &mut reference_next,
                &mut reference_exponents,
                |l, ring_start, basis, lane_exponents| {
                    let pair = (l - m) / 2;
                    let offset = l - m;
                    let next_first_even = if l + 2 <= lmax {
                        first_coefficients[offset + 2]
                    } else {
                        Complex64::new(0.0, 0.0)
                    };
                    let next_second_even = if l + 2 <= lmax {
                        second_coefficients[offset + 2]
                    } else {
                        Complex64::new(0.0, 0.0)
                    };
                    let first_even = (first_coefficients[offset] * epsilon[l + 1]
                        + next_first_even * epsilon[l + 2])
                        * alpha[pair];
                    let second_even = (second_coefficients[offset] * epsilon[l + 1]
                        + next_second_even * epsilon[l + 2])
                        * alpha[pair];
                    let first_odd = if l < lmax {
                        first_coefficients[offset + 1] * alpha[pair]
                    } else {
                        Complex64::new(0.0, 0.0)
                    };
                    let second_odd = if l < lmax {
                        second_coefficients[offset + 1] * alpha[pair]
                    } else {
                        Complex64::new(0.0, 0.0)
                    };
                    for lane in 0..basis.len() {
                        let ring = ring_start + lane;
                        let inverse_sine = sin_theta[ring].recip();
                        let inverse_sine_squared = inverse_sine * inverse_sine;
                        let cosine_over_sine_squared = cos_theta[ring] * inverse_sine_squared;
                        reference_first[ring] += multiply_complex_scaled_product(
                            first_even,
                            basis[lane],
                            inverse_sine_squared,
                            lane_exponents[lane],
                        ) + multiply_complex_scaled_product(
                            first_odd,
                            basis[lane],
                            cosine_over_sine_squared,
                            lane_exponents[lane],
                        );
                        reference_second[ring] += multiply_complex_scaled_product(
                            second_even,
                            basis[lane],
                            inverse_sine_squared,
                            lane_exponents[lane],
                        ) + multiply_complex_scaled_product(
                            second_odd,
                            basis[lane],
                            cosine_over_sine_squared,
                            lane_exponents[lane],
                        );
                    }
                    Ok(())
                },
            )
            .unwrap();

            for lane in 0..lane_count {
                let inverse_sine = sin_theta[lane].recip();
                let inverse_sine_squared = inverse_sine * inverse_sine;
                let actual_first = Complex64::new(
                    first_sum_re[lane] * inverse_sine_squared,
                    first_sum_im[lane] * inverse_sine_squared,
                );
                let actual_second = Complex64::new(
                    second_sum_re[lane] * inverse_sine_squared,
                    second_sum_im[lane] * inverse_sine_squared,
                );
                assert!(
                    (actual_first - reference_first[lane]).norm()
                        <= 3e-12 * reference_first[lane].norm().max(1.0),
                    "first lane={lane}, staged={actual_first:?}, scaled={:?}",
                    reference_first[lane]
                );
                assert!(
                    (actual_second - reference_second[lane]).norm()
                        <= 3e-12 * reference_second[lane].norm().max(1.0),
                    "second lane={lane}, staged={actual_second:?}, scaled={:?}",
                    reference_second[lane]
                );
            }

            let mut extreme_first = first_coefficients.clone();
            extreme_first[0] = Complex64::new(f64::MAX, 0.0);
            assert!(
                !super::super::scalar_paired::try_synthesize_order_pair_transition_dispatched(
                    arch,
                    m,
                    lmax,
                    &sin_theta,
                    &cos_theta,
                    sectoral_norm,
                    &epsilon,
                    &alpha,
                    &coefficient_a,
                    &coefficient_b,
                    &extreme_first,
                    &second_coefficients,
                    &mut previous,
                    &mut current,
                    &mut next,
                    &mut exponents,
                    &mut first_sum_re,
                    &mut first_sum_im,
                    &mut second_sum_re,
                    &mut second_sum_im,
                )
                .unwrap()
            );
        }
    }

    #[test]
    fn staged_synthesis_transition_falls_back_on_degree_pair_cancellation() {
        let m = 2048;
        let lmax = m + 1;
        let sin_theta = [(3.0_f64).sqrt() * 0.5];
        let cos_theta = [0.5];
        let sectoral_norm = sectoral_normalizations(m).unwrap()[m];
        let mut epsilon = vec![0.0; lmax + 4];
        fill_epsilon(m, lmax, &mut epsilon);
        let factor_count = lmax / 2 + 2;
        let mut alpha = vec![0.0; factor_count];
        let mut coefficient_a = vec![0.0; factor_count];
        let mut coefficient_b = vec![0.0; factor_count];
        super::super::scalar_paired::fill_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        );
        let first_coefficients = [
            Complex64::new(1.0, 0.0),
            Complex64::new(-2.0 / alpha[0], 0.0),
        ];
        let second_coefficients = [Complex64::new(0.0, 0.0); 2];
        let mut previous = [0.0];
        let mut current = [0.0];
        let mut next = [0.0];
        let mut exponents = [0];
        let mut first_sum_re = [0.0];
        let mut first_sum_im = [0.0];
        let mut second_sum_re = [0.0];
        let mut second_sum_im = [0.0];

        for arch in [Arch::Scalar, Arch::new()] {
            assert!(
                !super::super::scalar_paired::try_synthesize_order_pair_transition_dispatched(
                    arch,
                    m,
                    lmax,
                    &sin_theta,
                    &cos_theta,
                    sectoral_norm,
                    &epsilon,
                    &alpha,
                    &coefficient_a,
                    &coefficient_b,
                    &first_coefficients,
                    &second_coefficients,
                    &mut previous,
                    &mut current,
                    &mut next,
                    &mut exponents,
                    &mut first_sum_re,
                    &mut first_sum_im,
                    &mut second_sum_re,
                    &mut second_sum_im,
                )
                .unwrap(),
                "the scaled path must handle a cancelled paired coefficient"
            );
            let even = first_coefficients[0] * epsilon[m + 1] * alpha[0];
            let odd = first_coefficients[1] * alpha[0];
            assert_eq!(even.re, 1.0);
            assert_eq!(odd.re, -2.0);
            assert_eq!(odd.re * cos_theta[0], -1.0);
            let reference = multiply_complex_scaled(even, current[0], exponents[0])
                + multiply_complex_scaled_product(odd, current[0], cos_theta[0], exponents[0]);
            assert_eq!(reference, Complex64::new(0.0, 0.0));
        }
    }

    #[test]
    fn paired_synthesis_transition_rejects_prefix_underflow_revived_by_spin_weight() {
        let m = 102;
        let lmax = 511;
        let sin_theta = [0.01];
        let cos_theta = [(1.0_f64 - sin_theta[0] * sin_theta[0]).sqrt()];
        let sectoral_norm = sectoral_normalizations(m).unwrap()[m];
        let mut epsilon = vec![0.0; lmax + 4];
        fill_epsilon(m, lmax, &mut epsilon);
        let factor_count = lmax / 2 + 2;
        let mut alpha = vec![0.0; factor_count];
        let mut coefficient_a = vec![0.0; factor_count];
        let mut coefficient_b = vec![0.0; factor_count];
        super::super::scalar_paired::fill_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        );
        let mut first_coefficients = vec![Complex64::new(0.0, 0.0); lmax - m + 1];
        let second_coefficients = vec![Complex64::new(0.0, 0.0); lmax - m + 1];
        first_coefficients[0] = Complex64::new(2.0 * SMALL, 0.0);

        for arch in [Arch::Scalar, Arch::new()] {
            let mut control_previous = [0.0];
            let mut control_current = [0.0];
            let mut control_next = [0.0];
            let mut control_exponents = [0];
            let mut control_first_sum_re = [0.0];
            let mut control_first_sum_im = [0.0];
            let mut control_second_sum_re = [0.0];
            let mut control_second_sum_im = [0.0];
            assert!(
                super::super::scalar_paired::try_synthesize_order_pair_transition_dispatched(
                    arch,
                    m,
                    lmax,
                    &sin_theta,
                    &cos_theta,
                    sectoral_norm,
                    &epsilon,
                    &alpha,
                    &coefficient_a,
                    &coefficient_b,
                    &second_coefficients,
                    &second_coefficients,
                    &mut control_previous,
                    &mut control_current,
                    &mut control_next,
                    &mut control_exponents,
                    &mut control_first_sum_re,
                    &mut control_first_sum_im,
                    &mut control_second_sum_re,
                    &mut control_second_sum_im,
                )
                .unwrap(),
                "the same recurrence must transition when its prefix is zero"
            );

            let mut previous = [0.0];
            let mut current = [0.0];
            let mut next = [0.0];
            let mut exponents = [0];
            let mut first_sum_re = [0.0];
            let mut first_sum_im = [0.0];
            let mut second_sum_re = [0.0];
            let mut second_sum_im = [0.0];
            assert!(
                !super::super::scalar_paired::try_synthesize_order_pair_transition_dispatched(
                    arch,
                    m,
                    lmax,
                    &sin_theta,
                    &cos_theta,
                    sectoral_norm,
                    &epsilon,
                    &alpha,
                    &coefficient_a,
                    &coefficient_b,
                    &first_coefficients,
                    &second_coefficients,
                    &mut previous,
                    &mut current,
                    &mut next,
                    &mut exponents,
                    &mut first_sum_re,
                    &mut first_sum_im,
                    &mut second_sum_re,
                    &mut second_sum_im,
                )
                .unwrap()
            );

            let first_even = first_coefficients[0] * epsilon[m + 1] * alpha[0];
            let inverse_sine_squared = sin_theta[0].recip().powi(2);
            let (sectoral_mantissa, sectoral_exponent) =
                sectoral_value(sin_theta[0], m, sectoral_norm).unwrap();
            assert_eq!(
                multiply_complex_scaled(first_even, sectoral_mantissa, sectoral_exponent).re,
                0.0,
                "the unweighted term rounds away before late spin scaling"
            );
            let revived = multiply_complex_scaled_product(
                first_even,
                sectoral_mantissa,
                inverse_sine_squared,
                sectoral_exponent,
            );
            assert!(revived.re.is_subnormal() && revived.re != 0.0);
            let (sectoral_mantissa, sectoral_exponent) =
                sectoral_value(sin_theta[0], m, sectoral_norm).unwrap();
            let expected = multiply_complex_scaled_product(
                first_even,
                sectoral_mantissa,
                inverse_sine_squared,
                sectoral_exponent,
            );
            assert!(expected.re.is_subnormal());
            assert_ne!(expected.re, 0.0);

            // The transition declines before writing spectra, so the caller's
            // established scaled callback can safely rerun this order.
            let fallback = multiply_complex_scaled_product(
                first_even,
                sectoral_mantissa,
                inverse_sine_squared,
                sectoral_exponent,
            );
            assert_eq!(fallback, expected);
        }
    }

    #[test]
    fn polar_order_cutoff_matches_ducc_bounds() {
        assert_eq!(crate::grid::polar_order_limit(64, 0, 0.0_f64, 1.0), 64);
        assert_eq!(crate::grid::polar_order_limit(255, 0, 0.0_f64, 1.0), 100);
        assert_eq!(crate::grid::polar_order_limit(255, 0, 1.0_f64, 0.0), 255);
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
