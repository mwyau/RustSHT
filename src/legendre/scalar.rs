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

use num_complex::{Complex32, Complex64};
use pulp::{Arch, Simd, WithSimd};

use super::scalar_paired::PairedScalarRecurrencePrecision;

use crate::error::ShtError;
use crate::legendre::{
    LegendrePrecision, ScaledValue as BinaryScaledValue, define_f64_scaled_arithmetic,
    define_scale_binary, define_scale_complex_normal_binary, scalar_recurrence_step,
};

type ScaledValue = BinaryScaledValue<f64>;

pub(super) const BIG: f64 = <f64 as LegendrePrecision>::BIG;
pub(super) const SMALL: f64 = <f64 as LegendrePrecision>::SMALL;
const SCALE_UP: f64 = <f64 as LegendrePrecision>::SCALE_UP;
const SCALE_DOWN: f64 = <f64 as LegendrePrecision>::SCALE_DOWN;
const SCALE_EXPONENT: i64 = <f64 as LegendrePrecision>::SCALE_EXPONENT;
pub(crate) const LATITUDE_BLOCK_SIZE: usize = 64;

// Native-f32 policy and arithmetic leaves used by the shared scalar traversals.
type ScaledValueF32 = BinaryScaledValue<f32>;

const BIG_F32: f32 = <f32 as LegendrePrecision>::BIG;
const SMALL_F32: f32 = <f32 as LegendrePrecision>::SMALL;
#[cfg(test)]
const SCALE_UP_F32: f32 = <f32 as LegendrePrecision>::SCALE_UP;
#[cfg(test)]
const SCALE_DOWN_F32: f32 = <f32 as LegendrePrecision>::SCALE_DOWN;
const NORMAL_RANGE_SKIP_LMAX_T1023: usize = 1023;
const NORMAL_RANGE_SKIP_LMAX_T1279: usize = 1279;

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

/// Precision leaves required by the shared scalar recurrence traversal.
///
/// Initialization, SIMD arithmetic and normalization stay typed so the f64
/// seed path and native-f32 range policy remain unchanged.
pub(crate) trait ScalarRecurrencePrecision: LegendrePrecision {
    fn allow_normal_range_skip(lmax: usize) -> bool;

    fn initialize_recurrence(
        m: usize,
        sin_theta: &[Self],
        sectoral_norm: Self,
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<(), ShtError>;

    fn step<S: Simd>(
        simd: S,
        epsilon: &[Self],
        degree: usize,
        current: &[Self],
        previous: &[Self],
        cosine: &[Self],
        next: &mut [Self],
    );

    fn normalize_pair(
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<(), ShtError>;

    fn normalize_pair_with_state(
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<bool, ShtError>;

    fn needs_scaling<S: Simd>(simd: S, previous: &[Self], current: &[Self]) -> bool;
}

impl ScalarRecurrencePrecision for f64 {
    #[inline(always)]
    fn allow_normal_range_skip(_: usize) -> bool {
        true
    }

    #[inline(always)]
    fn initialize_recurrence(
        m: usize,
        sin_theta: &[Self],
        sectoral_norm: Self,
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<(), ShtError> {
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
        Ok(())
    }

    #[inline(always)]
    fn step<S: Simd>(
        simd: S,
        epsilon: &[Self],
        degree: usize,
        current: &[Self],
        previous: &[Self],
        cosine: &[Self],
        next: &mut [Self],
    ) {
        scalar_recurrence_step!(
            simd,
            S,
            epsilon,
            degree,
            current,
            previous,
            cosine,
            next,
            splat_f64s,
            as_simd_f64s,
            as_mut_simd_f64s,
            mul_f64s,
            sub_f64s,
            div_f64s
        );
    }

    #[inline(always)]
    fn normalize_pair(
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<(), ShtError> {
        normalize_pair_lanes(previous, current, exponents).map(|_| ())
    }

    #[inline(always)]
    fn normalize_pair_with_state(
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<bool, ShtError> {
        normalize_pair_lanes(previous, current, exponents)
    }

    #[inline(always)]
    fn needs_scaling<S: Simd>(simd: S, previous: &[Self], current: &[Self]) -> bool {
        pair_lanes_need_scaling(simd, previous, current)
    }
}

impl ScalarRecurrencePrecision for f32 {
    #[inline(always)]
    fn allow_normal_range_skip(lmax: usize) -> bool {
        matches!(
            lmax,
            NORMAL_RANGE_SKIP_LMAX_T1023 | NORMAL_RANGE_SKIP_LMAX_T1279
        )
    }

    #[inline(always)]
    fn initialize_recurrence(
        m: usize,
        sin_theta: &[Self],
        sectoral_norm: Self,
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<(), ShtError> {
        super::numeric::initialize_scalar_recurrence_lanes(
            m,
            sin_theta,
            sectoral_norm,
            super::numeric::ScalarRecurrenceBuffers {
                previous,
                current,
                exponents,
            },
            scaled_power_f32,
            normalize_value_f32,
        )
    }

    #[inline(always)]
    fn step<S: Simd>(
        simd: S,
        epsilon: &[Self],
        degree: usize,
        current: &[Self],
        previous: &[Self],
        cosine: &[Self],
        next: &mut [Self],
    ) {
        scalar_recurrence_step!(
            simd,
            S,
            epsilon,
            degree,
            current,
            previous,
            cosine,
            next,
            splat_f32s,
            as_simd_f32s,
            as_mut_simd_f32s,
            mul_f32s,
            sub_f32s,
            div_f32s
        );
    }

    #[inline(always)]
    fn normalize_pair(
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<(), ShtError> {
        normalize_pair_lanes_f32(previous, current, exponents)
    }

    #[inline(always)]
    fn normalize_pair_with_state(
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<bool, ShtError> {
        normalize_pair_lanes_with_state_f32(previous, current, exponents)
    }

    #[inline(always)]
    fn needs_scaling<S: Simd>(simd: S, previous: &[Self], current: &[Self]) -> bool {
        pair_lanes_need_scaling_f32(simd, previous, current)
    }
}

impl PairedScalarRecurrencePrecision for f32 {
    #[inline(always)]
    fn initialize_pair_state(
        m: usize,
        sin_theta: &[Self],
        sectoral_norm: Self,
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<(), ShtError> {
        super::numeric::initialize_scalar_recurrence_lanes(
            m,
            sin_theta,
            sectoral_norm,
            super::numeric::ScalarRecurrenceBuffers {
                previous,
                current,
                exponents,
            },
            scaled_power_f32,
            normalize_value_f32,
        )
    }

    #[inline(always)]
    fn step_pair<S: Simd>(
        simd: S,
        coefficient_a: Self,
        coefficient_b: Self,
        cosine: &[Self],
        previous: &[Self],
        current: &[Self],
        next: &mut [Self],
    ) {
        let (current_vectors, current_tail) = S::as_simd_f32s(current);
        let (previous_vectors, previous_tail) = S::as_simd_f32s(previous);
        let (cos_vectors, cos_tail) = S::as_simd_f32s(cosine);
        let (next_vectors, next_tail) = S::as_mut_simd_f32s(next);
        let a = simd.splat_f32s(coefficient_a);
        let b = simd.splat_f32s(coefficient_b);
        for index in 0..current_vectors.len() {
            let cos_squared = simd.mul_f32s(cos_vectors[index], cos_vectors[index]);
            let leading = simd.mul_add_f32s(a, cos_squared, b);
            next_vectors[index] =
                simd.mul_add_f32s(leading, current_vectors[index], previous_vectors[index]);
        }
        for index in 0..current_tail.len() {
            let cos_squared = cos_tail[index] * cos_tail[index];
            let leading = coefficient_a.mul_add(cos_squared, coefficient_b);
            next_tail[index] = leading.mul_add(current_tail[index], previous_tail[index]);
        }
    }

    #[inline(always)]
    fn normalize_pair_state(
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<bool, ShtError> {
        normalize_pair_lanes_with_state_f32(previous, current, exponents)
    }

    #[inline(always)]
    fn pair_state_needs_scaling<S: Simd>(simd: S, previous: &[Self], current: &[Self]) -> bool {
        pair_lanes_need_scaling_f32(simd, previous, current)
    }
}

/// Visits one order over latitude lanes using one shared degree traversal and
/// precision-specific typed initialization, SIMD and normalization leaves.
#[expect(
    clippy::too_many_arguments,
    reason = "The kernel receives geometry, recurrence factors and caller-owned scratch explicitly."
)]
pub(crate) fn for_each_y_lm_dispatched<T, F>(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[T],
    cos_theta: &[T],
    sectoral_norm: T,
    epsilon: &[T],
    previous: &mut [T],
    current: &mut [T],
    next: &mut [T],
    exponents: &mut [i64],
    visit: F,
) -> Result<(), ShtError>
where
    T: ScalarRecurrencePrecision,
    F: FnMut(usize, usize, &[T], &[i64]) -> Result<(), ShtError>,
{
    if T::allow_normal_range_skip(lmax) {
        for_each_y_lm_dispatched_impl::<T, F, true>(
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
        for_each_y_lm_dispatched_impl::<T, F, false>(
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
    reason = "The recurrence receives geometry, factors and caller-owned lane scratch explicitly."
)]
fn for_each_y_lm_dispatched_impl<T, F, const ALLOW_NORMAL_RANGE_SKIP: bool>(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[T],
    cos_theta: &[T],
    sectoral_norm: T,
    epsilon: &[T],
    previous: &mut [T],
    current: &mut [T],
    next: &mut [T],
    exponents: &mut [i64],
    mut visit: F,
) -> Result<(), ShtError>
where
    T: ScalarRecurrencePrecision,
    F: FnMut(usize, usize, &[T], &[i64]) -> Result<(), ShtError>,
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
        arch.dispatch(ForEachYlm::<T, _, ALLOW_NORMAL_RANGE_SKIP> {
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

struct ForEachYlm<'a, T, F, const ALLOW_NORMAL_RANGE_SKIP: bool> {
    ring_start: usize,
    m: usize,
    lmax: usize,
    sin_theta: &'a [T],
    cos_theta: &'a [T],
    sectoral_norm: T,
    epsilon: &'a [T],
    previous: &'a mut [T],
    current: &'a mut [T],
    next: &'a mut [T],
    exponents: &'a mut [i64],
    visit: F,
}

impl<T, F, const ALLOW_NORMAL_RANGE_SKIP: bool> WithSimd
    for ForEachYlm<'_, T, F, ALLOW_NORMAL_RANGE_SKIP>
where
    T: ScalarRecurrencePrecision,
    F: FnMut(usize, usize, &[T], &[i64]) -> Result<(), ShtError>,
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
        T::initialize_recurrence(m, sin_theta, sectoral_norm, previous, current, exponents)?;

        for degree in m..=lmax {
            visit(degree, ring_start, current, exponents)?;
            if degree == lmax {
                break;
            }

            T::step(simd, epsilon, degree, current, previous, cos_theta, next);

            if ALLOW_NORMAL_RANGE_SKIP {
                if normal_range {
                    if T::needs_scaling(simd, current, next) {
                        normal_range = T::normalize_pair_with_state(current, next, exponents)?;
                    }
                } else {
                    normal_range = T::normalize_pair_with_state(current, next, exponents)?;
                }
            } else {
                T::normalize_pair(current, next, exponents)?;
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

define_scale_complex_normal_binary!(
    scale_complex_normal_binary,
    Complex64,
    f64,
    u64,
    52,
    0x7ff,
    4096,
);

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
define_f64_scaled_arithmetic!(
    scaled_power,
    multiply,
    normalize_value,
    "scalar recurrence scale",
);

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

pub(crate) fn multiply_complex_scaled_f32(
    value: Complex32,
    factor: f32,
    exponent: i64,
) -> Complex32 {
    if factor.is_normal() {
        if value.re.is_normal() && value.im.is_normal() {
            let product = Complex32::new(value.re * factor, value.im * factor);
            if product.re.is_normal()
                && product.im.is_normal()
                && let Some(scaled) = scale_complex_normal_binary_f32(product, exponent)
            {
                return scaled;
            }
        }
        Complex32::new(
            multiply_component_scaled_with_normal_factor_f32(value.re, factor, exponent),
            multiply_component_scaled_with_normal_factor_f32(value.im, factor, exponent),
        )
    } else {
        Complex32::new(
            multiply_component_scaled_f32(value.re, factor, exponent),
            multiply_component_scaled_f32(value.im, factor, exponent),
        )
    }
}

define_scale_complex_normal_binary!(
    scale_complex_normal_binary_f32,
    Complex32,
    f32,
    u32,
    23,
    0xff,
    512,
);

fn multiply_component_scaled_with_normal_factor_f32(value: f32, factor: f32, exponent: i64) -> f32 {
    if value.is_normal() {
        let product = value * factor;
        if product.is_normal()
            && let Some(scaled) = scale_normal_binary_f32(product, exponent)
        {
            return scaled;
        }
    }
    if value == 0.0 || !value.is_finite() {
        return super::numeric::scale_binary_f32(value * factor, exponent);
    }
    let (value_mantissa, value_exponent) = split_binary_f32(value);
    let (factor_mantissa, factor_exponent) = split_binary_f32(factor);
    super::numeric::scale_binary_f32(
        value_mantissa * factor_mantissa,
        exponent
            .saturating_add(value_exponent)
            .saturating_add(factor_exponent),
    )
}

fn multiply_component_scaled_f32(value: f32, factor: f32, exponent: i64) -> f32 {
    if value.is_normal() && factor.is_normal() {
        let product = value * factor;
        if product.is_normal()
            && let Some(scaled) = scale_normal_binary_f32(product, exponent)
        {
            return scaled;
        }
    }
    if value == 0.0 || factor == 0.0 || !value.is_finite() || !factor.is_finite() {
        return super::numeric::scale_binary_f32(value * factor, exponent);
    }
    let (value_mantissa, value_exponent) = split_binary_f32(value);
    let (factor_mantissa, factor_exponent) = split_binary_f32(factor);
    super::numeric::scale_binary_f32(
        value_mantissa * factor_mantissa,
        exponent
            .saturating_add(value_exponent)
            .saturating_add(factor_exponent),
    )
}

fn split_binary_f32(value: f32) -> (f32, i64) {
    let bits = value.to_bits();
    let sign = bits & (1_u32 << 31);
    let raw_exponent = ((bits >> 23) & 0xff) as i64;
    let fraction = bits & ((1_u32 << 23) - 1);
    if raw_exponent == 0 {
        let scaled = value * 2.0_f32.powi(24);
        let (mantissa, exponent) = split_normal_f32(scaled);
        (mantissa, exponent - 24)
    } else {
        let mantissa = f32::from_bits(sign | (126_u32 << 23) | fraction);
        (mantissa, raw_exponent - 126)
    }
}

fn split_normal_f32(value: f32) -> (f32, i64) {
    let bits = value.to_bits();
    let sign = bits & (1_u32 << 31);
    let raw_exponent = ((bits >> 23) & 0xff) as i64;
    let fraction = bits & ((1_u32 << 23) - 1);
    (
        f32::from_bits(sign | (126_u32 << 23) | fraction),
        raw_exponent - 126,
    )
}

#[inline(always)]
fn scaled_power_f32(base: f32, exponent: usize) -> Result<ScaledValueF32, ShtError> {
    super::numeric::scaled_power(base, exponent, "scalar recurrence scale")
}

#[inline(always)]
fn normalize_value_f32(value: f32, exponent: i64) -> Result<ScaledValueF32, ShtError> {
    super::numeric::normalize_scaled_value(value, exponent, "scalar recurrence scale")
}

#[inline(always)]
fn normalize_pair_lanes_f32(
    previous: &mut [f32],
    current: &mut [f32],
    exponents: &mut [i64],
) -> Result<(), ShtError> {
    super::numeric::normalize_pair_lanes_f32::<false>(
        previous,
        current,
        exponents,
        "scalar recurrence scale",
    )
    .map(|_| ())
}

#[inline(always)]
fn normalize_pair_lanes_with_state_f32(
    previous: &mut [f32],
    current: &mut [f32],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    let mut normal_range = true;
    for lane in 0..previous.len() {
        let (maximum_bits, _) = super::numeric::normalize_pair_lane_f32(
            &mut previous[lane],
            &mut current[lane],
            &mut exponents[lane],
            "scalar recurrence scale",
        )?;
        normal_range &= exponents[lane] == 0
            && maximum_bits <= BIG_F32.to_bits()
            && (maximum_bits == 0 || maximum_bits >= SMALL_F32.to_bits());
    }
    Ok(normal_range)
}

#[inline(always)]
#[cfg(test)]
fn pair_is_in_normal_range_f32(previous: f32, current: f32) -> bool {
    let maximum = previous.abs().max(current.abs());
    maximum <= BIG_F32 && (maximum == 0.0 || maximum >= SMALL_F32)
}

#[inline(always)]
fn pair_lanes_need_scaling_f32<S: Simd>(simd: S, previous: &[f32], current: &[f32]) -> bool {
    super::numeric::f32_pair_lanes_need_scaling::<S, true>(simd, previous, current)
}

fn scale_normal_binary_f32(value: f32, exponent: i64) -> Option<f32> {
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
mod tests;
