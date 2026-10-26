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
use std::{cell::Cell, marker::PhantomData, ptr::NonNull};

use crate::legendre::{SCALAR_ORDER_BATCH_SIZE, longitude_phase, scalar, scalar_paired};

mod kernel;

pub(crate) use kernel::synthesize_prepared_component_batch_dispatched;
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

struct ScalarF64LaneCount;

impl WithSimd for ScalarF64LaneCount {
    type Output = usize;

    #[inline(always)]
    fn with_simd<S: Simd>(self, _simd: S) -> Self::Output {
        S::F64_LANES
    }
}

pub(crate) fn f64_lane_count(arch: Arch) -> usize {
    arch.dispatch(ScalarF64LaneCount)
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
pub(crate) struct ValidatedReflectedGeometry<'a> {
    sin_theta: &'a [f64],
    cos_theta: &'a [f64],
    ring_order_limits: &'a [usize],
}

pub(crate) fn validate_reflected_geometry<'a>(
    sin_theta: &'a [f64],
    cos_theta: &'a [f64],
    ring_order_limits: &'a [usize],
) -> Option<ValidatedReflectedGeometry<'a>> {
    let ntheta = sin_theta.len();
    if ntheta == 0 || cos_theta.len() != ntheta || ring_order_limits.len() != ntheta {
        return None;
    }
    for north in 0..ntheta.div_ceil(2) {
        let south = ntheta - 1 - north;
        if (sin_theta[north] - sin_theta[south]).abs() > 5.0e-15
            || (cos_theta[north] + cos_theta[south]).abs() > 5.0e-15
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
    geometry: &ValidatedReflectedGeometry<'_>,
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
    scratch: &'a mut ScalarOrderedScratch<'_, f64>,
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
        Complex64::new(0.0, 0.0),
    ) {
        spectra.fill(Complex64::new(0.0, 0.0));
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
pub(crate) unsafe fn synthesize_order_output_dispatched<'a>(
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
    geometry: &ValidatedReflectedGeometry<'a>,
    output: &DisjointSpectrumOutput<'a, f64>,
    scratch: &'a mut ScalarOrderedScratch<'_, f64>,
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
    geometry: &ValidatedReflectedGeometry<'a>,
    spectra: &'a mut [Complex64],
    disjoint_output: Option<DisjointSpectrumWriter<'a, f64>>,
    scratch: &'a mut ScalarOrderedScratch<'_, f64>,
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
    if !succeeded && !output_is_disjoint {
        spectra.fill(Complex64::new(0.0, 0.0));
    }
    succeeded
}

struct ScalarOrderedSynthesis<'a, 's> {
    alm: &'a [Complex64],
    lmax: usize,
    order_start: usize,
    order_end: usize,
    nphi: usize,
    spectrum_len: usize,
    disjoint_output: Option<DisjointSpectrumWriter<'a, f64>>,
    phi0: f64,
    sectoral_norms: &'a [f64],
    sin_theta: &'a [f64],
    cos_theta: &'a [f64],
    ring_order_limits: &'a [usize],
    spectra: &'a mut [Complex64],
    scratch: &'a mut ScalarOrderedScratch<'s, f64>,
}

impl WithSimd for ScalarOrderedSynthesis<'_, '_> {
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
            let power_lower_limit = kernel::power_lower_limit(m);
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
                let active_count = (north_count - ring_cursor).min(SCALAR_ORDER_BATCH_SIZE);
                for lane in 0..active_count {
                    let north = ring_cursor + lane;
                    let sine = sin_theta[north];
                    let cosine = cos_theta[north];
                    scratch.packed_csq[lane] = if cosine.abs() > 0.99 {
                        (1.0 - sine) * (1.0 + sine)
                    } else {
                        cosine * cosine
                    };
                    debug_assert!(m <= ring_order_limits[north]);
                }

                // DUCC pads an incomplete vector with the last valid ring.
                // Repeating its recurrence inputs lets the ordinary and
                // scaled paths stay vectorized; reconstruction still writes
                // only the active lanes below.
                let simd_lanes = S::F64_LANES;
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

                if !kernel::synthesize_batch(
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
                for lane in 0..active_count {
                    let north = ring_cursor + lane;
                    let south = ntheta - 1 - north;
                    let even_re = scratch.sum_even_re[lane];
                    let even_im = scratch.sum_even_im[lane];
                    let odd_re = scratch.sum_odd_re[lane] * cos_theta[north];
                    let odd_im = scratch.sum_odd_im[lane] * cos_theta[north];
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
fn scratch_output(
    spectra: &mut [Complex64],
    spectrum_len: usize,
    disjoint_output: &mut Option<DisjointSpectrumWriter<'_, f64>>,
    order: usize,
    ring: usize,
    value: Complex64,
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

fn is_subnormal(value: f64) -> bool {
    value != 0.0 && value.abs() < f64::MIN_POSITIVE
}

fn prepare_order(
    scratch: &mut ScalarOrderedScratch<'_, f64>,
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
    if !scalar_paired::fill_factors(
        m,
        lmax,
        scratch.epsilon,
        scratch.alpha,
        scratch.coefficient_a,
        scratch.coefficient_b,
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

#[cfg(test)]
mod tests {
    use super::{
        FTOL, SCALE_DOWN, SCALE_UP, WithSimd, all_scale_nonnegative, initialize_sectoral_batch,
        mypow, normalize_batch, normalize_lane, power_lower_limit, rescale_batch,
        rescale_prefix_step, scalar_integer_power, set_correction_factors,
        validate_reflected_geometry,
    };
    #[cfg(feature = "parallel")]
    use num_complex::Complex64;
    use pulp::{Arch, Simd};

    #[cfg(feature = "parallel")]
    use super::initialize_inactive_spectrum_tails;

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

    struct PrefixStepDispatch<'a> {
        previous: &'a mut [f64],
        current: &'a mut [f64],
        scales: &'a mut [i64],
    }

    impl WithSimd for PrefixStepDispatch<'_> {
        type Output = Option<bool>;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            rescale_prefix_step(simd, self.previous, self.current, self.scales)
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
    fn reflected_geometry_preflight_checks_symmetry_and_order_limits() {
        let sin_theta = [0.2, 0.8, 0.8, 0.2];
        let cos_theta = [0.98, 0.6, -0.6, -0.98];
        let ring_order_limits = [1, 3, 3, 1];
        assert!(validate_reflected_geometry(&sin_theta, &cos_theta, &ring_order_limits,).is_some());

        let mut asymmetric_sin = sin_theta;
        asymmetric_sin[3] += 1.0e-12;
        assert!(
            validate_reflected_geometry(&asymmetric_sin, &cos_theta, &ring_order_limits,).is_none()
        );

        let nonmonotone_limits = [3, 1, 1, 3];
        assert!(
            validate_reflected_geometry(&sin_theta, &cos_theta, &nonmonotone_limits,).is_none()
        );
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn sparse_spectrum_tails_follow_northern_reflected_order_limits() {
        let sin_theta = [0.2, 0.8, 0.8, 0.2];
        let cos_theta = [0.98, 0.6, -0.6, -0.98];
        let limits = [1, 3, 99, 99];
        let geometry = validate_reflected_geometry(&sin_theta, &cos_theta, &limits).unwrap();
        let mut spectra = vec![Complex64::new(3.0, -2.0); 4 * 5];
        assert!(initialize_inactive_spectrum_tails(
            &mut spectra,
            5,
            3,
            &geometry,
            Complex64::new(0.0, 0.0),
        ));

        for ring in [0, 3] {
            let row = &spectra[ring * 5..(ring + 1) * 5];
            assert_eq!(row[0], Complex64::new(3.0, -2.0));
            assert_eq!(row[1], Complex64::new(3.0, -2.0));
            assert!(
                row[2..]
                    .iter()
                    .all(|&value| value == Complex64::new(0.0, 0.0))
            );
        }
        for ring in [1, 2] {
            let row = &spectra[ring * 5..(ring + 1) * 5];
            assert!(
                row[..4]
                    .iter()
                    .all(|&value| value == Complex64::new(3.0, -2.0))
            );
            assert_eq!(row[4], Complex64::new(0.0, 0.0));
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
        sine[3] = 0.0;
        sine[4] = f64::from_bits(1);
        sine[18] = lower_limit * 0.25;
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
    fn sectoral_initializer_declines_scale_exponent_overflow() {
        let mut sine = [0.5; 19];
        sine[1] = f64::from_bits(1);
        let mut values = [0.0; 19];
        let mut scales = [0; 19];
        assert!(!Arch::new().dispatch(InitializeDispatch {
            sine: &sine,
            values: &mut values,
            scales: &mut scales,
            exponent: usize::MAX,
            lower_limit: power_lower_limit(usize::MAX),
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
    fn insignificant_prefix_step_skips_scale_reduction_without_rescaling() {
        let previous = vec![0.25; 19];
        let current = vec![FTOL; 19];
        let scales = vec![i64::MAX; 19];

        for arch in [Arch::Scalar, Arch::new()] {
            let mut actual_previous = previous.clone();
            let mut actual_current = current.clone();
            let mut actual_scales = scales.clone();
            let below_limit = arch
                .dispatch(PrefixStepDispatch {
                    previous: &mut actual_previous,
                    current: &mut actual_current,
                    scales: &mut actual_scales,
                })
                .unwrap();

            assert!(below_limit);
            assert_eq!(actual_previous, previous);
            assert_eq!(actual_current, current);
            assert_eq!(actual_scales, scales);
        }
    }

    #[test]
    fn insignificant_prefix_step_checks_scale_epoch_after_rescaling() {
        let previous = vec![0.25; 19];
        let mut current = vec![FTOL; 19];
        current[0] = FTOL * 2.0;
        let mut scales = vec![-1; 19];

        for arch in [Arch::Scalar, Arch::new()] {
            let mut actual_previous = previous.clone();
            let mut actual_current = current.clone();
            let mut actual_scales = scales.clone();
            let below_limit = arch
                .dispatch(PrefixStepDispatch {
                    previous: &mut actual_previous,
                    current: &mut actual_current,
                    scales: &mut actual_scales,
                })
                .unwrap();

            assert!(below_limit);
            assert_eq!(actual_scales[0], 0);

            scales[0] = 0;
            let mut actual_previous = previous.clone();
            let mut actual_current = current.clone();
            let mut actual_scales = scales.clone();
            let below_limit = arch
                .dispatch(PrefixStepDispatch {
                    previous: &mut actual_previous,
                    current: &mut actual_current,
                    scales: &mut actual_scales,
                })
                .unwrap();
            assert!(!below_limit);
            assert_eq!(actual_scales[0], 1);
            scales[0] = -1;
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
