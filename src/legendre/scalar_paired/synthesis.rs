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

use super::*;

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
pub(super) struct OriginalSynthesisPairCoefficients<'a> {
    pub(super) epsilon: &'a [f64],
    pub(super) alpha: &'a [f64],
    pub(super) first_coefficients: &'a [Complex64],
    pub(super) second_coefficients: &'a [Complex64],
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

pub(super) struct FusedSynthesisPairBlock<'a, C> {
    pub(super) m: usize,
    pub(super) lmax: usize,
    pub(super) pair_start: usize,
    pub(super) cos_theta: &'a [f64],
    pub(super) coefficient_a: &'a [f64],
    pub(super) coefficient_b: &'a [f64],
    pub(super) coefficients: C,
    pub(super) previous: &'a [f64],
    pub(super) current: &'a [f64],
    pub(super) first_sum_re: &'a mut [f64],
    pub(super) first_sum_im: &'a mut [f64],
    pub(super) second_sum_re: &'a mut [f64],
    pub(super) second_sum_im: &'a mut [f64],
    pub(super) initialize_sums: bool,
    pub(super) check_normal_range: bool,
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
