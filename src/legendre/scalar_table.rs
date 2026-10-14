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

use crate::grid::{GridKind, GridSpec};
use crate::harmonics::HarmonicSpec;

use super::{ShtScalar, sin_cos_from_f64};

const MIN_PRECOMPUTED_LMAX: usize = 72;
const MAX_PRECOMPUTED_LMAX: usize = 255;
const MAX_TABLE_BYTES: usize = 34 * 1024 * 1024;
// Keep direct table products far from overflow; larger and non-finite input
// scales use the binary-scaled recurrence instead.
const TABLE_INPUT_SCALE_LIMIT: f64 = 1.0e100;

/// North-hemisphere scalar basis for the measured small-band GL specialization.
/// Values are contiguous in north latitude for each `(m, l)` pair.
pub(crate) struct ScalarLegendreTable<T> {
    values: Vec<T>,
    order_offsets: Vec<usize>,
    north_ring_count: usize,
    input_scale_limit: T,
}

impl<T: ShtScalar> ScalarLegendreTable<T> {
    pub(crate) fn build_if_beneficial(
        grid: &GridSpec,
        harmonics: &HarmonicSpec,
        sectoral_norms: &[T],
    ) -> Option<Self> {
        let lmax = harmonics.lmax();
        let mmax = harmonics.mmax();
        let table_lmax = match harmonics.spin() {
            0 => lmax,
            1 => lmax.checked_add(1)?,
            _ => return None,
        };
        let maximum_table_lmax = match harmonics.spin() {
            0 => MAX_PRECOMPUTED_LMAX,
            1 => 128,
            _ => unreachable!(),
        };
        if !T::supports_precomputed_scalar()
            || grid.kind() != GridKind::GaussLegendre
            || grid.ntheta() != lmax + 1
            || lmax < MIN_PRECOMPUTED_LMAX
            || !(MIN_PRECOMPUTED_LMAX..=maximum_table_lmax).contains(&table_lmax)
            || mmax < lmax - lmax / 4
            || mmax > (grid.nphi() - 1) / 2
        {
            return None;
        }

        let north_ring_count = grid.ntheta().div_ceil(2);
        let order_count = mmax.checked_add(1)?;
        let mut element_count = 0_usize;
        for m in 0..=mmax {
            element_count =
                element_count.checked_add((table_lmax - m + 1).checked_mul(north_ring_count)?)?;
        }
        let table_bytes = element_count.checked_mul(std::mem::size_of::<T>())?;
        if table_bytes > MAX_TABLE_BYTES {
            return None;
        }
        let input_scale_limit = T::from_f64(TABLE_INPUT_SCALE_LIMIT)
            .expect("the f64 precomputation input limit is representable");

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
                    if sin_theta != T::zero() && current == T::zero() && !exact_equatorial_zero {
                        // A contributing recurrence lane became unrepresentable
                        // in the table. Retain the scaled implementation.
                        return None;
                    }
                    let basis_offset = order_offset + degree_offset * north_ring_count;
                    values[basis_offset + north_ring] = current;
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
        value.is_finite() && value.abs() <= self.input_scale_limit
    }

    pub(crate) fn north_ring_count(&self) -> usize {
        self.north_ring_count
    }

    #[inline]
    pub(crate) fn degree_lanes(&self, m: usize, degree_offset: usize) -> &[T] {
        let start = self.order_offsets[m] + degree_offset * self.north_ring_count;
        &self.values[start..start + self.north_ring_count]
    }
}
