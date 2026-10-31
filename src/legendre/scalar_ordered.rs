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

use num_complex::Complex;
use pulp::{Arch, Simd, WithSimd};
use std::{cell::Cell, marker::PhantomData, ptr::NonNull};

use crate::legendre::{
    LegendrePrecision, OrderedSimdPrecision, SCALAR_ORDER_BATCH_SIZE, longitude_phase,
    scalar_paired,
};

mod kernel;

#[cfg(test)]
pub(crate) use kernel::WorkCounters as ScalarOrderedWorkCounters;
pub(crate) use kernel::synthesize_prepared_component_batch_dispatched;

pub(super) fn initialize_sectoral_batch_f32<S: Simd>(
    simd: S,
    sine: &[f32],
    values: &mut [f32],
    scales: &mut [i64],
    exponent: usize,
    lower_limit: f32,
    factor_value: f32,
) -> bool {
    kernel::initialize_sectoral_batch_native::<f32, S>(
        simd,
        sine,
        values,
        scales,
        exponent,
        lower_limit,
        factor_value,
    )
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
    kernel::initialize_sectoral_batch_f64(
        simd,
        sine,
        values,
        scales,
        exponent,
        lower_limit,
        factor_value,
    )
}

#[inline(always)]
pub(super) fn rescale_ordered_batch_f32<
    S: Simd,
    const TRACK_ANY_RESCALED: bool,
    const TRACK_PREFIX_EPOCH: bool,
>(
    simd: S,
    previous: &mut [f32],
    current: &mut [f32],
    scale: &mut [i64],
) -> Option<(bool, bool)> {
    if previous.len() != current.len() || current.len() != scale.len() {
        return None;
    }

    let lanes = S::F32_LANES;
    if lanes <= 1 || lanes > 16 || S::I32_LANES != lanes {
        return kernel::rescale_batch_inner_scalar::<f32, S, TRACK_ANY_RESCALED, TRACK_PREFIX_EPOCH>(
            simd, previous, current, scale,
        );
    }

    let mut lane_flags = [0_i32; 16];
    let flag_storage_supported = {
        let (flag_vectors, flag_tail) = S::as_mut_simd_i32s(&mut lane_flags[..lanes]);
        flag_vectors.len() == 1 && flag_tail.is_empty()
    };
    if !flag_storage_supported {
        return kernel::rescale_batch_inner_scalar::<f32, S, TRACK_ANY_RESCALED, TRACK_PREFIX_EPOCH>(
            simd, previous, current, scale,
        );
    }

    let (previous_vectors, previous_tail) = S::as_mut_simd_f32s(previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f32s(current);
    if previous_vectors.len() != current_vectors.len() || previous_tail.len() != current_tail.len()
    {
        return None;
    }

    let mut any_rescaled = false;
    let mut all_below_one = true;
    let ftol = simd.splat_f32s(<f32 as OrderedSimdPrecision>::SCALE_FTOL);
    let scale_down = simd.splat_f32s(<f32 as LegendrePrecision>::SCALE_DOWN);
    let zero = simd.splat_f32s(0.0);

    for vector_index in 0..current_vectors.len() {
        let start = vector_index * lanes;
        let scale_group = &mut scale[start..start + lanes];
        let previous_vector = previous_vectors[vector_index];
        let current_vector = current_vectors[vector_index];
        let rescale_mask = simd.greater_than_f32s(simd.abs_f32s(current_vector), ftol);

        if simd.first_true_m32s(rescale_mask) == lanes {
            if TRACK_PREFIX_EPOCH {
                all_below_one &= scale_group.iter().all(|&value| value < 1);
            }
            continue;
        }

        let scaled_previous = simd.mul_f32s(previous_vector, scale_down);
        let scaled_current = simd.mul_f32s(current_vector, scale_down);
        let previous_underflow = simd.and_m32s(
            simd.not_m32s(simd.equal_f32s(previous_vector, zero)),
            simd.equal_f32s(scaled_previous, zero),
        );
        let current_underflow = simd.and_m32s(
            simd.not_m32s(simd.equal_f32s(current_vector, zero)),
            simd.equal_f32s(scaled_current, zero),
        );
        let underflow = simd.or_m32s(previous_underflow, current_underflow);
        let safe_rescale = simd.and_m32s(rescale_mask, simd.not_m32s(underflow));
        let next_previous = simd.select_f32s(safe_rescale, scaled_previous, previous_vector);
        let next_current = simd.select_f32s(safe_rescale, scaled_current, current_vector);
        let stored_flags = {
            let (flag_vectors, flag_tail) = S::as_mut_simd_i32s(&mut lane_flags[..lanes]);
            if flag_vectors.len() != 1 || !flag_tail.is_empty() {
                false
            } else {
                flag_vectors[0] =
                    simd.select_i32s(safe_rescale, simd.splat_i32s(1), simd.splat_i32s(0));
                true
            }
        };
        if !stored_flags {
            return None;
        }

        for (lane, &update) in scale_group.iter().zip(&lane_flags[..lanes]) {
            if update != 0 {
                lane.checked_add(1)?;
            }
        }
        for (lane, &update) in scale_group.iter_mut().zip(&lane_flags[..lanes]) {
            if update != 0 {
                *lane += 1;
                if TRACK_ANY_RESCALED {
                    any_rescaled = true;
                }
            }
            if TRACK_PREFIX_EPOCH {
                all_below_one &= *lane < 1;
            }
        }
        previous_vectors[vector_index] = next_previous;
        current_vectors[vector_index] = next_current;
    }

    let tail_offset = previous_vectors.len() * lanes;
    for lane in 0..current_tail.len() {
        let index = tail_offset + lane;
        if current_tail[lane].abs() > <f32 as OrderedSimdPrecision>::SCALE_FTOL {
            let next_scale = scale[index].checked_add(1)?;
            let next_previous = previous_tail[lane] * <f32 as LegendrePrecision>::SCALE_DOWN;
            let next_current = current_tail[lane] * <f32 as LegendrePrecision>::SCALE_DOWN;
            if (previous_tail[lane] == 0.0 || next_previous != 0.0)
                && (current_tail[lane] == 0.0 || next_current != 0.0)
            {
                previous_tail[lane] = next_previous;
                current_tail[lane] = next_current;
                scale[index] = next_scale;
                if TRACK_ANY_RESCALED {
                    any_rescaled = true;
                }
            }
        }
        if TRACK_PREFIX_EPOCH {
            all_below_one &= scale[index] < 1;
        }
    }

    Some((any_rescaled, all_below_one))
}

pub(super) fn rescale_ordered_batch_f64<
    S: Simd,
    const TRACK_ANY_RESCALED: bool,
    const TRACK_PREFIX_EPOCH: bool,
>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale: &mut [i64],
) -> Option<(bool, bool)> {
    kernel::rescale_batch_inner_scalar::<f64, S, TRACK_ANY_RESCALED, TRACK_PREFIX_EPOCH>(
        simd, previous, current, scale,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The shared typed prefix routine receives the complete scaled recurrence state."
)]
pub(super) fn skip_negligible_scaled_prefix<T: OrderedSimdPrecision, S: Simd>(
    simd: S,
    m: usize,
    lmax: usize,
    pair_count: usize,
    csq: &[T],
    coefficient_a: &[T],
    coefficient_b: &[T],
    previous: &mut [T],
    current: &mut [T],
    scale: &mut [i64],
) -> Option<(usize, usize, bool)> {
    kernel::skip_negligible_scaled_prefix::<T, S>(
        simd,
        m,
        lmax,
        pair_count,
        csq,
        coefficient_a,
        coefficient_b,
        previous,
        current,
        scale,
    )
}

pub(super) fn rescale_prefix_batch_f64<
    S: Simd,
    const TRACK_ANY_RESCALED: bool,
    const TRACK_PREFIX_EPOCH: bool,
>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale: &mut [i64],
) -> Option<(bool, bool)> {
    kernel::f64_rescale_batch_inner::<S, TRACK_ANY_RESCALED, TRACK_PREFIX_EPOCH>(
        simd, previous, current, scale,
    )
}

#[cfg(test)]
pub(crate) fn reset_work_counters() {
    kernel::reset_work_counters();
}

#[cfg(test)]
pub(crate) fn work_counter_snapshot() -> ScalarOrderedWorkCounters {
    kernel::work_counter_snapshot()
}
#[cfg(test)]
use kernel::{
    FTOL, SCALE_DOWN, SCALE_UP, all_scale_nonnegative, initialize_sectoral_batch, mypow,
    normalize_batch, normalize_lane, power_lower_limit, rescale_batch, rescale_prefix_step,
    scalar_integer_power, set_correction_factors,
};

pub(crate) struct ScalarOrderedScratch<'a, T> {
    pub(crate) epsilon: &'a mut [T],
    pub(crate) alpha: &'a mut [T],
    pub(crate) coefficient_a: &'a mut [T],
    pub(crate) coefficient_b: &'a mut [T],
    pub(crate) coefficient_even: &'a mut [num_complex::Complex<T>],
    pub(crate) coefficient_odd: &'a mut [num_complex::Complex<T>],
    pub(crate) packed_sin: &'a mut [T],
    pub(crate) packed_csq: &'a mut [T],
    pub(crate) previous: &'a mut [T],
    pub(crate) current: &'a mut [T],
    pub(crate) scale: &'a mut [i64],
    pub(crate) corfac: &'a mut [T],
    pub(crate) sum_even_re: &'a mut [T],
    pub(crate) sum_even_im: &'a mut [T],
    pub(crate) sum_odd_re: &'a mut [T],
    pub(crate) sum_odd_im: &'a mut [T],
}

/// Shared destination for parallel jobs that each own a contiguous Fourier-order range.
///
/// The output is ring-major, while each parallel job writes disjoint
/// contiguous order columns. This avoids materializing and transposing a second full
/// spectrum array.
pub(crate) struct DisjointSpectrumOutput<'a, T> {
    pointer: NonNull<num_complex::Complex<T>>,
    ntheta: usize,
    spectrum_len: usize,
    _exclusive: PhantomData<&'a mut [num_complex::Complex<T>]>,
}

// SAFETY: a shared output can only produce a writer through the unsafe
// `writer_for_unique_order_range` method. The caller assigns each order exactly
// once and does not assign overlapping ranges; the buffer is not read until all
// parallel order jobs have joined.
unsafe impl<T: Send + Sync> Sync for DisjointSpectrumOutput<'_, T> {}

#[allow(dead_code)]
impl<'a, T: Send + Sync> DisjointSpectrumOutput<'a, T> {
    pub(crate) fn new(
        spectra: &'a mut [num_complex::Complex<T>],
        ntheta: usize,
        spectrum_len: usize,
    ) -> Option<Self> {
        let required = ntheta.checked_mul(spectrum_len)?;
        if ntheta == 0 || spectrum_len == 0 || spectra.len() < required {
            return None;
        }
        Some(Self {
            pointer: NonNull::new(spectra.as_mut_ptr())?,
            ntheta,
            spectrum_len,
            _exclusive: PhantomData,
        })
    }

    /// Creates the sole writer for `orders` across the active parallel region.
    ///
    /// # Safety
    ///
    /// The caller must ensure no other live writer targets any order in the
    /// range. Each ring and order must also be written at most once before the
    /// output is read.
    pub(crate) unsafe fn writer_for_unique_order_range(
        &self,
        order_start: usize,
        order_end: usize,
    ) -> Option<DisjointSpectrumWriter<'a, T>> {
        (order_start < order_end && order_end <= self.spectrum_len).then_some(
            DisjointSpectrumWriter {
                pointer: self.pointer,
                ntheta: self.ntheta,
                spectrum_len: self.spectrum_len,
                order_start,
                order_end,
                _buffer: PhantomData,
                _not_sync: PhantomData,
            },
        )
    }

    /// Creates a sole writer for one order, retained for spin-1's independent
    /// per-order output path.
    ///
    /// # Safety
    ///
    /// No other live writer may target this order, and each ring in the order
    /// must be written at most once before the output is read.
    pub(crate) unsafe fn writer_for_unique_order(
        &self,
        order: usize,
    ) -> Option<DisjointSpectrumWriter<'a, T>> {
        let order_end = order.checked_add(1)?;
        // SAFETY: this wrapper requests a one-element range; exclusivity is
        // the caller's obligation, as documented above.
        unsafe { self.writer_for_unique_order_range(order, order_end) }
    }
}

pub(crate) struct DisjointSpectrumWriter<'a, T> {
    pointer: NonNull<num_complex::Complex<T>>,
    ntheta: usize,
    spectrum_len: usize,
    order_start: usize,
    order_end: usize,
    _buffer: PhantomData<&'a num_complex::Complex<T>>,
    _not_sync: PhantomData<Cell<()>>,
}

/// Immutable geometry whose north/south reflection and polar-order limits
/// were checked once before the synthesis call entered its per-order workers.
pub(crate) struct ValidatedReflectedGeometry<'a, T> {
    sin_theta: &'a [T],
    cos_theta: &'a [T],
    ring_order_limits: &'a [usize],
}

pub(crate) fn validate_reflected_geometry<'a, T: OrderedSimdPrecision>(
    sin_theta: &'a [T],
    cos_theta: &'a [T],
    ring_order_limits: &'a [usize],
) -> Option<ValidatedReflectedGeometry<'a, T>> {
    let ntheta = sin_theta.len();
    if ntheta == 0 || cos_theta.len() != ntheta || ring_order_limits.len() != ntheta {
        return None;
    }
    for north in 0..ntheta.div_ceil(2) {
        let south = ntheta - 1 - north;
        if (sin_theta[north] - sin_theta[south]).abs() > T::REFLECTION_TOLERANCE
            || (cos_theta[north] + cos_theta[south]).abs() > T::REFLECTION_TOLERANCE
            || (north > 0 && ring_order_limits[north] < ring_order_limits[north - 1])
        {
            return None;
        }
    }
    Some(ValidatedReflectedGeometry {
        sin_theta,
        cos_theta,
        ring_order_limits,
    })
}

/// Zeroes only the Fourier cells that the reflected-order kernel will not
/// write. Each active prefix is written by one order job; the remaining tail
/// contains polar-excluded orders and modes above `mmax`.
pub(crate) fn initialize_inactive_spectrum_tails<T: Copy>(
    spectra: &mut [num_complex::Complex<T>],
    spectrum_len: usize,
    mmax: usize,
    geometry: &ValidatedReflectedGeometry<'_, T>,
    zero: num_complex::Complex<T>,
) -> bool {
    let Some(required_len) = geometry.ring_order_limits.len().checked_mul(spectrum_len) else {
        return false;
    };
    if spectrum_len == 0 || mmax >= spectrum_len || spectra.len() != required_len {
        return false;
    }

    let ntheta = geometry.ring_order_limits.len();
    for (ring, row) in spectra.chunks_exact_mut(spectrum_len).enumerate() {
        let north = ring.min(ntheta - 1 - ring);
        let first_inactive = geometry.ring_order_limits[north].min(mmax) + 1;
        row[first_inactive..].fill(zero);
    }
    true
}

impl<T: Copy> DisjointSpectrumWriter<'_, T> {
    #[cfg(feature = "parallel")]
    #[inline]
    pub(crate) fn write(&self, ring: usize, value: num_complex::Complex<T>) {
        debug_assert_eq!(self.order_end, self.order_start + 1);
        self.write_order(ring, self.order_start, value);
    }

    #[inline]
    pub(crate) fn write_order(&self, ring: usize, order: usize, value: num_complex::Complex<T>) {
        debug_assert!(ring < self.ntheta);
        debug_assert!((self.order_start..self.order_end).contains(&order));
        debug_assert!(order < self.spectrum_len);
        let index = ring * self.spectrum_len + order;
        // SAFETY: the output constructor checked `ntheta * spectrum_len` for
        // overflow and against the slice length. This writer owns a unique
        // order range, and the recurrence emits each physical ring once for it.
        unsafe { self.pointer.as_ptr().add(index).write(value) };
    }
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
pub(crate) fn synthesize_dispatched<'a, T: OrderedSimdPrecision>(
    arch: Arch,
    alm: &'a [Complex<T>],
    lmax: usize,
    mmax: usize,
    nphi: usize,
    spectrum_len: usize,
    phi0: f64,
    sectoral_norms: &'a [T],
    sin_theta: &'a [T],
    cos_theta: &'a [T],
    ring_order_limits: &'a [usize],
    spectra: &'a mut [Complex<T>],
    scratch: &'a mut ScalarOrderedScratch<'_, T>,
) -> bool {
    let Some(order_end) = mmax.checked_add(1) else {
        return false;
    };
    let Some(geometry) = validate_reflected_geometry(sin_theta, cos_theta, ring_order_limits)
    else {
        return false;
    };
    if !initialize_inactive_spectrum_tails(
        spectra,
        spectrum_len,
        mmax,
        &geometry,
        Complex::new(T::zero(), T::zero()),
    ) {
        spectra.fill(Complex::new(T::zero(), T::zero()));
        return false;
    }
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
        &geometry,
        spectra,
        None,
        scratch,
    )
}

#[cfg(feature = "parallel")]
/// Runs a short order range against its uniquely owned output columns.
///
/// # Safety
///
/// The caller must assign the complete order range to exactly one live
/// parallel job, no other job may own an overlapping range, and that job must
/// not write any ring/order cell more than once. All jobs must join before the
/// destination is read.
#[expect(
    clippy::too_many_arguments,
    reason = "The private order-range dispatch receives validated plan data and disjoint output columns."
)]
pub(crate) unsafe fn synthesize_order_output_dispatched<
    'a,
    T: OrderedSimdPrecision + Send + Sync,
>(
    arch: Arch,
    alm: &'a [Complex<T>],
    lmax: usize,
    mmax: usize,
    order_start: usize,
    order_end: usize,
    nphi: usize,
    spectrum_len: usize,
    phi0: f64,
    sectoral_norms: &'a [T],
    geometry: &ValidatedReflectedGeometry<'a, T>,
    output: &DisjointSpectrumOutput<'a, T>,
    scratch: &'a mut ScalarOrderedScratch<'_, T>,
) -> bool {
    if output.ntheta != geometry.sin_theta.len() || output.spectrum_len != spectrum_len {
        return false;
    }
    // SAFETY: the caller assigns one dynamic job to each nonoverlapping range
    // and never creates another writer for these orders during the parallel region.
    let Some(writer) = (unsafe { output.writer_for_unique_order_range(order_start, order_end) })
    else {
        return false;
    };
    let mut empty_spectra = [];
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
        geometry,
        &mut empty_spectra,
        Some(writer),
        scratch,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The private scalar dispatch receives validated plan data and reusable workspace scratch."
)]
fn synthesize_range_dispatched<'a, T: OrderedSimdPrecision>(
    arch: Arch,
    alm: &'a [Complex<T>],
    lmax: usize,
    mmax: usize,
    order_start: usize,
    order_end: usize,
    nphi: usize,
    spectrum_len: usize,
    phi0: f64,
    sectoral_norms: &'a [T],
    geometry: &ValidatedReflectedGeometry<'a, T>,
    spectra: &'a mut [Complex<T>],
    disjoint_output: Option<DisjointSpectrumWriter<'a, T>>,
    scratch: &'a mut ScalarOrderedScratch<'_, T>,
) -> bool {
    let sin_theta = geometry.sin_theta;
    let cos_theta = geometry.cos_theta;
    let ring_order_limits = geometry.ring_order_limits;
    let ntheta = sin_theta.len();
    let Some(order_count) = mmax.checked_add(1) else {
        return false;
    };
    let Some(expected_spectrum_count) = ntheta.checked_mul(spectrum_len) else {
        return false;
    };
    let output_len_valid = if disjoint_output.is_some() {
        spectra.is_empty()
            && disjoint_output.as_ref().is_some_and(|writer| {
                writer.order_start == order_start && writer.order_end == order_end
            })
    } else {
        spectra.len() == expected_spectrum_count
    };
    if mmax > lmax
        || order_start >= order_end
        || order_end > order_count
        || ntheta == 0
        || cos_theta.len() != ntheta
        || ring_order_limits.len() != ntheta
        || !output_len_valid
        || sectoral_norms.len() <= mmax
        || spectrum_len == 0
        || mmax >= spectrum_len
        || nphi == 0
        || scratch.packed_sin.len() < SCALAR_ORDER_BATCH_SIZE
        || scratch.packed_csq.len() < SCALAR_ORDER_BATCH_SIZE
        || scratch.previous.len() < SCALAR_ORDER_BATCH_SIZE
        || scratch.current.len() < SCALAR_ORDER_BATCH_SIZE
        || scratch.scale.len() < SCALAR_ORDER_BATCH_SIZE
        || scratch.corfac.len() < SCALAR_ORDER_BATCH_SIZE
        || scratch.sum_even_re.len() < SCALAR_ORDER_BATCH_SIZE
        || scratch.sum_even_im.len() < SCALAR_ORDER_BATCH_SIZE
        || scratch.sum_odd_re.len() < SCALAR_ORDER_BATCH_SIZE
        || scratch.sum_odd_im.len() < SCALAR_ORDER_BATCH_SIZE
    {
        return false;
    }

    if !phi0.is_finite() {
        return false;
    }
    let output_is_disjoint = disjoint_output.is_some();
    let succeeded = arch.dispatch(ScalarOrderedSynthesis {
        alm,
        lmax,
        order_start,
        order_end,
        nphi,
        spectrum_len,
        disjoint_output,
        phi0,
        sectoral_norms,
        sin_theta,
        cos_theta,
        ring_order_limits,
        spectra,
        scratch,
    });
    #[cfg(test)]
    kernel::record_route_result(succeeded);
    if !succeeded && !output_is_disjoint {
        spectra.fill(Complex::new(T::zero(), T::zero()));
    }
    succeeded
}

struct ScalarOrderedSynthesis<'a, 's, T> {
    alm: &'a [Complex<T>],
    lmax: usize,
    order_start: usize,
    order_end: usize,
    nphi: usize,
    spectrum_len: usize,
    disjoint_output: Option<DisjointSpectrumWriter<'a, T>>,
    phi0: f64,
    sectoral_norms: &'a [T],
    sin_theta: &'a [T],
    cos_theta: &'a [T],
    ring_order_limits: &'a [usize],
    spectra: &'a mut [Complex<T>],
    scratch: &'a mut ScalarOrderedScratch<'s, T>,
}

impl<T: OrderedSimdPrecision> WithSimd for ScalarOrderedSynthesis<'_, '_, T> {
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self {
            alm,
            lmax,
            order_start,
            order_end,
            nphi,
            spectrum_len,
            mut disjoint_output,
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
            let power_lower_limit = kernel::power_lower_limit::<T>(m);
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

            let phase = longitude_phase::<T>(m, phi0, false);
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
                let active_count = (north_count - ring_cursor).min(SCALAR_ORDER_BATCH_SIZE);
                for lane in 0..active_count {
                    let north = ring_cursor + lane;
                    let sine = sin_theta[north];
                    let cosine = cos_theta[north];
                    scratch.packed_csq[lane] = if cosine.abs() > T::from_f64(0.99).unwrap() {
                        (T::one() - sine) * (T::one() + sine)
                    } else {
                        cosine * cosine
                    };
                    debug_assert!(m <= ring_order_limits[north]);
                }

                // DUCC pads an incomplete vector with the last valid ring.
                // Repeating its recurrence inputs lets the ordinary and
                // scaled paths stay vectorized; reconstruction still writes
                // only the active lanes below.
                let simd_lanes = T::lane_count::<S>();
                if simd_lanes == 0 {
                    return false;
                }
                let Some(simd_count) = active_count.div_ceil(simd_lanes).checked_mul(simd_lanes)
                else {
                    return false;
                };
                if simd_count > SCALAR_ORDER_BATCH_SIZE {
                    return false;
                }
                if simd_count > active_count {
                    let last = active_count - 1;
                    for lane in active_count..simd_count {
                        scratch.packed_csq[lane] = scratch.packed_csq[last];
                    }
                }

                if !kernel::synthesize_batch::<T, S>(
                    simd,
                    scratch,
                    m,
                    lmax,
                    sectoral_norm,
                    power_lower_limit,
                    kernel::ScalarSynthesisBatch {
                        sin_theta: &sin_theta[ring_cursor..ring_cursor + active_count],
                        active_lanes: active_count,
                        vector_lanes: simd_count,
                    },
                ) {
                    return false;
                }
                #[cfg(test)]
                {
                    let center_writes = usize::from(
                        !ntheta.is_multiple_of(2)
                            && (ring_cursor..ring_cursor + active_count).contains(&(ntheta / 2)),
                    );
                    kernel::record_output_ring_batch(2 * active_count - center_writes);
                }
                for lane in 0..active_count {
                    let north = ring_cursor + lane;
                    let south = ntheta - 1 - north;
                    let even_re = scratch.sum_even_re[lane];
                    let even_im = scratch.sum_even_im[lane];
                    let odd_re = scratch.sum_odd_re[lane] * cos_theta[north];
                    let odd_im = scratch.sum_odd_im[lane] * cos_theta[north];
                    let (mut north_value, mut south_value) = (
                        Complex::new(even_re + odd_re, even_im + odd_im),
                        Complex::new(even_re - odd_re, even_im - odd_im),
                    );
                    north_value = north_value * phase;
                    south_value = south_value * phase;
                    if nphi.is_multiple_of(2) && m == nphi / 2 {
                        let two = T::one() + T::one();
                        north_value = Complex::new(two * north_value.re, T::zero());
                        south_value = Complex::new(two * south_value.re, T::zero());
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
                        &mut disjoint_output,
                        m,
                        north,
                        north_value,
                    );
                    if south != north {
                        scratch_output(
                            spectra,
                            spectrum_len,
                            &mut disjoint_output,
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
fn scratch_output<T: Copy>(
    spectra: &mut [Complex<T>],
    spectrum_len: usize,
    disjoint_output: &mut Option<DisjointSpectrumWriter<'_, T>>,
    order: usize,
    ring: usize,
    value: Complex<T>,
) {
    if let Some(output) = disjoint_output {
        output.write_order(ring, order, value);
        return;
    }
    spectra[ring * spectrum_len + order] = value;
}

fn packed_order_offset(m: usize, lmax: usize) -> Option<usize> {
    let row_start = m.checked_mul(lmax.checked_add(1)?)?;
    let preceding = m.checked_mul(m.saturating_sub(1))?.checked_div(2)?;
    row_start.checked_sub(preceding)
}

fn is_subnormal<T: OrderedSimdPrecision>(value: T) -> bool {
    value != T::zero() && value.abs() < T::MIN_POSITIVE
}

fn prepare_order<T: OrderedSimdPrecision>(
    scratch: &mut ScalarOrderedScratch<'_, T>,
    m: usize,
    lmax: usize,
    coefficients: &[Complex<T>],
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
    if T::PREPARE_ORDER_CONSTANTS_IN_F64 {
        return prepare_order_with_wide_constants(scratch, m, lmax, coefficients, pair_count);
    }
    crate::legendre::numeric::fill_epsilon(m, lmax, scratch.epsilon);
    if !scalar_paired::fill_factors_typed(
        m,
        lmax,
        scratch.epsilon,
        scratch.alpha,
        scratch.coefficient_a,
        scratch.coefficient_b,
        T::SYNTHESIS_FACTOR_LIMIT,
    ) {
        return false;
    }
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

/// Prepares native-f32 recurrence tables from f64 order constants while
/// keeping the runtime spectrum coefficients and transformed coefficient
/// arrays in f32. The paired recurrence is ill-conditioned near polar roots;
/// rounding each recursively generated alpha factor to f32 before its next
/// use can materially shift high-degree values.
fn prepare_order_with_wide_constants<T: OrderedSimdPrecision>(
    scratch: &mut ScalarOrderedScratch<'_, T>,
    m: usize,
    lmax: usize,
    coefficients: &[Complex<T>],
    pair_count: usize,
) -> bool {
    let epsilon64 = |degree: usize| {
        if degree == m {
            return 0.0_f64;
        }
        let degree = degree as f64;
        let order = m as f64;
        (((degree - order) * (degree + order)) / ((2.0 * degree + 1.0) * (2.0 * degree - 1.0)))
            .sqrt()
    };

    for (degree, epsilon) in scratch
        .epsilon
        .iter_mut()
        .enumerate()
        .take(lmax + 4)
        .skip(m)
    {
        *epsilon = T::from_f64(epsilon64(degree)).expect("epsilon is representable in f32");
    }

    let mut alpha64 = 1.0 / epsilon64(m + 1);
    let alpha1 = epsilon64(m + 1) / (epsilon64(m + 2) * epsilon64(m + 3));
    for pair in 0..pair_count {
        if pair == 1 {
            alpha64 = alpha1;
        } else if pair >= 2 {
            let degree = m + pair * 2;
            let sign = if (pair - 1).is_multiple_of(2) {
                1.0_f64
            } else {
                -1.0_f64
            };
            alpha64 = sign / (epsilon64(degree) * epsilon64(degree + 1) * alpha64);
        }
        let degree = m + pair * 2;
        let alpha = T::from_f64(alpha64).expect("order normalization is representable in f32");
        let sign = if pair.is_multiple_of(2) {
            1.0_f64
        } else {
            -1.0_f64
        };
        let coefficient_a64 = sign * alpha64 * alpha64;
        let coefficient_b64 =
            -coefficient_a64 * (epsilon64(degree + 2).powi(2) + epsilon64(degree + 1).powi(2));
        let coefficient_a = T::from_f64(coefficient_a64).expect("factor is representable in f32");
        let coefficient_b = T::from_f64(coefficient_b64).expect("factor is representable in f32");
        if !alpha.is_finite()
            || !coefficient_a.is_finite()
            || !coefficient_b.is_finite()
            || alpha.abs() > T::SYNTHESIS_FACTOR_LIMIT
            || coefficient_a.abs() > T::SYNTHESIS_FACTOR_LIMIT
            || coefficient_b.abs() > T::SYNTHESIS_FACTOR_LIMIT
        {
            return false;
        }
        scratch.alpha[pair] = alpha;
        scratch.coefficient_a[pair] = coefficient_a;
        scratch.coefficient_b[pair] = coefficient_b;

        let offset = pair * 2;
        let Some(&coefficient) = coefficients.get(offset) else {
            return false;
        };
        let next_even = if degree + 2 <= lmax {
            let Some(&next_even) = coefficients.get(offset + 2) else {
                return false;
            };
            next_even
        } else {
            Complex::new(T::zero(), T::zero())
        };
        let even_first = T::from_f64(epsilon64(degree + 1) * alpha64)
            .expect("paired coefficient factor is representable in f32");
        let even_second = T::from_f64(epsilon64(degree + 2) * alpha64)
            .expect("paired coefficient factor is representable in f32");
        let even = coefficient * even_first + next_even * even_second;
        let odd = if degree < lmax {
            let Some(&coefficient) = coefficients.get(offset + 1) else {
                return false;
            };
            coefficient * alpha
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
        scratch.coefficient_even[pair] = even;
        scratch.coefficient_odd[pair] = odd;
    }
    !scratch.coefficient_even[..pair_count]
        .iter()
        .chain(&scratch.coefficient_odd[..pair_count])
        .any(|coefficient| is_subnormal(coefficient.re) || is_subnormal(coefficient.im))
}

#[cfg(test)]
mod tests;
