// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * Native Rust translation of DUCC's scalar GL synthesis execution in
 * src/ducc0/sht/sht_inner_loop.h, revision
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

use crate::legendre::{SCALAR_DUCC_BATCH_SIZE, longitude_phase, scalar, scalar_paired};

const SCALE_UP: f64 = f64::from_bits(1823_u64 << 52);
const SCALE_DOWN: f64 = f64::from_bits(223_u64 << 52);
const HALF_SCALE: f64 = f64::from_bits(1423_u64 << 52);
const FTOL: f64 = f64::from_bits((1023_u64 - 60) << 52);

pub(crate) struct ScalarDuccScratch<'a, T> {
    pub(crate) epsilon: &'a mut [T],
    pub(crate) alpha: &'a mut [T],
    pub(crate) coefficient_a: &'a mut [T],
    pub(crate) coefficient_b: &'a mut [T],
    pub(crate) coefficient_even: &'a mut [num_complex::Complex<T>],
    pub(crate) coefficient_odd: &'a mut [num_complex::Complex<T>],
    pub(crate) packed_sin: &'a mut [T],
    pub(crate) packed_cos: &'a mut [T],
    pub(crate) packed_csq: &'a mut [T],
    pub(crate) packed_north: &'a mut [usize],
    pub(crate) previous: &'a mut [T],
    pub(crate) current: &'a mut [T],
    pub(crate) scale: &'a mut [i64],
    pub(crate) corfac: &'a mut [T],
    pub(crate) sum_even_re: &'a mut [T],
    pub(crate) sum_even_im: &'a mut [T],
    pub(crate) sum_odd_re: &'a mut [T],
    pub(crate) sum_odd_im: &'a mut [T],
}

/// Runs DUCC's order-major, reflected-ring scalar synthesis for GL geometry.
///
/// A failed preflight or unsafe numerical route clears partial Fourier output
/// before returning `false`; the caller can then run the established scaled
/// RustSHT implementation without double accumulation.
#[expect(
    clippy::too_many_arguments,
    reason = "The private dispatch receives validated plan data and reusable workspace scratch."
)]
pub(crate) fn synthesize_dispatched<'a>(
    arch: Arch,
    alm: &'a [Complex64],
    lmax: usize,
    mmax: usize,
    nphi: usize,
    spectrum_len: usize,
    phi0: f64,
    sectoral_norms: &'a [f64],
    sin_theta: &'a [f64],
    cos_theta: &'a [f64],
    ring_order_limits: &'a [usize],
    spectra: &'a mut [Complex64],
    scratch: &'a mut ScalarDuccScratch<'_, f64>,
) -> bool {
    let Some(order_end) = mmax.checked_add(1) else {
        return false;
    };
    synthesize_range_dispatched(
        arch,
        alm,
        lmax,
        mmax,
        0,
        order_end,
        nphi,
        spectrum_len,
        phi0,
        sectoral_norms,
        sin_theta,
        cos_theta,
        ring_order_limits,
        spectra,
        false,
        scratch,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The private order-group dispatch receives validated plan data and worker scratch."
)]
#[cfg(feature = "parallel")]
pub(crate) fn synthesize_order_range_dispatched<'a>(
    arch: Arch,
    alm: &'a [Complex64],
    lmax: usize,
    mmax: usize,
    order_start: usize,
    order_end: usize,
    nphi: usize,
    spectrum_len: usize,
    phi0: f64,
    sectoral_norms: &'a [f64],
    sin_theta: &'a [f64],
    cos_theta: &'a [f64],
    ring_order_limits: &'a [usize],
    spectra: &'a mut [Complex64],
    scratch: &'a mut ScalarDuccScratch<'_, f64>,
) -> bool {
    synthesize_range_dispatched(
        arch,
        alm,
        lmax,
        mmax,
        order_start,
        order_end,
        nphi,
        spectrum_len,
        phi0,
        sectoral_norms,
        sin_theta,
        cos_theta,
        ring_order_limits,
        spectra,
        true,
        scratch,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Both scalar DUCC layouts share one validated recurrence dispatch."
)]
fn synthesize_range_dispatched<'a>(
    arch: Arch,
    alm: &'a [Complex64],
    lmax: usize,
    mmax: usize,
    order_start: usize,
    order_end: usize,
    nphi: usize,
    spectrum_len: usize,
    phi0: f64,
    sectoral_norms: &'a [f64],
    sin_theta: &'a [f64],
    cos_theta: &'a [f64],
    ring_order_limits: &'a [usize],
    spectra: &'a mut [Complex64],
    order_major_output: bool,
    scratch: &'a mut ScalarDuccScratch<'_, f64>,
) -> bool {
    let ntheta = sin_theta.len();
    let Some(order_count) = mmax.checked_add(1) else {
        return false;
    };
    let Some(expected_spectrum_count) = ntheta.checked_mul(if order_major_output {
        order_end.saturating_sub(order_start)
    } else {
        spectrum_len
    }) else {
        return false;
    };
    if mmax > lmax
        || order_start >= order_end
        || order_end > order_count
        || ntheta == 0
        || cos_theta.len() != ntheta
        || ring_order_limits.len() != ntheta
        || spectra.len() != expected_spectrum_count
        || sectoral_norms.len() <= mmax
        || spectrum_len == 0
        || mmax >= spectrum_len
        || nphi == 0
        || scratch.packed_sin.len() < SCALAR_DUCC_BATCH_SIZE
        || scratch.packed_cos.len() < SCALAR_DUCC_BATCH_SIZE
        || scratch.packed_csq.len() < SCALAR_DUCC_BATCH_SIZE
        || scratch.packed_north.len() < SCALAR_DUCC_BATCH_SIZE
        || scratch.previous.len() < SCALAR_DUCC_BATCH_SIZE
        || scratch.current.len() < SCALAR_DUCC_BATCH_SIZE
        || scratch.scale.len() < SCALAR_DUCC_BATCH_SIZE
        || scratch.corfac.len() < SCALAR_DUCC_BATCH_SIZE
        || scratch.sum_even_re.len() < SCALAR_DUCC_BATCH_SIZE
        || scratch.sum_even_im.len() < SCALAR_DUCC_BATCH_SIZE
        || scratch.sum_odd_re.len() < SCALAR_DUCC_BATCH_SIZE
        || scratch.sum_odd_im.len() < SCALAR_DUCC_BATCH_SIZE
    {
        return false;
    }

    // The candidate pairs reflected GL rings. Reject a geometry that does not
    // satisfy that contract and leave it to the general latitude kernel.
    for north in 0..ntheta.div_ceil(2) {
        let south = ntheta - 1 - north;
        if (sin_theta[north] - sin_theta[south]).abs() > 5.0e-15
            || (cos_theta[north] + cos_theta[south]).abs() > 5.0e-15
            || (north > 0 && ring_order_limits[north] < ring_order_limits[north - 1])
        {
            return false;
        }
    }

    if !phi0.is_finite() {
        return false;
    }
    spectra.fill(Complex64::new(0.0, 0.0));
    let succeeded = arch.dispatch(ScalarDuccSynthesis {
        alm,
        lmax,
        order_start,
        order_end,
        nphi,
        spectrum_len,
        order_major_output,
        phi0,
        sectoral_norms,
        sin_theta,
        cos_theta,
        ring_order_limits,
        spectra,
        scratch,
    });
    if !succeeded {
        spectra.fill(Complex64::new(0.0, 0.0));
    }
    succeeded
}

struct ScalarDuccSynthesis<'a, 's> {
    alm: &'a [Complex64],
    lmax: usize,
    order_start: usize,
    order_end: usize,
    nphi: usize,
    spectrum_len: usize,
    order_major_output: bool,
    phi0: f64,
    sectoral_norms: &'a [f64],
    sin_theta: &'a [f64],
    cos_theta: &'a [f64],
    ring_order_limits: &'a [usize],
    spectra: &'a mut [Complex64],
    scratch: &'a mut ScalarDuccScratch<'s, f64>,
}

impl WithSimd for ScalarDuccSynthesis<'_, '_> {
    type Output = bool;

    #[inline]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self {
            alm,
            lmax,
            order_start,
            order_end,
            nphi,
            spectrum_len,
            order_major_output,
            phi0,
            sectoral_norms,
            sin_theta,
            cos_theta,
            ring_order_limits,
            spectra,
            scratch,
        } = self;
        let ntheta = sin_theta.len();
        let north_count = ntheta.div_ceil(2);
        let mut first_active_north_ring = 0;

        for (m, &sectoral_norm) in sectoral_norms
            .iter()
            .enumerate()
            .take(order_end)
            .skip(order_start)
        {
            // DUCC's scaled sectoral route is selected by a per-order lower
            // bound. Compute that bound once and reuse it for every ring
            // batch belonging to this order.
            let power_lower_limit = power_lower_limit(m);
            let Some(order_offset) = packed_order_offset(m, lmax) else {
                return false;
            };
            let Some(order_len) = lmax.checked_sub(m).and_then(|n| n.checked_add(1)) else {
                return false;
            };
            let Some(coefficient_end) = order_offset.checked_add(order_len) else {
                return false;
            };
            let Some(order_alm) = alm.get(order_offset..coefficient_end) else {
                return false;
            };
            if order_alm.iter().any(|coefficient| {
                !coefficient.re.is_finite()
                    || !coefficient.im.is_finite()
                    || is_subnormal(coefficient.re)
                    || is_subnormal(coefficient.im)
            }) {
                return false;
            }
            if !prepare_order(scratch, m, lmax, order_alm) {
                return false;
            }

            let phase = longitude_phase::<f64>(m, phi0, false);
            if !phase.re.is_finite() || !phase.im.is_finite() {
                return false;
            }
            // The polar order limit is monotone from pole to equator. Find
            // the first eligible north ring by advancing its boundary as the
            // order increases. Each excluded ring is examined once overall.
            while first_active_north_ring < north_count
                && ring_order_limits[first_active_north_ring] < m
            {
                first_active_north_ring += 1;
            }
            let mut ring_cursor = first_active_north_ring;
            while ring_cursor < north_count {
                let active_count = (north_count - ring_cursor).min(SCALAR_DUCC_BATCH_SIZE);
                for lane in 0..active_count {
                    let north = ring_cursor + lane;
                    let south = ntheta - 1 - north;
                    let sine = sin_theta[north];
                    let cosine = cos_theta[north];
                    scratch.packed_sin[lane] = sine;
                    scratch.packed_cos[lane] = cosine;
                    scratch.packed_csq[lane] = if cosine.abs() > 0.99 {
                        (1.0 - sine) * (1.0 + sine)
                    } else {
                        cosine * cosine
                    };
                    scratch.packed_north[lane] = north;
                    debug_assert!(m <= ring_order_limits[north]);
                    debug_assert_eq!(south, ntheta - 1 - scratch.packed_north[lane]);
                }

                if !synthesize_batch(
                    simd,
                    scratch,
                    m,
                    lmax,
                    sectoral_norm,
                    power_lower_limit,
                    active_count,
                ) {
                    return false;
                }
                for lane in 0..active_count {
                    let north = scratch.packed_north[lane];
                    let south = ntheta - 1 - north;
                    let even_re = scratch.sum_even_re[lane];
                    let even_im = scratch.sum_even_im[lane];
                    let odd_re = scratch.sum_odd_re[lane] * scratch.packed_cos[lane];
                    let odd_im = scratch.sum_odd_im[lane] * scratch.packed_cos[lane];
                    let (mut north_value, mut south_value) = (
                        Complex64::new(even_re + odd_re, even_im + odd_im),
                        Complex64::new(even_re - odd_re, even_im - odd_im),
                    );
                    north_value *= phase;
                    south_value *= phase;
                    if nphi.is_multiple_of(2) && m == nphi / 2 {
                        north_value = Complex64::new(2.0 * north_value.re, 0.0);
                        south_value = Complex64::new(2.0 * south_value.re, 0.0);
                    }
                    if !north_value.re.is_finite()
                        || !north_value.im.is_finite()
                        || !south_value.re.is_finite()
                        || !south_value.im.is_finite()
                    {
                        return false;
                    }
                    scratch_output(
                        spectra,
                        spectrum_len,
                        ntheta,
                        order_start,
                        order_major_output,
                        m,
                        north,
                        north_value,
                    );
                    if south != north {
                        scratch_output(
                            spectra,
                            spectrum_len,
                            ntheta,
                            order_start,
                            order_major_output,
                            m,
                            south,
                            south_value,
                        );
                    }
                }
                ring_cursor += active_count;
            }
        }
        true
    }
}

#[inline]
#[expect(
    clippy::too_many_arguments,
    reason = "The private output helper supports both Fourier spectrum layouts."
)]
fn scratch_output(
    spectra: &mut [Complex64],
    spectrum_len: usize,
    ntheta: usize,
    order_start: usize,
    order_major_output: bool,
    order: usize,
    ring: usize,
    value: Complex64,
) {
    let index = if order_major_output {
        (order - order_start) * ntheta + ring
    } else {
        ring * spectrum_len + order
    };
    spectra[index] = value;
}

fn packed_order_offset(m: usize, lmax: usize) -> Option<usize> {
    let row_start = m.checked_mul(lmax.checked_add(1)?)?;
    let preceding = m.checked_mul(m.saturating_sub(1))?.checked_div(2)?;
    row_start.checked_sub(preceding)
}

fn is_subnormal(value: f64) -> bool {
    value != 0.0 && value.abs() < f64::MIN_POSITIVE
}

fn prepare_order(
    scratch: &mut ScalarDuccScratch<'_, f64>,
    m: usize,
    lmax: usize,
    coefficients: &[Complex64],
) -> bool {
    let Some(required_epsilon_len) = lmax.checked_add(3) else {
        return false;
    };
    let Some(pair_count) = lmax
        .checked_sub(m)
        .and_then(|n| n.checked_add(2))
        .map(|n| n / 2)
    else {
        return false;
    };
    if scratch.epsilon.len() < required_epsilon_len
        || scratch.alpha.len() < pair_count
        || scratch.coefficient_a.len() < pair_count
        || scratch.coefficient_b.len() < pair_count
        || scratch.coefficient_even.len() < pair_count
        || scratch.coefficient_odd.len() < pair_count
    {
        return false;
    }
    scalar::fill_epsilon(m, lmax, scratch.epsilon);
    scalar_paired::fill_factors(
        m,
        lmax,
        scratch.epsilon,
        scratch.alpha,
        scratch.coefficient_a,
        scratch.coefficient_b,
    );
    if !scalar_paired::prepare_synthesis_coefficients(
        m,
        lmax,
        scratch.epsilon,
        scratch.alpha,
        coefficients,
        &mut scratch.coefficient_even[..pair_count],
        &mut scratch.coefficient_odd[..pair_count],
    ) {
        return false;
    }
    !scratch.coefficient_even[..pair_count]
        .iter()
        .chain(&scratch.coefficient_odd[..pair_count])
        .any(|coefficient| is_subnormal(coefficient.re) || is_subnormal(coefficient.im))
}

fn synthesize_batch(
    simd: impl Simd,
    scratch: &mut ScalarDuccScratch<'_, f64>,
    m: usize,
    lmax: usize,
    sectoral_norm: f64,
    power_lower_limit: f64,
    active_count: usize,
) -> bool {
    let Some(pair_count) = lmax
        .checked_sub(m)
        .and_then(|n| n.checked_add(2))
        .map(|n| n / 2)
    else {
        return false;
    };
    let sign = if m.is_multiple_of(2) { 1.0 } else { -1.0 };
    if !initialize_sectoral_batch(
        simd,
        &scratch.packed_sin[..active_count],
        &mut scratch.current[..active_count],
        &mut scratch.scale[..active_count],
        m,
        power_lower_limit,
        sign * sectoral_norm,
    ) {
        return false;
    }
    for lane in 0..active_count {
        scratch.previous[lane] = 0.0;
        scratch.sum_even_re[lane] = 0.0;
        scratch.sum_even_im[lane] = 0.0;
        scratch.sum_odd_re[lane] = 0.0;
        scratch.sum_odd_im[lane] = 0.0;
    }

    let mut previous = &mut scratch.previous[..active_count];
    let mut current = &mut scratch.current[..active_count];
    let Some((mut degree, mut pair, skip_remainder)) = iter_to_ieee(
        simd,
        m,
        lmax,
        &scratch.packed_csq[..active_count],
        scratch.coefficient_a,
        scratch.coefficient_b,
        &mut previous,
        &mut current,
        &mut scratch.scale[..active_count],
    ) else {
        return false;
    };
    if skip_remainder || degree > lmax {
        return true;
    }

    set_correction_factors(
        simd,
        &scratch.scale[..active_count],
        &mut scratch.corfac[..active_count],
    );
    let mut full_ieee = all_scale_nonnegative(simd, &scratch.scale[..active_count]);
    while !full_ieee && degree <= lmax && pair < pair_count {
        scaled_pair_step(
            simd,
            &scratch.corfac[..active_count],
            &scratch.packed_csq[..active_count],
            scratch.coefficient_a[pair],
            scratch.coefficient_b[pair],
            scratch.coefficient_even[pair],
            scratch.coefficient_odd[pair],
            previous,
            current,
            &mut scratch.sum_even_re[..active_count],
            &mut scratch.sum_even_im[..active_count],
            &mut scratch.sum_odd_re[..active_count],
            &mut scratch.sum_odd_im[..active_count],
        );
        degree += 2;
        pair += 1;
        let Some(rescaled) = rescale_batch(
            simd,
            &mut previous[..],
            &mut current[..],
            &mut scratch.scale[..active_count],
        ) else {
            return false;
        };
        if rescaled {
            set_correction_factors(
                simd,
                &scratch.scale[..active_count],
                &mut scratch.corfac[..active_count],
            );
        }
        full_ieee = all_scale_nonnegative(simd, &scratch.scale[..active_count]);
    }

    if degree <= lmax && pair < pair_count && full_ieee {
        apply_correction_and_clear_scale(
            simd,
            &mut previous[..],
            &mut current[..],
            &scratch.corfac[..active_count],
            &mut scratch.scale[..active_count],
        );
        while pair + 1 < pair_count {
            ordinary_two_pair_step(
                simd,
                &scratch.packed_csq[..active_count],
                scratch.coefficient_a[pair],
                scratch.coefficient_b[pair],
                scratch.coefficient_a[pair + 1],
                scratch.coefficient_b[pair + 1],
                scratch.coefficient_even[pair],
                scratch.coefficient_odd[pair],
                scratch.coefficient_even[pair + 1],
                scratch.coefficient_odd[pair + 1],
                previous,
                current,
                &mut scratch.sum_even_re[..active_count],
                &mut scratch.sum_even_im[..active_count],
                &mut scratch.sum_odd_re[..active_count],
                &mut scratch.sum_odd_im[..active_count],
            );
            pair += 2;
        }
        if pair < pair_count {
            ordinary_one_pair_step(
                simd,
                &scratch.packed_csq[..active_count],
                scratch.coefficient_a[pair],
                scratch.coefficient_b[pair],
                scratch.coefficient_even[pair],
                scratch.coefficient_odd[pair],
                previous,
                current,
                &mut scratch.sum_even_re[..active_count],
                &mut scratch.sum_even_im[..active_count],
                &mut scratch.sum_odd_re[..active_count],
                &mut scratch.sum_odd_im[..active_count],
            );
        }
    }

    for lane in 0..active_count {
        if !scratch.sum_even_re[lane].is_finite()
            || !scratch.sum_even_im[lane].is_finite()
            || !scratch.sum_odd_re[lane].is_finite()
            || !scratch.sum_odd_im[lane].is_finite()
            || !previous[lane].is_finite()
            || !current[lane].is_finite()
        {
            return false;
        }
    }
    true
}

fn power_lower_limit(exponent: usize) -> f64 {
    if exponent == 0 {
        0.0
    } else {
        (-400.0 * std::f64::consts::LN_2 / exponent as f64).exp()
    }
}

#[inline]
fn integer_power_f64s<S: Simd>(simd: S, mut base: S::f64s, mut exponent: usize) -> S::f64s {
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

/// Forms sectoral powers with SIMD for lanes whose powers stay in the
/// ordinary range. Lanes below DUCC's per-order threshold use the checked
/// scaled scalar path, which also protects subnormal and underflow cases.
fn initialize_sectoral_batch<S: Simd>(
    simd: S,
    sine: &[f64],
    values: &mut [f64],
    scales: &mut [i64],
    exponent: usize,
    lower_limit: f64,
    factor_value: f64,
) -> bool {
    if sine.len() != values.len() || sine.len() != scales.len() {
        return false;
    }
    if sine.iter().any(|value| !value.is_finite()) {
        return false;
    }

    let (sine_vectors, sine_tail) = S::as_simd_f64s(sine);
    let (value_vectors, value_tail) = S::as_mut_simd_f64s(values);
    let threshold = simd.splat_f64s(lower_limit);
    let one = simd.splat_f64s(1.0);
    let factor = simd.splat_f64s(factor_value);
    let mut has_scaled_lanes = false;
    for (sine_vector, value_vector) in sine_vectors.iter().zip(value_vectors.iter_mut()) {
        let normal_range = simd.greater_than_or_equal_f64s(simd.abs_f64s(*sine_vector), threshold);
        has_scaled_lanes |= mask_any(simd, simd.not_m64s(normal_range));
        let safe_base = simd.select_f64s(normal_range, *sine_vector, one);
        let power = integer_power_f64s(simd, safe_base, exponent);
        *value_vector = simd.select_f64s(
            normal_range,
            simd.mul_f64s(power, factor),
            simd.splat_f64s(0.0),
        );
    }
    for (sine_value, value) in sine_tail.iter().zip(value_tail.iter_mut()) {
        if sine_value.abs() >= lower_limit {
            *value = scalar_integer_power(*sine_value, exponent) * factor_value;
        } else {
            *value = 0.0;
            has_scaled_lanes = true;
        }
    }

    scales.fill(0);
    if has_scaled_lanes {
        for lane in 0..sine.len() {
            if sine[lane].abs() < lower_limit {
                let Some((mut value, scale)) = mypow(sine[lane], exponent, lower_limit) else {
                    return false;
                };
                value *= factor_value;
                values[lane] = value;
                scales[lane] = scale;
            }
        }
    }
    normalize_batch(simd, values, scales, FTOL)
}

fn scalar_integer_power(mut base: f64, mut exponent: usize) -> f64 {
    let mut result = 1.0;
    while exponent > 0 {
        if exponent & 1 != 0 {
            result *= base;
        }
        exponent >>= 1;
        if exponent > 0 {
            base *= base;
        }
    }
    result
}

fn mypow(value: f64, exponent: usize, lower_limit: f64) -> Option<(f64, i64)> {
    if exponent == 0 {
        return Some((1.0, 0));
    }
    if value.abs() >= lower_limit {
        let result = scalar_integer_power(value, exponent);
        return result.is_finite().then_some((result, 0));
    }

    let mut base = value;
    let mut base_scale = 0_i64;
    if !normalize_lane(&mut base, &mut base_scale, HALF_SCALE) {
        return None;
    }
    let mut result = 1.0;
    let mut result_scale = 0_i64;
    let mut power = exponent;
    while power > 0 {
        if power & 1 != 0 {
            result *= base;
            result_scale = result_scale.checked_add(base_scale)?;
            if !normalize_lane(&mut result, &mut result_scale, HALF_SCALE) {
                return None;
            }
        }
        power >>= 1;
        if power > 0 {
            base *= base;
            base_scale = base_scale.checked_mul(2)?;
            if !normalize_lane(&mut base, &mut base_scale, HALF_SCALE) {
                return None;
            }
        }
    }
    Some((result, result_scale))
}

fn normalize_lane(value: &mut f64, scale: &mut i64, maximum: f64) -> bool {
    if !value.is_finite() {
        return false;
    }
    while value.abs() > maximum {
        *value *= SCALE_DOWN;
        let Some(next_scale) = scale.checked_add(1) else {
            return false;
        };
        *scale = next_scale;
    }
    let minimum = maximum * SCALE_DOWN;
    if *value != 0.0 {
        while value.abs() < minimum {
            *value *= SCALE_UP;
            let Some(next_scale) = scale.checked_sub(1) else {
                return false;
            };
            *scale = next_scale;
        }
    }
    value.is_finite()
}

fn normalize_batch<S: Simd>(simd: S, values: &mut [f64], scales: &mut [i64], maximum: f64) -> bool {
    if values.len() != scales.len() {
        return false;
    }
    let (value_vectors, value_tail) = S::as_mut_simd_f64s(values);
    let (scale_vectors, scale_tail) = S::as_mut_simd_i64s(scales);
    let maximum_vector = simd.splat_f64s(maximum);
    let minimum_vector = simd.splat_f64s(maximum * SCALE_DOWN);
    let scale_down = simd.splat_f64s(SCALE_DOWN);
    let scale_up = simd.splat_f64s(SCALE_UP);
    let scale_one = simd.splat_i64s(1);
    let scale_minus_one = simd.splat_i64s(-1);
    let scale_max = simd.splat_i64s(i64::MAX);
    let scale_min = simd.splat_i64s(i64::MIN);
    let zero = simd.splat_f64s(0.0);

    for index in 0..value_vectors.len() {
        let mut magnitude = simd.abs_f64s(value_vectors[index]);
        if !mask_all(
            simd,
            simd.less_than_or_equal_f64s(magnitude, simd.splat_f64s(f64::MAX)),
        ) {
            return false;
        }
        loop {
            let too_large = simd.greater_than_f64s(magnitude, maximum_vector);
            if !mask_any(simd, too_large) {
                break;
            }
            let overflow =
                simd.and_m64s(too_large, simd.equal_i64s(scale_vectors[index], scale_max));
            if mask_any(simd, overflow) {
                return false;
            }
            value_vectors[index] = simd.select_f64s(
                too_large,
                simd.mul_f64s(value_vectors[index], scale_down),
                value_vectors[index],
            );
            scale_vectors[index] = simd.select_i64s(
                too_large,
                simd.add_i64s(scale_vectors[index], scale_one),
                scale_vectors[index],
            );
            magnitude = simd.abs_f64s(value_vectors[index]);
        }
        loop {
            let nonzero = simd.not_m64s(simd.equal_f64s(magnitude, zero));
            let too_small = simd.and_m64s(nonzero, simd.less_than_f64s(magnitude, minimum_vector));
            if !mask_any(simd, too_small) {
                break;
            }
            let overflow =
                simd.and_m64s(too_small, simd.equal_i64s(scale_vectors[index], scale_min));
            if mask_any(simd, overflow) {
                return false;
            }
            value_vectors[index] = simd.select_f64s(
                too_small,
                simd.mul_f64s(value_vectors[index], scale_up),
                value_vectors[index],
            );
            scale_vectors[index] = simd.select_i64s(
                too_small,
                simd.add_i64s(scale_vectors[index], scale_minus_one),
                scale_vectors[index],
            );
            magnitude = simd.abs_f64s(value_vectors[index]);
        }
    }
    for (value, scale) in value_tail.iter_mut().zip(scale_tail.iter_mut()) {
        if !normalize_lane(value, scale, maximum) {
            return false;
        }
    }
    true
}

#[expect(
    clippy::too_many_arguments,
    reason = "DUCC prefix advancement mutates one batch's recurrence pair and scale state."
)]
fn iter_to_ieee<'a, S: Simd>(
    simd: S,
    m: usize,
    lmax: usize,
    csq: &[f64],
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    previous: &mut &'a mut [f64],
    current: &mut &'a mut [f64],
    scale: &mut [i64],
) -> Option<(usize, usize, bool)> {
    let mut below_limit = rescale_for_prefix(simd, &mut previous[..], &mut current[..], scale)?;
    let mut degree = m;
    let mut pair = 0;
    while below_limit {
        if degree > lmax {
            return Some((degree, pair, true));
        }
        if lmax.saturating_sub(degree) < 4 {
            return Some((lmax.saturating_add(1), pair, true));
        }
        recurrence_two_pair_step(
            simd,
            csq,
            coefficient_a[pair],
            coefficient_b[pair],
            coefficient_a[pair + 1],
            coefficient_b[pair + 1],
            &mut previous[..],
            &mut current[..],
        );
        degree += 4;
        pair += 2;
        below_limit = rescale_for_prefix(simd, &mut previous[..], &mut current[..], scale)?;
    }
    Some((degree, pair, false))
}

fn mask_any<S: Simd>(simd: S, mask: S::m64s) -> bool {
    let bits = simd.select_f64s(mask, simd.splat_f64s(1.0), simd.splat_f64s(0.0));
    simd.reduce_max_f64s(bits) > 0.0
}

fn mask_all<S: Simd>(simd: S, mask: S::m64s) -> bool {
    let bits = simd.select_f64s(mask, simd.splat_f64s(1.0), simd.splat_f64s(0.0));
    simd.reduce_min_f64s(bits) > 0.0
}

/// Applies DUCC's recurrence rescale to selected lanes and returns whether
/// any lane was rescaled and whether every lane remains below scale epoch 1.
#[inline]
fn rescale_batch<S: Simd>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale: &mut [i64],
) -> Option<bool> {
    rescale_batch_inner(simd, previous, current, scale, true, false).map(|status| status.0)
}

#[inline]
fn rescale_for_prefix<S: Simd>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale: &mut [i64],
) -> Option<bool> {
    rescale_batch_inner(simd, previous, current, scale, false, true).map(|status| status.1)
}

#[inline]
fn rescale_batch_inner<S: Simd>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale: &mut [i64],
    track_any_rescaled: bool,
    track_prefix_epoch: bool,
) -> Option<(bool, bool)> {
    if previous.len() != current.len() || current.len() != scale.len() {
        return None;
    }
    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
    let (scale_vectors, scale_tail) = S::as_mut_simd_i64s(scale);
    let ftol = simd.splat_f64s(FTOL);
    let scale_down = simd.splat_f64s(SCALE_DOWN);
    let scale_one = simd.splat_i64s(1);
    let scale_max = simd.splat_i64s(i64::MAX);
    let mut any_rescaled = false;
    let mut all_below_one = true;

    for index in 0..current_vectors.len() {
        let rescale = simd.greater_than_f64s(simd.abs_f64s(current_vectors[index]), ftol);
        let overflow = simd.and_m64s(rescale, simd.equal_i64s(scale_vectors[index], scale_max));
        if mask_any(simd, overflow) {
            return None;
        }
        if track_any_rescaled {
            any_rescaled |= mask_any(simd, rescale);
        }
        previous_vectors[index] = simd.select_f64s(
            rescale,
            simd.mul_f64s(previous_vectors[index], scale_down),
            previous_vectors[index],
        );
        current_vectors[index] = simd.select_f64s(
            rescale,
            simd.mul_f64s(current_vectors[index], scale_down),
            current_vectors[index],
        );
        scale_vectors[index] = simd.select_i64s(
            rescale,
            simd.add_i64s(scale_vectors[index], scale_one),
            scale_vectors[index],
        );
        if track_prefix_epoch {
            all_below_one &= mask_all(simd, simd.less_than_i64s(scale_vectors[index], scale_one));
        }
    }

    for lane in 0..current_tail.len() {
        if current_tail[lane].abs() > FTOL {
            previous_tail[lane] *= SCALE_DOWN;
            current_tail[lane] *= SCALE_DOWN;
            scale_tail[lane] = scale_tail[lane].checked_add(1)?;
            if track_any_rescaled {
                any_rescaled = true;
            }
        }
        if track_prefix_epoch {
            all_below_one &= scale_tail[lane] < 1;
        }
    }
    Some((any_rescaled, all_below_one))
}

fn set_correction_factors<S: Simd>(simd: S, scale: &[i64], correction: &mut [f64]) {
    let (scale_vectors, scale_tail) = S::as_simd_i64s(scale);
    let (correction_vectors, correction_tail) = S::as_mut_simd_f64s(correction);
    let zero_i64 = simd.splat_i64s(0);
    let one_i64 = simd.splat_i64s(1);
    let zero_f64 = simd.splat_f64s(0.0);
    let one_f64 = simd.splat_f64s(1.0);
    let scale_up = simd.splat_f64s(SCALE_UP);
    for (scale_vector, correction_vector) in scale_vectors.iter().zip(correction_vectors.iter_mut())
    {
        let below_one = simd.less_than_i64s(*scale_vector, one_i64);
        let negative = simd.less_than_i64s(*scale_vector, zero_i64);
        let nonnegative_correction = simd.select_f64s(below_one, one_f64, scale_up);
        *correction_vector = simd.select_f64s(negative, zero_f64, nonnegative_correction);
    }
    for (scale_value, correction_value) in scale_tail.iter().zip(correction_tail.iter_mut()) {
        *correction_value = if *scale_value < 0 {
            0.0
        } else if *scale_value < 1 {
            1.0
        } else {
            SCALE_UP
        };
    }
}

fn all_scale_nonnegative<S: Simd>(simd: S, scale: &[i64]) -> bool {
    let (scale_vectors, scale_tail) = S::as_simd_i64s(scale);
    let zero = simd.splat_i64s(0);
    let mut all_nonnegative = true;
    for scale_vector in scale_vectors {
        all_nonnegative &= mask_all(simd, simd.greater_than_or_equal_i64s(*scale_vector, zero));
    }
    all_nonnegative && scale_tail.iter().all(|&value| value >= 0)
}

fn apply_correction_and_clear_scale<S: Simd>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    correction: &[f64],
    scale: &mut [i64],
) {
    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
    let (correction_vectors, correction_tail) = S::as_simd_f64s(correction);
    for index in 0..current_vectors.len() {
        previous_vectors[index] = simd.mul_f64s(previous_vectors[index], correction_vectors[index]);
        current_vectors[index] = simd.mul_f64s(current_vectors[index], correction_vectors[index]);
    }
    for lane in 0..current_tail.len() {
        previous_tail[lane] *= correction_tail[lane];
        current_tail[lane] *= correction_tail[lane];
    }
    scale.fill(0);
}

#[expect(
    clippy::too_many_arguments,
    reason = "DUCC advances two paired recurrence states in one SIMD loop."
)]
fn recurrence_two_pair_step<S: Simd>(
    simd: S,
    csq: &[f64],
    coefficient_a1: f64,
    coefficient_b1: f64,
    coefficient_a2: f64,
    coefficient_b2: f64,
    previous: &mut [f64],
    current: &mut [f64],
) {
    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
    let (csq_vectors, csq_tail) = S::as_simd_f64s(csq);
    let a1 = simd.splat_f64s(coefficient_a1);
    let b1 = simd.splat_f64s(coefficient_b1);
    let a2 = simd.splat_f64s(coefficient_a2);
    let b2 = simd.splat_f64s(coefficient_b2);
    for index in 0..current_vectors.len() {
        let old_previous = previous_vectors[index];
        let old_current = current_vectors[index];
        let leading1 = simd.mul_add_f64s(a1, csq_vectors[index], b1);
        let next1 = simd.mul_add_f64s(leading1, old_current, old_previous);
        let leading2 = simd.mul_add_f64s(a2, csq_vectors[index], b2);
        let next2 = simd.mul_add_f64s(leading2, next1, old_current);
        previous_vectors[index] = next1;
        current_vectors[index] = next2;
    }
    for lane in 0..current_tail.len() {
        let leading1 = coefficient_a1.mul_add(csq_tail[lane], coefficient_b1);
        let next1 = leading1.mul_add(current_tail[lane], previous_tail[lane]);
        let leading2 = coefficient_a2.mul_add(csq_tail[lane], coefficient_b2);
        let next2 = leading2.mul_add(next1, current_tail[lane]);
        previous_tail[lane] = next1;
        current_tail[lane] = next2;
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The scaled DUCC loop fuses one recurrence step with even/odd coefficient accumulation."
)]
fn scaled_pair_step<S: Simd>(
    simd: S,
    correction: &[f64],
    csq: &[f64],
    coefficient_a: f64,
    coefficient_b: f64,
    even: Complex64,
    odd: Complex64,
    previous: &mut [f64],
    current: &mut [f64],
    sum_even_re: &mut [f64],
    sum_even_im: &mut [f64],
    sum_odd_re: &mut [f64],
    sum_odd_im: &mut [f64],
) {
    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
    let (correction_vectors, correction_tail) = S::as_simd_f64s(correction);
    let (csq_vectors, csq_tail) = S::as_simd_f64s(csq);
    let (even_re_vectors, even_re_tail) = S::as_mut_simd_f64s(sum_even_re);
    let (even_im_vectors, even_im_tail) = S::as_mut_simd_f64s(sum_even_im);
    let (odd_re_vectors, odd_re_tail) = S::as_mut_simd_f64s(sum_odd_re);
    let (odd_im_vectors, odd_im_tail) = S::as_mut_simd_f64s(sum_odd_im);
    let even_re = simd.splat_f64s(even.re);
    let even_im = simd.splat_f64s(even.im);
    let odd_re = simd.splat_f64s(odd.re);
    let odd_im = simd.splat_f64s(odd.im);
    let a = simd.splat_f64s(coefficient_a);
    let b = simd.splat_f64s(coefficient_b);
    for index in 0..current_vectors.len() {
        let old_current = current_vectors[index];
        let basis = simd.mul_f64s(old_current, correction_vectors[index]);
        even_re_vectors[index] = simd.mul_add_f64s(basis, even_re, even_re_vectors[index]);
        even_im_vectors[index] = simd.mul_add_f64s(basis, even_im, even_im_vectors[index]);
        odd_re_vectors[index] = simd.mul_add_f64s(basis, odd_re, odd_re_vectors[index]);
        odd_im_vectors[index] = simd.mul_add_f64s(basis, odd_im, odd_im_vectors[index]);
        let leading = simd.mul_add_f64s(a, csq_vectors[index], b);
        current_vectors[index] = simd.mul_add_f64s(leading, old_current, previous_vectors[index]);
        previous_vectors[index] = old_current;
    }
    for lane in 0..current_tail.len() {
        let old_current = current_tail[lane];
        let basis = old_current * correction_tail[lane];
        even_re_tail[lane] = basis.mul_add(even.re, even_re_tail[lane]);
        even_im_tail[lane] = basis.mul_add(even.im, even_im_tail[lane]);
        odd_re_tail[lane] = basis.mul_add(odd.re, odd_re_tail[lane]);
        odd_im_tail[lane] = basis.mul_add(odd.im, odd_im_tail[lane]);
        let leading = coefficient_a.mul_add(csq_tail[lane], coefficient_b);
        current_tail[lane] = leading.mul_add(old_current, previous_tail[lane]);
        previous_tail[lane] = old_current;
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The ordinary DUCC AVX2 loop handles four degrees and both parity sums per SIMD lane pass."
)]
fn ordinary_two_pair_step<S: Simd>(
    simd: S,
    csq: &[f64],
    coefficient_a1: f64,
    coefficient_b1: f64,
    coefficient_a2: f64,
    coefficient_b2: f64,
    even1: Complex64,
    odd1: Complex64,
    even2: Complex64,
    odd2: Complex64,
    previous: &mut [f64],
    current: &mut [f64],
    sum_even_re: &mut [f64],
    sum_even_im: &mut [f64],
    sum_odd_re: &mut [f64],
    sum_odd_im: &mut [f64],
) {
    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
    let (csq_vectors, csq_tail) = S::as_simd_f64s(csq);
    let (even_re_vectors, even_re_tail) = S::as_mut_simd_f64s(sum_even_re);
    let (even_im_vectors, even_im_tail) = S::as_mut_simd_f64s(sum_even_im);
    let (odd_re_vectors, odd_re_tail) = S::as_mut_simd_f64s(sum_odd_re);
    let (odd_im_vectors, odd_im_tail) = S::as_mut_simd_f64s(sum_odd_im);
    let a1 = simd.splat_f64s(coefficient_a1);
    let b1 = simd.splat_f64s(coefficient_b1);
    let a2 = simd.splat_f64s(coefficient_a2);
    let b2 = simd.splat_f64s(coefficient_b2);
    let even1_re = simd.splat_f64s(even1.re);
    let even1_im = simd.splat_f64s(even1.im);
    let odd1_re = simd.splat_f64s(odd1.re);
    let odd1_im = simd.splat_f64s(odd1.im);
    let even2_re = simd.splat_f64s(even2.re);
    let even2_im = simd.splat_f64s(even2.im);
    let odd2_re = simd.splat_f64s(odd2.re);
    let odd2_im = simd.splat_f64s(odd2.im);
    for index in 0..current_vectors.len() {
        let old_previous = previous_vectors[index];
        let old_current = current_vectors[index];
        even_re_vectors[index] = simd.mul_add_f64s(old_current, even1_re, even_re_vectors[index]);
        even_im_vectors[index] = simd.mul_add_f64s(old_current, even1_im, even_im_vectors[index]);
        odd_re_vectors[index] = simd.mul_add_f64s(old_current, odd1_re, odd_re_vectors[index]);
        odd_im_vectors[index] = simd.mul_add_f64s(old_current, odd1_im, odd_im_vectors[index]);
        let leading1 = simd.mul_add_f64s(a1, csq_vectors[index], b1);
        let next1 = simd.mul_add_f64s(leading1, old_current, old_previous);
        even_re_vectors[index] = simd.mul_add_f64s(next1, even2_re, even_re_vectors[index]);
        even_im_vectors[index] = simd.mul_add_f64s(next1, even2_im, even_im_vectors[index]);
        odd_re_vectors[index] = simd.mul_add_f64s(next1, odd2_re, odd_re_vectors[index]);
        odd_im_vectors[index] = simd.mul_add_f64s(next1, odd2_im, odd_im_vectors[index]);
        let leading2 = simd.mul_add_f64s(a2, csq_vectors[index], b2);
        let next2 = simd.mul_add_f64s(leading2, next1, old_current);
        previous_vectors[index] = next1;
        current_vectors[index] = next2;
    }
    for lane in 0..current_tail.len() {
        let old_previous = previous_tail[lane];
        let old_current = current_tail[lane];
        even_re_tail[lane] = old_current.mul_add(even1.re, even_re_tail[lane]);
        even_im_tail[lane] = old_current.mul_add(even1.im, even_im_tail[lane]);
        odd_re_tail[lane] = old_current.mul_add(odd1.re, odd_re_tail[lane]);
        odd_im_tail[lane] = old_current.mul_add(odd1.im, odd_im_tail[lane]);
        let leading1 = coefficient_a1.mul_add(csq_tail[lane], coefficient_b1);
        let next1 = leading1.mul_add(old_current, old_previous);
        even_re_tail[lane] = next1.mul_add(even2.re, even_re_tail[lane]);
        even_im_tail[lane] = next1.mul_add(even2.im, even_im_tail[lane]);
        odd_re_tail[lane] = next1.mul_add(odd2.re, odd_re_tail[lane]);
        odd_im_tail[lane] = next1.mul_add(odd2.im, odd_im_tail[lane]);
        let leading2 = coefficient_a2.mul_add(csq_tail[lane], coefficient_b2);
        let next2 = leading2.mul_add(next1, old_current);
        previous_tail[lane] = next1;
        current_tail[lane] = next2;
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The final ordinary degree pair shares one vector recurrence and accumulation pass."
)]
#[inline(always)]
fn ordinary_one_pair_step<S: Simd>(
    simd: S,
    csq: &[f64],
    coefficient_a: f64,
    coefficient_b: f64,
    even: Complex64,
    odd: Complex64,
    previous: &mut [f64],
    current: &mut [f64],
    sum_even_re: &mut [f64],
    sum_even_im: &mut [f64],
    sum_odd_re: &mut [f64],
    sum_odd_im: &mut [f64],
) {
    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
    let (csq_vectors, csq_tail) = S::as_simd_f64s(csq);
    let (even_re_vectors, even_re_tail) = S::as_mut_simd_f64s(sum_even_re);
    let (even_im_vectors, even_im_tail) = S::as_mut_simd_f64s(sum_even_im);
    let (odd_re_vectors, odd_re_tail) = S::as_mut_simd_f64s(sum_odd_re);
    let (odd_im_vectors, odd_im_tail) = S::as_mut_simd_f64s(sum_odd_im);
    let a = simd.splat_f64s(coefficient_a);
    let b = simd.splat_f64s(coefficient_b);
    let even_re = simd.splat_f64s(even.re);
    let even_im = simd.splat_f64s(even.im);
    let odd_re = simd.splat_f64s(odd.re);
    let odd_im = simd.splat_f64s(odd.im);
    for index in 0..current_vectors.len() {
        let old_previous = previous_vectors[index];
        let old_current = current_vectors[index];
        even_re_vectors[index] = simd.mul_add_f64s(old_current, even_re, even_re_vectors[index]);
        even_im_vectors[index] = simd.mul_add_f64s(old_current, even_im, even_im_vectors[index]);
        odd_re_vectors[index] = simd.mul_add_f64s(old_current, odd_re, odd_re_vectors[index]);
        odd_im_vectors[index] = simd.mul_add_f64s(old_current, odd_im, odd_im_vectors[index]);
        let leading = simd.mul_add_f64s(a, csq_vectors[index], b);
        let next = simd.mul_add_f64s(leading, old_current, old_previous);
        previous_vectors[index] = old_current;
        current_vectors[index] = next;
    }
    for lane in 0..current_tail.len() {
        let old_previous = previous_tail[lane];
        let old_current = current_tail[lane];
        even_re_tail[lane] = old_current.mul_add(even.re, even_re_tail[lane]);
        even_im_tail[lane] = old_current.mul_add(even.im, even_im_tail[lane]);
        odd_re_tail[lane] = old_current.mul_add(odd.re, odd_re_tail[lane]);
        odd_im_tail[lane] = old_current.mul_add(odd.im, odd_im_tail[lane]);
        let leading = coefficient_a.mul_add(csq_tail[lane], coefficient_b);
        let next = leading.mul_add(old_current, old_previous);
        previous_tail[lane] = old_current;
        current_tail[lane] = next;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FTOL, SCALE_DOWN, SCALE_UP, WithSimd, all_scale_nonnegative, initialize_sectoral_batch,
        mypow, normalize_batch, normalize_lane, power_lower_limit, rescale_batch,
        scalar_integer_power, set_correction_factors,
    };
    use pulp::{Arch, Simd};

    struct InitializeDispatch<'a> {
        sine: &'a [f64],
        values: &'a mut [f64],
        scales: &'a mut [i64],
        exponent: usize,
        lower_limit: f64,
        factor: f64,
    }

    impl WithSimd for InitializeDispatch<'_> {
        type Output = bool;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            initialize_sectoral_batch(
                simd,
                self.sine,
                self.values,
                self.scales,
                self.exponent,
                self.lower_limit,
                self.factor,
            )
        }
    }

    struct NormalizeDispatch<'a> {
        values: &'a mut [f64],
        scales: &'a mut [i64],
        maximum: f64,
    }

    impl WithSimd for NormalizeDispatch<'_> {
        type Output = bool;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            normalize_batch(simd, self.values, self.scales, self.maximum)
        }
    }

    struct RescaleDispatch<'a> {
        previous: &'a mut [f64],
        current: &'a mut [f64],
        scales: &'a mut [i64],
    }

    impl WithSimd for RescaleDispatch<'_> {
        type Output = Option<bool>;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            rescale_batch(simd, self.previous, self.current, self.scales)
        }
    }

    struct CorrectionDispatch<'a> {
        scales: &'a [i64],
        correction: &'a mut [f64],
    }

    impl WithSimd for CorrectionDispatch<'_> {
        type Output = bool;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            set_correction_factors(simd, self.scales, self.correction);
            all_scale_nonnegative(simd, self.scales)
        }
    }

    #[test]
    fn sectoral_initializer_matches_checked_lane_route_across_vector_tail() {
        let exponent = 17;
        let lower_limit = power_lower_limit(exponent);
        let mut sine = (0..19)
            .map(|index| 0.2 + index as f64 * 0.03)
            .collect::<Vec<_>>();
        sine[0] = lower_limit;
        sine[1] = lower_limit * 0.5;
        sine[2] = 1.0;
        let factor = -0.75;

        for arch in [Arch::Scalar, Arch::new()] {
            let mut values = vec![f64::NAN; sine.len()];
            let mut scales = vec![i64::MIN; sine.len()];
            assert!(arch.dispatch(InitializeDispatch {
                sine: &sine,
                values: &mut values,
                scales: &mut scales,
                exponent,
                lower_limit,
                factor,
            }));

            for lane in 0..sine.len() {
                let (mut expected, mut expected_scale) =
                    mypow(sine[lane], exponent, lower_limit).unwrap();
                expected *= factor;
                assert!(normalize_lane(&mut expected, &mut expected_scale, FTOL));
                assert_eq!(values[lane].to_bits(), expected.to_bits(), "lane {lane}");
                assert_eq!(scales[lane], expected_scale, "lane {lane}");
            }
        }
    }

    #[test]
    fn sectoral_initializer_declines_nonfinite_lanes() {
        let sine = [0.5, f64::NAN, 0.75];
        let mut values = [0.0; 3];
        let mut scales = [0; 3];
        assert!(!Arch::new().dispatch(InitializeDispatch {
            sine: &sine,
            values: &mut values,
            scales: &mut scales,
            exponent: 3,
            lower_limit: power_lower_limit(3),
            factor: 1.0,
        }));
    }

    #[test]
    fn zero_order_initialization_keeps_exact_unit_power() {
        let sine = [0.0, 0.25, 0.75, 1.0, 0.3];
        let mut values = [f64::NAN; 5];
        let mut scales = [i64::MIN; 5];
        let factor = FTOL * 0.5;
        assert!(Arch::new().dispatch(InitializeDispatch {
            sine: &sine,
            values: &mut values,
            scales: &mut scales,
            exponent: 0,
            lower_limit: power_lower_limit(0),
            factor,
        }));
        assert_eq!(values, [factor; 5]);
        assert_eq!(scales, [0; 5]);
        assert_eq!(scalar_integer_power(0.0, 0), 1.0);
    }

    #[test]
    fn vector_normalization_matches_checked_lane_route_across_vector_tail() {
        let mut values = (0..19)
            .map(|index| 0.05 + index as f64 * 0.2)
            .collect::<Vec<_>>();
        values[0] = 2.0;
        values[1] = f64::MIN_POSITIVE;
        values[2] = f64::from_bits(1);
        values[3] = 0.0;
        values[4] = -2.0;
        let mut expected = values.clone();
        let scales = vec![0_i64; values.len()];
        let mut expected_scales = scales.clone();
        for lane in 0..expected.len() {
            assert!(normalize_lane(
                &mut expected[lane],
                &mut expected_scales[lane],
                FTOL,
            ));
        }

        for arch in [Arch::Scalar, Arch::new()] {
            let mut actual = values.clone();
            let mut actual_scales = scales.clone();
            assert!(arch.dispatch(NormalizeDispatch {
                values: &mut actual,
                scales: &mut actual_scales,
                maximum: FTOL,
            }));
            assert_eq!(actual, expected);
            assert_eq!(actual_scales, expected_scales);
        }
    }

    #[test]
    fn vector_rescaling_matches_lane_epoch_and_tail_rules() {
        let previous = (0..19).map(|lane| 0.25 + lane as f64).collect::<Vec<_>>();
        let current = (0..19)
            .map(|lane| match lane % 5 {
                0 => FTOL * 0.5,
                1 => FTOL,
                2 => FTOL * 2.0,
                3 => -FTOL * 3.0,
                _ => 0.0,
            })
            .collect::<Vec<_>>();
        let scales = (0..19)
            .map(|lane| match lane % 5 {
                0 => -2,
                1 => -1,
                2 => 0,
                3 => 1,
                _ => i64::MAX,
            })
            .collect::<Vec<_>>();
        let mut expected_previous = previous.clone();
        let mut expected_current = current.clone();
        let mut expected_scales = scales.clone();
        let mut expected_any = false;
        for lane in 0..current.len() {
            if current[lane].abs() > FTOL {
                expected_previous[lane] *= SCALE_DOWN;
                expected_current[lane] *= SCALE_DOWN;
                expected_scales[lane] = expected_scales[lane].checked_add(1).unwrap();
                expected_any = true;
            }
        }

        for arch in [Arch::Scalar, Arch::new()] {
            let mut actual_previous = previous.clone();
            let mut actual_current = current.clone();
            let mut actual_scales = scales.clone();
            let actual_any = arch
                .dispatch(RescaleDispatch {
                    previous: &mut actual_previous,
                    current: &mut actual_current,
                    scales: &mut actual_scales,
                })
                .unwrap();
            assert_eq!(actual_any, expected_any);
            assert_eq!(actual_previous, expected_previous);
            assert_eq!(actual_current, expected_current);
            assert_eq!(actual_scales, expected_scales);
        }
    }

    #[test]
    fn vector_correction_factors_preserve_scale_epoch_semantics() {
        let scales = (0..19)
            .map(|lane| match lane % 4 {
                0 => -2,
                1 => 0,
                2 => 1,
                _ => 3,
            })
            .collect::<Vec<_>>();
        let mut correction = vec![f64::NAN; scales.len()];
        let all_nonnegative = Arch::new().dispatch(CorrectionDispatch {
            scales: &scales,
            correction: &mut correction,
        });
        assert!(!all_nonnegative);
        for (scale, actual) in scales.iter().zip(correction) {
            let expected = if *scale < 0 {
                0.0
            } else if *scale < 1 {
                1.0
            } else {
                SCALE_UP
            };
            assert_eq!(actual.to_bits(), expected.to_bits());
        }
    }

    #[test]
    fn normalization_and_rescaling_reject_exponent_overflow() {
        let mut values = vec![2.0; 19];
        let mut scales = vec![0_i64; values.len()];
        scales[5] = i64::MAX;
        assert!(!Arch::new().dispatch(NormalizeDispatch {
            values: &mut values,
            scales: &mut scales,
            maximum: FTOL,
        }));

        let mut previous = vec![0.5; 19];
        let mut current = vec![FTOL * 0.5; 19];
        let mut scales = vec![0_i64; current.len()];
        current[7] = FTOL * 2.0;
        scales[7] = i64::MAX;
        assert!(
            Arch::new()
                .dispatch(RescaleDispatch {
                    previous: &mut previous,
                    current: &mut current,
                    scales: &mut scales,
                })
                .is_none()
        );
    }
}
