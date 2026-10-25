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

use num_complex::Complex64;
use pulp::{Arch, Simd, WithSimd};
use rustfft::num_traits::Float;

use crate::error::ShtError;
use crate::legendre::scalar::{
    lane_values_in_normal_range, scalar_value_in_normal_range, simd_values_in_normal_range,
};
use crate::legendre::{LATITUDE_BLOCK_SIZE, scalar};

#[cfg(test)]
#[path = "scalar_paired_kernel_bench.rs"]
mod kernel_bench;

const SIMD_SECTORAL_INIT_MIN_LMAX: usize = 511;
const ANALYSIS_FTOL: f64 = f64::from_bits(963_u64 << 52);
const ANALYSIS_SCALE_STEP: i64 = 800;
const ANALYSIS_SCALE_UP: f64 = f64::from_bits(1823_u64 << 52);
const ANALYSIS_SCALE_DOWN: f64 = f64::from_bits(223_u64 << 52);
const ANALYSIS_FBIGHALF: f64 = f64::from_bits(1423_u64 << 52);
const ANALYSIS_MAX_SCALE_EPOCH: f64 = 4.5e15;
const PAIRED_FACTOR_LIMIT: f64 = f64::from_bits(1151_u64 << 52);
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
pub(crate) fn begin_scalar_analysis_route_profile() {
    use std::sync::atomic::Ordering;

    SCALAR_ANALYSIS_ROUTE_PROFILE_ENABLED.store(false, Ordering::Relaxed);
    SCALAR_ANALYSIS_NORMAL_SEED_BLOCKS.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_PREFIX_ENABLED_SCALED_BLOCKS.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_CHECKED_SCALED_BLOCKS.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_PREFIX_INITIALIZATION_DECLINES.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_PREFIX_RECURRENCE_DECLINES.store(0, Ordering::Relaxed);
    SCALAR_ANALYSIS_CHECKED_SCALED_RECURRENCE_DECLINES.store(0, Ordering::Relaxed);
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
    let factor_count = lmax / 2 + 2;
    debug_assert!(epsilon.len() >= lmax + 4);
    debug_assert!(alpha.len() >= factor_count.max(2));
    debug_assert!(coefficient_a.len() >= factor_count);
    debug_assert!(coefficient_b.len() >= factor_count);

    alpha[0] = 1.0 / epsilon[m + 1];
    alpha[1] = epsilon[m + 1] / (epsilon[m + 2] * epsilon[m + 3]);
    for (index, degree) in (m + 2..lmax + 1).step_by(2).enumerate() {
        let alpha_index = index + 1;
        let sign = if alpha_index.is_multiple_of(2) {
            1.0
        } else {
            -1.0
        };
        alpha[alpha_index + 1] =
            sign / (epsilon[degree + 2] * epsilon[degree + 3] * alpha[alpha_index]);
    }

    for (index, degree) in (m..lmax + 2).step_by(2).enumerate() {
        let sign = if index.is_multiple_of(2) { 1.0 } else { -1.0 };
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
        .all(|value| value.is_finite() && value.abs() <= PAIRED_FACTOR_LIMIT)
}

/// Transforms the scalar harmonic coefficients used by the paired synthesis
/// recurrence. These values depend on order and degree, not on latitude.
pub(crate) fn prepare_synthesis_coefficients(
    m: usize,
    lmax: usize,
    epsilon: &[f64],
    alpha: &[f64],
    coefficients: &[Complex64],
    coefficient_even: &mut [Complex64],
    coefficient_odd: &mut [Complex64],
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
            Complex64::new(0.0, 0.0)
        };
        let even = (coefficients[offset] * epsilon[degree + 1] + next_even * epsilon[degree + 2])
            * alpha[pair];
        let odd = if degree < lmax {
            coefficients[offset + 1] * alpha[pair]
        } else {
            Complex64::new(0.0, 0.0)
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
pub(crate) fn prepare_synthesis_coefficient_pair(
    m: usize,
    lmax: usize,
    epsilon: &[f64],
    alpha: &[f64],
    first_coefficients: &[Complex64],
    second_coefficients: &[Complex64],
    first_even: &mut [Complex64],
    first_odd: &mut [Complex64],
    second_even: &mut [Complex64],
    second_odd: &mut [Complex64],
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
            Complex64::new(0.0, 0.0)
        };
        let second_next = if degree + 2 <= lmax {
            second_coefficients[offset + 2]
        } else {
            Complex64::new(0.0, 0.0)
        };
        let first_next_odd = if degree < lmax {
            first_coefficients[offset + 1]
        } else {
            Complex64::new(0.0, 0.0)
        };
        let second_next_odd = if degree < lmax {
            second_coefficients[offset + 1]
        } else {
            Complex64::new(0.0, 0.0)
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

/// Tries a register-resident scalar synthesis block for ordinary recurrence values.
///
/// A block is declined when any lane is excluded by the polar order limit or
/// sectoral initialization needs binary scaling. Callers then use the general
/// paired recurrence, which retains per-lane scaling.
#[expect(
    clippy::too_many_arguments,
    reason = "The fused block receives recurrence factors, spectra and worker scratch explicitly."
)]
#[cfg(test)]
pub(crate) fn synthesize_order_block_dispatched(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    epsilon: &[f64],
    alpha: &[f64],
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    coefficients: &[Complex64],
    ring_order_limits: &[usize],
    spectrum_len: usize,
    spectra: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    let pair_count = (lmax - m + 2) / 2;
    let mut coefficient_even = vec![Complex64::new(0.0, 0.0); pair_count];
    let mut coefficient_odd = vec![Complex64::new(0.0, 0.0); pair_count];
    if !prepare_synthesis_coefficients(
        m,
        lmax,
        epsilon,
        alpha,
        coefficients,
        &mut coefficient_even,
        &mut coefficient_odd,
    ) {
        return Ok(false);
    }
    synthesize_order_block_precomputed_dispatched(
        arch,
        m,
        m,
        lmax,
        sin_theta,
        cos_theta,
        sectoral_norm,
        coefficient_a,
        coefficient_b,
        &coefficient_even,
        &coefficient_odd,
        ring_order_limits,
        spectrum_len,
        spectra,
        previous,
        current,
        next,
        exponents,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The precomputed fused block receives operator data and worker scratch explicitly."
)]
pub(crate) fn synthesize_order_block_precomputed_dispatched(
    arch: Arch,
    m: usize,
    output_order: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    coefficient_even: &[Complex64],
    coefficient_odd: &[Complex64],
    ring_order_limits: &[usize],
    spectrum_len: usize,
    spectra: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    debug_assert_eq!(sin_theta.len(), ring_order_limits.len());
    debug_assert_eq!(spectra.len(), sin_theta.len() * spectrum_len);
    debug_assert!(previous.len() >= sin_theta.len());
    debug_assert!(current.len() >= sin_theta.len());
    debug_assert!(next.len() >= sin_theta.len());
    debug_assert!(exponents.len() >= sin_theta.len());
    let pair_count = (lmax - m + 2) / 2;
    debug_assert!(coefficient_even.len() >= pair_count);
    debug_assert!(coefficient_odd.len() >= pair_count);

    if sin_theta.is_empty() || ring_order_limits.iter().any(|&limit| m > limit) {
        return Ok(false);
    }

    for ring in 0..sin_theta.len() {
        let (value, exponent) = scalar::sectoral_value(sin_theta[ring], m, sectoral_norm)?;
        previous[ring] = 0.0;
        current[ring] = value;
        exponents[ring] = exponent;
    }
    if !scalar::normalize_pair_lanes(
        &mut previous[..sin_theta.len()],
        &mut current[..sin_theta.len()],
        &mut exponents[..sin_theta.len()],
    )? {
        return Ok(false);
    }

    arch.dispatch(FusedSynthesisBlock {
        m,
        lmax,
        cos_theta,
        coefficient_a,
        coefficient_b,
        coefficient_even,
        coefficient_odd,
        current: &current[..sin_theta.len()],
        sum_re: &mut previous[..sin_theta.len()],
        sum_im: &mut next[..sin_theta.len()],
    });
    for ring in 0..sin_theta.len() {
        spectra[ring * spectrum_len + output_order] += Complex64::new(previous[ring], next[ring]);
    }
    Ok(true)
}

#[expect(
    clippy::too_many_arguments,
    reason = "The partitioned block receives existing worker-owned compaction and recurrence scratch explicitly."
)]
pub(crate) fn synthesize_order_block_partitioned_dispatched(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    coefficient_even: &[Complex64],
    coefficient_odd: &[Complex64],
    ring_order_limits: &[usize],
    spectrum_len: usize,
    spectra: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
    compact_theta_sin: &mut [f64],
    compact_theta_cos: &mut [f64],
    compact_ring_indices: &mut [usize],
) -> Result<bool, ShtError> {
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    debug_assert_eq!(sin_theta.len(), ring_order_limits.len());
    debug_assert!(sin_theta.len() <= LATITUDE_BLOCK_SIZE);
    debug_assert_eq!(spectra.len(), sin_theta.len() * spectrum_len);
    debug_assert!(previous.len() >= sin_theta.len());
    debug_assert!(current.len() >= sin_theta.len());
    debug_assert!(next.len() >= sin_theta.len());
    debug_assert!(exponents.len() >= sin_theta.len());
    debug_assert!(compact_theta_sin.len() >= sin_theta.len());
    debug_assert!(compact_theta_cos.len() >= sin_theta.len());
    debug_assert!(compact_ring_indices.len() >= sin_theta.len());
    let pair_count = (lmax - m + 2) / 2;
    debug_assert!(coefficient_a.len() >= pair_count);
    debug_assert!(coefficient_b.len() >= pair_count);
    debug_assert!(coefficient_even.len() >= pair_count);
    debug_assert!(coefficient_odd.len() >= pair_count);

    if sin_theta.is_empty() || ring_order_limits.iter().any(|&limit| m > limit) {
        return Ok(false);
    }

    for ring in 0..sin_theta.len() {
        let (value, exponent) = scalar::sectoral_value(sin_theta[ring], m, sectoral_norm)?;
        previous[ring] = 0.0;
        current[ring] = value;
        exponents[ring] = exponent;
    }
    let all_normal = scalar::normalize_pair_lanes(
        &mut previous[..sin_theta.len()],
        &mut current[..sin_theta.len()],
        &mut exponents[..sin_theta.len()],
    )?;
    if all_normal {
        arch.dispatch(FusedSynthesisBlock {
            m,
            lmax,
            cos_theta,
            coefficient_a,
            coefficient_b,
            coefficient_even,
            coefficient_odd,
            current: &current[..sin_theta.len()],
            sum_re: &mut previous[..sin_theta.len()],
            sum_im: &mut next[..sin_theta.len()],
        });
        for ring in 0..sin_theta.len() {
            spectra[ring * spectrum_len + m] += Complex64::new(previous[ring], next[ring]);
        }
        return Ok(true);
    }

    let mut normal_lanes = [false; LATITUDE_BLOCK_SIZE];
    let mut normal_count = 0;
    for ring in 0..sin_theta.len() {
        let value = current[ring];
        let magnitude = value.abs();
        let normal = exponents[ring] == 0
            && (value == 0.0
                || (value.is_finite() && (scalar::SMALL..=scalar::BIG).contains(&magnitude)));
        normal_lanes[ring] = normal;
        normal_count += usize::from(normal);
    }

    // Keep short SIMD batches on the established scaled path; splitting is
    // useful when it can supply at least one full AVX2 f64 vector.
    if normal_count < 4 || normal_count == sin_theta.len() {
        return Ok(false);
    }

    let mut compact_count = 0;
    for ring in 0..sin_theta.len() {
        if normal_lanes[ring] {
            compact_theta_sin[compact_count] = sin_theta[ring];
            compact_theta_cos[compact_count] = cos_theta[ring];
            compact_ring_indices[compact_count] = ring;
            current[compact_count] = current[ring];
            compact_count += 1;
        }
    }
    debug_assert_eq!(compact_count, normal_count);
    arch.dispatch(FusedSynthesisBlock {
        m,
        lmax,
        cos_theta: &compact_theta_cos[..normal_count],
        coefficient_a,
        coefficient_b,
        coefficient_even,
        coefficient_odd,
        current: &current[..normal_count],
        sum_re: &mut previous[..normal_count],
        sum_im: &mut next[..normal_count],
    });
    for compact_ring in 0..normal_count {
        let ring = compact_ring_indices[compact_ring];
        spectra[ring * spectrum_len + m] +=
            Complex64::new(previous[compact_ring], next[compact_ring]);
    }

    let mut scaled_count = 0;
    for ring in 0..sin_theta.len() {
        if !normal_lanes[ring] {
            compact_theta_sin[scaled_count] = sin_theta[ring];
            compact_theta_cos[scaled_count] = cos_theta[ring];
            compact_ring_indices[scaled_count] = ring;
            scaled_count += 1;
        }
    }
    debug_assert!(scaled_count > 0);
    for_each_y_lm_paired_dispatched(
        arch,
        m,
        lmax,
        &compact_theta_sin[..scaled_count],
        &compact_theta_cos[..scaled_count],
        sectoral_norm,
        coefficient_a,
        coefficient_b,
        previous,
        current,
        next,
        exponents,
        |l, local_ring_start, basis, lane_exponents| {
            let pair_index = (l - m) / 2;
            let lane_end = local_ring_start + basis.len();
            for (lane, &value) in basis.iter().enumerate() {
                let compact_ring = local_ring_start + lane;
                let ring = compact_ring_indices[compact_ring];
                let contribution = scalar::multiply_complex_scaled(
                    coefficient_even[pair_index],
                    value,
                    lane_exponents[lane],
                ) + scalar::multiply_complex_scaled_product(
                    coefficient_odd[pair_index],
                    value,
                    compact_theta_cos[compact_ring],
                    lane_exponents[lane],
                );
                spectra[ring * spectrum_len + m] += contribution;
            }
            debug_assert!(lane_end <= scaled_count);
            Ok(())
        },
    )?;
    Ok(true)
}

/// Tries to synthesize two scalar coefficient rows with one shared recurrence.
///
/// Spin-2 scalar conversion produces two rows evaluated at the same latitude
/// lanes. Keeping both accumulators live avoids advancing the paired recurrence
/// twice. The caller uses the scaled path when the block is polar or inactive.
#[expect(
    clippy::too_many_arguments,
    reason = "The paired-channel block receives two spectra and four worker accumulators explicitly."
)]
pub(crate) fn synthesize_order_pair_block_dispatched(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    epsilon: &[f64],
    alpha: &[f64],
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    first_coefficients: &[Complex64],
    second_coefficients: &[Complex64],
    ring_order_limits: &[usize],
    spectrum_len: usize,
    first_spectra: &mut [Complex64],
    second_spectra: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
    first_sum_re: &mut [f64],
    first_sum_im: &mut [f64],
    second_sum_re: &mut [f64],
    second_sum_im: &mut [f64],
) -> Result<bool, ShtError> {
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    debug_assert_eq!(sin_theta.len(), ring_order_limits.len());
    debug_assert_eq!(first_spectra.len(), sin_theta.len() * spectrum_len);
    debug_assert_eq!(second_spectra.len(), first_spectra.len());
    debug_assert!(previous.len() >= sin_theta.len());
    debug_assert!(current.len() >= sin_theta.len());
    debug_assert!(next.len() >= sin_theta.len());
    debug_assert!(exponents.len() >= sin_theta.len());
    debug_assert!(first_sum_re.len() >= sin_theta.len());
    debug_assert!(first_sum_im.len() >= sin_theta.len());
    debug_assert!(second_sum_re.len() >= sin_theta.len());
    debug_assert!(second_sum_im.len() >= sin_theta.len());
    debug_assert!(first_coefficients.len() > lmax - m);
    debug_assert!(second_coefficients.len() > lmax - m);

    if sin_theta.is_empty() || ring_order_limits.iter().any(|&limit| m > limit) {
        return Ok(false);
    }

    for ring in 0..sin_theta.len() {
        let (value, exponent) = scalar::sectoral_value(sin_theta[ring], m, sectoral_norm)?;
        previous[ring] = 0.0;
        current[ring] = value;
        exponents[ring] = exponent;
    }
    if !scalar::normalize_pair_lanes(
        &mut previous[..sin_theta.len()],
        &mut current[..sin_theta.len()],
        &mut exponents[..sin_theta.len()],
    )? {
        return Ok(false);
    }

    let fused = arch.dispatch(FusedSynthesisPairBlock {
        m,
        lmax,
        pair_start: 0,
        cos_theta,
        coefficient_a,
        coefficient_b,
        coefficients: OriginalSynthesisPairCoefficients {
            epsilon,
            alpha,
            first_coefficients,
            second_coefficients,
        },
        previous: &previous[..sin_theta.len()],
        current: &current[..sin_theta.len()],
        first_sum_re: &mut first_sum_re[..sin_theta.len()],
        first_sum_im: &mut first_sum_im[..sin_theta.len()],
        second_sum_re: &mut second_sum_re[..sin_theta.len()],
        second_sum_im: &mut second_sum_im[..sin_theta.len()],
        initialize_sums: true,
        check_normal_range: false,
    });
    debug_assert!(fused);
    for ring in 0..sin_theta.len() {
        let spectrum_index = ring * spectrum_len + m;
        first_spectra[spectrum_index] += Complex64::new(first_sum_re[ring], first_sum_im[ring]);
        second_spectra[spectrum_index] += Complex64::new(second_sum_re[ring], second_sum_im[ring]);
    }
    Ok(true)
}

#[expect(
    clippy::too_many_arguments,
    reason = "The prepared dual-channel block receives both spectra and caller-owned recurrence scratch."
)]
pub(crate) fn synthesize_prepared_order_pair_block_dispatched(
    arch: Arch,
    m: usize,
    output_order: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    first_even: &[Complex64],
    first_odd: &[Complex64],
    second_even: &[Complex64],
    second_odd: &[Complex64],
    ring_order_limits: &[usize],
    spectrum_len: usize,
    first_spectra: &mut [Complex64],
    second_spectra: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
    first_sum_re: &mut [f64],
    first_sum_im: &mut [f64],
    second_sum_re: &mut [f64],
    second_sum_im: &mut [f64],
) -> Result<bool, ShtError> {
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    debug_assert_eq!(sin_theta.len(), ring_order_limits.len());
    debug_assert_eq!(first_spectra.len(), sin_theta.len() * spectrum_len);
    debug_assert_eq!(second_spectra.len(), first_spectra.len());
    debug_assert!(previous.len() >= sin_theta.len());
    debug_assert!(current.len() >= sin_theta.len());
    debug_assert!(next.len() >= sin_theta.len());
    debug_assert!(exponents.len() >= sin_theta.len());
    debug_assert!(first_sum_re.len() >= sin_theta.len());
    debug_assert!(first_sum_im.len() >= sin_theta.len());
    debug_assert!(second_sum_re.len() >= sin_theta.len());
    debug_assert!(second_sum_im.len() >= sin_theta.len());
    let pair_count = (lmax - m + 2) / 2;
    debug_assert!(coefficient_a.len() >= pair_count);
    debug_assert!(coefficient_b.len() >= pair_count);
    debug_assert!(first_even.len() >= pair_count);
    debug_assert!(first_odd.len() >= pair_count);
    debug_assert!(second_even.len() >= pair_count);
    debug_assert!(second_odd.len() >= pair_count);

    let coefficient_a = &coefficient_a[..pair_count];
    let coefficient_b = &coefficient_b[..pair_count];
    let first_even = &first_even[..pair_count];
    let first_odd = &first_odd[..pair_count];
    let second_even = &second_even[..pair_count];
    let second_odd = &second_odd[..pair_count];

    if sin_theta.is_empty() || ring_order_limits.iter().any(|&limit| m > limit) {
        return Ok(false);
    }

    for ring in 0..sin_theta.len() {
        let (value, exponent) = scalar::sectoral_value(sin_theta[ring], m, sectoral_norm)?;
        previous[ring] = 0.0;
        current[ring] = value;
        exponents[ring] = exponent;
    }
    if !scalar::normalize_pair_lanes(
        &mut previous[..sin_theta.len()],
        &mut current[..sin_theta.len()],
        &mut exponents[..sin_theta.len()],
    )? {
        return Ok(false);
    }

    arch.dispatch(FusedPreparedSynthesisPairBlock {
        pair_count,
        cos_theta,
        coefficient_a,
        coefficient_b,
        first_even,
        first_odd,
        second_even,
        second_odd,
        current: &current[..sin_theta.len()],
        first_sum_re: &mut first_sum_re[..sin_theta.len()],
        first_sum_im: &mut first_sum_im[..sin_theta.len()],
        second_sum_re: &mut second_sum_re[..sin_theta.len()],
        second_sum_im: &mut second_sum_im[..sin_theta.len()],
    });
    for ring in 0..sin_theta.len() {
        let spectrum_index = ring * spectrum_len + output_order;
        first_spectra[spectrum_index] += Complex64::new(first_sum_re[ring], first_sum_im[ring]);
        second_spectra[spectrum_index] += Complex64::new(second_sum_re[ring], second_sum_im[ring]);
    }
    Ok(true)
}

#[expect(
    clippy::too_many_arguments,
    reason = "The transition kernel receives coefficient factors and caller-owned lane scratch explicitly."
)]
pub(crate) fn try_synthesize_order_pair_transition_dispatched(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    epsilon: &[f64],
    alpha: &[f64],
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    first_coefficients: &[Complex64],
    second_coefficients: &[Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
    first_sum_re: &mut [f64],
    first_sum_im: &mut [f64],
    second_sum_re: &mut [f64],
    second_sum_im: &mut [f64],
) -> Result<bool, ShtError> {
    let lane_count = sin_theta.len();
    debug_assert_eq!(lane_count, cos_theta.len());
    debug_assert!(lane_count <= LATITUDE_BLOCK_SIZE);
    debug_assert!(previous.len() >= lane_count);
    debug_assert!(current.len() >= lane_count);
    debug_assert!(next.len() >= lane_count);
    debug_assert!(exponents.len() >= lane_count);
    debug_assert!(first_sum_re.len() >= lane_count);
    debug_assert!(first_sum_im.len() >= lane_count);
    debug_assert!(second_sum_re.len() >= lane_count);
    debug_assert!(second_sum_im.len() >= lane_count);
    debug_assert!(first_coefficients.len() > lmax - m);
    debug_assert!(second_coefficients.len() > lmax - m);

    if lane_count == 0
        || sin_theta
            .iter()
            .any(|&sine| !sine.is_finite() || sine < 0.01)
    {
        return Ok(false);
    }

    let pair_count = (lmax - m + 2) / 2;
    let maximum_sum_bound = 2.0 * scalar::BIG * scalar::BIG * pair_count as f64 * 10_000.0;
    if !maximum_sum_bound.is_finite()
        || !paired_scalar_coefficients_are_safe(
            m,
            lmax,
            epsilon,
            alpha,
            first_coefficients,
            second_coefficients,
        )
    {
        return Ok(false);
    }

    dispatch_synthesis_pair_transition(
        arch,
        m,
        lmax,
        sin_theta,
        cos_theta,
        sectoral_norm,
        coefficient_a,
        coefficient_b,
        OriginalSynthesisPairCoefficients {
            epsilon,
            alpha,
            first_coefficients,
            second_coefficients,
        },
        previous,
        current,
        next,
        exponents,
        first_sum_re,
        first_sum_im,
        second_sum_re,
        second_sum_im,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The prepared transition receives four checked coefficient-pair rows and lane scratch."
)]
pub(crate) fn try_synthesize_prepared_order_pair_transition_dispatched(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    first_even: &[Complex64],
    first_odd: &[Complex64],
    second_even: &[Complex64],
    second_odd: &[Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
    first_sum_re: &mut [f64],
    first_sum_im: &mut [f64],
    second_sum_re: &mut [f64],
    second_sum_im: &mut [f64],
) -> Result<bool, ShtError> {
    let lane_count = sin_theta.len();
    debug_assert_eq!(lane_count, cos_theta.len());
    debug_assert!(lane_count <= LATITUDE_BLOCK_SIZE);
    debug_assert!(previous.len() >= lane_count);
    debug_assert!(current.len() >= lane_count);
    debug_assert!(next.len() >= lane_count);
    debug_assert!(exponents.len() >= lane_count);
    debug_assert!(first_sum_re.len() >= lane_count);
    debug_assert!(first_sum_im.len() >= lane_count);
    debug_assert!(second_sum_re.len() >= lane_count);
    debug_assert!(second_sum_im.len() >= lane_count);

    if lane_count == 0
        || sin_theta
            .iter()
            .any(|&sine| !sine.is_finite() || sine < 0.01)
    {
        return Ok(false);
    }

    let pair_count = (lmax - m + 2) / 2;
    let maximum_sum_bound = 2.0 * scalar::BIG * scalar::BIG * pair_count as f64 * 10_000.0;
    if !maximum_sum_bound.is_finite()
        || !prepared_pair_coefficients_are_safe(
            pair_count,
            first_even,
            first_odd,
            second_even,
            second_odd,
        )
    {
        return Ok(false);
    }

    dispatch_synthesis_pair_transition(
        arch,
        m,
        lmax,
        sin_theta,
        cos_theta,
        sectoral_norm,
        coefficient_a,
        coefficient_b,
        PreparedSynthesisPairCoefficients {
            first_even,
            first_odd,
            second_even,
            second_odd,
        },
        previous,
        current,
        next,
        exponents,
        first_sum_re,
        first_sum_im,
        second_sum_re,
        second_sum_im,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The shared checked transition initializes recurrence lanes and four output sums."
)]
fn dispatch_synthesis_pair_transition<C: SynthesisPairCoefficientSource>(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    coefficients: C,
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
    first_sum_re: &mut [f64],
    first_sum_im: &mut [f64],
    second_sum_re: &mut [f64],
    second_sum_im: &mut [f64],
) -> Result<bool, ShtError> {
    let lane_count = sin_theta.len();
    debug_assert_eq!(lane_count, cos_theta.len());
    debug_assert!(lane_count <= LATITUDE_BLOCK_SIZE);
    debug_assert!(previous.len() >= lane_count);
    debug_assert!(current.len() >= lane_count);
    debug_assert!(next.len() >= lane_count);
    debug_assert!(exponents.len() >= lane_count);
    debug_assert!(first_sum_re.len() >= lane_count);
    debug_assert!(first_sum_im.len() >= lane_count);
    debug_assert!(second_sum_re.len() >= lane_count);
    debug_assert!(second_sum_im.len() >= lane_count);

    let completed = arch.dispatch(StagedSynthesisPairTransitionPerVector {
        m,
        lmax,
        sin_theta,
        cos_theta,
        sectoral_norm,
        coefficient_a,
        coefficient_b,
        coefficients,
        previous: &mut previous[..lane_count],
        current: &mut current[..lane_count],
        next: &mut next[..lane_count],
        exponents: &mut exponents[..lane_count],
        first_sum_re: &mut first_sum_re[..lane_count],
        first_sum_im: &mut first_sum_im[..lane_count],
        second_sum_re: &mut second_sum_re[..lane_count],
        second_sum_im: &mut second_sum_im[..lane_count],
    });
    if !completed
        || !finite_prefix_sums(
            &first_sum_re[..lane_count],
            &first_sum_im[..lane_count],
            &second_sum_re[..lane_count],
            &second_sum_im[..lane_count],
        )
    {
        return Ok(false);
    }
    Ok(true)
}

trait SynthesisPairCoefficientSource: Copy {
    fn values(
        self,
        m: usize,
        lmax: usize,
        pair: usize,
    ) -> (Complex64, Complex64, Complex64, Complex64);
}

#[derive(Clone, Copy)]
struct OriginalSynthesisPairCoefficients<'a> {
    epsilon: &'a [f64],
    alpha: &'a [f64],
    first_coefficients: &'a [Complex64],
    second_coefficients: &'a [Complex64],
}

impl SynthesisPairCoefficientSource for OriginalSynthesisPairCoefficients<'_> {
    #[inline(always)]
    fn values(
        self,
        m: usize,
        lmax: usize,
        pair: usize,
    ) -> (Complex64, Complex64, Complex64, Complex64) {
        let l = m + 2 * pair;
        let offset = l - m;
        let next_first_even = if l + 2 <= lmax {
            self.first_coefficients[offset + 2]
        } else {
            Complex64::new(0.0, 0.0)
        };
        let next_second_even = if l + 2 <= lmax {
            self.second_coefficients[offset + 2]
        } else {
            Complex64::new(0.0, 0.0)
        };
        let first_even = (self.first_coefficients[offset] * self.epsilon[l + 1]
            + next_first_even * self.epsilon[l + 2])
            * self.alpha[pair];
        let second_even = (self.second_coefficients[offset] * self.epsilon[l + 1]
            + next_second_even * self.epsilon[l + 2])
            * self.alpha[pair];
        let first_odd = if l < lmax {
            self.first_coefficients[offset + 1] * self.alpha[pair]
        } else {
            Complex64::new(0.0, 0.0)
        };
        let second_odd = if l < lmax {
            self.second_coefficients[offset + 1] * self.alpha[pair]
        } else {
            Complex64::new(0.0, 0.0)
        };
        (first_even, first_odd, second_even, second_odd)
    }
}

#[derive(Clone, Copy)]
struct PreparedSynthesisPairCoefficients<'a> {
    first_even: &'a [Complex64],
    first_odd: &'a [Complex64],
    second_even: &'a [Complex64],
    second_odd: &'a [Complex64],
}

impl SynthesisPairCoefficientSource for PreparedSynthesisPairCoefficients<'_> {
    #[inline(always)]
    fn values(
        self,
        _m: usize,
        _lmax: usize,
        pair: usize,
    ) -> (Complex64, Complex64, Complex64, Complex64) {
        (
            self.first_even[pair],
            self.first_odd[pair],
            self.second_even[pair],
            self.second_odd[pair],
        )
    }
}

fn prepared_pair_coefficients_are_safe(
    pair_count: usize,
    first_even: &[Complex64],
    first_odd: &[Complex64],
    second_even: &[Complex64],
    second_odd: &[Complex64],
) -> bool {
    first_even.len() >= pair_count
        && first_odd.len() >= pair_count
        && second_even.len() >= pair_count
        && second_odd.len() >= pair_count
        && (0..pair_count).all(|pair| {
            complex_components_in_normal_range(first_even[pair])
                && complex_components_in_normal_range(first_odd[pair])
                && complex_components_in_normal_range(second_even[pair])
                && complex_components_in_normal_range(second_odd[pair])
        })
}

/// Runs the checked prefix independently for each SIMD latitude vector so a
/// vector that reaches ordinary range can enter the register-resident suffix
/// without waiting for unrelated scaled lanes elsewhere in the block.
struct StagedSynthesisPairTransitionPerVector<'a, C> {
    m: usize,
    lmax: usize,
    sin_theta: &'a [f64],
    cos_theta: &'a [f64],
    sectoral_norm: f64,
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    coefficients: C,
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
    exponents: &'a mut [i64],
    first_sum_re: &'a mut [f64],
    first_sum_im: &'a mut [f64],
    second_sum_re: &'a mut [f64],
    second_sum_im: &'a mut [f64],
}

impl<C: SynthesisPairCoefficientSource> WithSimd for StagedSynthesisPairTransitionPerVector<'_, C> {
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> bool {
        let Self {
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            coefficients,
            previous,
            current,
            next,
            exponents,
            first_sum_re,
            first_sum_im,
            second_sum_re,
            second_sum_im,
        } = self;
        let vector_lanes = S::F64_LANES.max(1);
        for start in (0..sin_theta.len()).step_by(vector_lanes) {
            let end = (start + vector_lanes).min(sin_theta.len());
            let completed = StagedSynthesisPairTransition {
                m,
                lmax,
                sin_theta: &sin_theta[start..end],
                cos_theta: &cos_theta[start..end],
                sectoral_norm,
                coefficient_a,
                coefficient_b,
                coefficients,
                previous: &mut previous[start..end],
                current: &mut current[start..end],
                next: &mut next[start..end],
                exponents: &mut exponents[start..end],
                first_sum_re: &mut first_sum_re[start..end],
                first_sum_im: &mut first_sum_im[start..end],
                second_sum_re: &mut second_sum_re[start..end],
                second_sum_im: &mut second_sum_im[start..end],
            }
            .with_simd(simd);
            if !completed {
                return false;
            }
        }
        true
    }
}

#[inline]
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

#[expect(clippy::too_many_arguments, reason = "SIMD helper carries separate lane accumulators")]
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

fn paired_scalar_coefficients_are_safe(
    m: usize,
    lmax: usize,
    epsilon: &[f64],
    alpha: &[f64],
    first_coefficients: &[Complex64],
    second_coefficients: &[Complex64],
) -> bool {
    let pair_count = (lmax - m + 2) / 2;
    debug_assert!(alpha.len() >= pair_count);
    for (pair, &pair_alpha) in alpha.iter().take(pair_count).enumerate() {
        let l = m + 2 * pair;
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
            * pair_alpha;
        let second_even = (second_coefficients[offset] * epsilon[l + 1]
            + next_second_even * epsilon[l + 2])
            * pair_alpha;
        let first_odd = if l < lmax {
            first_coefficients[offset + 1] * pair_alpha
        } else {
            Complex64::new(0.0, 0.0)
        };
        let second_odd = if l < lmax {
            second_coefficients[offset + 1] * pair_alpha
        } else {
            Complex64::new(0.0, 0.0)
        };
        if !complex_components_in_normal_range(first_even)
            || !complex_components_in_normal_range(second_even)
            || !complex_components_in_normal_range(first_odd)
            || !complex_components_in_normal_range(second_odd)
        {
            return false;
        }
    }
    true
}

fn complex_components_in_normal_range(value: Complex64) -> bool {
    scalar_value_in_normal_range(value.re) && scalar_value_in_normal_range(value.im)
}

struct StagedSynthesisPairTransition<'a, C> {
    m: usize,
    lmax: usize,
    sin_theta: &'a [f64],
    cos_theta: &'a [f64],
    sectoral_norm: f64,
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    coefficients: C,
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
    exponents: &'a mut [i64],
    first_sum_re: &'a mut [f64],
    first_sum_im: &'a mut [f64],
    second_sum_re: &'a mut [f64],
    second_sum_im: &'a mut [f64],
}

#[inline(never)]
fn normalize_synthesis_pair_lanes_simd<S: Simd>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    exponents: &mut [i64],
) -> Option<bool> {
    scalar::normalize_pair_lanes_simd(simd, previous, current, exponents)
}

impl<C: SynthesisPairCoefficientSource> WithSimd for StagedSynthesisPairTransition<'_, C> {
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> bool {
        let StagedSynthesisPairTransition {
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            coefficients,
            previous,
            current,
            next,
            exponents,
            first_sum_re,
            first_sum_im,
            second_sum_re,
            second_sum_im,
        } = self;
        let mut previous = previous;
        let mut current = current;
        let mut next = next;
        for lane in 0..sin_theta.len() {
            let (value, exponent) = match scalar::sectoral_value(sin_theta[lane], m, sectoral_norm)
            {
                Ok(value) => value,
                Err(_) => return false,
            };
            previous[lane] = 0.0;
            current[lane] = value;
            exponents[lane] = exponent;
            first_sum_re[lane] = 0.0;
            first_sum_im[lane] = 0.0;
            second_sum_re[lane] = 0.0;
            second_sum_im[lane] = 0.0;
        }
        let Some(mut normal_range) =
            normalize_synthesis_pair_lanes_simd(simd, previous, current, exponents)
        else {
            return false;
        };
        if normal_range
            && lane_values_in_normal_range(previous)
            && lane_values_in_normal_range(current)
        {
            return FusedSynthesisPairBlock {
                m,
                lmax,
                pair_start: 0,
                cos_theta,
                coefficient_a,
                coefficient_b,
                coefficients,
                previous,
                current,
                first_sum_re,
                first_sum_im,
                second_sum_re,
                second_sum_im,
                initialize_sums: false,
                check_normal_range: true,
            }
            .with_simd(simd);
        }

        let pair_count = (lmax - m + 2) / 2;
        for pair in 0..pair_count {
            let l = m + 2 * pair;
            let (first_even, first_odd, second_even, second_odd) =
                coefficients.values(m, lmax, pair);
            if !try_accumulate_scaled_pair_vector(
                simd,
                cos_theta,
                current,
                exponents,
                first_even,
                first_odd,
                second_even,
                second_odd,
                first_sum_re,
                first_sum_im,
                second_sum_re,
                second_sum_im,
            ) {
                for lane in 0..current.len() {
                    let cosine = cos_theta[lane];
                    let first_odd_factor =
                        Complex64::new(first_odd.re * cosine, first_odd.im * cosine);
                    let second_odd_factor =
                        Complex64::new(second_odd.re * cosine, second_odd.im * cosine);
                    let first_factor = Complex64::new(
                        first_odd.re.mul_add(cosine, first_even.re),
                        first_odd.im.mul_add(cosine, first_even.im),
                    );
                    let second_factor = Complex64::new(
                        second_odd.re.mul_add(cosine, second_even.re),
                        second_odd.im.mul_add(cosine, second_even.im),
                    );
                    if !complex_sum_is_normal_or_zero(first_factor, first_even, first_odd_factor)
                        || !complex_sum_is_normal_or_zero(
                            second_factor,
                            second_even,
                            second_odd_factor,
                        )
                    {
                        return false;
                    }
                    let first = scalar::multiply_complex_scaled(
                        first_factor,
                        current[lane],
                        exponents[lane],
                    );
                    let second = scalar::multiply_complex_scaled(
                        second_factor,
                        current[lane],
                        exponents[lane],
                    );
                    if !complex_scaled_product_is_normal_or_zero(
                        first,
                        first_factor,
                        current[lane],
                        1.0,
                    ) || !complex_scaled_product_is_normal_or_zero(
                        second,
                        second_factor,
                        current[lane],
                        1.0,
                    ) {
                        return false;
                    }
                    let first_sum = Complex64::new(first_sum_re[lane], first_sum_im[lane]);
                    let second_sum = Complex64::new(second_sum_re[lane], second_sum_im[lane]);
                    let next_first_sum = first_sum + first;
                    let next_second_sum = second_sum + second;
                    if !complex_sum_is_normal_or_zero(next_first_sum, first_sum, first)
                        || !complex_sum_is_normal_or_zero(next_second_sum, second_sum, second)
                    {
                        return false;
                    }
                    first_sum_re[lane] = next_first_sum.re;
                    first_sum_im[lane] = next_first_sum.im;
                    second_sum_re[lane] = next_second_sum.re;
                    second_sum_im[lane] = next_second_sum.im;
                }
            }
            if l + 2 > lmax {
                break;
            }

            let (current_vectors, current_tail) = S::as_simd_f64s(current);
            let (previous_vectors, previous_tail) = S::as_simd_f64s(previous);
            let (cos_vectors, cos_tail) = S::as_simd_f64s(cos_theta);
            let (next_vectors, next_tail) = S::as_mut_simd_f64s(next);
            let a = simd.splat_f64s(coefficient_a[pair]);
            let b = simd.splat_f64s(coefficient_b[pair]);
            for index in 0..current_vectors.len() {
                let cosine_squared = simd.mul_f64s(cos_vectors[index], cos_vectors[index]);
                let leading = simd.mul_add_f64s(a, cosine_squared, b);
                next_vectors[index] =
                    simd.mul_add_f64s(leading, current_vectors[index], previous_vectors[index]);
            }
            for index in 0..current_tail.len() {
                let cosine_squared = cos_tail[index] * cos_tail[index];
                let leading = coefficient_a[pair].mul_add(cosine_squared, coefficient_b[pair]);
                next_tail[index] = leading.mul_add(current_tail[index], previous_tail[index]);
            }
            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
            if normal_range {
                if scalar::pair_lanes_need_scaling(simd, previous, current) {
                    let Some(ready) =
                        normalize_synthesis_pair_lanes_simd(simd, previous, current, exponents)
                    else {
                        return false;
                    };
                    normal_range = ready;
                }
            } else {
                let Some(ready) =
                    normalize_synthesis_pair_lanes_simd(simd, previous, current, exponents)
                else {
                    return false;
                };
                normal_range = ready;
            }
            if normal_range
                && lane_values_in_normal_range(previous)
                && lane_values_in_normal_range(current)
            {
                if !finite_prefix_sums(first_sum_re, first_sum_im, second_sum_re, second_sum_im) {
                    return false;
                }
                return FusedSynthesisPairBlock {
                    m,
                    lmax,
                    pair_start: pair + 1,
                    cos_theta,
                    coefficient_a,
                    coefficient_b,
                    coefficients,
                    previous,
                    current,
                    first_sum_re,
                    first_sum_im,
                    second_sum_re,
                    second_sum_im,
                    initialize_sums: false,
                    check_normal_range: true,
                }
                .with_simd(simd);
            }
        }
        false
    }
}

/// Accumulates a paired scalar analysis block across its latitude lanes.
///
/// Normal recurrence lanes use vector products and horizontal reductions.
/// Lanes carrying significant binary scale exponents retain the checked scalar
/// path; the safeguarded GL analysis route omits negative epochs under its
/// latitude-wide error budget.
#[expect(
    clippy::too_many_arguments,
    reason = "The analysis kernel receives parity spectra and recurrence lanes explicitly."
)]
pub(crate) fn accumulate_analysis_pair_dispatched(
    arch: Arch,
    products_safe: bool,
    normal_range: bool,
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &[f64],
    opposite_imag: &[f64],
    basis: &[f64],
    exponents: &[i64],
    cos_theta: &[f64],
) -> (Complex64, Complex64) {
    debug_assert_eq!(same_real.len(), basis.len());
    debug_assert_eq!(same_imag.len(), basis.len());
    debug_assert_eq!(opposite_real.len(), basis.len());
    debug_assert_eq!(opposite_imag.len(), basis.len());
    debug_assert_eq!(exponents.len(), basis.len());
    debug_assert_eq!(cos_theta.len(), basis.len());

    if products_safe {
        if normal_range {
            debug_assert!(exponents.iter().all(|&exponent| exponent == 0));
            return arch.dispatch(AnalysisPairAccumulation::<false> {
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                basis,
                cos_theta,
            });
        }
        if exponents.contains(&0) {
            return arch.dispatch(MixedAnalysisPairAccumulation::<false> {
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                basis,
                exponents,
                cos_theta,
                allow_insignificant_scaled_lanes: false,
            });
        }
    }

    let mut same = Complex64::new(0.0, 0.0);
    let mut opposite = Complex64::new(0.0, 0.0);
    for lane in 0..basis.len() {
        same += scalar::multiply_complex_scaled(
            Complex64::new(same_real[lane], same_imag[lane]),
            basis[lane],
            exponents[lane],
        );
        opposite += scalar::multiply_complex_scaled_product(
            Complex64::new(opposite_real[lane], opposite_imag[lane]),
            basis[lane],
            cos_theta[lane],
            exponents[lane],
        );
    }
    (same, opposite)
}

/// Accumulates two scalar-conversion channels in one SIMD pass when their
/// recurrence and products are in the normal range.
#[expect(
    clippy::too_many_arguments,
    reason = "The two analysis channels share the same basis, exponent and latitude vectors."
)]
pub(crate) fn accumulate_analysis_pair_channels_dispatched(
    arch: Arch,
    products_safe: [bool; 2],
    normal_range: bool,
    same_real: [&[f64]; 2],
    same_imag: [&[f64]; 2],
    opposite_real: [&[f64]; 2],
    opposite_imag: [&[f64]; 2],
    basis: &[f64],
    exponents: &[i64],
    cos_theta: &[f64],
) -> [(Complex64, Complex64); 2] {
    if products_safe == [true, true] && normal_range {
        debug_assert!(exponents.iter().all(|&exponent| exponent == 0));
        return arch.dispatch(AnalysisPairChannelsAccumulation {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis,
            cos_theta,
        });
    }

    if products_safe == [true, true] && !normal_range && exponents.contains(&0) {
        return arch.dispatch(MixedAnalysisPairChannelsAccumulation {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis,
            exponents,
            cos_theta,
        });
    }

    std::array::from_fn(|channel| {
        accumulate_analysis_pair_dispatched(
            arch,
            products_safe[channel],
            normal_range,
            same_real[channel],
            same_imag[channel],
            opposite_real[channel],
            opposite_imag[channel],
            basis,
            exponents,
            cos_theta,
        )
    })
}

/// Checks whether an analysis block can safely use ordinary SIMD accumulation.
pub(crate) fn analysis_products_safe(
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &[f64],
    opposite_imag: &[f64],
    cos_theta: &[f64],
) -> bool {
    debug_assert_eq!(same_real.len(), same_imag.len());
    debug_assert_eq!(same_real.len(), opposite_real.len());
    debug_assert_eq!(same_real.len(), opposite_imag.len());
    debug_assert_eq!(same_real.len(), cos_theta.len());
    // Bound the absolute sum as well as each product. This keeps both the
    // vector reduction and accumulation across previous latitude tiles finite.
    let max_input = f64::MAX / scalar::BIG / (2.0 * same_real.len().max(1) as f64);
    same_real
        .iter()
        .chain(same_imag)
        .chain(opposite_real)
        .chain(opposite_imag)
        .all(|value| (value.is_normal() || *value == 0.0) && value.abs() <= max_input)
        && cos_theta
            .iter()
            .all(|value| value.is_finite() && value.abs() <= 1.0)
}

/// Checks the ordinary-range accumulation bound while applying the opposite
/// parity cosine weight. A failed result may leave earlier lanes weighted;
/// callers restore the packed opposite channels from the original spectra.
pub(crate) fn prepare_analysis_products_and_opposite(
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &mut [f64],
    opposite_imag: &mut [f64],
    cos_theta: &[f64],
) -> (bool, bool) {
    debug_assert_eq!(same_real.len(), same_imag.len());
    debug_assert_eq!(same_real.len(), opposite_real.len());
    debug_assert_eq!(same_real.len(), opposite_imag.len());
    debug_assert_eq!(same_real.len(), cos_theta.len());
    let max_input = f64::MAX / scalar::BIG / (2.0 * same_real.len().max(1) as f64);
    let mut products_safe = true;
    let mut opposite_safe = true;

    for lane in 0..same_real.len() {
        let same_re = same_real[lane];
        let same_im = same_imag[lane];
        let opposite_re = opposite_real[lane];
        let opposite_im = opposite_imag[lane];
        let cosine = cos_theta[lane];
        let lane_inputs_safe = [same_re, same_im, opposite_re, opposite_im]
            .into_iter()
            .all(|value| (value.is_normal() || value == 0.0) && value.abs() <= max_input)
            && cosine.is_finite()
            && cosine.abs() <= 1.0;
        if !lane_inputs_safe {
            products_safe = false;
            opposite_safe = false;
            continue;
        }

        let weighted_real = opposite_re * cosine;
        let weighted_imag = opposite_im * cosine;
        let real_safe = weighted_real.is_normal()
            || (weighted_real == 0.0 && (opposite_re == 0.0 || cosine == 0.0));
        let imag_safe = weighted_imag.is_normal()
            || (weighted_imag == 0.0 && (opposite_im == 0.0 || cosine == 0.0));
        if real_safe && imag_safe {
            opposite_real[lane] = weighted_real;
            opposite_imag[lane] = weighted_imag;
        } else {
            opposite_safe = false;
        }
    }

    (products_safe, products_safe && opposite_safe)
}

/// Applies DUCC's cosine weight to the opposite channels while checking that
/// the transformation does not introduce subnormal products.
///
/// A `false` result can leave part of the opposite channels weighted; callers
/// must restore them from the original spectra before using the scaled path.
pub(crate) fn prepare_analysis_opposite(
    opposite_real: &mut [f64],
    opposite_imag: &mut [f64],
    cos_theta: &[f64],
) -> bool {
    debug_assert_eq!(opposite_real.len(), opposite_imag.len());
    debug_assert_eq!(opposite_real.len(), cos_theta.len());
    for ((real, imag), &cosine) in opposite_real.iter_mut().zip(opposite_imag).zip(cos_theta) {
        let weighted_real = *real * cosine;
        let weighted_imag = *imag * cosine;
        let real_safe =
            weighted_real.is_normal() || (weighted_real == 0.0 && (*real == 0.0 || cosine == 0.0));
        let imag_safe =
            weighted_imag.is_normal() || (weighted_imag == 0.0 && (*imag == 0.0 || cosine == 0.0));
        if !real_safe || !imag_safe {
            return false;
        }
        *real = weighted_real;
        *imag = weighted_imag;
    }
    true
}

/// Initializes ordinary-range sectoral values across SIMD latitude lanes.
///
/// Returns `false` when any lane needs the existing scaled scalar path. The
/// candidate values are checked before the recurrence consumes them.
fn initialize_normal_analysis_sectoral_dispatched(
    arch: Arch,
    m: usize,
    sin_theta: &[f64],
    sectoral_norm: f64,
    current: &mut [f64],
) -> Result<NormalSectoralInitialization, ShtError> {
    arch.dispatch(InitializeNormalAnalysisSectoral {
        m,
        sin_theta,
        sectoral_norm,
        current,
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "The shared initializer receives the order geometry and caller-owned recurrence lanes explicitly."
)]
fn initialize_analysis_sectoral_lanes(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    sectoral_norm: f64,
    previous: &mut [f64],
    current: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    debug_assert!(previous.len() >= sin_theta.len());
    debug_assert!(current.len() >= sin_theta.len());
    debug_assert!(exponents.len() >= sin_theta.len());
    previous[..sin_theta.len()].fill(0.0);

    let initialization = if lmax >= SIMD_SECTORAL_INIT_MIN_LMAX {
        initialize_normal_analysis_sectoral_dispatched(
            arch,
            m,
            sin_theta,
            sectoral_norm,
            &mut current[..sin_theta.len()],
        )?
    } else {
        NormalSectoralInitialization::default()
    };
    if initialization.complete {
        exponents[..sin_theta.len()].fill(0);
        return Ok(true);
    }

    for lane in 0..sin_theta.len() {
        let (value, exponent) = scalar::sectoral_value(sin_theta[lane], m, sectoral_norm)?;
        current[lane] = value;
        exponents[lane] = exponent;
    }
    scalar::normalize_pair_lanes(
        &mut previous[..sin_theta.len()],
        &mut current[..sin_theta.len()],
        &mut exponents[..sin_theta.len()],
    )
}

/// Initializes the analysis recurrence with DUCC's `mypow` scale epochs.
/// The ordinary initializer remains the cheap path when every seed is already
/// in RustSHT's normal range; this route handles mixed and underflowing seeds
/// without falling back to one scalar power per latitude lane.
#[expect(
    clippy::too_many_arguments,
    reason = "The scaled initializer writes caller-owned recurrence lanes and exponent state."
)]
fn initialize_scaled_analysis_sectoral_lanes(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    sectoral_norm: f64,
    previous: &mut [f64],
    current: &mut [f64],
    scale_epochs: &mut [f64],
) -> Result<Option<bool>, ShtError> {
    debug_assert!(previous.len() >= sin_theta.len());
    debug_assert!(current.len() >= sin_theta.len());
    debug_assert!(scale_epochs.len() >= sin_theta.len());
    previous[..sin_theta.len()].fill(0.0);

    let initialization = if lmax >= SIMD_SECTORAL_INIT_MIN_LMAX {
        initialize_normal_analysis_sectoral_dispatched(
            arch,
            m,
            sin_theta,
            sectoral_norm,
            &mut current[..sin_theta.len()],
        )?
    } else {
        NormalSectoralInitialization::default()
    };
    if initialization.complete {
        return Ok(Some(true));
    }

    let initialized_scaled = arch.dispatch(InitializeScaledAnalysisSectoral {
        m,
        sin_theta,
        sectoral_norm,
        normal_prefix_vectors: initialization.completed_vectors,
        current: &mut current[..sin_theta.len()],
        scale_epochs: &mut scale_epochs[..sin_theta.len()],
    });
    if initialized_scaled.is_some() {
        return Ok(Some(false));
    }

    // An invalid or unrepresentable SIMD seed uses the robust order fallback.
    Ok(None)
}

#[derive(Clone, Copy, Default)]
struct NormalSectoralInitialization {
    complete: bool,
    completed_vectors: usize,
}

struct InitializeScaledAnalysisSectoral<'a> {
    m: usize,
    sin_theta: &'a [f64],
    sectoral_norm: f64,
    normal_prefix_vectors: usize,
    current: &'a mut [f64],
    scale_epochs: &'a mut [f64],
}

impl WithSimd for InitializeScaledAnalysisSectoral<'_> {
    type Output = Option<()>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let InitializeScaledAnalysisSectoral {
            m,
            sin_theta,
            sectoral_norm,
            normal_prefix_vectors,
            current,
            scale_epochs,
        } = self;
        if !sectoral_norm.is_finite() || sectoral_norm <= 0.0 {
            return None;
        }
        let (sine_vectors, sine_tail) = S::as_simd_f64s(sin_theta);
        let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
        let (scale_vectors, scale_tail) = S::as_mut_simd_f64s(scale_epochs);
        if current_vectors.len() != sine_vectors.len()
            || current_tail.len() != sine_tail.len()
            || scale_vectors.len() != sine_vectors.len()
            || scale_tail.len() != sine_tail.len()
            || normal_prefix_vectors > sine_vectors.len()
        {
            return None;
        }

        let one = simd.splat_f64s(1.0);
        let zero_scale = simd.splat_f64s(0.0);
        let maximum_finite = simd.splat_f64s(f64::MAX);
        let sine_limit = simd.splat_f64s(1.0);
        let power_limit = simd.splat_f64s(analysis_sectoral_power_limit(m));
        let coefficient =
            simd.splat_f64s(sectoral_norm * if m.is_multiple_of(2) { 1.0 } else { -1.0 });

        for scale in &mut scale_vectors[..normal_prefix_vectors] {
            *scale = zero_scale;
        }
        for index in normal_prefix_vectors..sine_vectors.len() {
            let sine = sine_vectors[index];
            let sine_magnitude = simd.abs_f64s(sine);
            if !analysis_mask_all(
                simd,
                simd.and_m64s(
                    simd.less_than_or_equal_f64s(sine_magnitude, sine_limit),
                    simd.less_than_or_equal_f64s(sine_magnitude, maximum_finite),
                ),
            ) {
                return None;
            }

            // DUCC's mypow uses an unscaled exponentiation for a SIMD vector
            // when every lane is above its per-order underflow limit. Keep
            // that quick route for safe vectors even when another vector in
            // this block needs scaled initialization.
            if analysis_mask_all(
                simd,
                simd.greater_than_or_equal_f64s(sine_magnitude, power_limit),
            ) {
                let mut value = simd.mul_f64s(integer_power_vector(simd, sine, m), coefficient);
                let mut value_scale = zero_scale;
                if !normalize_analysis_power_vector(
                    simd,
                    &mut value,
                    &mut value_scale,
                    ANALYSIS_FTOL,
                ) {
                    return None;
                }
                current_vectors[index] = value;
                scale_vectors[index] = value_scale;
                continue;
            }

            let mut base = sine;
            let mut base_scale = zero_scale;
            if !normalize_analysis_power_vector(simd, &mut base, &mut base_scale, ANALYSIS_FBIGHALF)
            {
                return None;
            }

            let mut value = one;
            let mut value_scale = zero_scale;
            let mut remaining = m;
            while remaining != 0 {
                if remaining & 1 != 0 {
                    value = simd.mul_f64s(value, base);
                    value_scale = simd.add_f64s(value_scale, base_scale);
                    if !normalize_analysis_power_vector(
                        simd,
                        &mut value,
                        &mut value_scale,
                        ANALYSIS_FBIGHALF,
                    ) {
                        return None;
                    }
                }
                remaining >>= 1;
                if remaining != 0 {
                    base = simd.mul_f64s(base, base);
                    base_scale = simd.add_f64s(base_scale, base_scale);
                    if !normalize_analysis_power_vector(
                        simd,
                        &mut base,
                        &mut base_scale,
                        ANALYSIS_FBIGHALF,
                    ) {
                        return None;
                    }
                }
            }

            value = simd.mul_f64s(value, coefficient);
            if !normalize_analysis_power_vector(
                simd,
                &mut value,
                &mut value_scale,
                ANALYSIS_FBIGHALF,
            ) || !normalize_analysis_power_vector(
                simd,
                &mut value,
                &mut value_scale,
                ANALYSIS_FTOL,
            ) {
                return None;
            }
            current_vectors[index] = value;
            scale_vectors[index] = value_scale;
        }

        for lane in 0..sine_tail.len() {
            let sine = sine_tail[lane];
            if !sine.is_finite() || sine.abs() > 1.0 {
                return None;
            }
            let Ok((value, exponent)) = scalar::sectoral_value(sine, m, sectoral_norm) else {
                return None;
            };
            let (value, exponent) = normalize_analysis_seed_scalar(value, exponent)?;
            let scale = exponent as f64 / ANALYSIS_SCALE_STEP as f64;
            if !scale.is_finite() {
                return None;
            }
            current_tail[lane] = value;
            scale_tail[lane] = scale;
        }
        Some(())
    }
}

#[inline]
fn analysis_sectoral_power_limit(m: usize) -> f64 {
    if m == 0 {
        0.0
    } else {
        (-400.0 * std::f64::consts::LN_2 / m as f64).exp()
    }
}

#[inline(always)]
fn normalize_analysis_power_vector<S: Simd>(
    simd: S,
    value: &mut S::f64s,
    scale: &mut S::f64s,
    maximum_value: f64,
) -> bool {
    let maximum = simd.splat_f64s(maximum_value);
    let minimum = simd.splat_f64s(maximum_value * ANALYSIS_SCALE_DOWN);
    let maximum_finite = simd.splat_f64s(f64::MAX);
    let zero = simd.splat_f64s(0.0);
    let one = simd.splat_f64s(1.0);

    let mut magnitude = simd.abs_f64s(*value);
    if !analysis_mask_all(
        simd,
        simd.less_than_or_equal_f64s(magnitude, maximum_finite),
    ) {
        return false;
    }
    let too_large = simd.greater_than_f64s(magnitude, maximum);
    let nonzero = simd.not_m64s(simd.equal_f64s(magnitude, zero));
    let too_small = simd.and_m64s(nonzero, simd.less_than_f64s(magnitude, minimum));
    if !analysis_mask_any(simd, simd.or_m64s(too_large, too_small)) {
        return true;
    }
    loop {
        let too_large = simd.greater_than_f64s(magnitude, maximum);
        if !analysis_mask_any(simd, too_large) {
            break;
        }
        let next_scale = simd.add_f64s(*scale, one);
        if !analysis_mask_all(
            simd,
            simd.less_than_or_equal_f64s(simd.abs_f64s(next_scale), simd.splat_f64s(4.5e15)),
        ) {
            return false;
        }
        *value = simd.select_f64s(
            too_large,
            simd.mul_f64s(*value, simd.splat_f64s(ANALYSIS_SCALE_DOWN)),
            *value,
        );
        *scale = simd.select_f64s(too_large, next_scale, *scale);
        magnitude = simd.abs_f64s(*value);
    }
    loop {
        let nonzero = simd.not_m64s(simd.equal_f64s(magnitude, zero));
        let too_small = simd.and_m64s(nonzero, simd.less_than_f64s(magnitude, minimum));
        if !analysis_mask_any(simd, too_small) {
            break;
        }
        let next_scale = simd.sub_f64s(*scale, one);
        if !analysis_mask_all(
            simd,
            simd.less_than_or_equal_f64s(simd.abs_f64s(next_scale), simd.splat_f64s(4.5e15)),
        ) {
            return false;
        }
        *value = simd.select_f64s(
            too_small,
            simd.mul_f64s(*value, simd.splat_f64s(ANALYSIS_SCALE_UP)),
            *value,
        );
        *scale = simd.select_f64s(too_small, next_scale, *scale);
        magnitude = simd.abs_f64s(*value);
    }
    true
}

fn normalize_analysis_seed_scalar(mut value: f64, mut exponent: i64) -> Option<(f64, i64)> {
    if !value.is_finite() {
        return None;
    }
    let minimum = ANALYSIS_FTOL * ANALYSIS_SCALE_DOWN;
    while value.abs() > ANALYSIS_FTOL {
        value *= ANALYSIS_SCALE_DOWN;
        exponent = exponent.checked_add(ANALYSIS_SCALE_STEP)?;
    }
    while value != 0.0 && value.abs() < minimum {
        value *= ANALYSIS_SCALE_UP;
        exponent = exponent.checked_sub(ANALYSIS_SCALE_STEP)?;
    }
    Some((value, exponent))
}

struct InitializeNormalAnalysisSectoral<'a> {
    m: usize,
    sin_theta: &'a [f64],
    sectoral_norm: f64,
    current: &'a mut [f64],
}

impl WithSimd for InitializeNormalAnalysisSectoral<'_> {
    type Output = Result<NormalSectoralInitialization, ShtError>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let InitializeNormalAnalysisSectoral {
            m,
            sin_theta,
            sectoral_norm,
            current,
        } = self;
        debug_assert_eq!(sin_theta.len(), current.len());

        if !sectoral_norm.is_finite() || sectoral_norm <= 0.0 {
            return Ok(NormalSectoralInitialization::default());
        }
        let (sine_vectors, sine_tail) = S::as_simd_f64s(sin_theta);
        let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
        debug_assert_eq!(sine_vectors.len(), current_vectors.len());
        debug_assert_eq!(sine_tail.len(), current_tail.len());

        let one = simd.splat_f64s(1.0);
        let zero = simd.splat_f64s(0.0);
        let sine_max = simd.splat_f64s(1.0);
        let sign = if m.is_multiple_of(2) { 1.0 } else { -1.0 };
        let factor = simd.splat_f64s(sectoral_norm * sign);
        let minimum = simd.splat_f64s(scalar::SMALL);
        let maximum = simd.splat_f64s(scalar::BIG);
        for (index, (output, &sine)) in current_vectors.iter_mut().zip(sine_vectors).enumerate() {
            let value = simd.mul_f64s(integer_power_vector(simd, sine, m), factor);
            let magnitude = simd.abs_f64s(value);
            let zero_result_is_valid = simd.and_m64s(
                simd.equal_f64s(magnitude, zero),
                simd.equal_f64s(simd.abs_f64s(sine), zero),
            );
            let above_minimum = simd.or_m64s(
                simd.greater_than_or_equal_f64s(magnitude, minimum),
                zero_result_is_valid,
            );
            let in_range = simd.and_m64s(
                simd.and_m64s(
                    above_minimum,
                    simd.less_than_or_equal_f64s(magnitude, maximum),
                ),
                simd.less_than_or_equal_f64s(simd.abs_f64s(sine), sine_max),
            );
            if simd.reduce_min_f64s(simd.select_f64s(in_range, one, zero)) != 1.0 {
                return Ok(NormalSectoralInitialization {
                    complete: false,
                    completed_vectors: index,
                });
            }
            *output = value;
        }
        for (&sine, output) in sine_tail.iter().zip(current_tail) {
            if !sine.is_finite() || sine.abs() > 1.0 {
                return Ok(NormalSectoralInitialization {
                    complete: false,
                    completed_vectors: sine_vectors.len(),
                });
            }
            let (value, exponent) = scalar::sectoral_value(sine, m, sectoral_norm)?;
            if exponent != 0 || !scalar::scalar_value_in_normal_range(value) {
                return Ok(NormalSectoralInitialization {
                    complete: false,
                    completed_vectors: sine_vectors.len(),
                });
            }
            *output = value;
        }
        Ok(NormalSectoralInitialization {
            complete: true,
            completed_vectors: sine_vectors.len(),
        })
    }
}

#[inline(always)]
fn integer_power_vector<S: Simd>(simd: S, mut base: S::f64s, mut exponent: usize) -> S::f64s {
    let mut result = simd.splat_f64s(1.0);
    while exponent > 0 {
        if exponent & 1 != 0 {
            result = simd.mul_f64s(result, base);
        }
        exponent >>= 1;
        if exponent > 0 {
            base = simd.mul_f64s(base, base);
        }
    }
    result
}

/// Computes a scalar analysis block, transitioning scaled lanes to the
/// ordinary SIMD recurrence when the complete active block is in range.
///
/// Returns `false` when a later ordinary recurrence step leaves the safe range.
/// Staged coefficients may then be partial and must be discarded by the caller.
#[expect(
    clippy::too_many_arguments,
    reason = "The fused scalar analysis block receives parity spectra and worker-owned recurrence scratch explicitly."
)]
#[cfg(test)]
#[allow(unexpected_cfgs)]
pub(crate) fn fused_scalar_analysis_pair_block_dispatched(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &[f64],
    opposite_imag: &[f64],
    staged_coefficients: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    let mut cos_theta_squared = [0.0; LATITUDE_BLOCK_SIZE];
    for ((squared, &sine), &cosine) in cos_theta_squared.iter_mut().zip(sin_theta).zip(cos_theta) {
        *squared = if cosine.abs() > 0.99 {
            (1.0 - sine) * (1.0 + sine)
        } else {
            cosine * cosine
        };
    }
    let mut range_check_was_exercised = false;
    fused_scalar_analysis_pair_block_impl::<false, { !cfg!(rustsht_unchecked_analysis_pair_range) }>(
        arch,
        m,
        lmax,
        false,
        &mut range_check_was_exercised,
        sin_theta,
        cos_theta,
        &cos_theta_squared[..sin_theta.len()],
        sectoral_norm,
        coefficient_a,
        coefficient_b,
        same_real,
        same_imag,
        opposite_real,
        opposite_imag,
        staged_coefficients,
        previous,
        current,
        next,
        &mut [],
        &mut [],
        exponents,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The preweighted block receives active geometry, spectra and worker scratch explicitly."
)]
/// Runs a preweighted scalar-analysis block.
///
/// `range_check_is_proven_safe` may disable the ordinary recurrence group scan
/// only after this block completed checked validation for the same order, plan,
/// latitude geometry, prefix mode and SIMD backend. Input-product checks remain
/// the caller's responsibility.
/// `range_check_was_exercised` reports whether the checked ordinary recurrence
/// group scan ran for this block, including after scaled-to-ordinary promotion.
#[allow(unexpected_cfgs)]
pub(crate) fn fused_scalar_analysis_pair_block_preweighted_dispatched(
    arch: Arch,
    m: usize,
    lmax: usize,
    allow_insignificant_prefix: bool,
    range_check_is_proven_safe: bool,
    range_check_was_exercised: &mut bool,
    sin_theta: &[f64],
    cos_theta: &[f64],
    cos_theta_squared: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &[f64],
    opposite_imag: &[f64],
    staged_coefficients: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    scale_epochs: &mut [f64],
    scale_corrections: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    *range_check_was_exercised = false;
    if range_check_is_proven_safe {
        fused_scalar_analysis_pair_block_impl::<true, false>(
            arch,
            m,
            lmax,
            allow_insignificant_prefix,
            range_check_was_exercised,
            sin_theta,
            cos_theta,
            cos_theta_squared,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            previous,
            current,
            next,
            scale_epochs,
            scale_corrections,
            exponents,
        )
    } else {
        fused_scalar_analysis_pair_block_impl::<
            true,
            { !cfg!(rustsht_unchecked_analysis_pair_range) },
        >(
            arch,
            m,
            lmax,
            allow_insignificant_prefix,
            range_check_was_exercised,
            sin_theta,
            cos_theta,
            cos_theta_squared,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            previous,
            current,
            next,
            scale_epochs,
            scale_corrections,
            exponents,
        )
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The scalar analysis block shares caller-owned coefficients, spectra and recurrence scratch."
)]
fn fused_scalar_analysis_pair_block_impl<
    const OPPOSITE_COSINE_WEIGHTED: bool,
    const CHECK_PAIR_GROUP_RANGE: bool,
>(
    arch: Arch,
    m: usize,
    lmax: usize,
    allow_insignificant_prefix: bool,
    range_check_was_exercised: &mut bool,
    sin_theta: &[f64],
    cos_theta: &[f64],
    cos_theta_squared: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &[f64],
    opposite_imag: &[f64],
    staged_coefficients: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    scale_epochs: &mut [f64],
    scale_corrections: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    debug_assert!(m <= lmax);
    let coefficient_count = lmax - m + 1;
    debug_assert!(sin_theta.len() <= LATITUDE_BLOCK_SIZE);
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    debug_assert_eq!(sin_theta.len(), cos_theta_squared.len());
    debug_assert_eq!(sin_theta.len(), same_real.len());
    debug_assert_eq!(sin_theta.len(), same_imag.len());
    debug_assert_eq!(sin_theta.len(), opposite_real.len());
    debug_assert_eq!(sin_theta.len(), opposite_imag.len());
    debug_assert!(staged_coefficients.len() >= coefficient_count);
    debug_assert!(previous.len() >= sin_theta.len());
    debug_assert!(current.len() >= sin_theta.len());
    debug_assert!(next.len() >= sin_theta.len());
    debug_assert!(exponents.len() >= sin_theta.len());

    if sin_theta.is_empty() {
        return Ok(false);
    }

    debug_assert!(
        !(allow_insignificant_prefix && OPPOSITE_COSINE_WEIGHTED)
            || scale_epochs.len() >= sin_theta.len()
    );
    debug_assert!(
        !(allow_insignificant_prefix && OPPOSITE_COSINE_WEIGHTED)
            || scale_corrections.len() >= sin_theta.len()
    );
    let initialized_normal = if allow_insignificant_prefix && OPPOSITE_COSINE_WEIGHTED {
        let Some(initialized_normal) = initialize_scaled_analysis_sectoral_lanes(
            arch,
            m,
            lmax,
            sin_theta,
            sectoral_norm,
            previous,
            current,
            scale_epochs,
        )?
        else {
            #[cfg(all(test, feature = "parallel"))]
            record_scalar_analysis_initialization_decline();
            return Ok(false);
        };
        initialized_normal
    } else {
        initialize_analysis_sectoral_lanes(
            arch,
            m,
            lmax,
            sin_theta,
            sectoral_norm,
            previous,
            current,
            exponents,
        )?
    };

    #[cfg(all(test, feature = "parallel"))]
    record_scalar_analysis_seed_route(
        initialized_normal,
        allow_insignificant_prefix && OPPOSITE_COSINE_WEIGHTED,
    );

    if !initialized_normal {
        let prefix_route = allow_insignificant_prefix && OPPOSITE_COSINE_WEIGHTED;
        let succeeded = if prefix_route {
            arch.dispatch(FusedScaledScalarAnalysisPair::<
                OPPOSITE_COSINE_WEIGHTED,
                CHECK_PAIR_GROUP_RANGE,
            > {
                m,
                lmax,
                range_check_was_exercised,
                cos_theta,
                cos_theta_squared,
                coefficient_a,
                coefficient_b,
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                staged_coefficients: &mut staged_coefficients[..coefficient_count],
                previous: &mut previous[..sin_theta.len()],
                current: &mut current[..sin_theta.len()],
                next: &mut next[..sin_theta.len()],
                scale_epochs: &mut scale_epochs[..sin_theta.len()],
                scale_corrections: &mut scale_corrections[..sin_theta.len()],
            })?
        } else {
            arch.dispatch(FusedCheckedScaledScalarAnalysisPair::<
                OPPOSITE_COSINE_WEIGHTED,
                CHECK_PAIR_GROUP_RANGE,
            > {
                m,
                lmax,
                range_check_was_exercised,
                cos_theta,
                cos_theta_squared,
                coefficient_a,
                coefficient_b,
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                staged_coefficients: &mut staged_coefficients[..coefficient_count],
                previous: &mut previous[..sin_theta.len()],
                current: &mut current[..sin_theta.len()],
                next: &mut next[..sin_theta.len()],
                exponents: &mut exponents[..sin_theta.len()],
            })?
        };
        #[cfg(all(test, feature = "parallel"))]
        if !succeeded {
            record_scalar_analysis_recurrence_decline(prefix_route);
        }
        return Ok(succeeded);
    }

    // A successful block overwrites every coefficient; a declined block's scratch is discarded.
    let succeeded = arch.dispatch(FusedScalarAnalysisPair::<
        OPPOSITE_COSINE_WEIGHTED,
        CHECK_PAIR_GROUP_RANGE,
    > {
        m,
        lmax,
        range_check_was_exercised,
        pair_start: 0,
        cos_theta,
        cos_theta_squared,
        coefficient_a,
        coefficient_b,
        same_real,
        same_imag,
        opposite_real,
        opposite_imag,
        staged_coefficients: &mut staged_coefficients[..coefficient_count],
        second_channel: None,
        previous: &mut previous[..sin_theta.len()],
        current: &mut current[..sin_theta.len()],
        next: &mut next[..sin_theta.len()],
    });
    Ok(succeeded)
}

#[inline(always)]
fn analysis_mask_all<S: Simd>(simd: S, mask: S::m64s) -> bool {
    #[cfg(all(target_arch = "x86_64", rustsht_avx2_mask_all))]
    if const {
        S::F64_LANES == 4
            && core::mem::size_of::<S::m64s>() == core::mem::size_of::<std::arch::x86_64::__m256d>()
    } {
        // SAFETY: the four-lane Pulp mask is the AVX2 lane mask selected only
        // by the V3 dispatch path. Its size check above permits a bitwise copy.
        let mask = unsafe { core::mem::transmute_copy(&mask) };
        // SAFETY: the Pulp V3 specialization is dispatched only when AVX2 is
        // supported by the current CPU.
        return unsafe { analysis_mask_all_avx2(mask) };
    }

    simd.reduce_min_f64s(simd.select_f64s(mask, simd.splat_f64s(1.0), simd.splat_f64s(0.0))) > 0.0
}

#[allow(dead_code, unexpected_cfgs)]
#[cfg(all(target_arch = "x86_64", rustsht_avx2_mask_all))]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn analysis_mask_all_avx2(mask: std::arch::x86_64::__m256d) -> bool {
    use std::arch::x86_64::_mm256_movemask_pd;

    // Pulp comparison masks use an all-ones lane for true and zero for false.
    _mm256_movemask_pd(mask) == 0b1111
}

#[inline(always)]
fn analysis_mask_any<S: Simd>(simd: S, mask: S::m64s) -> bool {
    simd.reduce_max_f64s(simd.select_f64s(mask, simd.splat_f64s(1.0), simd.splat_f64s(0.0))) > 0.0
}

/// Tests DUCC's `iter_to_ieee` scale-epoch frontier. Sectoral initialization
/// normalizes each lane to `sht_ftol`; the recurrence increments its epoch
/// whenever a lane crosses that threshold.
#[inline(always)]
#[cfg(test)]
fn analysis_scale_epochs_below_one<S: Simd>(simd: S, scale_epochs: &[f64]) -> bool {
    let (scale_vectors, scale_tail) = S::as_simd_f64s(scale_epochs);
    let one = simd.splat_f64s(1.0);
    for &scale in scale_vectors {
        if !analysis_mask_all(simd, simd.less_than_f64s(scale, one)) {
            return false;
        }
    }
    scale_tail.iter().all(|&scale| scale < 1.0)
}

#[inline(always)]
fn analysis_recurrence_state_is_finite(previous: &[f64], current: &[f64]) -> bool {
    previous.len() == current.len()
        && previous
            .iter()
            .chain(current)
            .all(|value| value.is_finite())
}

#[inline(always)]
fn analysis_value_in_ieee_range(value: f64) -> bool {
    value.is_finite()
        && (value == 0.0 || (value.abs() >= f64::MIN_POSITIVE && value.abs() <= scalar::BIG))
}

#[inline(always)]
fn analysis_scale_correction(value: f64, scale_epoch: f64) -> Option<f64> {
    if !value.is_finite() || !scale_epoch.is_finite() {
        return None;
    }
    if scale_epoch < 0.0 {
        return Some(0.0);
    }
    let correction = if scale_epoch < 1.0 {
        1.0
    } else {
        ANALYSIS_SCALE_UP
    };
    analysis_value_in_ieee_range(value * correction).then_some(correction)
}

fn prepare_analysis_scale_corrections(
    basis: &[f64],
    scale_epochs: &[f64],
    corrections: &mut [f64],
) -> Option<(bool, bool)> {
    debug_assert_eq!(basis.len(), scale_epochs.len());
    debug_assert!(corrections.len() >= basis.len());
    let mut below_limit = true;
    let mut all_negative = true;
    for ((correction, &value), &scale_epoch) in corrections.iter_mut().zip(basis).zip(scale_epochs)
    {
        below_limit &= scale_epoch < 1.0;
        all_negative &= scale_epoch < 0.0;
        let next = analysis_scale_correction(value, scale_epoch)?;
        *correction = next;
    }
    Some((below_limit, all_negative))
}

/// Checks the scale range once before the recurrence. At most one epoch is
/// added per paired-degree step, so the inner loop need not recheck the same
/// exponent bound after every SIMD vector update.
fn analysis_scale_epochs_allow_recurrence(scale_epochs: &[f64], pair_count: usize) -> bool {
    let increments = pair_count as f64;
    if !increments.is_finite() || increments > ANALYSIS_MAX_SCALE_EPOCH {
        return false;
    }
    let maximum_start_epoch = ANALYSIS_MAX_SCALE_EPOCH - increments;
    scale_epochs.iter().all(|&epoch| {
        epoch.is_finite() && epoch >= -ANALYSIS_MAX_SCALE_EPOCH && epoch <= maximum_start_epoch
    })
}

/// Advances two recurrence pairs and applies DUCC's scale threshold in one
/// latitude pass. The caller enters with every epoch below one; carry the
/// updated below-limit state forward instead of rereading the epoch array
/// after each pair group. This route discards coefficients until the
/// recurrence reaches IEEE scale.
#[inline(always)]
fn advance_and_rescale_analysis_pair<S: Simd>(
    simd: S,
    coefficients: [(f64, f64); 2],
    cos_theta_squared: &[f64],
    recurrence: (&mut [f64], &mut [f64]),
    scale_epochs: &mut [f64],
    corrections: &mut [f64],
    skip_insignificant_validation: bool,
) -> Option<(bool, bool)> {
    let [
        (coefficient_a1, coefficient_b1),
        (coefficient_a2, coefficient_b2),
    ] = coefficients;
    let (previous, current) = recurrence;
    if previous.len() != current.len()
        || current.len() != cos_theta_squared.len()
        || current.len() != scale_epochs.len()
        || current.len() != corrections.len()
    {
        return None;
    }
    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
    let (cosine_squared_vectors, cosine_squared_tail) = S::as_simd_f64s(cos_theta_squared);
    let (scale_vectors, scale_tail) = S::as_mut_simd_f64s(scale_epochs);
    let (correction_vectors, correction_tail) = S::as_mut_simd_f64s(corrections);
    let a1 = simd.splat_f64s(coefficient_a1);
    let b1 = simd.splat_f64s(coefficient_b1);
    let a2 = simd.splat_f64s(coefficient_a2);
    let b2 = simd.splat_f64s(coefficient_b2);
    let zero = simd.splat_f64s(0.0);
    let one = simd.splat_f64s(1.0);
    let ftol = simd.splat_f64s(ANALYSIS_FTOL);
    let down = simd.splat_f64s(ANALYSIS_SCALE_DOWN);
    let up = simd.splat_f64s(ANALYSIS_SCALE_UP);
    let min_normal = simd.splat_f64s(f64::MIN_POSITIVE);
    let max_finite = simd.splat_f64s(f64::MAX);
    let max_mantissa = simd.splat_f64s(scalar::BIG);
    let mut all_vectors_valid = simd.equal_f64s(zero, zero);
    let mut all_rescaled_epochs_negative = simd.equal_f64s(zero, zero);
    let mut any_rescaled = false;
    let mut any_vector_rescaled = false;
    let mut below_limit = true;
    let mut all_tail_epochs_negative = true;

    for index in 0..current_vectors.len() {
        let previous_value = previous_vectors[index];
        let current_value = current_vectors[index];
        let cosine_squared = cosine_squared_vectors[index];
        let first_leading = simd.mul_add_f64s(a1, cosine_squared, b1);
        let first = simd.mul_add_f64s(first_leading, current_value, previous_value);
        let second_leading = simd.mul_add_f64s(a2, cosine_squared, b2);
        let second = simd.mul_add_f64s(second_leading, first, current_value);
        let magnitude = simd.abs_f64s(second);
        let rescale = simd.greater_than_f64s(magnitude, ftol);
        let vector_rescaled = simd.reduce_max_f64s(magnitude) > ANALYSIS_FTOL;
        any_rescaled |= vector_rescaled;
        any_vector_rescaled |= vector_rescaled;

        if !vector_rescaled {
            // The active prefix has scale epochs below one. Negative epochs
            // intentionally suppress insignificant mantissas; epoch zero
            // still requires an ordinary nonzero mantissa. Since every lane
            // is below ftol here, no upper-magnitude check is needed.
            if !skip_insignificant_validation {
                let finite_mantissa = simd.less_than_or_equal_f64s(magnitude, max_finite);
                let zero_or_normal = simd.or_m64s(
                    simd.equal_f64s(magnitude, zero),
                    simd.greater_than_or_equal_f64s(magnitude, min_normal),
                );
                let negative_epoch = simd.less_than_f64s(scale_vectors[index], zero);
                let current_valid = simd.and_m64s(
                    finite_mantissa,
                    simd.or_m64s(negative_epoch, zero_or_normal),
                );
                all_vectors_valid = simd.and_m64s(all_vectors_valid, current_valid);
            }
            previous_vectors[index] = first;
            current_vectors[index] = second;
            continue;
        }

        let updated_previous = simd.select_f64s(rescale, simd.mul_f64s(first, down), first);
        let updated_current = simd.select_f64s(rescale, simd.mul_f64s(second, down), second);
        let next_scale = simd.add_f64s(scale_vectors[index], one);
        let updated_scale = simd.select_f64s(rescale, next_scale, scale_vectors[index]);
        // A vector that did not rescale retains the entry invariant that all
        // epochs are below one. DUCC performs this all-lane test only for
        // vectors whose scale changed; avoid an unconditional reduction here.
        if vector_rescaled {
            below_limit &= analysis_mask_all(simd, simd.less_than_f64s(updated_scale, one));
            if skip_insignificant_validation {
                all_rescaled_epochs_negative = simd.and_m64s(
                    all_rescaled_epochs_negative,
                    simd.less_than_f64s(updated_scale, zero),
                );
            }
        }
        let correction = simd.select_f64s(
            simd.less_than_f64s(updated_scale, zero),
            zero,
            simd.select_f64s(simd.less_than_f64s(updated_scale, one), one, up),
        );
        let updated_correction = simd.select_f64s(rescale, correction, correction_vectors[index]);

        let corrected_current = simd.mul_f64s(updated_current, updated_correction);
        let current_magnitude = simd.abs_f64s(corrected_current);
        let finite_mantissa =
            simd.less_than_or_equal_f64s(simd.abs_f64s(updated_current), max_finite);
        let current_valid = simd.and_m64s(
            finite_mantissa,
            simd.or_m64s(
                simd.equal_f64s(current_magnitude, zero),
                simd.and_m64s(
                    simd.greater_than_or_equal_f64s(current_magnitude, min_normal),
                    simd.less_than_or_equal_f64s(current_magnitude, max_mantissa),
                ),
            ),
        );
        all_vectors_valid = simd.and_m64s(all_vectors_valid, current_valid);
        previous_vectors[index] = updated_previous;
        current_vectors[index] = updated_current;
        scale_vectors[index] = updated_scale;
        correction_vectors[index] = updated_correction;
    }

    for lane in 0..current_tail.len() {
        let first = coefficient_a1
            .mul_add(cosine_squared_tail[lane], coefficient_b1)
            .mul_add(current_tail[lane], previous_tail[lane]);
        let second = coefficient_a2
            .mul_add(cosine_squared_tail[lane], coefficient_b2)
            .mul_add(first, current_tail[lane]);
        if !first.is_finite() || !second.is_finite() {
            return None;
        }
        let rescale = second.abs() > ANALYSIS_FTOL;
        any_rescaled |= rescale;
        let (updated_previous, updated_current) = if rescale {
            let next_scale = scale_tail[lane] + 1.0;
            if !next_scale.is_finite()
                || next_scale == scale_tail[lane]
                || next_scale.abs() > 4.5e15
            {
                return None;
            }
            let previous_scaled = first * ANALYSIS_SCALE_DOWN;
            let current_scaled = second * ANALYSIS_SCALE_DOWN;
            if second != 0.0 && current_scaled.abs() < f64::MIN_POSITIVE {
                return None;
            }
            scale_tail[lane] = next_scale;
            below_limit &= next_scale < 1.0;
            correction_tail[lane] = analysis_scale_correction(current_scaled, next_scale)?;
            (previous_scaled, current_scaled)
        } else {
            (first, second)
        };
        if skip_insignificant_validation {
            all_tail_epochs_negative &= scale_tail[lane] < 0.0;
        }
        let corrected_current = updated_current * correction_tail[lane];
        if !analysis_value_in_ieee_range(corrected_current) {
            return None;
        }
        previous_tail[lane] = updated_previous;
        current_tail[lane] = updated_current;
    }

    if !analysis_mask_all(simd, all_vectors_valid) {
        return None;
    }
    if skip_insignificant_validation
        && any_rescaled
        && !previous
            .iter()
            .chain(current.iter())
            .all(|value| value.is_finite())
    {
        return None;
    }
    let remains_insignificant_prefix = skip_insignificant_validation
        && all_tail_epochs_negative
        && (!any_vector_rescaled || analysis_mask_all(simd, all_rescaled_epochs_negative));
    Some((below_limit, remains_insignificant_prefix))
}

/// Applies DUCC's per-lane `rescale` rule while the analysis recurrence is in
/// exponent space. Invalid scale or value ranges decline to the checked path.
#[inline(always)]
fn rescale_scaled_analysis_pair<S: Simd, const CHECK_NONNEGATIVE: bool>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale_epochs: &mut [f64],
    corrections: &mut [f64],
) -> Option<(bool, bool)> {
    if previous.len() != current.len()
        || current.len() != scale_epochs.len()
        || current.len() != corrections.len()
    {
        return None;
    }
    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
    let (scale_vectors, scale_tail) = S::as_mut_simd_f64s(scale_epochs);
    let (correction_vectors, correction_tail) = S::as_mut_simd_f64s(corrections);
    let zero_f64 = simd.splat_f64s(0.0);
    let ftol = simd.splat_f64s(ANALYSIS_FTOL);
    let down = simd.splat_f64s(ANALYSIS_SCALE_DOWN);
    let up = simd.splat_f64s(ANALYSIS_SCALE_UP);
    let one = simd.splat_f64s(1.0);
    let min_normal = simd.splat_f64s(f64::MIN_POSITIVE);
    let max_finite = simd.splat_f64s(f64::MAX);
    let max_mantissa = simd.splat_f64s(scalar::BIG);
    let mut all_vectors_valid = simd.equal_f64s(zero_f64, zero_f64);
    let mut all_nonnegative_lanes = simd.equal_f64s(zero_f64, zero_f64);
    let mut all_nonnegative_tails = true;
    let mut any_rescaled = false;

    for index in 0..current_vectors.len() {
        let previous_value = previous_vectors[index];
        let current_value = current_vectors[index];
        let current_magnitude = simd.abs_f64s(current_value);
        let rescale = simd.greater_than_f64s(current_magnitude, ftol);
        if !CHECK_NONNEGATIVE {
            // Reducing the magnitudes avoids materializing a 0/1 vector from
            // the comparison mask before the horizontal test. NaNs are
            // rejected by the accumulated range validation below.
            let vector_rescaled = simd.reduce_max_f64s(current_magnitude) > ANALYSIS_FTOL;
            any_rescaled |= vector_rescaled;
            if !vector_rescaled {
                // DUCC's rescale() returns immediately when no lane crosses
                // the threshold. Keep the Rust range check, but avoid scale,
                // correction and state-update work for this vector.
                let corrected_current = simd.mul_f64s(current_value, correction_vectors[index]);
                let corrected_magnitude = simd.abs_f64s(corrected_current);
                let current_is_finite = simd.less_than_or_equal_f64s(current_magnitude, max_finite);
                let current_valid = simd.and_m64s(
                    current_is_finite,
                    simd.or_m64s(
                        simd.equal_f64s(corrected_magnitude, zero_f64),
                        simd.and_m64s(
                            simd.greater_than_or_equal_f64s(corrected_magnitude, min_normal),
                            simd.less_than_or_equal_f64s(corrected_magnitude, max_mantissa),
                        ),
                    ),
                );
                all_vectors_valid = simd.and_m64s(all_vectors_valid, current_valid);
                continue;
            }
        }
        let next_scale = simd.add_f64s(scale_vectors[index], one);
        let previous_scaled =
            simd.select_f64s(rescale, simd.mul_f64s(previous_value, down), previous_value);
        let current_scaled =
            simd.select_f64s(rescale, simd.mul_f64s(current_value, down), current_value);
        // Rescaling is triggered above 2^-60 and multiplies by 2^-800, so a
        // finite rescaled current remains above 2^-860 and cannot underflow.
        // The previous value may become subnormal; retain that IEEE result.

        previous_vectors[index] = previous_scaled;
        current_vectors[index] = current_scaled;
        let updated_scale = simd.select_f64s(rescale, next_scale, scale_vectors[index]);
        scale_vectors[index] = updated_scale;
        let correction = simd.select_f64s(
            simd.less_than_f64s(updated_scale, zero_f64),
            zero_f64,
            simd.select_f64s(simd.less_than_f64s(updated_scale, one), one, up),
        );
        let updated_correction = simd.select_f64s(rescale, correction, correction_vectors[index]);
        correction_vectors[index] = updated_correction;
        let corrected_current = simd.mul_f64s(current_scaled, updated_correction);
        let current_magnitude = simd.abs_f64s(corrected_current);
        let current_is_finite =
            simd.less_than_or_equal_f64s(simd.abs_f64s(current_scaled), max_finite);
        let current_valid = simd.and_m64s(
            current_is_finite,
            simd.or_m64s(
                simd.equal_f64s(current_magnitude, zero_f64),
                simd.and_m64s(
                    simd.greater_than_or_equal_f64s(current_magnitude, min_normal),
                    simd.less_than_or_equal_f64s(current_magnitude, max_mantissa),
                ),
            ),
        );
        all_vectors_valid = simd.and_m64s(all_vectors_valid, current_valid);
        if CHECK_NONNEGATIVE {
            all_nonnegative_lanes = simd.and_m64s(
                all_nonnegative_lanes,
                simd.greater_than_or_equal_f64s(updated_scale, zero_f64),
            );
        }
    }

    for lane in 0..current_tail.len() {
        if !previous_tail[lane].is_finite() || !current_tail[lane].is_finite() {
            return None;
        }
        let rescale = current_tail[lane].abs() > ANALYSIS_FTOL;
        if !CHECK_NONNEGATIVE {
            any_rescaled |= rescale;
        }
        if rescale {
            let next_scale = scale_tail[lane] + 1.0;
            if !next_scale.is_finite()
                || next_scale == scale_tail[lane]
                || next_scale.abs() > 4.5e15
            {
                return None;
            }
            let previous_scaled = previous_tail[lane] * ANALYSIS_SCALE_DOWN;
            let current_scaled = current_tail[lane] * ANALYSIS_SCALE_DOWN;
            if current_tail[lane] != 0.0 && current_scaled.abs() < f64::MIN_POSITIVE {
                return None;
            }
            previous_tail[lane] = previous_scaled;
            current_tail[lane] = current_scaled;
            scale_tail[lane] = next_scale;
            correction_tail[lane] = analysis_scale_correction(current_scaled, next_scale)?;
        }
        let corrected_current = current_tail[lane] * correction_tail[lane];
        if !analysis_value_in_ieee_range(corrected_current) {
            return None;
        }
        if CHECK_NONNEGATIVE {
            all_nonnegative_tails &= scale_tail[lane] >= 0.0;
        }
    }
    if !analysis_mask_all(simd, all_vectors_valid) {
        return None;
    }
    let all_nonnegative = !CHECK_NONNEGATIVE
        || (analysis_mask_all(simd, all_nonnegative_lanes) && all_nonnegative_tails);
    Some((any_rescaled, all_nonnegative))
}

/// Converts a complete nonnegative scale-epoch block into ordinary recurrence
/// values when every lane remains inside the fast kernel's checked range.
/// Negative epochs continue in the scaled loop; unsupported magnitudes decline
/// without changing any lane.
#[inline(always)]
fn promote_analysis_pair_to_ordinary<S: Simd>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale_epochs: &mut [f64],
    corrections: &mut [f64],
) -> Option<bool> {
    if previous.len() != current.len()
        || current.len() != scale_epochs.len()
        || current.len() != corrections.len()
    {
        return None;
    }
    let (scale_vectors, scale_tail) = S::as_mut_simd_f64s(scale_epochs);
    let (correction_vectors, correction_tail) = S::as_mut_simd_f64s(corrections);
    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
    let zero = simd.splat_f64s(0.0);
    for index in 0..scale_vectors.len() {
        if !analysis_mask_all(
            simd,
            simd.greater_than_or_equal_f64s(scale_vectors[index], zero),
        ) {
            return Some(false);
        }
        let corrected_previous = simd.mul_f64s(previous_vectors[index], correction_vectors[index]);
        let corrected_current = simd.mul_f64s(current_vectors[index], correction_vectors[index]);
        let previous_magnitude = simd.abs_f64s(corrected_previous);
        let current_magnitude = simd.abs_f64s(corrected_current);
        let previous_valid = simd.or_m64s(
            simd.equal_f64s(previous_magnitude, zero),
            simd.and_m64s(
                simd.greater_than_or_equal_f64s(
                    previous_magnitude,
                    simd.splat_f64s(f64::MIN_POSITIVE),
                ),
                simd.less_than_or_equal_f64s(previous_magnitude, simd.splat_f64s(scalar::BIG)),
            ),
        );
        let current_valid = simd.or_m64s(
            simd.equal_f64s(current_magnitude, zero),
            simd.and_m64s(
                simd.greater_than_or_equal_f64s(
                    current_magnitude,
                    simd.splat_f64s(f64::MIN_POSITIVE),
                ),
                simd.less_than_or_equal_f64s(current_magnitude, simd.splat_f64s(scalar::BIG)),
            ),
        );
        if !analysis_mask_all(simd, simd.and_m64s(previous_valid, current_valid)) {
            return Some(false);
        }
    }
    if scale_tail
        .iter()
        .any(|&scale| scale < 0.0 || !scale.is_finite())
    {
        return Some(false);
    }
    for lane in 0..current_tail.len() {
        let previous_value = previous_tail[lane] * correction_tail[lane];
        let current_value = current_tail[lane] * correction_tail[lane];
        if !analysis_value_in_ieee_range(previous_value)
            || !analysis_value_in_ieee_range(current_value)
        {
            return Some(false);
        }
    }

    for index in 0..previous_vectors.len() {
        previous_vectors[index] = simd.mul_f64s(previous_vectors[index], correction_vectors[index]);
        current_vectors[index] = simd.mul_f64s(current_vectors[index], correction_vectors[index]);
        scale_vectors[index] = zero;
        correction_vectors[index] = simd.splat_f64s(1.0);
    }
    for lane in 0..previous_tail.len() {
        previous_tail[lane] *= correction_tail[lane];
        current_tail[lane] *= correction_tail[lane];
        scale_tail[lane] = 0.0;
        correction_tail[lane] = 1.0;
    }
    Some(true)
}

#[inline(always)]
fn zero_staged_analysis_pair(
    staged_coefficients: &mut [Complex64],
    m: usize,
    lmax: usize,
    pair_index: usize,
) {
    let degree = m + 2 * pair_index;
    let offset = degree - m;
    staged_coefficients[offset] = Complex64::new(0.0, 0.0);
    if degree < lmax {
        staged_coefficients[offset + 1] = Complex64::new(0.0, 0.0);
    }
}

struct FusedScaledScalarAnalysisPair<
    'a,
    const OPPOSITE_COSINE_WEIGHTED: bool,
    const CHECK_PAIR_GROUP_RANGE: bool = { !cfg!(rustsht_unchecked_analysis_pair_range) },
> {
    m: usize,
    lmax: usize,
    range_check_was_exercised: &'a mut bool,
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    staged_coefficients: &'a mut [Complex64],
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
    scale_epochs: &'a mut [f64],
    scale_corrections: &'a mut [f64],
}

/// Checked scaled path for analysis variants that cannot use insignificant
/// prefix advancement. Scaled lanes retain exact exponent-aware accumulation.
struct FusedCheckedScaledScalarAnalysisPair<
    'a,
    const OPPOSITE_COSINE_WEIGHTED: bool,
    const CHECK_PAIR_GROUP_RANGE: bool,
> {
    m: usize,
    lmax: usize,
    range_check_was_exercised: &'a mut bool,
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    staged_coefficients: &'a mut [Complex64],
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
    exponents: &'a mut [i64],
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool, const CHECK_PAIR_GROUP_RANGE: bool> WithSimd
    for FusedCheckedScaledScalarAnalysisPair<'_, OPPOSITE_COSINE_WEIGHTED, CHECK_PAIR_GROUP_RANGE>
{
    type Output = Result<bool, ShtError>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedCheckedScaledScalarAnalysisPair {
            m,
            lmax,
            range_check_was_exercised,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            previous,
            current,
            next,
            exponents,
        } = self;
        let mut previous = previous;
        let mut current = current;
        let mut next = next;
        let pair_count = (lmax - m + 2) / 2;
        let mut normal_range = false;
        for pair_index in 0..pair_count {
            if normal_range {
                return Ok(FusedScalarAnalysisPair::<
                    OPPOSITE_COSINE_WEIGHTED,
                    CHECK_PAIR_GROUP_RANGE,
                > {
                    m,
                    lmax,
                    range_check_was_exercised,
                    pair_start: pair_index,
                    cos_theta,
                    cos_theta_squared,
                    coefficient_a,
                    coefficient_b,
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    staged_coefficients,
                    second_channel: None,
                    previous,
                    current,
                    next,
                }
                .with_simd(simd));
            }
            let l = m + 2 * pair_index;
            let offset = l - m;
            let (same, opposite) = if pair_index + 1 < pair_count {
                FusedCheckedMixedAnalysisPairStep::<OPPOSITE_COSINE_WEIGHTED> {
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    basis_previous: previous,
                    basis_current: current,
                    exponents,
                    cos_theta,
                    cos_theta_squared,
                    coefficient_a: coefficient_a[pair_index],
                    coefficient_b: coefficient_b[pair_index],
                    basis_next: next,
                }
                .with_simd(simd)
            } else {
                MixedAnalysisPairAccumulation::<OPPOSITE_COSINE_WEIGHTED> {
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    basis: current,
                    exponents,
                    cos_theta,
                    allow_insignificant_scaled_lanes: false,
                }
                .with_simd(simd)
            };
            staged_coefficients[offset] = same;
            if l < lmax {
                staged_coefficients[offset + 1] = opposite;
            }
            if pair_index + 1 == pair_count {
                return Ok(true);
            }
            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
            let Some(ready) = scalar::normalize_pair_lanes_simd(simd, previous, current, exponents)
            else {
                return Ok(false);
            };
            normal_range = ready;
        }
        Ok(true)
    }
}

struct FusedCheckedMixedAnalysisPairStep<'a, const OPPOSITE_COSINE_WEIGHTED: bool> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis_previous: &'a [f64],
    basis_current: &'a [f64],
    exponents: &'a [i64],
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    coefficient_a: f64,
    coefficient_b: f64,
    basis_next: &'a mut [f64],
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool> WithSimd
    for FusedCheckedMixedAnalysisPairStep<'_, OPPOSITE_COSINE_WEIGHTED>
{
    type Output = (Complex64, Complex64);

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedCheckedMixedAnalysisPairStep {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis_previous,
            basis_current,
            exponents,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            basis_next,
        } = self;
        let (same_real_vectors, same_real_tail) = S::as_simd_f64s(same_real);
        let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(same_imag);
        let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(opposite_real);
        let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(opposite_imag);
        let (previous_vectors, previous_tail) = S::as_simd_f64s(basis_previous);
        let (current_vectors, current_tail) = S::as_simd_f64s(basis_current);
        let (exponent_vectors, exponent_tail) = S::as_simd_i64s(exponents);
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
        let (cosine_squared_vectors, cosine_squared_tail) = S::as_simd_f64s(cos_theta_squared);
        let (next_vectors, next_tail) = S::as_mut_simd_f64s(basis_next);
        let zero = simd.splat_f64s(0.0);
        let zero_exponent = simd.splat_i64s(0);
        let minimum = simd.splat_f64s(f64::MIN_POSITIVE);
        let maximum = simd.splat_f64s(scalar::BIG);
        let a = simd.splat_f64s(coefficient_a);
        let b = simd.splat_f64s(coefficient_b);
        let mut same_re = zero;
        let mut same_im = zero;
        let mut opposite_re = zero;
        let mut opposite_im = zero;
        let mut vector_needs_scalar = false;
        for index in 0..current_vectors.len() {
            let magnitude = simd.abs_f64s(current_vectors[index]);
            let is_zero = simd.equal_f64s(magnitude, zero);
            let is_normal = simd.and_m64s(
                simd.greater_than_or_equal_f64s(magnitude, minimum),
                simd.less_than_or_equal_f64s(magnitude, maximum),
            );
            let exponent_zero = simd.equal_i64s(exponent_vectors[index], zero_exponent);
            let normal = simd.and_m64s(exponent_zero, simd.or_m64s(is_zero, is_normal));
            vector_needs_scalar |= !analysis_mask_all(simd, normal);
            let basis = simd.select_f64s(normal, current_vectors[index], zero);
            same_re = simd.mul_add_f64s(
                simd.select_f64s(normal, same_real_vectors[index], zero),
                basis,
                same_re,
            );
            same_im = simd.mul_add_f64s(
                simd.select_f64s(normal, same_imag_vectors[index], zero),
                basis,
                same_im,
            );
            let opposite_re_coeff = simd.select_f64s(normal, opposite_real_vectors[index], zero);
            let opposite_im_coeff = simd.select_f64s(normal, opposite_imag_vectors[index], zero);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_re = simd.mul_add_f64s(opposite_re_coeff, basis, opposite_re);
                opposite_im = simd.mul_add_f64s(opposite_im_coeff, basis, opposite_im);
            } else {
                opposite_re = simd.mul_add_f64s(
                    simd.mul_f64s(opposite_re_coeff, cosine_vectors[index]),
                    basis,
                    opposite_re,
                );
                opposite_im = simd.mul_add_f64s(
                    simd.mul_f64s(opposite_im_coeff, cosine_vectors[index]),
                    basis,
                    opposite_im,
                );
            }
            let leading = simd.mul_add_f64s(a, cosine_squared_vectors[index], b);
            next_vectors[index] =
                simd.mul_add_f64s(leading, current_vectors[index], previous_vectors[index]);
        }
        let mut same = Complex64::new(simd.reduce_sum_f64s(same_re), simd.reduce_sum_f64s(same_im));
        let mut opposite = Complex64::new(
            simd.reduce_sum_f64s(opposite_re),
            simd.reduce_sum_f64s(opposite_im),
        );
        if vector_needs_scalar {
            let vector_lane_count = basis_current.len() - current_tail.len();
            for lane in 0..vector_lane_count {
                let magnitude = basis_current[lane].abs();
                let normal = exponents[lane] == 0
                    && (magnitude == 0.0 || (f64::MIN_POSITIVE..=scalar::BIG).contains(&magnitude));
                if !normal {
                    same += scalar::multiply_complex_scaled(
                        Complex64::new(same_real[lane], same_imag[lane]),
                        basis_current[lane],
                        exponents[lane],
                    );
                    let coefficient = Complex64::new(opposite_real[lane], opposite_imag[lane]);
                    opposite += if OPPOSITE_COSINE_WEIGHTED {
                        scalar::multiply_complex_scaled(
                            coefficient,
                            basis_current[lane],
                            exponents[lane],
                        )
                    } else {
                        scalar::multiply_complex_scaled_product(
                            coefficient,
                            basis_current[lane],
                            cos_theta[lane],
                            exponents[lane],
                        )
                    };
                }
            }
        }
        for lane in 0..current_tail.len() {
            let magnitude = current_tail[lane].abs();
            if exponent_tail[lane] == 0
                && (magnitude == 0.0 || (f64::MIN_POSITIVE..=scalar::BIG).contains(&magnitude))
            {
                same.re = same_real_tail[lane].mul_add(current_tail[lane], same.re);
                same.im = same_imag_tail[lane].mul_add(current_tail[lane], same.im);
                if OPPOSITE_COSINE_WEIGHTED {
                    opposite.re = opposite_real_tail[lane].mul_add(current_tail[lane], opposite.re);
                    opposite.im = opposite_imag_tail[lane].mul_add(current_tail[lane], opposite.im);
                } else {
                    opposite.re = (opposite_real_tail[lane] * cosine_tail[lane])
                        .mul_add(current_tail[lane], opposite.re);
                    opposite.im = (opposite_imag_tail[lane] * cosine_tail[lane])
                        .mul_add(current_tail[lane], opposite.im);
                }
            } else {
                let exponent = exponent_tail[lane];
                same += scalar::multiply_complex_scaled(
                    Complex64::new(same_real_tail[lane], same_imag_tail[lane]),
                    current_tail[lane],
                    exponent,
                );
                let coefficient =
                    Complex64::new(opposite_real_tail[lane], opposite_imag_tail[lane]);
                opposite += if OPPOSITE_COSINE_WEIGHTED {
                    scalar::multiply_complex_scaled(coefficient, current_tail[lane], exponent)
                } else {
                    scalar::multiply_complex_scaled_product(
                        coefficient,
                        current_tail[lane],
                        cosine_tail[lane],
                        exponent,
                    )
                };
            }
            let leading = coefficient_a.mul_add(cosine_squared_tail[lane], coefficient_b);
            next_tail[lane] = leading.mul_add(current_tail[lane], previous_tail[lane]);
        }
        (same, opposite)
    }
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool, const CHECK_PAIR_GROUP_RANGE: bool> WithSimd
    for FusedScaledScalarAnalysisPair<'_, OPPOSITE_COSINE_WEIGHTED, CHECK_PAIR_GROUP_RANGE>
{
    type Output = Result<bool, ShtError>;

    #[allow(unexpected_cfgs)]
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedScaledScalarAnalysisPair {
            m,
            lmax,
            range_check_was_exercised,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            previous,
            current,
            next,
            scale_epochs,
            scale_corrections,
        } = self;
        let mut previous = previous;
        let mut current = current;
        let mut next = next;
        let pair_count = (lmax - m + 2) / 2;
        let mut pair_start = 0;
        if !analysis_scale_epochs_allow_recurrence(scale_epochs, pair_count) {
            return Ok(false);
        }
        let Some((mut below_limit, mut skip_insignificant_validation)) =
            prepare_analysis_scale_corrections(
                current,
                scale_epochs,
                &mut scale_corrections[..current.len()],
            )
        else {
            return Ok(false);
        };
        while below_limit && pair_start < pair_count {
            zero_staged_analysis_pair(staged_coefficients, m, lmax, pair_start);
            if pair_count - pair_start == 1 {
                return Ok(analysis_recurrence_state_is_finite(previous, current));
            }

            let first_pair = pair_start;
            let second_pair = first_pair + 1;
            zero_staged_analysis_pair(staged_coefficients, m, lmax, second_pair);
            if pair_count - second_pair == 1 {
                return Ok(analysis_recurrence_state_is_finite(previous, current));
            }
            let Some((remains_below_limit, remains_insignificant_prefix)) =
                advance_and_rescale_analysis_pair(
                    simd,
                    [
                        (coefficient_a[first_pair], coefficient_b[first_pair]),
                        (coefficient_a[second_pair], coefficient_b[second_pair]),
                    ],
                    cos_theta_squared,
                    (&mut *previous, &mut *current),
                    scale_epochs,
                    scale_corrections,
                    skip_insignificant_validation,
                )
            else {
                return Ok(false);
            };
            pair_start += 2;
            below_limit = remains_below_limit;
            skip_insignificant_validation = remains_insignificant_prefix;
        }
        if pair_start == pair_count {
            return Ok(analysis_recurrence_state_is_finite(previous, current));
        }

        let initial_normal_range = promote_analysis_pair_to_ordinary(
            simd,
            previous,
            current,
            scale_epochs,
            &mut scale_corrections[..current.len()],
        );
        let Some(mut normal_range) = initial_normal_range else {
            return Ok(false);
        };

        for pair_index in pair_start..pair_count {
            if normal_range {
                return Ok(FusedScalarAnalysisPair::<
                    OPPOSITE_COSINE_WEIGHTED,
                    CHECK_PAIR_GROUP_RANGE,
                > {
                    m,
                    lmax,
                    range_check_was_exercised,
                    pair_start: pair_index,
                    cos_theta,
                    cos_theta_squared,
                    coefficient_a,
                    coefficient_b,
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    staged_coefficients,
                    second_channel: None,
                    previous,
                    current,
                    next,
                }
                .with_simd(simd));
            }
            let l = m + 2 * pair_index;
            let offset = l - m;
            let (same_sum, opposite_sum) = if pair_index + 1 < pair_count {
                FusedMixedAnalysisPairStep::<OPPOSITE_COSINE_WEIGHTED> {
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    basis_previous: previous,
                    basis_current: current,
                    cos_theta,
                    cos_theta_squared,
                    scale_corrections: &scale_corrections[..current.len()],
                    coefficient_a: coefficient_a[pair_index],
                    coefficient_b: coefficient_b[pair_index],
                    basis_next: next,
                }
                .with_simd(simd)
            } else {
                FusedMixedAnalysisPairAccumulation::<OPPOSITE_COSINE_WEIGHTED> {
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    basis_current: current,
                    cos_theta,
                    scale_corrections: &scale_corrections[..current.len()],
                }
                .with_simd(simd)
            };
            staged_coefficients[offset] = same_sum;
            if l < lmax {
                staged_coefficients[offset + 1] = opposite_sum;
            }
            if pair_index + 1 == pair_count {
                return Ok(true);
            }

            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
            let Some((_, all_nonnegative)) = rescale_scaled_analysis_pair::<S, true>(
                simd,
                previous,
                current,
                scale_epochs,
                &mut scale_corrections[..current.len()],
            ) else {
                return Ok(false);
            };
            normal_range = if all_nonnegative {
                match promote_analysis_pair_to_ordinary(
                    simd,
                    previous,
                    current,
                    scale_epochs,
                    &mut scale_corrections[..current.len()],
                ) {
                    Some(ready) => ready,
                    None => return Ok(false),
                }
            } else {
                false
            };
        }
        Ok(true)
    }
}

#[allow(unexpected_cfgs)]
struct FusedScalarAnalysisPair<
    'a,
    const OPPOSITE_COSINE_WEIGHTED: bool,
    // The custom cfg supports whole-transform A/B builds. Production also
    // selects the unchecked specialization after a cached checked validation.
    const CHECK_PAIR_GROUP_RANGE: bool = { !cfg!(rustsht_unchecked_analysis_pair_range) },
    // Experimental checked mode folds the group-state reduction into the
    // recurrence traversal instead of rereading the resulting state arrays.
    const FUSE_PAIR_GROUP_RANGE_CHECK: bool = { cfg!(rustsht_fused_analysis_pair_range_check) },
    // Test-only source attribution compiles the same loop without coefficient
    // products; the production specialization always keeps accumulation.
    const ACCUMULATE: bool = true,
    // Optional second channel is reduced from basis states saved by the first
    // channel's fused recurrence pass.
    const SEQUENTIAL_SECOND_CHANNEL: bool = false,
> {
    m: usize,
    lmax: usize,
    range_check_was_exercised: &'a mut bool,
    pair_start: usize,
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    staged_coefficients: &'a mut [Complex64],
    second_channel: Option<FusedScalarAnalysisPairSecondChannel<'a>>,
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
}

struct FusedScalarAnalysisPairSecondChannel<'a> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    staged_coefficients: &'a mut [Complex64],
}

struct FusedAnalysisPairStep<'a, const OPPOSITE_COSINE_WEIGHTED: bool> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis_previous: &'a [f64],
    basis_current: &'a [f64],
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    coefficient_a: f64,
    coefficient_b: f64,
    basis_next: &'a mut [f64],
}

struct FusedAnalysisPairGroup {
    first_same: Complex64,
    first_opposite: Complex64,
    second_same: Complex64,
    second_opposite: Complex64,
    states_in_range: bool,
    tails_in_range: bool,
}

struct FusedAnalysisTwoPairSteps<'a, const OPPOSITE_COSINE_WEIGHTED: bool> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis_previous: &'a mut [f64],
    basis_current: &'a mut [f64],
    basis_snapshot: &'a mut [f64],
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    first_coefficient_a: f64,
    first_coefficient_b: f64,
    second_coefficient_a: f64,
    second_coefficient_b: f64,
}

/// Reduces the real and imaginary lanes of one complex spectrum independently.
#[inline(always)]
fn reduce_analysis_complex_pair<S: Simd>(simd: S, real: S::f64s, imag: S::f64s) -> Complex64 {
    Complex64::new(simd.reduce_sum_f64s(real), simd.reduce_sum_f64s(imag))
}

#[inline(always)]
fn fused_analysis_pair_step_with_simd<S: Simd, const OPPOSITE_COSINE_WEIGHTED: bool>(
    simd: S,
    step: FusedAnalysisPairStep<'_, OPPOSITE_COSINE_WEIGHTED>,
) -> (Complex64, Complex64, S::m64s, bool) {
    let FusedAnalysisPairStep {
        same_real,
        same_imag,
        opposite_real,
        opposite_imag,
        basis_previous,
        basis_current,
        cos_theta,
        cos_theta_squared,
        coefficient_a,
        coefficient_b,
        basis_next,
    } = step;
    let (same_real_vectors, same_real_tail) = S::as_simd_f64s(same_real);
    let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(same_imag);
    let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(opposite_real);
    let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(opposite_imag);
    let (previous_vectors, previous_tail) = S::as_simd_f64s(basis_previous);
    let (current_vectors, current_tail) = S::as_simd_f64s(basis_current);
    let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
    let (cosine_squared_vectors, cosine_squared_tail) = S::as_simd_f64s(cos_theta_squared);
    let (next_vectors, next_tail) = S::as_mut_simd_f64s(basis_next);
    debug_assert_eq!(same_real_vectors.len(), current_vectors.len());
    debug_assert_eq!(same_imag_vectors.len(), current_vectors.len());
    debug_assert_eq!(opposite_real_vectors.len(), current_vectors.len());
    debug_assert_eq!(opposite_imag_vectors.len(), current_vectors.len());
    debug_assert_eq!(previous_vectors.len(), current_vectors.len());
    debug_assert_eq!(cosine_vectors.len(), current_vectors.len());
    debug_assert_eq!(cosine_squared_vectors.len(), current_vectors.len());
    debug_assert_eq!(next_vectors.len(), current_vectors.len());
    debug_assert_eq!(same_real_tail.len(), current_tail.len());
    debug_assert_eq!(same_imag_tail.len(), current_tail.len());
    debug_assert_eq!(opposite_real_tail.len(), current_tail.len());
    debug_assert_eq!(opposite_imag_tail.len(), current_tail.len());
    debug_assert_eq!(previous_tail.len(), current_tail.len());
    debug_assert_eq!(cosine_tail.len(), current_tail.len());
    debug_assert_eq!(cosine_squared_tail.len(), current_tail.len());
    debug_assert_eq!(next_tail.len(), current_tail.len());

    let zero = simd.splat_f64s(0.0);
    let minimum = simd.splat_f64s(f64::MIN_POSITIVE);
    let a = simd.splat_f64s(coefficient_a);
    let b = simd.splat_f64s(coefficient_b);
    let mut same_re = zero;
    let mut same_im = zero;
    let mut opposite_re = zero;
    let mut opposite_im = zero;
    let mut all_vectors_in_range = simd.equal_f64s(zero, zero);
    let mut all_tails_in_range = true;

    // The recurrence is normalized scalar spherical-harmonic synthesis.
    // By the addition theorem, |Y_lm| <= sqrt((2l+1)/(4*pi)); for every
    // representable usize degree this is far below scalar::BIG (2^400).
    // fill_factors rejects nonfinite or unbounded recurrence factors, so the
    // per-degree upper comparison is redundant. Keep the zero/subnormal test:
    // those values still require the checked scaled path.
    // Each vector contributes to the current coefficient while the same
    // pass advances and range-checks its recurrence state.
    for (
        (
            ((((((same_real, same_imag), opposite_real), opposite_imag), previous), basis), cosine),
            cosine_squared,
        ),
        next,
    ) in same_real_vectors
        .iter()
        .zip(same_imag_vectors.iter())
        .zip(opposite_real_vectors.iter())
        .zip(opposite_imag_vectors.iter())
        .zip(previous_vectors.iter())
        .zip(current_vectors.iter())
        .zip(cosine_vectors.iter())
        .zip(cosine_squared_vectors.iter())
        .zip(next_vectors.iter_mut())
    {
        let basis = *basis;
        same_re = simd.mul_add_f64s(*same_real, basis, same_re);
        same_im = simd.mul_add_f64s(*same_imag, basis, same_im);
        if OPPOSITE_COSINE_WEIGHTED {
            opposite_re = simd.mul_add_f64s(*opposite_real, basis, opposite_re);
        } else {
            opposite_re = simd.add_f64s(
                opposite_re,
                simd.mul_f64s(simd.mul_f64s(*opposite_real, basis), *cosine),
            );
        }
        if OPPOSITE_COSINE_WEIGHTED {
            opposite_im = simd.mul_add_f64s(*opposite_imag, basis, opposite_im);
        } else {
            opposite_im = simd.add_f64s(
                opposite_im,
                simd.mul_f64s(simd.mul_f64s(*opposite_imag, basis), *cosine),
            );
        }

        let leading = simd.mul_add_f64s(a, *cosine_squared, b);
        let next_value = simd.mul_add_f64s(leading, basis, *previous);
        *next = next_value;
        let magnitude = simd.abs_f64s(next_value);
        let is_zero = simd.equal_f64s(magnitude, zero);
        let above_minimum = simd.greater_than_or_equal_f64s(magnitude, minimum);
        let in_range = simd.or_m64s(is_zero, above_minimum);
        all_vectors_in_range = simd.and_m64s(all_vectors_in_range, in_range);
    }

    let mut same_re_sum = simd.reduce_sum_f64s(same_re);
    let mut same_im_sum = simd.reduce_sum_f64s(same_im);
    let mut opposite_re_sum = simd.reduce_sum_f64s(opposite_re);
    let mut opposite_im_sum = simd.reduce_sum_f64s(opposite_im);
    for (
        (
            ((((((same_real, same_imag), opposite_real), opposite_imag), previous), basis), cosine),
            cosine_squared,
        ),
        next,
    ) in same_real_tail
        .iter()
        .zip(same_imag_tail.iter())
        .zip(opposite_real_tail.iter())
        .zip(opposite_imag_tail.iter())
        .zip(previous_tail.iter())
        .zip(current_tail.iter())
        .zip(cosine_tail.iter())
        .zip(cosine_squared_tail.iter())
        .zip(next_tail.iter_mut())
    {
        let basis = *basis;
        let cosine = *cosine;
        same_re_sum = (*same_real).mul_add(basis, same_re_sum);
        same_im_sum = (*same_imag).mul_add(basis, same_im_sum);
        if OPPOSITE_COSINE_WEIGHTED {
            opposite_re_sum = (*opposite_real).mul_add(basis, opposite_re_sum);
            opposite_im_sum = (*opposite_imag).mul_add(basis, opposite_im_sum);
        } else {
            opposite_re_sum += (*opposite_real * basis) * cosine;
            opposite_im_sum += (*opposite_imag * basis) * cosine;
        }

        let leading = coefficient_a.mul_add(*cosine_squared, coefficient_b);
        let next_value = leading.mul_add(basis, *previous);
        *next = next_value;
        let magnitude = next_value.abs();
        all_tails_in_range &= magnitude == 0.0 || magnitude >= f64::MIN_POSITIVE;
    }

    (
        Complex64::new(same_re_sum, same_im_sum),
        Complex64::new(opposite_re_sum, opposite_im_sum),
        all_vectors_in_range,
        all_tails_in_range,
    )
}

#[allow(unexpected_cfgs)]
#[inline(always)]
fn fused_analysis_two_pair_steps_with_simd<
    S: Simd,
    const OPPOSITE_COSINE_WEIGHTED: bool,
    const CHECK_GROUP_RANGE: bool,
    const TRACK_STATE_RANGE: bool,
    const ACCUMULATE: bool,
    const SNAPSHOT_CURRENT: bool,
>(
    simd: S,
    step: FusedAnalysisTwoPairSteps<'_, OPPOSITE_COSINE_WEIGHTED>,
) -> Option<FusedAnalysisPairGroup> {
    let FusedAnalysisTwoPairSteps {
        same_real,
        same_imag,
        opposite_real,
        opposite_imag,
        basis_previous,
        basis_current,
        basis_snapshot,
        cos_theta,
        cos_theta_squared,
        first_coefficient_a,
        first_coefficient_b,
        second_coefficient_a,
        second_coefficient_b,
    } = step;
    if same_real.len() != basis_current.len()
        || same_imag.len() != basis_current.len()
        || opposite_real.len() != basis_current.len()
        || opposite_imag.len() != basis_current.len()
        || basis_previous.len() != basis_current.len()
        || cos_theta.len() != basis_current.len()
        || cos_theta_squared.len() != basis_current.len()
    {
        return None;
    }

    let (same_real_vectors, same_real_tail) = S::as_simd_f64s(same_real);
    let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(same_imag);
    let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(opposite_real);
    let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(opposite_imag);
    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(basis_previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(basis_current);
    let (snapshot_vectors, snapshot_tail) = S::as_mut_simd_f64s(basis_snapshot);
    let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
    let (cosine_squared_vectors, cosine_squared_tail) = S::as_simd_f64s(cos_theta_squared);
    if same_real_vectors.len() != current_vectors.len()
        || same_imag_vectors.len() != current_vectors.len()
        || opposite_real_vectors.len() != current_vectors.len()
        || opposite_imag_vectors.len() != current_vectors.len()
        || previous_vectors.len() != current_vectors.len()
        || (SNAPSHOT_CURRENT && snapshot_vectors.len() != current_vectors.len())
        || (!SNAPSHOT_CURRENT && !snapshot_vectors.is_empty())
        || cosine_vectors.len() != current_vectors.len()
        || cosine_squared_vectors.len() != current_vectors.len()
        || same_real_tail.len() != current_tail.len()
        || same_imag_tail.len() != current_tail.len()
        || opposite_real_tail.len() != current_tail.len()
        || opposite_imag_tail.len() != current_tail.len()
        || previous_tail.len() != current_tail.len()
        || (SNAPSHOT_CURRENT && snapshot_tail.len() != current_tail.len())
        || (!SNAPSHOT_CURRENT && !snapshot_tail.is_empty())
        || cosine_tail.len() != current_tail.len()
        || cosine_squared_tail.len() != current_tail.len()
    {
        return None;
    }

    let zero = simd.splat_f64s(0.0);
    // The experiment re-materializes coefficient splats at the recurrence
    // expressions; the default keeps the established broadcast lifetimes.
    #[cfg(not(rustsht_rematerialize_analysis_coefficients))]
    let first_a = simd.splat_f64s(first_coefficient_a);
    #[cfg(not(rustsht_rematerialize_analysis_coefficients))]
    let first_b = simd.splat_f64s(first_coefficient_b);
    #[cfg(not(rustsht_rematerialize_analysis_coefficients))]
    let second_a = simd.splat_f64s(second_coefficient_a);
    #[cfg(not(rustsht_rematerialize_analysis_coefficients))]
    let second_b = simd.splat_f64s(second_coefficient_b);
    let minimum = simd.splat_f64s(f64::MIN_POSITIVE);
    let mut first_same_re = zero;
    let mut first_same_im = zero;
    let mut first_opposite_re = zero;
    let mut first_opposite_im = zero;
    let mut second_same_re = zero;
    let mut second_same_im = zero;
    let mut second_opposite_re = zero;
    let mut second_opposite_im = zero;
    let mut vector_states_in_range = simd.equal_f64s(zero, zero);
    let mut scalar_states_in_range = true;
    let mut tails_in_range = true;

    macro_rules! update_analysis_pair_vector {
        ($previous:expr, $current:expr, $same_real:expr, $same_imag:expr,
         $opposite_real:expr, $opposite_imag:expr, $cosine:expr,
         $cosine_squared:expr) => {{
            let previous_value = *($previous);
            let current_value = *($current);
            if ACCUMULATE {
                first_same_re = simd.mul_add_f64s(*($same_real), current_value, first_same_re);
                first_same_im = simd.mul_add_f64s(*($same_imag), current_value, first_same_im);
                if OPPOSITE_COSINE_WEIGHTED {
                    first_opposite_re =
                        simd.mul_add_f64s(*($opposite_real), current_value, first_opposite_re);
                    first_opposite_im =
                        simd.mul_add_f64s(*($opposite_imag), current_value, first_opposite_im);
                } else {
                    first_opposite_re = simd.add_f64s(
                        first_opposite_re,
                        simd.mul_f64s(simd.mul_f64s(*($opposite_real), current_value), *($cosine)),
                    );
                    first_opposite_im = simd.add_f64s(
                        first_opposite_im,
                        simd.mul_f64s(simd.mul_f64s(*($opposite_imag), current_value), *($cosine)),
                    );
                }
            }

            #[cfg(rustsht_rematerialize_analysis_coefficients)]
            let first_leading = simd.mul_add_f64s(
                simd.splat_f64s(first_coefficient_a),
                *($cosine_squared),
                simd.splat_f64s(first_coefficient_b),
            );
            #[cfg(not(rustsht_rematerialize_analysis_coefficients))]
            let first_leading = simd.mul_add_f64s(first_a, *($cosine_squared), first_b);
            let first_next = simd.mul_add_f64s(first_leading, current_value, previous_value);

            if ACCUMULATE {
                second_same_re = simd.mul_add_f64s(*($same_real), first_next, second_same_re);
                second_same_im = simd.mul_add_f64s(*($same_imag), first_next, second_same_im);
                if OPPOSITE_COSINE_WEIGHTED {
                    second_opposite_re =
                        simd.mul_add_f64s(*($opposite_real), first_next, second_opposite_re);
                    second_opposite_im =
                        simd.mul_add_f64s(*($opposite_imag), first_next, second_opposite_im);
                } else {
                    second_opposite_re = simd.add_f64s(
                        second_opposite_re,
                        simd.mul_f64s(simd.mul_f64s(*($opposite_real), first_next), *($cosine)),
                    );
                    second_opposite_im = simd.add_f64s(
                        second_opposite_im,
                        simd.mul_f64s(simd.mul_f64s(*($opposite_imag), first_next), *($cosine)),
                    );
                }
            }

            #[cfg(rustsht_rematerialize_analysis_coefficients)]
            let second_leading = simd.mul_add_f64s(
                simd.splat_f64s(second_coefficient_a),
                *($cosine_squared),
                simd.splat_f64s(second_coefficient_b),
            );
            #[cfg(not(rustsht_rematerialize_analysis_coefficients))]
            let second_leading = simd.mul_add_f64s(second_a, *($cosine_squared), second_b);
            let second_next = simd.mul_add_f64s(second_leading, first_next, current_value);
            if CHECK_GROUP_RANGE && TRACK_STATE_RANGE {
                let previous_magnitude = simd.abs_f64s(first_next);
                let current_magnitude = simd.abs_f64s(second_next);
                let previous_in_range = simd.or_m64s(
                    simd.equal_f64s(previous_magnitude, zero),
                    simd.greater_than_or_equal_f64s(previous_magnitude, minimum),
                );
                let current_in_range = simd.or_m64s(
                    simd.equal_f64s(current_magnitude, zero),
                    simd.greater_than_or_equal_f64s(current_magnitude, minimum),
                );
                let states_in_range = simd.and_m64s(previous_in_range, current_in_range);
                // The scalar-reduction candidate shortens the vector-mask live
                // range at the cost of one horizontal reduction per vector chunk.
                if cfg!(rustsht_scalar_fused_analysis_pair_range_check) {
                    scalar_states_in_range &= analysis_mask_all(simd, states_in_range);
                } else {
                    vector_states_in_range = simd.and_m64s(vector_states_in_range, states_in_range);
                }
            }
            *($previous) = first_next;
            *($current) = second_next;
        }};
    }

    let mut snapshot_vectors = snapshot_vectors.iter_mut();
    for (
        ((((((previous, current), same_real), same_imag), opposite_real), opposite_imag), cosine),
        cosine_squared,
    ) in previous_vectors
        .iter_mut()
        .zip(current_vectors.iter_mut())
        .zip(same_real_vectors.iter())
        .zip(same_imag_vectors.iter())
        .zip(opposite_real_vectors.iter())
        .zip(opposite_imag_vectors.iter())
        .zip(cosine_vectors.iter())
        .zip(cosine_squared_vectors.iter())
    {
        if SNAPSHOT_CURRENT {
            let snapshot = snapshot_vectors.next()?;
            *snapshot = *current;
        }
        update_analysis_pair_vector!(
            previous,
            current,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            cosine,
            cosine_squared
        );
    }

    let (first_same, first_opposite, second_same, second_opposite) = if ACCUMULATE {
        (
            reduce_analysis_complex_pair(simd, first_same_re, first_same_im),
            reduce_analysis_complex_pair(simd, first_opposite_re, first_opposite_im),
            reduce_analysis_complex_pair(simd, second_same_re, second_same_im),
            reduce_analysis_complex_pair(simd, second_opposite_re, second_opposite_im),
        )
    } else {
        let zero = Complex64::new(0.0, 0.0);
        (zero, zero, zero, zero)
    };
    let mut first_same_re_sum = first_same.re;
    let mut first_same_im_sum = first_same.im;
    let mut first_opposite_re_sum = first_opposite.re;
    let mut first_opposite_im_sum = first_opposite.im;
    let mut second_same_re_sum = second_same.re;
    let mut second_same_im_sum = second_same.im;
    let mut second_opposite_re_sum = second_opposite.re;
    let mut second_opposite_im_sum = second_opposite.im;

    let mut snapshot_tail = snapshot_tail.iter_mut();
    for (
        ((((((previous, current), same_real), same_imag), opposite_real), opposite_imag), cosine),
        cosine_squared,
    ) in previous_tail
        .iter_mut()
        .zip(current_tail.iter_mut())
        .zip(same_real_tail.iter())
        .zip(same_imag_tail.iter())
        .zip(opposite_real_tail.iter())
        .zip(opposite_imag_tail.iter())
        .zip(cosine_tail.iter())
        .zip(cosine_squared_tail.iter())
    {
        let previous_value = *previous;
        let current_value = *current;
        if SNAPSHOT_CURRENT {
            let snapshot = snapshot_tail.next()?;
            *snapshot = current_value;
        }
        if ACCUMULATE {
            first_same_re_sum = (*same_real).mul_add(current_value, first_same_re_sum);
            first_same_im_sum = (*same_imag).mul_add(current_value, first_same_im_sum);
            if OPPOSITE_COSINE_WEIGHTED {
                first_opposite_re_sum =
                    (*opposite_real).mul_add(current_value, first_opposite_re_sum);
                first_opposite_im_sum =
                    (*opposite_imag).mul_add(current_value, first_opposite_im_sum);
            } else {
                first_opposite_re_sum += (*opposite_real * current_value) * *cosine;
                first_opposite_im_sum += (*opposite_imag * current_value) * *cosine;
            }
        }

        let first_next = first_coefficient_a
            .mul_add(*cosine_squared, first_coefficient_b)
            .mul_add(current_value, previous_value);
        let first_magnitude = first_next.abs();
        tails_in_range &= first_magnitude == 0.0 || first_magnitude >= f64::MIN_POSITIVE;

        if ACCUMULATE {
            second_same_re_sum = (*same_real).mul_add(first_next, second_same_re_sum);
            second_same_im_sum = (*same_imag).mul_add(first_next, second_same_im_sum);
            if OPPOSITE_COSINE_WEIGHTED {
                second_opposite_re_sum =
                    (*opposite_real).mul_add(first_next, second_opposite_re_sum);
                second_opposite_im_sum =
                    (*opposite_imag).mul_add(first_next, second_opposite_im_sum);
            } else {
                second_opposite_re_sum += (*opposite_real * first_next) * *cosine;
                second_opposite_im_sum += (*opposite_imag * first_next) * *cosine;
            }
        }

        let second_next = second_coefficient_a
            .mul_add(*cosine_squared, second_coefficient_b)
            .mul_add(first_next, current_value);
        let second_magnitude = second_next.abs();
        tails_in_range &= second_magnitude == 0.0 || second_magnitude >= f64::MIN_POSITIVE;
        *previous = first_next;
        *current = second_next;
    }

    let states_in_range = if CHECK_GROUP_RANGE && TRACK_STATE_RANGE {
        let vectors_in_range = if cfg!(rustsht_scalar_fused_analysis_pair_range_check) {
            scalar_states_in_range
        } else {
            analysis_mask_all(simd, vector_states_in_range)
        };
        let tails_in_range = previous_tail.iter().zip(current_tail.iter()).all(
            |(&previous_value, &current_value)| {
                (previous_value == 0.0 || previous_value.abs() >= f64::MIN_POSITIVE)
                    && (current_value == 0.0 || current_value.abs() >= f64::MIN_POSITIVE)
            },
        );
        vectors_in_range && tails_in_range
    } else {
        true
    };
    Some(FusedAnalysisPairGroup {
        first_same: Complex64::new(first_same_re_sum, first_same_im_sum),
        first_opposite: Complex64::new(first_opposite_re_sum, first_opposite_im_sum),
        second_same: Complex64::new(second_same_re_sum, second_same_im_sum),
        second_opposite: Complex64::new(second_opposite_re_sum, second_opposite_im_sum),
        states_in_range,
        tails_in_range,
    })
}

/// Checks the two recurrence states produced by a fused pair group after its
/// register-heavy accumulation pass has completed. This keeps the safety
/// check out of the multi-accumulator loop while preserving fallback on any
/// nonzero subnormal lane.
#[allow(unexpected_cfgs)]
#[inline(always)]
fn analysis_pair_group_state_in_range<S: Simd>(simd: S, previous: &[f64], current: &[f64]) -> bool {
    if previous.len() != current.len() {
        return false;
    }
    let (previous_vectors, previous_tail) = S::as_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_simd_f64s(current);
    if previous_vectors.len() != current_vectors.len() || previous_tail.len() != current_tail.len()
    {
        return false;
    }

    let zero = simd.splat_f64s(0.0);
    let minimum = simd.splat_f64s(f64::MIN_POSITIVE);
    #[cfg(not(rustsht_split_analysis_range_masks))]
    let mut vectors_in_range = simd.equal_f64s(zero, zero);
    #[cfg(not(rustsht_split_analysis_range_masks))]
    for (&previous_value, &current_value) in previous_vectors.iter().zip(current_vectors) {
        vectors_in_range = simd.and_m64s(
            vectors_in_range,
            analysis_pair_state_range_mask(simd, previous_value, current_value, zero, minimum),
        );
    }
    // Keep two independent mask chains in the scan loop. The AVX2 loop used
    // one loop-carried YMM AND chain after checking each recurrence group.
    #[cfg(rustsht_split_analysis_range_masks)]
    let mut first_vectors_in_range = simd.equal_f64s(zero, zero);
    #[cfg(rustsht_split_analysis_range_masks)]
    let mut second_vectors_in_range = simd.equal_f64s(zero, zero);
    #[cfg(rustsht_split_analysis_range_masks)]
    for pair in 0..previous_vectors.len() / 2 {
        let first = pair * 2;
        first_vectors_in_range = simd.and_m64s(
            first_vectors_in_range,
            analysis_pair_state_range_mask(
                simd,
                previous_vectors[first],
                current_vectors[first],
                zero,
                minimum,
            ),
        );
        let second = first + 1;
        second_vectors_in_range = simd.and_m64s(
            second_vectors_in_range,
            analysis_pair_state_range_mask(
                simd,
                previous_vectors[second],
                current_vectors[second],
                zero,
                minimum,
            ),
        );
    }
    #[cfg(rustsht_split_analysis_range_masks)]
    if previous_vectors.len() % 2 != 0 {
        let last = previous_vectors.len() - 1;
        first_vectors_in_range = simd.and_m64s(
            first_vectors_in_range,
            analysis_pair_state_range_mask(
                simd,
                previous_vectors[last],
                current_vectors[last],
                zero,
                minimum,
            ),
        );
    }
    #[cfg(rustsht_split_analysis_range_masks)]
    let vectors_in_range = simd.and_m64s(first_vectors_in_range, second_vectors_in_range);
    if !analysis_mask_all(simd, vectors_in_range) {
        return false;
    }

    previous_tail
        .iter()
        .zip(current_tail)
        .all(|(&previous_value, &current_value)| {
            (previous_value == 0.0 || previous_value.abs() >= f64::MIN_POSITIVE)
                && (current_value == 0.0 || current_value.abs() >= f64::MIN_POSITIVE)
        })
}

#[inline(always)]
fn analysis_pair_state_range_mask<S: Simd>(
    simd: S,
    previous: S::f64s,
    current: S::f64s,
    zero: S::f64s,
    minimum: S::f64s,
) -> S::m64s {
    let previous_magnitude = simd.abs_f64s(previous);
    let current_magnitude = simd.abs_f64s(current);
    let previous_in_range = simd.or_m64s(
        simd.equal_f64s(previous_magnitude, zero),
        simd.greater_than_or_equal_f64s(previous_magnitude, minimum),
    );
    let current_in_range = simd.or_m64s(
        simd.equal_f64s(current_magnitude, zero),
        simd.greater_than_or_equal_f64s(current_magnitude, minimum),
    );
    simd.and_m64s(previous_in_range, current_in_range)
}

#[inline(always)]
fn store_analysis_pair(
    coefficients: &mut [Complex64],
    offset: usize,
    has_odd: bool,
    same: Complex64,
    opposite: Complex64,
) {
    coefficients[offset] = same;
    if has_odd {
        coefficients[offset + 1] = opposite;
    }
}

impl<
    const OPPOSITE_COSINE_WEIGHTED: bool,
    const CHECK_PAIR_GROUP_RANGE: bool,
    const FUSE_PAIR_GROUP_RANGE_CHECK: bool,
    const ACCUMULATE: bool,
    const SEQUENTIAL_SECOND_CHANNEL: bool,
> WithSimd
    for FusedScalarAnalysisPair<
        '_,
        OPPOSITE_COSINE_WEIGHTED,
        CHECK_PAIR_GROUP_RANGE,
        FUSE_PAIR_GROUP_RANGE_CHECK,
        ACCUMULATE,
        SEQUENTIAL_SECOND_CHANNEL,
    >
{
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedScalarAnalysisPair {
            m,
            lmax,
            range_check_was_exercised,
            pair_start,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            second_channel,
            previous,
            current,
            next,
        } = self;
        let mut previous = previous;
        let mut current = current;
        let mut next = next;
        let mut second_channel = second_channel;
        let lane_count = current.len();
        if previous.len() != lane_count
            || next.len() != lane_count
            || cos_theta.len() != lane_count
            || cos_theta_squared.len() != lane_count
            || same_real.len() != lane_count
            || same_imag.len() != lane_count
            || opposite_real.len() != lane_count
            || opposite_imag.len() != lane_count
        {
            return false;
        }
        if SEQUENTIAL_SECOND_CHANNEL != second_channel.is_some() {
            return false;
        }
        if let Some(second) = second_channel.as_ref()
            && (second.same_real.len() != lane_count
                || second.same_imag.len() != lane_count
                || second.opposite_real.len() != lane_count
                || second.opposite_imag.len() != lane_count
                || second.staged_coefficients.len() < lmax - m + 1)
        {
            return false;
        }
        let zero = simd.splat_f64s(0.0);
        let mut all_vectors_in_range = simd.equal_f64s(zero, zero);
        let mut all_tails_in_range = true;

        let pair_count = (lmax - m + 2) / 2;
        let recurrence_stop = pair_count.saturating_sub(1);
        if CHECK_PAIR_GROUP_RANGE && pair_start < recurrence_stop.saturating_sub(1) {
            *range_check_was_exercised = true;
        }
        let mut pair_index = pair_start;
        while pair_index < recurrence_stop.saturating_sub(1) {
            let Some(group) = fused_analysis_two_pair_steps_with_simd::<
                S,
                OPPOSITE_COSINE_WEIGHTED,
                CHECK_PAIR_GROUP_RANGE,
                FUSE_PAIR_GROUP_RANGE_CHECK,
                ACCUMULATE,
                SEQUENTIAL_SECOND_CHANNEL,
            >(
                simd,
                FusedAnalysisTwoPairSteps {
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    basis_previous: previous,
                    basis_current: current,
                    basis_snapshot: if SEQUENTIAL_SECOND_CHANNEL {
                        &mut next[..]
                    } else {
                        &mut []
                    },
                    cos_theta,
                    cos_theta_squared,
                    first_coefficient_a: coefficient_a[pair_index],
                    first_coefficient_b: coefficient_b[pair_index],
                    second_coefficient_a: coefficient_a[pair_index + 1],
                    second_coefficient_b: coefficient_b[pair_index + 1],
                },
            ) else {
                return false;
            };
            if CHECK_PAIR_GROUP_RANGE {
                let group_in_range = if FUSE_PAIR_GROUP_RANGE_CHECK {
                    group.states_in_range
                } else {
                    analysis_pair_group_state_in_range(simd, previous, current)
                };
                if !group_in_range {
                    #[cfg(test)]
                    ANALYSIS_PAIR_GROUP_RANGE_DECLINES
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return false;
                }
            }
            all_tails_in_range &= group.tails_in_range;

            let first_l = m + 2 * pair_index;
            let first_offset = first_l - m;
            store_analysis_pair(
                staged_coefficients,
                first_offset,
                first_l < lmax,
                group.first_same,
                group.first_opposite,
            );

            let second_l = first_l + 2;
            let second_offset = second_l - m;
            store_analysis_pair(
                staged_coefficients,
                second_offset,
                second_l < lmax,
                group.second_same,
                group.second_opposite,
            );
            if let Some(second) = second_channel.as_mut() {
                let [(first_same, first_opposite), (second_same, second_opposite)] =
                    AnalysisTwoBasisAccumulation {
                        same_real: second.same_real,
                        same_imag: second.same_imag,
                        opposite_real: second.opposite_real,
                        opposite_imag: second.opposite_imag,
                        first_basis: next,
                        second_basis: previous,
                    }
                    .with_simd(simd);
                store_analysis_pair(
                    second.staged_coefficients,
                    first_offset,
                    first_l < lmax,
                    first_same,
                    first_opposite,
                );
                store_analysis_pair(
                    second.staged_coefficients,
                    second_offset,
                    second_l < lmax,
                    second_same,
                    second_opposite,
                );
            }
            pair_index += 2;
        }

        while pair_index < recurrence_stop {
            let l = m + 2 * pair_index;
            let offset = l - m;
            if let Some(second) = second_channel.as_mut() {
                let (same_sum, opposite_sum) =
                    AnalysisPairAccumulation::<OPPOSITE_COSINE_WEIGHTED> {
                        same_real: second.same_real,
                        same_imag: second.same_imag,
                        opposite_real: second.opposite_real,
                        opposite_imag: second.opposite_imag,
                        basis: current,
                        cos_theta,
                    }
                    .with_simd(simd);
                store_analysis_pair(
                    second.staged_coefficients,
                    offset,
                    l < lmax,
                    same_sum,
                    opposite_sum,
                );
            }
            let (same_sum, opposite_sum, pair_vectors_in_range, pair_tails_in_range) =
                fused_analysis_pair_step_with_simd::<S, OPPOSITE_COSINE_WEIGHTED>(
                    simd,
                    FusedAnalysisPairStep {
                        same_real,
                        same_imag,
                        opposite_real,
                        opposite_imag,
                        basis_previous: previous,
                        basis_current: current,
                        cos_theta,
                        cos_theta_squared,
                        coefficient_a: coefficient_a[pair_index],
                        coefficient_b: coefficient_b[pair_index],
                        basis_next: next,
                    },
                );
            all_vectors_in_range = simd.and_m64s(all_vectors_in_range, pair_vectors_in_range);
            all_tails_in_range &= pair_tails_in_range;
            if ACCUMULATE {
                store_analysis_pair(
                    staged_coefficients,
                    offset,
                    l < lmax,
                    same_sum,
                    opposite_sum,
                );
            } else {
                store_analysis_pair(
                    staged_coefficients,
                    offset,
                    l < lmax,
                    Complex64::new(0.0, 0.0),
                    Complex64::new(0.0, 0.0),
                );
            }
            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
            pair_index += 1;
        }

        let last_pair = pair_count - 1;
        let l = m + 2 * last_pair;
        let offset = l - m;
        let (same_sum, opposite_sum) = if ACCUMULATE {
            AnalysisPairAccumulation::<OPPOSITE_COSINE_WEIGHTED> {
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                basis: current,
                cos_theta,
            }
            .with_simd(simd)
        } else {
            let zero = Complex64::new(0.0, 0.0);
            (zero, zero)
        };
        if let Some(second) = second_channel.as_mut() {
            let (same_sum, opposite_sum) = AnalysisPairAccumulation::<OPPOSITE_COSINE_WEIGHTED> {
                same_real: second.same_real,
                same_imag: second.same_imag,
                opposite_real: second.opposite_real,
                opposite_imag: second.opposite_imag,
                basis: current,
                cos_theta,
            }
            .with_simd(simd);
            store_analysis_pair(
                second.staged_coefficients,
                offset,
                l < lmax,
                same_sum,
                opposite_sum,
            );
        }
        store_analysis_pair(
            staged_coefficients,
            offset,
            l < lmax,
            same_sum,
            opposite_sum,
        );
        analysis_mask_all(simd, all_vectors_in_range) && all_tails_in_range
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The scalar analysis dispatch binds one channel's spectra to shared recurrence geometry and scratch."
)]
#[inline(always)]
fn dispatch_fused_scalar_analysis_pair<const CHECK_PAIR_GROUP_RANGE: bool>(
    arch: Arch,
    m: usize,
    lmax: usize,
    range_check_was_exercised: &mut bool,
    opposite_preweighted: bool,
    cos_theta: &[f64],
    cos_theta_squared: &[f64],
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &[f64],
    opposite_imag: &[f64],
    staged_coefficients: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> bool {
    if opposite_preweighted {
        dispatch_fused_scalar_analysis_pair_weighted::<true, CHECK_PAIR_GROUP_RANGE>(
            arch,
            m,
            lmax,
            range_check_was_exercised,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            previous,
            current,
            next,
        )
    } else {
        dispatch_fused_scalar_analysis_pair_weighted::<false, CHECK_PAIR_GROUP_RANGE>(
            arch,
            m,
            lmax,
            range_check_was_exercised,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            previous,
            current,
            next,
        )
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The scalar analysis dispatch binds one channel's spectra to shared recurrence geometry and scratch."
)]
#[inline(always)]
fn dispatch_fused_scalar_analysis_pair_weighted<
    const OPPOSITE_COSINE_WEIGHTED: bool,
    const CHECK_PAIR_GROUP_RANGE: bool,
>(
    arch: Arch,
    m: usize,
    lmax: usize,
    range_check_was_exercised: &mut bool,
    cos_theta: &[f64],
    cos_theta_squared: &[f64],
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &[f64],
    opposite_imag: &[f64],
    staged_coefficients: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> bool {
    arch.dispatch(
        FusedScalarAnalysisPair::<OPPOSITE_COSINE_WEIGHTED, CHECK_PAIR_GROUP_RANGE> {
            m,
            lmax,
            range_check_was_exercised,
            pair_start: 0,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            second_channel: None,
            previous,
            current,
            next,
        },
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The shared recurrence keeps Q inputs and staged U reductions in separate channel rows."
)]
#[allow(unexpected_cfgs)]
#[inline(always)]
fn dispatch_fused_scalar_analysis_pair_two_channels<
    const OPPOSITE_COSINE_WEIGHTED: bool,
    const CHECK_PAIR_GROUP_RANGE: bool,
>(
    arch: Arch,
    m: usize,
    lmax: usize,
    range_check_was_exercised: &mut bool,
    cos_theta: &[f64],
    cos_theta_squared: &[f64],
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    q_same_real: &[f64],
    q_same_imag: &[f64],
    q_opposite_real: &[f64],
    q_opposite_imag: &[f64],
    u_same_real: &[f64],
    u_same_imag: &[f64],
    u_opposite_real: &[f64],
    u_opposite_imag: &[f64],
    staged_q: &mut [Complex64],
    staged_u: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> bool {
    arch.dispatch(FusedScalarAnalysisPair::<
        OPPOSITE_COSINE_WEIGHTED,
        CHECK_PAIR_GROUP_RANGE,
        { cfg!(rustsht_fused_analysis_pair_range_check) },
        true,
        true,
    > {
        m,
        lmax,
        range_check_was_exercised,
        pair_start: 0,
        cos_theta,
        cos_theta_squared,
        coefficient_a,
        coefficient_b,
        same_real: q_same_real,
        same_imag: q_same_imag,
        opposite_real: q_opposite_real,
        opposite_imag: q_opposite_imag,
        staged_coefficients: staged_q,
        second_channel: Some(FusedScalarAnalysisPairSecondChannel {
            same_real: u_same_real,
            same_imag: u_same_imag,
            opposite_real: u_opposite_real,
            opposite_imag: u_opposite_imag,
            staged_coefficients: staged_u,
        }),
        previous,
        current,
        next,
    })
}

/// Computes a normal-range scalar analysis block for two spin-conversion channels.
///
/// Returns `false` when the recurrence requires binary scaling. Any staged
/// coefficients must then be discarded by the caller.
#[expect(
    clippy::too_many_arguments,
    reason = "The two-channel analysis block receives lane spectra and worker-owned recurrence scratch explicitly."
)]
#[cfg(test)]
pub(crate) fn fused_scalar_analysis_pair_channels_block_dispatched(
    arch: Arch,
    sequential_channels: bool,
    opposite_preweighted: bool,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: [&[f64]; 2],
    same_imag: [&[f64]; 2],
    opposite_real: [&[f64]; 2],
    opposite_imag: [&[f64]; 2],
    staged_first: &mut [Complex64],
    staged_second: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    let mut range_check_was_exercised = false;
    fused_scalar_analysis_pair_channels_block_with_range_proof(
        arch,
        sequential_channels,
        opposite_preweighted,
        false,
        false,
        &mut range_check_was_exercised,
        m,
        lmax,
        sin_theta,
        cos_theta,
        sectoral_norm,
        coefficient_a,
        coefficient_b,
        same_real,
        same_imag,
        opposite_real,
        opposite_imag,
        staged_first,
        staged_second,
        previous,
        current,
        next,
        exponents,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The checked shared-recurrence trial stages both scalar channels independently."
)]
#[inline(never)]
fn try_fused_scalar_analysis_pair_shared_recurrence(
    arch: Arch,
    range_check_is_proven_safe: bool,
    range_check_was_exercised: &mut bool,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: [&[f64]; 2],
    same_imag: [&[f64]; 2],
    opposite_real: [&[f64]; 2],
    opposite_imag: [&[f64]; 2],
    staged_first: &mut [Complex64],
    staged_second: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
) -> Result<Option<bool>, ShtError> {
    let mut q_opposite_real = [0.0; LATITUDE_BLOCK_SIZE];
    let mut q_opposite_imag = [0.0; LATITUDE_BLOCK_SIZE];
    let mut u_opposite_real = [0.0; LATITUDE_BLOCK_SIZE];
    let mut u_opposite_imag = [0.0; LATITUDE_BLOCK_SIZE];
    q_opposite_real[..sin_theta.len()].copy_from_slice(opposite_real[0]);
    q_opposite_imag[..sin_theta.len()].copy_from_slice(opposite_imag[0]);
    u_opposite_real[..sin_theta.len()].copy_from_slice(opposite_real[1]);
    u_opposite_imag[..sin_theta.len()].copy_from_slice(opposite_imag[1]);
    let q_opposite_prepared = prepare_analysis_opposite(
        &mut q_opposite_real[..sin_theta.len()],
        &mut q_opposite_imag[..sin_theta.len()],
        cos_theta,
    );
    let u_opposite_prepared = prepare_analysis_opposite(
        &mut u_opposite_real[..sin_theta.len()],
        &mut u_opposite_imag[..sin_theta.len()],
        cos_theta,
    );
    if !q_opposite_prepared || !u_opposite_prepared {
        return Ok(None);
    }
    if !initialize_analysis_sectoral_lanes(
        arch,
        m,
        lmax,
        sin_theta,
        sectoral_norm,
        &mut previous[..sin_theta.len()],
        &mut current[..sin_theta.len()],
        &mut exponents[..sin_theta.len()],
    )? {
        return Ok(Some(false));
    }

    let mut cos_theta_squared = [0.0; LATITUDE_BLOCK_SIZE];
    for (squared, &cosine) in cos_theta_squared.iter_mut().zip(cos_theta) {
        *squared = cosine * cosine;
    }
    let mut shared_range_check_was_exercised = false;
    let recurrence_succeeded = if range_check_is_proven_safe {
        dispatch_fused_scalar_analysis_pair_two_channels::<true, false>(
            arch,
            m,
            lmax,
            &mut shared_range_check_was_exercised,
            cos_theta,
            &cos_theta_squared[..sin_theta.len()],
            coefficient_a,
            coefficient_b,
            same_real[0],
            same_imag[0],
            &q_opposite_real[..sin_theta.len()],
            &q_opposite_imag[..sin_theta.len()],
            same_real[1],
            same_imag[1],
            &u_opposite_real[..sin_theta.len()],
            &u_opposite_imag[..sin_theta.len()],
            staged_first,
            staged_second,
            &mut previous[..sin_theta.len()],
            &mut current[..sin_theta.len()],
            &mut next[..sin_theta.len()],
        )
    } else {
        dispatch_fused_scalar_analysis_pair_two_channels::<
            true,
            { !cfg!(rustsht_unchecked_analysis_pair_range) },
        >(
            arch,
            m,
            lmax,
            &mut shared_range_check_was_exercised,
            cos_theta,
            &cos_theta_squared[..sin_theta.len()],
            coefficient_a,
            coefficient_b,
            same_real[0],
            same_imag[0],
            &q_opposite_real[..sin_theta.len()],
            &q_opposite_imag[..sin_theta.len()],
            same_real[1],
            same_imag[1],
            &u_opposite_real[..sin_theta.len()],
            &u_opposite_imag[..sin_theta.len()],
            staged_first,
            staged_second,
            &mut previous[..sin_theta.len()],
            &mut current[..sin_theta.len()],
            &mut next[..sin_theta.len()],
        )
    };
    *range_check_was_exercised |= shared_range_check_was_exercised;

    let pair_count = (lmax - m + 2) / 2;
    let single_pair_tail_writes_next = !pair_count.saturating_sub(1).is_multiple_of(2);
    let state_within_upper_bound = previous[..sin_theta.len()]
        .iter()
        .chain(&current[..sin_theta.len()])
        .all(|value| value.is_finite() && value.abs() <= scalar::BIG)
        && (!single_pair_tail_writes_next
            || next[..sin_theta.len()]
                .iter()
                .all(|value| value.is_finite() && value.abs() <= scalar::BIG));
    Ok(Some(recurrence_succeeded && state_within_upper_bound))
}

#[expect(
    clippy::too_many_arguments,
    reason = "The two-channel analysis block receives lane spectra and worker-owned recurrence scratch explicitly."
)]
pub(crate) fn fused_scalar_analysis_pair_channels_block_with_range_proof(
    arch: Arch,
    sequential_channels: bool,
    opposite_preweighted: bool,
    shared_recurrence_sequential_accumulation: bool,
    range_check_is_proven_safe: bool,
    range_check_was_exercised: &mut bool,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: [&[f64]; 2],
    same_imag: [&[f64]; 2],
    opposite_real: [&[f64]; 2],
    opposite_imag: [&[f64]; 2],
    staged_first: &mut [Complex64],
    staged_second: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    *range_check_was_exercised = false;
    debug_assert!(m <= lmax);
    debug_assert!(!opposite_preweighted || sequential_channels);
    debug_assert!(
        !shared_recurrence_sequential_accumulation || (sequential_channels && opposite_preweighted)
    );
    let coefficient_count = lmax - m + 1;
    debug_assert!(sin_theta.len() <= LATITUDE_BLOCK_SIZE);
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    for channel in 0..2 {
        debug_assert_eq!(sin_theta.len(), same_real[channel].len());
        debug_assert_eq!(sin_theta.len(), same_imag[channel].len());
        debug_assert_eq!(sin_theta.len(), opposite_real[channel].len());
        debug_assert_eq!(sin_theta.len(), opposite_imag[channel].len());
    }
    debug_assert!(staged_first.len() >= coefficient_count);
    debug_assert!(staged_second.len() >= coefficient_count);
    debug_assert!(previous.len() >= sin_theta.len());
    debug_assert!(current.len() >= sin_theta.len());
    debug_assert!(next.len() >= sin_theta.len());
    debug_assert!(exponents.len() >= sin_theta.len());

    if sin_theta.is_empty() {
        return Ok(false);
    }

    if shared_recurrence_sequential_accumulation
        && let Some(fused) = try_fused_scalar_analysis_pair_shared_recurrence(
            arch,
            range_check_is_proven_safe,
            range_check_was_exercised,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            &mut staged_first[..coefficient_count],
            &mut staged_second[..coefficient_count],
            previous,
            current,
            next,
            exponents,
        )?
    {
        return Ok(fused);
    }

    if !sequential_channels {
        if !initialize_analysis_sectoral_lanes(
            arch,
            m,
            lmax,
            sin_theta,
            sectoral_norm,
            &mut previous[..sin_theta.len()],
            &mut current[..sin_theta.len()],
            &mut exponents[..sin_theta.len()],
        )? {
            return Ok(false);
        }
        return Ok(arch.dispatch(FusedScalarAnalysisPairChannels {
            m,
            lmax,
            cos_theta,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_first: &mut staged_first[..coefficient_count],
            staged_second: &mut staged_second[..coefficient_count],
            previous: &mut previous[..sin_theta.len()],
            current: &mut current[..sin_theta.len()],
            next: &mut next[..sin_theta.len()],
        }));
    }

    let mut cos_theta_squared = [0.0; LATITUDE_BLOCK_SIZE];
    for (squared, &cosine) in cos_theta_squared.iter_mut().zip(cos_theta) {
        *squared = cosine * cosine;
    }

    // DUCC's spin-1 analysis runs one scalar recurrence per converted channel.
    // Keep the reflected inputs packed once, then reuse the checked single-channel
    // kernel with its smaller accumulator set for each row.
    let mut channels_succeeded = true;
    let mut first_channel_proved_recurrence_range = range_check_is_proven_safe;
    // Weight each opposite input once per lane; retain the originals for fallback.
    let mut weighted_opposite_real = [0.0; LATITUDE_BLOCK_SIZE];
    let mut weighted_opposite_imag = [0.0; LATITUDE_BLOCK_SIZE];
    for (channel, staged) in [
        (0, &mut staged_first[..coefficient_count]),
        (1, &mut staged_second[..coefficient_count]),
    ] {
        if !initialize_analysis_sectoral_lanes(
            arch,
            m,
            lmax,
            sin_theta,
            sectoral_norm,
            &mut previous[..sin_theta.len()],
            &mut current[..sin_theta.len()],
            &mut exponents[..sin_theta.len()],
        )? {
            channels_succeeded = false;
            continue;
        }
        let (opposite_real, opposite_imag, channel_opposite_preweighted) = if opposite_preweighted {
            weighted_opposite_real[..sin_theta.len()].copy_from_slice(opposite_real[channel]);
            weighted_opposite_imag[..sin_theta.len()].copy_from_slice(opposite_imag[channel]);
            if prepare_analysis_opposite(
                &mut weighted_opposite_real[..sin_theta.len()],
                &mut weighted_opposite_imag[..sin_theta.len()],
                cos_theta,
            ) {
                (
                    &weighted_opposite_real[..sin_theta.len()],
                    &weighted_opposite_imag[..sin_theta.len()],
                    true,
                )
            } else {
                (opposite_real[channel], opposite_imag[channel], false)
            }
        } else {
            (opposite_real[channel], opposite_imag[channel], false)
        };
        let mut channel_range_check_was_exercised = false;
        let recurrence_succeeded = if first_channel_proved_recurrence_range {
            dispatch_fused_scalar_analysis_pair::<false>(
                arch,
                m,
                lmax,
                &mut channel_range_check_was_exercised,
                channel_opposite_preweighted,
                cos_theta,
                &cos_theta_squared[..sin_theta.len()],
                coefficient_a,
                coefficient_b,
                same_real[channel],
                same_imag[channel],
                opposite_real,
                opposite_imag,
                staged,
                &mut previous[..sin_theta.len()],
                &mut current[..sin_theta.len()],
                &mut next[..sin_theta.len()],
            )
        } else {
            dispatch_fused_scalar_analysis_pair::<{ !cfg!(rustsht_unchecked_analysis_pair_range) }>(
                arch,
                m,
                lmax,
                &mut channel_range_check_was_exercised,
                channel_opposite_preweighted,
                cos_theta,
                &cos_theta_squared[..sin_theta.len()],
                coefficient_a,
                coefficient_b,
                same_real[channel],
                same_imag[channel],
                opposite_real,
                opposite_imag,
                staged,
                &mut previous[..sin_theta.len()],
                &mut current[..sin_theta.len()],
                &mut next[..sin_theta.len()],
            )
        };
        *range_check_was_exercised |= channel_range_check_was_exercised;
        // The two-pair groups advance `previous` and `current` directly and
        // leave `next` untouched. Only the single-pair tail writes that slot.
        let pair_count = (lmax - m + 2) / 2;
        let single_pair_tail_writes_next = !pair_count.saturating_sub(1).is_multiple_of(2);
        let state_within_upper_bound = previous[..sin_theta.len()]
            .iter()
            .chain(&current[..sin_theta.len()])
            .all(|value| value.is_finite() && value.abs() <= scalar::BIG)
            && (!single_pair_tail_writes_next
                || next[..sin_theta.len()]
                    .iter()
                    .all(|value| value.is_finite() && value.abs() <= scalar::BIG));
        let channel_succeeded = recurrence_succeeded && state_within_upper_bound;
        if channel == 0 {
            // The basis recurrence depends only on order and latitude geometry.
            // A completed checked first-channel traversal therefore proves that
            // the reinitialized recurrence for channel two stays in range too.
            first_channel_proved_recurrence_range = channel_succeeded
                && (channel_range_check_was_exercised || range_check_is_proven_safe);
        }
        channels_succeeded &= channel_succeeded;
    }
    Ok(channels_succeeded)
}

struct FusedScalarAnalysisPairChannels<'a> {
    m: usize,
    lmax: usize,
    cos_theta: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    same_real: [&'a [f64]; 2],
    same_imag: [&'a [f64]; 2],
    opposite_real: [&'a [f64]; 2],
    opposite_imag: [&'a [f64]; 2],
    staged_first: &'a mut [Complex64],
    staged_second: &'a mut [Complex64],
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
}

impl WithSimd for FusedScalarAnalysisPairChannels<'_> {
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedScalarAnalysisPairChannels {
            m,
            lmax,
            cos_theta,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_first,
            staged_second,
            previous,
            current,
            next,
        } = self;
        let mut previous = previous;
        let mut current = current;
        let mut next = next;

        for (pair_index, l) in (m..=lmax).step_by(2).enumerate() {
            let offset = l - m;
            let channel_sums = AnalysisPairChannelsAccumulation {
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                basis: current,
                cos_theta,
            }
            .with_simd(simd);
            staged_first[offset] = channel_sums[0].0;
            staged_second[offset] = channel_sums[1].0;
            if l < lmax {
                staged_first[offset + 1] = channel_sums[0].1;
                staged_second[offset + 1] = channel_sums[1].1;
            }

            if l + 2 > lmax {
                break;
            }

            let (current_vectors, current_tail) = S::as_simd_f64s(current);
            let (previous_vectors, previous_tail) = S::as_simd_f64s(previous);
            let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
            let (next_vectors, next_tail) = S::as_mut_simd_f64s(next);
            let a = simd.splat_f64s(coefficient_a[pair_index]);
            let b = simd.splat_f64s(coefficient_b[pair_index]);
            let big = simd.splat_f64s(scalar::BIG);
            let zeros = simd.splat_f64s(0.0);
            debug_assert_eq!(current_vectors.len(), previous_vectors.len());
            debug_assert_eq!(current_vectors.len(), cosine_vectors.len());
            debug_assert_eq!(current_vectors.len(), next_vectors.len());
            // The previous/current pair is normal on entry. Check each new
            // next value before it becomes current on the following iteration.
            for index in 0..current_vectors.len() {
                let cosine_squared = simd.mul_f64s(cosine_vectors[index], cosine_vectors[index]);
                let leading = simd.mul_add_f64s(a, cosine_squared, b);
                let next_value =
                    simd.mul_add_f64s(leading, current_vectors[index], previous_vectors[index]);
                next_vectors[index] = next_value;

                let magnitude = simd.abs_f64s(next_value);
                let below_upper_bound = simd.less_than_or_equal_f64s(magnitude, big);
                let is_zero = simd.equal_f64s(magnitude, zeros);
                let lower_check = simd.select_f64s(is_zero, big, magnitude);
                let in_range = simd.select_f64s(below_upper_bound, lower_check, zeros);
                if simd.reduce_min_f64s(in_range) < scalar::SMALL {
                    return false;
                }
            }
            for index in 0..current_tail.len() {
                let cosine_squared = cosine_tail[index] * cosine_tail[index];
                let leading =
                    coefficient_a[pair_index].mul_add(cosine_squared, coefficient_b[pair_index]);
                let next_value = leading.mul_add(current_tail[index], previous_tail[index]);
                next_tail[index] = next_value;
                let magnitude = next_value.abs();
                if !next_value.is_finite()
                    || magnitude > scalar::BIG
                    || (magnitude != 0.0 && magnitude < scalar::SMALL)
                {
                    return false;
                }
            }

            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
        }
        true
    }
}

struct AnalysisPairAccumulation<'a, const OPPOSITE_COSINE_WEIGHTED: bool = false> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis: &'a [f64],
    cos_theta: &'a [f64],
}

struct AnalysisTwoBasisAccumulation<'a> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    first_basis: &'a [f64],
    second_basis: &'a [f64],
}

impl WithSimd for AnalysisTwoBasisAccumulation<'_> {
    type Output = [(Complex64, Complex64); 2];

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let (same_real_vectors, same_real_tail) = S::as_simd_f64s(self.same_real);
        let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(self.same_imag);
        let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(self.opposite_real);
        let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(self.opposite_imag);
        let (first_basis_vectors, first_basis_tail) = S::as_simd_f64s(self.first_basis);
        let (second_basis_vectors, second_basis_tail) = S::as_simd_f64s(self.second_basis);
        debug_assert_eq!(same_real_vectors.len(), same_imag_vectors.len());
        debug_assert_eq!(same_real_vectors.len(), opposite_real_vectors.len());
        debug_assert_eq!(same_real_vectors.len(), opposite_imag_vectors.len());
        debug_assert_eq!(same_real_vectors.len(), first_basis_vectors.len());
        debug_assert_eq!(same_real_vectors.len(), second_basis_vectors.len());

        let zero = simd.splat_f64s(0.0);
        let mut first_sums = [zero; 4];
        let mut second_sums = [zero; 4];
        for (
            ((((same_real, same_imag), opposite_real), opposite_imag), first_basis),
            second_basis,
        ) in same_real_vectors
            .iter()
            .zip(same_imag_vectors)
            .zip(opposite_real_vectors)
            .zip(opposite_imag_vectors)
            .zip(first_basis_vectors)
            .zip(second_basis_vectors)
        {
            first_sums[0] = simd.mul_add_f64s(*same_real, *first_basis, first_sums[0]);
            first_sums[1] = simd.mul_add_f64s(*same_imag, *first_basis, first_sums[1]);
            first_sums[2] = simd.mul_add_f64s(*opposite_real, *first_basis, first_sums[2]);
            first_sums[3] = simd.mul_add_f64s(*opposite_imag, *first_basis, first_sums[3]);
            second_sums[0] = simd.mul_add_f64s(*same_real, *second_basis, second_sums[0]);
            second_sums[1] = simd.mul_add_f64s(*same_imag, *second_basis, second_sums[1]);
            second_sums[2] = simd.mul_add_f64s(*opposite_real, *second_basis, second_sums[2]);
            second_sums[3] = simd.mul_add_f64s(*opposite_imag, *second_basis, second_sums[3]);
        }

        let mut first = [
            simd.reduce_sum_f64s(first_sums[0]),
            simd.reduce_sum_f64s(first_sums[1]),
            simd.reduce_sum_f64s(first_sums[2]),
            simd.reduce_sum_f64s(first_sums[3]),
        ];
        let mut second = [
            simd.reduce_sum_f64s(second_sums[0]),
            simd.reduce_sum_f64s(second_sums[1]),
            simd.reduce_sum_f64s(second_sums[2]),
            simd.reduce_sum_f64s(second_sums[3]),
        ];
        debug_assert_eq!(same_real_tail.len(), same_imag_tail.len());
        debug_assert_eq!(same_real_tail.len(), opposite_real_tail.len());
        debug_assert_eq!(same_real_tail.len(), opposite_imag_tail.len());
        debug_assert_eq!(same_real_tail.len(), first_basis_tail.len());
        debug_assert_eq!(same_real_tail.len(), second_basis_tail.len());
        for lane in 0..same_real_tail.len() {
            first[0] = same_real_tail[lane].mul_add(first_basis_tail[lane], first[0]);
            first[1] = same_imag_tail[lane].mul_add(first_basis_tail[lane], first[1]);
            first[2] = opposite_real_tail[lane].mul_add(first_basis_tail[lane], first[2]);
            first[3] = opposite_imag_tail[lane].mul_add(first_basis_tail[lane], first[3]);
            second[0] = same_real_tail[lane].mul_add(second_basis_tail[lane], second[0]);
            second[1] = same_imag_tail[lane].mul_add(second_basis_tail[lane], second[1]);
            second[2] = opposite_real_tail[lane].mul_add(second_basis_tail[lane], second[2]);
            second[3] = opposite_imag_tail[lane].mul_add(second_basis_tail[lane], second[3]);
        }

        [
            (
                Complex64::new(first[0], first[1]),
                Complex64::new(first[2], first[3]),
            ),
            (
                Complex64::new(second[0], second[1]),
                Complex64::new(second[2], second[3]),
            ),
        ]
    }
}

struct AnalysisPairChannelsAccumulation<'a> {
    same_real: [&'a [f64]; 2],
    same_imag: [&'a [f64]; 2],
    opposite_real: [&'a [f64]; 2],
    opposite_imag: [&'a [f64]; 2],
    basis: &'a [f64],
    cos_theta: &'a [f64],
}

impl WithSimd for AnalysisPairChannelsAccumulation<'_> {
    type Output = [(Complex64, Complex64); 2];

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let (same_real_0, same_real_tail_0) = S::as_simd_f64s(self.same_real[0]);
        let (same_imag_0, same_imag_tail_0) = S::as_simd_f64s(self.same_imag[0]);
        let (opposite_real_0, opposite_real_tail_0) = S::as_simd_f64s(self.opposite_real[0]);
        let (opposite_imag_0, opposite_imag_tail_0) = S::as_simd_f64s(self.opposite_imag[0]);
        let (same_real_1, same_real_tail_1) = S::as_simd_f64s(self.same_real[1]);
        let (same_imag_1, same_imag_tail_1) = S::as_simd_f64s(self.same_imag[1]);
        let (opposite_real_1, opposite_real_tail_1) = S::as_simd_f64s(self.opposite_real[1]);
        let (opposite_imag_1, opposite_imag_tail_1) = S::as_simd_f64s(self.opposite_imag[1]);
        let (basis_vectors, basis_tail) = S::as_simd_f64s(self.basis);
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(self.cos_theta);
        for vectors in [
            same_real_0,
            same_imag_0,
            opposite_real_0,
            opposite_imag_0,
            same_real_1,
            same_imag_1,
            opposite_real_1,
            opposite_imag_1,
            cosine_vectors,
        ] {
            debug_assert_eq!(vectors.len(), basis_vectors.len());
        }

        let channel_0 = same_real_0
            .iter()
            .zip(same_imag_0)
            .zip(opposite_real_0)
            .zip(opposite_imag_0);
        let channel_1 = same_real_1
            .iter()
            .zip(same_imag_1)
            .zip(opposite_real_1)
            .zip(opposite_imag_1);
        let vectors = basis_vectors
            .iter()
            .zip(cosine_vectors)
            .zip(channel_0)
            .zip(channel_1);

        let zeros = simd.splat_f64s(0.0);
        let mut sums_0 = [zeros; 4];
        let mut sums_1 = [zeros; 4];
        for (
            ((basis, cosine), (((same_real, same_imag), opposite_real), opposite_imag)),
            (((same_real_1, same_imag_1), opposite_real_1), opposite_imag_1),
        ) in vectors
        {
            let basis = *basis;
            let cosine = *cosine;
            let opposite_real_scaled = simd.mul_f64s(*opposite_real, basis);
            let opposite_imag_scaled = simd.mul_f64s(*opposite_imag, basis);
            let opposite_real_1_scaled = simd.mul_f64s(*opposite_real_1, basis);
            let opposite_imag_1_scaled = simd.mul_f64s(*opposite_imag_1, basis);
            sums_0[0] = simd.mul_add_f64s(*same_real, basis, sums_0[0]);
            sums_0[1] = simd.mul_add_f64s(*same_imag, basis, sums_0[1]);
            sums_0[2] = simd.mul_add_f64s(opposite_real_scaled, cosine, sums_0[2]);
            sums_0[3] = simd.mul_add_f64s(opposite_imag_scaled, cosine, sums_0[3]);
            sums_1[0] = simd.mul_add_f64s(*same_real_1, basis, sums_1[0]);
            sums_1[1] = simd.mul_add_f64s(*same_imag_1, basis, sums_1[1]);
            sums_1[2] = simd.mul_add_f64s(opposite_real_1_scaled, cosine, sums_1[2]);
            sums_1[3] = simd.mul_add_f64s(opposite_imag_1_scaled, cosine, sums_1[3]);
        }

        let mut results = [
            (
                Complex64::new(
                    simd.reduce_sum_f64s(sums_0[0]),
                    simd.reduce_sum_f64s(sums_0[1]),
                ),
                Complex64::new(
                    simd.reduce_sum_f64s(sums_0[2]),
                    simd.reduce_sum_f64s(sums_0[3]),
                ),
            ),
            (
                Complex64::new(
                    simd.reduce_sum_f64s(sums_1[0]),
                    simd.reduce_sum_f64s(sums_1[1]),
                ),
                Complex64::new(
                    simd.reduce_sum_f64s(sums_1[2]),
                    simd.reduce_sum_f64s(sums_1[3]),
                ),
            ),
        ];

        debug_assert_eq!(basis_tail.len(), same_real_tail_0.len());
        debug_assert_eq!(basis_tail.len(), same_imag_tail_0.len());
        debug_assert_eq!(basis_tail.len(), opposite_real_tail_0.len());
        debug_assert_eq!(basis_tail.len(), opposite_imag_tail_0.len());
        debug_assert_eq!(basis_tail.len(), same_real_tail_1.len());
        debug_assert_eq!(basis_tail.len(), same_imag_tail_1.len());
        debug_assert_eq!(basis_tail.len(), opposite_real_tail_1.len());
        debug_assert_eq!(basis_tail.len(), opposite_imag_tail_1.len());
        debug_assert_eq!(basis_tail.len(), cosine_tail.len());
        for lane in 0..basis_tail.len() {
            for (channel, same_real, same_imag, opposite_real, opposite_imag) in [
                (
                    0,
                    same_real_tail_0,
                    same_imag_tail_0,
                    opposite_real_tail_0,
                    opposite_imag_tail_0,
                ),
                (
                    1,
                    same_real_tail_1,
                    same_imag_tail_1,
                    opposite_real_tail_1,
                    opposite_imag_tail_1,
                ),
            ] {
                let opposite_real_scaled = opposite_real[lane] * basis_tail[lane];
                let opposite_imag_scaled = opposite_imag[lane] * basis_tail[lane];
                results[channel].0.re =
                    same_real[lane].mul_add(basis_tail[lane], results[channel].0.re);
                results[channel].0.im =
                    same_imag[lane].mul_add(basis_tail[lane], results[channel].0.im);
                results[channel].1.re =
                    opposite_real_scaled.mul_add(cosine_tail[lane], results[channel].1.re);
                results[channel].1.im =
                    opposite_imag_scaled.mul_add(cosine_tail[lane], results[channel].1.im);
            }
        }
        results
    }
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool> WithSimd
    for AnalysisPairAccumulation<'_, OPPOSITE_COSINE_WEIGHTED>
{
    type Output = (Complex64, Complex64);

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let AnalysisPairAccumulation {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis,
            cos_theta,
        } = self;
        let (same_real_vectors, same_real_tail) = S::as_simd_f64s(same_real);
        let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(same_imag);
        let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(opposite_real);
        let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(opposite_imag);
        let (basis_vectors, basis_tail) = S::as_simd_f64s(basis);
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
        debug_assert_eq!(same_real_vectors.len(), basis_vectors.len());
        debug_assert_eq!(same_imag_vectors.len(), basis_vectors.len());
        debug_assert_eq!(opposite_real_vectors.len(), basis_vectors.len());
        debug_assert_eq!(opposite_imag_vectors.len(), basis_vectors.len());
        debug_assert_eq!(cosine_vectors.len(), basis_vectors.len());

        let zeros = simd.splat_f64s(0.0);
        let mut same_re_even = zeros;
        let mut same_im_even = zeros;
        let mut opposite_re_even = zeros;
        let mut opposite_im_even = zeros;
        let mut same_re_odd = zeros;
        let mut same_im_odd = zeros;
        let mut opposite_re_odd = zeros;
        let mut opposite_im_odd = zeros;
        let (same_real_pairs, same_real_remainder) = same_real_vectors.as_chunks::<2>();
        let (same_imag_pairs, same_imag_remainder) = same_imag_vectors.as_chunks::<2>();
        let (opposite_real_pairs, opposite_real_remainder) = opposite_real_vectors.as_chunks::<2>();
        let (opposite_imag_pairs, opposite_imag_remainder) = opposite_imag_vectors.as_chunks::<2>();
        let (basis_pairs, basis_remainder) = basis_vectors.as_chunks::<2>();
        let (cosine_pairs, cosine_remainder) = cosine_vectors.as_chunks::<2>();
        debug_assert_eq!(same_real_pairs.len(), basis_pairs.len());
        debug_assert_eq!(same_imag_pairs.len(), basis_pairs.len());
        debug_assert_eq!(opposite_real_pairs.len(), basis_pairs.len());
        debug_assert_eq!(opposite_imag_pairs.len(), basis_pairs.len());
        debug_assert_eq!(cosine_pairs.len(), basis_pairs.len());
        debug_assert_eq!(same_real_remainder.len(), basis_remainder.len());
        debug_assert_eq!(same_imag_remainder.len(), basis_remainder.len());
        debug_assert_eq!(opposite_real_remainder.len(), basis_remainder.len());
        debug_assert_eq!(opposite_imag_remainder.len(), basis_remainder.len());
        debug_assert_eq!(cosine_remainder.len(), basis_remainder.len());
        let vector_pairs = same_real_pairs
            .iter()
            .zip(same_imag_pairs)
            .zip(opposite_real_pairs)
            .zip(opposite_imag_pairs)
            .zip(basis_pairs)
            .zip(cosine_pairs);
        for (((((same_real, same_imag), opposite_real), opposite_imag), basis), cosine) in
            vector_pairs
        {
            same_re_even = simd.mul_add_f64s(same_real[0], basis[0], same_re_even);
            same_im_even = simd.mul_add_f64s(same_imag[0], basis[0], same_im_even);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_re_even = simd.mul_add_f64s(opposite_real[0], basis[0], opposite_re_even);
            } else {
                opposite_re_even = simd.add_f64s(
                    opposite_re_even,
                    simd.mul_f64s(simd.mul_f64s(opposite_real[0], basis[0]), cosine[0]),
                );
            }
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_im_even = simd.mul_add_f64s(opposite_imag[0], basis[0], opposite_im_even);
            } else {
                opposite_im_even = simd.add_f64s(
                    opposite_im_even,
                    simd.mul_f64s(simd.mul_f64s(opposite_imag[0], basis[0]), cosine[0]),
                );
            }
            same_re_odd = simd.mul_add_f64s(same_real[1], basis[1], same_re_odd);
            same_im_odd = simd.mul_add_f64s(same_imag[1], basis[1], same_im_odd);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_re_odd = simd.mul_add_f64s(opposite_real[1], basis[1], opposite_re_odd);
                opposite_im_odd = simd.mul_add_f64s(opposite_imag[1], basis[1], opposite_im_odd);
            } else {
                opposite_re_odd = simd.add_f64s(
                    opposite_re_odd,
                    simd.mul_f64s(simd.mul_f64s(opposite_real[1], basis[1]), cosine[1]),
                );
                opposite_im_odd = simd.add_f64s(
                    opposite_im_odd,
                    simd.mul_f64s(simd.mul_f64s(opposite_imag[1], basis[1]), cosine[1]),
                );
            }
        }
        if !basis_remainder.is_empty() {
            let basis = basis_remainder[0];
            let cosine = cosine_remainder[0];
            same_re_even = simd.mul_add_f64s(same_real_remainder[0], basis, same_re_even);
            same_im_even = simd.mul_add_f64s(same_imag_remainder[0], basis, same_im_even);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_re_even =
                    simd.mul_add_f64s(opposite_real_remainder[0], basis, opposite_re_even);
            } else {
                opposite_re_even = simd.add_f64s(
                    opposite_re_even,
                    simd.mul_f64s(simd.mul_f64s(opposite_real_remainder[0], basis), cosine),
                );
            }
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_im_even =
                    simd.mul_add_f64s(opposite_imag_remainder[0], basis, opposite_im_even);
            } else {
                opposite_im_even = simd.add_f64s(
                    opposite_im_even,
                    simd.mul_f64s(simd.mul_f64s(opposite_imag_remainder[0], basis), cosine),
                );
            }
        }
        let mut same_re = simd.reduce_sum_f64s(simd.add_f64s(same_re_even, same_re_odd));
        let mut same_im = simd.reduce_sum_f64s(simd.add_f64s(same_im_even, same_im_odd));
        let mut opposite_re =
            simd.reduce_sum_f64s(simd.add_f64s(opposite_re_even, opposite_re_odd));
        let mut opposite_im =
            simd.reduce_sum_f64s(simd.add_f64s(opposite_im_even, opposite_im_odd));
        for lane in 0..basis_tail.len() {
            same_re = same_real_tail[lane].mul_add(basis_tail[lane], same_re);
            same_im = same_imag_tail[lane].mul_add(basis_tail[lane], same_im);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_re = opposite_real_tail[lane].mul_add(basis_tail[lane], opposite_re);
                opposite_im = opposite_imag_tail[lane].mul_add(basis_tail[lane], opposite_im);
            } else {
                opposite_re += (opposite_real_tail[lane] * basis_tail[lane]) * cosine_tail[lane];
                opposite_im += (opposite_imag_tail[lane] * basis_tail[lane]) * cosine_tail[lane];
            }
        }
        (
            Complex64::new(same_re, same_im),
            Complex64::new(opposite_re, opposite_im),
        )
    }
}

struct MixedAnalysisPairAccumulation<'a, const OPPOSITE_COSINE_WEIGHTED: bool = false> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis: &'a [f64],
    exponents: &'a [i64],
    cos_theta: &'a [f64],
    allow_insignificant_scaled_lanes: bool,
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool> WithSimd
    for MixedAnalysisPairAccumulation<'_, OPPOSITE_COSINE_WEIGHTED>
{
    type Output = (Complex64, Complex64);

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let MixedAnalysisPairAccumulation {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis,
            exponents,
            cos_theta,
            allow_insignificant_scaled_lanes,
        } = self;
        let (same_real_vectors, same_real_tail) = S::as_simd_f64s(same_real);
        let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(same_imag);
        let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(opposite_real);
        let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(opposite_imag);
        let (basis_vectors, basis_tail) = S::as_simd_f64s(basis);
        let (exponent_vectors, exponent_tail) = S::as_simd_i64s(exponents);
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
        debug_assert_eq!(same_real_vectors.len(), basis_vectors.len());
        debug_assert_eq!(same_imag_vectors.len(), basis_vectors.len());
        debug_assert_eq!(opposite_real_vectors.len(), basis_vectors.len());
        debug_assert_eq!(opposite_imag_vectors.len(), basis_vectors.len());
        debug_assert_eq!(exponent_vectors.len(), basis_vectors.len());
        debug_assert_eq!(cosine_vectors.len(), basis_vectors.len());
        debug_assert_eq!(same_real_tail.len(), basis_tail.len());
        debug_assert_eq!(same_imag_tail.len(), basis_tail.len());
        debug_assert_eq!(opposite_real_tail.len(), basis_tail.len());
        debug_assert_eq!(opposite_imag_tail.len(), basis_tail.len());
        debug_assert_eq!(exponent_tail.len(), basis_tail.len());
        debug_assert_eq!(cosine_tail.len(), basis_tail.len());

        let zero_exponent = simd.splat_i64s(0);
        let zeros = simd.splat_f64s(0.0);
        let small = simd.splat_f64s(scalar::SMALL);
        let big = simd.splat_f64s(scalar::BIG);
        let mut same_re = simd.splat_f64s(0.0);
        let mut same_im = simd.splat_f64s(0.0);
        let mut opposite_re = simd.splat_f64s(0.0);
        let mut opposite_im = simd.splat_f64s(0.0);
        let mut vector_needs_scaled_fallback = false;
        for index in 0..basis_vectors.len() {
            let exponent_is_zero = simd.equal_i64s(exponent_vectors[index], zero_exponent);
            let exponent_is_positive =
                simd.greater_than_i64s(exponent_vectors[index], zero_exponent);
            let basis_magnitude = simd.abs_f64s(basis_vectors[index]);
            let basis_is_zero = simd.equal_f64s(basis_magnitude, zeros);
            let basis_is_large_enough = simd.greater_than_or_equal_f64s(basis_magnitude, small);
            let basis_is_small_enough = simd.less_than_or_equal_f64s(basis_magnitude, big);
            let basis_is_normal_or_zero = simd.and_m64s(
                basis_is_small_enough,
                simd.or_m64s(basis_is_zero, basis_is_large_enough),
            );
            if allow_insignificant_scaled_lanes {
                let zero_exponent_needs_fallback =
                    simd.and_m64s(exponent_is_zero, simd.not_m64s(basis_is_normal_or_zero));
                vector_needs_scaled_fallback |= analysis_mask_any(
                    simd,
                    simd.or_m64s(exponent_is_positive, zero_exponent_needs_fallback),
                );
            }
            let normal_lanes = simd.and_m64s(exponent_is_zero, basis_is_normal_or_zero);
            let basis = simd.select_f64s(normal_lanes, basis_vectors[index], zeros);
            let same_real = simd.select_f64s(normal_lanes, same_real_vectors[index], zeros);
            let same_imag = simd.select_f64s(normal_lanes, same_imag_vectors[index], zeros);
            let opposite_real = simd.select_f64s(normal_lanes, opposite_real_vectors[index], zeros);
            let opposite_imag = simd.select_f64s(normal_lanes, opposite_imag_vectors[index], zeros);
            same_re = simd.add_f64s(same_re, simd.mul_f64s(same_real, basis));
            same_im = simd.add_f64s(same_im, simd.mul_f64s(same_imag, basis));
            let opposite_re_product = if OPPOSITE_COSINE_WEIGHTED {
                simd.mul_f64s(opposite_real, basis)
            } else {
                simd.mul_f64s(simd.mul_f64s(opposite_real, basis), cosine_vectors[index])
            };
            let opposite_im_product = if OPPOSITE_COSINE_WEIGHTED {
                simd.mul_f64s(opposite_imag, basis)
            } else {
                simd.mul_f64s(simd.mul_f64s(opposite_imag, basis), cosine_vectors[index])
            };
            opposite_re = simd.add_f64s(opposite_re, opposite_re_product);
            opposite_im = simd.add_f64s(opposite_im, opposite_im_product);
        }

        let mut same = Complex64::new(simd.reduce_sum_f64s(same_re), simd.reduce_sum_f64s(same_im));
        let mut opposite = Complex64::new(
            simd.reduce_sum_f64s(opposite_re),
            simd.reduce_sum_f64s(opposite_im),
        );
        let vector_lane_count = basis.len() - basis_tail.len();
        if !allow_insignificant_scaled_lanes || vector_needs_scaled_fallback {
            for lane in 0..vector_lane_count {
                let needs_scaled_fallback = if allow_insignificant_scaled_lanes {
                    exponents[lane] > 0
                        || (exponents[lane] == 0 && !scalar_value_in_normal_range(basis[lane]))
                } else {
                    exponents[lane] != 0 || !scalar_value_in_normal_range(basis[lane])
                };
                if needs_scaled_fallback {
                    same += scalar::multiply_complex_scaled(
                        Complex64::new(same_real[lane], same_imag[lane]),
                        basis[lane],
                        exponents[lane],
                    );
                    let opposite_value = Complex64::new(opposite_real[lane], opposite_imag[lane]);
                    opposite += if OPPOSITE_COSINE_WEIGHTED {
                        scalar::multiply_complex_scaled(
                            opposite_value,
                            basis[lane],
                            exponents[lane],
                        )
                    } else {
                        scalar::multiply_complex_scaled_product(
                            opposite_value,
                            basis[lane],
                            cos_theta[lane],
                            exponents[lane],
                        )
                    };
                }
            }
        }
        for lane in 0..basis_tail.len() {
            if allow_insignificant_scaled_lanes && exponent_tail[lane] < 0 {
                // The normalized mantissa is at most 2^400 and negative scale
                // epochs are multiples of 800, so this basis contribution is
                // below 2^-400. The caller's latitude-wide budget covers its
                // omission after coefficient accumulation.
                continue;
            } else if exponent_tail[lane] == 0 && scalar_value_in_normal_range(basis_tail[lane]) {
                same.re += same_real_tail[lane] * basis_tail[lane];
                same.im += same_imag_tail[lane] * basis_tail[lane];
                if OPPOSITE_COSINE_WEIGHTED {
                    opposite.re += opposite_real_tail[lane] * basis_tail[lane];
                    opposite.im += opposite_imag_tail[lane] * basis_tail[lane];
                } else {
                    opposite.re +=
                        (opposite_real_tail[lane] * basis_tail[lane]) * cosine_tail[lane];
                    opposite.im +=
                        (opposite_imag_tail[lane] * basis_tail[lane]) * cosine_tail[lane];
                }
            } else {
                same += scalar::multiply_complex_scaled(
                    Complex64::new(same_real_tail[lane], same_imag_tail[lane]),
                    basis_tail[lane],
                    exponent_tail[lane],
                );
                let opposite_value =
                    Complex64::new(opposite_real_tail[lane], opposite_imag_tail[lane]);
                opposite += if OPPOSITE_COSINE_WEIGHTED {
                    scalar::multiply_complex_scaled(
                        opposite_value,
                        basis_tail[lane],
                        exponent_tail[lane],
                    )
                } else {
                    scalar::multiply_complex_scaled_product(
                        opposite_value,
                        basis_tail[lane],
                        cosine_tail[lane],
                        exponent_tail[lane],
                    )
                };
            }
        }
        (same, opposite)
    }
}

/// Accumulates mixed scaled lanes for Q and U while computing the ordinary
/// lane mask once. Each channel uses four live SIMD sums, keeping AVX2
/// register pressure bounded.
struct MixedAnalysisPairChannelsAccumulation<'a> {
    same_real: [&'a [f64]; 2],
    same_imag: [&'a [f64]; 2],
    opposite_real: [&'a [f64]; 2],
    opposite_imag: [&'a [f64]; 2],
    basis: &'a [f64],
    exponents: &'a [i64],
    cos_theta: &'a [f64],
}

impl WithSimd for MixedAnalysisPairChannelsAccumulation<'_> {
    type Output = [(Complex64, Complex64); 2];

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let MixedAnalysisPairChannelsAccumulation {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis,
            exponents,
            cos_theta,
        } = self;
        debug_assert_eq!(basis.len(), exponents.len());
        debug_assert_eq!(basis.len(), cos_theta.len());
        for channel in 0..2 {
            debug_assert_eq!(basis.len(), same_real[channel].len());
            debug_assert_eq!(basis.len(), same_imag[channel].len());
            debug_assert_eq!(basis.len(), opposite_real[channel].len());
            debug_assert_eq!(basis.len(), opposite_imag[channel].len());
        }
        let mut ordinary_basis = [0.0; LATITUDE_BLOCK_SIZE];
        let (basis_vectors, basis_tail) = S::as_simd_f64s(basis);
        let (exponent_vectors, exponent_tail) = S::as_simd_i64s(exponents);
        let (ordinary_basis_vectors, ordinary_basis_tail) =
            S::as_mut_simd_f64s(&mut ordinary_basis[..basis.len()]);
        debug_assert!(basis.len() <= LATITUDE_BLOCK_SIZE);
        debug_assert_eq!(basis_vectors.len(), exponent_vectors.len());
        debug_assert_eq!(basis_vectors.len(), ordinary_basis_vectors.len());
        debug_assert_eq!(basis_tail.len(), exponent_tail.len());
        debug_assert_eq!(basis_tail.len(), ordinary_basis_tail.len());

        let zero_f64 = simd.splat_f64s(0.0);
        let minimum = simd.splat_f64s(scalar::SMALL);
        let maximum = simd.splat_f64s(scalar::BIG);
        let zero_i64 = simd.splat_i64s(0);
        for index in 0..basis_vectors.len() {
            let magnitude = simd.abs_f64s(basis_vectors[index]);
            let in_range = simd.and_m64s(
                simd.less_than_or_equal_f64s(magnitude, maximum),
                simd.or_m64s(
                    simd.equal_f64s(magnitude, zero_f64),
                    simd.greater_than_or_equal_f64s(magnitude, minimum),
                ),
            );
            let ordinary =
                simd.and_m64s(simd.equal_i64s(exponent_vectors[index], zero_i64), in_range);
            ordinary_basis_vectors[index] =
                simd.select_f64s(ordinary, basis_vectors[index], zero_f64);
        }
        for lane in 0..basis_tail.len() {
            if exponent_tail[lane] == 0 && scalar_value_in_normal_range(basis_tail[lane]) {
                ordinary_basis_tail[lane] = basis_tail[lane];
            }
        }

        let (same_real_0, _) = S::as_simd_f64s(same_real[0]);
        let (same_imag_0, _) = S::as_simd_f64s(same_imag[0]);
        let (opposite_real_0, _) = S::as_simd_f64s(opposite_real[0]);
        let (opposite_imag_0, _) = S::as_simd_f64s(opposite_imag[0]);
        let (same_real_1, _) = S::as_simd_f64s(same_real[1]);
        let (same_imag_1, _) = S::as_simd_f64s(same_imag[1]);
        let (opposite_real_1, _) = S::as_simd_f64s(opposite_real[1]);
        let (opposite_imag_1, _) = S::as_simd_f64s(opposite_imag[1]);
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
        for vectors in [
            same_real_0,
            same_imag_0,
            opposite_real_0,
            opposite_imag_0,
            same_real_1,
            same_imag_1,
            opposite_real_1,
            opposite_imag_1,
            cosine_vectors,
        ] {
            debug_assert_eq!(vectors.len(), ordinary_basis_vectors.len());
        }

        let zeros = simd.splat_f64s(0.0);
        let mut sums_0 = [zeros; 4];
        for index in 0..ordinary_basis_vectors.len() {
            let basis = ordinary_basis_vectors[index];
            let cosine = cosine_vectors[index];
            sums_0[0] = simd.add_f64s(sums_0[0], simd.mul_f64s(same_real_0[index], basis));
            sums_0[1] = simd.add_f64s(sums_0[1], simd.mul_f64s(same_imag_0[index], basis));
            sums_0[2] = simd.add_f64s(
                sums_0[2],
                simd.mul_f64s(simd.mul_f64s(opposite_real_0[index], basis), cosine),
            );
            sums_0[3] = simd.add_f64s(
                sums_0[3],
                simd.mul_f64s(simd.mul_f64s(opposite_imag_0[index], basis), cosine),
            );
        }
        let mut results = [
            (
                Complex64::new(
                    simd.reduce_sum_f64s(sums_0[0]),
                    simd.reduce_sum_f64s(sums_0[1]),
                ),
                Complex64::new(
                    simd.reduce_sum_f64s(sums_0[2]),
                    simd.reduce_sum_f64s(sums_0[3]),
                ),
            ),
            (Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)),
        ];

        let mut sums_1 = [zeros; 4];
        for index in 0..ordinary_basis_vectors.len() {
            let basis = ordinary_basis_vectors[index];
            let cosine = cosine_vectors[index];
            sums_1[0] = simd.add_f64s(sums_1[0], simd.mul_f64s(same_real_1[index], basis));
            sums_1[1] = simd.add_f64s(sums_1[1], simd.mul_f64s(same_imag_1[index], basis));
            sums_1[2] = simd.add_f64s(
                sums_1[2],
                simd.mul_f64s(simd.mul_f64s(opposite_real_1[index], basis), cosine),
            );
            sums_1[3] = simd.add_f64s(
                sums_1[3],
                simd.mul_f64s(simd.mul_f64s(opposite_imag_1[index], basis), cosine),
            );
        }
        results[1] = (
            Complex64::new(
                simd.reduce_sum_f64s(sums_1[0]),
                simd.reduce_sum_f64s(sums_1[1]),
            ),
            Complex64::new(
                simd.reduce_sum_f64s(sums_1[2]),
                simd.reduce_sum_f64s(sums_1[3]),
            ),
        );

        let vector_lane_count = basis.len() - basis_tail.len();
        for lane in 0..vector_lane_count {
            if exponents[lane] == 0 && scalar_value_in_normal_range(basis[lane]) {
                continue;
            }
            for channel in 0..2 {
                results[channel].0 += scalar::multiply_complex_scaled(
                    Complex64::new(same_real[channel][lane], same_imag[channel][lane]),
                    basis[lane],
                    exponents[lane],
                );
                let opposite =
                    Complex64::new(opposite_real[channel][lane], opposite_imag[channel][lane]);
                results[channel].1 += scalar::multiply_complex_scaled_product(
                    opposite,
                    basis[lane],
                    cos_theta[lane],
                    exponents[lane],
                );
            }
        }
        for lane in 0..basis_tail.len() {
            if exponent_tail[lane] == 0 && scalar_value_in_normal_range(basis_tail[lane]) {
                for channel in 0..2 {
                    results[channel].0.re +=
                        same_real[channel][vector_lane_count + lane] * basis_tail[lane];
                    results[channel].0.im +=
                        same_imag[channel][vector_lane_count + lane] * basis_tail[lane];
                    results[channel].1.re += (opposite_real[channel][vector_lane_count + lane]
                        * basis_tail[lane])
                        * cosine_tail[lane];
                    results[channel].1.im += (opposite_imag[channel][vector_lane_count + lane]
                        * basis_tail[lane])
                        * cosine_tail[lane];
                }
            } else {
                for channel in 0..2 {
                    results[channel].0 += scalar::multiply_complex_scaled(
                        Complex64::new(
                            same_real[channel][vector_lane_count + lane],
                            same_imag[channel][vector_lane_count + lane],
                        ),
                        basis_tail[lane],
                        exponent_tail[lane],
                    );
                    results[channel].1 += scalar::multiply_complex_scaled_product(
                        Complex64::new(
                            opposite_real[channel][vector_lane_count + lane],
                            opposite_imag[channel][vector_lane_count + lane],
                        ),
                        basis_tail[lane],
                        cosine_tail[lane],
                        exponent_tail[lane],
                    );
                }
            }
        }
        results
    }
}

struct FusedMixedAnalysisPairStep<'a, const OPPOSITE_COSINE_WEIGHTED: bool> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis_previous: &'a [f64],
    basis_current: &'a [f64],
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    scale_corrections: &'a [f64],
    coefficient_a: f64,
    coefficient_b: f64,
    basis_next: &'a mut [f64],
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool> WithSimd
    for FusedMixedAnalysisPairStep<'_, OPPOSITE_COSINE_WEIGHTED>
{
    type Output = (Complex64, Complex64);

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedMixedAnalysisPairStep {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis_previous,
            basis_current,
            cos_theta,
            cos_theta_squared,
            scale_corrections,
            coefficient_a,
            coefficient_b,
            basis_next,
        } = self;
        let (same_real_vectors, same_real_tail) = S::as_simd_f64s(same_real);
        let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(same_imag);
        let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(opposite_real);
        let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(opposite_imag);
        let (previous_vectors, previous_tail) = S::as_simd_f64s(basis_previous);
        let (current_vectors, current_tail) = S::as_simd_f64s(basis_current);
        let (correction_vectors, correction_tail) = S::as_simd_f64s(scale_corrections);
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
        let (cosine_squared_vectors, cosine_squared_tail) = S::as_simd_f64s(cos_theta_squared);
        let (next_vectors, next_tail) = S::as_mut_simd_f64s(basis_next);
        debug_assert_eq!(same_real_vectors.len(), current_vectors.len());
        debug_assert_eq!(same_imag_vectors.len(), current_vectors.len());
        debug_assert_eq!(opposite_real_vectors.len(), current_vectors.len());
        debug_assert_eq!(opposite_imag_vectors.len(), current_vectors.len());
        debug_assert_eq!(previous_vectors.len(), current_vectors.len());
        debug_assert_eq!(correction_vectors.len(), current_vectors.len());
        debug_assert_eq!(cosine_vectors.len(), current_vectors.len());
        debug_assert_eq!(cosine_squared_vectors.len(), current_vectors.len());
        debug_assert_eq!(next_vectors.len(), current_vectors.len());
        debug_assert_eq!(same_real_tail.len(), current_tail.len());
        debug_assert_eq!(same_imag_tail.len(), current_tail.len());
        debug_assert_eq!(opposite_real_tail.len(), current_tail.len());
        debug_assert_eq!(opposite_imag_tail.len(), current_tail.len());
        debug_assert_eq!(previous_tail.len(), current_tail.len());
        debug_assert_eq!(correction_tail.len(), current_tail.len());
        debug_assert_eq!(cosine_tail.len(), current_tail.len());
        debug_assert_eq!(cosine_squared_tail.len(), current_tail.len());
        debug_assert_eq!(next_tail.len(), current_tail.len());

        let zero = simd.splat_f64s(0.0);
        let a = simd.splat_f64s(coefficient_a);
        let b = simd.splat_f64s(coefficient_b);
        let mut same_re = zero;
        let mut same_im = zero;
        let mut opposite_re = zero;
        let mut opposite_im = zero;

        // Apply the per-lane scale correction while accumulating this degree
        // pair, then advance the recurrence state for the next pair.
        for index in 0..current_vectors.len() {
            let basis = simd.mul_f64s(current_vectors[index], correction_vectors[index]);
            same_re = simd.mul_add_f64s(same_real_vectors[index], basis, same_re);
            same_im = simd.mul_add_f64s(same_imag_vectors[index], basis, same_im);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_re = simd.mul_add_f64s(opposite_real_vectors[index], basis, opposite_re);
                opposite_im = simd.mul_add_f64s(opposite_imag_vectors[index], basis, opposite_im);
            } else {
                opposite_re = simd.mul_add_f64s(
                    simd.mul_f64s(opposite_real_vectors[index], cosine_vectors[index]),
                    basis,
                    opposite_re,
                );
                opposite_im = simd.mul_add_f64s(
                    simd.mul_f64s(opposite_imag_vectors[index], cosine_vectors[index]),
                    basis,
                    opposite_im,
                );
            }
            let leading = simd.mul_add_f64s(a, cosine_squared_vectors[index], b);
            next_vectors[index] =
                simd.mul_add_f64s(leading, current_vectors[index], previous_vectors[index]);
        }

        let mut same = Complex64::new(simd.reduce_sum_f64s(same_re), simd.reduce_sum_f64s(same_im));
        let mut opposite = Complex64::new(
            simd.reduce_sum_f64s(opposite_re),
            simd.reduce_sum_f64s(opposite_im),
        );
        for lane in 0..current_tail.len() {
            let basis = current_tail[lane] * correction_tail[lane];
            same.re = same_real_tail[lane].mul_add(basis, same.re);
            same.im = same_imag_tail[lane].mul_add(basis, same.im);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite.re = opposite_real_tail[lane].mul_add(basis, opposite.re);
                opposite.im = opposite_imag_tail[lane].mul_add(basis, opposite.im);
            } else {
                opposite.re =
                    (opposite_real_tail[lane] * cosine_tail[lane]).mul_add(basis, opposite.re);
                opposite.im =
                    (opposite_imag_tail[lane] * cosine_tail[lane]).mul_add(basis, opposite.im);
            }
            let leading = coefficient_a.mul_add(cosine_squared_tail[lane], coefficient_b);
            next_tail[lane] = leading.mul_add(current_tail[lane], previous_tail[lane]);
        }
        (same, opposite)
    }
}

struct FusedMixedAnalysisPairAccumulation<'a, const OPPOSITE_COSINE_WEIGHTED: bool> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis_current: &'a [f64],
    cos_theta: &'a [f64],
    scale_corrections: &'a [f64],
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool> WithSimd
    for FusedMixedAnalysisPairAccumulation<'_, OPPOSITE_COSINE_WEIGHTED>
{
    type Output = (Complex64, Complex64);

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedMixedAnalysisPairAccumulation {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis_current,
            cos_theta,
            scale_corrections,
        } = self;
        let (same_real_vectors, same_real_tail) = S::as_simd_f64s(same_real);
        let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(same_imag);
        let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(opposite_real);
        let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(opposite_imag);
        let (basis_vectors, basis_tail) = S::as_simd_f64s(basis_current);
        let (correction_vectors, correction_tail) = S::as_simd_f64s(scale_corrections);
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
        let zero = simd.splat_f64s(0.0);
        let mut same_re = zero;
        let mut same_im = zero;
        let mut opposite_re = zero;
        let mut opposite_im = zero;
        for index in 0..basis_vectors.len() {
            let correction = correction_vectors[index];
            let basis = simd.mul_f64s(basis_vectors[index], correction);
            same_re = simd.mul_add_f64s(same_real_vectors[index], basis, same_re);
            same_im = simd.mul_add_f64s(same_imag_vectors[index], basis, same_im);
            let opposite_re_coefficient = opposite_real_vectors[index];
            let opposite_im_coefficient = opposite_imag_vectors[index];
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_re = simd.mul_add_f64s(opposite_re_coefficient, basis, opposite_re);
                opposite_im = simd.mul_add_f64s(opposite_im_coefficient, basis, opposite_im);
            } else {
                opposite_re = simd.mul_add_f64s(
                    simd.mul_f64s(opposite_re_coefficient, cosine_vectors[index]),
                    basis,
                    opposite_re,
                );
                opposite_im = simd.mul_add_f64s(
                    simd.mul_f64s(opposite_im_coefficient, cosine_vectors[index]),
                    basis,
                    opposite_im,
                );
            }
        }
        let mut same = Complex64::new(simd.reduce_sum_f64s(same_re), simd.reduce_sum_f64s(same_im));
        let mut opposite = Complex64::new(
            simd.reduce_sum_f64s(opposite_re),
            simd.reduce_sum_f64s(opposite_im),
        );
        for lane in 0..basis_tail.len() {
            let correction = correction_tail[lane];
            let basis = basis_tail[lane] * correction;
            same.re = same_real_tail[lane].mul_add(basis, same.re);
            same.im = same_imag_tail[lane].mul_add(basis, same.im);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite.re = opposite_real_tail[lane].mul_add(basis, opposite.re);
                opposite.im = opposite_imag_tail[lane].mul_add(basis, opposite.im);
            } else {
                opposite.re =
                    (opposite_real_tail[lane] * cosine_tail[lane]).mul_add(basis, opposite.re);
                opposite.im =
                    (opposite_imag_tail[lane] * cosine_tail[lane]).mul_add(basis, opposite.im);
            }
        }
        (same, opposite)
    }
}

struct FusedSynthesisBlock<'a> {
    m: usize,
    lmax: usize,
    cos_theta: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    coefficient_even: &'a [Complex64],
    coefficient_odd: &'a [Complex64],
    current: &'a [f64],
    sum_re: &'a mut [f64],
    sum_im: &'a mut [f64],
}

impl WithSimd for FusedSynthesisBlock<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) {
        let FusedSynthesisBlock {
            m,
            lmax,
            cos_theta,
            coefficient_a,
            coefficient_b,
            coefficient_even,
            coefficient_odd,
            current,
            sum_re,
            sum_im,
        } = self;
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
        let (initial_vectors, initial_tail) = S::as_simd_f64s(current);
        let (sum_re_vectors, sum_re_tail) = S::as_mut_simd_f64s(sum_re);
        let (sum_im_vectors, sum_im_tail) = S::as_mut_simd_f64s(sum_im);
        let pair_count = (lmax - m + 2) / 2;

        for lane_block in 0..cosine_vectors.len() {
            let cosine = cosine_vectors[lane_block];
            let mut previous = simd.splat_f64s(0.0);
            let mut current = initial_vectors[lane_block];
            let mut accumulated_re = simd.splat_f64s(0.0);
            let mut accumulated_im = simd.splat_f64s(0.0);
            for pair in 0..pair_count {
                let coefficient_even = coefficient_even[pair];
                let coefficient_odd = coefficient_odd[pair];
                let coefficient_re = simd.mul_add_f64s(
                    simd.splat_f64s(coefficient_odd.re),
                    cosine,
                    simd.splat_f64s(coefficient_even.re),
                );
                let coefficient_im = simd.mul_add_f64s(
                    simd.splat_f64s(coefficient_odd.im),
                    cosine,
                    simd.splat_f64s(coefficient_even.im),
                );
                accumulated_re = simd.mul_add_f64s(current, coefficient_re, accumulated_re);
                accumulated_im = simd.mul_add_f64s(current, coefficient_im, accumulated_im);

                if pair + 1 < pair_count {
                    let cosine_squared = simd.mul_f64s(cosine, cosine);
                    let leading = simd.mul_add_f64s(
                        simd.splat_f64s(coefficient_a[pair]),
                        cosine_squared,
                        simd.splat_f64s(coefficient_b[pair]),
                    );
                    let next = simd.mul_add_f64s(leading, current, previous);
                    previous = current;
                    current = next;
                }
            }
            sum_re_vectors[lane_block] = accumulated_re;
            sum_im_vectors[lane_block] = accumulated_im;
        }

        for lane in 0..cosine_tail.len() {
            let cosine = cosine_tail[lane];
            let mut previous = 0.0;
            let mut current = initial_tail[lane];
            let mut accumulated_re = 0.0;
            let mut accumulated_im = 0.0;
            for pair in 0..pair_count {
                let coefficient_even = coefficient_even[pair];
                let coefficient_odd = coefficient_odd[pair];
                accumulated_re = current.mul_add(
                    coefficient_odd.re.mul_add(cosine, coefficient_even.re),
                    accumulated_re,
                );
                accumulated_im = current.mul_add(
                    coefficient_odd.im.mul_add(cosine, coefficient_even.im),
                    accumulated_im,
                );

                if pair + 1 < pair_count {
                    let leading = coefficient_a[pair].mul_add(cosine * cosine, coefficient_b[pair]);
                    let next = leading.mul_add(current, previous);
                    previous = current;
                    current = next;
                }
            }
            sum_re_tail[lane] = accumulated_re;
            sum_im_tail[lane] = accumulated_im;
        }
    }
}

fn finite_prefix_sums(
    first_real: &[f64],
    first_imag: &[f64],
    second_real: &[f64],
    second_imag: &[f64],
) -> bool {
    let limit = f64::MAX * 0.5;
    first_real
        .iter()
        .chain(first_imag)
        .chain(second_real)
        .chain(second_imag)
        .all(|value| value.is_finite() && value.abs() <= limit)
}

struct FusedPreparedSynthesisPairBlock<'a> {
    pair_count: usize,
    cos_theta: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    first_even: &'a [Complex64],
    first_odd: &'a [Complex64],
    second_even: &'a [Complex64],
    second_odd: &'a [Complex64],
    current: &'a [f64],
    first_sum_re: &'a mut [f64],
    first_sum_im: &'a mut [f64],
    second_sum_re: &'a mut [f64],
    second_sum_im: &'a mut [f64],
}

impl WithSimd for FusedPreparedSynthesisPairBlock<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) {
        let FusedPreparedSynthesisPairBlock {
            pair_count,
            cos_theta,
            coefficient_a,
            coefficient_b,
            first_even,
            first_odd,
            second_even,
            second_odd,
            current,
            first_sum_re,
            first_sum_im,
            second_sum_re,
            second_sum_im,
        } = self;
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
        let (initial_vectors, initial_tail) = S::as_simd_f64s(current);
        let (first_sum_re_vectors, first_sum_re_tail) = S::as_mut_simd_f64s(first_sum_re);
        let (first_sum_im_vectors, first_sum_im_tail) = S::as_mut_simd_f64s(first_sum_im);
        let (second_sum_re_vectors, second_sum_re_tail) = S::as_mut_simd_f64s(second_sum_re);
        let (second_sum_im_vectors, second_sum_im_tail) = S::as_mut_simd_f64s(second_sum_im);
        let coefficient_pairs = coefficient_a
            .iter()
            .zip(coefficient_b)
            .zip(first_even)
            .zip(first_odd)
            .zip(second_even)
            .zip(second_odd)
            .enumerate();
        for lane_block in 0..cosine_vectors.len() {
            let cosine = cosine_vectors[lane_block];
            let mut previous = simd.splat_f64s(0.0);
            let mut current = initial_vectors[lane_block];
            let mut first_re = simd.splat_f64s(0.0);
            let mut first_im = simd.splat_f64s(0.0);
            let mut second_re = simd.splat_f64s(0.0);
            let mut second_im = simd.splat_f64s(0.0);
            for (
                pair,
                (
                    ((((coefficient_a, coefficient_b), first_even), first_odd), second_even),
                    second_odd,
                ),
            ) in coefficient_pairs.clone()
            {
                let first_coefficient_re = simd.mul_add_f64s(
                    simd.splat_f64s(first_odd.re),
                    cosine,
                    simd.splat_f64s(first_even.re),
                );
                let first_coefficient_im = simd.mul_add_f64s(
                    simd.splat_f64s(first_odd.im),
                    cosine,
                    simd.splat_f64s(first_even.im),
                );
                let second_coefficient_re = simd.mul_add_f64s(
                    simd.splat_f64s(second_odd.re),
                    cosine,
                    simd.splat_f64s(second_even.re),
                );
                let second_coefficient_im = simd.mul_add_f64s(
                    simd.splat_f64s(second_odd.im),
                    cosine,
                    simd.splat_f64s(second_even.im),
                );
                first_re = simd.mul_add_f64s(current, first_coefficient_re, first_re);
                first_im = simd.mul_add_f64s(current, first_coefficient_im, first_im);
                second_re = simd.mul_add_f64s(current, second_coefficient_re, second_re);
                second_im = simd.mul_add_f64s(current, second_coefficient_im, second_im);

                if pair + 1 < pair_count {
                    let cosine_squared = simd.mul_f64s(cosine, cosine);
                    let leading = simd.mul_add_f64s(
                        simd.splat_f64s(*coefficient_a),
                        cosine_squared,
                        simd.splat_f64s(*coefficient_b),
                    );
                    let next = simd.mul_add_f64s(leading, current, previous);
                    previous = current;
                    current = next;
                }
            }
            first_sum_re_vectors[lane_block] = first_re;
            first_sum_im_vectors[lane_block] = first_im;
            second_sum_re_vectors[lane_block] = second_re;
            second_sum_im_vectors[lane_block] = second_im;
        }

        for lane in 0..cosine_tail.len() {
            let cosine = cosine_tail[lane];
            let mut previous = 0.0;
            let mut current = initial_tail[lane];
            let mut first_re = 0.0;
            let mut first_im = 0.0;
            let mut second_re = 0.0;
            let mut second_im = 0.0;
            for (
                pair,
                (
                    ((((coefficient_a, coefficient_b), first_even), first_odd), second_even),
                    second_odd,
                ),
            ) in coefficient_pairs.clone()
            {
                first_re = current.mul_add(first_odd.re.mul_add(cosine, first_even.re), first_re);
                first_im = current.mul_add(first_odd.im.mul_add(cosine, first_even.im), first_im);
                second_re =
                    current.mul_add(second_odd.re.mul_add(cosine, second_even.re), second_re);
                second_im =
                    current.mul_add(second_odd.im.mul_add(cosine, second_even.im), second_im);

                if pair + 1 < pair_count {
                    let leading = coefficient_a.mul_add(cosine * cosine, *coefficient_b);
                    let next = leading.mul_add(current, previous);
                    previous = current;
                    current = next;
                }
            }
            first_sum_re_tail[lane] = first_re;
            first_sum_im_tail[lane] = first_im;
            second_sum_re_tail[lane] = second_re;
            second_sum_im_tail[lane] = second_im;
        }
    }
}

#[cfg(feature = "parallel")]
#[expect(
    clippy::too_many_arguments,
    reason = "The symmetry experiment writes both hemispheres for two scalar channels."
)]
pub(crate) fn synthesize_hemisphere_pair_block_dispatched(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    first_even: &[Complex64],
    first_odd: &[Complex64],
    second_even: &[Complex64],
    second_odd: &[Complex64],
    spectrum_len: usize,
    north_first: &mut [Complex64],
    south_first: &mut [Complex64],
    north_second: &mut [Complex64],
    south_second: &mut [Complex64],
    current: &mut [f64],
    basis_tile: &mut [f64],
    sums: &mut [&mut [f64]; 8],
) -> Result<bool, ShtError> {
    let lane_count = sin_theta.len();
    debug_assert_eq!(lane_count, cos_theta.len());
    debug_assert!(lane_count <= LATITUDE_BLOCK_SIZE);
    debug_assert_eq!(north_first.len(), lane_count * spectrum_len);
    debug_assert_eq!(south_first.len(), north_first.len());
    debug_assert_eq!(north_second.len(), north_first.len());
    debug_assert_eq!(south_second.len(), north_first.len());
    debug_assert!(current.len() >= lane_count);
    debug_assert!(basis_tile.len() >= HEMISPHERE_PAIR_DEGREE_TILE * lane_count);
    debug_assert!(sums.iter().all(|values| values.len() >= lane_count));
    if sin_theta
        .iter()
        .any(|&sine| !sine.is_finite() || sine < 0.01)
        || cos_theta
            .iter()
            .any(|&cosine| !cosine.is_finite() || cosine.abs() > 1.0)
    {
        return Ok(false);
    }
    let pair_count = (lmax - m + 2) / 2;
    if coefficient_a[..pair_count]
        .iter()
        .chain(&coefficient_b[..pair_count])
        .any(|&value| !scalar_value_in_normal_range(value))
    {
        return Ok(false);
    }
    for coefficients in [first_even, first_odd, second_even, second_odd] {
        if coefficients[..pair_count]
            .iter()
            .flat_map(|coefficient| [coefficient.re, coefficient.im])
            .any(|value| !scalar_value_in_normal_range(value))
        {
            return Ok(false);
        }
    }
    for lane in 0..lane_count {
        let (value, exponent) = scalar::sectoral_value(sin_theta[lane], m, sectoral_norm)?;
        if exponent != 0 || !scalar_value_in_normal_range(value) {
            return Ok(false);
        }
        current[lane] = value;
    }

    let completed = arch.dispatch(FusedHemispherePairBlock {
        pair_count,
        cos_theta,
        coefficient_a: &coefficient_a[..pair_count],
        coefficient_b: &coefficient_b[..pair_count],
        first_even: &first_even[..pair_count],
        first_odd: &first_odd[..pair_count],
        second_even: &second_even[..pair_count],
        second_odd: &second_odd[..pair_count],
        current: &current[..lane_count],
        basis_tile: &mut basis_tile[..HEMISPHERE_PAIR_DEGREE_TILE * lane_count],
        sums: sums.each_mut().map(|values| &mut values[..lane_count]),
    });
    if !completed {
        return Ok(false);
    }

    for lane in 0..lane_count {
        let inverse_sine = sin_theta[lane].recip();
        let first_north =
            Complex64::new(sums[0][lane] + sums[2][lane], sums[1][lane] + sums[3][lane])
                * inverse_sine;
        let first_south =
            Complex64::new(sums[0][lane] - sums[2][lane], sums[1][lane] - sums[3][lane])
                * inverse_sine;
        let second_north =
            Complex64::new(sums[4][lane] + sums[6][lane], sums[5][lane] + sums[7][lane])
                * inverse_sine;
        let second_south =
            Complex64::new(sums[4][lane] - sums[6][lane], sums[5][lane] - sums[7][lane])
                * inverse_sine;
        if [
            first_north.re,
            first_north.im,
            first_south.re,
            first_south.im,
            second_north.re,
            second_north.im,
            second_south.re,
            second_south.im,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Ok(false);
        }
        sums[0][lane] = first_north.re;
        sums[1][lane] = first_north.im;
        sums[2][lane] = first_south.re;
        sums[3][lane] = first_south.im;
        sums[4][lane] = second_north.re;
        sums[5][lane] = second_north.im;
        sums[6][lane] = second_south.re;
        sums[7][lane] = second_south.im;
    }
    for lane in 0..lane_count {
        north_first[lane * spectrum_len + m] += Complex64::new(sums[0][lane], sums[1][lane]);
        south_first[(lane_count - 1 - lane) * spectrum_len + m] +=
            Complex64::new(sums[2][lane], sums[3][lane]);
        north_second[lane * spectrum_len + m] += Complex64::new(sums[4][lane], sums[5][lane]);
        south_second[(lane_count - 1 - lane) * spectrum_len + m] +=
            Complex64::new(sums[6][lane], sums[7][lane]);
    }
    Ok(true)
}

#[cfg(feature = "parallel")]
struct FusedHemispherePairBlock<'a> {
    pair_count: usize,
    cos_theta: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    first_even: &'a [Complex64],
    first_odd: &'a [Complex64],
    second_even: &'a [Complex64],
    second_odd: &'a [Complex64],
    current: &'a [f64],
    basis_tile: &'a mut [f64],
    sums: [&'a mut [f64]; 8],
}

#[cfg(feature = "parallel")]
impl WithSimd for FusedHemispherePairBlock<'_> {
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> bool {
        let FusedHemispherePairBlock {
            pair_count,
            cos_theta,
            coefficient_a,
            coefficient_b,
            first_even,
            first_odd,
            second_even,
            second_odd,
            current,
            basis_tile,
            sums,
        } = self;
        let (cos_vectors, cos_tail) = S::as_simd_f64s(cos_theta);
        let (current_vectors, current_tail) = S::as_simd_f64s(current);
        let [sum0, sum1, sum2, sum3, sum4, sum5, sum6, sum7] = sums;
        let (vector0, tail0) = S::as_mut_simd_f64s(sum0);
        let (vector1, tail1) = S::as_mut_simd_f64s(sum1);
        let (vector2, tail2) = S::as_mut_simd_f64s(sum2);
        let (vector3, tail3) = S::as_mut_simd_f64s(sum3);
        let (vector4, tail4) = S::as_mut_simd_f64s(sum4);
        let (vector5, tail5) = S::as_mut_simd_f64s(sum5);
        let (vector6, tail6) = S::as_mut_simd_f64s(sum6);
        let (vector7, tail7) = S::as_mut_simd_f64s(sum7);
        let vector_sums = [
            vector0, vector1, vector2, vector3, vector4, vector5, vector6, vector7,
        ];
        let tail_sums = [tail0, tail1, tail2, tail3, tail4, tail5, tail6, tail7];
        let lane_count = cos_theta.len();
        for lane_block in 0..cos_vectors.len() {
            let cosine = cos_vectors[lane_block];
            let cosine_squared = simd.mul_f64s(cosine, cosine);
            let mut previous = simd.splat_f64s(0.0);
            let mut basis = current_vectors[lane_block];
            let zero = simd.splat_f64s(0.0);
            vector_sums[0][lane_block] = zero;
            vector_sums[1][lane_block] = zero;
            vector_sums[2][lane_block] = zero;
            vector_sums[3][lane_block] = zero;
            vector_sums[4][lane_block] = zero;
            vector_sums[5][lane_block] = zero;
            vector_sums[6][lane_block] = zero;
            vector_sums[7][lane_block] = zero;
            let mut pair_start = 0;
            while pair_start < pair_count {
                let tile_count = (pair_count - pair_start).min(HEMISPHERE_PAIR_DEGREE_TILE);

                // Advance the shared recurrence once and stage a short basis
                // tile. Channel accumulation then keeps four sums live at a
                // time instead of spilling all eight AVX2 parity sums.
                for tile_pair in 0..tile_count {
                    let basis_row = tile_pair * lane_count;
                    let pair = pair_start + tile_pair;
                    {
                        let row = &mut basis_tile[basis_row..basis_row + lane_count];
                        let (basis_vectors, _) = S::as_mut_simd_f64s(row);
                        basis_vectors[lane_block] = basis;
                    }
                    if pair + 1 < pair_count {
                        let leading = simd.mul_add_f64s(
                            simd.splat_f64s(coefficient_a[pair]),
                            cosine_squared,
                            simd.splat_f64s(coefficient_b[pair]),
                        );
                        let next = simd.mul_add_f64s(leading, basis, previous);
                        if !simd_values_in_normal_range(simd, next) {
                            return false;
                        }
                        previous = basis;
                        basis = next;
                    }
                }

                let mut first_sum0 = vector_sums[0][lane_block];
                let mut first_sum1 = vector_sums[1][lane_block];
                let mut first_sum2 = vector_sums[2][lane_block];
                let mut first_sum3 = vector_sums[3][lane_block];
                for tile_pair in 0..tile_count {
                    let basis_row = tile_pair * lane_count;
                    let (basis_vectors, _) =
                        S::as_simd_f64s(&basis_tile[basis_row..basis_row + lane_count]);
                    let basis = basis_vectors[lane_block];
                    let pair = pair_start + tile_pair;
                    let coefficient = first_even[pair];
                    first_sum0 =
                        simd.mul_add_f64s(basis, simd.splat_f64s(coefficient.re), first_sum0);
                    first_sum1 =
                        simd.mul_add_f64s(basis, simd.splat_f64s(coefficient.im), first_sum1);
                    let coefficient = first_odd[pair];
                    first_sum2 = simd.mul_add_f64s(
                        basis,
                        simd.mul_f64s(cosine, simd.splat_f64s(coefficient.re)),
                        first_sum2,
                    );
                    first_sum3 = simd.mul_add_f64s(
                        basis,
                        simd.mul_f64s(cosine, simd.splat_f64s(coefficient.im)),
                        first_sum3,
                    );
                }
                vector_sums[0][lane_block] = first_sum0;
                vector_sums[1][lane_block] = first_sum1;
                vector_sums[2][lane_block] = first_sum2;
                vector_sums[3][lane_block] = first_sum3;

                let mut second_sum0 = vector_sums[4][lane_block];
                let mut second_sum1 = vector_sums[5][lane_block];
                let mut second_sum2 = vector_sums[6][lane_block];
                let mut second_sum3 = vector_sums[7][lane_block];
                for tile_pair in 0..tile_count {
                    let basis_row = tile_pair * lane_count;
                    let (basis_vectors, _) =
                        S::as_simd_f64s(&basis_tile[basis_row..basis_row + lane_count]);
                    let basis = basis_vectors[lane_block];
                    let pair = pair_start + tile_pair;
                    let coefficient = second_even[pair];
                    second_sum0 =
                        simd.mul_add_f64s(basis, simd.splat_f64s(coefficient.re), second_sum0);
                    second_sum1 =
                        simd.mul_add_f64s(basis, simd.splat_f64s(coefficient.im), second_sum1);
                    let coefficient = second_odd[pair];
                    second_sum2 = simd.mul_add_f64s(
                        basis,
                        simd.mul_f64s(cosine, simd.splat_f64s(coefficient.re)),
                        second_sum2,
                    );
                    second_sum3 = simd.mul_add_f64s(
                        basis,
                        simd.mul_f64s(cosine, simd.splat_f64s(coefficient.im)),
                        second_sum3,
                    );
                }
                vector_sums[4][lane_block] = second_sum0;
                vector_sums[5][lane_block] = second_sum1;
                vector_sums[6][lane_block] = second_sum2;
                vector_sums[7][lane_block] = second_sum3;
                pair_start += tile_count;
            }
        }
        for lane in 0..cos_tail.len() {
            let cosine = cos_tail[lane];
            let mut previous = 0.0;
            let mut basis = current_tail[lane];
            let mut sums = [0.0; 8];
            for pair in 0..pair_count {
                let first_even = first_even[pair];
                let first_odd = first_odd[pair];
                let second_even = second_even[pair];
                let second_odd = second_odd[pair];
                sums[0] = basis.mul_add(first_even.re, sums[0]);
                sums[1] = basis.mul_add(first_even.im, sums[1]);
                sums[2] = basis.mul_add(cosine * first_odd.re, sums[2]);
                sums[3] = basis.mul_add(cosine * first_odd.im, sums[3]);
                sums[4] = basis.mul_add(second_even.re, sums[4]);
                sums[5] = basis.mul_add(second_even.im, sums[5]);
                sums[6] = basis.mul_add(cosine * second_odd.re, sums[6]);
                sums[7] = basis.mul_add(cosine * second_odd.im, sums[7]);
                if pair + 1 < pair_count {
                    let leading = coefficient_a[pair].mul_add(cosine * cosine, coefficient_b[pair]);
                    let next = leading.mul_add(basis, previous);
                    if !scalar_value_in_normal_range(next) {
                        return false;
                    }
                    previous = basis;
                    basis = next;
                }
            }
            for sum in 0..8 {
                tail_sums[sum][lane] = sums[sum];
            }
        }
        true
    }
}

struct FusedSynthesisPairBlock<'a, C> {
    m: usize,
    lmax: usize,
    pair_start: usize,
    cos_theta: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    coefficients: C,
    previous: &'a [f64],
    current: &'a [f64],
    first_sum_re: &'a mut [f64],
    first_sum_im: &'a mut [f64],
    second_sum_re: &'a mut [f64],
    second_sum_im: &'a mut [f64],
    initialize_sums: bool,
    check_normal_range: bool,
}

impl<C: SynthesisPairCoefficientSource> WithSimd for FusedSynthesisPairBlock<'_, C> {
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> bool {
        let FusedSynthesisPairBlock {
            m,
            lmax,
            pair_start,
            cos_theta,
            coefficient_a,
            coefficient_b,
            coefficients,
            previous: initial_previous,
            current,
            first_sum_re,
            first_sum_im,
            second_sum_re,
            second_sum_im,
            initialize_sums,
            check_normal_range,
        } = self;
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
        let (previous_vectors, previous_tail) = S::as_simd_f64s(initial_previous);
        let (initial_vectors, initial_tail) = S::as_simd_f64s(current);
        let (first_sum_re_vectors, first_sum_re_tail) = S::as_mut_simd_f64s(first_sum_re);
        let (first_sum_im_vectors, first_sum_im_tail) = S::as_mut_simd_f64s(first_sum_im);
        let (second_sum_re_vectors, second_sum_re_tail) = S::as_mut_simd_f64s(second_sum_re);
        let (second_sum_im_vectors, second_sum_im_tail) = S::as_mut_simd_f64s(second_sum_im);
        let pair_count = (lmax - m + 2) / 2;
        debug_assert!(pair_start <= pair_count);
        debug_assert_eq!(previous_vectors.len(), initial_vectors.len());
        debug_assert_eq!(previous_tail.len(), initial_tail.len());

        for lane_block in 0..cosine_vectors.len() {
            let cosine = cosine_vectors[lane_block];
            let mut previous = previous_vectors[lane_block];
            let mut current = initial_vectors[lane_block];
            if check_normal_range
                && (!simd_values_in_normal_range(simd, previous)
                    || !simd_values_in_normal_range(simd, current))
            {
                return false;
            }
            let mut first_re = if initialize_sums {
                simd.splat_f64s(0.0)
            } else {
                first_sum_re_vectors[lane_block]
            };
            let mut first_im = if initialize_sums {
                simd.splat_f64s(0.0)
            } else {
                first_sum_im_vectors[lane_block]
            };
            let mut second_re = if initialize_sums {
                simd.splat_f64s(0.0)
            } else {
                second_sum_re_vectors[lane_block]
            };
            let mut second_im = if initialize_sums {
                simd.splat_f64s(0.0)
            } else {
                second_sum_im_vectors[lane_block]
            };
            for pair in pair_start..pair_count {
                let (first_even, first_odd, second_even, second_odd) =
                    coefficients.values(m, lmax, pair);
                let first_coefficient_re = simd.mul_add_f64s(
                    simd.splat_f64s(first_odd.re),
                    cosine,
                    simd.splat_f64s(first_even.re),
                );
                let first_coefficient_im = simd.mul_add_f64s(
                    simd.splat_f64s(first_odd.im),
                    cosine,
                    simd.splat_f64s(first_even.im),
                );
                let second_coefficient_re = simd.mul_add_f64s(
                    simd.splat_f64s(second_odd.re),
                    cosine,
                    simd.splat_f64s(second_even.re),
                );
                let second_coefficient_im = simd.mul_add_f64s(
                    simd.splat_f64s(second_odd.im),
                    cosine,
                    simd.splat_f64s(second_even.im),
                );
                first_re = simd.mul_add_f64s(current, first_coefficient_re, first_re);
                first_im = simd.mul_add_f64s(current, first_coefficient_im, first_im);
                second_re = simd.mul_add_f64s(current, second_coefficient_re, second_re);
                second_im = simd.mul_add_f64s(current, second_coefficient_im, second_im);

                if pair + 1 < pair_count {
                    let cosine_squared = simd.mul_f64s(cosine, cosine);
                    let leading = simd.mul_add_f64s(
                        simd.splat_f64s(coefficient_a[pair]),
                        cosine_squared,
                        simd.splat_f64s(coefficient_b[pair]),
                    );
                    let next = simd.mul_add_f64s(leading, current, previous);
                    previous = current;
                    current = next;
                    // `previous` was the checked `current` from the prior step.
                    if check_normal_range && !simd_values_in_normal_range(simd, current) {
                        return false;
                    }
                }
            }
            first_sum_re_vectors[lane_block] = first_re;
            first_sum_im_vectors[lane_block] = first_im;
            second_sum_re_vectors[lane_block] = second_re;
            second_sum_im_vectors[lane_block] = second_im;
        }

        for lane in 0..cosine_tail.len() {
            let cosine = cosine_tail[lane];
            let mut previous = previous_tail[lane];
            let mut current = initial_tail[lane];
            if check_normal_range
                && (!scalar_value_in_normal_range(previous)
                    || !scalar_value_in_normal_range(current))
            {
                return false;
            }
            let mut first_re = if initialize_sums {
                0.0
            } else {
                first_sum_re_tail[lane]
            };
            let mut first_im = if initialize_sums {
                0.0
            } else {
                first_sum_im_tail[lane]
            };
            let mut second_re = if initialize_sums {
                0.0
            } else {
                second_sum_re_tail[lane]
            };
            let mut second_im = if initialize_sums {
                0.0
            } else {
                second_sum_im_tail[lane]
            };
            for pair in pair_start..pair_count {
                let (first_even, first_odd, second_even, second_odd) =
                    coefficients.values(m, lmax, pair);
                first_re = current.mul_add(first_odd.re.mul_add(cosine, first_even.re), first_re);
                first_im = current.mul_add(first_odd.im.mul_add(cosine, first_even.im), first_im);
                second_re =
                    current.mul_add(second_odd.re.mul_add(cosine, second_even.re), second_re);
                second_im =
                    current.mul_add(second_odd.im.mul_add(cosine, second_even.im), second_im);

                if pair + 1 < pair_count {
                    let leading = coefficient_a[pair].mul_add(cosine * cosine, coefficient_b[pair]);
                    let next = leading.mul_add(current, previous);
                    previous = current;
                    current = next;
                    // `previous` was the checked `current` from the prior step.
                    if check_normal_range && !scalar_value_in_normal_range(current) {
                        return false;
                    }
                }
            }
            first_sum_re_tail[lane] = first_re;
            first_sum_im_tail[lane] = first_im;
            second_sum_re_tail[lane] = second_re;
            second_sum_im_tail[lane] = second_im;
        }
        true
    }
}

/// Visits transformed scalar basis values for degree pairs over latitude lanes.
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

/// Visits paired scalar basis values and reports whether all lanes are in the
/// ordinary floating-point range at each degree pair.
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
    mut visit: F,
) -> Result<(), ShtError>
where
    F: FnMut(usize, usize, &[f64], &[i64], bool) -> Result<(), ShtError>,
{
    debug_assert!(m <= lmax);
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
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
            visit: &mut visit,
        })?;
    }
    Ok(())
}

struct ForEachPairedYlm<'a, F> {
    ring_start: usize,
    m: usize,
    lmax: usize,
    sin_theta: &'a [f64],
    cos_theta: &'a [f64],
    sectoral_norm: f64,
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
    exponents: &'a mut [i64],
    visit: F,
}

impl<F> WithSimd for ForEachPairedYlm<'_, F>
where
    F: FnMut(usize, usize, &[f64], &[i64], bool) -> Result<(), ShtError>,
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
        } = self;
        let mut previous = previous;
        let mut current = current;
        let mut next = next;

        for ring in 0..sin_theta.len() {
            let (value, exponent) = scalar::sectoral_value(sin_theta[ring], m, sectoral_norm)?;
            previous[ring] = 0.0;
            current[ring] = value;
            exponents[ring] = exponent;
        }
        let mut normal_range = scalar::normalize_pair_lanes(previous, current, exponents)?;

        for (pair_index, l) in (m..=lmax).step_by(2).enumerate() {
            visit(l, ring_start, current, exponents, normal_range)?;
            if l + 2 > lmax {
                break;
            }

            let (current_vectors, current_tail) = S::as_simd_f64s(current);
            let (previous_vectors, previous_tail) = S::as_simd_f64s(previous);
            let (cos_vectors, cos_tail) = S::as_simd_f64s(cos_theta);
            let (next_vectors, next_tail) = S::as_mut_simd_f64s(next);
            let a = simd.splat_f64s(coefficient_a[pair_index]);
            let b = simd.splat_f64s(coefficient_b[pair_index]);
            for index in 0..current_vectors.len() {
                let cos_squared = simd.mul_f64s(cos_vectors[index], cos_vectors[index]);
                let leading = simd.mul_add_f64s(a, cos_squared, b);
                next_vectors[index] =
                    simd.mul_add_f64s(leading, current_vectors[index], previous_vectors[index]);
            }
            for index in 0..current_tail.len() {
                let cos_squared = cos_tail[index] * cos_tail[index];
                let leading =
                    coefficient_a[pair_index].mul_add(cos_squared, coefficient_b[pair_index]);
                next_tail[index] = leading.mul_add(current_tail[index], previous_tail[index]);
            }

            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
            if normal_range {
                if scalar::pair_lanes_need_scaling(simd, previous, current) {
                    normal_range = scalar::normalize_pair_lanes(previous, current, exponents)?;
                }
            } else {
                normal_range = scalar::normalize_pair_lanes(previous, current, exponents)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insignificant_prefix_uses_ducc_threshold_at_all_latitude_counts() {
        assert_eq!(ANALYSIS_FTOL, 2.0_f64.powi(-60));
        assert!(!insignificant_prefix_skip_allowed(0));
        for latitude_count in [1, 2047, 2048, 2049, 3073, 4096, 8192] {
            assert!(insignificant_prefix_skip_allowed(latitude_count));
        }
    }

    struct F64LaneCount;

    impl WithSimd for F64LaneCount {
        type Output = usize;

        fn with_simd<S: Simd>(self, _: S) -> Self::Output {
            S::F64_LANES
        }
    }

    struct ScaledPairAccumulationProbe<'a> {
        cos_theta: &'a [f64],
        current: &'a [f64],
        exponents: &'a [i64],
        first_even: Complex64,
        first_odd: Complex64,
        second_even: Complex64,
        second_odd: Complex64,
        first_sum_re: &'a mut [f64],
        first_sum_im: &'a mut [f64],
        second_sum_re: &'a mut [f64],
        second_sum_im: &'a mut [f64],
    }

    impl WithSimd for ScaledPairAccumulationProbe<'_> {
        type Output = bool;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            try_accumulate_scaled_pair_vector(
                simd,
                self.cos_theta,
                self.current,
                self.exponents,
                self.first_even,
                self.first_odd,
                self.second_even,
                self.second_odd,
                self.first_sum_re,
                self.first_sum_im,
                self.second_sum_re,
                self.second_sum_im,
            )
        }
    }

    #[test]
    fn scaled_synthesis_prefix_vector_matches_scalar_and_declines_unsafe_products() {
        let lane_count = Arch::new().dispatch(F64LaneCount);
        if lane_count < 2 {
            return;
        }
        let cos_theta = (0..lane_count)
            .map(|lane| -0.4 + 0.3 * lane as f64)
            .collect::<Vec<_>>();
        let current = (0..lane_count)
            .map(|lane| {
                if lane == 0 {
                    0.0
                } else {
                    0.75 + 0.25 * lane as f64
                }
            })
            .collect::<Vec<_>>();
        let exponents = (0..lane_count)
            .map(|lane| {
                if lane == 0 {
                    i64::MAX
                } else {
                    [4, -4, 0, 1, -2, 3, 2, -1][lane % 8]
                }
            })
            .collect::<Vec<_>>();
        let first_even = Complex64::new(0.5, -0.75);
        let first_odd = Complex64::new(0.125, 0.25);
        let second_even = Complex64::new(-0.75, 0.625);
        let second_odd = Complex64::new(0.2, -0.15);
        let mut first_sum_re = vec![0.125; lane_count];
        let mut first_sum_im = vec![-0.25; lane_count];
        let mut second_sum_re = vec![-0.375; lane_count];
        let mut second_sum_im = vec![0.5; lane_count];
        let mut expected = [
            first_sum_re.clone(),
            first_sum_im.clone(),
            second_sum_re.clone(),
            second_sum_im.clone(),
        ];
        for lane in 0..lane_count {
            let cosine = cos_theta[lane];
            let first_factor = Complex64::new(
                first_odd.re.mul_add(cosine, first_even.re),
                first_odd.im.mul_add(cosine, first_even.im),
            );
            let second_factor = Complex64::new(
                second_odd.re.mul_add(cosine, second_even.re),
                second_odd.im.mul_add(cosine, second_even.im),
            );
            let first =
                scalar::multiply_complex_scaled(first_factor, current[lane], exponents[lane]);
            let second =
                scalar::multiply_complex_scaled(second_factor, current[lane], exponents[lane]);
            expected[0][lane] += first.re;
            expected[1][lane] += first.im;
            expected[2][lane] += second.re;
            expected[3][lane] += second.im;
        }

        assert!(Arch::new().dispatch(ScaledPairAccumulationProbe {
            cos_theta: &cos_theta,
            current: &current,
            exponents: &exponents,
            first_even,
            first_odd,
            second_even,
            second_odd,
            first_sum_re: &mut first_sum_re,
            first_sum_im: &mut first_sum_im,
            second_sum_re: &mut second_sum_re,
            second_sum_im: &mut second_sum_im,
        }));
        for (actual, expected) in [&first_sum_re, &first_sum_im, &second_sum_re, &second_sum_im]
            .into_iter()
            .zip(&expected)
        {
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(actual, expected)| { actual.to_bits() == expected.to_bits() })
            );
        }

        let tiny = 1e-200;
        let unsafe_even = Complex64::new(tiny, tiny);
        let unsafe_odd = Complex64::new(0.0, 0.0);
        let mut unsafe_first_re = vec![1.0; lane_count];
        let mut unsafe_first_im = vec![2.0; lane_count];
        let mut unsafe_second_re = vec![3.0; lane_count];
        let mut unsafe_second_im = vec![4.0; lane_count];
        let before = [
            unsafe_first_re.clone(),
            unsafe_first_im.clone(),
            unsafe_second_re.clone(),
            unsafe_second_im.clone(),
        ];
        assert!(!Arch::new().dispatch(ScaledPairAccumulationProbe {
            cos_theta: &cos_theta,
            current: &vec![tiny; lane_count],
            exponents: &vec![0; lane_count],
            first_even: unsafe_even,
            first_odd: unsafe_odd,
            second_even: unsafe_even,
            second_odd: unsafe_odd,
            first_sum_re: &mut unsafe_first_re,
            first_sum_im: &mut unsafe_first_im,
            second_sum_re: &mut unsafe_second_re,
            second_sum_im: &mut unsafe_second_im,
        }));
        for (actual, before) in [
            &unsafe_first_re,
            &unsafe_first_im,
            &unsafe_second_re,
            &unsafe_second_im,
        ]
        .into_iter()
        .zip(&before)
        {
            assert_eq!(actual, before);
        }

        let mut overflow_first_re = vec![1.0; lane_count];
        let mut overflow_first_im = vec![2.0; lane_count];
        let mut overflow_second_re = vec![3.0; lane_count];
        let mut overflow_second_im = vec![4.0; lane_count];
        let before = [
            overflow_first_re.clone(),
            overflow_first_im.clone(),
            overflow_second_re.clone(),
            overflow_second_im.clone(),
        ];
        assert!(!Arch::new().dispatch(ScaledPairAccumulationProbe {
            cos_theta: &cos_theta,
            current: &vec![1.0; lane_count],
            exponents: &vec![i64::MAX; lane_count],
            first_even,
            first_odd,
            second_even,
            second_odd,
            first_sum_re: &mut overflow_first_re,
            first_sum_im: &mut overflow_first_im,
            second_sum_re: &mut overflow_second_re,
            second_sum_im: &mut overflow_second_im,
        }));
        for (actual, before) in [
            &overflow_first_re,
            &overflow_first_im,
            &overflow_second_re,
            &overflow_second_im,
        ]
        .into_iter()
        .zip(&before)
        {
            assert_eq!(actual, before);
        }
    }

    struct ScaleEpochBelowProbe<'a>(&'a [f64]);

    impl WithSimd for ScaleEpochBelowProbe<'_> {
        type Output = bool;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            analysis_scale_epochs_below_one(simd, self.0)
        }
    }

    struct AdvanceAnalysisPrefixProbe<'a> {
        cosine_squared: &'a [f64],
        previous: &'a mut [f64],
        current: &'a mut [f64],
        scale_epochs: &'a mut [f64],
        corrections: &'a mut [f64],
        skip_insignificant_validation: bool,
    }

    impl WithSimd for AdvanceAnalysisPrefixProbe<'_> {
        type Output = Option<(bool, bool, bool, bool)>;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            let Self {
                cosine_squared,
                previous,
                current,
                scale_epochs,
                corrections,
                skip_insignificant_validation,
            } = self;
            advance_and_rescale_analysis_pair(
                simd,
                [(0.0, 1.0), (0.0, 1.0)],
                cosine_squared,
                (previous, current),
                scale_epochs,
                corrections,
                skip_insignificant_validation,
            )
            .map(|(below_limit, remains_insignificant_prefix)| {
                (
                    below_limit,
                    remains_insignificant_prefix,
                    analysis_scale_epochs_below_one(simd, scale_epochs),
                    scale_epochs.iter().all(|&epoch| epoch < 0.0),
                )
            })
        }
    }

    struct AnalysisMaskAllProbe;

    impl WithSimd for AnalysisMaskAllProbe {
        type Output = (bool, bool, bool);

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            let one = simd.splat_f64s(1.0);
            let zero = simd.splat_f64s(0.0);
            let all_true = analysis_mask_all(simd, simd.equal_f64s(one, one));
            let all_false = analysis_mask_all(simd, simd.equal_f64s(one, zero));
            let values: Vec<_> = (0..S::F64_LANES)
                .map(|lane| if lane == 0 { 0.0 } else { 1.0 })
                .collect();
            let (vectors, tail) = S::as_simd_f64s(&values);
            assert_eq!(vectors.len(), 1);
            assert!(tail.is_empty());
            let one_false_lane = analysis_mask_all(simd, simd.equal_f64s(vectors[0], one));
            (all_true, all_false, one_false_lane)
        }
    }

    #[test]
    fn analysis_mask_all_handles_all_and_mixed_lanes() {
        assert_eq!(
            Arch::new().dispatch(AnalysisMaskAllProbe),
            (true, false, false)
        );
    }

    #[test]
    fn prefix_advance_carries_the_epoch_frontier_across_vectors_and_tails() {
        for lane_count in [4, 5, 8, 9] {
            let cosine_squared = vec![0.25; lane_count];
            let mut previous = vec![0.0; lane_count];
            let mut current = vec![ANALYSIS_FTOL * 0.5; lane_count];
            let mut scale_epochs = vec![-1.0; lane_count];
            let mut corrections = vec![0.0; lane_count];

            assert_eq!(
                Arch::new().dispatch(AdvanceAnalysisPrefixProbe {
                    cosine_squared: &cosine_squared,
                    previous: &mut previous,
                    current: &mut current,
                    scale_epochs: &mut scale_epochs,
                    corrections: &mut corrections,
                    skip_insignificant_validation: true,
                }),
                Some((true, true, true, true)),
                "no-crossing lanes={lane_count}"
            );

            previous.fill(0.0);
            current.fill(0.5);
            scale_epochs.fill(-1.0);
            corrections.fill(0.0);
            assert_eq!(
                Arch::new().dispatch(AdvanceAnalysisPrefixProbe {
                    cosine_squared: &cosine_squared,
                    previous: &mut previous,
                    current: &mut current,
                    scale_epochs: &mut scale_epochs,
                    corrections: &mut corrections,
                    skip_insignificant_validation: true,
                }),
                Some((true, false, true, false)),
                "epoch-zero lanes={lane_count}"
            );

            previous.fill(0.0);
            current.fill(0.5);
            scale_epochs.fill(-1.0);
            corrections.fill(0.0);
            scale_epochs[0] = 0.0;
            corrections[0] = 1.0;
            assert_eq!(
                Arch::new().dispatch(AdvanceAnalysisPrefixProbe {
                    cosine_squared: &cosine_squared,
                    previous: &mut previous,
                    current: &mut current,
                    scale_epochs: &mut scale_epochs,
                    corrections: &mut corrections,
                    skip_insignificant_validation: false,
                }),
                Some((false, false, false, false)),
                "crossing-lane lanes={lane_count}"
            );

            if lane_count >= 8 {
                previous.fill(0.0);
                current.fill(ANALYSIS_FTOL * 0.5);
                current[4..8].fill(0.5);
                scale_epochs.fill(-1.0);
                corrections.fill(0.0);
                scale_epochs[4] = 0.0;
                corrections[4] = 1.0;
                assert_eq!(
                    Arch::new().dispatch(AdvanceAnalysisPrefixProbe {
                        cosine_squared: &cosine_squared,
                        previous: &mut previous,
                        current: &mut current,
                        scale_epochs: &mut scale_epochs,
                        corrections: &mut corrections,
                        skip_insignificant_validation: false,
                    }),
                    Some((false, false, false, false)),
                    "sparse-crossing-vector lanes={lane_count}"
                );
            }
        }
    }

    #[test]
    fn vector_sectoral_initialization_matches_high_order_reference_and_tail() {
        let m = 720;
        let lane_count = 21;
        let sin_theta: Vec<_> = (0..lane_count)
            .map(|lane| 0.71 + lane as f64 * 0.001)
            .collect();
        let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
        let mut current = vec![0.0; lane_count];

        let initialization = initialize_normal_analysis_sectoral_dispatched(
            Arch::new(),
            m,
            &sin_theta,
            sectoral_norm,
            &mut current,
        )
        .unwrap();
        assert!(initialization.complete);
        assert_eq!(
            initialization.completed_vectors,
            lane_count / Arch::new().dispatch(F64LaneCount)
        );

        for (&sine, &actual) in sin_theta.iter().zip(&current) {
            let (expected, exponent) = scalar::sectoral_value(sine, m, sectoral_norm).unwrap();
            assert_eq!(exponent, 0);
            assert!((actual - expected).abs() <= expected.abs() * 2.0e-14);
        }
    }

    #[test]
    fn scaled_sectoral_initialization_matches_ducc_power_routes_and_tail() {
        let arch = Arch::new();
        let sin_theta = [
            0.0, 1.0e-30, 0.2, 0.5, 0.7, 0.95, 0.999, 1.0, 0.42, 0.83, 0.91,
        ];

        for m in [0, 1, 250, 720] {
            let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
            let mut current = vec![f64::NAN; sin_theta.len()];
            let mut scale_epochs = vec![-1.0; sin_theta.len()];
            assert!(
                arch.dispatch(InitializeScaledAnalysisSectoral {
                    m,
                    sin_theta: &sin_theta,
                    sectoral_norm,
                    normal_prefix_vectors: 0,
                    current: &mut current,
                    scale_epochs: &mut scale_epochs,
                })
                .is_some()
            );

            for (lane, (&sine, &actual)) in sin_theta.iter().zip(&current).enumerate() {
                let (value, exponent) = scalar::sectoral_value(sine, m, sectoral_norm).unwrap();
                let (expected, exponent) = normalize_analysis_seed_scalar(value, exponent).unwrap();
                assert_eq!(
                    scale_epochs[lane],
                    exponent as f64 / ANALYSIS_SCALE_STEP as f64
                );
                if expected == 0.0 {
                    assert_eq!(actual, expected);
                } else {
                    assert!((actual - expected).abs() <= expected.abs() * 2.0e-13);
                }
            }
        }
    }

    #[test]
    fn scaled_sectoral_initialization_reuses_normal_vector_prefix() {
        let arch = Arch::new();
        let m = 720;
        let lane_width = arch.dispatch(F64LaneCount);
        let mut sin_theta = vec![0.71; lane_width * 2];
        sin_theta.extend(std::iter::repeat_n(0.5, lane_width));
        sin_theta.push(0.83);
        let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
        let mut current = vec![f64::NAN; sin_theta.len()];
        let initialization = initialize_normal_analysis_sectoral_dispatched(
            arch,
            m,
            &sin_theta,
            sectoral_norm,
            &mut current,
        )
        .unwrap();
        assert!(!initialization.complete);
        assert_eq!(initialization.completed_vectors, 2);

        let prefix_len = initialization.completed_vectors * lane_width;
        for lane in 0..prefix_len {
            let (expected, exponent) =
                scalar::sectoral_value(sin_theta[lane], m, sectoral_norm).unwrap();
            assert_eq!(exponent, 0);
            assert!((current[lane] - expected).abs() <= expected.abs() * 2.0e-14);
        }
        current[..prefix_len].fill(1234.5);
        let mut scale_epochs = vec![-7.0; sin_theta.len()];
        let result = arch.dispatch(InitializeScaledAnalysisSectoral {
            m,
            sin_theta: &sin_theta,
            sectoral_norm,
            normal_prefix_vectors: initialization.completed_vectors,
            current: &mut current,
            scale_epochs: &mut scale_epochs,
        });
        assert_eq!(result, Some(()));

        for lane in 0..prefix_len {
            assert_eq!(current[lane], 1234.5);
            assert_eq!(scale_epochs[lane], 0.0);
        }
        for lane in prefix_len..sin_theta.len() {
            let (value, exponent) =
                scalar::sectoral_value(sin_theta[lane], m, sectoral_norm).unwrap();
            let (expected, exponent) = normalize_analysis_seed_scalar(value, exponent).unwrap();
            assert_eq!(
                scale_epochs[lane],
                exponent as f64 / ANALYSIS_SCALE_STEP as f64
            );
            if expected == 0.0 {
                assert_eq!(current[lane], expected);
            } else {
                assert!((current[lane] - expected).abs() <= expected.abs() * 2.0e-13);
            }
        }

        let mut previous = vec![f64::NAN; sin_theta.len()];
        current.fill(f64::NAN);
        scale_epochs.fill(-7.0);
        assert_eq!(
            initialize_scaled_analysis_sectoral_lanes(
                arch,
                m,
                m,
                &sin_theta,
                sectoral_norm,
                &mut previous,
                &mut current,
                &mut scale_epochs,
            )
            .unwrap(),
            Some(false)
        );
        assert!(previous.iter().all(|&value| value == 0.0));
        assert!(scale_epochs[..prefix_len].iter().all(|&epoch| epoch == 0.0));
        for (lane, &actual) in current.iter().enumerate() {
            let (value, exponent) =
                scalar::sectoral_value(sin_theta[lane], m, sectoral_norm).unwrap();
            let (expected, _) = normalize_analysis_seed_scalar(value, exponent).unwrap();
            if expected == 0.0 {
                assert_eq!(actual, expected);
            } else {
                assert!((actual - expected).abs() <= expected.abs() * 2.0e-13);
            }
        }
    }

    #[test]
    fn fused_scalar_analysis_block_matches_paired_reference_with_simd_tails() {
        let m = 2;
        let lmax = 511;
        let lane_count = 21;
        let angles: Vec<_> = (0..lane_count)
            .map(|lane| 0.04 + lane as f64 * 0.075)
            .collect();
        let sin_theta: Vec<_> = angles.iter().map(|angle| angle.sin()).collect();
        let cos_theta: Vec<_> = angles.iter().map(|angle| angle.cos()).collect();
        let same_real: Vec<_> = (0..lane_count)
            .map(|lane| (lane + 1) as f64 * 0.0013)
            .collect();
        let same_imag: Vec<_> = (0..lane_count)
            .map(|lane| -((lane + 2) as f64) * 0.0009)
            .collect();
        let opposite_real: Vec<_> = (0..lane_count)
            .map(|lane| (lane + 3) as f64 * 0.0011)
            .collect();
        let opposite_imag: Vec<_> = (0..lane_count)
            .map(|lane| -((lane + 4) as f64) * 0.0007)
            .collect();
        let mut epsilon = vec![0.0; lmax + 4];
        scalar::fill_epsilon(m, lmax + 3, &mut epsilon);
        let mut alpha = vec![0.0; lmax / 2 + 2];
        let mut coefficient_a = vec![0.0; lmax / 2 + 2];
        let mut coefficient_b = vec![0.0; lmax / 2 + 2];
        fill_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        );
        let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
        let coefficient_count = lmax - m + 1;
        let mut staged = vec![Complex64::new(17.0, -9.0); coefficient_count];
        let mut previous = vec![0.0; lane_count];
        let mut current = vec![0.0; lane_count];
        let mut next = vec![0.0; lane_count];
        let mut exponents = vec![0; lane_count];
        let arch = Arch::new();

        let fused = fused_scalar_analysis_pair_block_dispatched(
            arch,
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            &same_real,
            &same_imag,
            &opposite_real,
            &opposite_imag,
            &mut staged,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
        )
        .unwrap();
        assert!(fused);

        let mut reference = vec![Complex64::new(0.0, 0.0); coefficient_count];
        for_each_y_lm_paired_status_dispatched(
            arch,
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
            |l, ring_start, basis, lane_exponents, normal_range| {
                let lane_end = ring_start + basis.len();
                let (same, opposite) = accumulate_analysis_pair_dispatched(
                    arch,
                    true,
                    normal_range,
                    &same_real[ring_start..lane_end],
                    &same_imag[ring_start..lane_end],
                    &opposite_real[ring_start..lane_end],
                    &opposite_imag[ring_start..lane_end],
                    basis,
                    lane_exponents,
                    &cos_theta[ring_start..lane_end],
                );
                let offset = l - m;
                reference[offset] += same;
                if l < lmax {
                    reference[offset + 1] += opposite;
                }
                Ok(())
            },
        )
        .unwrap();

        for (actual, expected) in staged.iter().zip(&reference) {
            assert!((*actual - expected).norm() <= 2.0e-14);
        }

        let weighted_opposite_real: Vec<_> = opposite_real
            .iter()
            .zip(&cos_theta)
            .map(|(&spectrum, &cosine)| spectrum * cosine)
            .collect();
        let weighted_opposite_imag: Vec<_> = opposite_imag
            .iter()
            .zip(&cos_theta)
            .map(|(&spectrum, &cosine)| spectrum * cosine)
            .collect();
        let cos_theta_squared: Vec<_> = sin_theta
            .iter()
            .zip(&cos_theta)
            .map(|(&sine, &cosine)| {
                if cosine.abs() > 0.99 {
                    (1.0 - sine) * (1.0 + sine)
                } else {
                    cosine * cosine
                }
            })
            .collect();
        let mut staged_preweighted = vec![Complex64::new(17.0, -9.0); coefficient_count];
        previous.fill(0.0);
        current.fill(0.0);
        next.fill(0.0);
        let mut scale_epochs = vec![0.0; sin_theta.len()];
        let mut range_check_was_exercised = false;
        exponents.fill(0);
        let fused_preweighted = fused_scalar_analysis_pair_block_preweighted_dispatched(
            arch,
            m,
            lmax,
            false,
            false,
            &mut range_check_was_exercised,
            &sin_theta,
            &cos_theta,
            &cos_theta_squared,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            &same_real,
            &same_imag,
            &weighted_opposite_real,
            &weighted_opposite_imag,
            &mut staged_preweighted,
            &mut previous,
            &mut current,
            &mut next,
            &mut scale_epochs,
            &mut [],
            &mut exponents,
        )
        .unwrap();
        assert!(fused_preweighted);
        for (actual, expected) in staged_preweighted.iter().zip(&reference) {
            assert!((*actual - expected).norm() <= 2.0e-14);
        }
    }

    #[test]
    fn analysis_preparation_rejects_subnormal_cosine_weight() {
        let mut opposite_real = [1.0];
        let mut opposite_imag = [0.0];
        assert!(!prepare_analysis_opposite(
            &mut opposite_real,
            &mut opposite_imag,
            &[1.0e-310],
        ));
        let mut zero_real = [0.0];
        let mut zero_imag = [0.0];
        assert!(prepare_analysis_opposite(
            &mut zero_real,
            &mut zero_imag,
            &[1.0e-310],
        ));
        assert_eq!(zero_real, [0.0]);
        assert_eq!(zero_imag, [0.0]);
    }

    #[test]
    fn fused_analysis_input_preparation_matches_separate_preflights() {
        for lane_count in [1, 4, 5, 9] {
            let same_real: Vec<_> = (0..lane_count)
                .map(|lane| 0.2 + lane as f64 * 0.01)
                .collect();
            let same_imag: Vec<_> = (0..lane_count)
                .map(|lane| -0.3 + lane as f64 * 0.008)
                .collect();
            let opposite_real: Vec<_> = (0..lane_count)
                .map(|lane| 0.4 - lane as f64 * 0.007)
                .collect();
            let opposite_imag: Vec<_> = (0..lane_count)
                .map(|lane| -0.1 + lane as f64 * 0.003)
                .collect();
            let cosine: Vec<_> = (0..lane_count)
                .map(|lane| -0.8 + lane as f64 * 1.6 / lane_count.max(1) as f64)
                .collect();
            let products_safe = analysis_products_safe(
                &same_real,
                &same_imag,
                &opposite_real,
                &opposite_imag,
                &cosine,
            );
            let mut separate_real = opposite_real.clone();
            let mut separate_imag = opposite_imag.clone();
            let separate_weighting = products_safe
                && prepare_analysis_opposite(&mut separate_real, &mut separate_imag, &cosine);
            let mut fused_real = opposite_real.clone();
            let mut fused_imag = opposite_imag.clone();
            let fused = prepare_analysis_products_and_opposite(
                &same_real,
                &same_imag,
                &mut fused_real,
                &mut fused_imag,
                &cosine,
            );
            assert_eq!(fused, (products_safe, separate_weighting));
            if separate_weighting {
                assert_eq!(fused_real, separate_real);
                assert_eq!(fused_imag, separate_imag);
            }
        }

        let same_real = [1.0, 1.0];
        let same_imag = [0.0, 0.0];
        let opposite_real = [1.0, 1.0];
        let opposite_imag = [0.0, 0.0];
        let mut weighted_real = opposite_real;
        let mut weighted_imag = opposite_imag;
        assert_eq!(
            prepare_analysis_products_and_opposite(
                &same_real,
                &same_imag,
                &mut weighted_real,
                &mut weighted_imag,
                &[1.0, 1.0e-310],
            ),
            (true, false)
        );
        let mut zero_real = [0.0];
        let mut zero_imag = [0.0];
        assert_eq!(
            prepare_analysis_products_and_opposite(
                &[0.0],
                &[0.0],
                &mut zero_real,
                &mut zero_imag,
                &[1.0e-310],
            ),
            (true, true)
        );
        let mut extreme_real = [0.25, f64::MAX / 16.0];
        let mut extreme_imag = [0.0, 0.0];
        assert_eq!(
            prepare_analysis_products_and_opposite(
                &[0.0, 0.0],
                &[0.0, 0.0],
                &mut extreme_real,
                &mut extreme_imag,
                &[1.0, 1.0],
            ),
            (false, false)
        );
    }

    #[test]
    fn analysis_scale_epoch_preflight_bounds_all_recurrence_steps() {
        assert!(analysis_scale_epochs_allow_recurrence(&[0.0, -3.0], 4));
        assert!(analysis_scale_epochs_allow_recurrence(
            &[ANALYSIS_MAX_SCALE_EPOCH - 2.0],
            2,
        ));
        assert!(!analysis_scale_epochs_allow_recurrence(
            &[ANALYSIS_MAX_SCALE_EPOCH - 1.0],
            2,
        ));
        assert!(!analysis_scale_epochs_allow_recurrence(&[f64::NAN], 1));
        assert!(!analysis_scale_epochs_allow_recurrence(&[f64::INFINITY], 1));
        assert!(!analysis_scale_epochs_allow_recurrence(
            &[-ANALYSIS_MAX_SCALE_EPOCH - 1.0],
            1,
        ));
        if usize::BITS == 64 {
            assert!(!analysis_scale_epochs_allow_recurrence(&[0.0], usize::MAX));
        }
    }

    #[test]
    fn scale_correction_preparation_carries_epoch_frontiers_across_tails() {
        let arch = Arch::new();
        let lane_width = arch.dispatch(F64LaneCount);
        for lane_count in [
            lane_width.saturating_sub(1),
            lane_width,
            lane_width + 1,
            lane_width * 2 + 3,
        ] {
            let scale_epochs: Vec<_> = (0..lane_count)
                .map(|lane| [-3.0, -1.0, 0.0, 0.5, 1.0, 2.0][lane % 6])
                .collect();
            let basis: Vec<_> = scale_epochs
                .iter()
                .map(|&epoch| if epoch < 1.0 { 0.5 } else { f64::MIN_POSITIVE })
                .collect();
            let mut corrections = vec![f64::NAN; lane_count];
            let prepared =
                prepare_analysis_scale_corrections(&basis, &scale_epochs, &mut corrections)
                    .unwrap();

            assert_eq!(
                prepared.0,
                arch.dispatch(ScaleEpochBelowProbe(&scale_epochs)),
                "below-one lane_count={lane_count}"
            );
            assert_eq!(
                prepared.1,
                scale_epochs.iter().all(|&epoch| epoch < 0.0),
                "all-negative lane_count={lane_count}"
            );
            for ((&value, &epoch), &correction) in basis.iter().zip(&scale_epochs).zip(&corrections)
            {
                assert_eq!(correction, analysis_scale_correction(value, epoch).unwrap());
            }
        }

        for invalid_epoch in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut correction = [0.0];
            assert_eq!(
                prepare_analysis_scale_corrections(&[0.5], &[invalid_epoch], &mut correction),
                None
            );
        }
        let mut correction = [0.0];
        assert_eq!(
            prepare_analysis_scale_corrections(&[f64::MAX], &[2.0], &mut correction),
            None
        );
    }

    #[test]
    fn paired_factor_preflight_rejects_nonfinite_recurrence_coefficients() {
        let lmax = 4;
        let epsilon = vec![0.0; lmax + 4];
        let factor_count = lmax / 2 + 2;
        let mut alpha = vec![0.0; factor_count];
        let mut coefficient_a = vec![0.0; factor_count];
        let mut coefficient_b = vec![0.0; factor_count];

        assert!(!fill_factors(
            0,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        ));
    }

    #[test]
    fn normalized_scalar_harmonic_bound_is_below_fast_factor_limit() {
        let max_degree = usize::MAX as f64;
        let addition_theorem_bound =
            ((2.0 * max_degree + 1.0) / (4.0 * std::f64::consts::PI)).sqrt();
        assert!(addition_theorem_bound < PAIRED_FACTOR_LIMIT);
        const { assert!(PAIRED_FACTOR_LIMIT < scalar::BIG) };
    }

    #[test]
    fn fused_two_channel_analysis_block_matches_paired_reference_with_simd_tails() {
        let m = 2;
        let lmax = 511;
        let lane_count = 17;
        let angles: Vec<_> = (0..lane_count)
            .map(|lane| 0.04 + lane as f64 * 0.075)
            .collect();
        let sin_theta: Vec<_> = angles.iter().map(|angle| angle.sin()).collect();
        let cos_theta: Vec<_> = angles.iter().map(|angle| angle.cos()).collect();
        let same_real: [Vec<f64>; 2] = std::array::from_fn(|channel| {
            (0..lane_count)
                .map(|lane| (lane + channel + 1) as f64 * 0.0013)
                .collect()
        });
        let same_imag: [Vec<f64>; 2] = std::array::from_fn(|channel| {
            (0..lane_count)
                .map(|lane| -((lane + channel + 2) as f64) * 0.0009)
                .collect()
        });
        let opposite_real: [Vec<f64>; 2] = std::array::from_fn(|channel| {
            (0..lane_count)
                .map(|lane| (lane + channel + 3) as f64 * 0.0011)
                .collect()
        });
        let opposite_imag: [Vec<f64>; 2] = std::array::from_fn(|channel| {
            (0..lane_count)
                .map(|lane| -((lane + channel + 4) as f64) * 0.0007)
                .collect()
        });
        let mut epsilon = vec![0.0; lmax + 4];
        scalar::fill_epsilon(m, lmax + 3, &mut epsilon);
        let mut alpha = vec![0.0; lmax / 2 + 2];
        let mut coefficient_a = vec![0.0; lmax / 2 + 2];
        let mut coefficient_b = vec![0.0; lmax / 2 + 2];
        fill_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        );
        let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
        let coefficient_count = lmax - m + 1;
        let mut staged_first = vec![Complex64::new(17.0, -9.0); coefficient_count];
        let mut staged_second = vec![Complex64::new(-13.0, 7.0); coefficient_count];
        let mut previous = vec![0.0; lane_count];
        let mut current = vec![0.0; lane_count];
        let mut next = vec![f64::INFINITY; lane_count];
        let mut exponents = vec![0; lane_count];
        let arch = Arch::new();

        let mut range_check_was_exercised = false;
        let fused = fused_scalar_analysis_pair_channels_block_with_range_proof(
            arch,
            true,
            false,
            false,
            false,
            &mut range_check_was_exercised,
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            [&same_real[0], &same_real[1]],
            [&same_imag[0], &same_imag[1]],
            [&opposite_real[0], &opposite_real[1]],
            [&opposite_imag[0], &opposite_imag[1]],
            &mut staged_first,
            &mut staged_second,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
        )
        .unwrap();
        assert!(fused);
        assert!(range_check_was_exercised);
        let checked_first = staged_first.clone();
        let checked_second = staged_second.clone();

        let mut unchecked_first = vec![Complex64::new(17.0, -9.0); coefficient_count];
        let mut unchecked_second = vec![Complex64::new(-13.0, 7.0); coefficient_count];
        range_check_was_exercised = false;
        let unchecked = fused_scalar_analysis_pair_channels_block_with_range_proof(
            arch,
            true,
            false,
            false,
            true,
            &mut range_check_was_exercised,
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            [&same_real[0], &same_real[1]],
            [&same_imag[0], &same_imag[1]],
            [&opposite_real[0], &opposite_real[1]],
            [&opposite_imag[0], &opposite_imag[1]],
            &mut unchecked_first,
            &mut unchecked_second,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
        )
        .unwrap();
        assert!(unchecked);
        assert!(!range_check_was_exercised);
        assert_eq!(unchecked_first, checked_first);
        assert_eq!(unchecked_second, checked_second);

        next.fill(0.0);
        let mut reference: [Vec<Complex64>; 2] =
            std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); coefficient_count]);
        for_each_y_lm_paired_status_dispatched(
            arch,
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
            |l, ring_start, basis, lane_exponents, normal_range| {
                let lane_end = ring_start + basis.len();
                let channel_sums = accumulate_analysis_pair_channels_dispatched(
                    arch,
                    [true, true],
                    normal_range,
                    [
                        &same_real[0][ring_start..lane_end],
                        &same_real[1][ring_start..lane_end],
                    ],
                    [
                        &same_imag[0][ring_start..lane_end],
                        &same_imag[1][ring_start..lane_end],
                    ],
                    [
                        &opposite_real[0][ring_start..lane_end],
                        &opposite_real[1][ring_start..lane_end],
                    ],
                    [
                        &opposite_imag[0][ring_start..lane_end],
                        &opposite_imag[1][ring_start..lane_end],
                    ],
                    basis,
                    lane_exponents,
                    &cos_theta[ring_start..lane_end],
                );
                let offset = l - m;
                for channel in 0..2 {
                    reference[channel][offset] += channel_sums[channel].0;
                    if l < lmax {
                        reference[channel][offset + 1] += channel_sums[channel].1;
                    }
                }
                Ok(())
            },
        )
        .unwrap();

        for (channel, staged) in [&staged_first, &staged_second].into_iter().enumerate() {
            for (actual, expected) in staged.iter().zip(&reference[channel]) {
                assert!((*actual - *expected).norm() <= 2.0e-14);
            }
        }

        let baseline_first = staged_first.clone();
        let baseline_second = staged_second.clone();
        let mut weighted_first = vec![Complex64::new(17.0, -9.0); coefficient_count];
        let mut weighted_second = vec![Complex64::new(-13.0, 7.0); coefficient_count];
        next.fill(0.0);
        let weighted = fused_scalar_analysis_pair_channels_block_dispatched(
            arch,
            true,
            true,
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            [&same_real[0], &same_real[1]],
            [&same_imag[0], &same_imag[1]],
            [&opposite_real[0], &opposite_real[1]],
            [&opposite_imag[0], &opposite_imag[1]],
            &mut weighted_first,
            &mut weighted_second,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
        )
        .unwrap();
        assert!(weighted);
        for (weighted, baseline) in [
            (&weighted_first, &baseline_first),
            (&weighted_second, &baseline_second),
        ] {
            for (actual, expected) in weighted.iter().zip(baseline) {
                assert!((*actual - *expected).norm() <= 5.0e-14);
            }
        }
    }

    #[test]
    fn fused_two_channel_analysis_preserves_opposite_product_range() {
        let lane_count = 9;
        let basis = vec![2.0_f64.powi(-1022); lane_count];
        let cosine = vec![2.0_f64.powi(-54); lane_count];
        let opposite_real: [Vec<f64>; 2] =
            std::array::from_fn(|_| vec![2.0_f64.powi(500); lane_count]);
        let zeros: [Vec<f64>; 2] = std::array::from_fn(|_| vec![0.0; lane_count]);
        let exponents = vec![0; lane_count];

        let sums = accumulate_analysis_pair_channels_dispatched(
            Arch::new(),
            [true, true],
            true,
            [&zeros[0], &zeros[1]],
            [&zeros[0], &zeros[1]],
            [&opposite_real[0], &opposite_real[1]],
            [&zeros[0], &zeros[1]],
            &basis,
            &exponents,
            &cosine,
        );

        let expected = lane_count as f64 * 2.0_f64.powi(-576);
        assert_eq!(sums[0].1.re, expected);
        assert_eq!(sums[1].1.re, expected);
    }

    #[test]
    fn fused_two_channel_analysis_block_declines_on_late_range_failure() {
        let m = 0;
        let lmax = 2;
        let lane_count = 17;
        let sin_theta = vec![1.0; lane_count];
        let same_real = [vec![1.0; lane_count], vec![1.0; lane_count]];
        let same_imag = [vec![0.0; lane_count], vec![0.0; lane_count]];
        let opposite_real = [vec![0.0; lane_count], vec![0.0; lane_count]];
        let opposite_imag = [vec![0.0; lane_count], vec![0.0; lane_count]];
        let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
        let mut coefficient_a = vec![0.0; lmax / 2 + 2];
        let coefficient_b = vec![0.0; lmax / 2 + 2];
        coefficient_a[0] = f64::MAX;
        let mut first_vector_fails = vec![0.0; lane_count];
        first_vector_fails[0] = 1.0;
        let mut later_vector_fails = vec![0.0; lane_count];
        later_vector_fails[8] = 1.0;
        let mut scalar_tail_fails = vec![0.0; lane_count];
        scalar_tail_fails[lane_count - 1] = 1.0;

        for cos_theta in [first_vector_fails, later_vector_fails, scalar_tail_fails] {
            let coefficient_count = lmax - m + 1;
            let mut staged_first = vec![Complex64::new(0.0, 0.0); coefficient_count];
            let mut staged_second = vec![Complex64::new(0.0, 0.0); coefficient_count];
            let mut previous = vec![0.0; lane_count];
            let mut current = vec![0.0; lane_count];
            let mut next = vec![0.0; lane_count];
            let mut exponents = vec![0; lane_count];

            let fused = fused_scalar_analysis_pair_channels_block_dispatched(
                Arch::new(),
                true,
                false,
                m,
                lmax,
                &sin_theta,
                &cos_theta,
                sectoral_norm,
                &coefficient_a,
                &coefficient_b,
                [&same_real[0], &same_real[1]],
                [&same_imag[0], &same_imag[1]],
                [&opposite_real[0], &opposite_real[1]],
                [&opposite_imag[0], &opposite_imag[1]],
                &mut staged_first,
                &mut staged_second,
                &mut previous,
                &mut current,
                &mut next,
                &mut exponents,
            )
            .unwrap();
            assert!(!fused);
            assert_ne!(staged_first[0], Complex64::new(0.0, 0.0));
            assert_ne!(staged_second[0], Complex64::new(0.0, 0.0));
        }
    }

    #[test]
    fn fused_two_channel_analysis_block_rejects_scaled_sectoral_lanes() {
        let m = 40;
        let lmax = 80;
        let sin_theta = [1.0e-6, 0.4];
        let cos_theta = [1.0, 0.916515138991168];
        let spectra = [0.25, -0.5];
        let mut epsilon = vec![0.0; lmax + 4];
        scalar::fill_epsilon(m, lmax + 3, &mut epsilon);
        let mut alpha = vec![0.0; lmax / 2 + 2];
        let mut coefficient_a = vec![0.0; lmax / 2 + 2];
        let mut coefficient_b = vec![0.0; lmax / 2 + 2];
        fill_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        );
        let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
        let coefficient_count = lmax - m + 1;
        let mut staged_first = vec![Complex64::new(0.0, 0.0); coefficient_count];
        let mut staged_second = vec![Complex64::new(0.0, 0.0); coefficient_count];
        let mut previous = [0.0; 2];
        let mut current = [0.0; 2];
        let mut next = [0.0; 2];
        let mut exponents = [0; 2];

        let fused = fused_scalar_analysis_pair_channels_block_dispatched(
            Arch::new(),
            true,
            false,
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            [&spectra, &spectra],
            [&spectra, &spectra],
            [&spectra, &spectra],
            [&spectra, &spectra],
            &mut staged_first,
            &mut staged_second,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
        )
        .unwrap();
        assert!(!fused);
    }

    #[test]
    #[allow(unexpected_cfgs)]
    fn fused_scalar_analysis_block_transitions_scaled_lanes_to_ordinary_simd() {
        let m = 250;
        let lmax = 720;
        let lane_count = 9;
        let sin_theta: Vec<_> = (0..lane_count)
            .map(|lane| {
                if lane + 1 == lane_count {
                    0.31
                } else {
                    0.31 + lane as f64 * 0.01
                }
            })
            .collect();
        let cos_theta: Vec<_> = sin_theta
            .iter()
            .map(|sine| (1.0 - sine * sine).sqrt())
            .collect();
        let same_real: Vec<_> = (0..lane_count)
            .map(|lane| 0.13 + lane as f64 * 0.007)
            .collect();
        let same_imag: Vec<_> = (0..lane_count)
            .map(|lane| -0.08 + lane as f64 * 0.003)
            .collect();
        let opposite_real: Vec<_> = (0..lane_count)
            .map(|lane| -0.17 + lane as f64 * 0.005)
            .collect();
        let opposite_imag: Vec<_> = (0..lane_count)
            .map(|lane| 0.09 - lane as f64 * 0.004)
            .collect();
        let mut epsilon = vec![0.0; lmax + 4];
        scalar::fill_epsilon(m, lmax + 3, &mut epsilon);
        let mut alpha = vec![0.0; lmax / 2 + 2];
        let mut coefficient_a = vec![0.0; lmax / 2 + 2];
        let mut coefficient_b = vec![0.0; lmax / 2 + 2];
        fill_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        );
        let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
        let coefficient_count = lmax - m + 1;
        let mut staged = vec![Complex64::new(0.0, 0.0); coefficient_count];
        let mut previous = vec![0.0; lane_count];
        let mut current = vec![0.0; lane_count];
        let mut next = vec![0.0; lane_count];
        let mut scale_epochs = vec![0.0; lane_count];
        let mut exponents = vec![0; lane_count];

        assert!(
            sin_theta
                .iter()
                .any(|&sine| scalar::sectoral_value(sine, m, sectoral_norm).unwrap().1 != 0)
        );

        let fused = fused_scalar_analysis_pair_block_dispatched(
            Arch::new(),
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            &same_real,
            &same_imag,
            &opposite_real,
            &opposite_imag,
            &mut staged,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
        )
        .unwrap();
        assert!(fused);

        let mut reference = vec![Complex64::new(0.0, 0.0); coefficient_count];
        let mut first_normal_degree = None;
        for_each_y_lm_paired_status_dispatched(
            Arch::new(),
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
            |l, ring_start, basis, lane_exponents, normal_range| {
                if normal_range && first_normal_degree.is_none() {
                    first_normal_degree = Some(l);
                }
                let lane_end = ring_start + basis.len();
                let (same, opposite) = accumulate_analysis_pair_dispatched(
                    Arch::new(),
                    true,
                    normal_range,
                    &same_real[ring_start..lane_end],
                    &same_imag[ring_start..lane_end],
                    &opposite_real[ring_start..lane_end],
                    &opposite_imag[ring_start..lane_end],
                    basis,
                    lane_exponents,
                    &cos_theta[ring_start..lane_end],
                );
                let offset = l - m;
                reference[offset] += same;
                if l < lmax {
                    reference[offset + 1] += opposite;
                }
                Ok(())
            },
        )
        .unwrap();
        assert!(first_normal_degree.is_some_and(|degree| degree > m));
        let reference_scale = reference
            .iter()
            .map(|value| value.norm())
            .fold(0.0, f64::max);
        for (actual, expected) in staged.iter().zip(&reference) {
            assert!(
                (*actual - *expected).norm() <= reference_scale * 2.0e-12 + 1.0e-300,
                "scaled transition differs: actual={actual:?}, expected={expected:?}"
            );
        }

        let weighted_opposite_real: Vec<_> = opposite_real
            .iter()
            .zip(&cos_theta)
            .map(|(&spectrum, &cosine)| spectrum * cosine)
            .collect();
        let weighted_opposite_imag: Vec<_> = opposite_imag
            .iter()
            .zip(&cos_theta)
            .map(|(&spectrum, &cosine)| spectrum * cosine)
            .collect();
        let cos_theta_squared: Vec<_> = sin_theta
            .iter()
            .zip(&cos_theta)
            .map(|(&sine, &cosine)| {
                if cosine.abs() > 0.99 {
                    (1.0 - sine) * (1.0 + sine)
                } else {
                    cosine * cosine
                }
            })
            .collect();
        assert!(insignificant_prefix_skip_allowed(lane_count));
        let input_scale = same_real
            .iter()
            .zip(&same_imag)
            .chain(weighted_opposite_real.iter().zip(&weighted_opposite_imag))
            .map(|(&real, &imag)| real.hypot(imag))
            .fold(0.0_f64, f64::max);
        let omission_bound = 2.0 * lane_count as f64 * ANALYSIS_FTOL * input_scale;
        let mut checked_staged = vec![Complex64::new(0.0, 0.0); coefficient_count];
        let mut checked_previous = vec![0.0; lane_count];
        let mut checked_current = vec![0.0; lane_count];
        let mut checked_next = vec![0.0; lane_count];
        let mut checked_scale_epochs = vec![0.0; lane_count];
        let mut checked_exponents = vec![0; lane_count];
        let mut checked_range_was_exercised = false;
        assert!(
            fused_scalar_analysis_pair_block_preweighted_dispatched(
                Arch::new(),
                m,
                lmax,
                false,
                false,
                &mut checked_range_was_exercised,
                &sin_theta,
                &cos_theta,
                &cos_theta_squared,
                sectoral_norm,
                &coefficient_a,
                &coefficient_b,
                &same_real,
                &same_imag,
                &weighted_opposite_real,
                &weighted_opposite_imag,
                &mut checked_staged,
                &mut checked_previous,
                &mut checked_current,
                &mut checked_next,
                &mut checked_scale_epochs,
                &mut [],
                &mut checked_exponents,
            )
            .unwrap()
        );
        staged.fill(Complex64::new(0.0, 0.0));
        previous.fill(0.0);
        current.fill(0.0);
        next.fill(0.0);
        scale_epochs.fill(0.0);
        exponents.fill(0);
        let mut scale_corrections = vec![0.0; lane_count];
        let mut range_check_was_exercised = false;
        let fused_preweighted = fused_scalar_analysis_pair_block_preweighted_dispatched(
            Arch::new(),
            m,
            lmax,
            true,
            false,
            &mut range_check_was_exercised,
            &sin_theta,
            &cos_theta,
            &cos_theta_squared,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            &same_real,
            &same_imag,
            &weighted_opposite_real,
            &weighted_opposite_imag,
            &mut staged,
            &mut previous,
            &mut current,
            &mut next,
            &mut scale_epochs,
            &mut scale_corrections,
            &mut exponents,
        )
        .unwrap();
        assert!(fused_preweighted);
        assert_eq!(staged[0], Complex64::new(0.0, 0.0));
        let prefix_len = staged
            .iter()
            .take_while(|&&value| value == Complex64::new(0.0, 0.0))
            .count();
        assert!(prefix_len > 0);
        for (degree_offset, expected) in reference.iter().take(prefix_len).enumerate() {
            assert!(
                expected.norm() <= omission_bound,
                "skipped pair {degree_offset} error {:.3e} exceeds {:.3e}",
                expected.norm(),
                omission_bound,
            );
        }
        for (degree, (actual, checked)) in staged.iter().zip(&checked_staged).enumerate() {
            assert!(
                (*actual - *checked).norm() <= omission_bound,
                "insignificant terms at offset {degree} exceed {omission_bound:e}"
            );
        }
        for (actual, expected) in staged.iter().zip(&reference) {
            assert!(
                (*actual - *expected).norm() <= reference_scale * 2.0e-12 + 1.0e-300,
                "preweighted scaled transition differs: actual={actual:?}, expected={expected:?}"
            );
        }
    }

    #[test]
    fn fused_scalar_analysis_block_declines_on_late_recurrence_range_failure() {
        let m = 0;
        let lmax = 2;
        let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
        let coefficient_a = vec![0.0; lmax / 2 + 2];
        let mut coefficient_b = vec![0.0; lmax / 2 + 2];
        coefficient_b[0] = 1.0e-308;
        let same_real = [1.0; 5];
        let same_imag = [0.0; 5];
        let opposite_real = [0.0; 5];
        let opposite_imag = [0.0; 5];

        for (sin_theta, cos_theta) in [
            ([0.0; 5], [1.0; 5]),
            ([1.0, 1.0, 1.0, 1.0, 0.0], [0.0, 0.0, 0.0, 0.0, 1.0]),
        ] {
            let mut staged = vec![Complex64::new(0.0, 0.0); lmax - m + 1];
            let mut previous = [0.0; 5];
            let mut current = [0.0; 5];
            let mut next = [0.0; 5];
            let mut exponents = [0; 5];
            let fused = fused_scalar_analysis_pair_block_dispatched(
                Arch::new(),
                m,
                lmax,
                &sin_theta,
                &cos_theta,
                sectoral_norm,
                &coefficient_a,
                &coefficient_b,
                &same_real,
                &same_imag,
                &opposite_real,
                &opposite_imag,
                &mut staged,
                &mut previous,
                &mut current,
                &mut next,
                &mut exponents,
            )
            .unwrap();
            assert!(!fused);
            assert_ne!(staged[0], Complex64::new(0.0, 0.0));
        }
    }

    #[test]
    fn dual_analysis_accumulation_matches_independent_channels_and_scaled_fallback() {
        for lane_count in [1, 3, 4, 5, 7, 8, 9, 31, 32, 33] {
            let basis: Vec<_> = (0..lane_count)
                .map(|lane| 0.2 + 0.01 * lane as f64)
                .collect();
            let cosine: Vec<_> = (0..lane_count)
                .map(|lane| -0.8 + 0.02 * lane as f64)
                .collect();
            let exponents = vec![0; lane_count];
            let same_real: [Vec<f64>; 2] = std::array::from_fn(|channel| {
                (0..lane_count)
                    .map(|lane| (lane + channel + 1) as f64 * 1.0e-3)
                    .collect::<Vec<_>>()
            });
            let same_imag: [Vec<f64>; 2] = std::array::from_fn(|channel| {
                (0..lane_count)
                    .map(|lane| -((lane + channel + 2) as f64) * 1.1e-3)
                    .collect::<Vec<_>>()
            });
            let opposite_real: [Vec<f64>; 2] = std::array::from_fn(|channel| {
                (0..lane_count)
                    .map(|lane| (lane + channel + 3) as f64 * 0.9e-3)
                    .collect::<Vec<_>>()
            });
            let opposite_imag: [Vec<f64>; 2] = std::array::from_fn(|channel| {
                (0..lane_count)
                    .map(|lane| -((lane + channel + 4) as f64) * 0.7e-3)
                    .collect::<Vec<_>>()
            });
            let actual = accumulate_analysis_pair_channels_dispatched(
                Arch::new(),
                [true, true],
                true,
                [&same_real[0], &same_real[1]],
                [&same_imag[0], &same_imag[1]],
                [&opposite_real[0], &opposite_real[1]],
                [&opposite_imag[0], &opposite_imag[1]],
                &basis,
                &exponents,
                &cosine,
            );
            for channel in 0..2 {
                let expected = accumulate_analysis_pair_dispatched(
                    Arch::new(),
                    true,
                    true,
                    &same_real[channel],
                    &same_imag[channel],
                    &opposite_real[channel],
                    &opposite_imag[channel],
                    &basis,
                    &exponents,
                    &cosine,
                );
                for (actual, expected) in [
                    (actual[channel].0, expected.0),
                    (actual[channel].1, expected.1),
                ] {
                    let error = (actual - expected).norm();
                    assert!(error <= 2.0e-14, "lanes={lane_count}, error={error:.3e}");
                }
            }
        }

        let basis = [0.5, 0.25, 0.125, 0.75, 0.4];
        let cosine = [-0.7, -0.2, 0.1, 0.5, 0.9];
        let exponents = [0, -100, 0, -20, 0];
        let same_real = [[0.25; 5], [f64::MAX / 8.0; 5]];
        let same_imag = [[-0.5; 5], [0.0; 5]];
        let opposite_real = [[0.75; 5], [f64::MAX / 8.0; 5]];
        let opposite_imag = [[-0.125; 5], [0.0; 5]];
        let products_safe = [
            analysis_products_safe(
                &same_real[0],
                &same_imag[0],
                &opposite_real[0],
                &opposite_imag[0],
                &cosine,
            ),
            analysis_products_safe(
                &same_real[1],
                &same_imag[1],
                &opposite_real[1],
                &opposite_imag[1],
                &cosine,
            ),
        ];
        assert!(products_safe[0]);
        assert!(!products_safe[1]);
        let actual = accumulate_analysis_pair_channels_dispatched(
            Arch::new(),
            products_safe,
            false,
            [&same_real[0], &same_real[1]],
            [&same_imag[0], &same_imag[1]],
            [&opposite_real[0], &opposite_real[1]],
            [&opposite_imag[0], &opposite_imag[1]],
            &basis,
            &exponents,
            &cosine,
        );
        for channel in 0..2 {
            let expected = accumulate_analysis_pair_dispatched(
                Arch::new(),
                products_safe[channel],
                false,
                &same_real[channel],
                &same_imag[channel],
                &opposite_real[channel],
                &opposite_imag[channel],
                &basis,
                &exponents,
                &cosine,
            );
            assert_eq!(actual[channel], expected);
        }
    }

    #[test]
    fn mixed_dual_analysis_accumulation_matches_channel_kernels() {
        for lane_count in [6, LATITUDE_BLOCK_SIZE] {
            let basis: Vec<_> = (0..lane_count)
                .map(|lane| 0.2 + 0.003 * lane as f64)
                .collect();
            let cosine: Vec<_> = (0..lane_count)
                .map(|lane| -0.9 + 1.8 * lane as f64 / lane_count as f64)
                .collect();
            let exponents: Vec<_> = (0..lane_count)
                .map(|lane| match lane % 5 {
                    1 => 800,
                    3 => -800,
                    _ => 0,
                })
                .collect();
            let same_real: [Vec<f64>; 2] = std::array::from_fn(|channel| {
                (0..lane_count)
                    .map(|lane| (lane + channel + 1) as f64 * 1.0e-4)
                    .collect()
            });
            let same_imag: [Vec<f64>; 2] = std::array::from_fn(|channel| {
                (0..lane_count)
                    .map(|lane| -((lane + channel + 2) as f64) * 1.1e-4)
                    .collect()
            });
            let opposite_real: [Vec<f64>; 2] = std::array::from_fn(|channel| {
                (0..lane_count)
                    .map(|lane| (lane + channel + 3) as f64 * 0.9e-4)
                    .collect()
            });
            let opposite_imag: [Vec<f64>; 2] = std::array::from_fn(|channel| {
                (0..lane_count)
                    .map(|lane| -((lane + channel + 4) as f64) * 0.7e-4)
                    .collect()
            });
            for channel in 0..2 {
                assert!(analysis_products_safe(
                    &same_real[channel],
                    &same_imag[channel],
                    &opposite_real[channel],
                    &opposite_imag[channel],
                    &cosine,
                ));
            }

            let actual = accumulate_analysis_pair_channels_dispatched(
                Arch::new(),
                [true, true],
                false,
                [&same_real[0], &same_real[1]],
                [&same_imag[0], &same_imag[1]],
                [&opposite_real[0], &opposite_real[1]],
                [&opposite_imag[0], &opposite_imag[1]],
                &basis,
                &exponents,
                &cosine,
            );
            for channel in 0..2 {
                let expected = accumulate_analysis_pair_dispatched(
                    Arch::new(),
                    true,
                    false,
                    &same_real[channel],
                    &same_imag[channel],
                    &opposite_real[channel],
                    &opposite_imag[channel],
                    &basis,
                    &exponents,
                    &cosine,
                );
                assert_eq!(actual[channel], expected, "lanes={lane_count}");
            }
        }
    }

    #[test]
    fn synthesis_coefficient_preparation_matches_degree_transform_at_boundaries() {
        for m in 0..=4 {
            for lmax in m..=m + 9 {
                let mut epsilon = vec![0.0; lmax + 4];
                for (degree, value) in epsilon.iter_mut().enumerate().skip(m + 1) {
                    let degree_f = degree as f64;
                    let order_f = m as f64;
                    *value = (((degree_f - order_f) * (degree_f + order_f))
                        / ((2.0 * degree_f + 1.0) * (2.0 * degree_f - 1.0)))
                        .sqrt();
                }
                let factor_count = lmax / 2 + 2;
                let mut alpha = vec![0.0; factor_count];
                let mut coefficient_a = vec![0.0; factor_count];
                let mut coefficient_b = vec![0.0; factor_count];
                fill_factors(
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
                        )
                    })
                    .collect();
                let pair_count = (lmax - m + 2) / 2;
                let mut coefficient_even = vec![Complex64::new(0.0, 0.0); pair_count];
                let mut coefficient_odd = vec![Complex64::new(0.0, 0.0); pair_count];

                assert!(prepare_synthesis_coefficients(
                    m,
                    lmax,
                    &epsilon,
                    &alpha,
                    &coefficients,
                    &mut coefficient_even,
                    &mut coefficient_odd,
                ));

                for pair in 0..pair_count {
                    let degree = m + 2 * pair;
                    let offset = degree - m;
                    let next_even = if degree + 2 <= lmax {
                        coefficients[offset + 2]
                    } else {
                        Complex64::new(0.0, 0.0)
                    };
                    let expected_even = (coefficients[offset] * epsilon[degree + 1]
                        + next_even * epsilon[degree + 2])
                        * alpha[pair];
                    let expected_odd = if degree < lmax {
                        coefficients[offset + 1] * alpha[pair]
                    } else {
                        Complex64::new(0.0, 0.0)
                    };
                    assert_eq!(coefficient_even[pair], expected_even);
                    assert_eq!(coefficient_odd[pair], expected_odd);
                }
            }
        }
    }

    #[test]
    fn paired_spin_coefficient_preparation_matches_independent_rows() {
        for m in 0..=4 {
            for lmax in m..=m + 9 {
                let mut epsilon = vec![0.0; lmax + 4];
                scalar::fill_epsilon(m, lmax, &mut epsilon);
                let factor_count = lmax / 2 + 2;
                let mut alpha = vec![0.0; factor_count];
                let mut coefficient_a = vec![0.0; factor_count];
                let mut coefficient_b = vec![0.0; factor_count];
                assert!(fill_factors(
                    m,
                    lmax,
                    &epsilon,
                    &mut alpha,
                    &mut coefficient_a,
                    &mut coefficient_b,
                ));
                let first: Vec<_> = (m..=lmax)
                    .map(|degree| {
                        Complex64::new(
                            (0.31 * degree as f64 + 0.73 * m as f64).sin(),
                            (0.19 * degree as f64 - 0.41 * m as f64).cos(),
                        )
                    })
                    .collect();
                let second: Vec<_> = (m..=lmax)
                    .map(|degree| {
                        Complex64::new(
                            (0.47 * degree as f64 - 0.29 * m as f64).cos(),
                            (0.13 * degree as f64 + 0.67 * m as f64).sin(),
                        )
                    })
                    .collect();
                let pair_count = (lmax - m + 2) / 2;
                let mut first_even = vec![Complex64::new(0.0, 0.0); pair_count];
                let mut first_odd = vec![Complex64::new(0.0, 0.0); pair_count];
                let mut second_even = vec![Complex64::new(0.0, 0.0); pair_count];
                let mut second_odd = vec![Complex64::new(0.0, 0.0); pair_count];
                let mut expected_first_even = first_even.clone();
                let mut expected_first_odd = first_odd.clone();
                let mut expected_second_even = second_even.clone();
                let mut expected_second_odd = second_odd.clone();

                assert!(prepare_synthesis_coefficient_pair(
                    m,
                    lmax,
                    &epsilon,
                    &alpha,
                    &first,
                    &second,
                    &mut first_even,
                    &mut first_odd,
                    &mut second_even,
                    &mut second_odd,
                ));
                assert!(prepare_synthesis_coefficients(
                    m,
                    lmax,
                    &epsilon,
                    &alpha,
                    &first,
                    &mut expected_first_even,
                    &mut expected_first_odd,
                ));
                assert!(prepare_synthesis_coefficients(
                    m,
                    lmax,
                    &epsilon,
                    &alpha,
                    &second,
                    &mut expected_second_even,
                    &mut expected_second_odd,
                ));
                assert_eq!(first_even, expected_first_even);
                assert_eq!(first_odd, expected_first_odd);
                assert_eq!(second_even, expected_second_even);
                assert_eq!(second_odd, expected_second_odd);
            }
        }
    }

    #[test]
    fn paired_spin_coefficient_preparation_rejects_nonfinite_second_row() {
        let (m, lmax) = (0, 1);
        let mut epsilon = vec![0.0; lmax + 4];
        scalar::fill_epsilon(m, lmax, &mut epsilon);
        let alpha = [2.0];
        let first = [Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)];
        let second = [Complex64::new(0.0, 0.0), Complex64::new(f64::MAX, 0.0)];
        let mut first_even = [Complex64::new(0.0, 0.0)];
        let mut first_odd = [Complex64::new(0.0, 0.0)];
        let mut second_even = [Complex64::new(0.0, 0.0)];
        let mut second_odd = [Complex64::new(0.0, 0.0)];

        assert!(!prepare_synthesis_coefficient_pair(
            m,
            lmax,
            &epsilon,
            &alpha,
            &first,
            &second,
            &mut first_even,
            &mut first_odd,
            &mut second_even,
            &mut second_odd,
        ));
    }

    #[test]
    fn synthesis_coefficient_preparation_rejects_nonfinite_transformed_pairs() {
        let (m, lmax) = (0, 1);
        let mut epsilon = vec![0.0; lmax + 4];
        scalar::fill_epsilon(m, lmax, &mut epsilon);
        let factor_count = lmax / 2 + 2;
        let mut alpha = vec![0.0; factor_count];
        let mut coefficient_a = vec![0.0; factor_count];
        let mut coefficient_b = vec![0.0; factor_count];
        fill_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        );

        for input in [1.0, f64::from_bits(1)] {
            let coefficients = [Complex64::new(0.0, 0.0), Complex64::new(input, 0.0)];
            let mut even = [Complex64::new(0.0, 0.0)];
            let mut odd = [Complex64::new(0.0, 0.0)];
            assert!(prepare_synthesis_coefficients(
                m,
                lmax,
                &epsilon,
                &alpha,
                &coefficients,
                &mut even,
                &mut odd,
            ));
            assert!(even[0].re.is_finite() && odd[0].re.is_finite());
        }

        let coefficients = [Complex64::new(0.0, 0.0), Complex64::new(1.1e308, 0.0)];
        let mut even = [Complex64::new(0.0, 0.0)];
        let mut odd = [Complex64::new(0.0, 0.0)];
        assert!(!prepare_synthesis_coefficients(
            m,
            lmax,
            &epsilon,
            &alpha,
            &coefficients,
            &mut even,
            &mut odd,
        ));
        assert_eq!(coefficients[1], Complex64::new(1.1e308, 0.0));
    }

    #[test]
    fn partitioned_synthesis_matches_scaled_reference_for_mixed_initial_lanes() {
        let m = 32;
        let lmax = 63;
        let sine: [f64; 7] = [0.99, 0.95, 0.90, 0.85, 0.80, 1.0e-10, 3.0e-10];
        let sin_theta = sine.to_vec();
        let cos_theta: Vec<_> = sine
            .iter()
            .map(|value| (1.0 - value * value).sqrt())
            .collect();
        let ring_order_limits = vec![lmax; sine.len()];
        let sectoral_norm = scalar::sectoral_normalizations(lmax).unwrap()[m];
        assert_eq!(
            scalar::sectoral_value(sine[0], m, sectoral_norm).unwrap().1,
            0
        );
        assert_ne!(
            scalar::sectoral_value(sine[5], m, sectoral_norm).unwrap().1,
            0
        );
        let mut epsilon = vec![0.0; lmax + 4];
        scalar::fill_epsilon(m, lmax, &mut epsilon);
        let factor_count = lmax / 2 + 2;
        let mut alpha = vec![0.0; factor_count];
        let mut coefficient_a = vec![0.0; factor_count];
        let mut coefficient_b = vec![0.0; factor_count];
        fill_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        );
        let coefficients: Vec<_> = (m..=lmax)
            .map(|degree| {
                Complex64::new((0.31 * degree as f64).sin(), (0.17 * degree as f64).cos())
            })
            .collect();
        let pair_count = (lmax - m + 2) / 2;
        let mut coefficient_even = vec![Complex64::new(0.0, 0.0); pair_count];
        let mut coefficient_odd = vec![Complex64::new(0.0, 0.0); pair_count];
        assert!(prepare_synthesis_coefficients(
            m,
            lmax,
            &epsilon,
            &alpha,
            &coefficients,
            &mut coefficient_even,
            &mut coefficient_odd,
        ));

        let mut actual = vec![Complex64::new(0.0, 0.0); sine.len() * (lmax + 1)];
        let mut previous = vec![0.0; sine.len()];
        let mut current = vec![0.0; sine.len()];
        let mut next = vec![0.0; sine.len()];
        let mut exponents = vec![0; sine.len()];
        let mut compact_theta_sin = vec![0.0; sine.len()];
        let mut compact_theta_cos = vec![0.0; sine.len()];
        let mut compact_ring_indices = vec![0; sine.len()];
        let used = synthesize_order_block_partitioned_dispatched(
            Arch::new(),
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            &coefficient_even,
            &coefficient_odd,
            &ring_order_limits,
            lmax + 1,
            &mut actual,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
            &mut compact_theta_sin,
            &mut compact_theta_cos,
            &mut compact_ring_indices,
        )
        .unwrap();
        assert!(used);

        let mut expected = vec![Complex64::new(0.0, 0.0); actual.len()];
        for_each_y_lm_paired_dispatched(
            Arch::new(),
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
            |l, local_ring_start, basis, lane_exponents| {
                let pair = (l - m) / 2;
                for (lane, &value) in basis.iter().enumerate() {
                    let ring = local_ring_start + lane;
                    expected[ring * (lmax + 1) + m] += scalar::multiply_complex_scaled(
                        coefficient_even[pair],
                        value,
                        lane_exponents[lane],
                    ) + scalar::multiply_complex_scaled_product(
                        coefficient_odd[pair],
                        value,
                        cos_theta[ring],
                        lane_exponents[lane],
                    );
                }
                Ok(())
            },
        )
        .unwrap();

        for ring in 0..sine.len() {
            let difference =
                (actual[ring * (lmax + 1) + m] - expected[ring * (lmax + 1) + m]).norm();
            assert!(difference < 3.0e-11, "ring={ring} error={difference:.3e}");
        }
    }

    #[test]
    fn fused_synthesis_continuation_declines_on_late_tail_range_failure() {
        let lane_count = 5;
        let cosine = vec![0.5; lane_count];
        let epsilon = vec![1.0; 8];
        let alpha = vec![1.0; 4];
        let coefficient_a = [12.0, 0.0, 0.0, 0.0];
        let coefficient_b = [0.0; 4];
        let first_coefficients = [Complex64::new(1.0, 0.5); 4];
        let second_coefficients = [Complex64::new(-0.25, 0.75); 4];
        let previous = [0.0; 5];
        let current = [1.0, 1.0, 1.0, 1.0, scalar::BIG * 0.75];
        let mut first_sum_re = [10.0; 5];
        let mut first_sum_im = [20.0; 5];
        let mut second_sum_re = [30.0; 5];
        let mut second_sum_im = [40.0; 5];

        let completed = Arch::new().dispatch(FusedSynthesisPairBlock {
            m: 0,
            lmax: 3,
            pair_start: 0,
            cos_theta: &cosine,
            coefficient_a: &coefficient_a,
            coefficient_b: &coefficient_b,
            coefficients: OriginalSynthesisPairCoefficients {
                epsilon: &epsilon,
                alpha: &alpha,
                first_coefficients: &first_coefficients,
                second_coefficients: &second_coefficients,
            },
            previous: &previous,
            current: &current,
            first_sum_re: &mut first_sum_re,
            first_sum_im: &mut first_sum_im,
            second_sum_re: &mut second_sum_re,
            second_sum_im: &mut second_sum_im,
            initialize_sums: false,
            check_normal_range: true,
        });

        assert!(!completed);
        assert_ne!(first_sum_re[0], 10.0);
        assert_ne!(second_sum_im[3], 40.0);
        assert_eq!(first_sum_re[4], 10.0);
        assert_eq!(first_sum_im[4], 20.0);
        assert_eq!(second_sum_re[4], 30.0);
        assert_eq!(second_sum_im[4], 40.0);
    }

    #[test]
    fn prepared_order_pair_transition_matches_original_coefficients() {
        let m = 0;
        let lmax = 3;
        let pair_count = (lmax - m + 2) / 2;
        let sin_theta = [0.2, 0.7, 0.98];
        let cos_theta = sin_theta.map(|sine| (1.0 - sine * sine).sqrt());
        let sectoral_norm = 0.75;
        let epsilon = [1.0, 0.9, 1.1, 0.8, 1.2, 1.0];
        let alpha = [0.95, 1.05];
        let coefficient_a = [0.15, 0.1];
        let coefficient_b = [0.2, -0.05];
        let first_coefficients = [
            Complex64::new(0.25, -0.125),
            Complex64::new(0.5, 0.375),
            Complex64::new(-0.75, 0.25),
            Complex64::new(0.125, 0.625),
        ];
        let second_coefficients = [
            Complex64::new(-0.125, 0.5),
            Complex64::new(0.75, -0.25),
            Complex64::new(0.375, 0.125),
            Complex64::new(-0.5, 0.25),
        ];
        let mut first_even = [Complex64::new(0.0, 0.0); 2];
        let mut first_odd = [Complex64::new(0.0, 0.0); 2];
        let mut second_even = [Complex64::new(0.0, 0.0); 2];
        let mut second_odd = [Complex64::new(0.0, 0.0); 2];
        assert!(prepare_synthesis_coefficient_pair(
            m,
            lmax,
            &epsilon,
            &alpha,
            &first_coefficients,
            &second_coefficients,
            &mut first_even,
            &mut first_odd,
            &mut second_even,
            &mut second_odd,
        ));

        let mut original_first_sum_re = [0.0; 3];
        let mut original_first_sum_im = [0.0; 3];
        let mut original_second_sum_re = [0.0; 3];
        let mut original_second_sum_im = [0.0; 3];
        let mut original_previous = [0.0; 3];
        let mut original_current = [0.0; 3];
        let mut original_next = [0.0; 3];
        let mut original_exponents = [0; 3];
        let original_completed = try_synthesize_order_pair_transition_dispatched(
            Arch::new(),
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
            &mut original_previous,
            &mut original_current,
            &mut original_next,
            &mut original_exponents,
            &mut original_first_sum_re,
            &mut original_first_sum_im,
            &mut original_second_sum_re,
            &mut original_second_sum_im,
        )
        .unwrap();

        let mut prepared_first_sum_re = [0.0; 3];
        let mut prepared_first_sum_im = [0.0; 3];
        let mut prepared_second_sum_re = [0.0; 3];
        let mut prepared_second_sum_im = [0.0; 3];
        let mut prepared_previous = [0.0; 3];
        let mut prepared_current = [0.0; 3];
        let mut prepared_next = [0.0; 3];
        let mut prepared_exponents = [0; 3];
        let prepared_completed = try_synthesize_prepared_order_pair_transition_dispatched(
            Arch::new(),
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            &first_even[..pair_count],
            &first_odd[..pair_count],
            &second_even[..pair_count],
            &second_odd[..pair_count],
            &mut prepared_previous,
            &mut prepared_current,
            &mut prepared_next,
            &mut prepared_exponents,
            &mut prepared_first_sum_re,
            &mut prepared_first_sum_im,
            &mut prepared_second_sum_re,
            &mut prepared_second_sum_im,
        )
        .unwrap();

        assert!(original_completed);
        assert_eq!(prepared_completed, original_completed);
        assert_eq!(prepared_first_sum_re, original_first_sum_re);
        assert_eq!(prepared_first_sum_im, original_first_sum_im);
        assert_eq!(prepared_second_sum_re, original_second_sum_re);
        assert_eq!(prepared_second_sum_im, original_second_sum_im);
    }

    #[test]
    fn prepared_order_pair_transition_promotes_scaled_simd_vector() {
        let m = 100;
        let lmax = 106;
        let pair_count = (lmax - m + 2) / 2;
        let lanes = 9;
        let sin_theta = [0.01; 9];
        let cos_theta = sin_theta.map(|sine| (1.0 - sine * sine).sqrt());
        let sectoral_norm = 0.75;
        let epsilon = vec![1.0; lmax + 3];
        let alpha = vec![1.0; pair_count];
        let coefficient_a = vec![0.0; pair_count];
        let coefficient_b = vec![2.0_f64.powi(300); pair_count];
        let large_coefficient = 2.0_f64.powi(264);
        let first_coefficients = [Complex64::new(large_coefficient, large_coefficient); 7];
        let second_coefficients = [Complex64::new(-large_coefficient, large_coefficient * 0.5); 7];
        let mut first_even = vec![Complex64::new(0.0, 0.0); pair_count];
        let mut first_odd = vec![Complex64::new(0.0, 0.0); pair_count];
        let mut second_even = vec![Complex64::new(0.0, 0.0); pair_count];
        let mut second_odd = vec![Complex64::new(0.0, 0.0); pair_count];
        assert!(prepare_synthesis_coefficient_pair(
            m,
            lmax,
            &epsilon,
            &alpha,
            &first_coefficients,
            &second_coefficients,
            &mut first_even,
            &mut first_odd,
            &mut second_even,
            &mut second_odd,
        ));

        let mut original_first_sum_re = [0.0; 9];
        let mut original_first_sum_im = [0.0; 9];
        let mut original_second_sum_re = [0.0; 9];
        let mut original_second_sum_im = [0.0; 9];
        let mut original_previous = [0.0; 9];
        let mut original_current = [0.0; 9];
        let mut original_next = [0.0; 9];
        let mut original_exponents = [0; 9];
        let original_completed = try_synthesize_order_pair_transition_dispatched(
            Arch::new(),
            m,
            lmax,
            &sin_theta[..lanes],
            &cos_theta[..lanes],
            sectoral_norm,
            &epsilon,
            &alpha,
            &coefficient_a,
            &coefficient_b,
            &first_coefficients,
            &second_coefficients,
            &mut original_previous,
            &mut original_current,
            &mut original_next,
            &mut original_exponents,
            &mut original_first_sum_re,
            &mut original_first_sum_im,
            &mut original_second_sum_re,
            &mut original_second_sum_im,
        )
        .unwrap();

        let mut prepared_first_sum_re = [0.0; 9];
        let mut prepared_first_sum_im = [0.0; 9];
        let mut prepared_second_sum_re = [0.0; 9];
        let mut prepared_second_sum_im = [0.0; 9];
        let mut prepared_previous = [0.0; 9];
        let mut prepared_current = [0.0; 9];
        let mut prepared_next = [0.0; 9];
        let mut prepared_exponents = [0; 9];
        let prepared_completed = try_synthesize_prepared_order_pair_transition_dispatched(
            Arch::new(),
            m,
            lmax,
            &sin_theta[..lanes],
            &cos_theta[..lanes],
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            &first_even,
            &first_odd,
            &second_even,
            &second_odd,
            &mut prepared_previous,
            &mut prepared_current,
            &mut prepared_next,
            &mut prepared_exponents,
            &mut prepared_first_sum_re,
            &mut prepared_first_sum_im,
            &mut prepared_second_sum_re,
            &mut prepared_second_sum_im,
        )
        .unwrap();

        assert!(
            original_completed,
            "fixture should reach the ordinary suffix"
        );
        assert_eq!(prepared_completed, original_completed);
        assert_eq!(prepared_exponents, original_exponents);
        assert_eq!(prepared_first_sum_re, original_first_sum_re);
        assert_eq!(prepared_first_sum_im, original_first_sum_im);
        assert_eq!(prepared_second_sum_re, original_second_sum_re);
        assert_eq!(prepared_second_sum_im, original_second_sum_im);
    }
}
