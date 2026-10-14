// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * This module adapts DUCC's transformed paired-degree scalar recurrence from
 * src/ducc0/sht/sht_inner_loop.h, revision
 * c1ed01b8fda69d2a378190f0915f1a19762dddbf.
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

use crate::error::ShtError;
use crate::legendre::{LATITUDE_BLOCK_SIZE, scalar};

/// Prepares DUCC's per-order coefficient transformation and paired recurrence.
pub(crate) fn fill_factors(
    m: usize,
    lmax: usize,
    epsilon: &[f64],
    alpha: &mut [f64],
    coefficient_a: &mut [f64],
    coefficient_b: &mut [f64],
) {
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
) {
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
        coefficient_even[pair] = (coefficients[offset] * epsilon[degree + 1]
            + next_even * epsilon[degree + 2])
            * alpha[pair];
        coefficient_odd[pair] = if degree < lmax {
            coefficients[offset + 1] * alpha[pair]
        } else {
            Complex64::new(0.0, 0.0)
        };
    }
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
    prepare_synthesis_coefficients(
        m,
        lmax,
        epsilon,
        alpha,
        coefficients,
        &mut coefficient_even,
        &mut coefficient_odd,
    );
    synthesize_order_block_precomputed_dispatched(
        arch,
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
        spectra[ring * spectrum_len + m] += Complex64::new(previous[ring], next[ring]);
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
        epsilon,
        alpha,
        coefficient_a,
        coefficient_b,
        first_coefficients,
        second_coefficients,
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
        let spectrum_index = ring * spectrum_len + m;
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

    let completed = arch.dispatch(StagedSynthesisPairTransition {
        m,
        lmax,
        sin_theta,
        cos_theta,
        sectoral_norm,
        coefficient_a,
        coefficient_b,
        epsilon,
        alpha,
        first_coefficients,
        second_coefficients,
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

struct StagedSynthesisPairTransition<'a> {
    m: usize,
    lmax: usize,
    sin_theta: &'a [f64],
    cos_theta: &'a [f64],
    sectoral_norm: f64,
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    epsilon: &'a [f64],
    alpha: &'a [f64],
    first_coefficients: &'a [Complex64],
    second_coefficients: &'a [Complex64],
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
    exponents: &'a mut [i64],
    first_sum_re: &'a mut [f64],
    first_sum_im: &'a mut [f64],
    second_sum_re: &'a mut [f64],
    second_sum_im: &'a mut [f64],
}

impl WithSimd for StagedSynthesisPairTransition<'_> {
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
            epsilon,
            alpha,
            first_coefficients,
            second_coefficients,
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
        let mut normal_range = match scalar::normalize_pair_lanes(previous, current, exponents) {
            Ok(normal_range) => normal_range,
            Err(_) => return false,
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
                epsilon,
                alpha,
                coefficient_a,
                coefficient_b,
                first_coefficients,
                second_coefficients,
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
            for lane in 0..current.len() {
                let cosine = cos_theta[lane];
                let first_odd_factor = Complex64::new(first_odd.re * cosine, first_odd.im * cosine);
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
                    || !complex_sum_is_normal_or_zero(second_factor, second_even, second_odd_factor)
                {
                    return false;
                }
                let first =
                    scalar::multiply_complex_scaled(first_factor, current[lane], exponents[lane]);
                let second =
                    scalar::multiply_complex_scaled(second_factor, current[lane], exponents[lane]);
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
                    normal_range = match scalar::normalize_pair_lanes(previous, current, exponents)
                    {
                        Ok(normal_range) => normal_range,
                        Err(_) => return false,
                    };
                }
            } else {
                normal_range = match scalar::normalize_pair_lanes(previous, current, exponents) {
                    Ok(normal_range) => normal_range,
                    Err(_) => return false,
                };
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
                    epsilon,
                    alpha,
                    coefficient_a,
                    coefficient_b,
                    first_coefficients,
                    second_coefficients,
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
/// Lanes carrying binary scale exponents retain the scalar scaled path.
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
            return arch.dispatch(AnalysisPairAccumulation {
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                basis,
                cos_theta,
            });
        }
        if exponents.contains(&0) {
            return arch.dispatch(MixedAnalysisPairAccumulation {
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                basis,
                exponents,
                cos_theta,
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

/// Accumulates both spin-1 scalar-conversion channels in one SIMD pass when
/// their recurrence and products are in the normal range.
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

/// Checks once per latitude block whether Fourier values can multiply the
/// largest unscaled recurrence lane and reduce across the block safely.
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
    // vector reduction and the accumulation of vector partials finite.
    let max_input = f64::MAX / scalar::BIG / (2.0 * same_real.len().max(1) as f64);
    same_real
        .iter()
        .chain(same_imag)
        .chain(opposite_real)
        .chain(opposite_imag)
        .all(|value| value.is_finite() && value.abs() <= max_input)
        && cos_theta
            .iter()
            .all(|value| value.is_finite() && value.abs() <= 1.0)
}

struct AnalysisPairAccumulation<'a> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis: &'a [f64],
    cos_theta: &'a [f64],
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
            sums_0[0] = simd.add_f64s(sums_0[0], simd.mul_f64s(*same_real, basis));
            sums_0[1] = simd.add_f64s(sums_0[1], simd.mul_f64s(*same_imag, basis));
            sums_0[2] = simd.add_f64s(
                sums_0[2],
                simd.mul_f64s(simd.mul_f64s(*opposite_real, basis), cosine),
            );
            sums_0[3] = simd.add_f64s(
                sums_0[3],
                simd.mul_f64s(simd.mul_f64s(*opposite_imag, basis), cosine),
            );
            sums_1[0] = simd.add_f64s(sums_1[0], simd.mul_f64s(*same_real_1, basis));
            sums_1[1] = simd.add_f64s(sums_1[1], simd.mul_f64s(*same_imag_1, basis));
            sums_1[2] = simd.add_f64s(
                sums_1[2],
                simd.mul_f64s(simd.mul_f64s(*opposite_real_1, basis), cosine),
            );
            sums_1[3] = simd.add_f64s(
                sums_1[3],
                simd.mul_f64s(simd.mul_f64s(*opposite_imag_1, basis), cosine),
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
                results[channel].0.re += same_real[lane] * basis_tail[lane];
                results[channel].0.im += same_imag[lane] * basis_tail[lane];
                results[channel].1.re +=
                    (opposite_real[lane] * basis_tail[lane]) * cosine_tail[lane];
                results[channel].1.im +=
                    (opposite_imag[lane] * basis_tail[lane]) * cosine_tail[lane];
            }
        }
        results
    }
}

impl WithSimd for AnalysisPairAccumulation<'_> {
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
            same_re_even = simd.add_f64s(same_re_even, simd.mul_f64s(same_real[0], basis[0]));
            same_im_even = simd.add_f64s(same_im_even, simd.mul_f64s(same_imag[0], basis[0]));
            opposite_re_even = simd.add_f64s(
                opposite_re_even,
                simd.mul_f64s(simd.mul_f64s(opposite_real[0], basis[0]), cosine[0]),
            );
            opposite_im_even = simd.add_f64s(
                opposite_im_even,
                simd.mul_f64s(simd.mul_f64s(opposite_imag[0], basis[0]), cosine[0]),
            );
            same_re_odd = simd.add_f64s(same_re_odd, simd.mul_f64s(same_real[1], basis[1]));
            same_im_odd = simd.add_f64s(same_im_odd, simd.mul_f64s(same_imag[1], basis[1]));
            opposite_re_odd = simd.add_f64s(
                opposite_re_odd,
                simd.mul_f64s(simd.mul_f64s(opposite_real[1], basis[1]), cosine[1]),
            );
            opposite_im_odd = simd.add_f64s(
                opposite_im_odd,
                simd.mul_f64s(simd.mul_f64s(opposite_imag[1], basis[1]), cosine[1]),
            );
        }
        if !basis_remainder.is_empty() {
            let basis = basis_remainder[0];
            let cosine = cosine_remainder[0];
            same_re_even =
                simd.add_f64s(same_re_even, simd.mul_f64s(same_real_remainder[0], basis));
            same_im_even =
                simd.add_f64s(same_im_even, simd.mul_f64s(same_imag_remainder[0], basis));
            opposite_re_even = simd.add_f64s(
                opposite_re_even,
                simd.mul_f64s(simd.mul_f64s(opposite_real_remainder[0], basis), cosine),
            );
            opposite_im_even = simd.add_f64s(
                opposite_im_even,
                simd.mul_f64s(simd.mul_f64s(opposite_imag_remainder[0], basis), cosine),
            );
        }
        let mut same_re = simd.reduce_sum_f64s(simd.add_f64s(same_re_even, same_re_odd));
        let mut same_im = simd.reduce_sum_f64s(simd.add_f64s(same_im_even, same_im_odd));
        let mut opposite_re =
            simd.reduce_sum_f64s(simd.add_f64s(opposite_re_even, opposite_re_odd));
        let mut opposite_im =
            simd.reduce_sum_f64s(simd.add_f64s(opposite_im_even, opposite_im_odd));
        for lane in 0..basis_tail.len() {
            same_re += same_real_tail[lane] * basis_tail[lane];
            same_im += same_imag_tail[lane] * basis_tail[lane];
            opposite_re += (opposite_real_tail[lane] * basis_tail[lane]) * cosine_tail[lane];
            opposite_im += (opposite_imag_tail[lane] * basis_tail[lane]) * cosine_tail[lane];
        }
        (
            Complex64::new(same_re, same_im),
            Complex64::new(opposite_re, opposite_im),
        )
    }
}

struct MixedAnalysisPairAccumulation<'a> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis: &'a [f64],
    exponents: &'a [i64],
    cos_theta: &'a [f64],
}

impl WithSimd for MixedAnalysisPairAccumulation<'_> {
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
        for index in 0..basis_vectors.len() {
            let exponent_is_zero = simd.equal_i64s(exponent_vectors[index], zero_exponent);
            let basis_magnitude = simd.abs_f64s(basis_vectors[index]);
            let basis_is_zero = simd.equal_f64s(basis_magnitude, zeros);
            let basis_is_large_enough = simd.greater_than_or_equal_f64s(basis_magnitude, small);
            let basis_is_small_enough = simd.less_than_or_equal_f64s(basis_magnitude, big);
            let basis_is_normal_or_zero = simd.and_m64s(
                basis_is_small_enough,
                simd.or_m64s(basis_is_zero, basis_is_large_enough),
            );
            let normal_lanes = simd.and_m64s(exponent_is_zero, basis_is_normal_or_zero);
            let basis = simd.select_f64s(normal_lanes, basis_vectors[index], zeros);
            let same_real = simd.select_f64s(normal_lanes, same_real_vectors[index], zeros);
            let same_imag = simd.select_f64s(normal_lanes, same_imag_vectors[index], zeros);
            let opposite_real = simd.select_f64s(normal_lanes, opposite_real_vectors[index], zeros);
            let opposite_imag = simd.select_f64s(normal_lanes, opposite_imag_vectors[index], zeros);
            same_re = simd.add_f64s(same_re, simd.mul_f64s(same_real, basis));
            same_im = simd.add_f64s(same_im, simd.mul_f64s(same_imag, basis));
            opposite_re = simd.add_f64s(
                opposite_re,
                simd.mul_f64s(simd.mul_f64s(opposite_real, basis), cosine_vectors[index]),
            );
            opposite_im = simd.add_f64s(
                opposite_im,
                simd.mul_f64s(simd.mul_f64s(opposite_imag, basis), cosine_vectors[index]),
            );
        }

        let mut same = Complex64::new(simd.reduce_sum_f64s(same_re), simd.reduce_sum_f64s(same_im));
        let mut opposite = Complex64::new(
            simd.reduce_sum_f64s(opposite_re),
            simd.reduce_sum_f64s(opposite_im),
        );
        let vector_lane_count = basis.len() - basis_tail.len();
        for lane in 0..vector_lane_count {
            if exponents[lane] != 0 || !scalar_value_in_normal_range(basis[lane]) {
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
        }
        for lane in 0..basis_tail.len() {
            if exponent_tail[lane] == 0 && scalar_value_in_normal_range(basis_tail[lane]) {
                same.re += same_real_tail[lane] * basis_tail[lane];
                same.im += same_imag_tail[lane] * basis_tail[lane];
                opposite.re += (opposite_real_tail[lane] * basis_tail[lane]) * cosine_tail[lane];
                opposite.im += (opposite_imag_tail[lane] * basis_tail[lane]) * cosine_tail[lane];
            } else {
                same += scalar::multiply_complex_scaled(
                    Complex64::new(same_real_tail[lane], same_imag_tail[lane]),
                    basis_tail[lane],
                    exponent_tail[lane],
                );
                opposite += scalar::multiply_complex_scaled_product(
                    Complex64::new(opposite_real_tail[lane], opposite_imag_tail[lane]),
                    basis_tail[lane],
                    cosine_tail[lane],
                    exponent_tail[lane],
                );
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

#[inline(always)]
fn scalar_value_in_normal_range(value: f64) -> bool {
    value == 0.0
        || (value.is_finite() && value.abs() <= scalar::BIG && value.abs() >= scalar::SMALL)
}

#[inline(always)]
fn simd_values_in_normal_range<S: Simd>(simd: S, values: S::f64s) -> bool {
    let absolute = simd.abs_f64s(values);
    let upper_mask = simd.less_than_or_equal_f64s(absolute, simd.splat_f64s(scalar::BIG));
    let upper_ok = simd.select_f64s(upper_mask, simd.splat_f64s(1.0), simd.splat_f64s(0.0));
    if simd.reduce_min_f64s(upper_ok) != 1.0 {
        return false;
    }
    let zero_mask = simd.equal_f64s(absolute, simd.splat_f64s(0.0));
    let lower_values = simd.select_f64s(zero_mask, simd.splat_f64s(scalar::BIG), absolute);
    simd.reduce_min_f64s(lower_values) >= scalar::SMALL
}

fn lane_values_in_normal_range(values: &[f64]) -> bool {
    values
        .iter()
        .all(|&value| scalar_value_in_normal_range(value))
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
    sums: [&'a mut [f64]; 8],
}

#[cfg(feature = "parallel")]
impl WithSimd for FusedHemispherePairBlock<'_> {
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> bool {
        let (cos_vectors, cos_tail) = S::as_simd_f64s(self.cos_theta);
        let (current_vectors, current_tail) = S::as_simd_f64s(self.current);
        let [sum0, sum1, sum2, sum3, sum4, sum5, sum6, sum7] = self.sums;
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
        for lane_block in 0..cos_vectors.len() {
            let cosine = cos_vectors[lane_block];
            let mut previous = simd.splat_f64s(0.0);
            let mut basis = current_vectors[lane_block];
            let mut sums = [simd.splat_f64s(0.0); 8];
            for pair in 0..self.pair_count {
                let first_even = self.first_even[pair];
                let first_odd = self.first_odd[pair];
                let second_even = self.second_even[pair];
                let second_odd = self.second_odd[pair];
                sums[0] = simd.mul_add_f64s(basis, simd.splat_f64s(first_even.re), sums[0]);
                sums[1] = simd.mul_add_f64s(basis, simd.splat_f64s(first_even.im), sums[1]);
                sums[2] = simd.mul_add_f64s(
                    basis,
                    simd.mul_f64s(cosine, simd.splat_f64s(first_odd.re)),
                    sums[2],
                );
                sums[3] = simd.mul_add_f64s(
                    basis,
                    simd.mul_f64s(cosine, simd.splat_f64s(first_odd.im)),
                    sums[3],
                );
                sums[4] = simd.mul_add_f64s(basis, simd.splat_f64s(second_even.re), sums[4]);
                sums[5] = simd.mul_add_f64s(basis, simd.splat_f64s(second_even.im), sums[5]);
                sums[6] = simd.mul_add_f64s(
                    basis,
                    simd.mul_f64s(cosine, simd.splat_f64s(second_odd.re)),
                    sums[6],
                );
                sums[7] = simd.mul_add_f64s(
                    basis,
                    simd.mul_f64s(cosine, simd.splat_f64s(second_odd.im)),
                    sums[7],
                );
                if pair + 1 < self.pair_count {
                    let cosine_squared = simd.mul_f64s(cosine, cosine);
                    let leading = simd.mul_add_f64s(
                        simd.splat_f64s(self.coefficient_a[pair]),
                        cosine_squared,
                        simd.splat_f64s(self.coefficient_b[pair]),
                    );
                    let next = simd.mul_add_f64s(leading, basis, previous);
                    if !simd_values_in_normal_range(simd, next) {
                        return false;
                    }
                    previous = basis;
                    basis = next;
                }
            }
            for sum in 0..8 {
                vector_sums[sum][lane_block] = sums[sum];
            }
        }
        for lane in 0..cos_tail.len() {
            let cosine = cos_tail[lane];
            let mut previous = 0.0;
            let mut basis = current_tail[lane];
            let mut sums = [0.0; 8];
            for pair in 0..self.pair_count {
                let first_even = self.first_even[pair];
                let first_odd = self.first_odd[pair];
                let second_even = self.second_even[pair];
                let second_odd = self.second_odd[pair];
                sums[0] = basis.mul_add(first_even.re, sums[0]);
                sums[1] = basis.mul_add(first_even.im, sums[1]);
                sums[2] = basis.mul_add(cosine * first_odd.re, sums[2]);
                sums[3] = basis.mul_add(cosine * first_odd.im, sums[3]);
                sums[4] = basis.mul_add(second_even.re, sums[4]);
                sums[5] = basis.mul_add(second_even.im, sums[5]);
                sums[6] = basis.mul_add(cosine * second_odd.re, sums[6]);
                sums[7] = basis.mul_add(cosine * second_odd.im, sums[7]);
                if pair + 1 < self.pair_count {
                    let leading =
                        self.coefficient_a[pair].mul_add(cosine * cosine, self.coefficient_b[pair]);
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

struct FusedSynthesisPairBlock<'a> {
    m: usize,
    lmax: usize,
    pair_start: usize,
    cos_theta: &'a [f64],
    epsilon: &'a [f64],
    alpha: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    first_coefficients: &'a [Complex64],
    second_coefficients: &'a [Complex64],
    previous: &'a [f64],
    current: &'a [f64],
    first_sum_re: &'a mut [f64],
    first_sum_im: &'a mut [f64],
    second_sum_re: &'a mut [f64],
    second_sum_im: &'a mut [f64],
    initialize_sums: bool,
    check_normal_range: bool,
}

impl WithSimd for FusedSynthesisPairBlock<'_> {
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> bool {
        let FusedSynthesisPairBlock {
            m,
            lmax,
            pair_start,
            cos_theta,
            epsilon,
            alpha,
            coefficient_a,
            coefficient_b,
            first_coefficients,
            second_coefficients,
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
                    if check_normal_range
                        && (!simd_values_in_normal_range(simd, previous)
                            || !simd_values_in_normal_range(simd, current))
                    {
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
                    if check_normal_range
                        && (!scalar_value_in_normal_range(previous)
                            || !scalar_value_in_normal_range(current))
                    {
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

                prepare_synthesis_coefficients(
                    m,
                    lmax,
                    &epsilon,
                    &alpha,
                    &coefficients,
                    &mut coefficient_even,
                    &mut coefficient_odd,
                );

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
        prepare_synthesis_coefficients(
            m,
            lmax,
            &epsilon,
            &alpha,
            &coefficients,
            &mut coefficient_even,
            &mut coefficient_odd,
        );

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
            epsilon: &epsilon,
            alpha: &alpha,
            coefficient_a: &coefficient_a,
            coefficient_b: &coefficient_b,
            first_coefficients: &first_coefficients,
            second_coefficients: &second_coefficients,
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
}
