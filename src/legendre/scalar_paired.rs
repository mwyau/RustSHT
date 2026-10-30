// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * This module adapts DUCC's transformed paired-degree scalar recurrence from
 * src/ducc0/sht/sht_inner_loop.h, revision
 * c1ed01b8fda69d2a378190f0915f1a19762dddbf. The analysis prefix cutoff also
 * follows DUCC's iter_to_ieee logic from revision
 * 80388904b8cfc32400ba84af8a6a45aabfe60f86.
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

use num_complex::{Complex, Complex64};
use pulp::{Arch, Simd, WithSimd};
use rustfft::num_traits::Float;
use std::marker::PhantomData;

use crate::error::ShtError;
use crate::legendre::scalar::{
    lane_values_in_normal_range, scalar_value_in_normal_range, simd_values_in_normal_range,
};
use crate::legendre::{
    LATITUDE_BLOCK_SIZE, LegendrePrecision, SCALAR_ANALYSIS_PAIR_TILE_SIZE, scalar,
};

const SIMD_SECTORAL_INIT_MIN_LMAX: usize = 511;
const ANALYSIS_FTOL: f64 = f64::from_bits(963_u64 << 52);
const ANALYSIS_SCALE_STEP: i64 = 800;
const ANALYSIS_SCALE_UP: f64 = f64::from_bits(1823_u64 << 52);
const ANALYSIS_SCALE_DOWN: f64 = f64::from_bits(223_u64 << 52);
const ANALYSIS_FBIGHALF: f64 = f64::from_bits(1423_u64 << 52);
const ANALYSIS_MAX_SCALE_EPOCH: f64 = 4.5e15;
const PAIRED_FACTOR_LIMIT: f64 = f64::from_bits(1151_u64 << 52);
pub(crate) const PAIRED_FACTOR_LIMIT_F32: f32 = f32::from_bits(143_u32 << 23);
#[cfg(feature = "parallel")]
pub(crate) const HEMISPHERE_PAIR_DEGREE_TILE: usize = 8;

#[cfg(test)]
pub(crate) static ANALYSIS_PAIR_GROUP_RANGE_DECLINES: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(all(test, feature = "parallel"))]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ScalarAnalysisRouteCounts {
    pub(crate) normal_seed_blocks: usize,
    pub(crate) prefix_enabled_scaled_blocks: usize,
    pub(crate) checked_scaled_blocks: usize,
    pub(crate) prefix_initialization_declines: usize,
    pub(crate) prefix_recurrence_declines: usize,
    pub(crate) checked_scaled_recurrence_declines: usize,
    pub(crate) typed_ordinary_attempts: usize,
    pub(crate) typed_ordinary_successes: usize,
    pub(crate) typed_seed_declines: usize,
    pub(crate) typed_recurrence_declines: usize,
    pub(crate) typed_normal_pair_accumulations: usize,
    pub(crate) typed_scaled_pair_accumulations: usize,
}

#[cfg(all(test, feature = "parallel"))]
static SCALAR_ANALYSIS_ROUTE_PROFILE_ENABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
#[cfg(all(test, feature = "parallel"))]
static SCALAR_ANALYSIS_NORMAL_SEED_BLOCKS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(test, feature = "parallel"))]
static SCALAR_ANALYSIS_PREFIX_ENABLED_SCALED_BLOCKS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(test, feature = "parallel"))]
static SCALAR_ANALYSIS_CHECKED_SCALED_BLOCKS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(test, feature = "parallel"))]
static SCALAR_ANALYSIS_PREFIX_INITIALIZATION_DECLINES: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(test, feature = "parallel"))]
static SCALAR_ANALYSIS_PREFIX_RECURRENCE_DECLINES: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(test, feature = "parallel"))]
static SCALAR_ANALYSIS_CHECKED_SCALED_RECURRENCE_DECLINES: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(test, feature = "parallel"))]
static SCALAR_ANALYSIS_TYPED_ORDINARY_ATTEMPTS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(test, feature = "parallel"))]
static SCALAR_ANALYSIS_TYPED_ORDINARY_SUCCESSES: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(test, feature = "parallel"))]
static SCALAR_ANALYSIS_TYPED_SEED_DECLINES: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(test, feature = "parallel"))]
static SCALAR_ANALYSIS_TYPED_RECURRENCE_DECLINES: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(test, feature = "parallel"))]
static SCALAR_ANALYSIS_TYPED_NORMAL_PAIR_ACCUMULATIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(test, feature = "parallel"))]
static SCALAR_ANALYSIS_TYPED_SCALED_PAIR_ACCUMULATIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(test, feature = "parallel"))]
pub(crate) fn begin_scalar_analysis_route_profile() {
    use std::sync::atomic::Ordering;

    SCALAR_ANALYSIS_ROUTE_PROFILE_ENABLED.store(false, Ordering::Relaxed);
    SCALAR_ANALYSIS_NORMAL_SEED_BLOCKS.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_PREFIX_ENABLED_SCALED_BLOCKS.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_CHECKED_SCALED_BLOCKS.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_PREFIX_INITIALIZATION_DECLINES.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_PREFIX_RECURRENCE_DECLINES.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_CHECKED_SCALED_RECURRENCE_DECLINES.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_TYPED_ORDINARY_ATTEMPTS.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_TYPED_ORDINARY_SUCCESSES.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_TYPED_SEED_DECLINES.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_TYPED_RECURRENCE_DECLINES.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_TYPED_NORMAL_PAIR_ACCUMULATIONS.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_TYPED_SCALED_PAIR_ACCUMULATIONS.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_ROUTE_PROFILE_ENABLED.store(true, Ordering::Release);
}

#[cfg(all(test, feature = "parallel"))]
pub(crate) fn finish_scalar_analysis_route_profile() -> ScalarAnalysisRouteCounts {
    use std::sync::atomic::Ordering;

    SCALAR_ANALYSIS_ROUTE_PROFILE_ENABLED.store(false, Ordering::Release);
    ScalarAnalysisRouteCounts {
        normal_seed_blocks: SCALAR_ANALYSIS_NORMAL_SEED_BLOCKS.load(Ordering::Relaxed),
        prefix_enabled_scaled_blocks: SCALAR_ANALYSIS_PREFIX_ENABLED_SCALED_BLOCKS
            .load(Ordering::Relaxed),
        checked_scaled_blocks: SCALAR_ANALYSIS_CHECKED_SCALED_BLOCKS.load(Ordering::Relaxed),
        prefix_initialization_declines: SCALAR_ANALYSIS_PREFIX_INITIALIZATION_DECLINES
            .load(Ordering::Relaxed),
        prefix_recurrence_declines: SCALAR_ANALYSIS_PREFIX_RECURRENCE_DECLINES
            .load(Ordering::Relaxed),
        checked_scaled_recurrence_declines: SCALAR_ANALYSIS_CHECKED_SCALED_RECURRENCE_DECLINES
            .load(Ordering::Relaxed),
        typed_ordinary_attempts: SCALAR_ANALYSIS_TYPED_ORDINARY_ATTEMPTS.load(Ordering::Relaxed),
        typed_ordinary_successes: SCALAR_ANALYSIS_TYPED_ORDINARY_SUCCESSES.load(Ordering::Relaxed),
        typed_seed_declines: SCALAR_ANALYSIS_TYPED_SEED_DECLINES.load(Ordering::Relaxed),
        typed_recurrence_declines: SCALAR_ANALYSIS_TYPED_RECURRENCE_DECLINES
            .load(Ordering::Relaxed),
        typed_normal_pair_accumulations: SCALAR_ANALYSIS_TYPED_NORMAL_PAIR_ACCUMULATIONS
            .load(Ordering::Relaxed),
        typed_scaled_pair_accumulations: SCALAR_ANALYSIS_TYPED_SCALED_PAIR_ACCUMULATIONS
            .load(Ordering::Relaxed),
    }
}

#[cfg(all(test, feature = "parallel"))]
#[inline(always)]
pub(crate) fn record_typed_scalar_analysis_pair_accumulation(normal_range: bool) {
    use std::sync::atomic::Ordering;

    if !SCALAR_ANALYSIS_ROUTE_PROFILE_ENABLED.load(Ordering::Acquire) {
        return;
    }
    let counter = if normal_range {
        &SCALAR_ANALYSIS_TYPED_NORMAL_PAIR_ACCUMULATIONS
    } else {
        &SCALAR_ANALYSIS_TYPED_SCALED_PAIR_ACCUMULATIONS
    };
    counter.fetch_add(1, Ordering::Relaxed);
}

#[cfg(all(test, feature = "parallel"))]
pub(crate) fn record_typed_scalar_analysis_block(
    attempt: bool,
    seed_decline: bool,
    recurrence_decline: bool,
    success: bool,
) {
    use std::sync::atomic::Ordering;

    if !SCALAR_ANALYSIS_ROUTE_PROFILE_ENABLED.load(Ordering::Acquire) {
        return;
    }
    if attempt {
        SCALAR_ANALYSIS_TYPED_ORDINARY_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    }
    if seed_decline {
        SCALAR_ANALYSIS_TYPED_SEED_DECLINES.fetch_add(1, Ordering::Relaxed);
    }
    if success {
        SCALAR_ANALYSIS_TYPED_ORDINARY_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    }
    if recurrence_decline {
        SCALAR_ANALYSIS_TYPED_RECURRENCE_DECLINES.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(all(test, feature = "parallel"))]
#[inline(always)]
fn record_scalar_analysis_seed_route(initialized_normal: bool, prefix_allowed: bool) {
    use std::sync::atomic::Ordering;

    if !SCALAR_ANALYSIS_ROUTE_PROFILE_ENABLED.load(Ordering::Acquire) {
        return;
    }
    let counter = if initialized_normal {
        &SCALAR_ANALYSIS_NORMAL_SEED_BLOCKS
    } else if prefix_allowed {
        &SCALAR_ANALYSIS_PREFIX_ENABLED_SCALED_BLOCKS
    } else {
        &SCALAR_ANALYSIS_CHECKED_SCALED_BLOCKS
    };
    counter.fetch_add(1, Ordering::Relaxed);
}

#[cfg(all(test, feature = "parallel"))]
#[inline(always)]
fn record_scalar_analysis_initialization_decline() {
    use std::sync::atomic::Ordering;

    if !SCALAR_ANALYSIS_ROUTE_PROFILE_ENABLED.load(Ordering::Acquire) {
        return;
    }
    SCALAR_ANALYSIS_PREFIX_INITIALIZATION_DECLINES.fetch_add(1, Ordering::Relaxed);
}

#[cfg(all(test, feature = "parallel"))]
#[inline(always)]
fn record_scalar_analysis_recurrence_decline(prefix_allowed: bool) {
    use std::sync::atomic::Ordering;

    if !SCALAR_ANALYSIS_ROUTE_PROFILE_ENABLED.load(Ordering::Acquire) {
        return;
    }
    let counter = if prefix_allowed {
        &SCALAR_ANALYSIS_PREFIX_RECURRENCE_DECLINES
    } else {
        &SCALAR_ANALYSIS_CHECKED_SCALED_RECURRENCE_DECLINES
    };
    counter.fetch_add(1, Ordering::Relaxed);
}

/// DUCC applies `sht_ftol = 2^-60` independently of the number of latitude
/// lanes. Keep the prefix route available for any non-empty geometry; its
/// scaled recurrence still checks values, exponents and safe fallback state.
pub(crate) fn insignificant_prefix_skip_allowed(latitude_count: usize) -> bool {
    latitude_count != 0
}

/// Prepares DUCC's per-order coefficient transformation and paired recurrence.
pub(crate) fn fill_factors(
    m: usize,
    lmax: usize,
    epsilon: &[f64],
    alpha: &mut [f64],
    coefficient_a: &mut [f64],
    coefficient_b: &mut [f64],
) -> bool {
    fill_factors_typed(
        m,
        lmax,
        epsilon,
        alpha,
        coefficient_a,
        coefficient_b,
        PAIRED_FACTOR_LIMIT,
    )
}

/// Prepares the paired recurrence factors using the caller's native precision.
/// The f64 production path keeps its established factor limit through
/// `fill_factors`; a native-f32 paired route can select and validate its own
/// limit without converting through f64.
pub(crate) fn fill_factors_typed<T: Float>(
    m: usize,
    lmax: usize,
    epsilon: &[T],
    alpha: &mut [T],
    coefficient_a: &mut [T],
    coefficient_b: &mut [T],
    factor_limit: T,
) -> bool {
    let factor_count = lmax / 2 + 2;
    debug_assert!(epsilon.len() >= lmax + 4);
    debug_assert!(alpha.len() >= factor_count.max(2));
    debug_assert!(coefficient_a.len() >= factor_count);
    debug_assert!(coefficient_b.len() >= factor_count);

    let one = T::one();
    alpha[0] = one / epsilon[m + 1];
    alpha[1] = epsilon[m + 1] / (epsilon[m + 2] * epsilon[m + 3]);
    for (index, degree) in (m + 2..lmax + 1).step_by(2).enumerate() {
        let alpha_index = index + 1;
        let sign = if alpha_index.is_multiple_of(2) {
            one
        } else {
            -one
        };
        alpha[alpha_index + 1] =
            sign / (epsilon[degree + 2] * epsilon[degree + 3] * alpha[alpha_index]);
    }

    for (index, degree) in (m..lmax + 2).step_by(2).enumerate() {
        let sign = if index.is_multiple_of(2) { one } else { -one };
        coefficient_a[index] = sign * alpha[index] * alpha[index];
        coefficient_b[index] = -coefficient_a[index]
            * (epsilon[degree + 2] * epsilon[degree + 2]
                + epsilon[degree + 1] * epsilon[degree + 1]);
    }

    let pair_count = (lmax - m + 2) / 2;
    alpha[..pair_count]
        .iter()
        .chain(&coefficient_a[..pair_count])
        .chain(&coefficient_b[..pair_count])
        .all(|value| value.is_finite() && value.abs() <= factor_limit)
}

/// Transforms the scalar harmonic coefficients used by the paired synthesis
/// recurrence. These values depend on order and degree, not on latitude.
pub(crate) fn prepare_synthesis_coefficients<T: Float>(
    m: usize,
    lmax: usize,
    epsilon: &[T],
    alpha: &[T],
    coefficients: &[Complex<T>],
    coefficient_even: &mut [Complex<T>],
    coefficient_odd: &mut [Complex<T>],
) -> bool {
    debug_assert!(m <= lmax);
    debug_assert!(epsilon.len() > lmax + 2);
    let pair_count = (lmax - m + 2) / 2;
    debug_assert!(alpha.len() >= pair_count);
    debug_assert!(coefficients.len() > lmax - m);
    debug_assert!(coefficient_even.len() >= pair_count);
    debug_assert!(coefficient_odd.len() >= pair_count);

    for pair in 0..pair_count {
        let degree = m + 2 * pair;
        let offset = degree - m;
        let next_even = if degree + 2 <= lmax {
            coefficients[offset + 2]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        let even = (coefficients[offset] * epsilon[degree + 1] + next_even * epsilon[degree + 2])
            * alpha[pair];
        let odd = if degree < lmax {
            coefficients[offset + 1] * alpha[pair]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        if !even.re.is_finite()
            || !even.im.is_finite()
            || !odd.re.is_finite()
            || !odd.im.is_finite()
        {
            return false;
        }
        coefficient_even[pair] = even;
        coefficient_odd[pair] = odd;
    }
    true
}

/// Prepares both converted spin-1 scalar rows in one degree-pair pass.
#[derive(Clone, Copy)]
pub(crate) struct PreparedSynthesisCoefficientPair<T> {
    pub(crate) first_even: num_complex::Complex<T>,
    pub(crate) first_odd: num_complex::Complex<T>,
    pub(crate) second_even: num_complex::Complex<T>,
    pub(crate) second_odd: num_complex::Complex<T>,
}

/// Prepares one pair from the three-degree windows emitted by spin1_to_scalar.
/// Entries beyond `lmax` are ignored exactly as in the bulk preparation path.
pub(crate) fn prepare_synthesis_coefficient_pair_values<T: Float>(
    degree: usize,
    lmax: usize,
    epsilon: &[T],
    alpha: T,
    first: [num_complex::Complex<T>; 3],
    second: [num_complex::Complex<T>; 3],
) -> PreparedSynthesisCoefficientPair<T> {
    let epsilon_even = epsilon[degree + 1];
    let epsilon_odd = epsilon[degree + 2];
    let first_next_even = if degree + 2 <= lmax {
        first[2]
    } else {
        num_complex::Complex::new(T::zero(), T::zero())
    };
    let second_next_even = if degree + 2 <= lmax {
        second[2]
    } else {
        num_complex::Complex::new(T::zero(), T::zero())
    };
    let first_even = (first[0] * epsilon_even + first_next_even * epsilon_odd) * alpha;
    let second_even = (second[0] * epsilon_even + second_next_even * epsilon_odd) * alpha;
    let (first_odd, second_odd) = if degree < lmax {
        (first[1] * alpha, second[1] * alpha)
    } else {
        (
            num_complex::Complex::new(T::zero(), T::zero()),
            num_complex::Complex::new(T::zero(), T::zero()),
        )
    };
    PreparedSynthesisCoefficientPair {
        first_even,
        first_odd,
        second_even,
        second_odd,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The spin-1 conversion supplies two coefficient rows and their paired outputs."
)]
pub(crate) fn prepare_synthesis_coefficient_pair<T: Float>(
    m: usize,
    lmax: usize,
    epsilon: &[T],
    alpha: &[T],
    first_coefficients: &[Complex<T>],
    second_coefficients: &[Complex<T>],
    first_even: &mut [Complex<T>],
    first_odd: &mut [Complex<T>],
    second_even: &mut [Complex<T>],
    second_odd: &mut [Complex<T>],
) -> bool {
    debug_assert!(m <= lmax);
    debug_assert!(epsilon.len() > lmax + 2);
    let pair_count = (lmax - m + 2) / 2;
    debug_assert!(alpha.len() >= pair_count);
    debug_assert!(first_coefficients.len() > lmax - m);
    debug_assert!(second_coefficients.len() > lmax - m);
    debug_assert!(first_even.len() >= pair_count);
    debug_assert!(first_odd.len() >= pair_count);
    debug_assert!(second_even.len() >= pair_count);
    debug_assert!(second_odd.len() >= pair_count);

    for pair in 0..pair_count {
        let degree = m + 2 * pair;
        let offset = degree - m;
        let alpha = alpha[pair];
        let first_next = if degree + 2 <= lmax {
            first_coefficients[offset + 2]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        let second_next = if degree + 2 <= lmax {
            second_coefficients[offset + 2]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        let first_next_odd = if degree < lmax {
            first_coefficients[offset + 1]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        let second_next_odd = if degree < lmax {
            second_coefficients[offset + 1]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        let prepared = prepare_synthesis_coefficient_pair_values(
            degree,
            lmax,
            epsilon,
            alpha,
            [first_coefficients[offset], first_next_odd, first_next],
            [second_coefficients[offset], second_next_odd, second_next],
        );
        let first_even_value = prepared.first_even;
        let first_odd_value = prepared.first_odd;
        let second_even_value = prepared.second_even;
        let second_odd_value = prepared.second_odd;
        if [
            first_even_value,
            first_odd_value,
            second_even_value,
            second_odd_value,
        ]
        .iter()
        .flat_map(|coefficient| [coefficient.re, coefficient.im])
        .any(|value| !value.is_finite())
        {
            return false;
        }
        first_even[pair] = first_even_value;
        first_odd[pair] = first_odd_value;
        second_even[pair] = second_even_value;
        second_odd[pair] = second_odd_value;
    }
    true
}

fn complex_scaled_product_is_normal_or_zero(
    result: Complex64,
    coefficient: Complex64,
    recurrence: f64,
    additional_factor: f64,
) -> bool {
    component_scaled_product_is_normal_or_zero(
        result.re,
        coefficient.re,
        recurrence,
        additional_factor,
    ) && component_scaled_product_is_normal_or_zero(
        result.im,
        coefficient.im,
        recurrence,
        additional_factor,
    )
}

#[inline]
fn component_scaled_product_is_normal_or_zero(
    result: f64,
    coefficient: f64,
    recurrence: f64,
    additional_factor: f64,
) -> bool {
    result.is_normal()
        || (result == 0.0 && (coefficient == 0.0 || recurrence == 0.0 || additional_factor == 0.0))
}

#[inline]
fn complex_sum_is_normal_or_zero(result: Complex64, left: Complex64, right: Complex64) -> bool {
    component_sum_is_normal_or_zero(result.re, left.re, right.re)
        && component_sum_is_normal_or_zero(result.im, left.im, right.im)
}

#[inline]
fn component_sum_is_normal_or_zero(result: f64, left: f64, right: f64) -> bool {
    result.is_normal() || (result == 0.0 && left == 0.0 && right == 0.0)
}

#[inline(always)]
fn scale_normal_prefix_product(product: f64, exponent: i64) -> Option<f64> {
    if product == 0.0 {
        return Some(product);
    }
    if !product.is_normal() || !(-4096..=4096).contains(&exponent) {
        return None;
    }
    let bits = product.to_bits();
    let raw_exponent = ((bits >> 52) & 0x7ff) as i64;
    let scaled_exponent = raw_exponent + exponent;
    if !(1..0x7ff).contains(&scaled_exponent) {
        return None;
    }
    let exponent_mask = 0x7ff_u64 << 52;
    Some(f64::from_bits(
        (bits & !exponent_mask) | ((scaled_exponent as u64) << 52),
    ))
}

#[inline(always)]
fn scale_normal_prefix_products_simd<S: Simd>(
    products: [S::f64s; 4],
    exponents: &[i64],
) -> Option<[S::f64s; 4]> {
    #[cfg(target_arch = "x86_64")]
    if const {
        S::F64_LANES == 4
            && core::mem::size_of::<S::f64s>() == core::mem::size_of::<std::arch::x86_64::__m256d>()
    } {
        if exponents.len() != 4 {
            return None;
        }
        // SAFETY: the size test above selects only Pulp's four-lane AVX2 type.
        let input = products.map(|value| unsafe { core::mem::transmute_copy(&value) });
        // SAFETY: the same dispatch selects AVX2 at runtime, and the helper
        // validates every exponent and result before returning scaled values.
        let scaled = unsafe { scale_normal_prefix_products_avx2(input, exponents) }?;
        // SAFETY: the output has the same four-lane layout checked above.
        return Some(scaled.map(|value| unsafe { core::mem::transmute_copy(&value) }));
    }
    let _ = products;
    let _ = exponents;
    None
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn scale_normal_prefix_products_avx2(
    products: [std::arch::x86_64::__m256d; 4],
    exponents: &[i64],
) -> Option<[std::arch::x86_64::__m256d; 4]> {
    use std::arch::x86_64::{
        __m256i, _mm256_add_epi64, _mm256_and_si256, _mm256_andnot_si256, _mm256_blendv_pd,
        _mm256_castpd_si256, _mm256_castsi256_pd, _mm256_cmp_pd, _mm256_cmpgt_epi64,
        _mm256_loadu_si256, _mm256_movemask_pd, _mm256_or_si256, _mm256_set1_epi64x,
        _mm256_set1_pd, _mm256_slli_epi64, _mm256_srli_epi64,
    };

    // SAFETY: the caller dispatches this only for Pulp's four-lane AVX2
    // specialization. The exponent slice has exactly four lanes.
    unsafe {
        let zero = _mm256_set1_pd(0.0);
        let one = _mm256_set1_pd(1.0);
        let exponent_vector = _mm256_loadu_si256(exponents.as_ptr().cast::<__m256i>());
        let zero_cmp = _mm256_set1_epi64x(0);
        let max_encoded_exponent = _mm256_set1_epi64x(0x7ff);
        let mantissa_mask = _mm256_set1_epi64x(!(0x7ff_i64 << 52));
        let positive_exponent_limit = _mm256_set1_epi64x(0x7ff);
        let lower_exponent_limit = _mm256_set1_epi64x(0);

        let mut product_bits = [core::mem::zeroed::<__m256i>(); 4];
        let mut zero_masks = [zero; 4];
        let mut scaled_exponents = [zero_cmp; 4];
        let mut all_valid = _mm256_set1_epi64x(-1);
        for index in 0..4 {
            zero_masks[index] = _mm256_cmp_pd(products[index], zero, 0);
            let zero_mask_i = _mm256_castpd_si256(zero_masks[index]);
            let active_exponent = _mm256_andnot_si256(zero_mask_i, exponent_vector);
            let exponent_in_range = _mm256_and_si256(
                _mm256_cmpgt_epi64(active_exponent, _mm256_set1_epi64x(-4097)),
                _mm256_cmpgt_epi64(_mm256_set1_epi64x(4097), active_exponent),
            );
            let product_for_bits = _mm256_blendv_pd(products[index], one, zero_masks[index]);
            let bits = _mm256_castpd_si256(product_for_bits);
            let encoded_exponent =
                _mm256_and_si256(_mm256_srli_epi64(bits, 52), max_encoded_exponent);
            let scaled_exponent = _mm256_add_epi64(encoded_exponent, active_exponent);
            let adjusted_exponent_valid = _mm256_and_si256(
                _mm256_cmpgt_epi64(scaled_exponent, lower_exponent_limit),
                _mm256_cmpgt_epi64(positive_exponent_limit, scaled_exponent),
            );
            scaled_exponents[index] = scaled_exponent;
            product_bits[index] = bits;
            all_valid = _mm256_and_si256(
                all_valid,
                _mm256_and_si256(exponent_in_range, adjusted_exponent_valid),
            );
        }
        if _mm256_movemask_pd(_mm256_castsi256_pd(all_valid)) != 0b1111 {
            return None;
        }

        let mut scaled = [zero; 4];
        for index in 0..4 {
            let mantissa = _mm256_and_si256(product_bits[index], mantissa_mask);
            let exponent_bits = _mm256_slli_epi64(scaled_exponents[index], 52);
            let adjusted = _mm256_castsi256_pd(_mm256_or_si256(mantissa, exponent_bits));
            scaled[index] = _mm256_blendv_pd(adjusted, products[index], zero_masks[index]);
        }
        Some(scaled)
    }
}

#[inline(always)]
fn normal_or_zero_mask<S: Simd>(simd: S, value: S::f64s) -> S::m64s {
    let magnitude = simd.abs_f64s(value);
    let normal = simd.and_m64s(
        simd.greater_than_or_equal_f64s(magnitude, simd.splat_f64s(f64::MIN_POSITIVE)),
        simd.less_than_or_equal_f64s(magnitude, simd.splat_f64s(f64::MAX)),
    );
    simd.or_m64s(normal, simd.equal_f64s(value, simd.splat_f64s(0.0)))
}

#[expect(
    clippy::too_many_arguments,
    reason = "SIMD helper carries separate lane accumulators"
)]
#[inline(always)]
fn try_accumulate_scaled_pair_vector<S: Simd>(
    simd: S,
    cos_theta: &[f64],
    current: &[f64],
    exponents: &[i64],
    first_even: Complex64,
    first_odd: Complex64,
    second_even: Complex64,
    second_odd: Complex64,
    first_sum_re: &mut [f64],
    first_sum_im: &mut [f64],
    second_sum_re: &mut [f64],
    second_sum_im: &mut [f64],
) -> bool {
    const MAX_F64_LANES: usize = 8;
    if S::F64_LANES < 2 || S::F64_LANES > MAX_F64_LANES || cos_theta.len() != S::F64_LANES {
        return false;
    }
    let (cos_vectors, cos_tail) = S::as_simd_f64s(cos_theta);
    let (current_vectors, current_tail) = S::as_simd_f64s(current);
    let (exponent_vectors, exponent_tail) = S::as_simd_i64s(exponents);
    if cos_vectors.len() != 1
        || current_vectors.len() != 1
        || exponent_vectors.len() != 1
        || !cos_tail.is_empty()
        || !current_tail.is_empty()
        || !exponent_tail.is_empty()
    {
        return false;
    }

    let zero = simd.splat_f64s(0.0);
    let cosine = cos_vectors[0];
    let recurrence = current_vectors[0];
    let first_odd_factor_re = simd.mul_f64s(simd.splat_f64s(first_odd.re), cosine);
    let first_odd_factor_im = simd.mul_f64s(simd.splat_f64s(first_odd.im), cosine);
    let second_odd_factor_re = simd.mul_f64s(simd.splat_f64s(second_odd.re), cosine);
    let second_odd_factor_im = simd.mul_f64s(simd.splat_f64s(second_odd.im), cosine);
    let first_factor_re = simd.mul_add_f64s(
        simd.splat_f64s(first_odd.re),
        cosine,
        simd.splat_f64s(first_even.re),
    );
    let first_factor_im = simd.mul_add_f64s(
        simd.splat_f64s(first_odd.im),
        cosine,
        simd.splat_f64s(first_even.im),
    );
    let second_factor_re = simd.mul_add_f64s(
        simd.splat_f64s(second_odd.re),
        cosine,
        simd.splat_f64s(second_even.re),
    );
    let second_factor_im = simd.mul_add_f64s(
        simd.splat_f64s(second_odd.im),
        cosine,
        simd.splat_f64s(second_even.im),
    );

    let factor_valid = |factor, odd_factor, even_is_zero| {
        let magnitude = simd.abs_f64s(factor);
        let normal = simd.and_m64s(
            simd.greater_than_or_equal_f64s(magnitude, simd.splat_f64s(f64::MIN_POSITIVE)),
            simd.less_than_or_equal_f64s(magnitude, simd.splat_f64s(f64::MAX)),
        );
        if even_is_zero {
            simd.or_m64s(
                simd.and_m64s(
                    simd.equal_f64s(factor, zero),
                    simd.equal_f64s(odd_factor, zero),
                ),
                normal,
            )
        } else {
            normal
        }
    };
    if !analysis_mask_all(
        simd,
        simd.and_m64s(
            simd.and_m64s(
                factor_valid(first_factor_re, first_odd_factor_re, first_even.re == 0.0),
                factor_valid(first_factor_im, first_odd_factor_im, first_even.im == 0.0),
            ),
            simd.and_m64s(
                factor_valid(
                    second_factor_re,
                    second_odd_factor_re,
                    second_even.re == 0.0,
                ),
                factor_valid(
                    second_factor_im,
                    second_odd_factor_im,
                    second_even.im == 0.0,
                ),
            ),
        ),
    ) || !analysis_mask_all(simd, normal_or_zero_mask(simd, recurrence))
    {
        return false;
    }

    let first_product_re = simd.mul_f64s(first_factor_re, recurrence);
    let first_product_im = simd.mul_f64s(first_factor_im, recurrence);
    let second_product_re = simd.mul_f64s(second_factor_re, recurrence);
    let second_product_im = simd.mul_f64s(second_factor_im, recurrence);
    let product_valid = |product, factor| {
        let normal = normal_or_zero_mask(simd, product);
        simd.and_m64s(
            normal,
            simd.or_m64s(
                simd.not_m64s(simd.equal_f64s(product, zero)),
                simd.or_m64s(
                    simd.equal_f64s(factor, zero),
                    simd.equal_f64s(recurrence, zero),
                ),
            ),
        )
    };
    if !analysis_mask_all(
        simd,
        simd.and_m64s(
            simd.and_m64s(
                product_valid(first_product_re, first_factor_re),
                product_valid(first_product_im, first_factor_im),
            ),
            simd.and_m64s(
                product_valid(second_product_re, second_factor_re),
                product_valid(second_product_im, second_factor_im),
            ),
        ),
    ) {
        return false;
    }

    let [
        first_product_re,
        first_product_im,
        second_product_re,
        second_product_im,
    ] = if let Some(scaled) = scale_normal_prefix_products_simd::<S>(
        [
            first_product_re,
            first_product_im,
            second_product_re,
            second_product_im,
        ],
        exponents,
    ) {
        scaled
    } else {
        let lane_count = cos_theta.len();
        let mut product_values = [[0.0; MAX_F64_LANES]; 4];
        simd.partial_store_f64s(&mut product_values[0][..lane_count], first_product_re);
        simd.partial_store_f64s(&mut product_values[1][..lane_count], first_product_im);
        simd.partial_store_f64s(&mut product_values[2][..lane_count], second_product_re);
        simd.partial_store_f64s(&mut product_values[3][..lane_count], second_product_im);
        let mut scaled_values = [[0.0; MAX_F64_LANES]; 4];
        for lane in 0..lane_count {
            for component in 0..4 {
                let Some(value) =
                    scale_normal_prefix_product(product_values[component][lane], exponents[lane])
                else {
                    return false;
                };
                scaled_values[component][lane] = value;
            }
        }
        let mut scaled_vectors = [zero; 4];
        for component in 0..4 {
            let (vectors, tail) = S::as_simd_f64s(&scaled_values[component][..lane_count]);
            if vectors.len() != 1 || !tail.is_empty() {
                return false;
            }
            scaled_vectors[component] = vectors[0];
        }
        scaled_vectors
    };

    let (first_sum_re_vectors, first_sum_re_tail) = S::as_simd_f64s(first_sum_re);
    let (first_sum_im_vectors, first_sum_im_tail) = S::as_simd_f64s(first_sum_im);
    let (second_sum_re_vectors, second_sum_re_tail) = S::as_simd_f64s(second_sum_re);
    let (second_sum_im_vectors, second_sum_im_tail) = S::as_simd_f64s(second_sum_im);
    if first_sum_re_vectors.len() != 1
        || first_sum_im_vectors.len() != 1
        || second_sum_re_vectors.len() != 1
        || second_sum_im_vectors.len() != 1
        || !first_sum_re_tail.is_empty()
        || !first_sum_im_tail.is_empty()
        || !second_sum_re_tail.is_empty()
        || !second_sum_im_tail.is_empty()
    {
        return false;
    }

    let next_first_sum_re = simd.add_f64s(first_sum_re_vectors[0], first_product_re);
    let next_first_sum_im = simd.add_f64s(first_sum_im_vectors[0], first_product_im);
    let next_second_sum_re = simd.add_f64s(second_sum_re_vectors[0], second_product_re);
    let next_second_sum_im = simd.add_f64s(second_sum_im_vectors[0], second_product_im);
    let sum_valid = |next_sum, prior_sum, product| {
        let normal = normal_or_zero_mask(simd, next_sum);
        let exact_zero = simd.and_m64s(
            simd.equal_f64s(next_sum, zero),
            simd.and_m64s(
                simd.equal_f64s(prior_sum, zero),
                simd.equal_f64s(product, zero),
            ),
        );
        simd.or_m64s(
            simd.and_m64s(normal, simd.not_m64s(simd.equal_f64s(next_sum, zero))),
            exact_zero,
        )
    };
    if !analysis_mask_all(
        simd,
        simd.and_m64s(
            simd.and_m64s(
                sum_valid(next_first_sum_re, first_sum_re_vectors[0], first_product_re),
                sum_valid(next_first_sum_im, first_sum_im_vectors[0], first_product_im),
            ),
            simd.and_m64s(
                sum_valid(
                    next_second_sum_re,
                    second_sum_re_vectors[0],
                    second_product_re,
                ),
                sum_valid(
                    next_second_sum_im,
                    second_sum_im_vectors[0],
                    second_product_im,
                ),
            ),
        ),
    ) {
        return false;
    }

    let (first_sum_re_vectors, _) = S::as_mut_simd_f64s(first_sum_re);
    let (first_sum_im_vectors, _) = S::as_mut_simd_f64s(first_sum_im);
    let (second_sum_re_vectors, _) = S::as_mut_simd_f64s(second_sum_re);
    let (second_sum_im_vectors, _) = S::as_mut_simd_f64s(second_sum_im);
    first_sum_re_vectors[0] = next_first_sum_re;
    first_sum_im_vectors[0] = next_first_sum_im;
    second_sum_re_vectors[0] = next_second_sum_re;
    second_sum_im_vectors[0] = next_second_sum_im;
    true
}

mod synthesis;

#[cfg(test)]
use synthesis::{FusedSynthesisPairBlock, OriginalSynthesisPairCoefficients};

#[cfg(test)]
pub(crate) use synthesis::synthesize_order_block_dispatched;
#[cfg(test)]
pub(crate) use synthesis::synthesize_paired_scalar_order_prepared_f32;
pub(crate) use synthesis::{
    synthesize_hemisphere_pair_block_dispatched, synthesize_order_block_partitioned_dispatched,
    synthesize_order_block_precomputed_dispatched, synthesize_order_pair_block_dispatched,
    synthesize_paired_scalar_block_partitioned_f32,
    synthesize_prepared_order_pair_block_dispatched,
    try_synthesize_order_pair_transition_dispatched,
    try_synthesize_prepared_order_pair_transition_dispatched,
};

mod analysis;
use analysis::analysis_mask_all;

pub(crate) use analysis::{
    accumulate_analysis_pair_channels_dispatched, accumulate_analysis_pair_dispatched,
    analysis_products_safe, fused_scalar_analysis_pair_block_preweighted_dispatched,
    fused_scalar_analysis_pair_block_preweighted_typed,
    fused_scalar_analysis_pair_channels_block_native,
    fused_scalar_analysis_pair_channels_block_with_range_proof, prepare_analysis_opposite,
    prepare_analysis_products_and_opposite, prepare_analysis_products_and_opposite_typed,
};

/// Precision leaves used by the shared paired scalar traversal.
///
/// The ring scheduling, degree loop, state rotation, and range-check policy
/// stay in one implementation; only seed, typed SIMD update, and scaling
/// operations remain specialized.
pub(crate) trait PairedScalarRecurrencePrecision: LegendrePrecision {
    fn initialize_pair_state(
        m: usize,
        sin_theta: &[Self],
        sectoral_norm: Self,
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<(), ShtError>;

    fn step_pair<S: Simd>(
        simd: S,
        coefficient_a: Self,
        coefficient_b: Self,
        cosine: &[Self],
        previous: &[Self],
        current: &[Self],
        next: &mut [Self],
    );

    fn normalize_pair_state(
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<bool, ShtError>;

    #[inline(always)]
    fn normalize_pair_state_simd<S: Simd>(
        simd: S,
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<bool, ShtError> {
        let _ = simd;
        Self::normalize_pair_state(previous, current, exponents)
    }

    fn pair_state_needs_scaling<S: Simd>(simd: S, previous: &[Self], current: &[Self]) -> bool;
}

impl PairedScalarRecurrencePrecision for f64 {
    #[inline(always)]
    fn initialize_pair_state(
        m: usize,
        sin_theta: &[Self],
        sectoral_norm: Self,
        previous: &mut [Self],
        current: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<(), ShtError> {
        for ring in 0..sin_theta.len() {
            let (value, exponent) = scalar::sectoral_value(sin_theta[ring], m, sectoral_norm)?;
            previous[ring] = 0.0;
            current[ring] = value;
            exponents[ring] = exponent;
        }
        Ok(())
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
        let (current_vectors, current_tail) = S::as_simd_f64s(current);
        let (previous_vectors, previous_tail) = S::as_simd_f64s(previous);
        let (cos_vectors, cos_tail) = S::as_simd_f64s(cosine);
        let (next_vectors, next_tail) = S::as_mut_simd_f64s(next);
        let a = simd.splat_f64s(coefficient_a);
        let b = simd.splat_f64s(coefficient_b);
        for index in 0..current_vectors.len() {
            let cos_squared = simd.mul_f64s(cos_vectors[index], cos_vectors[index]);
            let leading = simd.mul_add_f64s(a, cos_squared, b);
            next_vectors[index] =
                simd.mul_add_f64s(leading, current_vectors[index], previous_vectors[index]);
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
        scalar::normalize_pair_lanes(previous, current, exponents)
    }

    #[inline(always)]
    fn pair_state_needs_scaling<S: Simd>(simd: S, previous: &[Self], current: &[Self]) -> bool {
        scalar::pair_lanes_need_scaling(simd, previous, current)
    }
}

/// Visits transformed scalar basis values for f64 consumers.
#[expect(
    clippy::too_many_arguments,
    reason = "The recurrence uses plan factors, geometry and caller-owned lane scratch."
)]
pub(crate) fn for_each_y_lm_paired_dispatched<F>(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
    mut visit: F,
) -> Result<(), ShtError>
where
    F: FnMut(usize, usize, &[f64], &[i64]) -> Result<(), ShtError>,
{
    for_each_y_lm_paired_status_dispatched(
        arch,
        m,
        lmax,
        sin_theta,
        cos_theta,
        sectoral_norm,
        coefficient_a,
        coefficient_b,
        previous,
        current,
        next,
        exponents,
        |l, ring_start, basis, lane_exponents, _normal_range| {
            visit(l, ring_start, basis, lane_exponents)
        },
    )
}

/// Visits paired f64 scalar basis values and reports their normal-range state.
#[expect(
    clippy::too_many_arguments,
    reason = "The recurrence uses plan factors, geometry and caller-owned lane scratch."
)]
pub(crate) fn for_each_y_lm_paired_status_dispatched<F>(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
    visit: F,
) -> Result<(), ShtError>
where
    F: FnMut(usize, usize, &[f64], &[i64], bool) -> Result<(), ShtError>,
{
    for_each_y_lm_paired_status_dispatched_typed::<f64, F>(
        arch,
        m,
        lmax,
        sin_theta,
        cos_theta,
        sectoral_norm,
        coefficient_a,
        coefficient_b,
        previous,
        current,
        next,
        exponents,
        visit,
    )
}

/// Shared order and latitude-block traversal for transformed scalar bases.
pub(crate) trait PairedScalarRecurrenceVisitor<T> {
    fn visit<S: Simd>(
        &mut self,
        degree: usize,
        ring_start: usize,
        basis: &[T],
        exponents: &[i64],
        normal_range: bool,
        simd: S,
    ) -> Result<(), ShtError>;
}

impl<T, F> PairedScalarRecurrenceVisitor<T> for F
where
    F: for<'a, 'b> FnMut(usize, usize, &'a [T], &'b [i64], bool) -> Result<(), ShtError>,
{
    #[inline(always)]
    fn visit<S: Simd>(
        &mut self,
        degree: usize,
        ring_start: usize,
        basis: &[T],
        exponents: &[i64],
        normal_range: bool,
        _simd: S,
    ) -> Result<(), ShtError> {
        self(degree, ring_start, basis, exponents, normal_range)
    }
}

struct PairedScalarVisitorRef<'a, V>(&'a mut V);

impl<T, V> PairedScalarRecurrenceVisitor<T> for PairedScalarVisitorRef<'_, V>
where
    V: PairedScalarRecurrenceVisitor<T>,
{
    #[inline(always)]
    fn visit<S: Simd>(
        &mut self,
        degree: usize,
        ring_start: usize,
        basis: &[T],
        exponents: &[i64],
        normal_range: bool,
        simd: S,
    ) -> Result<(), ShtError> {
        self.0
            .visit(degree, ring_start, basis, exponents, normal_range, simd)
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The recurrence uses plan factors, geometry and caller-owned lane scratch."
)]
pub(crate) fn for_each_y_lm_paired_status_dispatched_typed<T, F>(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[T],
    cos_theta: &[T],
    sectoral_norm: T,
    coefficient_a: &[T],
    coefficient_b: &[T],
    previous: &mut [T],
    current: &mut [T],
    next: &mut [T],
    exponents: &mut [i64],
    mut visit: F,
) -> Result<(), ShtError>
where
    T: PairedScalarRecurrencePrecision,
    F: PairedScalarRecurrenceVisitor<T>,
{
    debug_assert!(m <= lmax);
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    let pair_count = (lmax - m + 2) / 2;
    debug_assert!(coefficient_a.len() >= pair_count);
    debug_assert!(coefficient_b.len() >= pair_count);
    let scratch_len = sin_theta.len().min(LATITUDE_BLOCK_SIZE);
    debug_assert!(previous.len() >= scratch_len);
    debug_assert!(current.len() >= scratch_len);
    debug_assert!(next.len() >= scratch_len);
    debug_assert!(exponents.len() >= scratch_len);

    for ring_start in (0..sin_theta.len()).step_by(LATITUDE_BLOCK_SIZE) {
        let ring_end = (ring_start + LATITUDE_BLOCK_SIZE).min(sin_theta.len());
        let block_len = ring_end - ring_start;
        arch.dispatch(ForEachPairedYlm {
            ring_start,
            m,
            lmax,
            sin_theta: &sin_theta[ring_start..ring_end],
            cos_theta: &cos_theta[ring_start..ring_end],
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            previous: &mut previous[..block_len],
            current: &mut current[..block_len],
            next: &mut next[..block_len],
            exponents: &mut exponents[..block_len],
            visit: PairedScalarVisitorRef(&mut visit),
            _precision: PhantomData,
        })?;
    }
    Ok(())
}

struct ForEachPairedYlm<'a, T, F> {
    ring_start: usize,
    m: usize,
    lmax: usize,
    sin_theta: &'a [T],
    cos_theta: &'a [T],
    sectoral_norm: T,
    coefficient_a: &'a [T],
    coefficient_b: &'a [T],
    previous: &'a mut [T],
    current: &'a mut [T],
    next: &'a mut [T],
    exponents: &'a mut [i64],
    visit: F,
    _precision: PhantomData<T>,
}

impl<T, F> WithSimd for ForEachPairedYlm<'_, T, F>
where
    T: PairedScalarRecurrencePrecision,
    F: PairedScalarRecurrenceVisitor<T>,
{
    type Output = Result<(), ShtError>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let ForEachPairedYlm {
            ring_start,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            previous,
            current,
            next,
            exponents,
            mut visit,
            _precision: _,
        } = self;
        let mut previous = previous;
        let mut current = current;
        let mut next = next;

        T::initialize_pair_state(m, sin_theta, sectoral_norm, previous, current, exponents)?;
        let mut normal_range = T::normalize_pair_state_simd(simd, previous, current, exponents)?;

        for (pair_index, degree) in (m..=lmax).step_by(2).enumerate() {
            visit.visit(degree, ring_start, current, exponents, normal_range, simd)?;
            if degree + 2 > lmax {
                break;
            }

            T::step_pair(
                simd,
                coefficient_a[pair_index],
                coefficient_b[pair_index],
                cos_theta,
                previous,
                current,
                next,
            );

            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
            if !normal_range || T::pair_state_needs_scaling(simd, previous, current) {
                normal_range = T::normalize_pair_state_simd(simd, previous, current, exponents)?;
            }
        }
        Ok(())
    }
}
