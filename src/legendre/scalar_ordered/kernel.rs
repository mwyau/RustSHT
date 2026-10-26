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

use super::*;

pub(super) const SCALE_UP: f64 = f64::from_bits(1823_u64 << 52);
pub(super) const SCALE_DOWN: f64 = f64::from_bits(223_u64 << 52);
pub(super) const HALF_SCALE: f64 = f64::from_bits(1423_u64 << 52);
pub(super) const FTOL: f64 = f64::from_bits((1023_u64 - 60) << 52);

pub(super) struct ScalarSynthesisBatch<'a> {
    pub(super) sin_theta: &'a [f64],
    pub(super) active_lanes: usize,
    pub(super) vector_lanes: usize,
}

/// Executes one already prepared scalar component over a packed latitude batch.
///
/// The caller owns coefficient preparation and reflected-ring packing so spin
/// synthesis can reuse both for its Q and U passes. The scaled-prefix and
/// ordinary SIMD suffix remain the same implementation as scalar ordered
/// synthesis.
pub(crate) fn synthesize_prepared_component_batch_dispatched(
    arch: Arch,
    m: usize,
    lmax: usize,
    sectoral_norm: f64,
    sin_theta: &[f64],
    vector_lanes: usize,
    scratch: &mut ScalarOrderedScratch<'_, f64>,
) -> bool {
    let Some(pair_count) = lmax
        .checked_sub(m)
        .and_then(|n| n.checked_add(2))
        .map(|n| n / 2)
    else {
        return false;
    };
    if sin_theta.is_empty()
        || vector_lanes < sin_theta.len()
        || vector_lanes > SCALAR_ORDER_BATCH_SIZE
        || scratch.packed_sin.len() < vector_lanes
        || scratch.packed_csq.len() < vector_lanes
        || scratch.previous.len() < vector_lanes
        || scratch.current.len() < vector_lanes
        || scratch.scale.len() < vector_lanes
        || scratch.corfac.len() < vector_lanes
        || scratch.sum_even_re.len() < vector_lanes
        || scratch.sum_even_im.len() < vector_lanes
        || scratch.sum_odd_re.len() < vector_lanes
        || scratch.sum_odd_im.len() < vector_lanes
        || scratch.coefficient_a.len() < pair_count
        || scratch.coefficient_b.len() < pair_count
        || scratch.coefficient_even.len() < pair_count
        || scratch.coefficient_odd.len() < pair_count
    {
        return false;
    }
    arch.dispatch(PreparedComponentBatch {
        m,
        lmax,
        sectoral_norm,
        sin_theta,
        vector_lanes,
        scratch,
    })
}

struct PreparedComponentBatch<'a, 's> {
    m: usize,
    lmax: usize,
    sectoral_norm: f64,
    sin_theta: &'a [f64],
    vector_lanes: usize,
    scratch: &'a mut ScalarOrderedScratch<'s, f64>,
}

impl WithSimd for PreparedComponentBatch<'_, '_> {
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self {
            m,
            lmax,
            sectoral_norm,
            sin_theta,
            vector_lanes,
            scratch,
        } = self;
        synthesize_batch(
            simd,
            scratch,
            m,
            lmax,
            sectoral_norm,
            power_lower_limit(m),
            ScalarSynthesisBatch {
                sin_theta,
                active_lanes: sin_theta.len(),
                vector_lanes,
            },
        )
    }
}

#[inline(always)]
pub(super) fn synthesize_batch<S: Simd>(
    simd: S,
    scratch: &mut ScalarOrderedScratch<'_, f64>,
    m: usize,
    lmax: usize,
    sectoral_norm: f64,
    power_lower_limit: f64,
    batch: ScalarSynthesisBatch<'_>,
) -> bool {
    let ScalarSynthesisBatch {
        sin_theta: sine,
        active_lanes: input_count,
        vector_lanes: active_count,
    } = batch;
    let Some(pair_count) = lmax
        .checked_sub(m)
        .and_then(|n| n.checked_add(2))
        .map(|n| n / 2)
    else {
        return false;
    };
    let simd_lanes = S::F64_LANES;
    if simd_lanes == 0
        || input_count == 0
        || sine.len() != input_count
        || active_count < input_count
        || active_count > SCALAR_ORDER_BATCH_SIZE
        || !active_count.is_multiple_of(simd_lanes)
        || active_count != input_count.div_ceil(simd_lanes) * simd_lanes
    {
        return false;
    }

    let packed_csq = &scratch.packed_csq[..active_count];
    let coefficient_a = &scratch.coefficient_a[..];
    let coefficient_b = &scratch.coefficient_b[..];
    let coefficient_even = &scratch.coefficient_even[..];
    let coefficient_odd = &scratch.coefficient_odd[..];
    let sign = if m.is_multiple_of(2) { 1.0 } else { -1.0 };
    let initialized = if active_count == input_count {
        initialize_sectoral_batch(
            simd,
            sine,
            &mut scratch.current[..active_count],
            &mut scratch.scale[..active_count],
            m,
            power_lower_limit,
            sign * sectoral_norm,
        )
    } else {
        let last = input_count - 1;
        scratch.packed_sin[..input_count].copy_from_slice(sine);
        scratch.packed_sin[input_count..active_count].fill(sine[last]);
        let ScalarOrderedScratch {
            packed_sin,
            current,
            scale,
            ..
        } = scratch;
        initialize_sectoral_batch(
            simd,
            &packed_sin[..active_count],
            &mut current[..active_count],
            &mut scale[..active_count],
            m,
            power_lower_limit,
            sign * sectoral_norm,
        )
    };
    if !initialized {
        return false;
    }
    scratch.previous[..active_count].fill(0.0);
    scratch.sum_even_re[..active_count].fill(0.0);
    scratch.sum_even_im[..active_count].fill(0.0);
    scratch.sum_odd_re[..active_count].fill(0.0);
    scratch.sum_odd_im[..active_count].fill(0.0);

    // DUCC advances all vector groups in an active ring batch with one shared
    // degree index. This also reuses each degree's coefficients across every
    // SIMD group and avoids restarting the insignificant prefix per vector.
    let (degree, pair, full_ieee) = {
        let mut previous = &mut scratch.previous[..active_count];
        let mut current = &mut scratch.current[..active_count];
        let Some((mut degree, mut pair, skip_remainder)) = iter_to_ieee(
            simd,
            m,
            lmax,
            packed_csq,
            coefficient_a,
            coefficient_b,
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
                packed_csq,
                coefficient_a[pair],
                coefficient_b[pair],
                coefficient_even[pair],
                coefficient_odd[pair],
                &mut previous[..],
                &mut current[..],
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
        }
        (degree, pair, full_ieee)
    };

    if degree <= lmax && pair < pair_count && full_ieee {
        if simd_lanes > 1 {
            let vector_pair_width = 2 * simd_lanes;
            let mut start = 0;
            while start + vector_pair_width <= active_count {
                if !synthesize_ordinary_suffix_two_vectors(
                    simd,
                    scratch,
                    lmax,
                    pair_count,
                    degree,
                    pair,
                    start,
                    start + simd_lanes,
                ) {
                    return false;
                }
                start += vector_pair_width;
            }
            if start < active_count
                && !synthesize_ordinary_suffix_vector(
                    simd,
                    scratch,
                    lmax,
                    pair_count,
                    degree,
                    pair,
                    start,
                    active_count,
                )
            {
                return false;
            }
        } else {
            for start in (0..active_count).step_by(simd_lanes) {
                if !synthesize_ordinary_suffix_vector(
                    simd,
                    scratch,
                    lmax,
                    pair_count,
                    degree,
                    pair,
                    start,
                    start + simd_lanes,
                ) {
                    return false;
                }
            }
        }
    }

    for lane in 0..active_count {
        if !scratch.sum_even_re[lane].is_finite()
            || !scratch.sum_even_im[lane].is_finite()
            || !scratch.sum_odd_re[lane].is_finite()
            || !scratch.sum_odd_im[lane].is_finite()
            || !scratch.previous[lane].is_finite()
            || !scratch.current[lane].is_finite()
        {
            return false;
        }
    }
    true
}

#[expect(
    clippy::too_many_arguments,
    reason = "The ordinary DUCC suffix keeps one packed SIMD recurrence state in registers."
)]
fn synthesize_ordinary_suffix_vector<S: Simd>(
    simd: S,
    scratch: &mut ScalarOrderedScratch<'_, f64>,
    lmax: usize,
    pair_count: usize,
    degree: usize,
    mut pair: usize,
    start: usize,
    end: usize,
) -> bool {
    if degree > lmax || pair >= pair_count {
        return true;
    }
    let packed_csq = &scratch.packed_csq[start..end];
    let coefficient_a = &scratch.coefficient_a[..];
    let coefficient_b = &scratch.coefficient_b[..];
    let coefficient_even = &scratch.coefficient_even[..];
    let coefficient_odd = &scratch.coefficient_odd[..];
    let Some(csq_vector) = load_simd_vector::<S>(packed_csq) else {
        return false;
    };
    let Some(mut previous_vector) = load_simd_vector::<S>(&scratch.previous[start..end]) else {
        return false;
    };
    let Some(mut current_vector) = load_simd_vector::<S>(&scratch.current[start..end]) else {
        return false;
    };
    let Some(mut sum_even_re_vector) = load_simd_vector::<S>(&scratch.sum_even_re[start..end])
    else {
        return false;
    };
    let Some(mut sum_even_im_vector) = load_simd_vector::<S>(&scratch.sum_even_im[start..end])
    else {
        return false;
    };
    let Some(mut sum_odd_re_vector) = load_simd_vector::<S>(&scratch.sum_odd_re[start..end]) else {
        return false;
    };
    let Some(mut sum_odd_im_vector) = load_simd_vector::<S>(&scratch.sum_odd_im[start..end]) else {
        return false;
    };

    // Keep the ordinary suffix's four accumulators and recurrence values in
    // registers after the batch-wide scaled prefix reaches ordinary range.
    while pair + 1 < pair_count {
        ordinary_two_pair_step(
            simd,
            csq_vector,
            coefficient_a[pair],
            coefficient_b[pair],
            coefficient_a[pair + 1],
            coefficient_b[pair + 1],
            coefficient_even[pair],
            coefficient_odd[pair],
            coefficient_even[pair + 1],
            coefficient_odd[pair + 1],
            &mut previous_vector,
            &mut current_vector,
            &mut sum_even_re_vector,
            &mut sum_even_im_vector,
            &mut sum_odd_re_vector,
            &mut sum_odd_im_vector,
        );
        pair += 2;
    }
    if pair < pair_count {
        ordinary_one_pair_step(
            simd,
            csq_vector,
            coefficient_a[pair],
            coefficient_b[pair],
            coefficient_even[pair],
            coefficient_odd[pair],
            &mut previous_vector,
            &mut current_vector,
            &mut sum_even_re_vector,
            &mut sum_even_im_vector,
            &mut sum_odd_re_vector,
            &mut sum_odd_im_vector,
        );
    }

    store_simd_vector::<S>(&mut scratch.previous[start..end], previous_vector)
        && store_simd_vector::<S>(&mut scratch.current[start..end], current_vector)
        && store_simd_vector::<S>(&mut scratch.sum_even_re[start..end], sum_even_re_vector)
        && store_simd_vector::<S>(&mut scratch.sum_even_im[start..end], sum_even_im_vector)
        && store_simd_vector::<S>(&mut scratch.sum_odd_re[start..end], sum_odd_re_vector)
        && store_simd_vector::<S>(&mut scratch.sum_odd_im[start..end], sum_odd_im_vector)
}

#[inline(always)]
#[expect(
    clippy::too_many_arguments,
    reason = "Two ring vectors share each degree's coefficient broadcasts while retaining register state."
)]
fn synthesize_ordinary_suffix_two_vectors<S: Simd>(
    simd: S,
    scratch: &mut ScalarOrderedScratch<'_, f64>,
    lmax: usize,
    pair_count: usize,
    degree: usize,
    pair: usize,
    first_start: usize,
    second_start: usize,
) -> bool {
    if degree > lmax || pair >= pair_count {
        return true;
    }
    let lanes = S::F64_LANES;
    let first_end = first_start + lanes;
    let second_end = second_start + lanes;
    let Some(coefficient_a) = scratch.coefficient_a.get(pair..pair_count) else {
        return false;
    };
    let Some(coefficient_b) = scratch.coefficient_b.get(pair..pair_count) else {
        return false;
    };
    let Some(coefficient_even) = scratch.coefficient_even.get(pair..pair_count) else {
        return false;
    };
    let Some(coefficient_odd) = scratch.coefficient_odd.get(pair..pair_count) else {
        return false;
    };
    let coefficient_a_pairs = coefficient_a.as_chunks::<2>().0;
    let coefficient_b_pairs = coefficient_b.as_chunks::<2>().0;
    let coefficient_even_pairs = coefficient_even.as_chunks::<2>().0;
    let coefficient_odd_pairs = coefficient_odd.as_chunks::<2>().0;

    let Some(first_csq) = load_simd_vector::<S>(&scratch.packed_csq[first_start..first_end]) else {
        return false;
    };
    let Some(second_csq) = load_simd_vector::<S>(&scratch.packed_csq[second_start..second_end])
    else {
        return false;
    };
    let Some(mut first_previous) = load_simd_vector::<S>(&scratch.previous[first_start..first_end])
    else {
        return false;
    };
    let Some(mut first_current) = load_simd_vector::<S>(&scratch.current[first_start..first_end])
    else {
        return false;
    };
    let Some(mut first_even_re) =
        load_simd_vector::<S>(&scratch.sum_even_re[first_start..first_end])
    else {
        return false;
    };
    let Some(mut first_even_im) =
        load_simd_vector::<S>(&scratch.sum_even_im[first_start..first_end])
    else {
        return false;
    };
    let Some(mut first_odd_re) = load_simd_vector::<S>(&scratch.sum_odd_re[first_start..first_end])
    else {
        return false;
    };
    let Some(mut first_odd_im) = load_simd_vector::<S>(&scratch.sum_odd_im[first_start..first_end])
    else {
        return false;
    };
    let Some(mut second_previous) =
        load_simd_vector::<S>(&scratch.previous[second_start..second_end])
    else {
        return false;
    };
    let Some(mut second_current) =
        load_simd_vector::<S>(&scratch.current[second_start..second_end])
    else {
        return false;
    };
    let Some(mut second_even_re) =
        load_simd_vector::<S>(&scratch.sum_even_re[second_start..second_end])
    else {
        return false;
    };
    let Some(mut second_even_im) =
        load_simd_vector::<S>(&scratch.sum_even_im[second_start..second_end])
    else {
        return false;
    };
    let Some(mut second_odd_re) =
        load_simd_vector::<S>(&scratch.sum_odd_re[second_start..second_end])
    else {
        return false;
    };
    let Some(mut second_odd_im) =
        load_simd_vector::<S>(&scratch.sum_odd_im[second_start..second_end])
    else {
        return false;
    };

    for (((a, b), even), odd) in coefficient_a_pairs
        .iter()
        .zip(coefficient_b_pairs)
        .zip(coefficient_even_pairs)
        .zip(coefficient_odd_pairs)
    {
        ordinary_two_pair_step_two_vectors(
            simd,
            first_csq,
            second_csq,
            a[0],
            b[0],
            a[1],
            b[1],
            even[0],
            odd[0],
            even[1],
            odd[1],
            &mut first_previous,
            &mut first_current,
            &mut first_even_re,
            &mut first_even_im,
            &mut first_odd_re,
            &mut first_odd_im,
            &mut second_previous,
            &mut second_current,
            &mut second_even_re,
            &mut second_even_im,
            &mut second_odd_re,
            &mut second_odd_im,
        );
    }

    let tail_offset = coefficient_a_pairs.len() * 2;
    if tail_offset < coefficient_a.len() {
        let Some(a) = coefficient_a.get(tail_offset) else {
            return false;
        };
        let Some(b) = coefficient_b.get(tail_offset) else {
            return false;
        };
        let Some(even) = coefficient_even.get(tail_offset) else {
            return false;
        };
        let Some(odd) = coefficient_odd.get(tail_offset) else {
            return false;
        };
        ordinary_one_pair_step(
            simd,
            first_csq,
            *a,
            *b,
            *even,
            *odd,
            &mut first_previous,
            &mut first_current,
            &mut first_even_re,
            &mut first_even_im,
            &mut first_odd_re,
            &mut first_odd_im,
        );
        ordinary_one_pair_step(
            simd,
            second_csq,
            *a,
            *b,
            *even,
            *odd,
            &mut second_previous,
            &mut second_current,
            &mut second_even_re,
            &mut second_even_im,
            &mut second_odd_re,
            &mut second_odd_im,
        );
    }

    store_simd_vector::<S>(
        &mut scratch.previous[first_start..first_end],
        first_previous,
    ) && store_simd_vector::<S>(&mut scratch.current[first_start..first_end], first_current)
        && store_simd_vector::<S>(
            &mut scratch.sum_even_re[first_start..first_end],
            first_even_re,
        )
        && store_simd_vector::<S>(
            &mut scratch.sum_even_im[first_start..first_end],
            first_even_im,
        )
        && store_simd_vector::<S>(
            &mut scratch.sum_odd_re[first_start..first_end],
            first_odd_re,
        )
        && store_simd_vector::<S>(
            &mut scratch.sum_odd_im[first_start..first_end],
            first_odd_im,
        )
        && store_simd_vector::<S>(
            &mut scratch.previous[second_start..second_end],
            second_previous,
        )
        && store_simd_vector::<S>(
            &mut scratch.current[second_start..second_end],
            second_current,
        )
        && store_simd_vector::<S>(
            &mut scratch.sum_even_re[second_start..second_end],
            second_even_re,
        )
        && store_simd_vector::<S>(
            &mut scratch.sum_even_im[second_start..second_end],
            second_even_im,
        )
        && store_simd_vector::<S>(
            &mut scratch.sum_odd_re[second_start..second_end],
            second_odd_re,
        )
        && store_simd_vector::<S>(
            &mut scratch.sum_odd_im[second_start..second_end],
            second_odd_im,
        )
}

pub(super) fn power_lower_limit(exponent: usize) -> f64 {
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

/// Uses DUCC's vector fast route when all lanes stay above its per-order
/// threshold. If any lane approaches underflow, the vector uses normalized
/// mantissas and checked SIMD exponent updates; only an incomplete tail uses
/// the checked scalar route.
pub(super) fn initialize_sectoral_batch<S: Simd>(
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
    let threshold = simd.splat_f64s(lower_limit);
    let (sine_vectors, sine_tail) = S::as_simd_f64s(sine);
    let (value_vectors, value_tail) = S::as_mut_simd_f64s(values);
    let (scale_vectors, scale_tail) = S::as_mut_simd_i64s(scales);
    let factor = simd.splat_f64s(factor_value);
    let finite_limit = simd.splat_f64s(f64::MAX);
    for index in 0..sine_vectors.len() {
        if !mask_all(
            simd,
            simd.less_than_or_equal_f64s(simd.abs_f64s(sine_vectors[index]), finite_limit),
        ) {
            return false;
        }
        let scaled = simd.less_than_f64s(simd.abs_f64s(sine_vectors[index]), threshold);
        if mask_any(simd, scaled) {
            let Some((value, scale)) =
                scaled_integer_power_vector(simd, sine_vectors[index], exponent)
            else {
                return false;
            };
            value_vectors[index] = simd.mul_f64s(value, factor);
            scale_vectors[index] = scale;
        } else {
            value_vectors[index] = simd.mul_f64s(
                integer_power_f64s(simd, sine_vectors[index], exponent),
                factor,
            );
            scale_vectors[index] = simd.splat_i64s(0);
        }
    }
    if sine_tail.iter().any(|value| !value.is_finite()) {
        return false;
    }
    for ((&sine_value, value), scale) in sine_tail
        .iter()
        .zip(value_tail.iter_mut())
        .zip(scale_tail.iter_mut())
    {
        let Some((power, exponent)) = mypow(sine_value, exponent, lower_limit) else {
            return false;
        };
        *value = power * factor_value;
        *scale = exponent;
    }
    normalize_batch(simd, values, scales, FTOL)
}

#[inline(always)]
fn scaled_integer_power_vector<S: Simd>(
    simd: S,
    mut base: S::f64s,
    exponent: usize,
) -> Option<(S::f64s, S::i64s)> {
    let mut base_scale = simd.splat_i64s(0);
    let mut value = simd.splat_f64s(1.0);
    let mut scale = simd.splat_i64s(0);
    normalize_vector(simd, &mut base, &mut base_scale, HALF_SCALE)?;
    let mut power = exponent;
    while power > 0 {
        if power & 1 != 0 {
            value = simd.mul_f64s(value, base);
            checked_add_scale_vector(simd, &mut scale, base_scale)?;
            normalize_vector(simd, &mut value, &mut scale, HALF_SCALE)?;
        }
        base = simd.mul_f64s(base, base);
        checked_double_scale_vector(simd, &mut base_scale)?;
        normalize_vector(simd, &mut base, &mut base_scale, HALF_SCALE)?;
        power >>= 1;
    }
    Some((value, scale))
}

#[inline(always)]
fn checked_add_scale_vector<S: Simd>(simd: S, scale: &mut S::i64s, addend: S::i64s) -> Option<()> {
    let sum = simd.add_i64s(*scale, addend);
    let overflow_bits = simd.and_i64s(simd.xor_i64s(*scale, sum), simd.xor_i64s(addend, sum));
    let zero = simd.splat_i64s(0);
    if mask_any(simd, simd.less_than_i64s(overflow_bits, zero)) {
        return None;
    }
    *scale = sum;
    Some(())
}

#[inline(always)]
fn checked_double_scale_vector<S: Simd>(simd: S, scale: &mut S::i64s) -> Option<()> {
    let doubled = simd.add_i64s(*scale, *scale);
    let sign_changed = simd.less_than_i64s(simd.xor_i64s(*scale, doubled), simd.splat_i64s(0));
    if mask_any(simd, sign_changed) {
        return None;
    }
    *scale = doubled;
    Some(())
}

#[inline(always)]
fn normalize_vector<S: Simd>(
    simd: S,
    value: &mut S::f64s,
    scale: &mut S::i64s,
    maximum: f64,
) -> Option<()> {
    let maximum_vector = simd.splat_f64s(maximum);
    let minimum_vector = simd.splat_f64s(maximum * SCALE_DOWN);
    let scale_down = simd.splat_f64s(SCALE_DOWN);
    let scale_up = simd.splat_f64s(SCALE_UP);
    let scale_one = simd.splat_i64s(1);
    let scale_minus_one = simd.splat_i64s(-1);
    let scale_max = simd.splat_i64s(i64::MAX);
    let scale_min = simd.splat_i64s(i64::MIN);
    let zero = simd.splat_f64s(0.0);

    let mut magnitude = simd.abs_f64s(*value);
    if !mask_all(
        simd,
        simd.less_than_or_equal_f64s(magnitude, simd.splat_f64s(f64::MAX)),
    ) {
        return None;
    }
    loop {
        let too_large = simd.greater_than_f64s(magnitude, maximum_vector);
        if !mask_any(simd, too_large) {
            break;
        }
        let overflow = simd.and_m64s(too_large, simd.equal_i64s(*scale, scale_max));
        if mask_any(simd, overflow) {
            return None;
        }
        *value = simd.select_f64s(too_large, simd.mul_f64s(*value, scale_down), *value);
        *scale = simd.select_i64s(too_large, simd.add_i64s(*scale, scale_one), *scale);
        magnitude = simd.abs_f64s(*value);
    }
    loop {
        let nonzero = simd.not_m64s(simd.equal_f64s(magnitude, zero));
        let too_small = simd.and_m64s(nonzero, simd.less_than_f64s(magnitude, minimum_vector));
        if !mask_any(simd, too_small) {
            break;
        }
        let overflow = simd.and_m64s(too_small, simd.equal_i64s(*scale, scale_min));
        if mask_any(simd, overflow) {
            return None;
        }
        *value = simd.select_f64s(too_small, simd.mul_f64s(*value, scale_up), *value);
        *scale = simd.select_i64s(too_small, simd.add_i64s(*scale, scale_minus_one), *scale);
        magnitude = simd.abs_f64s(*value);
    }
    Some(())
}

pub(super) fn scalar_integer_power(mut base: f64, mut exponent: usize) -> f64 {
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

pub(super) fn mypow(value: f64, exponent: usize, lower_limit: f64) -> Option<(f64, i64)> {
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

pub(super) fn normalize_lane(value: &mut f64, scale: &mut i64, maximum: f64) -> bool {
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

pub(super) fn normalize_batch<S: Simd>(
    simd: S,
    values: &mut [f64],
    scales: &mut [i64],
    maximum: f64,
) -> bool {
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
    let mut below_limit = rescale_prefix_initial(simd, &mut previous[..], &mut current[..], scale)?;
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
        below_limit = rescale_prefix_step(simd, &mut previous[..], &mut current[..], scale)?;
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
pub(super) fn rescale_batch<S: Simd>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale: &mut [i64],
) -> Option<bool> {
    rescale_batch_inner::<S, true, false>(simd, previous, current, scale).map(|status| status.0)
}

#[inline]
fn rescale_prefix_initial<S: Simd>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale: &mut [i64],
) -> Option<bool> {
    rescale_batch_inner::<S, false, true>(simd, previous, current, scale).map(|status| status.1)
}

#[inline]
pub(super) fn rescale_prefix_step<S: Simd>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale: &mut [i64],
) -> Option<bool> {
    let (any_rescaled, _) = rescale_batch_inner::<S, true, false>(simd, previous, current, scale)?;
    Some(!any_rescaled || all_scale_below_one(simd, scale))
}

// Scaled recurrence calls this after every degree pair; inlining removes a
// helper call from that loop and lets the compiler keep SIMD state in place.
#[inline(always)]
fn rescale_batch_inner<S: Simd, const TRACK_ANY_RESCALED: bool, const TRACK_PREFIX_EPOCH: bool>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale: &mut [i64],
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
        if TRACK_ANY_RESCALED {
            let vector_rescaled = mask_any(simd, rescale);
            any_rescaled |= vector_rescaled;
            if !vector_rescaled {
                if TRACK_PREFIX_EPOCH {
                    all_below_one &=
                        mask_all(simd, simd.less_than_i64s(scale_vectors[index], scale_one));
                }
                continue;
            }
        }
        let overflow = simd.and_m64s(rescale, simd.equal_i64s(scale_vectors[index], scale_max));
        if mask_any(simd, overflow) {
            return None;
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
        if TRACK_PREFIX_EPOCH {
            all_below_one &= mask_all(simd, simd.less_than_i64s(scale_vectors[index], scale_one));
        }
    }

    for lane in 0..current_tail.len() {
        if current_tail[lane].abs() > FTOL {
            previous_tail[lane] *= SCALE_DOWN;
            current_tail[lane] *= SCALE_DOWN;
            scale_tail[lane] = scale_tail[lane].checked_add(1)?;
            if TRACK_ANY_RESCALED {
                any_rescaled = true;
            }
        }
        if TRACK_PREFIX_EPOCH {
            all_below_one &= scale_tail[lane] < 1;
        }
    }
    Some((any_rescaled, all_below_one))
}

pub(super) fn set_correction_factors<S: Simd>(simd: S, scale: &[i64], correction: &mut [f64]) {
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

pub(super) fn all_scale_nonnegative<S: Simd>(simd: S, scale: &[i64]) -> bool {
    let (scale_vectors, scale_tail) = S::as_simd_i64s(scale);
    let zero = simd.splat_i64s(0);
    let mut all_nonnegative = true;
    for scale_vector in scale_vectors {
        all_nonnegative &= mask_all(simd, simd.greater_than_or_equal_i64s(*scale_vector, zero));
    }
    all_nonnegative && scale_tail.iter().all(|&value| value >= 0)
}

fn all_scale_below_one<S: Simd>(simd: S, scale: &[i64]) -> bool {
    let (scale_vectors, scale_tail) = S::as_simd_i64s(scale);
    let one = simd.splat_i64s(1);
    let mut all_below_one = true;
    for scale_vector in scale_vectors {
        all_below_one &= mask_all(simd, simd.less_than_i64s(*scale_vector, one));
    }
    all_below_one && scale_tail.iter().all(|&value| value < 1)
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

#[inline(always)]
fn load_simd_vector<S: Simd>(values: &[f64]) -> Option<S::f64s> {
    let (vectors, tail) = S::as_simd_f64s(values);
    (vectors.len() == 1 && tail.is_empty()).then(|| vectors[0])
}

#[inline(always)]
fn store_simd_vector<S: Simd>(values: &mut [f64], vector: S::f64s) -> bool {
    let (vectors, tail) = S::as_mut_simd_f64s(values);
    if vectors.len() != 1 || !tail.is_empty() {
        return false;
    }
    vectors[0] = vector;
    true
}

#[expect(
    clippy::too_many_arguments,
    reason = "The ordinary DUCC AVX2 loop handles four degrees and both parity sums per SIMD lane pass."
)]
#[inline(always)]
fn ordinary_two_pair_step<S: Simd>(
    simd: S,
    csq: S::f64s,
    coefficient_a1: f64,
    coefficient_b1: f64,
    coefficient_a2: f64,
    coefficient_b2: f64,
    even1: Complex64,
    odd1: Complex64,
    even2: Complex64,
    odd2: Complex64,
    previous: &mut S::f64s,
    current: &mut S::f64s,
    sum_even_re: &mut S::f64s,
    sum_even_im: &mut S::f64s,
    sum_odd_re: &mut S::f64s,
    sum_odd_im: &mut S::f64s,
) {
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
    let old_previous = *previous;
    let old_current = *current;
    *sum_even_re = simd.mul_add_f64s(old_current, even1_re, *sum_even_re);
    *sum_even_im = simd.mul_add_f64s(old_current, even1_im, *sum_even_im);
    *sum_odd_re = simd.mul_add_f64s(old_current, odd1_re, *sum_odd_re);
    *sum_odd_im = simd.mul_add_f64s(old_current, odd1_im, *sum_odd_im);
    let leading1 = simd.mul_add_f64s(a1, csq, b1);
    let next1 = simd.mul_add_f64s(leading1, old_current, old_previous);
    *sum_even_re = simd.mul_add_f64s(next1, even2_re, *sum_even_re);
    *sum_even_im = simd.mul_add_f64s(next1, even2_im, *sum_even_im);
    *sum_odd_re = simd.mul_add_f64s(next1, odd2_re, *sum_odd_re);
    *sum_odd_im = simd.mul_add_f64s(next1, odd2_im, *sum_odd_im);
    let leading2 = simd.mul_add_f64s(a2, csq, b2);
    let next2 = simd.mul_add_f64s(leading2, next1, old_current);
    *previous = next1;
    *current = next2;
}

#[expect(
    clippy::too_many_arguments,
    reason = "The two-vector ordinary suffix shares each coefficient broadcast across its lanes."
)]
#[inline(always)]
fn ordinary_two_pair_step_two_vectors<S: Simd>(
    simd: S,
    first_csq: S::f64s,
    second_csq: S::f64s,
    coefficient_a1: f64,
    coefficient_b1: f64,
    coefficient_a2: f64,
    coefficient_b2: f64,
    even1: Complex64,
    odd1: Complex64,
    even2: Complex64,
    odd2: Complex64,
    first_previous: &mut S::f64s,
    first_current: &mut S::f64s,
    first_sum_even_re: &mut S::f64s,
    first_sum_even_im: &mut S::f64s,
    first_sum_odd_re: &mut S::f64s,
    first_sum_odd_im: &mut S::f64s,
    second_previous: &mut S::f64s,
    second_current: &mut S::f64s,
    second_sum_even_re: &mut S::f64s,
    second_sum_even_im: &mut S::f64s,
    second_sum_odd_re: &mut S::f64s,
    second_sum_odd_im: &mut S::f64s,
) {
    let even1_re = simd.splat_f64s(even1.re);
    *first_sum_even_re = simd.mul_add_f64s(*first_current, even1_re, *first_sum_even_re);
    *second_sum_even_re = simd.mul_add_f64s(*second_current, even1_re, *second_sum_even_re);
    let even1_im = simd.splat_f64s(even1.im);
    *first_sum_even_im = simd.mul_add_f64s(*first_current, even1_im, *first_sum_even_im);
    *second_sum_even_im = simd.mul_add_f64s(*second_current, even1_im, *second_sum_even_im);
    let odd1_re = simd.splat_f64s(odd1.re);
    *first_sum_odd_re = simd.mul_add_f64s(*first_current, odd1_re, *first_sum_odd_re);
    *second_sum_odd_re = simd.mul_add_f64s(*second_current, odd1_re, *second_sum_odd_re);
    let odd1_im = simd.splat_f64s(odd1.im);
    *first_sum_odd_im = simd.mul_add_f64s(*first_current, odd1_im, *first_sum_odd_im);
    *second_sum_odd_im = simd.mul_add_f64s(*second_current, odd1_im, *second_sum_odd_im);

    let a1 = simd.splat_f64s(coefficient_a1);
    let b1 = simd.splat_f64s(coefficient_b1);
    let first_leading1 = simd.mul_add_f64s(a1, first_csq, b1);
    let first_next1 = simd.mul_add_f64s(first_leading1, *first_current, *first_previous);
    *first_previous = first_next1;
    let second_leading1 = simd.mul_add_f64s(a1, second_csq, b1);
    let second_next1 = simd.mul_add_f64s(second_leading1, *second_current, *second_previous);
    *second_previous = second_next1;

    let even2_re = simd.splat_f64s(even2.re);
    *first_sum_even_re = simd.mul_add_f64s(*first_previous, even2_re, *first_sum_even_re);
    *second_sum_even_re = simd.mul_add_f64s(*second_previous, even2_re, *second_sum_even_re);
    let even2_im = simd.splat_f64s(even2.im);
    *first_sum_even_im = simd.mul_add_f64s(*first_previous, even2_im, *first_sum_even_im);
    *second_sum_even_im = simd.mul_add_f64s(*second_previous, even2_im, *second_sum_even_im);
    let odd2_re = simd.splat_f64s(odd2.re);
    *first_sum_odd_re = simd.mul_add_f64s(*first_previous, odd2_re, *first_sum_odd_re);
    *second_sum_odd_re = simd.mul_add_f64s(*second_previous, odd2_re, *second_sum_odd_re);
    let odd2_im = simd.splat_f64s(odd2.im);
    *first_sum_odd_im = simd.mul_add_f64s(*first_previous, odd2_im, *first_sum_odd_im);
    *second_sum_odd_im = simd.mul_add_f64s(*second_previous, odd2_im, *second_sum_odd_im);

    let a2 = simd.splat_f64s(coefficient_a2);
    let b2 = simd.splat_f64s(coefficient_b2);
    let first_leading2 = simd.mul_add_f64s(a2, first_csq, b2);
    let first_next2 = simd.mul_add_f64s(first_leading2, *first_previous, *first_current);
    *first_current = first_next2;
    let second_leading2 = simd.mul_add_f64s(a2, second_csq, b2);
    let second_next2 = simd.mul_add_f64s(second_leading2, *second_previous, *second_current);
    *second_current = second_next2;
}

#[expect(
    clippy::too_many_arguments,
    reason = "The final ordinary degree pair shares one vector recurrence and accumulation pass."
)]
#[inline(always)]
fn ordinary_one_pair_step<S: Simd>(
    simd: S,
    csq: S::f64s,
    coefficient_a: f64,
    coefficient_b: f64,
    even: Complex64,
    odd: Complex64,
    previous: &mut S::f64s,
    current: &mut S::f64s,
    sum_even_re: &mut S::f64s,
    sum_even_im: &mut S::f64s,
    sum_odd_re: &mut S::f64s,
    sum_odd_im: &mut S::f64s,
) {
    let a = simd.splat_f64s(coefficient_a);
    let b = simd.splat_f64s(coefficient_b);
    let even_re = simd.splat_f64s(even.re);
    let even_im = simd.splat_f64s(even.im);
    let odd_re = simd.splat_f64s(odd.re);
    let odd_im = simd.splat_f64s(odd.im);
    let old_previous = *previous;
    let old_current = *current;
    *sum_even_re = simd.mul_add_f64s(old_current, even_re, *sum_even_re);
    *sum_even_im = simd.mul_add_f64s(old_current, even_im, *sum_even_im);
    *sum_odd_re = simd.mul_add_f64s(old_current, odd_re, *sum_odd_re);
    *sum_odd_im = simd.mul_add_f64s(old_current, odd_im, *sum_odd_im);
    let leading = simd.mul_add_f64s(a, csq, b);
    let next = simd.mul_add_f64s(leading, old_current, old_previous);
    *previous = old_current;
    *current = next;
}
