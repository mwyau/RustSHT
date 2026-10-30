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
use crate::legendre::OrderedSimdPrecision;

#[cfg(test)]
pub(super) const SCALE_UP: f64 = f64::from_bits(1823_u64 << 52);
#[cfg(test)]
pub(super) const SCALE_DOWN: f64 = f64::from_bits(223_u64 << 52);
#[cfg(test)]
pub(super) const FTOL: f64 = f64::from_bits((1023_u64 - 60) << 52);

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WorkCounters {
    pub(crate) ring_batches: u64,
    pub(crate) output_ring_writes: u64,
    pub(crate) initial_scaled_lanes: u64,
    pub(crate) scaled_batches: u64,
    pub(crate) scaled_pair_updates: u64,
    pub(crate) uniform_scale_updates: u64,
    pub(crate) scale_negative_one_lanes: u64,
    pub(crate) scale_zero_lanes: u64,
    pub(crate) scale_positive_one_lanes: u64,
    pub(crate) scale_other_lanes: u64,
    pub(crate) shared_basis_pair_updates: u64,
    pub(crate) shared_basis_fallback_pair_updates: u64,
    pub(crate) max_scaled_prefix_pairs: u64,
    pub(crate) ordinary_pair_updates: u64,
    pub(crate) ordered_route_successes: u64,
    pub(crate) ordered_route_declines: u64,
}

#[cfg(test)]
mod work_counters {
    use super::WorkCounters;
    use std::sync::atomic::{AtomicU64, Ordering};

    static RING_BATCHES: AtomicU64 = AtomicU64::new(0);
    static OUTPUT_RING_WRITES: AtomicU64 = AtomicU64::new(0);
    static INITIAL_SCALED_LANES: AtomicU64 = AtomicU64::new(0);
    static SCALED_BATCHES: AtomicU64 = AtomicU64::new(0);
    static SCALED_PAIR_UPDATES: AtomicU64 = AtomicU64::new(0);
    static UNIFORM_SCALE_UPDATES: AtomicU64 = AtomicU64::new(0);
    static SCALE_NEGATIVE_ONE_LANES: AtomicU64 = AtomicU64::new(0);
    static SCALE_ZERO_LANES: AtomicU64 = AtomicU64::new(0);
    static SCALE_POSITIVE_ONE_LANES: AtomicU64 = AtomicU64::new(0);
    static SCALE_OTHER_LANES: AtomicU64 = AtomicU64::new(0);
    static SHARED_BASIS_PAIR_UPDATES: AtomicU64 = AtomicU64::new(0);
    static SHARED_BASIS_FALLBACK_PAIR_UPDATES: AtomicU64 = AtomicU64::new(0);
    static MAX_SCALED_PREFIX_PAIRS: AtomicU64 = AtomicU64::new(0);
    static ORDINARY_PAIR_UPDATES: AtomicU64 = AtomicU64::new(0);
    static ORDERED_ROUTE_SUCCESSES: AtomicU64 = AtomicU64::new(0);
    static ORDERED_ROUTE_DECLINES: AtomicU64 = AtomicU64::new(0);

    pub(super) fn reset() {
        for counter in [
            &RING_BATCHES,
            &OUTPUT_RING_WRITES,
            &INITIAL_SCALED_LANES,
            &SCALED_BATCHES,
            &SCALED_PAIR_UPDATES,
            &UNIFORM_SCALE_UPDATES,
            &SCALE_NEGATIVE_ONE_LANES,
            &SCALE_ZERO_LANES,
            &SCALE_POSITIVE_ONE_LANES,
            &SCALE_OTHER_LANES,
            &SHARED_BASIS_PAIR_UPDATES,
            &SHARED_BASIS_FALLBACK_PAIR_UPDATES,
            &MAX_SCALED_PREFIX_PAIRS,
            &ORDINARY_PAIR_UPDATES,
            &ORDERED_ROUTE_SUCCESSES,
            &ORDERED_ROUTE_DECLINES,
        ] {
            counter.store(0, Ordering::Relaxed);
        }
    }

    pub(super) fn record_ring_batch(output_ring_writes: usize) {
        RING_BATCHES.fetch_add(1, Ordering::Relaxed);
        OUTPUT_RING_WRITES.fetch_add(output_ring_writes as u64, Ordering::Relaxed);
    }

    pub(super) fn record_initial_scaled_lanes(count: usize) {
        if count != 0 {
            INITIAL_SCALED_LANES.fetch_add(count as u64, Ordering::Relaxed);
            SCALED_BATCHES.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(super) fn record_scaled_pair_update(lanes: usize, prefix_pairs: usize, scale: &[i64]) {
        SCALED_PAIR_UPDATES.fetch_add(lanes as u64, Ordering::Relaxed);
        MAX_SCALED_PREFIX_PAIRS.fetch_max(prefix_pairs as u64, Ordering::Relaxed);
        if scale
            .first()
            .is_some_and(|&first| scale.iter().all(|&value| value == first))
        {
            UNIFORM_SCALE_UPDATES.fetch_add(1, Ordering::Relaxed);
        }
        for &value in scale {
            let counter = match value {
                -1 => &SCALE_NEGATIVE_ONE_LANES,
                0 => &SCALE_ZERO_LANES,
                1 => &SCALE_POSITIVE_ONE_LANES,
                _ => &SCALE_OTHER_LANES,
            };
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(super) fn record_shared_basis_path(shared: bool) {
        let counter = if shared {
            &SHARED_BASIS_PAIR_UPDATES
        } else {
            &SHARED_BASIS_FALLBACK_PAIR_UPDATES
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn record_ordinary_pair_updates(updates: usize) {
        ORDINARY_PAIR_UPDATES.fetch_add(updates as u64, Ordering::Relaxed);
    }

    pub(super) fn record_ordered_route(success: bool) {
        if success {
            ORDERED_ROUTE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
        } else {
            ORDERED_ROUTE_DECLINES.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(super) fn snapshot() -> WorkCounters {
        WorkCounters {
            ring_batches: RING_BATCHES.load(Ordering::Relaxed),
            output_ring_writes: OUTPUT_RING_WRITES.load(Ordering::Relaxed),
            initial_scaled_lanes: INITIAL_SCALED_LANES.load(Ordering::Relaxed),
            scaled_batches: SCALED_BATCHES.load(Ordering::Relaxed),
            scaled_pair_updates: SCALED_PAIR_UPDATES.load(Ordering::Relaxed),
            uniform_scale_updates: UNIFORM_SCALE_UPDATES.load(Ordering::Relaxed),
            scale_negative_one_lanes: SCALE_NEGATIVE_ONE_LANES.load(Ordering::Relaxed),
            scale_zero_lanes: SCALE_ZERO_LANES.load(Ordering::Relaxed),
            scale_positive_one_lanes: SCALE_POSITIVE_ONE_LANES.load(Ordering::Relaxed),
            scale_other_lanes: SCALE_OTHER_LANES.load(Ordering::Relaxed),
            shared_basis_pair_updates: SHARED_BASIS_PAIR_UPDATES.load(Ordering::Relaxed),
            shared_basis_fallback_pair_updates: SHARED_BASIS_FALLBACK_PAIR_UPDATES
                .load(Ordering::Relaxed),
            max_scaled_prefix_pairs: MAX_SCALED_PREFIX_PAIRS.load(Ordering::Relaxed),
            ordinary_pair_updates: ORDINARY_PAIR_UPDATES.load(Ordering::Relaxed),
            ordered_route_successes: ORDERED_ROUTE_SUCCESSES.load(Ordering::Relaxed),
            ordered_route_declines: ORDERED_ROUTE_DECLINES.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
pub(super) fn reset_work_counters() {
    work_counters::reset();
}

#[cfg(test)]
pub(super) fn work_counter_snapshot() -> WorkCounters {
    work_counters::snapshot()
}

#[cfg(test)]
pub(super) fn record_output_ring_batch(output_ring_writes: usize) {
    work_counters::record_ring_batch(output_ring_writes);
}

#[cfg(test)]
pub(super) fn record_route_result(success: bool) {
    work_counters::record_ordered_route(success);
}

pub(super) struct ScalarSynthesisBatch<'a, T> {
    pub(super) sin_theta: &'a [T],
    pub(super) active_lanes: usize,
    pub(super) vector_lanes: usize,
}

/// Executes one already prepared scalar component over a packed latitude batch.
///
/// The caller owns coefficient preparation and reflected-ring packing so spin
/// synthesis can reuse both for its Q and U passes. The scaled-prefix and
/// ordinary SIMD suffix remain the same implementation as scalar ordered
/// synthesis.
pub(crate) fn synthesize_prepared_component_batch_dispatched<T: OrderedSimdPrecision>(
    arch: Arch,
    m: usize,
    lmax: usize,
    sectoral_norm: T,
    sin_theta: &[T],
    vector_lanes: usize,
    scratch: &mut ScalarOrderedScratch<'_, T>,
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

struct PreparedComponentBatch<'a, 's, T> {
    m: usize,
    lmax: usize,
    sectoral_norm: T,
    sin_theta: &'a [T],
    vector_lanes: usize,
    scratch: &'a mut ScalarOrderedScratch<'s, T>,
}

impl<T: OrderedSimdPrecision> WithSimd for PreparedComponentBatch<'_, '_, T> {
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
            power_lower_limit::<T>(m),
            ScalarSynthesisBatch {
                sin_theta,
                active_lanes: sin_theta.len(),
                vector_lanes,
            },
        )
    }
}

#[inline(always)]
pub(super) fn synthesize_batch<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    scratch: &mut ScalarOrderedScratch<'_, T>,
    m: usize,
    lmax: usize,
    sectoral_norm: T,
    power_lower_limit: T,
    batch: ScalarSynthesisBatch<'_, T>,
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
    let use_shared_scaled_basis_prefix = T::USE_SHARED_SCALED_BASIS_PREFIX
        && coefficients_allow_shared_basis::<T>(
            &scratch.coefficient_even[..pair_count],
            &scratch.coefficient_odd[..pair_count],
        );
    let simd_lanes = T::lane_count::<S>();
    if simd_lanes == 0
        || input_count == 0
        || sine.len() != input_count
        || active_count < input_count
        || active_count > SCALAR_ORDER_BATCH_SIZE
        || !active_count.is_multiple_of(simd_lanes)
        || active_count != input_count.div_ceil(simd_lanes) * simd_lanes
        || scratch.coefficient_even.len() < pair_count
        || scratch.coefficient_odd.len() < pair_count
    {
        return false;
    }

    let sign = if m.is_multiple_of(2) {
        T::one()
    } else {
        -T::one()
    };
    let initialized = if active_count == input_count {
        T::initialize_sectoral_batch(
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
        T::initialize_sectoral_batch(
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
    #[cfg(test)]
    work_counters::record_initial_scaled_lanes(
        scratch.scale[..active_count]
            .iter()
            .filter(|&&scale| scale < 0)
            .count(),
    );
    scratch.previous[..active_count].fill(T::zero());
    scratch.sum_even_re[..active_count].fill(T::zero());
    scratch.sum_even_im[..active_count].fill(T::zero());
    scratch.sum_odd_re[..active_count].fill(T::zero());
    scratch.sum_odd_im[..active_count].fill(T::zero());

    if T::SHARED_PREFIX_BATCH {
        let Some(status) = prepare_scaled_prefix_group::<T, S>(
            simd,
            scratch,
            m,
            lmax,
            pair_count,
            use_shared_scaled_basis_prefix,
            0,
            active_count,
        ) else {
            return false;
        };
        if status.full_ieee
            && !synthesize_ordinary_suffix_batch::<T, S>(
                simd,
                scratch,
                lmax,
                pair_count,
                status.degree,
                status.pair,
                active_count,
            )
        {
            return false;
        }
    } else {
        // Keep f32 exponent epochs independent per SIMD vector. A single tiny
        // polar lane must not force ordinary rings through scaled arithmetic.
        let vector_pair_width = 2 * simd_lanes;
        let group_step = if simd_lanes > 1 {
            vector_pair_width
        } else {
            simd_lanes
        };
        let mut group_start = 0;
        while group_start < active_count {
            let first_end = group_start + simd_lanes;
            let first = match prepare_scaled_prefix_group::<T, S>(
                simd,
                scratch,
                m,
                lmax,
                pair_count,
                use_shared_scaled_basis_prefix,
                group_start,
                first_end,
            ) {
                Some(status) => status,
                None => return false,
            };

            let has_second = group_step == vector_pair_width && first_end < active_count;
            let second = if has_second {
                match prepare_scaled_prefix_group::<T, S>(
                    simd,
                    scratch,
                    m,
                    lmax,
                    pair_count,
                    use_shared_scaled_basis_prefix,
                    first_end,
                    first_end + simd_lanes,
                ) {
                    Some(status) => Some(status),
                    None => return false,
                }
            } else {
                None
            };

            if first.full_ieee
                && let Some(second) = second
                && second.full_ieee
                && first.degree == second.degree
                && first.pair == second.pair
            {
                if !synthesize_ordinary_suffix_two_vectors::<T, S>(
                    simd,
                    scratch,
                    lmax,
                    pair_count,
                    first.degree,
                    first.pair,
                    group_start,
                    first_end,
                ) {
                    return false;
                }
            } else {
                if first.full_ieee
                    && !synthesize_ordinary_suffix_vector::<T, S>(
                        simd,
                        scratch,
                        lmax,
                        pair_count,
                        first.degree,
                        first.pair,
                        group_start,
                        first_end,
                    )
                {
                    return false;
                }
                if let Some(second) = second
                    && second.full_ieee
                    && !synthesize_ordinary_suffix_vector::<T, S>(
                        simd,
                        scratch,
                        lmax,
                        pair_count,
                        second.degree,
                        second.pair,
                        first_end,
                        first_end + simd_lanes,
                    )
                {
                    return false;
                }
            }
            group_start += group_step;
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

fn synthesize_ordinary_suffix_batch<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    scratch: &mut ScalarOrderedScratch<'_, T>,
    lmax: usize,
    pair_count: usize,
    degree: usize,
    pair: usize,
    active_count: usize,
) -> bool {
    let lanes = T::lane_count::<S>();
    if lanes <= 1 {
        for start in (0..active_count).step_by(lanes) {
            if !synthesize_ordinary_suffix_vector::<T, S>(
                simd,
                scratch,
                lmax,
                pair_count,
                degree,
                pair,
                start,
                start + lanes,
            ) {
                return false;
            }
        }
        return true;
    }

    let pair_width = 2 * lanes;
    let mut start = 0;
    while start + pair_width <= active_count {
        if !synthesize_ordinary_suffix_two_vectors::<T, S>(
            simd,
            scratch,
            lmax,
            pair_count,
            degree,
            pair,
            start,
            start + lanes,
        ) {
            return false;
        }
        start += pair_width;
    }
    start == active_count
        || synthesize_ordinary_suffix_vector::<T, S>(
            simd,
            scratch,
            lmax,
            pair_count,
            degree,
            pair,
            start,
            active_count,
        )
}

#[derive(Clone, Copy)]
struct ScaledPrefixStatus {
    degree: usize,
    pair: usize,
    full_ieee: bool,
}

#[expect(
    clippy::too_many_arguments,
    reason = "A vector group advances its own recurrence and scale slices."
)]
fn prepare_scaled_prefix_group<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    scratch: &mut ScalarOrderedScratch<'_, T>,
    m: usize,
    lmax: usize,
    pair_count: usize,
    use_shared_scaled_basis_prefix: bool,
    start: usize,
    end: usize,
) -> Option<ScaledPrefixStatus> {
    if start >= end
        || end > scratch.scale.len()
        || !(end - start).is_multiple_of(T::lane_count::<S>())
    {
        return None;
    }
    let (mut degree, mut pair, skip_remainder) = if T::SKIP_SCALED_PREFIX {
        T::skip_scaled_prefix(
            simd,
            m,
            lmax,
            pair_count,
            &scratch.packed_csq[start..end],
            scratch.coefficient_a,
            scratch.coefficient_b,
            &mut scratch.previous[start..end],
            &mut scratch.current[start..end],
            &mut scratch.scale[start..end],
        )?
    } else {
        rescale_prefix_initial(
            simd,
            &mut scratch.previous[start..end],
            &mut scratch.current[start..end],
            &mut scratch.scale[start..end],
        )?;
        (m, 0, false)
    };
    if skip_remainder {
        return Some(ScaledPrefixStatus {
            degree,
            pair,
            full_ieee: false,
        });
    }
    let shared_scaled_basis_prefix = use_shared_scaled_basis_prefix;
    let mut full_ieee = all_scale_nonnegative(simd, &scratch.scale[start..end]);
    while !full_ieee && degree <= lmax && pair < pair_count {
        if !scaled_pair_step::<T, S>(
            simd,
            &scratch.scale[start..end],
            &scratch.packed_csq[start..end],
            scratch.coefficient_a[pair],
            scratch.coefficient_b[pair],
            scratch.coefficient_even[pair],
            scratch.coefficient_odd[pair],
            shared_scaled_basis_prefix,
            &mut scratch.corfac[start..end],
            &mut scratch.previous[start..end],
            &mut scratch.current[start..end],
            &mut scratch.sum_even_re[start..end],
            &mut scratch.sum_even_im[start..end],
            &mut scratch.sum_odd_re[start..end],
            &mut scratch.sum_odd_im[start..end],
        ) {
            return None;
        }
        #[cfg(test)]
        work_counters::record_scaled_pair_update(end - start, pair + 1, &scratch.scale[start..end]);
        degree += 2;
        pair += 1;
        rescale_batch(
            simd,
            &mut scratch.previous[start..end],
            &mut scratch.current[start..end],
            &mut scratch.scale[start..end],
        )?;
        full_ieee = all_scale_nonnegative(simd, &scratch.scale[start..end]);
    }

    if degree <= lmax
        && pair < pair_count
        && full_ieee
        && !apply_correction_and_clear_scale(
            simd,
            &mut scratch.previous[start..end],
            &mut scratch.current[start..end],
            &mut scratch.scale[start..end],
        )
    {
        return None;
    }
    Some(ScaledPrefixStatus {
        degree,
        pair,
        full_ieee,
    })
}

fn coefficients_allow_shared_basis<T: OrderedSimdPrecision>(
    even: &[Complex<T>],
    odd: &[Complex<T>],
) -> bool {
    even.iter().chain(odd).all(|coefficient| {
        [coefficient.re, coefficient.im]
            .into_iter()
            .all(|value| value.is_finite() && value.abs() <= T::one())
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "The ordinary DUCC suffix keeps one packed SIMD recurrence state in registers."
)]
fn synthesize_ordinary_suffix_vector<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    scratch: &mut ScalarOrderedScratch<'_, T>,
    lmax: usize,
    pair_count: usize,
    degree: usize,
    pair: usize,
    start: usize,
    end: usize,
) -> bool {
    #[cfg(test)]
    work_counters::record_ordinary_pair_updates(
        pair_count
            .saturating_sub(pair)
            .saturating_mul(end.saturating_sub(start)),
    );
    if degree > lmax || pair >= pair_count {
        return true;
    }
    let packed_csq = &scratch.packed_csq[start..end];
    let coefficient_a = &scratch.coefficient_a[..];
    let coefficient_b = &scratch.coefficient_b[..];
    let coefficient_even = &scratch.coefficient_even[..];
    let coefficient_odd = &scratch.coefficient_odd[..];
    let Some(csq_vector) = load_simd_vector::<T, S>(packed_csq) else {
        return false;
    };
    let Some(mut previous_vector) = load_simd_vector::<T, S>(&scratch.previous[start..end]) else {
        return false;
    };
    let Some(mut current_vector) = load_simd_vector::<T, S>(&scratch.current[start..end]) else {
        return false;
    };
    let Some(mut sum_even_re_vector) = load_simd_vector::<T, S>(&scratch.sum_even_re[start..end])
    else {
        return false;
    };
    let Some(mut sum_even_im_vector) = load_simd_vector::<T, S>(&scratch.sum_even_im[start..end])
    else {
        return false;
    };
    let Some(mut sum_odd_re_vector) = load_simd_vector::<T, S>(&scratch.sum_odd_re[start..end])
    else {
        return false;
    };
    let Some(mut sum_odd_im_vector) = load_simd_vector::<T, S>(&scratch.sum_odd_im[start..end])
    else {
        return false;
    };

    let Some(coefficient_a) = coefficient_a.get(pair..pair_count) else {
        return false;
    };
    let Some(coefficient_b) = coefficient_b.get(pair..pair_count) else {
        return false;
    };
    let Some(coefficient_even) = coefficient_even.get(pair..pair_count) else {
        return false;
    };
    let Some(coefficient_odd) = coefficient_odd.get(pair..pair_count) else {
        return false;
    };
    let coefficient_a_pairs = coefficient_a.as_chunks::<2>().0;
    let coefficient_b_pairs = coefficient_b.as_chunks::<2>().0;
    let coefficient_even_pairs = coefficient_even.as_chunks::<2>().0;
    let coefficient_odd_pairs = coefficient_odd.as_chunks::<2>().0;

    // Iterate over checked coefficient slices once, then keep the ordinary
    // recurrence and accumulators in registers for the whole suffix.
    for (((a, b), even), odd) in coefficient_a_pairs
        .iter()
        .zip(coefficient_b_pairs)
        .zip(coefficient_even_pairs)
        .zip(coefficient_odd_pairs)
    {
        ordinary_two_pair_step::<T, S>(
            simd,
            csq_vector,
            a[0],
            b[0],
            a[1],
            b[1],
            even[0],
            odd[0],
            even[1],
            odd[1],
            &mut previous_vector,
            &mut current_vector,
            &mut sum_even_re_vector,
            &mut sum_even_im_vector,
            &mut sum_odd_re_vector,
            &mut sum_odd_im_vector,
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
        ordinary_one_pair_step::<T, S>(
            simd,
            csq_vector,
            *a,
            *b,
            *even,
            *odd,
            &mut previous_vector,
            &mut current_vector,
            &mut sum_even_re_vector,
            &mut sum_even_im_vector,
            &mut sum_odd_re_vector,
            &mut sum_odd_im_vector,
        );
    }

    store_simd_vector::<T, S>(&mut scratch.previous[start..end], previous_vector)
        && store_simd_vector::<T, S>(&mut scratch.current[start..end], current_vector)
        && store_simd_vector::<T, S>(&mut scratch.sum_even_re[start..end], sum_even_re_vector)
        && store_simd_vector::<T, S>(&mut scratch.sum_even_im[start..end], sum_even_im_vector)
        && store_simd_vector::<T, S>(&mut scratch.sum_odd_re[start..end], sum_odd_re_vector)
        && store_simd_vector::<T, S>(&mut scratch.sum_odd_im[start..end], sum_odd_im_vector)
}

#[inline(always)]
#[expect(
    clippy::too_many_arguments,
    reason = "Two ring vectors share each degree's coefficient broadcasts while retaining register state."
)]
fn synthesize_ordinary_suffix_two_vectors<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    scratch: &mut ScalarOrderedScratch<'_, T>,
    lmax: usize,
    pair_count: usize,
    degree: usize,
    pair: usize,
    first_start: usize,
    second_start: usize,
) -> bool {
    #[cfg(test)]
    work_counters::record_ordinary_pair_updates(
        pair_count.saturating_sub(pair) * 2 * T::lane_count::<S>(),
    );
    if degree > lmax || pair >= pair_count {
        return true;
    }
    let lanes = T::lane_count::<S>();
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

    let Some(first_csq) = load_simd_vector::<T, S>(&scratch.packed_csq[first_start..first_end])
    else {
        return false;
    };
    let Some(second_csq) = load_simd_vector::<T, S>(&scratch.packed_csq[second_start..second_end])
    else {
        return false;
    };
    let Some(mut first_previous) =
        load_simd_vector::<T, S>(&scratch.previous[first_start..first_end])
    else {
        return false;
    };
    let Some(mut first_current) =
        load_simd_vector::<T, S>(&scratch.current[first_start..first_end])
    else {
        return false;
    };
    let Some(mut first_even_re) =
        load_simd_vector::<T, S>(&scratch.sum_even_re[first_start..first_end])
    else {
        return false;
    };
    let Some(mut first_even_im) =
        load_simd_vector::<T, S>(&scratch.sum_even_im[first_start..first_end])
    else {
        return false;
    };
    let Some(mut first_odd_re) =
        load_simd_vector::<T, S>(&scratch.sum_odd_re[first_start..first_end])
    else {
        return false;
    };
    let Some(mut first_odd_im) =
        load_simd_vector::<T, S>(&scratch.sum_odd_im[first_start..first_end])
    else {
        return false;
    };
    let Some(mut second_previous) =
        load_simd_vector::<T, S>(&scratch.previous[second_start..second_end])
    else {
        return false;
    };
    let Some(mut second_current) =
        load_simd_vector::<T, S>(&scratch.current[second_start..second_end])
    else {
        return false;
    };
    let Some(mut second_even_re) =
        load_simd_vector::<T, S>(&scratch.sum_even_re[second_start..second_end])
    else {
        return false;
    };
    let Some(mut second_even_im) =
        load_simd_vector::<T, S>(&scratch.sum_even_im[second_start..second_end])
    else {
        return false;
    };
    let Some(mut second_odd_re) =
        load_simd_vector::<T, S>(&scratch.sum_odd_re[second_start..second_end])
    else {
        return false;
    };
    let Some(mut second_odd_im) =
        load_simd_vector::<T, S>(&scratch.sum_odd_im[second_start..second_end])
    else {
        return false;
    };

    for (((a, b), even), odd) in coefficient_a_pairs
        .iter()
        .zip(coefficient_b_pairs)
        .zip(coefficient_even_pairs)
        .zip(coefficient_odd_pairs)
    {
        ordinary_two_pair_step_two_vectors::<T, S>(
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
        ordinary_one_pair_step::<T, S>(
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
        ordinary_one_pair_step::<T, S>(
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

    store_simd_vector::<T, S>(
        &mut scratch.previous[first_start..first_end],
        first_previous,
    ) && store_simd_vector::<T, S>(&mut scratch.current[first_start..first_end], first_current)
        && store_simd_vector::<T, S>(
            &mut scratch.sum_even_re[first_start..first_end],
            first_even_re,
        )
        && store_simd_vector::<T, S>(
            &mut scratch.sum_even_im[first_start..first_end],
            first_even_im,
        )
        && store_simd_vector::<T, S>(
            &mut scratch.sum_odd_re[first_start..first_end],
            first_odd_re,
        )
        && store_simd_vector::<T, S>(
            &mut scratch.sum_odd_im[first_start..first_end],
            first_odd_im,
        )
        && store_simd_vector::<T, S>(
            &mut scratch.previous[second_start..second_end],
            second_previous,
        )
        && store_simd_vector::<T, S>(
            &mut scratch.current[second_start..second_end],
            second_current,
        )
        && store_simd_vector::<T, S>(
            &mut scratch.sum_even_re[second_start..second_end],
            second_even_re,
        )
        && store_simd_vector::<T, S>(
            &mut scratch.sum_even_im[second_start..second_end],
            second_even_im,
        )
        && store_simd_vector::<T, S>(
            &mut scratch.sum_odd_re[second_start..second_end],
            second_odd_re,
        )
        && store_simd_vector::<T, S>(
            &mut scratch.sum_odd_im[second_start..second_end],
            second_odd_im,
        )
}

pub(super) fn power_lower_limit<T: OrderedSimdPrecision>(exponent: usize) -> T {
    if exponent == 0 {
        T::zero()
    } else {
        let half_scale_exponent =
            T::from_i64(T::SCALE_EXPONENT / 2).expect("precision scale exponent is representable");
        let exponent = T::from_usize(exponent).expect("usize degree is representable");
        let ln_two =
            T::from_f64(std::f64::consts::LN_2).expect("ln(2) is representable in f32 and f64");
        (-(half_scale_exponent * ln_two) / exponent).exp()
    }
}

#[inline]
fn integer_power_vector<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    mut base: T::Vector<S>,
    mut exponent: usize,
) -> T::Vector<S> {
    let mut result = T::splat(simd, T::one());
    while exponent > 0 {
        if exponent & 1 != 0 {
            result = T::vmul(simd, result, base);
        }
        exponent >>= 1;
        if exponent > 0 {
            base = T::vmul(simd, base, base);
        }
    }
    result
}

#[inline(always)]
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

fn f64_mask_any<S: Simd>(simd: S, mask: S::m64s) -> bool {
    let bits = simd.select_f64s(mask, simd.splat_f64s(1.0), simd.splat_f64s(0.0));
    simd.reduce_max_f64s(bits) > 0.0
}

fn f64_mask_all<S: Simd>(simd: S, mask: S::m64s) -> bool {
    let bits = simd.select_f64s(mask, simd.splat_f64s(1.0), simd.splat_f64s(0.0));
    simd.reduce_min_f64s(bits) > 0.0
}

fn f64_all_scale_below_one<S: Simd>(simd: S, scale: &[i64]) -> bool {
    let (scale_vectors, scale_tail) = S::as_simd_i64s(scale);
    let one = simd.splat_i64s(1);
    scale_vectors
        .iter()
        .all(|&value| f64_mask_all(simd, simd.less_than_i64s(value, one)))
        && scale_tail.iter().all(|&value| value < 1)
}

fn f64_rescale_prefix_initial<S: Simd>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale: &mut [i64],
) -> Option<bool> {
    f64_rescale_batch_inner::<S, false, true>(simd, previous, current, scale).map(|status| status.1)
}

fn f64_rescale_prefix_step<S: Simd>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale: &mut [i64],
) -> Option<bool> {
    let (any_rescaled, _) =
        f64_rescale_batch_inner::<S, true, false>(simd, previous, current, scale)?;
    Some(!any_rescaled || f64_all_scale_below_one(simd, scale))
}

#[inline(always)]
fn f64_rescale_batch_inner<S: Simd, const TRACK_ANY: bool, const TRACK_PREFIX: bool>(
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
    let ftol = simd.splat_f64s(<f64 as OrderedSimdPrecision>::SCALE_FTOL);
    let scale_down = simd.splat_f64s(<f64 as LegendrePrecision>::SCALE_DOWN);
    let scale_one = simd.splat_i64s(1);
    let scale_max = simd.splat_i64s(i64::MAX);
    let mut any_rescaled = false;
    let mut all_below_one = true;

    for index in 0..current_vectors.len() {
        let rescale = simd.greater_than_f64s(simd.abs_f64s(current_vectors[index]), ftol);
        if TRACK_ANY {
            let vector_rescaled = f64_mask_any(simd, rescale);
            any_rescaled |= vector_rescaled;
            if !vector_rescaled {
                if TRACK_PREFIX {
                    all_below_one &=
                        f64_mask_all(simd, simd.less_than_i64s(scale_vectors[index], scale_one));
                }
                continue;
            }
        }
        let overflow = simd.and_m64s(rescale, simd.equal_i64s(scale_vectors[index], scale_max));
        if f64_mask_any(simd, overflow) {
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
        if TRACK_PREFIX {
            all_below_one &=
                f64_mask_all(simd, simd.less_than_i64s(scale_vectors[index], scale_one));
        }
    }

    for lane in 0..current_tail.len() {
        if current_tail[lane].abs() > <f64 as OrderedSimdPrecision>::SCALE_FTOL {
            previous_tail[lane] *= <f64 as LegendrePrecision>::SCALE_DOWN;
            current_tail[lane] *= <f64 as LegendrePrecision>::SCALE_DOWN;
            scale_tail[lane] = scale_tail[lane].checked_add(1)?;
            if TRACK_ANY {
                any_rescaled = true;
            }
        }
        if TRACK_PREFIX {
            all_below_one &= scale_tail[lane] < 1;
        }
    }
    Some((any_rescaled, all_below_one))
}

#[expect(
    clippy::too_many_arguments,
    reason = "The f64 skipped prefix advances two degree pairs across one shared batch."
)]
fn recurrence_two_pair_step_f64<S: Simd>(
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
    reason = "The f64 precision leaf mirrors the shared SIMD recurrence state interface."
)]
pub(super) fn skip_negligible_scaled_prefix_f64<S: Simd>(
    simd: S,
    m: usize,
    lmax: usize,
    pair_count: usize,
    csq: &[f64],
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    previous: &mut [f64],
    current: &mut [f64],
    scale: &mut [i64],
) -> Option<(usize, usize, bool)> {
    if previous.len() != current.len()
        || current.len() != scale.len()
        || csq.len() != current.len()
        || coefficient_a.len() < pair_count
        || coefficient_b.len() < pair_count
    {
        return None;
    }
    let mut below_limit = f64_rescale_prefix_initial(simd, previous, current, scale)?;
    let mut degree = m;
    let mut pair = 0;
    while below_limit {
        if degree > lmax || lmax.saturating_sub(degree) < 4 || pair + 1 >= pair_count {
            return Some((lmax.saturating_add(1), pair, true));
        }
        recurrence_two_pair_step_f64(
            simd,
            csq,
            coefficient_a[pair],
            coefficient_b[pair],
            coefficient_a[pair + 1],
            coefficient_b[pair + 1],
            previous,
            current,
        );
        degree += 4;
        pair += 2;
        below_limit = f64_rescale_prefix_step(simd, previous, current, scale)?;
    }
    Some((degree, pair, false))
}

#[inline(always)]
fn scaled_integer_power_f64s<S: Simd>(
    simd: S,
    mut base: S::f64s,
    exponent: usize,
) -> Option<(S::f64s, S::i64s)> {
    let mut base_scale = simd.splat_i64s(0);
    let mut value = simd.splat_f64s(1.0);
    let mut scale = simd.splat_i64s(0);
    normalize_f64_vector(
        simd,
        &mut base,
        &mut base_scale,
        <f64 as LegendrePrecision>::BIG,
    )?;
    let mut power = exponent;
    while power > 0 {
        if power & 1 != 0 {
            value = simd.mul_f64s(value, base);
            add_i64_scale_vector(simd, &mut scale, base_scale)?;
            normalize_f64_vector(
                simd,
                &mut value,
                &mut scale,
                <f64 as LegendrePrecision>::BIG,
            )?;
        }
        base = simd.mul_f64s(base, base);
        double_i64_scale_vector(simd, &mut base_scale)?;
        normalize_f64_vector(
            simd,
            &mut base,
            &mut base_scale,
            <f64 as LegendrePrecision>::BIG,
        )?;
        power >>= 1;
    }
    Some((value, scale))
}

fn add_i64_scale_vector<S: Simd>(simd: S, scale: &mut S::i64s, addend: S::i64s) -> Option<()> {
    let sum = simd.add_i64s(*scale, addend);
    let overflow_bits = simd.and_i64s(simd.xor_i64s(*scale, sum), simd.xor_i64s(addend, sum));
    let zero = simd.splat_i64s(0);
    if f64_mask_any(simd, simd.less_than_i64s(overflow_bits, zero)) {
        return None;
    }
    *scale = sum;
    Some(())
}

fn double_i64_scale_vector<S: Simd>(simd: S, scale: &mut S::i64s) -> Option<()> {
    let doubled = simd.add_i64s(*scale, *scale);
    let sign_changed = simd.less_than_i64s(simd.xor_i64s(*scale, doubled), simd.splat_i64s(0));
    if f64_mask_any(simd, sign_changed) {
        return None;
    }
    *scale = doubled;
    Some(())
}

fn normalize_f64_vector<S: Simd>(
    simd: S,
    value: &mut S::f64s,
    scale: &mut S::i64s,
    maximum: f64,
) -> Option<()> {
    let maximum_vector = simd.splat_f64s(maximum);
    let minimum_vector = simd.splat_f64s(maximum * <f64 as LegendrePrecision>::SCALE_DOWN);
    let scale_down = simd.splat_f64s(<f64 as LegendrePrecision>::SCALE_DOWN);
    let scale_up = simd.splat_f64s(<f64 as LegendrePrecision>::SCALE_UP);
    let scale_one = simd.splat_i64s(1);
    let scale_minus_one = simd.splat_i64s(-1);
    let scale_max = simd.splat_i64s(i64::MAX);
    let scale_min = simd.splat_i64s(i64::MIN);
    let zero = simd.splat_f64s(0.0);
    let mut magnitude = simd.abs_f64s(*value);
    if !f64_mask_all(
        simd,
        simd.less_than_or_equal_f64s(magnitude, simd.splat_f64s(f64::MAX)),
    ) {
        return None;
    }
    loop {
        let too_large = simd.greater_than_f64s(magnitude, maximum_vector);
        if !f64_mask_any(simd, too_large) {
            break;
        }
        if f64_mask_any(
            simd,
            simd.and_m64s(too_large, simd.equal_i64s(*scale, scale_max)),
        ) {
            return None;
        }
        *value = simd.select_f64s(too_large, simd.mul_f64s(*value, scale_down), *value);
        *scale = simd.select_i64s(too_large, simd.add_i64s(*scale, scale_one), *scale);
        magnitude = simd.abs_f64s(*value);
    }
    loop {
        let nonzero = simd.not_m64s(simd.equal_f64s(magnitude, zero));
        let too_small = simd.and_m64s(nonzero, simd.less_than_f64s(magnitude, minimum_vector));
        if !f64_mask_any(simd, too_small) {
            break;
        }
        if f64_mask_any(
            simd,
            simd.and_m64s(too_small, simd.equal_i64s(*scale, scale_min)),
        ) {
            return None;
        }
        *value = simd.select_f64s(too_small, simd.mul_f64s(*value, scale_up), *value);
        *scale = simd.select_i64s(too_small, simd.add_i64s(*scale, scale_minus_one), *scale);
        magnitude = simd.abs_f64s(*value);
    }
    Some(())
}

fn normalize_batch_f64<S: Simd>(
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
    for (value, scale) in value_vectors.iter_mut().zip(scale_vectors.iter_mut()) {
        if normalize_f64_vector(simd, value, scale, maximum).is_none() {
            return false;
        }
    }
    value_tail
        .iter_mut()
        .zip(scale_tail.iter_mut())
        .all(|(value, scale)| normalize_lane::<f64>(value, scale, maximum))
}

/// Uses the native SIMD power path when every lane stays above its per-order
/// threshold. A block containing a scaled lane uses the checked scalar power
/// leaf so each lane retains its own binary exponent without assuming that
/// f32 and i64 SIMD vectors have equal widths.
pub(super) fn initialize_sectoral_batch_native<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    sine: &[T],
    values: &mut [T],
    scales: &mut [i64],
    exponent: usize,
    lower_limit: T,
    factor_value: T,
) -> bool {
    if sine.len() != values.len() || sine.len() != scales.len() {
        return false;
    }
    let lane_count = T::lane_count::<S>();
    let vector_count = sine.len() / lane_count;
    for start in (0..vector_count * lane_count).step_by(lane_count) {
        let end = start + lane_count;
        if sine[start..end].iter().any(|value| !value.is_finite()) {
            return false;
        }
        if sine[start..end]
            .iter()
            .any(|value| value.abs() < lower_limit)
        {
            for lane in start..end {
                let Some((power, scale)) = mypow(sine[lane], exponent, lower_limit) else {
                    return false;
                };
                values[lane] = power * factor_value;
                scales[lane] = scale;
            }
        }
    }

    let (sine_vectors, sine_tail) = T::as_simd::<S>(sine);
    let (value_vectors, value_tail) = T::as_mut_simd::<S>(values);
    let factor = T::splat(simd, factor_value);
    let finite_limit = T::splat(simd, T::MAX_FINITE);
    for index in 0..sine_vectors.len() {
        if !T::mask_all(
            simd,
            T::less_equal(simd, T::vabs(simd, sine_vectors[index]), finite_limit),
        ) {
            return false;
        }
        let start = index * lane_count;
        let end = start + lane_count;
        if sine[start..end]
            .iter()
            .any(|value| value.abs() < lower_limit)
        {
            continue;
        } else {
            value_vectors[index] = T::vmul(
                simd,
                integer_power_vector::<T, S>(simd, sine_vectors[index], exponent),
                factor,
            );
            scales[start..end].fill(0);
        }
    }
    if sine_tail.iter().any(|value| !value.is_finite()) {
        return false;
    }
    let tail_start = sine.len() - sine_tail.len();
    for (offset, (&sine_value, value)) in sine_tail.iter().zip(value_tail.iter_mut()).enumerate() {
        let Some((power, scale)) = mypow(sine_value, exponent, lower_limit) else {
            return false;
        };
        *value = power * factor_value;
        scales[tail_start + offset] = scale;
    }
    normalize_batch::<T, S>(simd, values, scales, T::SCALE_FTOL)
}

pub(super) fn initialize_sectoral_batch_f64<S: Simd>(
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
        if !f64_mask_all(
            simd,
            simd.less_than_or_equal_f64s(simd.abs_f64s(sine_vectors[index]), finite_limit),
        ) {
            return false;
        }
        let scaled = simd.less_than_f64s(simd.abs_f64s(sine_vectors[index]), threshold);
        if f64_mask_any(simd, scaled) {
            let Some((value, scale)) =
                scaled_integer_power_f64s(simd, sine_vectors[index], exponent)
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
    normalize_batch_f64(
        simd,
        values,
        scales,
        <f64 as OrderedSimdPrecision>::SCALE_FTOL,
    )
}

#[cfg(test)]
pub(super) fn initialize_sectoral_batch<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    sine: &[T],
    values: &mut [T],
    scales: &mut [i64],
    exponent: usize,
    lower_limit: T,
    factor_value: T,
) -> bool {
    T::initialize_sectoral_batch(
        simd,
        sine,
        values,
        scales,
        exponent,
        lower_limit,
        factor_value,
    )
}

pub(super) fn scalar_integer_power<T: LegendrePrecision>(mut base: T, mut exponent: usize) -> T {
    let mut result = T::one();
    while exponent > 0 {
        if exponent & 1 != 0 {
            result = result * base;
        }
        exponent >>= 1;
        if exponent > 0 {
            base = base * base;
        }
    }
    result
}

pub(super) fn mypow<T: OrderedSimdPrecision>(
    value: T,
    exponent: usize,
    lower_limit: T,
) -> Option<(T, i64)> {
    if exponent == 0 {
        return Some((T::one(), 0));
    }
    if value.abs() >= lower_limit {
        let result = scalar_integer_power(value, exponent);
        return result.is_finite().then_some((result, 0));
    }

    let mut base = value;
    let mut base_scale = 0_i64;
    if !normalize_lane(&mut base, &mut base_scale, T::BIG) {
        return None;
    }
    let mut result = T::one();
    let mut result_scale = 0_i64;
    let mut power = exponent;
    while power > 0 {
        if power & 1 != 0 {
            result = result * base;
            result_scale = result_scale.checked_add(base_scale)?;
            if !normalize_lane(&mut result, &mut result_scale, T::BIG) {
                return None;
            }
        }
        power >>= 1;
        if power > 0 {
            base = base * base;
            base_scale = base_scale.checked_mul(2)?;
            if !normalize_lane(&mut base, &mut base_scale, T::BIG) {
                return None;
            }
        }
    }
    Some((result, result_scale))
}

pub(super) fn normalize_lane<T: OrderedSimdPrecision>(
    value: &mut T,
    scale: &mut i64,
    maximum: T,
) -> bool {
    if !value.is_finite() {
        return false;
    }
    while value.abs() > maximum {
        *value = *value * T::SCALE_DOWN;
        let Some(next_scale) = scale.checked_add(1) else {
            return false;
        };
        *scale = next_scale;
    }
    let minimum = maximum * T::SCALE_DOWN;
    if *value != T::zero() {
        while value.abs() < minimum {
            *value = *value * T::SCALE_UP;
            let Some(next_scale) = scale.checked_sub(1) else {
                return false;
            };
            *scale = next_scale;
        }
    }
    value.is_finite()
}

pub(super) fn normalize_batch<T: OrderedSimdPrecision, S: Simd>(
    _simd: S,
    values: &mut [T],
    scales: &mut [i64],
    maximum: T,
) -> bool {
    if values.len() != scales.len() {
        return false;
    }
    for (value, scale) in values.iter_mut().zip(scales.iter_mut()) {
        if !normalize_lane(value, scale, maximum) {
            return false;
        }
    }
    true
}

/// Applies DUCC's recurrence rescale to selected lanes and returns whether
/// any lane was rescaled and whether every lane remains below scale epoch 1.
#[inline]
pub(super) fn rescale_batch<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    previous: &mut [T],
    current: &mut [T],
    scale: &mut [i64],
) -> Option<bool> {
    rescale_batch_inner::<T, S, true, false>(simd, previous, current, scale).map(|status| status.0)
}

#[inline]
fn rescale_prefix_initial<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    previous: &mut [T],
    current: &mut [T],
    scale: &mut [i64],
) -> Option<bool> {
    rescale_batch_inner::<T, S, false, true>(simd, previous, current, scale).map(|status| status.1)
}

#[inline]
#[cfg(test)]
pub(super) fn rescale_prefix_step<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    previous: &mut [T],
    current: &mut [T],
    scale: &mut [i64],
) -> Option<bool> {
    let (any_rescaled, _) =
        rescale_batch_inner::<T, S, true, false>(simd, previous, current, scale)?;
    Some(!any_rescaled || all_scale_below_one(simd, scale))
}

// This is confined to the scaled prefix; ordinary recurrence and accumulation
// remain in the SIMD bulk path. Exponent counts remain checked per lane while
// the precision-specific leaf handles recurrence rescaling.
#[inline(always)]
fn rescale_batch_inner<
    T: OrderedSimdPrecision,
    S: Simd,
    const TRACK_ANY_RESCALED: bool,
    const TRACK_PREFIX_EPOCH: bool,
>(
    simd: S,
    previous: &mut [T],
    current: &mut [T],
    scale: &mut [i64],
) -> Option<(bool, bool)> {
    T::rescale_ordered_batch::<S, TRACK_ANY_RESCALED, TRACK_PREFIX_EPOCH>(
        simd, previous, current, scale,
    )
}

#[inline(always)]
pub(super) fn rescale_batch_inner_scalar<
    T: OrderedSimdPrecision,
    S: Simd,
    const TRACK_ANY_RESCALED: bool,
    const TRACK_PREFIX_EPOCH: bool,
>(
    _simd: S,
    previous: &mut [T],
    current: &mut [T],
    scale: &mut [i64],
) -> Option<(bool, bool)> {
    if previous.len() != current.len() || current.len() != scale.len() {
        return None;
    }
    let mut any_rescaled = false;
    let mut all_below_one = true;
    for lane in 0..current.len() {
        let rescale = current[lane].abs() > T::SCALE_FTOL;
        if rescale {
            let next_scale = scale[lane].checked_add(1)?;
            let next_previous = previous[lane] * T::SCALE_DOWN;
            let next_current = current[lane] * T::SCALE_DOWN;
            if (previous[lane] == T::zero() || next_previous != T::zero())
                && (current[lane] == T::zero() || next_current != T::zero())
            {
                previous[lane] = next_previous;
                current[lane] = next_current;
                scale[lane] = next_scale;
                if TRACK_ANY_RESCALED {
                    any_rescaled = true;
                }
            }
        }
        if TRACK_PREFIX_EPOCH {
            all_below_one &= scale[lane] < 1;
        }
    }
    Some((any_rescaled, all_below_one))
}

#[cfg(test)]
pub(super) fn set_correction_factors<T: OrderedSimdPrecision, S: Simd>(
    _simd: S,
    scale: &[i64],
    correction: &mut [T],
) {
    assert_eq!(scale.len(), correction.len());
    for (scale_value, correction_value) in scale.iter().zip(correction.iter_mut()) {
        *correction_value = if *scale_value < 0 {
            T::zero()
        } else if *scale_value < 1 {
            T::one()
        } else {
            T::SCALE_UP
        };
    }
}

pub(super) fn all_scale_nonnegative<S: Simd>(_simd: S, scale: &[i64]) -> bool {
    scale.iter().all(|&value| value >= 0)
}

#[cfg(test)]
fn all_scale_below_one<S: Simd>(_simd: S, scale: &[i64]) -> bool {
    scale.iter().all(|&value| value < 1)
}

fn apply_correction_and_clear_scale<T: OrderedSimdPrecision, S: Simd>(
    _simd: S,
    previous: &mut [T],
    current: &mut [T],
    scale: &mut [i64],
) -> bool {
    if previous.len() != current.len() || current.len() != scale.len() {
        return false;
    }
    for lane in 0..current.len() {
        let Some(exponent) = scale[lane].checked_mul(T::SCALE_EXPONENT) else {
            return false;
        };
        previous[lane] = T::scale_binary(previous[lane], exponent);
        current[lane] = T::scale_binary(current[lane], exponent);
        if !previous[lane].is_finite() || !current[lane].is_finite() {
            return false;
        }
    }
    scale.fill(0);
    true
}

#[expect(
    clippy::too_many_arguments,
    reason = "The scaled DUCC loop fuses one recurrence step with even/odd coefficient accumulation."
)]
fn scaled_pair_step<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    scale: &[i64],
    csq: &[T],
    coefficient_a: T,
    coefficient_b: T,
    even: Complex<T>,
    odd: Complex<T>,
    shared_scaled_basis_prefix: bool,
    scaled_basis: &mut [T],
    previous: &mut [T],
    current: &mut [T],
    sum_even_re: &mut [T],
    sum_even_im: &mut [T],
    sum_odd_re: &mut [T],
    sum_odd_im: &mut [T],
) -> bool {
    if scale.len() != current.len()
        || previous.len() != current.len()
        || csq.len() != current.len()
        || sum_even_re.len() != current.len()
        || sum_even_im.len() != current.len()
        || sum_odd_re.len() != current.len()
        || sum_odd_im.len() != current.len()
        || scaled_basis.len() != current.len()
    {
        return false;
    }

    let mut use_shared_scaled_basis = shared_scaled_basis_prefix;
    if use_shared_scaled_basis {
        if !T::scale_binary_simd(simd, current, scale, scaled_basis) {
            return false;
        }
        for basis in scaled_basis.iter().take(current.len()) {
            if !basis.is_finite() {
                use_shared_scaled_basis = false;
                break;
            }
        }
    }
    #[cfg(test)]
    work_counters::record_shared_basis_path(use_shared_scaled_basis);

    if !use_shared_scaled_basis {
        for lane in 0..current.len() {
            if !scaled_multiply_accumulate(
                &mut sum_even_re[lane],
                current[lane],
                even.re,
                scale[lane],
            ) || !scaled_multiply_accumulate(
                &mut sum_even_im[lane],
                current[lane],
                even.im,
                scale[lane],
            ) || !scaled_multiply_accumulate(
                &mut sum_odd_re[lane],
                current[lane],
                odd.re,
                scale[lane],
            ) || !scaled_multiply_accumulate(
                &mut sum_odd_im[lane],
                current[lane],
                odd.im,
                scale[lane],
            ) {
                return false;
            }
        }
    }
    let (previous_vectors, previous_tail) = T::as_mut_simd::<S>(previous);
    let (current_vectors, current_tail) = T::as_mut_simd::<S>(current);
    let (csq_vectors, csq_tail) = T::as_simd::<S>(csq);
    let (sum_even_re_vectors, sum_even_re_tail) = T::as_mut_simd::<S>(sum_even_re);
    let (sum_even_im_vectors, sum_even_im_tail) = T::as_mut_simd::<S>(sum_even_im);
    let (sum_odd_re_vectors, sum_odd_re_tail) = T::as_mut_simd::<S>(sum_odd_re);
    let (sum_odd_im_vectors, sum_odd_im_tail) = T::as_mut_simd::<S>(sum_odd_im);
    let (scaled_basis_vectors, scaled_basis_tail) = T::as_simd::<S>(scaled_basis);
    let a = T::splat(simd, coefficient_a);
    let b = T::splat(simd, coefficient_b);
    let even_re = T::splat(simd, even.re);
    let even_im = T::splat(simd, even.im);
    let odd_re = T::splat(simd, odd.re);
    let odd_im = T::splat(simd, odd.im);
    for index in 0..current_vectors.len() {
        let old_current = current_vectors[index];
        if use_shared_scaled_basis {
            let basis = scaled_basis_vectors[index];
            sum_even_re_vectors[index] =
                T::vmul_add(simd, basis, even_re, sum_even_re_vectors[index]);
            sum_even_im_vectors[index] =
                T::vmul_add(simd, basis, even_im, sum_even_im_vectors[index]);
            sum_odd_re_vectors[index] = T::vmul_add(simd, basis, odd_re, sum_odd_re_vectors[index]);
            sum_odd_im_vectors[index] = T::vmul_add(simd, basis, odd_im, sum_odd_im_vectors[index]);
        }
        let leading = T::vmul_add(simd, a, csq_vectors[index], b);
        current_vectors[index] = T::vmul_add(simd, leading, old_current, previous_vectors[index]);
        previous_vectors[index] = old_current;
    }
    for lane in 0..current_tail.len() {
        let old_current = current_tail[lane];
        if use_shared_scaled_basis {
            let basis = scaled_basis_tail[lane];
            sum_even_re_tail[lane] = basis.mul_add(even.re, sum_even_re_tail[lane]);
            sum_even_im_tail[lane] = basis.mul_add(even.im, sum_even_im_tail[lane]);
            sum_odd_re_tail[lane] = basis.mul_add(odd.re, sum_odd_re_tail[lane]);
            sum_odd_im_tail[lane] = basis.mul_add(odd.im, sum_odd_im_tail[lane]);
        }
        let leading = coefficient_a.mul_add(csq_tail[lane], coefficient_b);
        current_tail[lane] = leading.mul_add(old_current, previous_tail[lane]);
        previous_tail[lane] = old_current;
    }
    true
}

#[inline]
fn scaled_multiply_accumulate<T: OrderedSimdPrecision>(
    sum: &mut T,
    recurrence: T,
    coefficient: T,
    scale: i64,
) -> bool {
    if recurrence == T::zero() || coefficient == T::zero() {
        return true;
    }
    let mut coefficient_mantissa = coefficient;
    let mut coefficient_scale = 0_i64;
    if !normalize_lane(&mut coefficient_mantissa, &mut coefficient_scale, T::BIG) {
        return false;
    }
    let Some(exponent) = scale
        .checked_add(coefficient_scale)
        .and_then(|value| value.checked_mul(T::SCALE_EXPONENT))
    else {
        return false;
    };
    let contribution = T::scale_binary(recurrence * coefficient_mantissa, exponent);
    if !contribution.is_finite() {
        return false;
    }
    *sum = *sum + contribution;
    sum.is_finite()
}

#[inline(always)]
fn load_simd_vector<T: OrderedSimdPrecision, S: Simd>(values: &[T]) -> Option<T::Vector<S>> {
    let (vectors, tail) = T::as_simd::<S>(values);
    (vectors.len() == 1 && tail.is_empty()).then(|| vectors[0])
}

#[inline(always)]
fn store_simd_vector<T: OrderedSimdPrecision, S: Simd>(
    values: &mut [T],
    vector: T::Vector<S>,
) -> bool {
    let (vectors, tail) = T::as_mut_simd::<S>(values);
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
fn ordinary_two_pair_step<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    csq: T::Vector<S>,
    coefficient_a1: T,
    coefficient_b1: T,
    coefficient_a2: T,
    coefficient_b2: T,
    even1: Complex<T>,
    odd1: Complex<T>,
    even2: Complex<T>,
    odd2: Complex<T>,
    previous: &mut T::Vector<S>,
    current: &mut T::Vector<S>,
    sum_even_re: &mut T::Vector<S>,
    sum_even_im: &mut T::Vector<S>,
    sum_odd_re: &mut T::Vector<S>,
    sum_odd_im: &mut T::Vector<S>,
) {
    let a1 = T::splat(simd, coefficient_a1);
    let b1 = T::splat(simd, coefficient_b1);
    let a2 = T::splat(simd, coefficient_a2);
    let b2 = T::splat(simd, coefficient_b2);
    let even1_re = T::splat(simd, even1.re);
    let even1_im = T::splat(simd, even1.im);
    let odd1_re = T::splat(simd, odd1.re);
    let odd1_im = T::splat(simd, odd1.im);
    let even2_re = T::splat(simd, even2.re);
    let even2_im = T::splat(simd, even2.im);
    let odd2_re = T::splat(simd, odd2.re);
    let odd2_im = T::splat(simd, odd2.im);
    let old_previous = *previous;
    let old_current = *current;
    *sum_even_re = T::vmul_add(simd, old_current, even1_re, *sum_even_re);
    *sum_even_im = T::vmul_add(simd, old_current, even1_im, *sum_even_im);
    *sum_odd_re = T::vmul_add(simd, old_current, odd1_re, *sum_odd_re);
    *sum_odd_im = T::vmul_add(simd, old_current, odd1_im, *sum_odd_im);
    let leading1 = T::vmul_add(simd, a1, csq, b1);
    let next1 = T::vmul_add(simd, leading1, old_current, old_previous);
    *sum_even_re = T::vmul_add(simd, next1, even2_re, *sum_even_re);
    *sum_even_im = T::vmul_add(simd, next1, even2_im, *sum_even_im);
    *sum_odd_re = T::vmul_add(simd, next1, odd2_re, *sum_odd_re);
    *sum_odd_im = T::vmul_add(simd, next1, odd2_im, *sum_odd_im);
    let leading2 = T::vmul_add(simd, a2, csq, b2);
    let next2 = T::vmul_add(simd, leading2, next1, old_current);
    *previous = next1;
    *current = next2;
}

#[expect(
    clippy::too_many_arguments,
    reason = "The two-vector ordinary suffix shares each coefficient broadcast across its lanes."
)]
#[inline(always)]
fn ordinary_two_pair_step_two_vectors<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    first_csq: T::Vector<S>,
    second_csq: T::Vector<S>,
    coefficient_a1: T,
    coefficient_b1: T,
    coefficient_a2: T,
    coefficient_b2: T,
    even1: Complex<T>,
    odd1: Complex<T>,
    even2: Complex<T>,
    odd2: Complex<T>,
    first_previous: &mut T::Vector<S>,
    first_current: &mut T::Vector<S>,
    first_sum_even_re: &mut T::Vector<S>,
    first_sum_even_im: &mut T::Vector<S>,
    first_sum_odd_re: &mut T::Vector<S>,
    first_sum_odd_im: &mut T::Vector<S>,
    second_previous: &mut T::Vector<S>,
    second_current: &mut T::Vector<S>,
    second_sum_even_re: &mut T::Vector<S>,
    second_sum_even_im: &mut T::Vector<S>,
    second_sum_odd_re: &mut T::Vector<S>,
    second_sum_odd_im: &mut T::Vector<S>,
) {
    let even1_re = T::splat(simd, even1.re);
    *first_sum_even_re = T::vmul_add(simd, *first_current, even1_re, *first_sum_even_re);
    *second_sum_even_re = T::vmul_add(simd, *second_current, even1_re, *second_sum_even_re);
    let even1_im = T::splat(simd, even1.im);
    *first_sum_even_im = T::vmul_add(simd, *first_current, even1_im, *first_sum_even_im);
    *second_sum_even_im = T::vmul_add(simd, *second_current, even1_im, *second_sum_even_im);
    let odd1_re = T::splat(simd, odd1.re);
    *first_sum_odd_re = T::vmul_add(simd, *first_current, odd1_re, *first_sum_odd_re);
    *second_sum_odd_re = T::vmul_add(simd, *second_current, odd1_re, *second_sum_odd_re);
    let odd1_im = T::splat(simd, odd1.im);
    *first_sum_odd_im = T::vmul_add(simd, *first_current, odd1_im, *first_sum_odd_im);
    *second_sum_odd_im = T::vmul_add(simd, *second_current, odd1_im, *second_sum_odd_im);

    let a1 = T::splat(simd, coefficient_a1);
    let b1 = T::splat(simd, coefficient_b1);
    let first_leading1 = T::vmul_add(simd, a1, first_csq, b1);
    let first_next1 = T::vmul_add(simd, first_leading1, *first_current, *first_previous);
    *first_previous = first_next1;
    let second_leading1 = T::vmul_add(simd, a1, second_csq, b1);
    let second_next1 = T::vmul_add(simd, second_leading1, *second_current, *second_previous);
    *second_previous = second_next1;

    let even2_re = T::splat(simd, even2.re);
    *first_sum_even_re = T::vmul_add(simd, *first_previous, even2_re, *first_sum_even_re);
    *second_sum_even_re = T::vmul_add(simd, *second_previous, even2_re, *second_sum_even_re);
    let even2_im = T::splat(simd, even2.im);
    *first_sum_even_im = T::vmul_add(simd, *first_previous, even2_im, *first_sum_even_im);
    *second_sum_even_im = T::vmul_add(simd, *second_previous, even2_im, *second_sum_even_im);
    let odd2_re = T::splat(simd, odd2.re);
    *first_sum_odd_re = T::vmul_add(simd, *first_previous, odd2_re, *first_sum_odd_re);
    *second_sum_odd_re = T::vmul_add(simd, *second_previous, odd2_re, *second_sum_odd_re);
    let odd2_im = T::splat(simd, odd2.im);
    *first_sum_odd_im = T::vmul_add(simd, *first_previous, odd2_im, *first_sum_odd_im);
    *second_sum_odd_im = T::vmul_add(simd, *second_previous, odd2_im, *second_sum_odd_im);

    let a2 = T::splat(simd, coefficient_a2);
    let b2 = T::splat(simd, coefficient_b2);
    let first_leading2 = T::vmul_add(simd, a2, first_csq, b2);
    let first_next2 = T::vmul_add(simd, first_leading2, *first_previous, *first_current);
    *first_current = first_next2;
    let second_leading2 = T::vmul_add(simd, a2, second_csq, b2);
    let second_next2 = T::vmul_add(simd, second_leading2, *second_previous, *second_current);
    *second_current = second_next2;
}

#[expect(
    clippy::too_many_arguments,
    reason = "The final ordinary degree pair shares one vector recurrence and accumulation pass."
)]
#[inline(always)]
fn ordinary_one_pair_step<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    csq: T::Vector<S>,
    coefficient_a: T,
    coefficient_b: T,
    even: Complex<T>,
    odd: Complex<T>,
    previous: &mut T::Vector<S>,
    current: &mut T::Vector<S>,
    sum_even_re: &mut T::Vector<S>,
    sum_even_im: &mut T::Vector<S>,
    sum_odd_re: &mut T::Vector<S>,
    sum_odd_im: &mut T::Vector<S>,
) {
    let a = T::splat(simd, coefficient_a);
    let b = T::splat(simd, coefficient_b);
    let even_re = T::splat(simd, even.re);
    let even_im = T::splat(simd, even.im);
    let odd_re = T::splat(simd, odd.re);
    let odd_im = T::splat(simd, odd.im);
    let old_previous = *previous;
    let old_current = *current;
    *sum_even_re = T::vmul_add(simd, old_current, even_re, *sum_even_re);
    *sum_even_im = T::vmul_add(simd, old_current, even_im, *sum_even_im);
    *sum_odd_re = T::vmul_add(simd, old_current, odd_re, *sum_odd_re);
    *sum_odd_im = T::vmul_add(simd, old_current, odd_im, *sum_odd_im);
    let leading = T::vmul_add(simd, a, csq, b);
    let next = T::vmul_add(simd, leading, old_current, old_previous);
    *previous = old_current;
    *current = next;
}
