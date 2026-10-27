// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * This file adapts the normalized spin recurrence in DUCC's
 * sht_inner_loop.h to caller-owned latitude lanes and binary-scaled f32/f64
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
#[cfg(all(test, rustsht_profile_spin32_normalizer))]
use std::sync::atomic::{AtomicU64, Ordering};

use num_complex::Complex;
#[cfg(test)]
use num_complex::Complex64;
use pulp::{Arch, Simd, WithSimd};

use crate::error::ShtError;
use crate::legendre::{LegendrePrecision, define_f64_scaled_arithmetic};

const BIG: f64 = <f64 as LegendrePrecision>::BIG;
const SMALL: f64 = <f64 as LegendrePrecision>::SMALL;
const SCALE_UP: f64 = <f64 as LegendrePrecision>::SCALE_UP;
const SCALE_DOWN: f64 = <f64 as LegendrePrecision>::SCALE_DOWN;
const SCALE_EXPONENT: i64 = <f64 as LegendrePrecision>::SCALE_EXPONENT;

#[cfg(all(test, rustsht_profile_spin32_normalizer))]
static NORMALIZER_SCALED_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(all(test, rustsht_profile_spin32_normalizer))]
static NORMALIZER_SCALED_LANES: AtomicU64 = AtomicU64::new(0);
#[cfg(all(test, rustsht_profile_spin32_normalizer))]
static NORMALIZER_SCALE_STEPS: AtomicU64 = AtomicU64::new(0);
#[cfg(all(test, rustsht_profile_spin32_normalizer))]
static NORMALIZER_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(all(test, rustsht_profile_spin32_normalizer))]
static NORMALIZER_LANES: AtomicU64 = AtomicU64::new(0);

#[cfg(all(test, rustsht_profile_spin32_normalizer))]
pub(crate) fn reset_normalizer_profile() {
    NORMALIZER_SCALED_CALLS.store(0, Ordering::Relaxed);
    NORMALIZER_SCALED_LANES.store(0, Ordering::Relaxed);
    NORMALIZER_SCALE_STEPS.store(0, Ordering::Relaxed);
    NORMALIZER_CALLS.store(0, Ordering::Relaxed);
    NORMALIZER_LANES.store(0, Ordering::Relaxed);
}

#[cfg(all(test, rustsht_profile_spin32_normalizer))]
pub(crate) fn normalizer_profile() -> (u64, u64, u64, u64, u64) {
    (
        NORMALIZER_SCALED_CALLS.load(Ordering::Relaxed),
        NORMALIZER_SCALED_LANES.load(Ordering::Relaxed),
        NORMALIZER_SCALE_STEPS.load(Ordering::Relaxed),
        NORMALIZER_CALLS.load(Ordering::Relaxed),
        NORMALIZER_LANES.load(Ordering::Relaxed),
    )
}

pub(crate) type SpinInitial = super::spin_factors::SpinInitial<f64>;

/// Precision leaves for the shared native-spin recurrence traversal.
pub(crate) trait SpinRecurrencePrecision: LegendrePrecision {
    fn initial_values(
        cosine: Self,
        initial: &super::spin_factors::SpinInitial<Self>,
    ) -> Result<
        (
            crate::legendre::ScaledValue<Self>,
            crate::legendre::ScaledValue<Self>,
        ),
        ShtError,
    >;

    #[expect(
        clippy::too_many_arguments,
        reason = "The SIMD step receives independent spin coefficients and caller-owned lane state."
    )]
    fn recurrence_step<S: Simd>(
        simd: S,
        cosine: &[Self],
        coefficient_a: Self,
        coefficient_b: Self,
        current: &[Self],
        previous: &[Self],
        next: &mut [Self],
        add_coupling: bool,
    );

    fn normalize_pair<S: Simd>(
        simd: S,
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<(), ShtError>;
}

/// Visits the two normalized f64 spin recurrence sequences over latitude lanes.
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
    visit: impl FnMut(usize, usize, &[f64], &[i64], &[f64], &[i64]) -> Result<(), ShtError>,
) -> Result<(), ShtError> {
    for_each_spin_lm_dispatched_typed::<f64, _>(
        arch,
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
        visit,
    )
}

/// Shares latitude scheduling, degree traversal and recurrence-state rotation.
#[expect(
    clippy::too_many_arguments,
    reason = "The recurrence receives its plan factors and caller-owned lane scratch explicitly."
)]
pub(crate) fn for_each_spin_lm_dispatched_typed<T, F>(
    arch: Arch,
    initial: &super::spin_factors::SpinInitial<T>,
    lmax: usize,
    sin_theta: &[T],
    cos_theta: &[T],
    coefficient_a: &[T],
    coefficient_b: &[T],
    previous_plus: &mut [T],
    current_plus: &mut [T],
    next_plus: &mut [T],
    exponent_plus: &mut [i64],
    previous_minus: &mut [T],
    current_minus: &mut [T],
    next_minus: &mut [T],
    exponent_minus: &mut [i64],
    mut visit: F,
) -> Result<(), ShtError>
where
    T: SpinRecurrencePrecision,
    F: FnMut(usize, usize, &[T], &[i64], &[T], &[i64]) -> Result<(), ShtError>,
{
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

struct ForEachSpin<'a, T, F> {
    ring_start: usize,
    initial: &'a super::spin_factors::SpinInitial<T>,
    lmax: usize,
    sin_theta: &'a [T],
    cos_theta: &'a [T],
    coefficient_a: &'a [T],
    coefficient_b: &'a [T],
    previous_plus: &'a mut [T],
    current_plus: &'a mut [T],
    next_plus: &'a mut [T],
    exponent_plus: &'a mut [i64],
    previous_minus: &'a mut [T],
    current_minus: &'a mut [T],
    next_minus: &'a mut [T],
    exponent_minus: &'a mut [i64],
    visit: F,
}

impl<T, F> WithSimd for ForEachSpin<'_, T, F>
where
    T: SpinRecurrencePrecision,
    F: FnMut(usize, usize, &[T], &[i64], &[T], &[i64]) -> Result<(), ShtError>,
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
            let (plus, minus) = T::initial_values(cos_theta[ring], initial)?;
            previous_plus[ring] = T::zero();
            current_plus[ring] = initial.plus_sign * plus.mantissa;
            exponent_plus[ring] = plus.exponent;
            previous_minus[ring] = T::zero();
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

            T::recurrence_step(
                simd,
                cos_theta,
                coefficient_a[l + 1],
                coefficient_b[l + 1],
                current_plus,
                previous_plus,
                next_plus,
                false,
            );
            T::recurrence_step(
                simd,
                cos_theta,
                coefficient_a[l + 1],
                coefficient_b[l + 1],
                current_minus,
                previous_minus,
                next_minus,
                true,
            );
            T::normalize_pair(simd, current_plus, next_plus, exponent_plus)?;
            T::normalize_pair(simd, current_minus, next_minus, exponent_minus)?;
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
    reason = "The f32 SIMD step receives lane vectors and independent spin coefficients explicitly."
)]
#[inline(always)]
fn recurrence_step_f32<S: Simd>(
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

#[expect(
    clippy::too_many_arguments,
    reason = "The SIMD recurrence step receives vector lanes and independent positive/negative spin coefficients."
)]
fn recurrence_step_f64<S: Simd>(
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

fn normalize_pair_lanes_f32(
    previous: &mut [f32],
    current: &mut [f32],
    exponents: &mut [i64],
) -> Result<(), ShtError> {
    #[cfg(all(test, rustsht_profile_spin32_normalizer))]
    {
        NORMALIZER_CALLS.fetch_add(1, Ordering::Relaxed);
        NORMALIZER_LANES.fetch_add(previous.len() as u64, Ordering::Relaxed);
        let (scaled_lanes, scale_steps) = super::numeric::normalize_pair_lanes_f32::<true>(
            previous,
            current,
            exponents,
            "spin recurrence exponent",
        )?;
        if scaled_lanes != 0 {
            NORMALIZER_SCALED_CALLS.fetch_add(1, Ordering::Relaxed);
            NORMALIZER_SCALED_LANES.fetch_add(scaled_lanes as u64, Ordering::Relaxed);
            NORMALIZER_SCALE_STEPS.fetch_add(scale_steps as u64, Ordering::Relaxed);
        }
    }
    #[cfg(not(all(test, rustsht_profile_spin32_normalizer)))]
    {
        super::numeric::normalize_pair_lanes_f32::<false>(
            previous,
            current,
            exponents,
            "spin recurrence exponent",
        )?;
    }
    Ok(())
}

/// Skip the scalar lane walk when a Pulp scan proves that every f32 recurrence
/// pair is already within the established ordinary-range invariant.
#[inline(always)]
fn normalize_pair_lanes_f32_prechecked<S: Simd>(
    simd: S,
    previous: &mut [f32],
    current: &mut [f32],
    exponents: &mut [i64],
) -> Result<(), ShtError> {
    if pair_recurrence_state_f32_in_safe_range(simd, previous, current) {
        debug_assert_eq!(previous.len(), exponents.len());
        return Ok(());
    }
    normalize_pair_lanes_f32(previous, current, exponents)
}

#[inline(always)]
fn pair_recurrence_state_f32_in_safe_range<S: Simd>(
    simd: S,
    previous: &[f32],
    current: &[f32],
) -> bool {
    if previous.len() != current.len() {
        return false;
    }
    !super::numeric::f32_pair_lanes_need_scaling::<S, false>(simd, previous, current)
}

impl SpinRecurrencePrecision for f64 {
    #[inline(always)]
    fn initial_values(
        cosine: Self,
        initial: &super::spin_factors::SpinInitial<Self>,
    ) -> Result<
        (
            crate::legendre::ScaledValue<Self>,
            crate::legendre::ScaledValue<Self>,
        ),
        ShtError,
    > {
        super::spin_factors::initial_spin_values(cosine, initial, power, multiply)
    }

    #[inline(always)]
    fn recurrence_step<S: Simd>(
        simd: S,
        cosine: &[Self],
        coefficient_a: Self,
        coefficient_b: Self,
        current: &[Self],
        previous: &[Self],
        next: &mut [Self],
        add_coupling: bool,
    ) {
        recurrence_step_f64(
            simd,
            cosine,
            coefficient_a,
            coefficient_b,
            current,
            previous,
            next,
            add_coupling,
        );
    }

    #[inline(always)]
    fn normalize_pair<S: Simd>(
        _simd: S,
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<(), ShtError> {
        normalize_pair_lanes(previous, current, exponents)
    }
}

impl SpinRecurrencePrecision for f32 {
    #[inline(always)]
    fn initial_values(
        cosine: Self,
        initial: &super::spin_factors::SpinInitial<Self>,
    ) -> Result<
        (
            crate::legendre::ScaledValue<Self>,
            crate::legendre::ScaledValue<Self>,
        ),
        ShtError,
    > {
        super::spin_factors::initial_spin_values(cosine, initial, power_f32, multiply_f32)
    }

    #[inline(always)]
    fn recurrence_step<S: Simd>(
        simd: S,
        cosine: &[Self],
        coefficient_a: Self,
        coefficient_b: Self,
        current: &[Self],
        previous: &[Self],
        next: &mut [Self],
        add_coupling: bool,
    ) {
        recurrence_step_f32(
            simd,
            cosine,
            coefficient_a,
            coefficient_b,
            current,
            previous,
            next,
            add_coupling,
        );
    }

    #[inline(always)]
    fn normalize_pair<S: Simd>(
        simd: S,
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<(), ShtError> {
        normalize_pair_lanes_f32_prechecked(simd, previous, current, exponents)
    }
}

#[inline(always)]
fn power_f32(base: f32, exponent: usize) -> Result<crate::legendre::ScaledValue<f32>, ShtError> {
    super::numeric::scaled_power(base, exponent, "spin recurrence exponent")
}

#[inline(always)]
fn multiply_f32(
    left: crate::legendre::ScaledValue<f32>,
    right: crate::legendre::ScaledValue<f32>,
) -> Result<crate::legendre::ScaledValue<f32>, ShtError> {
    super::numeric::multiply_scaled(left, right, "spin recurrence exponent")
}

/// Typed Pulp operations and exceptional-input policy for spin analysis.
///
/// The accumulation formula, tail handling and reduction order are shared;
/// only Pulp's native f32/f64 vector leaves and established input guards differ.
pub(crate) trait SpinAnalysisPrecision:
    LegendrePrecision + crate::legendre::ShtScalar
{
    type SimdFloat<S: Simd>: Copy;

    fn input_is_vectorizable(value: Self) -> bool;
    fn simd_result_requires_scalar_fallback(result: &(Complex<Self>, Complex<Self>)) -> bool;

    fn as_simd<S: Simd>(values: &[Self]) -> (&[Self::SimdFloat<S>], &[Self]);
    fn splat<S: Simd>(simd: S, value: Self) -> Self::SimdFloat<S>;
    fn simd_add<S: Simd>(
        simd: S,
        left: Self::SimdFloat<S>,
        right: Self::SimdFloat<S>,
    ) -> Self::SimdFloat<S>;
    fn simd_sub<S: Simd>(
        simd: S,
        left: Self::SimdFloat<S>,
        right: Self::SimdFloat<S>,
    ) -> Self::SimdFloat<S>;
    fn simd_mul<S: Simd>(
        simd: S,
        left: Self::SimdFloat<S>,
        right: Self::SimdFloat<S>,
    ) -> Self::SimdFloat<S>;
    fn reduce_sum<S: Simd>(simd: S, value: Self::SimdFloat<S>) -> Self;
}

impl SpinAnalysisPrecision for f64 {
    type SimdFloat<S: Simd> = S::f64s;

    #[inline(always)]
    fn input_is_vectorizable(value: Self) -> bool {
        value.is_normal()
    }

    #[inline(always)]
    fn simd_result_requires_scalar_fallback(_result: &(Complex<Self>, Complex<Self>)) -> bool {
        false
    }

    #[inline(always)]
    fn as_simd<S: Simd>(values: &[Self]) -> (&[Self::SimdFloat<S>], &[Self]) {
        S::as_simd_f64s(values)
    }

    #[inline(always)]
    fn splat<S: Simd>(simd: S, value: Self) -> Self::SimdFloat<S> {
        simd.splat_f64s(value)
    }

    #[inline(always)]
    fn simd_add<S: Simd>(
        simd: S,
        left: Self::SimdFloat<S>,
        right: Self::SimdFloat<S>,
    ) -> Self::SimdFloat<S> {
        simd.add_f64s(left, right)
    }

    #[inline(always)]
    fn simd_sub<S: Simd>(
        simd: S,
        left: Self::SimdFloat<S>,
        right: Self::SimdFloat<S>,
    ) -> Self::SimdFloat<S> {
        simd.sub_f64s(left, right)
    }

    #[inline(always)]
    fn simd_mul<S: Simd>(
        simd: S,
        left: Self::SimdFloat<S>,
        right: Self::SimdFloat<S>,
    ) -> Self::SimdFloat<S> {
        simd.mul_f64s(left, right)
    }

    #[inline(always)]
    fn reduce_sum<S: Simd>(simd: S, value: Self::SimdFloat<S>) -> Self {
        simd.reduce_sum_f64s(value)
    }
}

impl SpinAnalysisPrecision for f32 {
    type SimdFloat<S: Simd> = S::f32s;

    #[inline(always)]
    fn input_is_vectorizable(value: Self) -> bool {
        value == 0.0 || value.is_normal()
    }

    #[inline(always)]
    fn simd_result_requires_scalar_fallback(result: &(Complex<Self>, Complex<Self>)) -> bool {
        ![result.0.re, result.0.im, result.1.re, result.1.im]
            .into_iter()
            .all(f32::is_finite)
    }

    #[inline(always)]
    fn as_simd<S: Simd>(values: &[Self]) -> (&[Self::SimdFloat<S>], &[Self]) {
        S::as_simd_f32s(values)
    }

    #[inline(always)]
    fn splat<S: Simd>(simd: S, value: Self) -> Self::SimdFloat<S> {
        simd.splat_f32s(value)
    }

    #[inline(always)]
    fn simd_add<S: Simd>(
        simd: S,
        left: Self::SimdFloat<S>,
        right: Self::SimdFloat<S>,
    ) -> Self::SimdFloat<S> {
        simd.add_f32s(left, right)
    }

    #[inline(always)]
    fn simd_sub<S: Simd>(
        simd: S,
        left: Self::SimdFloat<S>,
        right: Self::SimdFloat<S>,
    ) -> Self::SimdFloat<S> {
        simd.sub_f32s(left, right)
    }

    #[inline(always)]
    fn simd_mul<S: Simd>(
        simd: S,
        left: Self::SimdFloat<S>,
        right: Self::SimdFloat<S>,
    ) -> Self::SimdFloat<S> {
        simd.mul_f32s(left, right)
    }

    #[inline(always)]
    fn reduce_sum<S: Simd>(simd: S, value: Self::SimdFloat<S>) -> Self {
        simd.reduce_sum_f32s(value)
    }
}

/// Returns whether Fourier inputs can use the shared native-spin SIMD kernel.
pub(crate) fn analysis_inputs_vectorizable<T: SpinAnalysisPrecision>(
    m: usize,
    q_real: &[T],
    q_imag: &[T],
    u_real: &[T],
    u_imag: &[T],
    ring_order_limits: &[usize],
) -> bool {
    debug_assert_eq!(q_real.len(), q_imag.len());
    debug_assert_eq!(q_real.len(), u_real.len());
    debug_assert_eq!(q_real.len(), u_imag.len());
    debug_assert_eq!(q_real.len(), ring_order_limits.len());
    ring_order_limits.iter().all(|&limit| m <= limit)
        && q_real.iter().all(|&value| T::input_is_vectorizable(value))
        && q_imag.iter().all(|&value| T::input_is_vectorizable(value))
        && u_real.iter().all(|&value| T::input_is_vectorizable(value))
        && u_imag.iter().all(|&value| T::input_is_vectorizable(value))
}

/// Accumulates one spin-analysis degree, sharing its E/B formula and tails.
#[expect(
    clippy::too_many_arguments,
    reason = "The accumulation receives Fourier lanes, scaled bases, and polar cutoffs."
)]
pub(crate) fn accumulate_analysis_dispatched<
    T: SpinAnalysisPrecision + crate::legendre::ShtScalar,
>(
    arch: Arch,
    vectorizable_inputs: bool,
    m: usize,
    q_real: &[T],
    q_imag: &[T],
    u_real: &[T],
    u_imag: &[T],
    plus: &[T],
    plus_exponents: &[i64],
    minus: &[T],
    minus_exponents: &[i64],
    alpha_norm: T,
    ring_order_limits: &[usize],
    include_b: bool,
) -> (Complex<T>, Complex<T>) {
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
        && plus.iter().all(|&value| T::input_is_vectorizable(value))
        && minus.iter().all(|&value| T::input_is_vectorizable(value));
    if vectorizable {
        let simd_sums = arch.dispatch(SpinAnalysisAccumulation {
            q_real,
            q_imag,
            u_real,
            u_imag,
            plus,
            minus,
            alpha_norm,
            include_b,
        });
        if !T::simd_result_requires_scalar_fallback(&simd_sums) {
            return simd_sums;
        }
    }

    super::accumulate_spin_analysis_scalar_fallback::<T>(
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

struct SpinAnalysisAccumulation<'a, T> {
    q_real: &'a [T],
    q_imag: &'a [T],
    u_real: &'a [T],
    u_imag: &'a [T],
    plus: &'a [T],
    minus: &'a [T],
    alpha_norm: T,
    include_b: bool,
}

impl<T: SpinAnalysisPrecision> WithSimd for SpinAnalysisAccumulation<'_, T> {
    type Output = (Complex<T>, Complex<T>);

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
        let (q_real_vectors, q_real_tail) = T::as_simd::<S>(q_real);
        let (q_imag_vectors, q_imag_tail) = T::as_simd::<S>(q_imag);
        let (u_real_vectors, u_real_tail) = T::as_simd::<S>(u_real);
        let (u_imag_vectors, u_imag_tail) = T::as_simd::<S>(u_imag);
        let (plus_vectors, plus_tail) = T::as_simd::<S>(plus);
        let (minus_vectors, minus_tail) = T::as_simd::<S>(minus);
        let vector_count = plus_vectors.len();
        debug_assert_eq!(q_real_vectors.len(), vector_count);
        debug_assert_eq!(q_imag_vectors.len(), vector_count);
        debug_assert_eq!(u_real_vectors.len(), vector_count);
        debug_assert_eq!(u_imag_vectors.len(), vector_count);
        debug_assert_eq!(minus_vectors.len(), vector_count);

        let alpha = T::splat(simd, alpha_norm);
        let zero = T::splat(simd, T::zero());
        let mut e_real_sum = zero;
        let mut e_imag_sum = zero;
        let mut b_real_sum = zero;
        let mut b_imag_sum = zero;
        for index in 0..vector_count {
            let plus_value = T::simd_mul(simd, plus_vectors[index], alpha);
            let minus_value = T::simd_mul(simd, minus_vectors[index], alpha);
            let even_basis = T::simd_add(simd, plus_value, minus_value);
            let odd_basis = T::simd_sub(simd, plus_value, minus_value);
            e_real_sum = T::simd_add(
                simd,
                e_real_sum,
                T::simd_add(
                    simd,
                    T::simd_mul(simd, q_real_vectors[index], even_basis),
                    T::simd_mul(simd, u_imag_vectors[index], odd_basis),
                ),
            );
            e_imag_sum = T::simd_add(
                simd,
                e_imag_sum,
                T::simd_sub(
                    simd,
                    T::simd_mul(simd, q_imag_vectors[index], even_basis),
                    T::simd_mul(simd, u_real_vectors[index], odd_basis),
                ),
            );
            if include_b {
                b_real_sum = T::simd_add(
                    simd,
                    b_real_sum,
                    T::simd_sub(
                        simd,
                        T::simd_mul(simd, u_real_vectors[index], even_basis),
                        T::simd_mul(simd, q_imag_vectors[index], odd_basis),
                    ),
                );
                b_imag_sum = T::simd_add(
                    simd,
                    b_imag_sum,
                    T::simd_add(
                        simd,
                        T::simd_mul(simd, u_imag_vectors[index], even_basis),
                        T::simd_mul(simd, q_real_vectors[index], odd_basis),
                    ),
                );
            }
        }

        let mut e_real_tail_sum = T::zero();
        let mut e_imag_tail_sum = T::zero();
        let mut b_real_tail_sum = T::zero();
        let mut b_imag_tail_sum = T::zero();
        for lane in 0..plus_tail.len() {
            let plus_value = plus_tail[lane] * alpha_norm;
            let minus_value = minus_tail[lane] * alpha_norm;
            let even_basis = plus_value + minus_value;
            let odd_basis = plus_value - minus_value;
            e_real_tail_sum =
                e_real_tail_sum + q_real_tail[lane] * even_basis + u_imag_tail[lane] * odd_basis;
            e_imag_tail_sum =
                e_imag_tail_sum + q_imag_tail[lane] * even_basis - u_real_tail[lane] * odd_basis;
            if include_b {
                b_real_tail_sum = b_real_tail_sum + u_real_tail[lane] * even_basis
                    - q_imag_tail[lane] * odd_basis;
                b_imag_tail_sum = b_imag_tail_sum
                    + u_imag_tail[lane] * even_basis
                    + q_real_tail[lane] * odd_basis;
            }
        }

        (
            Complex::new(
                T::reduce_sum(simd, e_real_sum) + e_real_tail_sum,
                T::reduce_sum(simd, e_imag_sum) + e_imag_tail_sum,
            ),
            Complex::new(
                T::reduce_sum(simd, b_real_sum) + b_real_tail_sum,
                T::reduce_sum(simd, b_imag_sum) + b_imag_tail_sum,
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

pub(crate) fn scaled_basis_value_f32(value: f32, exponent: i64, alpha_norm: f32) -> f32 {
    if value == 0.0 || alpha_norm == 0.0 {
        return 0.0;
    }
    super::numeric::scale_binary_f32(value * alpha_norm, exponent)
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
        let initial = crate::legendre::spin_factors::fill_spin_factors::<f64>(
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
                let norm =
                    crate::legendre::spin_factors::spin_normalization::<f64>(spin, degree, false)
                        * alpha[degree];
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

#[cfg(test)]
mod f32_tests {
    use super::*;
    use crate::error::ShtError;
    use crate::legendre::{LATITUDE_BLOCK_SIZE, LegendrePrecision};
    use num_complex::Complex32;
    use pulp::{Simd, WithSimd};
    use std::f32::consts::PI;

    const BIG_F32: f32 = <f32 as LegendrePrecision>::BIG;
    const SMALL_F32: f32 = <f32 as LegendrePrecision>::SMALL;
    const SCALE_UP_F32: f32 = <f32 as LegendrePrecision>::SCALE_UP;
    const SCALE_DOWN_F32: f32 = <f32 as LegendrePrecision>::SCALE_DOWN;

    struct CheckPairStateRange<'a> {
        previous: &'a [f32],
        current: &'a [f32],
    }

    impl WithSimd for CheckPairStateRange<'_> {
        type Output = bool;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            pair_recurrence_state_f32_in_safe_range(simd, self.previous, self.current)
        }
    }

    struct NormalizePairPrechecked<'a> {
        previous: &'a mut [f32],
        current: &'a mut [f32],
        exponents: &'a mut [i64],
    }

    impl WithSimd for NormalizePairPrechecked<'_> {
        type Output = Result<(), ShtError>;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            normalize_pair_lanes_f32_prechecked(simd, self.previous, self.current, self.exponents)
        }
    }

    struct AnalysisLanes32 {
        q_real: Vec<f32>,
        q_imag: Vec<f32>,
        u_real: Vec<f32>,
        u_imag: Vec<f32>,
        plus: Vec<f32>,
        plus_exponents: Vec<i64>,
        minus: Vec<f32>,
        minus_exponents: Vec<i64>,
        limits: Vec<usize>,
    }

    impl AnalysisLanes32 {
        fn new(count: usize) -> Self {
            Self {
                q_real: (0..count).map(|lane| 0.25 + lane as f32 * 0.007).collect(),
                q_imag: (0..count).map(|lane| -0.4 + lane as f32 * 0.003).collect(),
                u_real: (0..count).map(|lane| 0.15 - lane as f32 * 0.002).collect(),
                u_imag: (0..count).map(|lane| 0.55 + lane as f32 * 0.004).collect(),
                plus: (0..count).map(|lane| 0.31 + lane as f32 * 0.001).collect(),
                plus_exponents: vec![0; count],
                minus: (0..count).map(|lane| -0.17 + lane as f32 * 0.002).collect(),
                minus_exponents: vec![0; count],
                limits: vec![64; count],
            }
        }

        fn accumulate(
            &self,
            arch: Arch,
            m: usize,
            alpha_norm: f32,
            include_b: bool,
        ) -> (Complex32, Complex32) {
            let vectorizable_inputs = super::analysis_inputs_vectorizable::<f32>(
                m,
                &self.q_real,
                &self.q_imag,
                &self.u_real,
                &self.u_imag,
                &self.limits,
            );
            super::accumulate_analysis_dispatched::<f32>(
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
            alpha_norm: f32,
            include_b: bool,
        ) -> (Complex32, Complex32) {
            let mut e_sum = Complex32::new(0.0, 0.0);
            let mut b_sum = Complex32::new(0.0, 0.0);
            for lane in 0..self.plus.len() {
                if m > self.limits[lane] {
                    continue;
                }
                let plus_value =
                    scaled_basis_value_f32(self.plus[lane], self.plus_exponents[lane], alpha_norm);
                let minus_value = scaled_basis_value_f32(
                    self.minus[lane],
                    self.minus_exponents[lane],
                    alpha_norm,
                );
                let even_basis = plus_value + minus_value;
                let odd_basis = plus_value - minus_value;
                e_sum.re += self.q_real[lane] * even_basis + self.u_imag[lane] * odd_basis;
                e_sum.im += self.q_imag[lane] * even_basis - self.u_real[lane] * odd_basis;
                if include_b {
                    b_sum.re += self.u_real[lane] * even_basis - self.q_imag[lane] * odd_basis;
                    b_sum.im += self.u_imag[lane] * even_basis + self.q_real[lane] * odd_basis;
                }
            }
            (e_sum, b_sum)
        }
    }

    #[test]
    fn simd_normalizer_precheck_matches_scalar_scaler_for_boundaries_and_tail() {
        let safe_previous = vec![
            0.0, -0.0, 0.25, -0.5, SMALL_F32, -SMALL_F32, BIG_F32, -BIG_F32, 0.0, 1.0, 0.0, 0.25,
            -0.75, 0.0, 2.0, 0.0, 0.5,
        ];
        let safe_current = vec![
            0.0, 0.0, -0.25, 0.5, 0.0, SMALL_F32, 0.0, BIG_F32, -0.0, -1.0, 0.0, -0.25, 0.75, 0.0,
            -2.0, 0.0, -0.5,
        ];
        let mut unsafe_previous = safe_previous.clone();
        let mut unsafe_current = safe_current.clone();
        unsafe_previous[0] = f32::from_bits(1);
        unsafe_previous[1] = f32::MAX;
        unsafe_current[2] = -f32::MAX;
        unsafe_current[8] = f32::from_bits(SMALL_F32.to_bits() - 1);
        unsafe_previous[9] = BIG_F32 * 2.0;
        unsafe_current[16] = f32::from_bits(SMALL_F32.to_bits() - 1);
        let mut nonfinite = safe_current.clone();
        nonfinite[3] = f32::INFINITY;
        nonfinite[11] = f32::NAN;

        for (name, arch) in [("scalar", Arch::Scalar), ("runtime", Arch::new())] {
            assert!(
                arch.dispatch(CheckPairStateRange {
                    previous: &safe_previous,
                    current: &safe_current,
                }),
                "{name} backend rejected safe values"
            );
            assert!(!arch.dispatch(CheckPairStateRange {
                previous: &unsafe_previous,
                current: &unsafe_current,
            }));
            assert!(!arch.dispatch(CheckPairStateRange {
                previous: &safe_previous,
                current: &nonfinite,
            }));

            for (initial_previous, initial_current) in [
                (&safe_previous, &safe_current),
                (&unsafe_previous, &unsafe_current),
            ] {
                let mut expected_previous = initial_previous.clone();
                let mut expected_current = initial_current.clone();
                let mut expected_exponents = vec![19_i64; initial_previous.len()];
                normalize_pair_lanes_f32(
                    &mut expected_previous,
                    &mut expected_current,
                    &mut expected_exponents,
                )
                .unwrap();

                let mut actual_previous = initial_previous.clone();
                let mut actual_current = initial_current.clone();
                let mut actual_exponents = vec![19_i64; initial_previous.len()];
                arch.dispatch(NormalizePairPrechecked {
                    previous: &mut actual_previous,
                    current: &mut actual_current,
                    exponents: &mut actual_exponents,
                })
                .unwrap();
                assert_eq!(
                    actual_previous
                        .iter()
                        .map(|value| value.to_bits())
                        .collect::<Vec<_>>(),
                    expected_previous
                        .iter()
                        .map(|value| value.to_bits())
                        .collect::<Vec<_>>(),
                );
                assert_eq!(
                    actual_current
                        .iter()
                        .map(|value| value.to_bits())
                        .collect::<Vec<_>>(),
                    expected_current
                        .iter()
                        .map(|value| value.to_bits())
                        .collect::<Vec<_>>(),
                );
                assert_eq!(actual_exponents, expected_exponents);
            }
        }
    }

    #[test]
    fn f32_spin_analysis_simd_accumulation_matches_scalar_blocks_and_tails() {
        for count in [64, 61] {
            let lanes = AnalysisLanes32::new(count);
            let expected = lanes.scalar_reference(4, 0.75, true);
            assert!(super::analysis_inputs_vectorizable::<f32>(
                4,
                &lanes.q_real,
                &lanes.q_imag,
                &lanes.u_real,
                &lanes.u_imag,
                &lanes.limits,
            ));
            for arch in [Arch::Scalar, Arch::new()] {
                let actual = lanes.accumulate(arch, 4, 0.75, true);
                assert!((actual.0 - expected.0).norm() < 1.0e-4);
                assert!((actual.1 - expected.1).norm() < 1.0e-4);

                let e_only = lanes.accumulate(arch, 4, 0.75, false);
                assert!((e_only.0 - expected.0).norm() < 1.0e-4);
                assert_eq!(e_only.1, Complex32::new(0.0, 0.0));
            }
        }
    }

    #[test]
    fn f32_spin_analysis_falls_back_when_simd_lane_partials_can_overflow() {
        let mut lanes = AnalysisLanes32::new(16);
        for lane in 0..lanes.q_real.len() {
            lanes.q_real[lane] = if lane.is_multiple_of(2) {
                f32::MAX
            } else {
                -f32::MAX
            };
            lanes.q_imag[lane] = 0.0;
            lanes.u_real[lane] = 0.0;
            lanes.u_imag[lane] = 0.0;
            lanes.plus[lane] = 0.5;
            lanes.minus[lane] = 0.5;
        }

        assert!(super::analysis_inputs_vectorizable::<f32>(
            4,
            &lanes.q_real,
            &lanes.q_imag,
            &lanes.u_real,
            &lanes.u_imag,
            &lanes.limits,
        ));
        let expected = lanes.scalar_reference(4, 1.0, false);
        assert_eq!(expected.0, Complex32::new(0.0, 0.0));
        for arch in [Arch::Scalar, Arch::new()] {
            assert_eq!(lanes.accumulate(arch, 4, 1.0, false), expected);
        }
    }

    #[test]
    fn f32_spin_analysis_zero_inputs_use_simd_and_scaled_lanes_use_scalar_accumulation() {
        let mut lanes = AnalysisLanes32::new(64);
        lanes.plus_exponents[11] = -800;
        lanes.limits[7] = 3;
        let expected = lanes.scalar_reference(4, 0.75, true);
        assert!(!super::analysis_inputs_vectorizable::<f32>(
            4,
            &lanes.q_real,
            &lanes.q_imag,
            &lanes.u_real,
            &lanes.u_imag,
            &lanes.limits,
        ));
        for arch in [Arch::Scalar, Arch::new()] {
            let actual = lanes.accumulate(arch, 4, 0.75, true);
            assert_eq!(actual, expected);
        }

        lanes.limits.fill(64);
        lanes.q_real[4] = 0.0;
        lanes.u_imag[9] = -0.0;
        lanes.plus[12] = 0.0;
        lanes.minus[17] = -0.0;
        assert!(
            lanes
                .plus
                .iter()
                .all(|&value| value == 0.0 || value.is_normal())
        );
        assert!(
            lanes
                .minus
                .iter()
                .all(|&value| value == 0.0 || value.is_normal())
        );
        let zero_expected = lanes.scalar_reference(4, 0.75, true);
        assert!(super::analysis_inputs_vectorizable::<f32>(
            4,
            &lanes.q_real,
            &lanes.q_imag,
            &lanes.u_real,
            &lanes.u_imag,
            &lanes.limits,
        ));
        for arch in [Arch::Scalar, Arch::new()] {
            let actual = lanes.accumulate(arch, 4, 0.75, true);
            assert!((actual.0 - zero_expected.0).norm() < 1.0e-4);
            assert!((actual.1 - zero_expected.1).norm() < 1.0e-4);
        }

        lanes.q_real[4] = f32::from_bits(1);
        lanes.plus[4] = f32::from_bits(1);
        assert!(
            !lanes
                .plus
                .iter()
                .all(|&value| value == 0.0 || value.is_normal())
        );
        assert!(!super::analysis_inputs_vectorizable::<f32>(
            4,
            &lanes.q_real,
            &lanes.q_imag,
            &lanes.u_real,
            &lanes.u_imag,
            &lanes.limits,
        ));
    }

    fn reference_normalize_pair_lanes_f32(
        previous: &mut [f32],
        current: &mut [f32],
        exponents: &mut [i64],
    ) {
        for lane in 0..previous.len() {
            let mut maximum = previous[lane].abs().max(current[lane].abs());
            while maximum > BIG_F32 {
                previous[lane] *= SCALE_DOWN_F32;
                current[lane] *= SCALE_DOWN_F32;
                exponents[lane] += 80;
                maximum *= SCALE_DOWN_F32;
            }
            while maximum != 0.0 && maximum < SMALL_F32 {
                previous[lane] *= SCALE_UP_F32;
                current[lane] *= SCALE_UP_F32;
                exponents[lane] -= 80;
                maximum *= SCALE_UP_F32;
            }
        }
    }

    #[test]
    fn f32_spin_pair_normalization_matches_float_magnitude_reference() {
        let initial_previous = [
            0.0,
            f32::from_bits(1),
            f32::from_bits(SMALL_F32.to_bits() - 1),
            -f32::from_bits(BIG_F32.to_bits() + 1),
            f32::MAX,
            f32::MIN_POSITIVE,
            -0.5,
            0.0,
        ];
        let initial_current = [
            -0.0,
            0.0,
            0.0,
            BIG_F32,
            f32::MIN_POSITIVE,
            -f32::from_bits(1),
            0.25,
            SMALL_F32,
        ];
        let mut actual_previous = initial_previous;
        let mut actual_current = initial_current;
        let mut actual_exponents = [0; 8];
        let mut expected_previous = initial_previous;
        let mut expected_current = initial_current;
        let mut expected_exponents = [0; 8];

        normalize_pair_lanes_f32(
            &mut actual_previous,
            &mut actual_current,
            &mut actual_exponents,
        )
        .unwrap();
        reference_normalize_pair_lanes_f32(
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
        let initial = crate::legendre::spin_factors::fill_spin_factors::<f32>(
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
        for_each_spin_lm_dispatched_typed::<f32, _>(
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
                let factor =
                    crate::legendre::spin_factors::spin_normalization::<f32>(spin, degree, false)
                        * alpha[degree];
                for lane in 0..plus.len() {
                    plus_values[degree - initial.start_degree][ring_start + lane] =
                        scaled_basis_value_f32(plus[lane], plus_exponents[lane], factor);
                    minus_values[degree - initial.start_degree][ring_start + lane] =
                        scaled_basis_value_f32(minus[lane], minus_exponents[lane], factor);
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
