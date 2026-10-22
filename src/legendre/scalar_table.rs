// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * This file contains an adaptation of the normalized scalar recurrence in
 * DUCC's sht_inner_loop.h.
 *
 * This code is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This code is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
 * General Public License for more details.
 *
 * Copyright (C) 2020-2025 Max-Planck-Society
 * Author: Martin Reinecke
 * Source revision: c1ed01b8fda69d2a378190f0915f1a19762dddbf.
 */

use num_complex::Complex;
use pulp::{Arch, Simd, WithSimd};
#[cfg(test)]
use std::sync::atomic::{AtomicU8, Ordering};

use crate::grid::{GridKind, GridSpec};
use crate::harmonics::HarmonicSpec;

use super::{ShtScalar, sin_cos_from_f64};

const MIN_PRECOMPUTED_LMAX: usize = 63;
const MAX_PRECOMPUTED_LMAX: usize = 255;
const MAX_SCALAR_SYNTHESIS_TABLE_LMAX: usize = 63;
pub(crate) const MAX_OVERSAMPLED_TABLE_ANALYSIS_LMAX: usize = 159;
const MAX_TABLE_BYTES: usize = 34 * 1024 * 1024;
// Keep direct table products far from overflow; larger and non-finite input
// scales use the binary-scaled recurrence instead.
const TABLE_INPUT_SCALE_LIMIT: f64 = 1.0e100;

#[cfg(test)]
static BENCHMARK_STRATEGY_OVERRIDE: AtomicU8 = AtomicU8::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LegendreStrategy {
    Recurrence,
    CompactTable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LegendreOperation {
    Synthesis,
    Analysis,
}

#[cfg(all(test, feature = "parallel"))]
pub(crate) fn set_benchmark_strategy_override(strategy: Option<LegendreStrategy>) {
    let encoded = match strategy {
        None => 0,
        Some(LegendreStrategy::Recurrence) => 1,
        Some(LegendreStrategy::CompactTable) => 2,
    };
    BENCHMARK_STRATEGY_OVERRIDE.store(encoded, Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn benchmark_strategy_override() -> Option<LegendreStrategy> {
    match BENCHMARK_STRATEGY_OVERRIDE.load(Ordering::Relaxed) {
        0 => None,
        1 => Some(LegendreStrategy::Recurrence),
        2 => Some(LegendreStrategy::CompactTable),
        _ => unreachable!("invalid benchmark strategy selector"),
    }
}

pub(crate) fn select_scalar_legendre_strategy<T: ShtScalar>(
    lmax: usize,
    mmax: usize,
    spin: usize,
    ntheta: usize,
    operation: LegendreOperation,
    thread_count: usize,
) -> LegendreStrategy {
    if spin == 0 && T::supports_precomputed_scalar() && lmax.checked_add(1) == Some(ntheta) {
        // The exhaustive exact-grid comparison found a simple scalar rule:
        // synthesis uses recurrence, as does parallel analysis; q1 analysis
        // uses the compact table until its estimated payload reaches 20 MiB.
        // Keep this separate from spin-1's table policy.
        let large_analysis_table = operation == LegendreOperation::Analysis
            && thread_count == 1
            && estimated_table_bytes::<T>(lmax, mmax, ntheta)
                .is_some_and(|bytes| bytes >= 20 * 1024 * 1024);
        if operation == LegendreOperation::Synthesis || thread_count >= 8 || large_analysis_table {
            return LegendreStrategy::Recurrence;
        }
    }

    // Preserve the established synthesis cutoff outside the exact-grid scalar
    // policy above.
    if operation == LegendreOperation::Synthesis && lmax > MAX_SCALAR_SYNTHESIS_TABLE_LMAX {
        return LegendreStrategy::Recurrence;
    }

    let oversampled = ntheta > lmax + 1;
    if oversampled {
        // Preserve the existing T63 over-resolved policy. Spin-1 uses its own
        // selector, including the broader measured T159 table eligibility.
        if mmax != lmax
            || (operation == LegendreOperation::Analysis
                && lmax > MAX_OVERSAMPLED_TABLE_ANALYSIS_LMAX)
            || (operation == LegendreOperation::Analysis && lmax == 159 && thread_count >= 16)
            || (operation == LegendreOperation::Synthesis && thread_count == 1)
            || (operation == LegendreOperation::Analysis
                && lmax == MIN_PRECOMPUTED_LMAX
                && thread_count >= 4)
        {
            return LegendreStrategy::Recurrence;
        }
    }
    // These measured exclusions remain for spin-1's shared scalar-table
    // implementation. Scalar exact-grid cases are handled by the
    // operation/thread/footprint rules above; other scalar cases use the
    // existing oversampling and lmax cutoffs.
    let measured_regression = spin != 0
        && match operation {
            LegendreOperation::Synthesis => thread_count == 1 && lmax <= 127,
            LegendreOperation::Analysis => {
                (thread_count >= 8 && lmax == MIN_PRECOMPUTED_LMAX && ntheta == lmax + 1)
                    || (thread_count == 1 && lmax == MAX_PRECOMPUTED_LMAX && mmax >= 192)
                    || (thread_count >= 16
                        && lmax == MAX_PRECOMPUTED_LMAX
                        && mmax == MAX_PRECOMPUTED_LMAX)
            }
        };
    if measured_regression {
        LegendreStrategy::Recurrence
    } else {
        LegendreStrategy::CompactTable
    }
}

fn estimated_table_bytes<T>(lmax: usize, mmax: usize, ntheta: usize) -> Option<usize> {
    let order_count = mmax.checked_add(1)?;
    let unweighted_count = order_count
        .checked_mul(lmax.checked_add(1)?)?
        .checked_sub(mmax.checked_mul(order_count)? / 2)?;
    unweighted_count
        .checked_mul(ntheta.div_ceil(2))?
        .checked_mul(std::mem::size_of::<T>())
}

/// Returns a conservative minimum total-use count for selected table paths.
///
/// This lookup intentionally covers only full-order GL cells with paired
/// cold and warm measurements. A finite caller estimate outside that matrix
/// keeps the recurrence rather than extrapolating setup costs.
pub(crate) fn measured_table_break_even_calls(
    lmax: usize,
    mmax: usize,
    ntheta: usize,
    nphi: usize,
    spin: usize,
    operation: LegendreOperation,
    thread_count: usize,
) -> Option<usize> {
    if mmax != lmax {
        return None;
    }

    match (spin, lmax, ntheta, nphi, operation, thread_count) {
        // Latest same-binary measurements, after the paired spin-1 kernels.
        (0, 127, 128, 257, LegendreOperation::Synthesis, 8) => Some(90),
        (0, 127, 128, 257, LegendreOperation::Analysis, 8) => Some(30),
        (1, 127, 128, 257, LegendreOperation::Synthesis, 1) => Some(14),
        (1, 159, 160, 321, LegendreOperation::Synthesis, 1) => Some(32),
        (1, 191, 192, 385, LegendreOperation::Synthesis, 1) => Some(49),
        (1, 72, 73, 147, LegendreOperation::Analysis, 1) => Some(17),
        (1, 72, 73, 147, LegendreOperation::Analysis, 8) => Some(30),
        (1, 85, 86, 173, LegendreOperation::Analysis, 1) => Some(20),
        (1, 85, 86, 173, LegendreOperation::Analysis, 8) => Some(25),
        (1, 127, 128, 257, LegendreOperation::Analysis, 1) => Some(21),
        (1, 127, 128, 257, LegendreOperation::Analysis, 8) => Some(23),
        (1, 159, 160, 321, LegendreOperation::Analysis, 1 | 8) => Some(30),
        (1, 191, 192, 385, LegendreOperation::Analysis, 1) => Some(54),
        (1, 191, 192, 385, LegendreOperation::Analysis, 8) => Some(70),

        // Full-order atmospheric GL grids. These counts come from the paired
        // first-call and steady-state measurements in the over-resolved study.
        (0, 63, 96, 192, LegendreOperation::Synthesis, 8) => Some(40),
        (0, 63, 96, 192, LegendreOperation::Analysis, 1) => Some(21),
        (0, 63, 96, 192, LegendreOperation::Analysis, 8) => Some(90),
        (1, 63, 96, 192, LegendreOperation::Analysis, 1) => Some(17),
        (1, 63, 96, 192, LegendreOperation::Analysis, 8) => Some(57),
        (0, 85, 128, 256, LegendreOperation::Synthesis, 8) => Some(70),
        (0, 85, 128, 256, LegendreOperation::Analysis, 1) => Some(46),
        (0, 85, 128, 256, LegendreOperation::Analysis, 8) => Some(66),
        (1, 85, 128, 256, LegendreOperation::Analysis, 1) => Some(19),
        (1, 85, 128, 256, LegendreOperation::Analysis, 8) => Some(40),
        (0, 127, 256, 512, LegendreOperation::Synthesis, 8) => Some(62),
        (0, 127, 256, 512, LegendreOperation::Analysis, 8) => Some(107),
        (1, 127, 256, 512, LegendreOperation::Analysis, 1) => Some(49),
        (1, 127, 256, 512, LegendreOperation::Analysis, 8) => Some(45),
        (0, 191, 384, 768, LegendreOperation::Synthesis, 8) => Some(74),

        // The T159 over-resolved analysis table uses the same storage for
        // scalar and spin-1 projection. Its q1/q16 scalar and q1 spin-1 cells
        // remain on recurrence because their measured policy is not a table.
        (0, 159, 320, 640, LegendreOperation::Analysis, 1) => Some(109),
        (0, 159, 320, 640, LegendreOperation::Analysis, 2) => Some(50),
        (0, 159, 320, 640, LegendreOperation::Analysis, 4) => Some(72),
        (0, 159, 320, 640, LegendreOperation::Analysis, 8) => Some(158),
        (1, 159, 320, 640, LegendreOperation::Analysis, 2) => Some(31),
        (1, 159, 320, 640, LegendreOperation::Analysis, 4) => Some(30),
        (1, 159, 320, 640, LegendreOperation::Analysis, 8) => Some(64),
        (1, 159, 320, 640, LegendreOperation::Analysis, 16) => Some(125),
        _ => None,
    }
}

/// North-hemisphere scalar basis for the measured small-band GL specialization.
/// Values are contiguous in north latitude for each `(m, l)` pair.
pub(crate) struct ScalarLegendreTable<T> {
    values: Vec<T>,
    order_offsets: Vec<usize>,
    north_ring_count: usize,
    input_scale_limit: T,
}

fn supported_layout<T: ShtScalar>(
    grid: &GridSpec,
    harmonics: &HarmonicSpec,
) -> Option<(usize, usize, usize)> {
    supported_layout_with_benchmark_range::<T>(grid, harmonics, false)
}

#[cfg(test)]
fn supported_layout_for_benchmark<T: ShtScalar>(
    grid: &GridSpec,
    harmonics: &HarmonicSpec,
) -> Option<(usize, usize, usize)> {
    supported_layout_with_benchmark_range::<T>(grid, harmonics, true)
}

fn supported_layout_with_benchmark_range<T: ShtScalar>(
    grid: &GridSpec,
    harmonics: &HarmonicSpec,
    benchmark_range: bool,
) -> Option<(usize, usize, usize)> {
    let lmax = harmonics.lmax();
    let mmax = harmonics.mmax();
    let table_lmax = match harmonics.spin() {
        0 => lmax,
        1 => lmax.checked_add(1)?,
        _ => return None,
    };
    let maximum_table_lmax = match harmonics.spin() {
        0 if benchmark_range => 256,
        0 => MAX_PRECOMPUTED_LMAX,
        1 => MAX_PRECOMPUTED_LMAX + 1,
        _ => unreachable!(),
    };
    let minimum_table_lmax = if benchmark_range && harmonics.spin() == 0 {
        0
    } else {
        MIN_PRECOMPUTED_LMAX
    };
    let minimum_ntheta = lmax.checked_add(1)?;
    // The over-resolved measurements used at most twice the minimum GL
    // latitude count. Keep larger layouts on recurrence rather than allowing
    // the payload limit alone to admit unmeasured sampling ratios.
    let maximum_measured_ntheta = minimum_ntheta.checked_mul(2)?;
    if !T::supports_precomputed_scalar()
        || grid.kind() != GridKind::GaussLegendre
        || grid.ntheta() < minimum_ntheta
        || grid.ntheta() > maximum_measured_ntheta
        || (!benchmark_range && lmax < MIN_PRECOMPUTED_LMAX)
        || !(minimum_table_lmax..=maximum_table_lmax).contains(&table_lmax)
        || (!benchmark_range && mmax < lmax - lmax / 4)
        || mmax > (grid.nphi() - 1) / 2
    {
        return None;
    }

    let north_ring_count = grid.ntheta().div_ceil(2);
    let mut element_count = 0_usize;
    for m in 0..=mmax {
        element_count =
            element_count.checked_add((table_lmax - m + 1).checked_mul(north_ring_count)?)?;
    }
    let table_bytes = element_count.checked_mul(std::mem::size_of::<T>())?;
    (table_bytes <= MAX_TABLE_BYTES).then_some((table_lmax, north_ring_count, element_count))
}

impl<T: ShtScalar> ScalarLegendreTable<T> {
    pub(crate) fn supports_grid_and_bandlimit(grid: &GridSpec, harmonics: &HarmonicSpec) -> bool {
        supported_layout::<T>(grid, harmonics).is_some()
    }

    #[cfg(test)]
    pub(crate) fn supports_benchmark_grid_and_bandlimit(
        grid: &GridSpec,
        harmonics: &HarmonicSpec,
    ) -> bool {
        supported_layout_for_benchmark::<T>(grid, harmonics).is_some()
    }

    pub(crate) fn input_scale_limit() -> T {
        T::from_f64(TABLE_INPUT_SCALE_LIMIT)
            .expect("the f64 precomputation input limit is representable")
    }

    pub(crate) fn input_value_is_safe(value: T, limit: T) -> bool {
        value.is_finite() && value.abs() <= limit
    }

    pub(crate) fn input_values_are_safe(values: &[num_complex::Complex<T>]) -> bool {
        let limit = Self::input_scale_limit();
        values.iter().all(|value| {
            Self::input_value_is_safe(value.re, limit) && Self::input_value_is_safe(value.im, limit)
        })
    }

    pub(crate) fn real_input_values_are_safe(values: &[T]) -> bool {
        let limit = Self::input_scale_limit();
        values
            .iter()
            .all(|&value| Self::input_value_is_safe(value, limit))
    }

    pub(crate) fn build_if_beneficial(
        grid: &GridSpec,
        harmonics: &HarmonicSpec,
        sectoral_norms: &[T],
    ) -> Option<Self> {
        Self::build_with_layout(grid, harmonics, sectoral_norms, false)
    }

    #[cfg(test)]
    pub(crate) fn build_for_benchmark(
        grid: &GridSpec,
        harmonics: &HarmonicSpec,
        sectoral_norms: &[T],
    ) -> Option<Self> {
        Self::build_with_layout(grid, harmonics, sectoral_norms, true)
    }

    fn build_with_layout(
        grid: &GridSpec,
        harmonics: &HarmonicSpec,
        sectoral_norms: &[T],
        benchmark_range: bool,
    ) -> Option<Self> {
        let lmax = harmonics.lmax();
        let mmax = harmonics.mmax();
        let layout = if benchmark_range {
            #[cfg(test)]
            {
                supported_layout_for_benchmark::<T>(grid, harmonics)
            }
            #[cfg(not(test))]
            {
                None
            }
        } else {
            supported_layout::<T>(grid, harmonics)
        };
        let (table_lmax, north_ring_count, element_count) = layout?;
        let order_count = mmax.checked_add(1)?;
        let input_scale_limit = Self::input_scale_limit();

        let mut values = Vec::new();
        values.try_reserve_exact(element_count).ok()?;
        values.resize(element_count, T::zero());
        let mut order_offsets = Vec::new();
        order_offsets.try_reserve_exact(order_count + 1).ok()?;
        let mut offset = 0;
        for m in 0..=mmax {
            order_offsets.push(offset);
            offset += (table_lmax - m + 1) * north_ring_count;
        }
        order_offsets.push(offset);

        let mut paired_order_limits = Vec::new();
        paired_order_limits
            .try_reserve_exact(north_ring_count)
            .ok()?;
        for north_ring in 0..north_ring_count {
            let south_ring = grid.ntheta() - 1 - north_ring;
            let (north_sin, north_cos) = sin_cos_from_f64::<T>(grid.theta()[north_ring]);
            let (south_sin, south_cos) = sin_cos_from_f64::<T>(grid.theta()[south_ring]);
            paired_order_limits.push(
                T::polar_order_limit(lmax, harmonics.spin(), north_sin, north_cos).max(
                    T::polar_order_limit(lmax, harmonics.spin(), south_sin, south_cos),
                ),
            );
        }

        let mut epsilon = vec![T::zero(); table_lmax + 1];
        for m in 0..=mmax {
            T::fill_epsilon(m, table_lmax, &mut epsilon);
            let order_offset = order_offsets[m];
            let phase = if m.is_multiple_of(2) {
                T::one()
            } else {
                -T::one()
            };
            for north_ring in 0..north_ring_count {
                let south_ring = grid.ntheta() - 1 - north_ring;
                let (sin_theta, cos_theta) = sin_cos_from_f64::<T>(grid.theta()[north_ring]);
                if m > paired_order_limits[north_ring] {
                    // Both mirrored samples are discarded by the polar cutoff,
                    // so this basis pair need not be constructed.
                    continue;
                }
                let mut previous = T::zero();
                let mut current = sectoral_norms[m] * phase * sin_theta.powi(m as i32);
                for degree_offset in 0..=(table_lmax - m) {
                    let l = m + degree_offset;
                    let exact_equatorial_zero = south_ring == north_ring
                        && grid.theta()[north_ring] == std::f64::consts::FRAC_PI_2
                        && !(l + m).is_multiple_of(2);
                    // Spin-1 conversion includes degree `ntheta`; on GL this
                    // is the root degree of the quadrature, so its m=0 basis
                    // is exactly zero on every sampled ring.
                    let exact_gl_null_degree =
                        harmonics.spin() == 1 && m == 0 && l == grid.ntheta();
                    if sin_theta != T::zero()
                        && current == T::zero()
                        && !exact_equatorial_zero
                        && !exact_gl_null_degree
                    {
                        // A contributing recurrence lane became unrepresentable
                        // in the table. Retain the scaled implementation.
                        return None;
                    }
                    let basis_offset = order_offset + degree_offset * north_ring_count;
                    values[basis_offset + north_ring] = if exact_gl_null_degree {
                        T::zero()
                    } else {
                        current
                    };
                    if degree_offset < table_lmax - m {
                        let next = (cos_theta * current - epsilon[l] * previous) / epsilon[l + 1];
                        previous = current;
                        current = next;
                    }
                }
            }
        }

        Some(Self {
            values,
            order_offsets,
            north_ring_count,
            input_scale_limit,
        })
    }

    #[inline]
    pub(crate) fn input_is_safe(&self, value: T) -> bool {
        Self::input_value_is_safe(value, self.input_scale_limit)
    }

    pub(crate) fn north_ring_count(&self) -> usize {
        self.north_ring_count
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn allocated_bytes_for_benchmark(&self) -> usize {
        self.values.capacity() * std::mem::size_of::<T>()
            + self.order_offsets.capacity() * std::mem::size_of::<usize>()
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn benchmark_max_table_bytes() -> usize {
        MAX_TABLE_BYTES
    }

    #[inline]
    pub(crate) fn degree_lanes(&self, m: usize, degree_offset: usize) -> &[T] {
        let start = self.order_offsets[m] + degree_offset * self.north_ring_count;
        &self.values[start..start + self.north_ring_count]
    }
}

pub(crate) fn analyze_group_scalar<T: ShtScalar>(
    order_spectra: &[Complex<T>],
    table: &ScalarLegendreTable<T>,
    ntheta: usize,
    lmax: usize,
    m_start: usize,
    m_end: usize,
    alm: &mut [Complex<T>],
) {
    let mut coefficient_start = 0;
    for m in m_start..m_end {
        let coefficient_count = lmax - m + 1;
        let coefficients = &mut alm[coefficient_start..coefficient_start + coefficient_count];
        let mut degree_offset = 0;
        while degree_offset + 1 < coefficient_count {
            let even_basis = table.degree_lanes(m, degree_offset);
            let odd_basis = table.degree_lanes(m, degree_offset + 1);
            let mut even_sum = Complex::new(T::zero(), T::zero());
            let mut odd_sum = Complex::new(T::zero(), T::zero());
            for north_ring in 0..table.north_ring_count() {
                let south_ring = ntheta - 1 - north_ring;
                let north = order_spectra[m * ntheta + north_ring];
                let south = if south_ring != north_ring {
                    order_spectra[m * ntheta + south_ring]
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                even_sum += (north + south) * even_basis[north_ring];
                odd_sum += (north - south) * odd_basis[north_ring];
            }
            coefficients[degree_offset] = even_sum;
            coefficients[degree_offset + 1] = odd_sum;
            degree_offset += 2;
        }
        if degree_offset < coefficient_count {
            let l = m + degree_offset;
            let parity = if (l + m).is_multiple_of(2) {
                T::one()
            } else {
                -T::one()
            };
            let mut sum = Complex::new(T::zero(), T::zero());
            for (north_ring, &basis) in table.degree_lanes(m, degree_offset).iter().enumerate() {
                let south_ring = ntheta - 1 - north_ring;
                let south = if south_ring != north_ring {
                    order_spectra[m * ntheta + south_ring] * parity
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                sum += (order_spectra[m * ntheta + north_ring] + south) * basis;
            }
            coefficients[degree_offset] = sum;
        }
        coefficient_start += coefficient_count;
    }
}

struct F64TableAnalysisGroup<'a> {
    order_spectra: &'a [Complex<f64>],
    table: &'a ScalarLegendreTable<f64>,
    ntheta: usize,
    lmax: usize,
    m_start: usize,
    m_end: usize,
    alm: &'a mut [Complex<f64>],
    scratch_even: &'a mut [Complex<f64>],
    scratch_odd: &'a mut [Complex<f64>],
}

#[expect(
    clippy::too_many_arguments,
    reason = "The table accumulator receives packed spectra, basis storage and reusable order scratch."
)]
pub(crate) fn analyze_group_f64(
    order_spectra: &[Complex<f64>],
    table: &ScalarLegendreTable<f64>,
    ntheta: usize,
    lmax: usize,
    m_start: usize,
    m_end: usize,
    alm: &mut [Complex<f64>],
    scratch_even: &mut [Complex<f64>],
    scratch_odd: &mut [Complex<f64>],
) {
    Arch::new().dispatch(F64TableAnalysisGroup {
        order_spectra,
        table,
        ntheta,
        lmax,
        m_start,
        m_end,
        alm,
        scratch_even,
        scratch_odd,
    });
}

impl WithSimd for F64TableAnalysisGroup<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) {
        if S::C64_LANES == 2
            && self.ntheta.is_multiple_of(4)
            && self.scratch_even.len() >= self.table.north_ring_count()
            && self.scratch_odd.len() >= self.table.north_ring_count()
        {
            analyze_group_f64_two_complex_lanes_cached(self, simd);
        } else {
            analyze_group_scalar(
                self.order_spectra,
                self.table,
                self.ntheta,
                self.lmax,
                self.m_start,
                self.m_end,
                self.alm,
            );
        }
    }
}

#[inline(always)]
fn analyze_group_f64_two_complex_lanes_cached<S: Simd>(group: F64TableAnalysisGroup<'_>, simd: S) {
    debug_assert_eq!(S::C64_LANES, 2);
    debug_assert_eq!(S::F64_LANES, 4);

    let F64TableAnalysisGroup {
        order_spectra,
        table,
        ntheta,
        lmax,
        m_start,
        m_end,
        alm,
        scratch_even,
        scratch_odd,
    } = group;
    let north_ring_count = table.north_ring_count();
    let lower_complex_lane = simd.mask_between_m64s(2, 4).mask();
    let mut coefficient_start = 0;

    for m in m_start..m_end {
        let coefficient_count = lmax - m + 1;
        let coefficients = &mut alm[coefficient_start..coefficient_start + coefficient_count];
        let order_start = m * ntheta;
        let order_row = &order_spectra[order_start..order_start + ntheta];
        let paired_ring_count = ntheta / 2;

        // The north/south sum, difference and lane rotation depend only on
        // the order and latitude pair. Reuse them across every degree pair
        // instead of rebuilding them in the inner table accumulation loop.
        for north_ring in 0..north_ring_count {
            let south_ring = ntheta - 1 - north_ring;
            let north = order_row[north_ring];
            let south = if south_ring != north_ring {
                order_row[south_ring]
            } else {
                Complex::new(0.0, 0.0)
            };
            scratch_even[north_ring] = north + south;
            scratch_odd[north_ring] = north - south;
        }

        let (even_vectors, even_tail) = S::as_simd_c64s(&scratch_even[..paired_ring_count]);
        let (odd_vectors, odd_tail) = S::as_simd_c64s(&scratch_odd[..paired_ring_count]);
        debug_assert!(even_tail.is_empty());
        debug_assert!(odd_tail.is_empty());
        debug_assert_eq!(even_vectors.len(), odd_vectors.len());

        let mut degree_offset = 0;
        while degree_offset + 1 < coefficient_count {
            let even_basis = table.degree_lanes(m, degree_offset);
            let odd_basis = table.degree_lanes(m, degree_offset + 1);
            let mut even_sum = simd.splat_f64s(0.0);
            let mut odd_sum = simd.splat_f64s(0.0);
            for (vector_index, (&even_values, &odd_values)) in
                even_vectors.iter().zip(odd_vectors.iter()).enumerate()
            {
                let north_ring = vector_index * 2;
                let even_values = pulp::bytemuck::cast(even_values);
                let odd_values = pulp::bytemuck::cast(odd_values);
                let even_weights = simd.select_f64s(
                    lower_complex_lane,
                    simd.splat_f64s(even_basis[north_ring + 1]),
                    simd.splat_f64s(even_basis[north_ring]),
                );
                let odd_weights = simd.select_f64s(
                    lower_complex_lane,
                    simd.splat_f64s(odd_basis[north_ring + 1]),
                    simd.splat_f64s(odd_basis[north_ring]),
                );
                even_sum = simd.add_f64s(even_sum, simd.mul_f64s(even_values, even_weights));
                odd_sum = simd.add_f64s(odd_sum, simd.mul_f64s(odd_values, odd_weights));
            }

            let even_complex = simd.reduce_sum_c64s(pulp::bytemuck::cast(even_sum));
            let odd_complex = simd.reduce_sum_c64s(pulp::bytemuck::cast(odd_sum));
            coefficients[degree_offset] = even_complex;
            coefficients[degree_offset + 1] = odd_complex;
            degree_offset += 2;
        }

        if degree_offset < coefficient_count {
            let l = m + degree_offset;
            let parity = if (l + m).is_multiple_of(2) { 1.0 } else { -1.0 };
            let mut sum = Complex::new(0.0, 0.0);
            for (north_ring, &basis) in table.degree_lanes(m, degree_offset).iter().enumerate() {
                let south_ring = ntheta - 1 - north_ring;
                let south = if south_ring != north_ring {
                    order_row[south_ring] * parity
                } else {
                    Complex::new(0.0, 0.0)
                };
                sum += (order_row[north_ring] + south) * basis;
            }
            coefficients[degree_offset] = sum;
        }
        coefficient_start += coefficient_count;
    }
}
