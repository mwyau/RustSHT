// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

/*
 * This module adapts DUCC's paired-degree scalar synthesis recurrence from
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

//! Scaled-prefix f64 scalar synthesis for active latitude tiles.

use num_complex::Complex64;
use pulp::{Arch, Simd, WithSimd};

use crate::error::ShtError;
use crate::legendre::scalar::{
    lane_values_in_normal_range, scalar_value_in_normal_range, simd_values_in_normal_range,
};
use crate::legendre::{scalar, scalar_paired};

#[cfg(test)]
use std::cell::RefCell;

/// Maximum number of active rings processed in one synthesis tile.
pub(crate) const ACTIVE_TILE_SIZE: usize = 128;

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default)]
struct FastRouteCounts {
    tiles: u64,
    completed_tiles: u64,
    declined_tiles: u64,
    empty_tiles: u64,
    coefficient_preflights: u64,
    unsafe_coefficients: u64,
    no_transition: u64,
    polar_pair_lanes: u64,
    prefix_pair_lanes: u64,
    suffix_pair_lanes: u64,
    fallback_pair_lanes: u64,
}

#[cfg(test)]
thread_local! {
    static FAST_ROUTE_COUNTS: RefCell<Option<FastRouteCounts>> = const { RefCell::new(None) };
}

#[cfg(test)]
fn update_route_counts(update: impl FnOnce(&mut FastRouteCounts)) {
    FAST_ROUTE_COUNTS.with_borrow_mut(|counts| {
        if let Some(counts) = counts {
            update(counts);
        }
    });
}

pub(crate) struct ScalarSynthesisFastScratch<'a, T> {
    pub(crate) spectra: &'a mut [num_complex::Complex<T>],
    pub(crate) previous: &'a mut [T],
    pub(crate) current: &'a mut [T],
    pub(crate) next: &'a mut [T],
    pub(crate) exponents: &'a mut [i64],
    pub(crate) sum_re: &'a mut [T],
    pub(crate) sum_im: &'a mut [T],
    pub(crate) compact_theta_sin: &'a mut [T],
    pub(crate) compact_theta_cos: &'a mut [T],
    pub(crate) compact_ring_indices: &'a mut [usize],
}

#[derive(Clone, Copy)]
struct TransitionPreflight {
    coefficients_safe: bool,
    sum_bound_safe: bool,
}

/// Packs active rings for one order, attempts the scalar-only transition
/// kernel, and stages each tile before scattering it to Fourier spectra.
#[expect(
    clippy::too_many_arguments,
    reason = "The scalar order kernel receives transform data and reusable worker scratch explicitly."
)]
pub(crate) fn synthesize_order_dispatched(
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
    scratch: &mut ScalarSynthesisFastScratch<'_, f64>,
) -> Result<bool, ShtError> {
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    debug_assert_eq!(sin_theta.len(), ring_order_limits.len());
    debug_assert_eq!(scratch.spectra.len(), sin_theta.len() * spectrum_len);
    if m > lmax {
        return Ok(false);
    }
    let pair_count = (lmax - m + 2) / 2;
    if scratch.previous.len() < ACTIVE_TILE_SIZE.min(sin_theta.len())
        || scratch.current.len() < ACTIVE_TILE_SIZE.min(sin_theta.len())
        || scratch.next.len() < ACTIVE_TILE_SIZE.min(sin_theta.len())
        || scratch.exponents.len() < ACTIVE_TILE_SIZE.min(sin_theta.len())
        || scratch.sum_re.len() < ACTIVE_TILE_SIZE.min(sin_theta.len())
        || scratch.sum_im.len() < ACTIVE_TILE_SIZE.min(sin_theta.len())
        || scratch.compact_theta_sin.len() < ACTIVE_TILE_SIZE.min(sin_theta.len())
        || scratch.compact_theta_cos.len() < ACTIVE_TILE_SIZE.min(sin_theta.len())
        || scratch.compact_ring_indices.len() < ACTIVE_TILE_SIZE.min(sin_theta.len())
        || coefficient_a.len() < pair_count
        || coefficient_b.len() < pair_count
        || coefficient_even.len() < pair_count
        || coefficient_odd.len() < pair_count
    {
        return Ok(false);
    }

    let mut preflight_cache = None;
    let mut tile_len = 0;
    for ring in 0..sin_theta.len() {
        if m > ring_order_limits[ring] {
            continue;
        }
        scratch.compact_theta_sin[tile_len] = sin_theta[ring];
        scratch.compact_theta_cos[tile_len] = cos_theta[ring];
        scratch.compact_ring_indices[tile_len] = ring;
        tile_len += 1;
        if tile_len == ACTIVE_TILE_SIZE {
            process_tile(
                arch,
                m,
                lmax,
                &mut preflight_cache,
                sectoral_norm,
                coefficient_a,
                coefficient_b,
                coefficient_even,
                coefficient_odd,
                spectrum_len,
                tile_len,
                scratch,
            )?;
            tile_len = 0;
        }
    }
    if tile_len > 0 {
        process_tile(
            arch,
            m,
            lmax,
            &mut preflight_cache,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            coefficient_even,
            coefficient_odd,
            spectrum_len,
            tile_len,
            scratch,
        )?;
    }
    Ok(true)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Tile processing is private and its inputs make staging and fallback ownership explicit."
)]
fn process_tile(
    arch: Arch,
    m: usize,
    lmax: usize,
    preflight_cache: &mut Option<TransitionPreflight>,
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    coefficient_even: &[Complex64],
    coefficient_odd: &[Complex64],
    spectrum_len: usize,
    tile_len: usize,
    scratch: &mut ScalarSynthesisFastScratch<'_, f64>,
) -> Result<(), ShtError> {
    #[cfg(test)]
    update_route_counts(|counts| counts.tiles += 1);
    // A single polar lane must not force otherwise ordinary lanes through the
    // scaled visitor. Partition the packed tile in place; ring indices travel
    // with their geometry so the final scatter remains correct.
    let mut ordinary_len = 0;
    let mut remaining = tile_len;
    while ordinary_len < remaining {
        let sine = scratch.compact_theta_sin[ordinary_len];
        if sine.is_finite() && sine >= 0.01 {
            ordinary_len += 1;
        } else {
            remaining -= 1;
            scratch.compact_theta_sin.swap(ordinary_len, remaining);
            scratch.compact_theta_cos.swap(ordinary_len, remaining);
            scratch.compact_ring_indices.swap(ordinary_len, remaining);
        }
    }
    let polar_len = tile_len - ordinary_len;
    #[cfg(test)]
    update_route_counts(|counts| {
        counts.polar_pair_lanes += polar_len as u64 * ((lmax - m + 2) / 2) as u64;
    });

    let fast = if ordinary_len > 0 {
        let preflight = *preflight_cache.get_or_insert_with(|| {
            make_transition_preflight(
                m,
                lmax,
                coefficient_a,
                coefficient_b,
                coefficient_even,
                coefficient_odd,
            )
        });
        try_transition_after_preflight(
            arch,
            m,
            lmax,
            preflight,
            &scratch.compact_theta_sin[..ordinary_len],
            &scratch.compact_theta_cos[..ordinary_len],
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            coefficient_even,
            coefficient_odd,
            &mut scratch.previous[..ordinary_len],
            &mut scratch.current[..ordinary_len],
            &mut scratch.next[..ordinary_len],
            &mut scratch.exponents[..ordinary_len],
            &mut scratch.sum_re[..ordinary_len],
            &mut scratch.sum_im[..ordinary_len],
        )
        .is_some()
    } else {
        false
    };

    #[cfg(test)]
    update_route_counts(|counts| {
        if fast {
            counts.completed_tiles += 1;
        } else {
            counts.declined_tiles += 1;
        }
    });

    if fast {
        scratch.sum_re[ordinary_len..tile_len].fill(0.0);
        scratch.sum_im[ordinary_len..tile_len].fill(0.0);
        if polar_len > 0 {
            accumulate_scaled_range(
                arch,
                m,
                lmax,
                &scratch.compact_theta_sin[ordinary_len..tile_len],
                &scratch.compact_theta_cos[ordinary_len..tile_len],
                sectoral_norm,
                coefficient_a,
                coefficient_b,
                coefficient_even,
                coefficient_odd,
                &mut scratch.previous[..polar_len],
                &mut scratch.current[..polar_len],
                &mut scratch.next[..polar_len],
                &mut scratch.exponents[..polar_len],
                &mut scratch.sum_re[ordinary_len..tile_len],
                &mut scratch.sum_im[ordinary_len..tile_len],
            )?;
            #[cfg(test)]
            update_route_counts(|counts| {
                counts.fallback_pair_lanes += polar_len as u64 * ((lmax - m + 2) / 2) as u64;
            });
        }
    } else {
        scratch.sum_re[..tile_len].fill(0.0);
        scratch.sum_im[..tile_len].fill(0.0);
        accumulate_scaled_range(
            arch,
            m,
            lmax,
            &scratch.compact_theta_sin[..tile_len],
            &scratch.compact_theta_cos[..tile_len],
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            coefficient_even,
            coefficient_odd,
            &mut scratch.previous[..tile_len],
            &mut scratch.current[..tile_len],
            &mut scratch.next[..tile_len],
            &mut scratch.exponents[..tile_len],
            &mut scratch.sum_re[..tile_len],
            &mut scratch.sum_im[..tile_len],
        )?;
        #[cfg(test)]
        update_route_counts(|counts| {
            counts.fallback_pair_lanes += tile_len as u64 * ((lmax - m + 2) / 2) as u64;
        });
    }

    for tile_ring in 0..tile_len {
        let ring = scratch.compact_ring_indices[tile_ring];
        scratch.spectra[ring * spectrum_len + m] +=
            Complex64::new(scratch.sum_re[tile_ring], scratch.sum_im[tile_ring]);
    }
    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "Scaled fallback receives one compact ring range and caller-owned worker scratch."
)]
fn accumulate_scaled_range(
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
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
    sum_re: &mut [f64],
    sum_im: &mut [f64],
) -> Result<(), ShtError> {
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    debug_assert_eq!(sin_theta.len(), sum_re.len());
    debug_assert_eq!(sin_theta.len(), sum_im.len());
    scalar_paired::for_each_y_lm_paired_dispatched(
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
        |l, ring_start, basis, lane_exponents| {
            let pair = (l - m) / 2;
            for lane in 0..basis.len() {
                let index = ring_start + lane;
                let even = scalar::multiply_complex_scaled(
                    coefficient_even[pair],
                    basis[lane],
                    lane_exponents[lane],
                );
                let odd = scalar::multiply_complex_scaled_product(
                    coefficient_odd[pair],
                    basis[lane],
                    cos_theta[index],
                    lane_exponents[lane],
                );
                sum_re[index] += even.re + odd.re;
                sum_im[index] += even.im + odd.im;
            }
            Ok(())
        },
    )
}

#[cfg(test)]
#[expect(
    clippy::too_many_arguments,
    reason = "The transition receives recurrence lanes, prepared coefficients and tile-local staging explicitly."
)]
fn try_transition_and_accumulate(
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
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
    sum_re: &mut [f64],
    sum_im: &mut [f64],
) -> Option<usize> {
    if sin_theta.is_empty() {
        #[cfg(test)]
        update_route_counts(|counts| counts.empty_tiles += 1);
        return None;
    }
    let preflight = make_transition_preflight(
        m,
        lmax,
        coefficient_a,
        coefficient_b,
        coefficient_even,
        coefficient_odd,
    );
    try_transition_after_preflight(
        arch,
        m,
        lmax,
        preflight,
        sin_theta,
        cos_theta,
        sectoral_norm,
        coefficient_a,
        coefficient_b,
        coefficient_even,
        coefficient_odd,
        previous,
        current,
        next,
        exponents,
        sum_re,
        sum_im,
    )
}

fn make_transition_preflight(
    m: usize,
    lmax: usize,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    coefficient_even: &[Complex64],
    coefficient_odd: &[Complex64],
) -> TransitionPreflight {
    #[cfg(test)]
    update_route_counts(|counts| counts.coefficient_preflights += 1);
    let pair_count = (lmax - m + 2) / 2;
    let coefficients_safe = ordinary_coefficients_are_safe(
        pair_count,
        coefficient_a,
        coefficient_b,
        coefficient_even,
        coefficient_odd,
    );
    let maximum_sum_bound = 2.0 * scalar::BIG * scalar::BIG * pair_count as f64 * 10_000.0;
    TransitionPreflight {
        coefficients_safe,
        sum_bound_safe: maximum_sum_bound.is_finite() && maximum_sum_bound <= f64::MAX * 0.5,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The transition receives recurrence lanes, prepared coefficients, preflight state, and tile-local staging explicitly."
)]
fn try_transition_after_preflight(
    arch: Arch,
    m: usize,
    lmax: usize,
    preflight: TransitionPreflight,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    coefficient_even: &[Complex64],
    coefficient_odd: &[Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
    sum_re: &mut [f64],
    sum_im: &mut [f64],
) -> Option<usize> {
    if sin_theta.is_empty() {
        #[cfg(test)]
        update_route_counts(|counts| counts.empty_tiles += 1);
        return None;
    }
    if !preflight.coefficients_safe {
        #[cfg(test)]
        update_route_counts(|counts| counts.unsafe_coefficients += 1);
        return None;
    }
    if !preflight.sum_bound_safe {
        #[cfg(test)]
        update_route_counts(|counts| counts.no_transition += 1);
        return None;
    }

    let transition_pair = arch.dispatch(StagedScalarSynthesis {
        m,
        lmax,
        sin_theta,
        cos_theta,
        sectoral_norm,
        coefficient_a,
        coefficient_b,
        coefficient_even,
        coefficient_odd,
        previous,
        current,
        next,
        exponents,
        sum_re,
        sum_im,
    });
    let valid = transition_pair.filter(|_| {
        sum_re
            .iter()
            .chain(sum_im.iter())
            .all(|value| value.is_finite() && value.abs() <= f64::MAX * 0.5)
    });
    #[cfg(test)]
    if valid.is_none() {
        update_route_counts(|counts| counts.no_transition += 1);
    }
    valid
}

#[inline]
fn ordinary_coefficients_are_safe(
    pair_count: usize,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    coefficient_even: &[Complex64],
    coefficient_odd: &[Complex64],
) -> bool {
    (0..pair_count).all(|pair| {
        scalar_value_in_normal_range(coefficient_a[pair])
            && scalar_value_in_normal_range(coefficient_b[pair])
            && complex_in_normal_range(coefficient_even[pair])
            && complex_in_normal_range(coefficient_odd[pair])
    })
}

#[inline]
fn complex_in_normal_range(value: Complex64) -> bool {
    scalar_value_in_normal_range(value.re) && scalar_value_in_normal_range(value.im)
}

#[inline(always)]
fn simd_values_are_finite<S: Simd>(simd: S, values: S::f64s) -> bool {
    let absolute = simd.abs_f64s(values);
    let ordered = simd.equal_f64s(values, values);
    let ordered_absolute = simd.select_f64s(ordered, absolute, simd.splat_f64s(f64::INFINITY));
    let finite_mask = simd.less_than_or_equal_f64s(ordered_absolute, simd.splat_f64s(f64::MAX));
    let finite = simd.select_f64s(finite_mask, simd.splat_f64s(1.0), simd.splat_f64s(0.0));
    simd.reduce_min_f64s(finite) == 1.0
}

struct StagedScalarSynthesis<'a> {
    m: usize,
    lmax: usize,
    sin_theta: &'a [f64],
    cos_theta: &'a [f64],
    sectoral_norm: f64,
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    coefficient_even: &'a [Complex64],
    coefficient_odd: &'a [Complex64],
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
    exponents: &'a mut [i64],
    sum_re: &'a mut [f64],
    sum_im: &'a mut [f64],
}

impl WithSimd for StagedScalarSynthesis<'_> {
    type Output = Option<usize>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Option<usize> {
        let Self {
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            coefficient_even,
            coefficient_odd,
            previous,
            current,
            next,
            exponents,
            sum_re,
            sum_im,
        } = self;
        let mut previous = previous;
        let mut current = current;
        let mut next = next;
        for lane in 0..sin_theta.len() {
            let (value, exponent) = match scalar::sectoral_value(sin_theta[lane], m, sectoral_norm)
            {
                Ok(value) => value,
                Err(_) => return None,
            };
            previous[lane] = 0.0;
            current[lane] = value;
            exponents[lane] = exponent;
            sum_re[lane] = 0.0;
            sum_im[lane] = 0.0;
        }
        let mut normal_range = match scalar::normalize_pair_lanes(previous, current, exponents) {
            Ok(normal_range) => normal_range,
            Err(_) => return None,
        };
        // Keep the prefix accumulator in the recurrence's scale. The fixed
        // active tile bound makes this small per-call state safe on the stack.
        let mut sum_exponents = [0_i64; ACTIVE_TILE_SIZE];
        sum_exponents[..sin_theta.len()].copy_from_slice(&exponents[..sin_theta.len()]);
        if normal_range
            && lane_values_in_normal_range(previous)
            && lane_values_in_normal_range(current)
        {
            FusedScalarSuffix {
                m,
                lmax,
                pair_start: 0,
                cos_theta,
                coefficient_a,
                coefficient_b,
                coefficient_even,
                coefficient_odd,
                previous,
                current,
                sum_re,
                sum_im,
            }
            .with_simd(simd);
            return Some(0);
        }

        let pair_count = (lmax - m + 2) / 2;
        for pair in 0..pair_count {
            #[cfg(test)]
            update_route_counts(|counts| counts.prefix_pair_lanes += current.len() as u64);
            let mut normalize = pair > 0 && !normal_range;
            if pair > 0 && normal_range {
                normalize = scalar::pair_lanes_need_scaling(simd, previous, current);
                if !normalize
                    && lane_values_in_normal_range(previous)
                    && lane_values_in_normal_range(current)
                {
                    if !materialize_scaled_accumulator(
                        sum_re,
                        sum_im,
                        &sum_exponents[..sin_theta.len()],
                    ) {
                        return None;
                    }
                    FusedScalarSuffix {
                        m,
                        lmax,
                        pair_start: pair,
                        cos_theta,
                        coefficient_a,
                        coefficient_b,
                        coefficient_even,
                        coefficient_odd,
                        previous,
                        current,
                        sum_re,
                        sum_im,
                    }
                    .with_simd(simd);
                    return Some(pair);
                }
            }
            let mut normal_after = if normalize { true } else { normal_range };
            let even = coefficient_even[pair];
            let odd = coefficient_odd[pair];
            for lane in 0..current.len() {
                if normalize {
                    let lane_normal = match scalar::normalize_pair_lane(
                        &mut previous[lane],
                        &mut current[lane],
                        &mut exponents[lane],
                    ) {
                        Ok(normal) => normal,
                        Err(_) => return None,
                    };
                    normal_after &= lane_normal;
                }
                let current_value = current[lane];
                if !scalar_value_in_normal_range(current_value) {
                    return None;
                }
                if sum_exponents[lane] != exponents[lane]
                    && !rebase_accumulator_lane(
                        &mut sum_re[lane],
                        &mut sum_im[lane],
                        &mut sum_exponents[lane],
                        exponents[lane],
                    )
                {
                    return None;
                }
            }

            // The scalar prefix still tracks and rebases each lane's scale.
            // Factor formation and accumulation then run over contiguous
            // ordinary lanes with SIMD; unsafe values decline the tile before
            // it is scattered, so the scaled visitor can recompute it.
            let (cos_vectors, cos_tail) = S::as_simd_f64s(cos_theta);
            let (current_vectors, current_tail) = S::as_simd_f64s(current);
            let (sum_re_vectors, sum_re_tail) = S::as_mut_simd_f64s(sum_re);
            let (sum_im_vectors, sum_im_tail) = S::as_mut_simd_f64s(sum_im);
            debug_assert_eq!(cos_vectors.len(), current_vectors.len());
            debug_assert_eq!(cos_tail.len(), current_tail.len());
            debug_assert_eq!(cos_vectors.len(), sum_re_vectors.len());
            debug_assert_eq!(cos_vectors.len(), sum_im_vectors.len());
            debug_assert_eq!(cos_tail.len(), sum_re_tail.len());
            debug_assert_eq!(cos_tail.len(), sum_im_tail.len());

            for vector_index in 0..cos_vectors.len() {
                let cosine = cos_vectors[vector_index];
                let current_value = current_vectors[vector_index];
                let factor_re =
                    simd.mul_add_f64s(simd.splat_f64s(odd.re), cosine, simd.splat_f64s(even.re));
                let factor_im =
                    simd.mul_add_f64s(simd.splat_f64s(odd.im), cosine, simd.splat_f64s(even.im));
                if !simd_values_in_normal_range(simd, factor_re)
                    || !simd_values_in_normal_range(simd, factor_im)
                {
                    return None;
                }
                let next_re = simd.add_f64s(
                    simd.mul_f64s(factor_re, current_value),
                    sum_re_vectors[vector_index],
                );
                let next_im = simd.add_f64s(
                    simd.mul_f64s(factor_im, current_value),
                    sum_im_vectors[vector_index],
                );
                if !simd_values_are_finite(simd, next_re) || !simd_values_are_finite(simd, next_im)
                {
                    return None;
                }
                sum_re_vectors[vector_index] = next_re;
                sum_im_vectors[vector_index] = next_im;
            }
            for lane in 0..cos_tail.len() {
                let cosine = cos_tail[lane];
                let current_value = current_tail[lane];
                let factor_re = odd.re.mul_add(cosine, even.re);
                let factor_im = odd.im.mul_add(cosine, even.im);
                if !scalar_value_in_normal_range(current_value)
                    || !scalar_value_in_normal_range(factor_re)
                    || !scalar_value_in_normal_range(factor_im)
                {
                    return None;
                }
                let next_re = sum_re_tail[lane] + factor_re * current_value;
                let next_im = sum_im_tail[lane] + factor_im * current_value;
                if !next_re.is_finite() || !next_im.is_finite() {
                    return None;
                }
                sum_re_tail[lane] = next_re;
                sum_im_tail[lane] = next_im;
            }
            if normalize {
                normal_range = normal_after;
            }
            if pair + 1 == pair_count {
                break;
            }

            advance_recurrence(
                simd,
                pair,
                cos_theta,
                coefficient_a,
                coefficient_b,
                previous,
                current,
                next,
            );
            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
        }
        None
    }
}

#[inline(always)]
#[expect(
    clippy::too_many_arguments,
    reason = "The SIMD recurrence update receives lane inputs and its three recurrence buffers explicitly."
)]
fn advance_recurrence<S: Simd>(
    simd: S,
    pair: usize,
    cos_theta: &[f64],
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) {
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
}

#[inline]
fn safe_rebased_accumulator(previous: f64, rebased: f64) -> bool {
    rebased.is_finite() && (previous == 0.0 || rebased.is_normal())
}

#[inline]
fn rebase_accumulator_lane(
    sum_re: &mut f64,
    sum_im: &mut f64,
    sum_exponent: &mut i64,
    target_exponent: i64,
) -> bool {
    let Some(scale) = sum_exponent.checked_sub(target_exponent) else {
        return false;
    };
    let re = scalar::scale_binary(*sum_re, scale);
    let im = scalar::scale_binary(*sum_im, scale);
    if !safe_rebased_accumulator(*sum_re, re) || !safe_rebased_accumulator(*sum_im, im) {
        return false;
    }
    *sum_re = re;
    *sum_im = im;
    *sum_exponent = target_exponent;
    true
}

fn materialize_scaled_accumulator(
    sum_re: &mut [f64],
    sum_im: &mut [f64],
    sum_exponents: &[i64],
) -> bool {
    debug_assert_eq!(sum_re.len(), sum_im.len());
    debug_assert_eq!(sum_re.len(), sum_exponents.len());
    for lane in 0..sum_re.len() {
        let previous_re = sum_re[lane];
        let previous_im = sum_im[lane];
        let re = scalar::scale_binary(previous_re, sum_exponents[lane]);
        let im = scalar::scale_binary(previous_im, sum_exponents[lane]);
        if !safe_rebased_accumulator(previous_re, re) || !safe_rebased_accumulator(previous_im, im)
        {
            return false;
        }
        sum_re[lane] = re;
        sum_im[lane] = im;
    }
    true
}

struct FusedScalarSuffix<'a> {
    m: usize,
    lmax: usize,
    pair_start: usize,
    cos_theta: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    coefficient_even: &'a [Complex64],
    coefficient_odd: &'a [Complex64],
    previous: &'a [f64],
    current: &'a [f64],
    sum_re: &'a mut [f64],
    sum_im: &'a mut [f64],
}

impl WithSimd for FusedScalarSuffix<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) {
        let Self {
            m,
            lmax,
            pair_start,
            cos_theta,
            coefficient_a,
            coefficient_b,
            coefficient_even,
            coefficient_odd,
            previous: initial_previous,
            current: initial_current,
            sum_re,
            sum_im,
        } = self;
        let (cos_vectors, cos_tail) = S::as_simd_f64s(cos_theta);
        let (previous_vectors, previous_tail) = S::as_simd_f64s(initial_previous);
        let (current_vectors, current_tail) = S::as_simd_f64s(initial_current);
        let (sum_re_vectors, sum_re_tail) = S::as_mut_simd_f64s(sum_re);
        let (sum_im_vectors, sum_im_tail) = S::as_mut_simd_f64s(sum_im);
        let pair_count = (lmax - m + 2) / 2;
        #[cfg(test)]
        update_route_counts(|counts| {
            counts.suffix_pair_lanes +=
                cos_theta.len() as u64 * pair_count.saturating_sub(pair_start) as u64;
        });
        for lane_block in 0..cos_vectors.len() {
            let cosine = cos_vectors[lane_block];
            let cosine_squared = simd.mul_f64s(cosine, cosine);
            let mut previous = previous_vectors[lane_block];
            let mut current = current_vectors[lane_block];
            let mut accumulated_re = sum_re_vectors[lane_block];
            let mut accumulated_im = sum_im_vectors[lane_block];
            for pair in pair_start..pair_count {
                let even = coefficient_even[pair];
                let odd = coefficient_odd[pair];
                let factor_re =
                    simd.mul_add_f64s(simd.splat_f64s(odd.re), cosine, simd.splat_f64s(even.re));
                let factor_im =
                    simd.mul_add_f64s(simd.splat_f64s(odd.im), cosine, simd.splat_f64s(even.im));
                accumulated_re = simd.mul_add_f64s(current, factor_re, accumulated_re);
                accumulated_im = simd.mul_add_f64s(current, factor_im, accumulated_im);
                if pair + 1 < pair_count {
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
        for lane in 0..cos_tail.len() {
            let cosine = cos_tail[lane];
            let mut previous = previous_tail[lane];
            let mut current = current_tail[lane];
            let mut accumulated_re = sum_re_tail[lane];
            let mut accumulated_im = sum_im_tail[lane];
            for pair in pair_start..pair_count {
                let even = coefficient_even[pair];
                let odd = coefficient_odd[pair];
                let factor_re = odd.re.mul_add(cosine, even.re);
                let factor_im = odd.im.mul_add(cosine, even.im);
                accumulated_re = current.mul_add(factor_re, accumulated_re);
                accumulated_im = current.mul_add(factor_im, accumulated_im);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{GridKind, GridSpec};
    use crate::legendre::LATITUDE_BLOCK_SIZE;

    type PreparedOrder = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<Complex64>, Vec<Complex64>);

    fn prepared_order(m: usize, lmax: usize, coefficients: &[Complex64]) -> PreparedOrder {
        let mut epsilon = vec![0.0; lmax + 4];
        scalar::fill_epsilon(m, lmax, &mut epsilon);
        let factor_count = lmax / 2 + 2;
        let mut alpha = vec![0.0; factor_count];
        let mut coefficient_a = vec![0.0; factor_count];
        let mut coefficient_b = vec![0.0; factor_count];
        scalar_paired::fill_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        );
        let pair_count = (lmax - m + 2) / 2;
        let mut coefficient_even = vec![Complex64::new(0.0, 0.0); pair_count];
        let mut coefficient_odd = vec![Complex64::new(0.0, 0.0); pair_count];
        assert!(scalar_paired::prepare_synthesis_coefficients(
            m,
            lmax,
            &epsilon,
            &alpha,
            coefficients,
            &mut coefficient_even,
            &mut coefficient_odd,
        ));
        (
            epsilon,
            coefficient_a,
            coefficient_b,
            coefficient_even,
            coefficient_odd,
        )
    }

    fn coefficient_row(m: usize, lmax: usize) -> Vec<Complex64> {
        (0..=lmax - m)
            .map(|index| {
                let re = (index.wrapping_mul(17) % 61) as f64 - 30.0;
                let im = (index.wrapping_mul(29) % 47) as f64 - 23.0;
                Complex64::new(re * 1.0e-5, im * 1.0e-5)
            })
            .collect()
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The independent test reference receives one order and its expected spectrum explicitly."
    )]
    fn paired_reference(
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
    ) {
        let mut previous = [0.0; LATITUDE_BLOCK_SIZE];
        let mut current = [0.0; LATITUDE_BLOCK_SIZE];
        let mut next = [0.0; LATITUDE_BLOCK_SIZE];
        let mut exponents = [0_i64; LATITUDE_BLOCK_SIZE];
        scalar_paired::for_each_y_lm_paired_dispatched(
            Arch::new(),
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
            |l, ring_start, basis, lane_exponents| {
                let pair = (l - m) / 2;
                for lane in 0..basis.len() {
                    let ring = ring_start + lane;
                    if m > ring_order_limits[ring] {
                        continue;
                    }
                    spectra[ring * spectrum_len + m] += scalar::multiply_complex_scaled(
                        coefficient_even[pair],
                        basis[lane],
                        lane_exponents[lane],
                    ) + scalar::multiply_complex_scaled_product(
                        coefficient_odd[pair],
                        basis[lane],
                        cos_theta[ring],
                        lane_exponents[lane],
                    );
                }
                Ok(())
            },
        )
        .expect("the paired reference path is valid for these tiles");
    }

    fn assert_spectra_close(actual: &[Complex64], expected: &[Complex64]) {
        let mut error_squared = 0.0;
        let mut reference_squared = 0.0;
        let mut max_absolute = 0.0_f64;
        for (&actual, &expected) in actual.iter().zip(expected) {
            let difference = actual - expected;
            error_squared += difference.norm_sqr();
            reference_squared += expected.norm_sqr();
            max_absolute = max_absolute.max(difference.norm());
        }
        let relative_l2 = error_squared.sqrt() / reference_squared.sqrt().max(1.0e-300);
        assert!(
            relative_l2 <= 2.0e-11 && max_absolute <= 2.0e-12,
            "relative_l2={relative_l2:e}, max_absolute={max_absolute:e}"
        );
    }

    struct TestScratch {
        previous: Vec<f64>,
        current: Vec<f64>,
        next: Vec<f64>,
        exponents: Vec<i64>,
        sum_re: Vec<f64>,
        sum_im: Vec<f64>,
        compact_theta_sin: Vec<f64>,
        compact_theta_cos: Vec<f64>,
        compact_ring_indices: Vec<usize>,
    }

    impl TestScratch {
        fn new(ntheta: usize) -> Self {
            let scratch_len = ntheta.min(ACTIVE_TILE_SIZE);
            Self {
                previous: vec![0.0; scratch_len],
                current: vec![0.0; scratch_len],
                next: vec![0.0; scratch_len],
                exponents: vec![0_i64; scratch_len],
                sum_re: vec![0.0; scratch_len],
                sum_im: vec![0.0; scratch_len],
                compact_theta_sin: vec![0.0; scratch_len],
                compact_theta_cos: vec![0.0; scratch_len],
                compact_ring_indices: vec![0; scratch_len],
            }
        }

        fn borrow<'a>(
            &'a mut self,
            spectra: &'a mut [Complex64],
        ) -> ScalarSynthesisFastScratch<'a, f64> {
            ScalarSynthesisFastScratch {
                spectra,
                previous: &mut self.previous,
                current: &mut self.current,
                next: &mut self.next,
                exponents: &mut self.exponents,
                sum_re: &mut self.sum_re,
                sum_im: &mut self.sum_im,
                compact_theta_sin: &mut self.compact_theta_sin,
                compact_theta_cos: &mut self.compact_theta_cos,
                compact_ring_indices: &mut self.compact_ring_indices,
            }
        }
    }

    fn fast_scratch<'a>(
        owner: &'a mut TestScratch,
        spectra: &'a mut [Complex64],
    ) -> ScalarSynthesisFastScratch<'a, f64> {
        owner.borrow(spectra)
    }

    #[test]
    fn coefficient_safety_preflight_is_shared_across_active_tiles() {
        let m = 0;
        let lmax = 255;
        let sin_theta = vec![0.5; 300];
        let cos_theta = vec![(1.0_f64 - 0.5 * 0.5).sqrt(); sin_theta.len()];
        let ring_order_limits = vec![usize::MAX; sin_theta.len()];
        let coefficients = coefficient_row(m, lmax);
        let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
        let (_, coefficient_a, coefficient_b, coefficient_even, coefficient_odd) =
            prepared_order(m, lmax, &coefficients);
        let mut candidate = vec![Complex64::new(0.0, 0.0); sin_theta.len()];
        FAST_ROUTE_COUNTS.with_borrow_mut(|counts| *counts = Some(FastRouteCounts::default()));
        {
            let mut owner = TestScratch::new(sin_theta.len());
            let mut scratch = fast_scratch(&mut owner, &mut candidate);
            assert!(
                synthesize_order_dispatched(
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
                    1,
                    &mut scratch,
                )
                .unwrap()
            );
        }
        let counts = FAST_ROUTE_COUNTS.with_borrow_mut(Option::take).unwrap();
        assert_eq!(counts.tiles, 3);
        assert_eq!(counts.coefficient_preflights, 1);

        let mut reference = vec![Complex64::new(0.0, 0.0); sin_theta.len()];
        paired_reference(
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
            1,
            &mut reference,
        );
        assert_spectra_close(&candidate, &reference);
    }

    #[test]
    fn scaled_accumulator_rebases_exactly_and_rejects_unsafe_ranges() {
        let mut sum_re = 0.75;
        let mut sum_im = -0.625;
        let mut sum_exponent = 800;
        let expected_re = scalar::scale_binary(sum_re, sum_exponent);
        let expected_im = scalar::scale_binary(sum_im, sum_exponent);
        assert!(rebase_accumulator_lane(
            &mut sum_re,
            &mut sum_im,
            &mut sum_exponent,
            400
        ));
        assert_eq!(scalar::scale_binary(sum_re, sum_exponent), expected_re);
        assert_eq!(scalar::scale_binary(sum_im, sum_exponent), expected_im);

        let state_before_failure = (sum_re, sum_im, sum_exponent);
        assert!(!rebase_accumulator_lane(
            &mut sum_re,
            &mut sum_im,
            &mut sum_exponent,
            -800
        ));
        assert_eq!((sum_re, sum_im, sum_exponent), state_before_failure);
    }

    #[test]
    fn active_ring_tiles_match_paired_reference_with_tail_and_polar_filter() {
        let m = 200;
        let lmax = 255;
        let grid = GridSpec::new(GridKind::GaussLegendre, 513, 1, 0.0).unwrap();
        let sin_theta: Vec<_> = grid.theta().iter().map(|theta| theta.sin()).collect();
        let cos_theta: Vec<_> = grid.theta().iter().map(|theta| theta.cos()).collect();
        let ring_order_limits: Vec<_> = sin_theta
            .iter()
            .zip(&cos_theta)
            .map(|(&sine, &cosine)| scalar::polar_order_limit(lmax, 0, sine, cosine))
            .collect();
        let coefficients = coefficient_row(m, lmax);
        let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
        let (_, coefficient_a, coefficient_b, coefficient_even, coefficient_odd) =
            prepared_order(m, lmax, &coefficients);
        let spectrum_len = m + 1;
        let mut candidate = vec![Complex64::new(0.0, 0.0); sin_theta.len() * spectrum_len];
        let mut reference = candidate.clone();
        {
            let mut owner = TestScratch::new(sin_theta.len());
            let mut scratch = fast_scratch(&mut owner, &mut candidate);
            assert!(
                synthesize_order_dispatched(
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
                    spectrum_len,
                    &mut scratch,
                )
                .unwrap()
            );
        }
        paired_reference(
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
            spectrum_len,
            &mut reference,
        );
        assert_spectra_close(&candidate, &reference);
    }

    #[test]
    fn scaled_prefix_transitions_to_the_ordinary_suffix() {
        let lmax = 2047;
        let arch = Arch::new();
        let mut observed = Vec::new();
        for (m, sine) in [
            (600, 0.30_f64),
            (790, 0.40),
            (900, 0.45),
            (925, 0.46),
            (998, 0.49),
        ] {
            let sin_theta = vec![sine; 8];
            let cosine = (1.0 - sine * sine).sqrt();
            let cos_theta = vec![cosine; sin_theta.len()];
            let coefficients = coefficient_row(m, lmax);
            let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
            let (_, coefficient_a, coefficient_b, coefficient_even, coefficient_odd) =
                prepared_order(m, lmax, &coefficients);
            let mut previous = vec![0.0; sin_theta.len()];
            let mut current = vec![0.0; sin_theta.len()];
            let mut next = vec![0.0; sin_theta.len()];
            let mut exponents = vec![0_i64; sin_theta.len()];
            let mut sum_re = vec![0.0; sin_theta.len()];
            let mut sum_im = vec![0.0; sin_theta.len()];
            let transition_pair = try_transition_and_accumulate(
                arch,
                m,
                lmax,
                &sin_theta,
                &cos_theta,
                sectoral_norm,
                &coefficient_a,
                &coefficient_b,
                &coefficient_even,
                &coefficient_odd,
                &mut previous,
                &mut current,
                &mut next,
                &mut exponents,
                &mut sum_re,
                &mut sum_im,
            );
            observed.push((m, sine, transition_pair));
            if let Some(pair_start) = transition_pair
                && pair_start > 0
            {
                let spectrum_len = m + 1;
                let mut candidate = vec![Complex64::new(0.0, 0.0); sin_theta.len() * spectrum_len];
                for ring in 0..sin_theta.len() {
                    candidate[ring * spectrum_len + m] = Complex64::new(sum_re[ring], sum_im[ring]);
                }
                let mut reference = vec![Complex64::new(0.0, 0.0); candidate.len()];
                paired_reference(
                    m,
                    lmax,
                    &sin_theta,
                    &cos_theta,
                    sectoral_norm,
                    &coefficient_a,
                    &coefficient_b,
                    &coefficient_even,
                    &coefficient_odd,
                    &vec![usize::MAX; sin_theta.len()],
                    spectrum_len,
                    &mut reference,
                );
                assert_spectra_close(&candidate, &reference);
                return;
            }
        }
        panic!("no scaled-prefix transition found in {observed:?}");
    }

    #[test]
    fn polar_fallback_stages_reference_before_a_single_scatter() {
        let m = 0;
        let lmax = 255;
        let sin_theta = vec![0.005, 0.006, 0.007, 0.008, 0.009];
        let cos_theta: Vec<_> = sin_theta
            .iter()
            .map(|&sine: &f64| (1.0 - sine * sine).sqrt())
            .collect();
        let ring_order_limits = vec![usize::MAX; sin_theta.len()];
        let coefficients = coefficient_row(m, lmax);
        let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
        let (_, coefficient_a, coefficient_b, coefficient_even, coefficient_odd) =
            prepared_order(m, lmax, &coefficients);
        let spectrum_len = m + 1;
        let sentinel = Complex64::new(0.25, -0.5);
        let mut candidate = vec![Complex64::new(0.0, 0.0); sin_theta.len() * spectrum_len];
        let mut reference = candidate.clone();
        for ring in 0..sin_theta.len() {
            candidate[ring * spectrum_len + m] = sentinel;
            reference[ring * spectrum_len + m] = sentinel;
        }
        {
            let mut owner = TestScratch::new(sin_theta.len());
            let mut scratch = fast_scratch(&mut owner, &mut candidate);
            assert!(
                synthesize_order_dispatched(
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
                    spectrum_len,
                    &mut scratch,
                )
                .unwrap()
            );
        }
        paired_reference(
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
            spectrum_len,
            &mut reference,
        );
        assert_spectra_close(&candidate, &reference);
    }

    #[test]
    fn mixed_tile_keeps_polar_lanes_on_scaled_fallback() {
        let m = 0;
        let lmax = 255;
        let sin_theta = vec![0.005, 0.02, 0.15, 0.5, 0.99];
        let cos_theta: Vec<_> = sin_theta
            .iter()
            .map(|&sine: &f64| (1.0 - sine * sine).sqrt())
            .collect();
        let ring_order_limits = vec![usize::MAX; sin_theta.len()];
        let coefficients = coefficient_row(m, lmax);
        let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
        let (_, coefficient_a, coefficient_b, coefficient_even, coefficient_odd) =
            prepared_order(m, lmax, &coefficients);
        let spectrum_len = m + 1;
        let sentinel = Complex64::new(0.25, -0.5);
        let mut candidate = vec![Complex64::new(0.0, 0.0); sin_theta.len() * spectrum_len];
        let mut reference = candidate.clone();
        for ring in 0..sin_theta.len() {
            candidate[ring * spectrum_len + m] = sentinel;
            reference[ring * spectrum_len + m] = sentinel;
        }
        {
            let mut owner = TestScratch::new(sin_theta.len());
            let mut scratch = fast_scratch(&mut owner, &mut candidate);
            assert!(
                synthesize_order_dispatched(
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
                    spectrum_len,
                    &mut scratch,
                )
                .unwrap()
            );
        }
        paired_reference(
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
            spectrum_len,
            &mut reference,
        );
        assert_spectra_close(&candidate, &reference);
    }

    #[test]
    fn extreme_finite_prepared_coefficient_matches_scaled_reference() {
        let m = 0;
        let lmax = 511;
        let grid = GridSpec::new(GridKind::GaussLegendre, lmax + 2, 1, 0.0).unwrap();
        let sin_theta: Vec<_> = grid.theta().iter().map(|theta| theta.sin()).collect();
        let cos_theta: Vec<_> = grid.theta().iter().map(|theta| theta.cos()).collect();
        let ring_order_limits: Vec<_> = sin_theta
            .iter()
            .zip(&cos_theta)
            .map(|(&sine, &cosine)| scalar::polar_order_limit(lmax, 0, sine, cosine))
            .collect();
        let coefficients = vec![Complex64::new(0.0, 0.0); lmax + 1];
        let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
        let (_, coefficient_a, coefficient_b, mut coefficient_even, coefficient_odd) =
            prepared_order(m, lmax, &coefficients);
        coefficient_even[0] = Complex64::new(1.0e300, 0.0);
        assert!(!ordinary_coefficients_are_safe(
            (lmax - m + 2) / 2,
            &coefficient_a,
            &coefficient_b,
            &coefficient_even,
            &coefficient_odd,
        ));
        let spectrum_len = m + 1;
        let mut candidate = vec![Complex64::new(0.0, 0.0); sin_theta.len() * spectrum_len];
        let mut reference = candidate.clone();
        {
            let mut owner = TestScratch::new(sin_theta.len());
            let mut scratch = fast_scratch(&mut owner, &mut candidate);
            assert!(
                synthesize_order_dispatched(
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
                    spectrum_len,
                    &mut scratch,
                )
                .unwrap()
            );
        }
        paired_reference(
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
            spectrum_len,
            &mut reference,
        );
        assert!(
            candidate
                .iter()
                .all(|value| value.re.is_finite() && value.im.is_finite())
        );
        assert_spectra_close(&candidate, &reference);
    }

    #[cfg(feature = "parallel")]
    #[test]
    #[ignore = "opt-in M6 isolated fast-route accounting"]
    fn m6_fast_route_counts() {
        use crate::{GridKind, GridSpec, HarmonicSpec, ShtPlan};

        let lmax: usize = std::env::var("RUSTSHT_FAST_ROUTE_LMAX")
            .unwrap_or_else(|_| "511".into())
            .parse()
            .unwrap();
        let threads: usize = std::env::var("RUSTSHT_FAST_ROUTE_THREADS")
            .unwrap_or_else(|_| "1".into())
            .parse()
            .unwrap();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        let plan = ShtPlan::new(
            GridSpec::new(GridKind::GaussLegendre, lmax + 1, 2 * (lmax + 1), 0.37).unwrap(),
            HarmonicSpec::scalar(lmax, lmax).unwrap(),
        )
        .unwrap();
        let mut workspace = pool.install(|| plan.workspace()).unwrap();
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        for m in 0..=lmax {
            for l in m..=lmax {
                alm[plan.harmonics().index(l, m).unwrap()] = Complex64::new(
                    ((l * 7 + m * 11) % 19) as f64 - 9.0,
                    if m == 0 {
                        0.0
                    } else {
                        ((l * 13 + m * 5) % 23) as f64 - 11.0
                    },
                ) * 1e-5;
            }
        }
        let mut map = vec![0.0; (lmax + 1) * 2 * (lmax + 1)];
        pool.install(|| plan.synthesis_into(&alm, &mut map, &mut workspace))
            .unwrap();
        let reference = map.clone();
        pool.broadcast(|_| {
            FAST_ROUTE_COUNTS.with_borrow_mut(|counts| *counts = Some(FastRouteCounts::default()))
        });
        pool.install(|| plan.synthesis_into(&alm, &mut map, &mut workspace))
            .unwrap();
        assert_eq!(map, reference, "route accounting must preserve output bits");
        let workers = pool.broadcast(|_| FAST_ROUTE_COUNTS.with_borrow_mut(Option::take));
        let mut total = FastRouteCounts::default();
        for (worker, counts) in workers.into_iter().enumerate() {
            let counts = counts.unwrap();
            println!("fast_worker={worker} {counts:?}");
            total.tiles += counts.tiles;
            total.completed_tiles += counts.completed_tiles;
            total.declined_tiles += counts.declined_tiles;
            total.empty_tiles += counts.empty_tiles;
            total.coefficient_preflights += counts.coefficient_preflights;
            total.unsafe_coefficients += counts.unsafe_coefficients;
            total.no_transition += counts.no_transition;
            total.polar_pair_lanes += counts.polar_pair_lanes;
            total.prefix_pair_lanes += counts.prefix_pair_lanes;
            total.suffix_pair_lanes += counts.suffix_pair_lanes;
            total.fallback_pair_lanes += counts.fallback_pair_lanes;
        }
        println!("M6 fast routes L={lmax} q={threads} {total:?}");
        assert_eq!(total.tiles, total.completed_tiles + total.declined_tiles);
    }
}
