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
use crate::legendre::numeric;
use num_complex::Complex32;

/// Precision leaves for the ordinary-range reflected scalar-analysis block.
/// The order/ring traversal and recurrence are shared; only the lane type,
/// FMA, reduction, and range predicate vary with the native scalar width.
pub(crate) trait FusedAnalysisPrecision: PairedScalarRecurrencePrecision {
    type Vector<S: Simd>: Copy;
    type ScaledState;
    const USE_EXISTING_SCALED_BLOCK: bool = false;

    fn as_simd<S: Simd>(values: &[Self]) -> (&[Self::Vector<S>], &[Self]);
    fn as_mut_simd<S: Simd>(values: &mut [Self]) -> (&mut [Self::Vector<S>], &mut [Self]);
    fn splat<S: Simd>(simd: S, value: Self) -> Self::Vector<S>;
    fn simd_fma<S: Simd>(
        simd: S,
        left: Self::Vector<S>,
        right: Self::Vector<S>,
        addend: Self::Vector<S>,
    ) -> Self::Vector<S>;
    fn reduce_sum<S: Simd>(simd: S, value: Self::Vector<S>) -> Self;
    fn simd_mul<S: Simd>(simd: S, left: Self::Vector<S>, right: Self::Vector<S>)
    -> Self::Vector<S>;
    fn vector_pair_needs_scaling<S: Simd>(
        simd: S,
        previous: Self::Vector<S>,
        current: Self::Vector<S>,
    ) -> bool;
    fn scalar_in_fast_range(value: Self) -> bool;
    fn materialize_scaled_seed(value: Self, exponent: i64) -> Option<Self>;
    fn prepare_scaled_state(
        same_real: &[Self],
        same_imag: &[Self],
        opposite_real: &[Self],
        opposite_imag: &[Self],
    ) -> Self::ScaledState;

    #[expect(
        clippy::too_many_arguments,
        reason = "The scaled accumulator consumes both reflected complex components, their lane exponents, and the native cosine weights."
    )]
    fn accumulate_scaled<S: Simd, const OPPOSITE_PREWEIGHTED: bool>(
        simd: S,
        same_real: &[Self],
        same_imag: &[Self],
        opposite_real: &[Self],
        opposite_imag: &[Self],
        basis: &[Self],
        exponents: &[i64],
        cos_theta: &[Self],
        state: &mut Self::ScaledState,
    ) -> (Complex<Self>, Complex<Self>);

    fn invalidate_scaled_state(state: &mut Self::ScaledState);
}

pub(crate) struct F32ScaledAnalysisState {
    maximum_coefficient_exponent: i32,
    common_exponent: i64,
    shift: i64,
    factors: [f32; SCALAR_ANALYSIS_PAIR_TILE_SIZE],
    hard_factors: [f32; SCALAR_ANALYSIS_PAIR_TILE_SIZE],
    hard_scale_exponents: [i64; SCALAR_ANALYSIS_PAIR_TILE_SIZE],
    hard_scalar_lanes: [bool; SCALAR_ANALYSIS_PAIR_TILE_SIZE],
    hard_vector_indices: [u8; SCALAR_ANALYSIS_PAIR_TILE_SIZE],
    hard_vector_count: usize,
    lane_count: usize,
    valid: bool,
}

impl FusedAnalysisPrecision for f32 {
    type Vector<S: Simd> = S::f32s;
    type ScaledState = F32ScaledAnalysisState;

    fn prepare_scaled_state(
        same_real: &[Self],
        same_imag: &[Self],
        opposite_real: &[Self],
        opposite_imag: &[Self],
    ) -> Self::ScaledState {
        let maximum_coefficient = same_real
            .iter()
            .chain(same_imag)
            .chain(opposite_real)
            .chain(opposite_imag)
            .fold(0.0_f32, |maximum, value| maximum.max(value.abs()));
        let raw_exponent = (maximum_coefficient.to_bits() >> 23) & 0xff;
        let maximum_coefficient_exponent = if maximum_coefficient == 0.0 {
            -149
        } else if raw_exponent == 0 {
            -126
        } else {
            raw_exponent as i32 - 126
        };
        F32ScaledAnalysisState {
            maximum_coefficient_exponent,
            common_exponent: 0,
            shift: 0,
            factors: [0.0; SCALAR_ANALYSIS_PAIR_TILE_SIZE],
            hard_factors: [0.0; SCALAR_ANALYSIS_PAIR_TILE_SIZE],
            hard_scale_exponents: [0; SCALAR_ANALYSIS_PAIR_TILE_SIZE],
            hard_scalar_lanes: [false; SCALAR_ANALYSIS_PAIR_TILE_SIZE],
            hard_vector_indices: [0; SCALAR_ANALYSIS_PAIR_TILE_SIZE],
            hard_vector_count: 0,
            lane_count: 0,
            valid: false,
        }
    }

    #[inline(always)]
    fn as_simd<S: Simd>(values: &[Self]) -> (&[Self::Vector<S>], &[Self]) {
        S::as_simd_f32s(values)
    }

    #[inline(always)]
    fn as_mut_simd<S: Simd>(values: &mut [Self]) -> (&mut [Self::Vector<S>], &mut [Self]) {
        S::as_mut_simd_f32s(values)
    }

    #[inline(always)]
    fn splat<S: Simd>(simd: S, value: Self) -> Self::Vector<S> {
        simd.splat_f32s(value)
    }

    #[inline(always)]
    fn simd_fma<S: Simd>(
        simd: S,
        left: Self::Vector<S>,
        right: Self::Vector<S>,
        addend: Self::Vector<S>,
    ) -> Self::Vector<S> {
        simd.mul_add_f32s(left, right, addend)
    }

    #[inline(always)]
    fn reduce_sum<S: Simd>(simd: S, value: Self::Vector<S>) -> Self {
        simd.reduce_sum_f32s(value)
    }

    #[inline(always)]
    fn simd_mul<S: Simd>(
        simd: S,
        left: Self::Vector<S>,
        right: Self::Vector<S>,
    ) -> Self::Vector<S> {
        simd.mul_f32s(left, right)
    }

    #[inline(always)]
    fn vector_pair_needs_scaling<S: Simd>(
        simd: S,
        previous: Self::Vector<S>,
        current: Self::Vector<S>,
    ) -> bool {
        let zero = simd.splat_f32s(0.0);
        let small = simd.splat_f32s(Self::SMALL);
        let big = simd.splat_f32s(Self::BIG);
        let valid = |value| {
            let magnitude = simd.abs_f32s(value);
            simd.or_m32s(
                simd.equal_f32s(magnitude, zero),
                simd.and_m32s(
                    simd.greater_than_or_equal_f32s(magnitude, small),
                    simd.less_than_or_equal_f32s(magnitude, big),
                ),
            )
        };
        let invalid = simd.or_m32s(
            simd.not_m32s(valid(previous)),
            simd.not_m32s(valid(current)),
        );
        simd.first_true_m32s(invalid) != S::F32_LANES
    }

    #[inline(always)]
    fn scalar_in_fast_range(value: Self) -> bool {
        value == 0.0 || (value.is_finite() && (Self::SMALL..=Self::BIG).contains(&value.abs()))
    }

    #[inline(always)]
    fn materialize_scaled_seed(value: Self, exponent: i64) -> Option<Self> {
        if value == 0.0 {
            return Some(value);
        }
        if !value.is_normal() || !(-4096..=4096).contains(&exponent) {
            return None;
        }
        let bits = value.to_bits();
        let raw_exponent = ((bits >> 23) & 0xff) as i64;
        let scaled_exponent = raw_exponent + exponent;
        if !(1..0xff).contains(&scaled_exponent) {
            return None;
        }
        let exponent_mask = 0xff_u32 << 23;
        Some(Self::from_bits(
            (bits & !exponent_mask) | ((scaled_exponent as u32) << 23),
        ))
    }

    #[inline(always)]
    fn accumulate_scaled<S: Simd, const OPPOSITE_PREWEIGHTED: bool>(
        simd: S,
        same_real: &[Self],
        same_imag: &[Self],
        opposite_real: &[Self],
        opposite_imag: &[Self],
        basis: &[Self],
        exponents: &[i64],
        cos_theta: &[Self],
        state: &mut Self::ScaledState,
    ) -> (Complex<Self>, Complex<Self>) {
        let F32ScaledAnalysisState {
            maximum_coefficient_exponent,
            common_exponent,
            shift,
            factors,
            hard_factors,
            hard_scale_exponents,
            hard_scalar_lanes,
            hard_vector_indices,
            hard_vector_count,
            lane_count,
            valid,
        } = state;
        if !*valid || *lane_count != basis.len() {
            *common_exponent = exponents.iter().copied().max().unwrap_or(0);
            let lane_bits = if basis.len() <= 1 {
                0
            } else {
                (usize::BITS - (basis.len() - 1).leading_zeros()) as i32
            };
            let shift_budget = 126 - *maximum_coefficient_exponent - 40 - lane_bits;
            *shift = i64::from(shift_budget.clamp(0, 78));
            for lane in 0..basis.len() {
                let factor_exponent = exponents[lane]
                    .saturating_sub(*common_exponent)
                    .saturating_add(*shift);
                factors[lane] = if factor_exponent < -86 {
                    0.0
                } else {
                    numeric::scale_binary_f32(1.0, factor_exponent)
                };
            }
            hard_factors.fill(0.0);
            hard_scalar_lanes.fill(false);
            *hard_vector_count = 0;
            let vector_width = S::F32_LANES;
            let vector_count = basis.len() / vector_width;
            for (vector_index, hard_scale_exponent) in hard_scale_exponents
                .iter_mut()
                .take(vector_count)
                .enumerate()
            {
                let lane_start = vector_index * vector_width;
                let lane_end = lane_start + vector_width;
                let mut hard_common = None::<i64>;
                for lane in lane_start..lane_end {
                    if factors[lane] == 0.0 {
                        hard_common = Some(
                            hard_common
                                .map_or(exponents[lane], |maximum| maximum.max(exponents[lane])),
                        );
                    }
                }
                let Some(hard_common) = hard_common else {
                    continue;
                };
                *hard_scale_exponent = hard_common.saturating_sub(*shift);
                let mut grouped_lanes = false;
                for lane in lane_start..lane_end {
                    if factors[lane] != 0.0 {
                        continue;
                    }
                    let factor_exponent = exponents[lane]
                        .saturating_sub(hard_common)
                        .saturating_add(*shift);
                    let cosine_is_safe = OPPOSITE_PREWEIGHTED
                        || cos_theta[lane] == 0.0
                        || cos_theta[lane].abs() >= f32::from_bits(87_u32 << 23);
                    let factor_floor = if OPPOSITE_PREWEIGHTED || cos_theta[lane] == 0.0 {
                        -45
                    } else {
                        -5
                    };
                    if factor_exponent < factor_floor || !cosine_is_safe {
                        hard_scalar_lanes[lane] = true;
                    } else {
                        hard_factors[lane] = numeric::scale_binary_f32(1.0, factor_exponent);
                        grouped_lanes = true;
                    }
                }
                if grouped_lanes {
                    debug_assert!(vector_index <= u8::MAX as usize);
                    hard_vector_indices[*hard_vector_count] = vector_index as u8;
                    *hard_vector_count += 1;
                }
            }
            for lane in (vector_count * vector_width)..basis.len() {
                hard_scalar_lanes[lane] = factors[lane] == 0.0;
            }
            *lane_count = basis.len();
            *valid = true;
        }

        let mut hard_same = Complex32::new(0.0, 0.0);
        let mut hard_opposite = Complex32::new(0.0, 0.0);
        for lane in 0..basis.len() {
            if hard_scalar_lanes[lane] {
                let same = scalar::multiply_complex_scaled_f32(
                    Complex32::new(same_real[lane], same_imag[lane]),
                    basis[lane],
                    exponents[lane],
                );
                let opposite_basis = if OPPOSITE_PREWEIGHTED {
                    basis[lane]
                } else {
                    basis[lane] * cos_theta[lane]
                };
                let opposite = scalar::multiply_complex_scaled_f32(
                    Complex32::new(opposite_real[lane], opposite_imag[lane]),
                    opposite_basis,
                    exponents[lane],
                );
                hard_same.re += same.re;
                hard_same.im += same.im;
                hard_opposite.re += opposite.re;
                hard_opposite.im += opposite.im;
            }
        }

        let (same_real_vectors, same_real_tail) = S::as_simd_f32s(same_real);
        let (same_imag_vectors, same_imag_tail) = S::as_simd_f32s(same_imag);
        let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f32s(opposite_real);
        let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f32s(opposite_imag);
        let (basis_vectors, basis_tail) = S::as_simd_f32s(basis);
        let (cosine_vectors, cosine_tail) = S::as_simd_f32s(cos_theta);
        let (factor_vectors, factor_tail) = S::as_simd_f32s(&factors[..basis.len()]);
        let (hard_factor_vectors, _) = S::as_simd_f32s(&hard_factors[..basis.len()]);
        debug_assert_eq!(same_real_vectors.len(), basis_vectors.len());
        debug_assert_eq!(same_imag_vectors.len(), basis_vectors.len());
        debug_assert_eq!(opposite_real_vectors.len(), basis_vectors.len());
        debug_assert_eq!(opposite_imag_vectors.len(), basis_vectors.len());
        debug_assert_eq!(cosine_vectors.len(), basis_vectors.len());
        debug_assert_eq!(factor_vectors.len(), basis_vectors.len());
        debug_assert_eq!(hard_factor_vectors.len(), basis_vectors.len());
        let zero = simd.splat_f32s(0.0);
        let mut same_re = zero;
        let mut same_im = zero;
        let mut opposite_re = zero;
        let mut opposite_im = zero;
        for index in 0..basis_vectors.len() {
            let aligned_basis = simd.mul_f32s(basis_vectors[index], factor_vectors[index]);
            same_re = simd.mul_add_f32s(same_real_vectors[index], aligned_basis, same_re);
            same_im = simd.mul_add_f32s(same_imag_vectors[index], aligned_basis, same_im);
            let opposite_basis = if OPPOSITE_PREWEIGHTED {
                basis_vectors[index]
            } else {
                simd.mul_f32s(basis_vectors[index], cosine_vectors[index])
            };
            let aligned_opposite_basis = simd.mul_f32s(opposite_basis, factor_vectors[index]);
            opposite_re = simd.mul_add_f32s(
                opposite_real_vectors[index],
                aligned_opposite_basis,
                opposite_re,
            );
            opposite_im = simd.mul_add_f32s(
                opposite_imag_vectors[index],
                aligned_opposite_basis,
                opposite_im,
            );
        }
        let mut same_re_sum = simd.reduce_sum_f32s(same_re);
        let mut same_im_sum = simd.reduce_sum_f32s(same_im);
        let mut opposite_re_sum = simd.reduce_sum_f32s(opposite_re);
        let mut opposite_im_sum = simd.reduce_sum_f32s(opposite_im);
        for lane in 0..basis_tail.len() {
            let aligned_basis = basis_tail[lane] * factor_tail[lane];
            same_re_sum = same_real_tail[lane].mul_add(aligned_basis, same_re_sum);
            same_im_sum = same_imag_tail[lane].mul_add(aligned_basis, same_im_sum);
            let opposite_basis = if OPPOSITE_PREWEIGHTED {
                basis_tail[lane]
            } else {
                basis_tail[lane] * cosine_tail[lane]
            };
            let aligned_opposite_basis = opposite_basis * factor_tail[lane];
            opposite_re_sum =
                opposite_real_tail[lane].mul_add(aligned_opposite_basis, opposite_re_sum);
            opposite_im_sum =
                opposite_imag_tail[lane].mul_add(aligned_opposite_basis, opposite_im_sum);
        }
        for &vector_index in hard_vector_indices.iter().take(*hard_vector_count) {
            let vector_index = vector_index as usize;
            let aligned_basis = simd.mul_f32s(
                basis_vectors[vector_index],
                hard_factor_vectors[vector_index],
            );
            let hard_same_re =
                simd.mul_add_f32s(same_real_vectors[vector_index], aligned_basis, zero);
            let hard_same_im =
                simd.mul_add_f32s(same_imag_vectors[vector_index], aligned_basis, zero);
            let aligned_opposite_basis = if OPPOSITE_PREWEIGHTED {
                aligned_basis
            } else {
                simd.mul_f32s(
                    simd.mul_f32s(basis_vectors[vector_index], cosine_vectors[vector_index]),
                    hard_factor_vectors[vector_index],
                )
            };
            let hard_opposite_re = simd.mul_add_f32s(
                opposite_real_vectors[vector_index],
                aligned_opposite_basis,
                zero,
            );
            let hard_opposite_im = simd.mul_add_f32s(
                opposite_imag_vectors[vector_index],
                aligned_opposite_basis,
                zero,
            );
            let scale_exponent = hard_scale_exponents[vector_index];
            hard_same.re +=
                numeric::scale_binary_f32(simd.reduce_sum_f32s(hard_same_re), scale_exponent);
            hard_same.im +=
                numeric::scale_binary_f32(simd.reduce_sum_f32s(hard_same_im), scale_exponent);
            hard_opposite.re +=
                numeric::scale_binary_f32(simd.reduce_sum_f32s(hard_opposite_re), scale_exponent);
            hard_opposite.im +=
                numeric::scale_binary_f32(simd.reduce_sum_f32s(hard_opposite_im), scale_exponent);
        }
        (
            Complex32::new(
                numeric::scale_binary_f32(same_re_sum, common_exponent.saturating_sub(*shift))
                    + hard_same.re,
                numeric::scale_binary_f32(same_im_sum, common_exponent.saturating_sub(*shift))
                    + hard_same.im,
            ),
            Complex32::new(
                numeric::scale_binary_f32(opposite_re_sum, common_exponent.saturating_sub(*shift))
                    + hard_opposite.re,
                numeric::scale_binary_f32(opposite_im_sum, common_exponent.saturating_sub(*shift))
                    + hard_opposite.im,
            ),
        )
    }

    #[inline(always)]
    fn invalidate_scaled_state(state: &mut Self::ScaledState) {
        state.valid = false;
    }
}

impl FusedAnalysisPrecision for f64 {
    type Vector<S: Simd> = S::f64s;
    type ScaledState = ();
    const USE_EXISTING_SCALED_BLOCK: bool = true;

    #[inline(always)]
    fn prepare_scaled_state(
        _same_real: &[Self],
        _same_imag: &[Self],
        _opposite_real: &[Self],
        _opposite_imag: &[Self],
    ) -> Self::ScaledState {
    }

    #[inline(always)]
    fn as_simd<S: Simd>(values: &[Self]) -> (&[Self::Vector<S>], &[Self]) {
        S::as_simd_f64s(values)
    }

    #[inline(always)]
    fn as_mut_simd<S: Simd>(values: &mut [Self]) -> (&mut [Self::Vector<S>], &mut [Self]) {
        S::as_mut_simd_f64s(values)
    }

    #[inline(always)]
    fn splat<S: Simd>(simd: S, value: Self) -> Self::Vector<S> {
        simd.splat_f64s(value)
    }

    #[inline(always)]
    fn simd_fma<S: Simd>(
        simd: S,
        left: Self::Vector<S>,
        right: Self::Vector<S>,
        addend: Self::Vector<S>,
    ) -> Self::Vector<S> {
        simd.mul_add_f64s(left, right, addend)
    }

    #[inline(always)]
    fn reduce_sum<S: Simd>(simd: S, value: Self::Vector<S>) -> Self {
        simd.reduce_sum_f64s(value)
    }

    #[inline(always)]
    fn simd_mul<S: Simd>(
        simd: S,
        left: Self::Vector<S>,
        right: Self::Vector<S>,
    ) -> Self::Vector<S> {
        simd.mul_f64s(left, right)
    }

    #[inline(always)]
    fn vector_pair_needs_scaling<S: Simd>(
        simd: S,
        previous: Self::Vector<S>,
        current: Self::Vector<S>,
    ) -> bool {
        let zero = simd.splat_f64s(0.0);
        let small = simd.splat_f64s(Self::SMALL);
        let big = simd.splat_f64s(Self::BIG);
        let valid = |value| {
            let magnitude = simd.abs_f64s(value);
            simd.or_m64s(
                simd.equal_f64s(magnitude, zero),
                simd.and_m64s(
                    simd.greater_than_or_equal_f64s(magnitude, small),
                    simd.less_than_or_equal_f64s(magnitude, big),
                ),
            )
        };
        let invalid = simd.or_m64s(
            simd.not_m64s(valid(previous)),
            simd.not_m64s(valid(current)),
        );
        simd.first_true_m64s(invalid) != S::F64_LANES
    }

    #[inline(always)]
    fn scalar_in_fast_range(value: Self) -> bool {
        value == 0.0 || (value.is_finite() && (Self::SMALL..=Self::BIG).contains(&value.abs()))
    }

    #[inline(always)]
    fn materialize_scaled_seed(value: Self, exponent: i64) -> Option<Self> {
        super::scale_normal_prefix_product(value, exponent)
    }

    #[inline(always)]
    fn accumulate_scaled<S: Simd, const OPPOSITE_PREWEIGHTED: bool>(
        simd: S,
        same_real: &[Self],
        same_imag: &[Self],
        opposite_real: &[Self],
        opposite_imag: &[Self],
        basis: &[Self],
        exponents: &[i64],
        cos_theta: &[Self],
        _state: &mut Self::ScaledState,
    ) -> (Complex<Self>, Complex<Self>) {
        if OPPOSITE_PREWEIGHTED {
            MixedAnalysisPairAccumulation::<true> {
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                basis,
                exponents,
                cos_theta,
                allow_insignificant_scaled_lanes: false,
            }
            .with_simd(simd)
        } else {
            MixedAnalysisPairAccumulation::<false> {
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                basis,
                exponents,
                cos_theta,
                allow_insignificant_scaled_lanes: false,
            }
            .with_simd(simd)
        }
    }

    #[inline(always)]
    fn invalidate_scaled_state(_state: &mut Self::ScaledState) {}
}

/// Shared native-f32/f64 scalar-analysis block. It consumes the preweighted
/// opposite spectrum, carries lane scale exponents when needed, accumulates
/// one even/odd degree pair, then advances the paired recurrence with FMA.
#[expect(
    clippy::too_many_arguments,
    reason = "The typed fused block receives reflected spectra, recurrence geometry, and worker scratch explicitly."
)]
pub(crate) fn fused_scalar_analysis_pair_block_preweighted_typed<T: FusedAnalysisPrecision>(
    arch: Arch,
    m: usize,
    lmax: usize,
    range_check_is_proven_safe: bool,
    range_check_was_exercised: &mut bool,
    sin_theta: &[T],
    cos_theta: &[T],
    cos_theta_squared: &[T],
    sectoral_norm: T,
    coefficient_a: &[T],
    coefficient_b: &[T],
    same_real: &[T],
    same_imag: &[T],
    opposite_real: &[T],
    opposite_imag: &[T],
    staged_coefficients: &mut [Complex<T>],
    previous: &mut [T],
    current: &mut [T],
    next: &mut [T],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    *range_check_was_exercised = false;
    let lane_count = sin_theta.len();
    if lane_count == 0
        || lane_count > SCALAR_ANALYSIS_PAIR_TILE_SIZE
        || cos_theta.len() != lane_count
        || cos_theta_squared.len() != lane_count
        || same_real.len() != lane_count
        || same_imag.len() != lane_count
        || opposite_real.len() != lane_count
        || opposite_imag.len() != lane_count
        || previous.len() < lane_count
        || current.len() < lane_count
        || next.len() < lane_count
        || exponents.len() < lane_count
    {
        return Ok(false);
    }

    let coefficient_count = lmax - m + 1;
    if staged_coefficients.len() < coefficient_count {
        return Ok(false);
    }
    let pair_count = coefficient_count.div_ceil(2);
    if coefficient_a.len() < pair_count || coefficient_b.len() < pair_count {
        return Ok(false);
    }
    #[cfg(all(test, feature = "parallel"))]
    super::record_typed_scalar_analysis_block(true, false, false, false);
    T::initialize_pair_state(
        m,
        sin_theta,
        sectoral_norm,
        &mut previous[..lane_count],
        &mut current[..lane_count],
        &mut exponents[..lane_count],
    )?;
    if T::USE_EXISTING_SCALED_BLOCK
        && exponents[..lane_count]
            .iter()
            .any(|&exponent| exponent != 0)
    {
        #[cfg(all(test, feature = "parallel"))]
        super::record_typed_scalar_analysis_block(false, true, false, false);
        return Ok(false);
    }
    *range_check_was_exercised = !range_check_is_proven_safe && pair_count > 1;
    let succeeded = arch.dispatch(FusedScalarAnalysisPairTyped {
        m,
        lmax,
        range_check_is_proven_safe,
        range_check_was_exercised,
        same_real,
        same_imag,
        opposite_real,
        opposite_imag,
        cos_theta,
        cos_theta_squared,
        coefficient_a,
        coefficient_b,
        staged_coefficients: &mut staged_coefficients[..coefficient_count],
        previous: &mut previous[..lane_count],
        current: &mut current[..lane_count],
        next: &mut next[..lane_count],
        exponents: &mut exponents[..lane_count],
    })?;
    #[cfg(all(test, feature = "parallel"))]
    super::record_typed_scalar_analysis_block(false, false, !succeeded, succeeded);
    Ok(succeeded)
}

struct FusedScalarAnalysisPairTyped<'a, T: FusedAnalysisPrecision> {
    m: usize,
    lmax: usize,
    range_check_is_proven_safe: bool,
    range_check_was_exercised: &'a mut bool,
    same_real: &'a [T],
    same_imag: &'a [T],
    opposite_real: &'a [T],
    opposite_imag: &'a [T],
    cos_theta: &'a [T],
    cos_theta_squared: &'a [T],
    coefficient_a: &'a [T],
    coefficient_b: &'a [T],
    staged_coefficients: &'a mut [Complex<T>],
    previous: &'a mut [T],
    current: &'a mut [T],
    next: &'a mut [T],
    exponents: &'a mut [i64],
}

impl<T: FusedAnalysisPrecision> WithSimd for FusedScalarAnalysisPairTyped<'_, T> {
    type Output = Result<bool, ShtError>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedScalarAnalysisPairTyped {
            m,
            lmax,
            range_check_is_proven_safe,
            range_check_was_exercised,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            staged_coefficients,
            previous,
            current,
            next,
            exponents,
        } = self;
        let mut previous_slice: &mut [T] = previous;
        let mut current_slice: &mut [T] = current;
        let mut next_slice: &mut [T] = next;
        let exponents_slice: &mut [i64] = exponents;
        let mut normal_range =
            T::normalize_pair_state_simd(simd, previous_slice, current_slice, exponents_slice)?;
        let mut scaled_state =
            T::prepare_scaled_state(same_real, same_imag, opposite_real, opposite_imag);
        let (same_re_vectors, same_re_tail) = T::as_simd::<S>(same_real);
        let (same_im_vectors, same_im_tail) = T::as_simd::<S>(same_imag);
        let (opposite_re_vectors, opposite_re_tail) = T::as_simd::<S>(opposite_real);
        let (opposite_im_vectors, opposite_im_tail) = T::as_simd::<S>(opposite_imag);
        let (cosine_squared_vectors, cosine_squared_tail) = T::as_simd::<S>(cos_theta_squared);
        if same_re_vectors.len() != same_im_vectors.len()
            || same_re_vectors.len() != opposite_re_vectors.len()
            || same_re_vectors.len() != opposite_im_vectors.len()
            || same_re_vectors.len() != cosine_squared_vectors.len()
            || same_re_tail.len() != same_im_tail.len()
            || same_re_tail.len() != opposite_re_tail.len()
            || same_re_tail.len() != opposite_im_tail.len()
            || same_re_tail.len() != cosine_squared_tail.len()
        {
            return Ok(false);
        }

        let zero = T::splat(simd, T::zero());
        let coefficient_count = lmax - m + 1;
        let pair_count = coefficient_count.div_ceil(2);

        for pair_index in 0..pair_count {
            #[cfg(all(test, feature = "parallel"))]
            super::record_typed_scalar_analysis_pair_accumulation(normal_range);
            let (same_re_sum, same_im_sum, opposite_re_sum, opposite_im_sum) = if normal_range {
                let (basis_vectors, basis_tail) = T::as_simd::<S>(current_slice);
                let mut same_re = zero;
                let mut same_im = zero;
                let mut opposite_re = zero;
                let mut opposite_im = zero;
                for index in 0..basis_vectors.len() {
                    same_re =
                        T::simd_fma(simd, same_re_vectors[index], basis_vectors[index], same_re);
                    same_im =
                        T::simd_fma(simd, same_im_vectors[index], basis_vectors[index], same_im);
                    opposite_re = T::simd_fma(
                        simd,
                        opposite_re_vectors[index],
                        basis_vectors[index],
                        opposite_re,
                    );
                    opposite_im = T::simd_fma(
                        simd,
                        opposite_im_vectors[index],
                        basis_vectors[index],
                        opposite_im,
                    );
                }
                let mut same_re_sum = T::reduce_sum(simd, same_re);
                let mut same_im_sum = T::reduce_sum(simd, same_im);
                let mut opposite_re_sum = T::reduce_sum(simd, opposite_re);
                let mut opposite_im_sum = T::reduce_sum(simd, opposite_im);
                for lane in 0..basis_tail.len() {
                    same_re_sum = same_re_tail[lane].mul_add(basis_tail[lane], same_re_sum);
                    same_im_sum = same_im_tail[lane].mul_add(basis_tail[lane], same_im_sum);
                    opposite_re_sum =
                        opposite_re_tail[lane].mul_add(basis_tail[lane], opposite_re_sum);
                    opposite_im_sum =
                        opposite_im_tail[lane].mul_add(basis_tail[lane], opposite_im_sum);
                }
                (same_re_sum, same_im_sum, opposite_re_sum, opposite_im_sum)
            } else {
                let (same, opposite) = T::accumulate_scaled::<S, true>(
                    simd,
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    current_slice,
                    exponents_slice,
                    cos_theta,
                    &mut scaled_state,
                );
                (same.re, same.im, opposite.re, opposite.im)
            };
            let offset = pair_index * 2;
            staged_coefficients[offset] = Complex::new(same_re_sum, same_im_sum);
            if offset + 1 < coefficient_count {
                staged_coefficients[offset + 1] = Complex::new(opposite_re_sum, opposite_im_sum);
            }

            if pair_index + 1 == pair_count {
                break;
            }

            let a = T::splat(simd, coefficient_a[pair_index]);
            let b = T::splat(simd, coefficient_b[pair_index]);
            let (previous_vectors, previous_tail) = T::as_simd::<S>(previous_slice);
            let (current_vectors, current_tail) = T::as_simd::<S>(current_slice);
            let (cos2_vectors, cos2_tail) = T::as_simd::<S>(cos_theta_squared);
            let (next_vectors, next_tail) = T::as_mut_simd::<S>(next_slice);
            if previous_vectors.len() != current_vectors.len()
                || previous_vectors.len() != cos2_vectors.len()
                || previous_vectors.len() != next_vectors.len()
                || previous_tail.len() != current_tail.len()
                || previous_tail.len() != cos2_tail.len()
                || previous_tail.len() != next_tail.len()
            {
                return Ok(false);
            }
            for index in 0..next_vectors.len() {
                let leading = T::simd_fma(simd, a, cos2_vectors[index], b);
                next_vectors[index] = T::simd_fma(
                    simd,
                    leading,
                    current_vectors[index],
                    previous_vectors[index],
                );
            }
            for lane in 0..next_tail.len() {
                let leading =
                    coefficient_a[pair_index].mul_add(cos2_tail[lane], coefficient_b[pair_index]);
                next_tail[lane] = leading.mul_add(current_tail[lane], previous_tail[lane]);
            }
            std::mem::swap(&mut previous_slice, &mut current_slice);
            std::mem::swap(&mut current_slice, &mut next_slice);
            let check_range = !range_check_is_proven_safe || !normal_range;
            if check_range {
                *range_check_was_exercised = true;
                if T::pair_state_needs_scaling(simd, previous_slice, current_slice) {
                    if previous_slice
                        .iter()
                        .chain(current_slice.iter())
                        .any(|value| !value.is_finite())
                    {
                        return Ok(false);
                    }
                    normal_range = T::normalize_pair_state_simd(
                        simd,
                        previous_slice,
                        current_slice,
                        exponents_slice,
                    )?;
                    T::invalidate_scaled_state(&mut scaled_state);
                }
            }
        }
        Ok(true)
    }
}

/// Accumulates a paired scalar analysis block across its latitude lanes.
///
/// Normal recurrence lanes use vector products and horizontal reductions.
/// Lanes carrying significant binary scale exponents retain the checked scalar
/// path; the safeguarded GL analysis route omits negative epochs under its
/// latitude-wide error budget.
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
            return arch.dispatch(AnalysisPairAccumulation::<false> {
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                basis,
                cos_theta,
            });
        }
        if exponents.contains(&0) {
            return arch.dispatch(MixedAnalysisPairAccumulation::<false> {
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                basis,
                exponents,
                cos_theta,
                allow_insignificant_scaled_lanes: false,
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

/// Accumulates two scalar-conversion channels in one SIMD pass when their
/// recurrence and products are in the normal range.
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

    if products_safe == [true, true] && !normal_range && exponents.contains(&0) {
        return arch.dispatch(MixedAnalysisPairChannelsAccumulation::<false> {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis,
            exponents,
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

/// Checks whether an analysis block can safely use ordinary SIMD accumulation.
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
    // vector reduction and accumulation across previous latitude tiles finite.
    let max_input = f64::MAX / scalar::BIG / (2.0 * same_real.len().max(1) as f64);
    Arch::new().dispatch(AnalysisProductsSafe {
        same_real,
        same_imag,
        opposite_real,
        opposite_imag,
        cos_theta,
        max_input,
    })
}

struct AnalysisProductsSafe<'a> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    cos_theta: &'a [f64],
    max_input: f64,
}

impl WithSimd for AnalysisProductsSafe<'_> {
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let (same_real_vectors, same_real_tail) = S::as_simd_f64s(self.same_real);
        let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(self.same_imag);
        let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(self.opposite_real);
        let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(self.opposite_imag);
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(self.cos_theta);
        debug_assert_eq!(same_real_vectors.len(), same_imag_vectors.len());
        debug_assert_eq!(same_real_vectors.len(), opposite_real_vectors.len());
        debug_assert_eq!(same_real_vectors.len(), opposite_imag_vectors.len());
        debug_assert_eq!(same_real_vectors.len(), cosine_vectors.len());
        debug_assert_eq!(same_real_tail.len(), same_imag_tail.len());
        debug_assert_eq!(same_real_tail.len(), opposite_real_tail.len());
        debug_assert_eq!(same_real_tail.len(), opposite_imag_tail.len());
        debug_assert_eq!(same_real_tail.len(), cosine_tail.len());

        let zeros = simd.splat_f64s(0.0);
        let minimum_normal = simd.splat_f64s(f64::MIN_POSITIVE);
        let maximum_input = simd.splat_f64s(self.max_input);
        let one = simd.splat_f64s(1.0);
        for index in 0..same_real_vectors.len() {
            let mut normal_inputs = simd.equal_f64s(zeros, zeros);
            for value in [
                same_real_vectors[index],
                same_imag_vectors[index],
                opposite_real_vectors[index],
                opposite_imag_vectors[index],
            ] {
                let magnitude = simd.abs_f64s(value);
                let normal_or_zero = simd.or_m64s(
                    simd.equal_f64s(magnitude, zeros),
                    simd.greater_than_or_equal_f64s(magnitude, minimum_normal),
                );
                let bounded = simd.less_than_or_equal_f64s(magnitude, maximum_input);
                normal_inputs =
                    simd.and_m64s(normal_inputs, simd.and_m64s(normal_or_zero, bounded));
            }
            let cosine_in_range =
                simd.less_than_or_equal_f64s(simd.abs_f64s(cosine_vectors[index]), one);
            if !analysis_mask_all(simd, simd.and_m64s(normal_inputs, cosine_in_range)) {
                return false;
            }
        }

        for lane in 0..same_real_tail.len() {
            for value in [
                same_real_tail[lane],
                same_imag_tail[lane],
                opposite_real_tail[lane],
                opposite_imag_tail[lane],
            ] {
                if !(value.is_normal() || value == 0.0) || value.abs() > self.max_input {
                    return false;
                }
            }
            if !cosine_tail[lane].is_finite() || cosine_tail[lane].abs() > 1.0 {
                return false;
            }
        }
        true
    }
}

/// Checks the ordinary-range accumulation bound while applying the opposite
/// parity cosine weight. A failed result may leave earlier lanes weighted;
/// callers restore the packed opposite channels from the original spectra.
pub(crate) fn prepare_analysis_products_and_opposite(
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &mut [f64],
    opposite_imag: &mut [f64],
    cos_theta: &[f64],
) -> (bool, bool) {
    debug_assert_eq!(same_real.len(), same_imag.len());
    debug_assert_eq!(same_real.len(), opposite_real.len());
    debug_assert_eq!(same_real.len(), opposite_imag.len());
    debug_assert_eq!(same_real.len(), cos_theta.len());
    let max_input = f64::MAX / scalar::BIG / (2.0 * same_real.len().max(1) as f64);
    let mut products_safe = true;
    let mut opposite_safe = true;

    for lane in 0..same_real.len() {
        let same_re = same_real[lane];
        let same_im = same_imag[lane];
        let opposite_re = opposite_real[lane];
        let opposite_im = opposite_imag[lane];
        let cosine = cos_theta[lane];
        let lane_inputs_safe = [same_re, same_im, opposite_re, opposite_im]
            .into_iter()
            .all(|value| (value.is_normal() || value == 0.0) && value.abs() <= max_input)
            && cosine.is_finite()
            && cosine.abs() <= 1.0;
        if !lane_inputs_safe {
            products_safe = false;
            opposite_safe = false;
            continue;
        }

        let weighted_real = opposite_re * cosine;
        let weighted_imag = opposite_im * cosine;
        let real_safe = weighted_real.is_normal()
            || (weighted_real == 0.0 && (opposite_re == 0.0 || cosine == 0.0));
        let imag_safe = weighted_imag.is_normal()
            || (weighted_imag == 0.0 && (opposite_im == 0.0 || cosine == 0.0));
        if real_safe && imag_safe {
            opposite_real[lane] = weighted_real;
            opposite_imag[lane] = weighted_imag;
        } else {
            opposite_safe = false;
        }
    }

    (products_safe, products_safe && opposite_safe)
}

/// Precision-generic input safety check and opposite-parity preweighting for
/// the shared native scalar-analysis kernel.
pub(crate) fn prepare_analysis_products_and_opposite_typed<T: LegendrePrecision>(
    same_real: &[T],
    same_imag: &[T],
    opposite_real: &mut [T],
    opposite_imag: &mut [T],
    cos_theta: &[T],
) -> (bool, bool) {
    debug_assert_eq!(same_real.len(), same_imag.len());
    debug_assert_eq!(same_real.len(), opposite_real.len());
    debug_assert_eq!(same_real.len(), opposite_imag.len());
    debug_assert_eq!(same_real.len(), cos_theta.len());
    let lane_count = T::from_usize(same_real.len().max(1)).expect("lane count is representable");
    let max_input = T::max_value() / T::BIG / (T::one() + T::one()) / lane_count;
    let mut products_safe = true;
    let mut opposite_safe = true;

    for lane in 0..same_real.len() {
        let values = [
            same_real[lane],
            same_imag[lane],
            opposite_real[lane],
            opposite_imag[lane],
        ];
        let cosine = cos_theta[lane];
        let inputs_safe = values
            .into_iter()
            .all(|value| (value.is_normal() || value == T::zero()) && value.abs() <= max_input)
            && cosine.is_finite()
            && cosine.abs() <= T::one();
        if !inputs_safe {
            products_safe = false;
            opposite_safe = false;
            continue;
        }

        let weighted_real = opposite_real[lane] * cosine;
        let weighted_imag = opposite_imag[lane] * cosine;
        let real_safe = weighted_real.is_normal()
            || (weighted_real == T::zero()
                && (opposite_real[lane] == T::zero() || cosine == T::zero()));
        let imag_safe = weighted_imag.is_normal()
            || (weighted_imag == T::zero()
                && (opposite_imag[lane] == T::zero() || cosine == T::zero()));
        if real_safe && imag_safe {
            opposite_real[lane] = weighted_real;
            opposite_imag[lane] = weighted_imag;
        } else {
            opposite_safe = false;
        }
    }
    (products_safe, products_safe && opposite_safe)
}

/// Applies DUCC's cosine weight to the opposite channels while checking that
/// the transformation does not introduce subnormal products.
///
/// A `false` result can leave part of the opposite channels weighted; callers
/// must restore them from the original spectra before using the scaled path.
pub(crate) fn prepare_analysis_opposite(
    opposite_real: &mut [f64],
    opposite_imag: &mut [f64],
    cos_theta: &[f64],
) -> bool {
    debug_assert_eq!(opposite_real.len(), opposite_imag.len());
    debug_assert_eq!(opposite_real.len(), cos_theta.len());
    for ((real, imag), &cosine) in opposite_real.iter_mut().zip(opposite_imag).zip(cos_theta) {
        let weighted_real = *real * cosine;
        let weighted_imag = *imag * cosine;
        let real_safe =
            weighted_real.is_normal() || (weighted_real == 0.0 && (*real == 0.0 || cosine == 0.0));
        let imag_safe =
            weighted_imag.is_normal() || (weighted_imag == 0.0 && (*imag == 0.0 || cosine == 0.0));
        if !real_safe || !imag_safe {
            return false;
        }
        *real = weighted_real;
        *imag = weighted_imag;
    }
    true
}

/// Initializes ordinary-range sectoral values across SIMD latitude lanes.
///
/// Returns `false` when any lane needs the existing scaled scalar path. The
/// candidate values are checked before the recurrence consumes them.
fn initialize_normal_analysis_sectoral_dispatched(
    arch: Arch,
    m: usize,
    sin_theta: &[f64],
    sectoral_norm: f64,
    current: &mut [f64],
) -> Result<NormalSectoralInitialization, ShtError> {
    arch.dispatch(InitializeNormalAnalysisSectoral {
        m,
        sin_theta,
        sectoral_norm,
        current,
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "The shared initializer receives the order geometry and caller-owned recurrence lanes explicitly."
)]
fn initialize_analysis_sectoral_lanes(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    sectoral_norm: f64,
    previous: &mut [f64],
    current: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    debug_assert!(previous.len() >= sin_theta.len());
    debug_assert!(current.len() >= sin_theta.len());
    debug_assert!(exponents.len() >= sin_theta.len());
    previous[..sin_theta.len()].fill(0.0);

    let initialization = if lmax >= SIMD_SECTORAL_INIT_MIN_LMAX {
        initialize_normal_analysis_sectoral_dispatched(
            arch,
            m,
            sin_theta,
            sectoral_norm,
            &mut current[..sin_theta.len()],
        )?
    } else {
        NormalSectoralInitialization::default()
    };
    if initialization.complete {
        exponents[..sin_theta.len()].fill(0);
        return Ok(true);
    }

    for lane in 0..sin_theta.len() {
        let (value, exponent) = scalar::sectoral_value(sin_theta[lane], m, sectoral_norm)?;
        current[lane] = value;
        exponents[lane] = exponent;
    }
    scalar::normalize_pair_lanes(
        &mut previous[..sin_theta.len()],
        &mut current[..sin_theta.len()],
        &mut exponents[..sin_theta.len()],
    )
}

/// Initializes the analysis recurrence with DUCC's `mypow` scale epochs.
/// The ordinary initializer remains the cheap path when every seed is already
/// in RustSHT's normal range; this route handles mixed and underflowing seeds
/// without falling back to one scalar power per latitude lane.
#[expect(
    clippy::too_many_arguments,
    reason = "The scaled initializer writes caller-owned recurrence lanes and exponent state."
)]
fn initialize_scaled_analysis_sectoral_lanes(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    sectoral_norm: f64,
    previous: &mut [f64],
    current: &mut [f64],
    scale_epochs: &mut [f64],
) -> Result<Option<bool>, ShtError> {
    debug_assert!(previous.len() >= sin_theta.len());
    debug_assert!(current.len() >= sin_theta.len());
    debug_assert!(scale_epochs.len() >= sin_theta.len());
    previous[..sin_theta.len()].fill(0.0);

    let initialization = if lmax >= SIMD_SECTORAL_INIT_MIN_LMAX {
        initialize_normal_analysis_sectoral_dispatched(
            arch,
            m,
            sin_theta,
            sectoral_norm,
            &mut current[..sin_theta.len()],
        )?
    } else {
        NormalSectoralInitialization::default()
    };
    if initialization.complete {
        return Ok(Some(true));
    }

    let initialized_scaled = arch.dispatch(InitializeScaledAnalysisSectoral {
        m,
        sin_theta,
        sectoral_norm,
        normal_prefix_vectors: initialization.completed_vectors,
        current: &mut current[..sin_theta.len()],
        scale_epochs: &mut scale_epochs[..sin_theta.len()],
    });
    if initialized_scaled.is_some() {
        return Ok(Some(false));
    }

    // An invalid or unrepresentable SIMD seed uses the robust order fallback.
    Ok(None)
}

#[derive(Clone, Copy, Default)]
struct NormalSectoralInitialization {
    complete: bool,
    completed_vectors: usize,
}

struct InitializeScaledAnalysisSectoral<'a> {
    m: usize,
    sin_theta: &'a [f64],
    sectoral_norm: f64,
    normal_prefix_vectors: usize,
    current: &'a mut [f64],
    scale_epochs: &'a mut [f64],
}

impl WithSimd for InitializeScaledAnalysisSectoral<'_> {
    type Output = Option<()>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let InitializeScaledAnalysisSectoral {
            m,
            sin_theta,
            sectoral_norm,
            normal_prefix_vectors,
            current,
            scale_epochs,
        } = self;
        if !sectoral_norm.is_finite() || sectoral_norm <= 0.0 {
            return None;
        }
        let (sine_vectors, sine_tail) = S::as_simd_f64s(sin_theta);
        let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
        let (scale_vectors, scale_tail) = S::as_mut_simd_f64s(scale_epochs);
        if current_vectors.len() != sine_vectors.len()
            || current_tail.len() != sine_tail.len()
            || scale_vectors.len() != sine_vectors.len()
            || scale_tail.len() != sine_tail.len()
            || normal_prefix_vectors > sine_vectors.len()
        {
            return None;
        }

        let one = simd.splat_f64s(1.0);
        let zero_scale = simd.splat_f64s(0.0);
        let maximum_finite = simd.splat_f64s(f64::MAX);
        let sine_limit = simd.splat_f64s(1.0);
        let power_limit = simd.splat_f64s(analysis_sectoral_power_limit(m));
        let coefficient =
            simd.splat_f64s(sectoral_norm * if m.is_multiple_of(2) { 1.0 } else { -1.0 });

        for scale in &mut scale_vectors[..normal_prefix_vectors] {
            *scale = zero_scale;
        }
        for index in normal_prefix_vectors..sine_vectors.len() {
            let sine = sine_vectors[index];
            let sine_magnitude = simd.abs_f64s(sine);
            if !analysis_mask_all(
                simd,
                simd.and_m64s(
                    simd.less_than_or_equal_f64s(sine_magnitude, sine_limit),
                    simd.less_than_or_equal_f64s(sine_magnitude, maximum_finite),
                ),
            ) {
                return None;
            }

            // DUCC's mypow uses an unscaled exponentiation for a SIMD vector
            // when every lane is above its per-order underflow limit. Keep
            // that quick route for safe vectors even when another vector in
            // this block needs scaled initialization.
            if analysis_mask_all(
                simd,
                simd.greater_than_or_equal_f64s(sine_magnitude, power_limit),
            ) {
                let mut value = simd.mul_f64s(integer_power_vector(simd, sine, m), coefficient);
                let mut value_scale = zero_scale;
                if !normalize_analysis_power_vector(
                    simd,
                    &mut value,
                    &mut value_scale,
                    ANALYSIS_FTOL,
                ) {
                    return None;
                }
                current_vectors[index] = value;
                scale_vectors[index] = value_scale;
                continue;
            }

            let mut base = sine;
            let mut base_scale = zero_scale;
            if !normalize_analysis_power_vector(simd, &mut base, &mut base_scale, ANALYSIS_FBIGHALF)
            {
                return None;
            }

            let mut value = one;
            let mut value_scale = zero_scale;
            let mut remaining = m;
            while remaining != 0 {
                if remaining & 1 != 0 {
                    value = simd.mul_f64s(value, base);
                    value_scale = simd.add_f64s(value_scale, base_scale);
                    if !normalize_analysis_power_vector(
                        simd,
                        &mut value,
                        &mut value_scale,
                        ANALYSIS_FBIGHALF,
                    ) {
                        return None;
                    }
                }
                remaining >>= 1;
                if remaining != 0 {
                    base = simd.mul_f64s(base, base);
                    base_scale = simd.add_f64s(base_scale, base_scale);
                    if !normalize_analysis_power_vector(
                        simd,
                        &mut base,
                        &mut base_scale,
                        ANALYSIS_FBIGHALF,
                    ) {
                        return None;
                    }
                }
            }

            value = simd.mul_f64s(value, coefficient);
            if !normalize_analysis_power_vector(
                simd,
                &mut value,
                &mut value_scale,
                ANALYSIS_FBIGHALF,
            ) || !normalize_analysis_power_vector(
                simd,
                &mut value,
                &mut value_scale,
                ANALYSIS_FTOL,
            ) {
                return None;
            }
            current_vectors[index] = value;
            scale_vectors[index] = value_scale;
        }

        for lane in 0..sine_tail.len() {
            let sine = sine_tail[lane];
            if !sine.is_finite() || sine.abs() > 1.0 {
                return None;
            }
            let Ok((value, exponent)) = scalar::sectoral_value(sine, m, sectoral_norm) else {
                return None;
            };
            let (value, exponent) = normalize_analysis_seed_scalar(value, exponent)?;
            let scale = exponent as f64 / ANALYSIS_SCALE_STEP as f64;
            if !scale.is_finite() {
                return None;
            }
            current_tail[lane] = value;
            scale_tail[lane] = scale;
        }
        Some(())
    }
}

#[inline]
fn analysis_sectoral_power_limit(m: usize) -> f64 {
    if m == 0 {
        0.0
    } else {
        (-400.0 * std::f64::consts::LN_2 / m as f64).exp()
    }
}

#[inline(always)]
fn normalize_analysis_power_vector<S: Simd>(
    simd: S,
    value: &mut S::f64s,
    scale: &mut S::f64s,
    maximum_value: f64,
) -> bool {
    let maximum = simd.splat_f64s(maximum_value);
    let minimum = simd.splat_f64s(maximum_value * ANALYSIS_SCALE_DOWN);
    let maximum_finite = simd.splat_f64s(f64::MAX);
    let zero = simd.splat_f64s(0.0);
    let one = simd.splat_f64s(1.0);

    let mut magnitude = simd.abs_f64s(*value);
    if !analysis_mask_all(
        simd,
        simd.less_than_or_equal_f64s(magnitude, maximum_finite),
    ) {
        return false;
    }
    let too_large = simd.greater_than_f64s(magnitude, maximum);
    let nonzero = simd.not_m64s(simd.equal_f64s(magnitude, zero));
    let too_small = simd.and_m64s(nonzero, simd.less_than_f64s(magnitude, minimum));
    if !analysis_mask_any(simd, simd.or_m64s(too_large, too_small)) {
        return true;
    }
    loop {
        let too_large = simd.greater_than_f64s(magnitude, maximum);
        if !analysis_mask_any(simd, too_large) {
            break;
        }
        let next_scale = simd.add_f64s(*scale, one);
        if !analysis_mask_all(
            simd,
            simd.less_than_or_equal_f64s(simd.abs_f64s(next_scale), simd.splat_f64s(4.5e15)),
        ) {
            return false;
        }
        *value = simd.select_f64s(
            too_large,
            simd.mul_f64s(*value, simd.splat_f64s(ANALYSIS_SCALE_DOWN)),
            *value,
        );
        *scale = simd.select_f64s(too_large, next_scale, *scale);
        magnitude = simd.abs_f64s(*value);
    }
    loop {
        let nonzero = simd.not_m64s(simd.equal_f64s(magnitude, zero));
        let too_small = simd.and_m64s(nonzero, simd.less_than_f64s(magnitude, minimum));
        if !analysis_mask_any(simd, too_small) {
            break;
        }
        let next_scale = simd.sub_f64s(*scale, one);
        if !analysis_mask_all(
            simd,
            simd.less_than_or_equal_f64s(simd.abs_f64s(next_scale), simd.splat_f64s(4.5e15)),
        ) {
            return false;
        }
        *value = simd.select_f64s(
            too_small,
            simd.mul_f64s(*value, simd.splat_f64s(ANALYSIS_SCALE_UP)),
            *value,
        );
        *scale = simd.select_f64s(too_small, next_scale, *scale);
        magnitude = simd.abs_f64s(*value);
    }
    true
}

fn normalize_analysis_seed_scalar(mut value: f64, mut exponent: i64) -> Option<(f64, i64)> {
    if !value.is_finite() {
        return None;
    }
    let minimum = ANALYSIS_FTOL * ANALYSIS_SCALE_DOWN;
    while value.abs() > ANALYSIS_FTOL {
        value *= ANALYSIS_SCALE_DOWN;
        exponent = exponent.checked_add(ANALYSIS_SCALE_STEP)?;
    }
    while value != 0.0 && value.abs() < minimum {
        value *= ANALYSIS_SCALE_UP;
        exponent = exponent.checked_sub(ANALYSIS_SCALE_STEP)?;
    }
    Some((value, exponent))
}

struct InitializeNormalAnalysisSectoral<'a> {
    m: usize,
    sin_theta: &'a [f64],
    sectoral_norm: f64,
    current: &'a mut [f64],
}

impl WithSimd for InitializeNormalAnalysisSectoral<'_> {
    type Output = Result<NormalSectoralInitialization, ShtError>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let InitializeNormalAnalysisSectoral {
            m,
            sin_theta,
            sectoral_norm,
            current,
        } = self;
        debug_assert_eq!(sin_theta.len(), current.len());

        if !sectoral_norm.is_finite() || sectoral_norm <= 0.0 {
            return Ok(NormalSectoralInitialization::default());
        }
        let (sine_vectors, sine_tail) = S::as_simd_f64s(sin_theta);
        let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
        debug_assert_eq!(sine_vectors.len(), current_vectors.len());
        debug_assert_eq!(sine_tail.len(), current_tail.len());

        let one = simd.splat_f64s(1.0);
        let zero = simd.splat_f64s(0.0);
        let sine_max = simd.splat_f64s(1.0);
        let sign = if m.is_multiple_of(2) { 1.0 } else { -1.0 };
        let factor = simd.splat_f64s(sectoral_norm * sign);
        let minimum = simd.splat_f64s(scalar::SMALL);
        let maximum = simd.splat_f64s(scalar::BIG);
        for (index, (output, &sine)) in current_vectors.iter_mut().zip(sine_vectors).enumerate() {
            let value = simd.mul_f64s(integer_power_vector(simd, sine, m), factor);
            let magnitude = simd.abs_f64s(value);
            let zero_result_is_valid = simd.and_m64s(
                simd.equal_f64s(magnitude, zero),
                simd.equal_f64s(simd.abs_f64s(sine), zero),
            );
            let above_minimum = simd.or_m64s(
                simd.greater_than_or_equal_f64s(magnitude, minimum),
                zero_result_is_valid,
            );
            let in_range = simd.and_m64s(
                simd.and_m64s(
                    above_minimum,
                    simd.less_than_or_equal_f64s(magnitude, maximum),
                ),
                simd.less_than_or_equal_f64s(simd.abs_f64s(sine), sine_max),
            );
            if simd.reduce_min_f64s(simd.select_f64s(in_range, one, zero)) != 1.0 {
                return Ok(NormalSectoralInitialization {
                    complete: false,
                    completed_vectors: index,
                });
            }
            *output = value;
        }
        for (&sine, output) in sine_tail.iter().zip(current_tail) {
            if !sine.is_finite() || sine.abs() > 1.0 {
                return Ok(NormalSectoralInitialization {
                    complete: false,
                    completed_vectors: sine_vectors.len(),
                });
            }
            let (value, exponent) = scalar::sectoral_value(sine, m, sectoral_norm)?;
            if exponent != 0 || !scalar::scalar_value_in_normal_range(value) {
                return Ok(NormalSectoralInitialization {
                    complete: false,
                    completed_vectors: sine_vectors.len(),
                });
            }
            *output = value;
        }
        Ok(NormalSectoralInitialization {
            complete: true,
            completed_vectors: sine_vectors.len(),
        })
    }
}

#[inline(always)]
fn integer_power_vector<S: Simd>(simd: S, mut base: S::f64s, mut exponent: usize) -> S::f64s {
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

/// Computes a scalar analysis block, transitioning scaled lanes to the
/// ordinary SIMD recurrence when the complete active block is in range.
///
/// Returns `false` when a later ordinary recurrence step leaves the safe range.
/// Staged coefficients may then be partial and must be discarded by the caller.
#[expect(
    clippy::too_many_arguments,
    reason = "The fused scalar analysis block receives parity spectra and worker-owned recurrence scratch explicitly."
)]
#[cfg(test)]
#[allow(unexpected_cfgs)]
pub(crate) fn fused_scalar_analysis_pair_block_dispatched(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &[f64],
    opposite_imag: &[f64],
    staged_coefficients: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    let mut cos_theta_squared = [0.0; SCALAR_ANALYSIS_PAIR_TILE_SIZE];
    for ((squared, &sine), &cosine) in cos_theta_squared.iter_mut().zip(sin_theta).zip(cos_theta) {
        *squared = if cosine.abs() > 0.99 {
            (1.0 - sine) * (1.0 + sine)
        } else {
            cosine * cosine
        };
    }
    let mut range_check_was_exercised = false;
    fused_scalar_analysis_pair_block_impl::<false, { !cfg!(rustsht_unchecked_analysis_pair_range) }>(
        arch,
        m,
        lmax,
        false,
        &mut range_check_was_exercised,
        sin_theta,
        cos_theta,
        &cos_theta_squared[..sin_theta.len()],
        sectoral_norm,
        coefficient_a,
        coefficient_b,
        same_real,
        same_imag,
        opposite_real,
        opposite_imag,
        staged_coefficients,
        previous,
        current,
        next,
        &mut [],
        &mut [],
        exponents,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The preweighted block receives active geometry, spectra and worker scratch explicitly."
)]
/// Runs a preweighted scalar-analysis block.
///
/// `range_check_is_proven_safe` may disable the ordinary recurrence group scan
/// only after this block completed checked validation for the same order, plan,
/// latitude geometry, prefix mode and SIMD backend. Input-product checks remain
/// the caller's responsibility.
/// `range_check_was_exercised` reports whether the checked ordinary recurrence
/// group scan ran for this block, including after scaled-to-ordinary promotion.
#[allow(unexpected_cfgs)]
pub(crate) fn fused_scalar_analysis_pair_block_preweighted_dispatched(
    arch: Arch,
    m: usize,
    lmax: usize,
    allow_insignificant_prefix: bool,
    range_check_is_proven_safe: bool,
    range_check_was_exercised: &mut bool,
    sin_theta: &[f64],
    cos_theta: &[f64],
    cos_theta_squared: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &[f64],
    opposite_imag: &[f64],
    staged_coefficients: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    scale_epochs: &mut [f64],
    scale_corrections: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    *range_check_was_exercised = false;
    if range_check_is_proven_safe {
        fused_scalar_analysis_pair_block_impl::<true, false>(
            arch,
            m,
            lmax,
            allow_insignificant_prefix,
            range_check_was_exercised,
            sin_theta,
            cos_theta,
            cos_theta_squared,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            previous,
            current,
            next,
            scale_epochs,
            scale_corrections,
            exponents,
        )
    } else {
        fused_scalar_analysis_pair_block_impl::<
            true,
            { !cfg!(rustsht_unchecked_analysis_pair_range) },
        >(
            arch,
            m,
            lmax,
            allow_insignificant_prefix,
            range_check_was_exercised,
            sin_theta,
            cos_theta,
            cos_theta_squared,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            previous,
            current,
            next,
            scale_epochs,
            scale_corrections,
            exponents,
        )
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The scalar analysis block shares caller-owned coefficients, spectra and recurrence scratch."
)]
fn fused_scalar_analysis_pair_block_impl<
    const OPPOSITE_COSINE_WEIGHTED: bool,
    const CHECK_PAIR_GROUP_RANGE: bool,
>(
    arch: Arch,
    m: usize,
    lmax: usize,
    allow_insignificant_prefix: bool,
    range_check_was_exercised: &mut bool,
    sin_theta: &[f64],
    cos_theta: &[f64],
    cos_theta_squared: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &[f64],
    opposite_imag: &[f64],
    staged_coefficients: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    scale_epochs: &mut [f64],
    scale_corrections: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    debug_assert!(m <= lmax);
    let coefficient_count = lmax - m + 1;
    debug_assert!(sin_theta.len() <= SCALAR_ANALYSIS_PAIR_TILE_SIZE);
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    debug_assert_eq!(sin_theta.len(), cos_theta_squared.len());
    debug_assert_eq!(sin_theta.len(), same_real.len());
    debug_assert_eq!(sin_theta.len(), same_imag.len());
    debug_assert_eq!(sin_theta.len(), opposite_real.len());
    debug_assert_eq!(sin_theta.len(), opposite_imag.len());
    debug_assert!(staged_coefficients.len() >= coefficient_count);
    debug_assert!(previous.len() >= sin_theta.len());
    debug_assert!(current.len() >= sin_theta.len());
    debug_assert!(next.len() >= sin_theta.len());
    debug_assert!(exponents.len() >= sin_theta.len());

    if sin_theta.is_empty() {
        return Ok(false);
    }

    debug_assert!(
        !(allow_insignificant_prefix && OPPOSITE_COSINE_WEIGHTED)
            || scale_epochs.len() >= sin_theta.len()
    );
    debug_assert!(
        !(allow_insignificant_prefix && OPPOSITE_COSINE_WEIGHTED)
            || scale_corrections.len() >= sin_theta.len()
    );
    let initialized_normal = if allow_insignificant_prefix && OPPOSITE_COSINE_WEIGHTED {
        let Some(initialized_normal) = initialize_scaled_analysis_sectoral_lanes(
            arch,
            m,
            lmax,
            sin_theta,
            sectoral_norm,
            previous,
            current,
            scale_epochs,
        )?
        else {
            #[cfg(all(test, feature = "parallel"))]
            record_scalar_analysis_initialization_decline();
            return Ok(false);
        };
        initialized_normal
    } else {
        initialize_analysis_sectoral_lanes(
            arch,
            m,
            lmax,
            sin_theta,
            sectoral_norm,
            previous,
            current,
            exponents,
        )?
    };

    #[cfg(all(test, feature = "parallel"))]
    record_scalar_analysis_seed_route(
        initialized_normal,
        allow_insignificant_prefix && OPPOSITE_COSINE_WEIGHTED,
    );

    if !initialized_normal {
        let prefix_route = allow_insignificant_prefix && OPPOSITE_COSINE_WEIGHTED;
        let succeeded = if prefix_route {
            arch.dispatch(FusedScaledScalarAnalysisPair::<
                OPPOSITE_COSINE_WEIGHTED,
                CHECK_PAIR_GROUP_RANGE,
            > {
                m,
                lmax,
                range_check_was_exercised,
                cos_theta,
                cos_theta_squared,
                coefficient_a,
                coefficient_b,
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                staged_coefficients: &mut staged_coefficients[..coefficient_count],
                previous: &mut previous[..sin_theta.len()],
                current: &mut current[..sin_theta.len()],
                next: &mut next[..sin_theta.len()],
                scale_epochs: &mut scale_epochs[..sin_theta.len()],
                scale_corrections: &mut scale_corrections[..sin_theta.len()],
            })?
        } else {
            arch.dispatch(FusedCheckedScaledScalarAnalysisPair::<
                OPPOSITE_COSINE_WEIGHTED,
                CHECK_PAIR_GROUP_RANGE,
            > {
                m,
                lmax,
                range_check_was_exercised,
                cos_theta,
                cos_theta_squared,
                coefficient_a,
                coefficient_b,
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                staged_coefficients: &mut staged_coefficients[..coefficient_count],
                previous: &mut previous[..sin_theta.len()],
                current: &mut current[..sin_theta.len()],
                next: &mut next[..sin_theta.len()],
                exponents: &mut exponents[..sin_theta.len()],
            })?
        };
        #[cfg(all(test, feature = "parallel"))]
        if !succeeded {
            record_scalar_analysis_recurrence_decline(prefix_route);
        }
        return Ok(succeeded);
    }

    // A successful block overwrites every coefficient; a declined block's scratch is discarded.
    let succeeded = arch.dispatch(FusedScalarAnalysisPair::<
        OPPOSITE_COSINE_WEIGHTED,
        CHECK_PAIR_GROUP_RANGE,
    > {
        m,
        lmax,
        range_check_was_exercised,
        pair_start: 0,
        cos_theta,
        cos_theta_squared,
        coefficient_a,
        coefficient_b,
        same_real,
        same_imag,
        opposite_real,
        opposite_imag,
        staged_coefficients: &mut staged_coefficients[..coefficient_count],
        second_channel: None,
        previous: &mut previous[..sin_theta.len()],
        current: &mut current[..sin_theta.len()],
        next: &mut next[..sin_theta.len()],
    });
    Ok(succeeded)
}

#[inline(always)]
pub(super) fn analysis_mask_all<S: Simd>(simd: S, mask: S::m64s) -> bool {
    #[cfg(all(target_arch = "x86_64", rustsht_avx2_mask_all))]
    if const {
        S::F64_LANES == 4
            && core::mem::size_of::<S::m64s>() == core::mem::size_of::<std::arch::x86_64::__m256d>()
    } {
        // SAFETY: the four-lane Pulp mask is the AVX2 lane mask selected only
        // by the V3 dispatch path. Its size check above permits a bitwise copy.
        let mask = unsafe { core::mem::transmute_copy(&mask) };
        // SAFETY: the Pulp V3 specialization is dispatched only when AVX2 is
        // supported by the current CPU.
        return unsafe { analysis_mask_all_avx2(mask) };
    }

    simd.reduce_min_f64s(simd.select_f64s(mask, simd.splat_f64s(1.0), simd.splat_f64s(0.0))) > 0.0
}

#[inline(always)]
fn analysis_mask_lane_bits<S: Simd>(mask: S::m64s) -> Option<u8> {
    #[cfg(target_arch = "x86_64")]
    if const {
        S::F64_LANES == 4
            && core::mem::size_of::<S::m64s>() == core::mem::size_of::<std::arch::x86_64::__m256d>()
    } {
        // SAFETY: the size test permits a bitwise copy of the four-lane comparison mask.
        let mask = unsafe { core::mem::transmute_copy(&mask) };
        // SAFETY: the caller obtains this four-lane mask through SIMD dispatch.
        return Some(unsafe { analysis_mask_lane_bits_avx(mask) });
    }

    let _ = mask;
    None
}

#[allow(dead_code, unexpected_cfgs)]
#[cfg(all(target_arch = "x86_64", rustsht_avx2_mask_all))]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn analysis_mask_all_avx2(mask: std::arch::x86_64::__m256d) -> bool {
    use std::arch::x86_64::_mm256_movemask_pd;

    // Pulp comparison masks use an all-ones lane for true and zero for false.
    _mm256_movemask_pd(mask) == 0b1111
}

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx")]
#[inline]
unsafe fn analysis_mask_lane_bits_avx(mask: std::arch::x86_64::__m256d) -> u8 {
    use std::arch::x86_64::_mm256_movemask_pd;

    // Pulp comparison masks use an all-ones lane for true and zero for false.
    _mm256_movemask_pd(mask) as u8
}

#[inline(always)]
fn analysis_mask_any<S: Simd>(simd: S, mask: S::m64s) -> bool {
    simd.reduce_max_f64s(simd.select_f64s(mask, simd.splat_f64s(1.0), simd.splat_f64s(0.0))) > 0.0
}

/// Tests DUCC's `iter_to_ieee` scale-epoch frontier. Sectoral initialization
/// normalizes each lane to `sht_ftol`; the recurrence increments its epoch
/// whenever a lane crosses that threshold.
#[inline(always)]
#[cfg(test)]
fn analysis_scale_epochs_below_one<S: Simd>(simd: S, scale_epochs: &[f64]) -> bool {
    let (scale_vectors, scale_tail) = S::as_simd_f64s(scale_epochs);
    let one = simd.splat_f64s(1.0);
    for &scale in scale_vectors {
        if !analysis_mask_all(simd, simd.less_than_f64s(scale, one)) {
            return false;
        }
    }
    scale_tail.iter().all(|&scale| scale < 1.0)
}

#[inline(always)]
fn analysis_recurrence_state_is_finite(previous: &[f64], current: &[f64]) -> bool {
    previous.len() == current.len()
        && previous
            .iter()
            .chain(current)
            .all(|value| value.is_finite())
}

#[inline(always)]
fn analysis_value_in_ieee_range(value: f64) -> bool {
    value.is_finite()
        && (value == 0.0 || (value.abs() >= f64::MIN_POSITIVE && value.abs() <= scalar::BIG))
}

#[inline(always)]
fn analysis_scale_correction(value: f64, scale_epoch: f64) -> Option<f64> {
    if !value.is_finite() || !scale_epoch.is_finite() {
        return None;
    }
    if scale_epoch < 0.0 {
        return Some(0.0);
    }
    let correction = if scale_epoch < 1.0 {
        1.0
    } else {
        ANALYSIS_SCALE_UP
    };
    analysis_value_in_ieee_range(value * correction).then_some(correction)
}

fn prepare_analysis_scale_corrections(
    basis: &[f64],
    scale_epochs: &[f64],
    corrections: &mut [f64],
) -> Option<(bool, bool)> {
    debug_assert_eq!(basis.len(), scale_epochs.len());
    debug_assert!(corrections.len() >= basis.len());
    let mut below_limit = true;
    let mut all_negative = true;
    for ((correction, &value), &scale_epoch) in corrections.iter_mut().zip(basis).zip(scale_epochs)
    {
        below_limit &= scale_epoch < 1.0;
        all_negative &= scale_epoch < 0.0;
        let next = analysis_scale_correction(value, scale_epoch)?;
        *correction = next;
    }
    Some((below_limit, all_negative))
}

/// Checks the scale range once before the recurrence. At most one epoch is
/// added per paired-degree step, so the inner loop need not recheck the same
/// exponent bound after every SIMD vector update.
fn analysis_scale_epochs_allow_recurrence(scale_epochs: &[f64], pair_count: usize) -> bool {
    let increments = pair_count as f64;
    if !increments.is_finite() || increments > ANALYSIS_MAX_SCALE_EPOCH {
        return false;
    }
    let maximum_start_epoch = ANALYSIS_MAX_SCALE_EPOCH - increments;
    scale_epochs.iter().all(|&epoch| {
        epoch.is_finite() && epoch >= -ANALYSIS_MAX_SCALE_EPOCH && epoch <= maximum_start_epoch
    })
}

/// Advances two recurrence pairs and applies DUCC's scale threshold in one
/// latitude pass. The caller enters with every epoch below one; carry the
/// updated below-limit state forward instead of rereading the epoch array
/// after each pair group. This route discards coefficients until the
/// recurrence reaches IEEE scale.
#[inline(always)]
fn advance_and_rescale_analysis_pair<S: Simd>(
    simd: S,
    coefficients: [(f64, f64); 2],
    cos_theta_squared: &[f64],
    recurrence: (&mut [f64], &mut [f64]),
    scale_epochs: &mut [f64],
    corrections: &mut [f64],
    skip_insignificant_validation: bool,
) -> Option<(bool, bool)> {
    let [
        (coefficient_a1, coefficient_b1),
        (coefficient_a2, coefficient_b2),
    ] = coefficients;
    let (previous, current) = recurrence;
    if previous.len() != current.len()
        || current.len() != cos_theta_squared.len()
        || current.len() != scale_epochs.len()
        || current.len() != corrections.len()
    {
        return None;
    }
    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
    let (cosine_squared_vectors, cosine_squared_tail) = S::as_simd_f64s(cos_theta_squared);
    let (scale_vectors, scale_tail) = S::as_mut_simd_f64s(scale_epochs);
    let (correction_vectors, correction_tail) = S::as_mut_simd_f64s(corrections);
    let a1 = simd.splat_f64s(coefficient_a1);
    let b1 = simd.splat_f64s(coefficient_b1);
    let a2 = simd.splat_f64s(coefficient_a2);
    let b2 = simd.splat_f64s(coefficient_b2);
    let zero = simd.splat_f64s(0.0);
    let one = simd.splat_f64s(1.0);
    let ftol = simd.splat_f64s(ANALYSIS_FTOL);
    let down = simd.splat_f64s(ANALYSIS_SCALE_DOWN);
    let up = simd.splat_f64s(ANALYSIS_SCALE_UP);
    let min_normal = simd.splat_f64s(f64::MIN_POSITIVE);
    let max_finite = simd.splat_f64s(f64::MAX);
    let max_mantissa = simd.splat_f64s(scalar::BIG);
    let mut all_vectors_valid = simd.equal_f64s(zero, zero);
    let mut all_rescaled_epochs_negative = simd.equal_f64s(zero, zero);
    let mut any_rescaled = false;
    let mut any_vector_rescaled = false;
    let mut below_limit = true;
    let mut all_tail_epochs_negative = true;

    for index in 0..current_vectors.len() {
        let previous_value = previous_vectors[index];
        let current_value = current_vectors[index];
        let cosine_squared = cosine_squared_vectors[index];
        let first_leading = simd.mul_add_f64s(a1, cosine_squared, b1);
        let first = simd.mul_add_f64s(first_leading, current_value, previous_value);
        let second_leading = simd.mul_add_f64s(a2, cosine_squared, b2);
        let second = simd.mul_add_f64s(second_leading, first, current_value);
        let magnitude = simd.abs_f64s(second);
        let rescale = simd.greater_than_f64s(magnitude, ftol);
        let vector_rescaled = simd.reduce_max_f64s(magnitude) > ANALYSIS_FTOL;
        any_rescaled |= vector_rescaled;
        any_vector_rescaled |= vector_rescaled;

        if !vector_rescaled {
            // The active prefix has scale epochs below one. Negative epochs
            // intentionally suppress insignificant mantissas; epoch zero
            // still requires an ordinary nonzero mantissa. Since every lane
            // is below ftol here, no upper-magnitude check is needed.
            if !skip_insignificant_validation {
                let finite_mantissa = simd.less_than_or_equal_f64s(magnitude, max_finite);
                let zero_or_normal = simd.or_m64s(
                    simd.equal_f64s(magnitude, zero),
                    simd.greater_than_or_equal_f64s(magnitude, min_normal),
                );
                let negative_epoch = simd.less_than_f64s(scale_vectors[index], zero);
                let current_valid = simd.and_m64s(
                    finite_mantissa,
                    simd.or_m64s(negative_epoch, zero_or_normal),
                );
                all_vectors_valid = simd.and_m64s(all_vectors_valid, current_valid);
            }
            previous_vectors[index] = first;
            current_vectors[index] = second;
            continue;
        }

        let updated_previous = simd.select_f64s(rescale, simd.mul_f64s(first, down), first);
        let updated_current = simd.select_f64s(rescale, simd.mul_f64s(second, down), second);
        let next_scale = simd.add_f64s(scale_vectors[index], one);
        let updated_scale = simd.select_f64s(rescale, next_scale, scale_vectors[index]);
        // A vector that did not rescale retains the entry invariant that all
        // epochs are below one. DUCC performs this all-lane test only for
        // vectors whose scale changed; avoid an unconditional reduction here.
        if vector_rescaled {
            below_limit &= analysis_mask_all(simd, simd.less_than_f64s(updated_scale, one));
            if skip_insignificant_validation {
                all_rescaled_epochs_negative = simd.and_m64s(
                    all_rescaled_epochs_negative,
                    simd.less_than_f64s(updated_scale, zero),
                );
            }
        }
        let correction = simd.select_f64s(
            simd.less_than_f64s(updated_scale, zero),
            zero,
            simd.select_f64s(simd.less_than_f64s(updated_scale, one), one, up),
        );
        let updated_correction = simd.select_f64s(rescale, correction, correction_vectors[index]);

        let corrected_current = simd.mul_f64s(updated_current, updated_correction);
        let current_magnitude = simd.abs_f64s(corrected_current);
        let finite_mantissa =
            simd.less_than_or_equal_f64s(simd.abs_f64s(updated_current), max_finite);
        let current_valid = simd.and_m64s(
            finite_mantissa,
            simd.or_m64s(
                simd.equal_f64s(current_magnitude, zero),
                simd.and_m64s(
                    simd.greater_than_or_equal_f64s(current_magnitude, min_normal),
                    simd.less_than_or_equal_f64s(current_magnitude, max_mantissa),
                ),
            ),
        );
        all_vectors_valid = simd.and_m64s(all_vectors_valid, current_valid);
        previous_vectors[index] = updated_previous;
        current_vectors[index] = updated_current;
        scale_vectors[index] = updated_scale;
        correction_vectors[index] = updated_correction;
    }

    for lane in 0..current_tail.len() {
        let first = coefficient_a1
            .mul_add(cosine_squared_tail[lane], coefficient_b1)
            .mul_add(current_tail[lane], previous_tail[lane]);
        let second = coefficient_a2
            .mul_add(cosine_squared_tail[lane], coefficient_b2)
            .mul_add(first, current_tail[lane]);
        if !first.is_finite() || !second.is_finite() {
            return None;
        }
        let rescale = second.abs() > ANALYSIS_FTOL;
        any_rescaled |= rescale;
        let (updated_previous, updated_current) = if rescale {
            let next_scale = scale_tail[lane] + 1.0;
            if !next_scale.is_finite()
                || next_scale == scale_tail[lane]
                || next_scale.abs() > 4.5e15
            {
                return None;
            }
            let previous_scaled = first * ANALYSIS_SCALE_DOWN;
            let current_scaled = second * ANALYSIS_SCALE_DOWN;
            if second != 0.0 && current_scaled.abs() < f64::MIN_POSITIVE {
                return None;
            }
            scale_tail[lane] = next_scale;
            below_limit &= next_scale < 1.0;
            correction_tail[lane] = analysis_scale_correction(current_scaled, next_scale)?;
            (previous_scaled, current_scaled)
        } else {
            (first, second)
        };
        if skip_insignificant_validation {
            all_tail_epochs_negative &= scale_tail[lane] < 0.0;
        }
        let corrected_current = updated_current * correction_tail[lane];
        if !analysis_value_in_ieee_range(corrected_current) {
            return None;
        }
        previous_tail[lane] = updated_previous;
        current_tail[lane] = updated_current;
    }

    if !analysis_mask_all(simd, all_vectors_valid) {
        return None;
    }
    if skip_insignificant_validation
        && any_rescaled
        && !previous
            .iter()
            .chain(current.iter())
            .all(|value| value.is_finite())
    {
        return None;
    }
    let remains_insignificant_prefix = skip_insignificant_validation
        && all_tail_epochs_negative
        && (!any_vector_rescaled || analysis_mask_all(simd, all_rescaled_epochs_negative));
    Some((below_limit, remains_insignificant_prefix))
}

/// Applies DUCC's per-lane `rescale` rule while the analysis recurrence is in
/// exponent space. Invalid scale or value ranges decline to the checked path.
#[inline(always)]
fn rescale_scaled_analysis_pair<S: Simd, const CHECK_NONNEGATIVE: bool>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale_epochs: &mut [f64],
    corrections: &mut [f64],
) -> Option<(bool, bool)> {
    if previous.len() != current.len()
        || current.len() != scale_epochs.len()
        || current.len() != corrections.len()
    {
        return None;
    }
    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
    let (scale_vectors, scale_tail) = S::as_mut_simd_f64s(scale_epochs);
    let (correction_vectors, correction_tail) = S::as_mut_simd_f64s(corrections);
    let zero_f64 = simd.splat_f64s(0.0);
    let ftol = simd.splat_f64s(ANALYSIS_FTOL);
    let down = simd.splat_f64s(ANALYSIS_SCALE_DOWN);
    let up = simd.splat_f64s(ANALYSIS_SCALE_UP);
    let one = simd.splat_f64s(1.0);
    let min_normal = simd.splat_f64s(f64::MIN_POSITIVE);
    let max_finite = simd.splat_f64s(f64::MAX);
    let max_mantissa = simd.splat_f64s(scalar::BIG);
    let mut all_vectors_valid = simd.equal_f64s(zero_f64, zero_f64);
    let mut all_nonnegative_lanes = simd.equal_f64s(zero_f64, zero_f64);
    let mut all_nonnegative_tails = true;
    let mut any_rescaled = false;

    for index in 0..current_vectors.len() {
        let previous_value = previous_vectors[index];
        let current_value = current_vectors[index];
        let current_magnitude = simd.abs_f64s(current_value);
        let rescale = simd.greater_than_f64s(current_magnitude, ftol);
        if !CHECK_NONNEGATIVE {
            // Reducing the magnitudes avoids materializing a 0/1 vector from
            // the comparison mask before the horizontal test. NaNs are
            // rejected by the accumulated range validation below.
            let vector_rescaled = simd.reduce_max_f64s(current_magnitude) > ANALYSIS_FTOL;
            any_rescaled |= vector_rescaled;
            if !vector_rescaled {
                // DUCC's rescale() returns immediately when no lane crosses
                // the threshold. Keep the Rust range check, but avoid scale,
                // correction and state-update work for this vector.
                let corrected_current = simd.mul_f64s(current_value, correction_vectors[index]);
                let corrected_magnitude = simd.abs_f64s(corrected_current);
                let current_is_finite = simd.less_than_or_equal_f64s(current_magnitude, max_finite);
                let current_valid = simd.and_m64s(
                    current_is_finite,
                    simd.or_m64s(
                        simd.equal_f64s(corrected_magnitude, zero_f64),
                        simd.and_m64s(
                            simd.greater_than_or_equal_f64s(corrected_magnitude, min_normal),
                            simd.less_than_or_equal_f64s(corrected_magnitude, max_mantissa),
                        ),
                    ),
                );
                all_vectors_valid = simd.and_m64s(all_vectors_valid, current_valid);
                continue;
            }
        }
        let next_scale = simd.add_f64s(scale_vectors[index], one);
        let previous_scaled =
            simd.select_f64s(rescale, simd.mul_f64s(previous_value, down), previous_value);
        let current_scaled =
            simd.select_f64s(rescale, simd.mul_f64s(current_value, down), current_value);
        // Rescaling is triggered above 2^-60 and multiplies by 2^-800, so a
        // finite rescaled current remains above 2^-860 and cannot underflow.
        // The previous value may become subnormal; retain that IEEE result.

        previous_vectors[index] = previous_scaled;
        current_vectors[index] = current_scaled;
        let updated_scale = simd.select_f64s(rescale, next_scale, scale_vectors[index]);
        scale_vectors[index] = updated_scale;
        let correction = simd.select_f64s(
            simd.less_than_f64s(updated_scale, zero_f64),
            zero_f64,
            simd.select_f64s(simd.less_than_f64s(updated_scale, one), one, up),
        );
        let updated_correction = simd.select_f64s(rescale, correction, correction_vectors[index]);
        correction_vectors[index] = updated_correction;
        let corrected_current = simd.mul_f64s(current_scaled, updated_correction);
        let current_magnitude = simd.abs_f64s(corrected_current);
        let current_is_finite =
            simd.less_than_or_equal_f64s(simd.abs_f64s(current_scaled), max_finite);
        let current_valid = simd.and_m64s(
            current_is_finite,
            simd.or_m64s(
                simd.equal_f64s(current_magnitude, zero_f64),
                simd.and_m64s(
                    simd.greater_than_or_equal_f64s(current_magnitude, min_normal),
                    simd.less_than_or_equal_f64s(current_magnitude, max_mantissa),
                ),
            ),
        );
        all_vectors_valid = simd.and_m64s(all_vectors_valid, current_valid);
        if CHECK_NONNEGATIVE {
            all_nonnegative_lanes = simd.and_m64s(
                all_nonnegative_lanes,
                simd.greater_than_or_equal_f64s(updated_scale, zero_f64),
            );
        }
    }

    for lane in 0..current_tail.len() {
        if !previous_tail[lane].is_finite() || !current_tail[lane].is_finite() {
            return None;
        }
        let rescale = current_tail[lane].abs() > ANALYSIS_FTOL;
        if !CHECK_NONNEGATIVE {
            any_rescaled |= rescale;
        }
        if rescale {
            let next_scale = scale_tail[lane] + 1.0;
            if !next_scale.is_finite()
                || next_scale == scale_tail[lane]
                || next_scale.abs() > 4.5e15
            {
                return None;
            }
            let previous_scaled = previous_tail[lane] * ANALYSIS_SCALE_DOWN;
            let current_scaled = current_tail[lane] * ANALYSIS_SCALE_DOWN;
            if current_tail[lane] != 0.0 && current_scaled.abs() < f64::MIN_POSITIVE {
                return None;
            }
            previous_tail[lane] = previous_scaled;
            current_tail[lane] = current_scaled;
            scale_tail[lane] = next_scale;
            correction_tail[lane] = analysis_scale_correction(current_scaled, next_scale)?;
        }
        let corrected_current = current_tail[lane] * correction_tail[lane];
        if !analysis_value_in_ieee_range(corrected_current) {
            return None;
        }
        if CHECK_NONNEGATIVE {
            all_nonnegative_tails &= scale_tail[lane] >= 0.0;
        }
    }
    if !analysis_mask_all(simd, all_vectors_valid) {
        return None;
    }
    let all_nonnegative = !CHECK_NONNEGATIVE
        || (analysis_mask_all(simd, all_nonnegative_lanes) && all_nonnegative_tails);
    Some((any_rescaled, all_nonnegative))
}

/// Converts a complete nonnegative scale-epoch block into ordinary recurrence
/// values when every lane remains inside the fast kernel's checked range.
/// Negative epochs continue in the scaled loop; unsupported magnitudes decline
/// without changing any lane.
#[inline(always)]
fn promote_analysis_pair_to_ordinary<S: Simd>(
    simd: S,
    previous: &mut [f64],
    current: &mut [f64],
    scale_epochs: &mut [f64],
    corrections: &mut [f64],
) -> Option<bool> {
    if previous.len() != current.len()
        || current.len() != scale_epochs.len()
        || current.len() != corrections.len()
    {
        return None;
    }
    let (scale_vectors, scale_tail) = S::as_mut_simd_f64s(scale_epochs);
    let (correction_vectors, correction_tail) = S::as_mut_simd_f64s(corrections);
    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(current);
    let zero = simd.splat_f64s(0.0);
    for index in 0..scale_vectors.len() {
        if !analysis_mask_all(
            simd,
            simd.greater_than_or_equal_f64s(scale_vectors[index], zero),
        ) {
            return Some(false);
        }
        let corrected_previous = simd.mul_f64s(previous_vectors[index], correction_vectors[index]);
        let corrected_current = simd.mul_f64s(current_vectors[index], correction_vectors[index]);
        let previous_magnitude = simd.abs_f64s(corrected_previous);
        let current_magnitude = simd.abs_f64s(corrected_current);
        let previous_valid = simd.or_m64s(
            simd.equal_f64s(previous_magnitude, zero),
            simd.and_m64s(
                simd.greater_than_or_equal_f64s(
                    previous_magnitude,
                    simd.splat_f64s(f64::MIN_POSITIVE),
                ),
                simd.less_than_or_equal_f64s(previous_magnitude, simd.splat_f64s(scalar::BIG)),
            ),
        );
        let current_valid = simd.or_m64s(
            simd.equal_f64s(current_magnitude, zero),
            simd.and_m64s(
                simd.greater_than_or_equal_f64s(
                    current_magnitude,
                    simd.splat_f64s(f64::MIN_POSITIVE),
                ),
                simd.less_than_or_equal_f64s(current_magnitude, simd.splat_f64s(scalar::BIG)),
            ),
        );
        if !analysis_mask_all(simd, simd.and_m64s(previous_valid, current_valid)) {
            return Some(false);
        }
    }
    if scale_tail
        .iter()
        .any(|&scale| scale < 0.0 || !scale.is_finite())
    {
        return Some(false);
    }
    for lane in 0..current_tail.len() {
        let previous_value = previous_tail[lane] * correction_tail[lane];
        let current_value = current_tail[lane] * correction_tail[lane];
        if !analysis_value_in_ieee_range(previous_value)
            || !analysis_value_in_ieee_range(current_value)
        {
            return Some(false);
        }
    }

    for index in 0..previous_vectors.len() {
        previous_vectors[index] = simd.mul_f64s(previous_vectors[index], correction_vectors[index]);
        current_vectors[index] = simd.mul_f64s(current_vectors[index], correction_vectors[index]);
        scale_vectors[index] = zero;
        correction_vectors[index] = simd.splat_f64s(1.0);
    }
    for lane in 0..previous_tail.len() {
        previous_tail[lane] *= correction_tail[lane];
        current_tail[lane] *= correction_tail[lane];
        scale_tail[lane] = 0.0;
        correction_tail[lane] = 1.0;
    }
    Some(true)
}

#[inline(always)]
fn zero_staged_analysis_pair(
    staged_coefficients: &mut [Complex64],
    m: usize,
    lmax: usize,
    pair_index: usize,
) {
    let degree = m + 2 * pair_index;
    let offset = degree - m;
    staged_coefficients[offset] = Complex64::new(0.0, 0.0);
    if degree < lmax {
        staged_coefficients[offset + 1] = Complex64::new(0.0, 0.0);
    }
}

struct FusedScaledScalarAnalysisPair<
    'a,
    const OPPOSITE_COSINE_WEIGHTED: bool,
    const CHECK_PAIR_GROUP_RANGE: bool = { !cfg!(rustsht_unchecked_analysis_pair_range) },
> {
    m: usize,
    lmax: usize,
    range_check_was_exercised: &'a mut bool,
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    staged_coefficients: &'a mut [Complex64],
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
    scale_epochs: &'a mut [f64],
    scale_corrections: &'a mut [f64],
}

/// Checked scaled path for analysis variants that cannot use insignificant
/// prefix advancement. Scaled lanes retain exact exponent-aware accumulation.
struct FusedCheckedScaledScalarAnalysisPair<
    'a,
    const OPPOSITE_COSINE_WEIGHTED: bool,
    const CHECK_PAIR_GROUP_RANGE: bool,
> {
    m: usize,
    lmax: usize,
    range_check_was_exercised: &'a mut bool,
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    staged_coefficients: &'a mut [Complex64],
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
    exponents: &'a mut [i64],
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool, const CHECK_PAIR_GROUP_RANGE: bool> WithSimd
    for FusedCheckedScaledScalarAnalysisPair<'_, OPPOSITE_COSINE_WEIGHTED, CHECK_PAIR_GROUP_RANGE>
{
    type Output = Result<bool, ShtError>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedCheckedScaledScalarAnalysisPair {
            m,
            lmax,
            range_check_was_exercised,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            previous,
            current,
            next,
            exponents,
        } = self;
        let mut previous = previous;
        let mut current = current;
        let mut next = next;
        let pair_count = (lmax - m + 2) / 2;
        let mut normal_range = false;
        for pair_index in 0..pair_count {
            if normal_range {
                return Ok(FusedScalarAnalysisPair::<
                    OPPOSITE_COSINE_WEIGHTED,
                    CHECK_PAIR_GROUP_RANGE,
                > {
                    m,
                    lmax,
                    range_check_was_exercised,
                    pair_start: pair_index,
                    cos_theta,
                    cos_theta_squared,
                    coefficient_a,
                    coefficient_b,
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    staged_coefficients,
                    second_channel: None,
                    previous,
                    current,
                    next,
                }
                .with_simd(simd));
            }
            let l = m + 2 * pair_index;
            let offset = l - m;
            let (same, opposite) = if pair_index + 1 < pair_count {
                FusedCheckedMixedAnalysisPairStep::<OPPOSITE_COSINE_WEIGHTED> {
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    basis_previous: previous,
                    basis_current: current,
                    exponents,
                    cos_theta,
                    cos_theta_squared,
                    coefficient_a: coefficient_a[pair_index],
                    coefficient_b: coefficient_b[pair_index],
                    basis_next: next,
                }
                .with_simd(simd)
            } else {
                MixedAnalysisPairAccumulation::<OPPOSITE_COSINE_WEIGHTED> {
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    basis: current,
                    exponents,
                    cos_theta,
                    allow_insignificant_scaled_lanes: false,
                }
                .with_simd(simd)
            };
            staged_coefficients[offset] = same;
            if l < lmax {
                staged_coefficients[offset + 1] = opposite;
            }
            if pair_index + 1 == pair_count {
                return Ok(true);
            }
            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
            let Some(ready) = scalar::normalize_pair_lanes_simd(simd, previous, current, exponents)
            else {
                return Ok(false);
            };
            normal_range = ready;
        }
        Ok(true)
    }
}

/// Checked scaled prefix for both spin-1 analysis channels, sharing one
/// recurrence and handing an ordinary suffix to the fused two-channel kernel.
struct FusedCheckedScaledAnalysisPairChannels<'a, const CHECK_PAIR_GROUP_RANGE: bool> {
    m: usize,
    lmax: usize,
    range_check_was_exercised: &'a mut bool,
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    same_real: [&'a [f64]; 2],
    same_imag: [&'a [f64]; 2],
    ordinary_opposite_real: [&'a [f64]; 2],
    ordinary_opposite_imag: [&'a [f64]; 2],
    staged_first: &'a mut [Complex64],
    staged_second: &'a mut [Complex64],
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
    exponents: &'a mut [i64],
}

#[allow(unexpected_cfgs)]
impl<const CHECK_PAIR_GROUP_RANGE: bool> WithSimd
    for FusedCheckedScaledAnalysisPairChannels<'_, CHECK_PAIR_GROUP_RANGE>
{
    type Output = Result<bool, ShtError>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedCheckedScaledAnalysisPairChannels {
            m,
            lmax,
            range_check_was_exercised,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            ordinary_opposite_real,
            ordinary_opposite_imag,
            staged_first,
            staged_second,
            previous,
            current,
            next,
            exponents,
        } = self;
        let mut previous = previous;
        let mut current = current;
        let mut next = next;
        let pair_count = (lmax - m + 2) / 2;
        let mut normal_range = false;

        for pair_index in 0..pair_count {
            if normal_range {
                let recurrence_succeeded = FusedScalarAnalysisPair::<
                    true,
                    CHECK_PAIR_GROUP_RANGE,
                    { cfg!(rustsht_fused_analysis_pair_range_check) },
                    true,
                    true,
                > {
                    m,
                    lmax,
                    range_check_was_exercised,
                    pair_start: pair_index,
                    cos_theta,
                    cos_theta_squared,
                    coefficient_a,
                    coefficient_b,
                    same_real: same_real[0],
                    same_imag: same_imag[0],
                    opposite_real: ordinary_opposite_real[0],
                    opposite_imag: ordinary_opposite_imag[0],
                    staged_coefficients: staged_first,
                    second_channel: Some(FusedScalarAnalysisPairSecondChannel {
                        same_real: same_real[1],
                        same_imag: same_imag[1],
                        opposite_real: ordinary_opposite_real[1],
                        opposite_imag: ordinary_opposite_imag[1],
                        staged_coefficients: staged_second,
                    }),
                    previous,
                    current,
                    next,
                }
                .with_simd(simd);
                return Ok(recurrence_succeeded);
            }

            let l = m + 2 * pair_index;
            let offset = l - m;
            if pair_index + 1 == pair_count {
                let channel_sums = MixedAnalysisPairChannelsAccumulation::<true> {
                    same_real,
                    same_imag,
                    opposite_real: ordinary_opposite_real,
                    opposite_imag: ordinary_opposite_imag,
                    basis: current,
                    exponents,
                    cos_theta,
                }
                .with_simd(simd);
                staged_first[offset] = channel_sums[0].0;
                staged_second[offset] = channel_sums[1].0;
                if l < lmax {
                    staged_first[offset + 1] = channel_sums[0].1;
                    staged_second[offset + 1] = channel_sums[1].1;
                }
                return Ok(true);
            }

            let Some((channel_sums, ready)) = FusedCheckedMixedAnalysisPairChannelsStep {
                same_real,
                same_imag,
                opposite_real: ordinary_opposite_real,
                opposite_imag: ordinary_opposite_imag,
                basis_previous: previous,
                basis_current: current,
                exponents,
                cos_theta,
                cos_theta_squared,
                coefficient_a: coefficient_a[pair_index],
                coefficient_b: coefficient_b[pair_index],
                basis_next: next,
            }
            .with_simd(simd) else {
                return Ok(false);
            };
            staged_first[offset] = channel_sums[0].0;
            staged_second[offset] = channel_sums[1].0;
            if l < lmax {
                staged_first[offset + 1] = channel_sums[0].1;
                staged_second[offset + 1] = channel_sums[1].1;
            }

            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
            normal_range = ready;
        }

        Ok(true)
    }
}

struct FusedCheckedMixedAnalysisPairChannelsStep<'a> {
    same_real: [&'a [f64]; 2],
    same_imag: [&'a [f64]; 2],
    opposite_real: [&'a [f64]; 2],
    opposite_imag: [&'a [f64]; 2],
    basis_previous: &'a [f64],
    basis_current: &'a mut [f64],
    exponents: &'a mut [i64],
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    coefficient_a: f64,
    coefficient_b: f64,
    basis_next: &'a mut [f64],
}

impl WithSimd for FusedCheckedMixedAnalysisPairChannelsStep<'_> {
    type Output = Option<([(Complex64, Complex64); 2], bool)>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedCheckedMixedAnalysisPairChannelsStep {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis_previous,
            basis_current,
            exponents,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            basis_next,
        } = self;
        if basis_previous.len() != basis_current.len()
            || basis_current.len() != exponents.len()
            || basis_current.len() != basis_next.len()
            || basis_current.len() != cos_theta.len()
            || basis_current.len() != cos_theta_squared.len()
        {
            return None;
        }
        let channel_sums = MixedAnalysisPairChannelsAccumulation::<true> {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis: basis_current,
            exponents,
            cos_theta,
        }
        .with_simd(simd);

        let (previous_vectors, previous_tail) = S::as_simd_f64s(basis_previous);
        let (current_vectors, current_tail) = S::as_mut_simd_f64s(basis_current);
        let (exponent_vectors, exponent_tail) = S::as_mut_simd_i64s(exponents);
        let (cosine_squared_vectors, cosine_squared_tail) = S::as_simd_f64s(cos_theta_squared);
        let (next_vectors, next_tail) = S::as_mut_simd_f64s(basis_next);
        debug_assert_eq!(previous_vectors.len(), current_vectors.len());
        debug_assert_eq!(current_vectors.len(), exponent_vectors.len());
        debug_assert_eq!(current_vectors.len(), cosine_squared_vectors.len());
        debug_assert_eq!(current_vectors.len(), next_vectors.len());
        debug_assert_eq!(previous_tail.len(), current_tail.len());
        debug_assert_eq!(current_tail.len(), exponent_tail.len());
        debug_assert_eq!(current_tail.len(), cosine_squared_tail.len());
        debug_assert_eq!(current_tail.len(), next_tail.len());

        let a = simd.splat_f64s(coefficient_a);
        let b = simd.splat_f64s(coefficient_b);
        let mut all_normal = true;
        for index in 0..current_vectors.len() {
            let leading = simd.mul_add_f64s(a, cosine_squared_vectors[index], b);
            let mut previous_value = current_vectors[index];
            let mut current_value =
                simd.mul_add_f64s(leading, previous_value, previous_vectors[index]);
            let mut exponent = exponent_vectors[index];
            all_normal &= scalar::normalize_pair_vector_simd(
                simd,
                &mut previous_value,
                &mut current_value,
                &mut exponent,
            )?;
            current_vectors[index] = previous_value;
            next_vectors[index] = current_value;
            exponent_vectors[index] = exponent;
        }
        for lane in 0..current_tail.len() {
            let leading = coefficient_a.mul_add(cosine_squared_tail[lane], coefficient_b);
            let next_value = leading.mul_add(current_tail[lane], previous_tail[lane]);
            if !current_tail[lane].is_finite() || !next_value.is_finite() {
                return None;
            }
            next_tail[lane] = next_value;
            let normalized = scalar::normalize_pair_lanes(
                &mut current_tail[lane..lane + 1],
                &mut next_tail[lane..lane + 1],
                &mut exponent_tail[lane..lane + 1],
            )
            .ok()?;
            all_normal &= normalized;
        }
        Some((channel_sums, all_normal))
    }
}

struct FusedCheckedMixedAnalysisPairStep<'a, const OPPOSITE_COSINE_WEIGHTED: bool> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis_previous: &'a [f64],
    basis_current: &'a [f64],
    exponents: &'a [i64],
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    coefficient_a: f64,
    coefficient_b: f64,
    basis_next: &'a mut [f64],
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool> WithSimd
    for FusedCheckedMixedAnalysisPairStep<'_, OPPOSITE_COSINE_WEIGHTED>
{
    type Output = (Complex64, Complex64);

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedCheckedMixedAnalysisPairStep {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis_previous,
            basis_current,
            exponents,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            basis_next,
        } = self;
        let (same_real_vectors, same_real_tail) = S::as_simd_f64s(same_real);
        let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(same_imag);
        let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(opposite_real);
        let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(opposite_imag);
        let (previous_vectors, previous_tail) = S::as_simd_f64s(basis_previous);
        let (current_vectors, current_tail) = S::as_simd_f64s(basis_current);
        let (exponent_vectors, exponent_tail) = S::as_simd_i64s(exponents);
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
        let (cosine_squared_vectors, cosine_squared_tail) = S::as_simd_f64s(cos_theta_squared);
        let (next_vectors, next_tail) = S::as_mut_simd_f64s(basis_next);
        let zero = simd.splat_f64s(0.0);
        let zero_exponent = simd.splat_i64s(0);
        let minimum = simd.splat_f64s(f64::MIN_POSITIVE);
        let maximum = simd.splat_f64s(scalar::BIG);
        let a = simd.splat_f64s(coefficient_a);
        let b = simd.splat_f64s(coefficient_b);
        let mut same_re = zero;
        let mut same_im = zero;
        let mut opposite_re = zero;
        let mut opposite_im = zero;
        let mut vector_needs_scalar = false;
        for index in 0..current_vectors.len() {
            let magnitude = simd.abs_f64s(current_vectors[index]);
            let is_zero = simd.equal_f64s(magnitude, zero);
            let is_normal = simd.and_m64s(
                simd.greater_than_or_equal_f64s(magnitude, minimum),
                simd.less_than_or_equal_f64s(magnitude, maximum),
            );
            let exponent_zero = simd.equal_i64s(exponent_vectors[index], zero_exponent);
            let normal = simd.and_m64s(exponent_zero, simd.or_m64s(is_zero, is_normal));
            vector_needs_scalar |= !analysis_mask_all(simd, normal);
            let basis = simd.select_f64s(normal, current_vectors[index], zero);
            same_re = simd.mul_add_f64s(
                simd.select_f64s(normal, same_real_vectors[index], zero),
                basis,
                same_re,
            );
            same_im = simd.mul_add_f64s(
                simd.select_f64s(normal, same_imag_vectors[index], zero),
                basis,
                same_im,
            );
            let opposite_re_coeff = simd.select_f64s(normal, opposite_real_vectors[index], zero);
            let opposite_im_coeff = simd.select_f64s(normal, opposite_imag_vectors[index], zero);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_re = simd.mul_add_f64s(opposite_re_coeff, basis, opposite_re);
                opposite_im = simd.mul_add_f64s(opposite_im_coeff, basis, opposite_im);
            } else {
                opposite_re = simd.mul_add_f64s(
                    simd.mul_f64s(opposite_re_coeff, cosine_vectors[index]),
                    basis,
                    opposite_re,
                );
                opposite_im = simd.mul_add_f64s(
                    simd.mul_f64s(opposite_im_coeff, cosine_vectors[index]),
                    basis,
                    opposite_im,
                );
            }
            let leading = simd.mul_add_f64s(a, cosine_squared_vectors[index], b);
            next_vectors[index] =
                simd.mul_add_f64s(leading, current_vectors[index], previous_vectors[index]);
        }
        let mut same = Complex64::new(simd.reduce_sum_f64s(same_re), simd.reduce_sum_f64s(same_im));
        let mut opposite = Complex64::new(
            simd.reduce_sum_f64s(opposite_re),
            simd.reduce_sum_f64s(opposite_im),
        );
        if vector_needs_scalar {
            let vector_lane_count = basis_current.len() - current_tail.len();
            for lane in 0..vector_lane_count {
                let magnitude = basis_current[lane].abs();
                let normal = exponents[lane] == 0
                    && (magnitude == 0.0 || (f64::MIN_POSITIVE..=scalar::BIG).contains(&magnitude));
                if !normal {
                    same += scalar::multiply_complex_scaled(
                        Complex64::new(same_real[lane], same_imag[lane]),
                        basis_current[lane],
                        exponents[lane],
                    );
                    let coefficient = Complex64::new(opposite_real[lane], opposite_imag[lane]);
                    opposite += if OPPOSITE_COSINE_WEIGHTED {
                        scalar::multiply_complex_scaled(
                            coefficient,
                            basis_current[lane],
                            exponents[lane],
                        )
                    } else {
                        scalar::multiply_complex_scaled_product(
                            coefficient,
                            basis_current[lane],
                            cos_theta[lane],
                            exponents[lane],
                        )
                    };
                }
            }
        }
        for lane in 0..current_tail.len() {
            let magnitude = current_tail[lane].abs();
            if exponent_tail[lane] == 0
                && (magnitude == 0.0 || (f64::MIN_POSITIVE..=scalar::BIG).contains(&magnitude))
            {
                same.re = same_real_tail[lane].mul_add(current_tail[lane], same.re);
                same.im = same_imag_tail[lane].mul_add(current_tail[lane], same.im);
                if OPPOSITE_COSINE_WEIGHTED {
                    opposite.re = opposite_real_tail[lane].mul_add(current_tail[lane], opposite.re);
                    opposite.im = opposite_imag_tail[lane].mul_add(current_tail[lane], opposite.im);
                } else {
                    opposite.re = (opposite_real_tail[lane] * cosine_tail[lane])
                        .mul_add(current_tail[lane], opposite.re);
                    opposite.im = (opposite_imag_tail[lane] * cosine_tail[lane])
                        .mul_add(current_tail[lane], opposite.im);
                }
            } else {
                let exponent = exponent_tail[lane];
                same += scalar::multiply_complex_scaled(
                    Complex64::new(same_real_tail[lane], same_imag_tail[lane]),
                    current_tail[lane],
                    exponent,
                );
                let coefficient =
                    Complex64::new(opposite_real_tail[lane], opposite_imag_tail[lane]);
                opposite += if OPPOSITE_COSINE_WEIGHTED {
                    scalar::multiply_complex_scaled(coefficient, current_tail[lane], exponent)
                } else {
                    scalar::multiply_complex_scaled_product(
                        coefficient,
                        current_tail[lane],
                        cosine_tail[lane],
                        exponent,
                    )
                };
            }
            let leading = coefficient_a.mul_add(cosine_squared_tail[lane], coefficient_b);
            next_tail[lane] = leading.mul_add(current_tail[lane], previous_tail[lane]);
        }
        (same, opposite)
    }
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool, const CHECK_PAIR_GROUP_RANGE: bool> WithSimd
    for FusedScaledScalarAnalysisPair<'_, OPPOSITE_COSINE_WEIGHTED, CHECK_PAIR_GROUP_RANGE>
{
    type Output = Result<bool, ShtError>;

    #[allow(unexpected_cfgs)]
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedScaledScalarAnalysisPair {
            m,
            lmax,
            range_check_was_exercised,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            previous,
            current,
            next,
            scale_epochs,
            scale_corrections,
        } = self;
        let mut previous = previous;
        let mut current = current;
        let mut next = next;
        let pair_count = (lmax - m + 2) / 2;
        let mut pair_start = 0;
        if !analysis_scale_epochs_allow_recurrence(scale_epochs, pair_count) {
            return Ok(false);
        }
        let Some((mut below_limit, mut skip_insignificant_validation)) =
            prepare_analysis_scale_corrections(
                current,
                scale_epochs,
                &mut scale_corrections[..current.len()],
            )
        else {
            return Ok(false);
        };
        while below_limit && pair_start < pair_count {
            zero_staged_analysis_pair(staged_coefficients, m, lmax, pair_start);
            if pair_count - pair_start == 1 {
                return Ok(analysis_recurrence_state_is_finite(previous, current));
            }

            let first_pair = pair_start;
            let second_pair = first_pair + 1;
            zero_staged_analysis_pair(staged_coefficients, m, lmax, second_pair);
            if pair_count - second_pair == 1 {
                return Ok(analysis_recurrence_state_is_finite(previous, current));
            }
            let Some((remains_below_limit, remains_insignificant_prefix)) =
                advance_and_rescale_analysis_pair(
                    simd,
                    [
                        (coefficient_a[first_pair], coefficient_b[first_pair]),
                        (coefficient_a[second_pair], coefficient_b[second_pair]),
                    ],
                    cos_theta_squared,
                    (&mut *previous, &mut *current),
                    scale_epochs,
                    scale_corrections,
                    skip_insignificant_validation,
                )
            else {
                return Ok(false);
            };
            pair_start += 2;
            below_limit = remains_below_limit;
            skip_insignificant_validation = remains_insignificant_prefix;
        }
        if pair_start == pair_count {
            return Ok(analysis_recurrence_state_is_finite(previous, current));
        }

        let initial_normal_range = promote_analysis_pair_to_ordinary(
            simd,
            previous,
            current,
            scale_epochs,
            &mut scale_corrections[..current.len()],
        );
        let Some(mut normal_range) = initial_normal_range else {
            return Ok(false);
        };

        for pair_index in pair_start..pair_count {
            if normal_range {
                return Ok(FusedScalarAnalysisPair::<
                    OPPOSITE_COSINE_WEIGHTED,
                    CHECK_PAIR_GROUP_RANGE,
                > {
                    m,
                    lmax,
                    range_check_was_exercised,
                    pair_start: pair_index,
                    cos_theta,
                    cos_theta_squared,
                    coefficient_a,
                    coefficient_b,
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    staged_coefficients,
                    second_channel: None,
                    previous,
                    current,
                    next,
                }
                .with_simd(simd));
            }
            let l = m + 2 * pair_index;
            let offset = l - m;
            let (same_sum, opposite_sum) = if pair_index + 1 < pair_count {
                FusedMixedAnalysisPairStep::<OPPOSITE_COSINE_WEIGHTED> {
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    basis_previous: previous,
                    basis_current: current,
                    cos_theta,
                    cos_theta_squared,
                    scale_corrections: &scale_corrections[..current.len()],
                    coefficient_a: coefficient_a[pair_index],
                    coefficient_b: coefficient_b[pair_index],
                    basis_next: next,
                }
                .with_simd(simd)
            } else {
                FusedMixedAnalysisPairAccumulation::<OPPOSITE_COSINE_WEIGHTED> {
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    basis_current: current,
                    cos_theta,
                    scale_corrections: &scale_corrections[..current.len()],
                }
                .with_simd(simd)
            };
            staged_coefficients[offset] = same_sum;
            if l < lmax {
                staged_coefficients[offset + 1] = opposite_sum;
            }
            if pair_index + 1 == pair_count {
                return Ok(true);
            }

            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
            let Some((_, all_nonnegative)) = rescale_scaled_analysis_pair::<S, true>(
                simd,
                previous,
                current,
                scale_epochs,
                &mut scale_corrections[..current.len()],
            ) else {
                return Ok(false);
            };
            normal_range = if all_nonnegative {
                match promote_analysis_pair_to_ordinary(
                    simd,
                    previous,
                    current,
                    scale_epochs,
                    &mut scale_corrections[..current.len()],
                ) {
                    Some(ready) => ready,
                    None => return Ok(false),
                }
            } else {
                false
            };
        }
        Ok(true)
    }
}

#[allow(unexpected_cfgs)]
struct FusedScalarAnalysisPair<
    'a,
    const OPPOSITE_COSINE_WEIGHTED: bool,
    // The custom cfg supports whole-transform A/B builds. Production also
    // selects the unchecked specialization after a cached checked validation.
    const CHECK_PAIR_GROUP_RANGE: bool = { !cfg!(rustsht_unchecked_analysis_pair_range) },
    // Experimental checked mode folds the group-state reduction into the
    // recurrence traversal instead of rereading the resulting state arrays.
    const FUSE_PAIR_GROUP_RANGE_CHECK: bool = { cfg!(rustsht_fused_analysis_pair_range_check) },
    // Test-only source attribution compiles the same loop without coefficient
    // products; the production specialization always keeps accumulation.
    const ACCUMULATE: bool = true,
    // Optional second channel is reduced from basis states saved by the first
    // channel's fused recurrence pass.
    const SEQUENTIAL_SECOND_CHANNEL: bool = false,
> {
    m: usize,
    lmax: usize,
    range_check_was_exercised: &'a mut bool,
    pair_start: usize,
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    staged_coefficients: &'a mut [Complex64],
    second_channel: Option<FusedScalarAnalysisPairSecondChannel<'a>>,
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
}

struct FusedScalarAnalysisPairSecondChannel<'a> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    staged_coefficients: &'a mut [Complex64],
}

struct FusedAnalysisPairStep<'a, const OPPOSITE_COSINE_WEIGHTED: bool> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis_previous: &'a [f64],
    basis_current: &'a [f64],
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    coefficient_a: f64,
    coefficient_b: f64,
    basis_next: &'a mut [f64],
}

struct FusedAnalysisPairGroup {
    first_same: Complex64,
    first_opposite: Complex64,
    second_same: Complex64,
    second_opposite: Complex64,
    states_in_range: bool,
    tails_in_range: bool,
}

struct FusedAnalysisTwoPairSteps<'a, const OPPOSITE_COSINE_WEIGHTED: bool> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis_previous: &'a mut [f64],
    basis_current: &'a mut [f64],
    basis_rotation_scratch: &'a mut [f64],
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    first_coefficient_a: f64,
    first_coefficient_b: f64,
    second_coefficient_a: f64,
    second_coefficient_b: f64,
}

/// Reduces the real and imaginary lanes of one complex spectrum independently.
#[inline(always)]
fn reduce_analysis_complex_pair<S: Simd>(simd: S, real: S::f64s, imag: S::f64s) -> Complex64 {
    Complex64::new(simd.reduce_sum_f64s(real), simd.reduce_sum_f64s(imag))
}

#[inline(always)]
fn fused_analysis_pair_step_with_simd<S: Simd, const OPPOSITE_COSINE_WEIGHTED: bool>(
    simd: S,
    step: FusedAnalysisPairStep<'_, OPPOSITE_COSINE_WEIGHTED>,
) -> (Complex64, Complex64, S::m64s, bool) {
    let FusedAnalysisPairStep {
        same_real,
        same_imag,
        opposite_real,
        opposite_imag,
        basis_previous,
        basis_current,
        cos_theta,
        cos_theta_squared,
        coefficient_a,
        coefficient_b,
        basis_next,
    } = step;
    let (same_real_vectors, same_real_tail) = S::as_simd_f64s(same_real);
    let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(same_imag);
    let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(opposite_real);
    let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(opposite_imag);
    let (previous_vectors, previous_tail) = S::as_simd_f64s(basis_previous);
    let (current_vectors, current_tail) = S::as_simd_f64s(basis_current);
    let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
    let (cosine_squared_vectors, cosine_squared_tail) = S::as_simd_f64s(cos_theta_squared);
    let (next_vectors, next_tail) = S::as_mut_simd_f64s(basis_next);
    debug_assert_eq!(same_real_vectors.len(), current_vectors.len());
    debug_assert_eq!(same_imag_vectors.len(), current_vectors.len());
    debug_assert_eq!(opposite_real_vectors.len(), current_vectors.len());
    debug_assert_eq!(opposite_imag_vectors.len(), current_vectors.len());
    debug_assert_eq!(previous_vectors.len(), current_vectors.len());
    debug_assert_eq!(cosine_vectors.len(), current_vectors.len());
    debug_assert_eq!(cosine_squared_vectors.len(), current_vectors.len());
    debug_assert_eq!(next_vectors.len(), current_vectors.len());
    debug_assert_eq!(same_real_tail.len(), current_tail.len());
    debug_assert_eq!(same_imag_tail.len(), current_tail.len());
    debug_assert_eq!(opposite_real_tail.len(), current_tail.len());
    debug_assert_eq!(opposite_imag_tail.len(), current_tail.len());
    debug_assert_eq!(previous_tail.len(), current_tail.len());
    debug_assert_eq!(cosine_tail.len(), current_tail.len());
    debug_assert_eq!(cosine_squared_tail.len(), current_tail.len());
    debug_assert_eq!(next_tail.len(), current_tail.len());

    let zero = simd.splat_f64s(0.0);
    let minimum = simd.splat_f64s(f64::MIN_POSITIVE);
    let a = simd.splat_f64s(coefficient_a);
    let b = simd.splat_f64s(coefficient_b);
    let mut same_re = zero;
    let mut same_im = zero;
    let mut opposite_re = zero;
    let mut opposite_im = zero;
    let mut all_vectors_in_range = simd.equal_f64s(zero, zero);
    let mut all_tails_in_range = true;

    // The recurrence is normalized scalar spherical-harmonic synthesis.
    // By the addition theorem, |Y_lm| <= sqrt((2l+1)/(4*pi)); for every
    // representable usize degree this is far below scalar::BIG (2^400).
    // fill_factors rejects nonfinite or unbounded recurrence factors, so the
    // per-degree upper comparison is redundant. Keep the zero/subnormal test:
    // those values still require the checked scaled path.
    // Each vector contributes to the current coefficient while the same
    // pass advances and range-checks its recurrence state.
    for (
        (
            ((((((same_real, same_imag), opposite_real), opposite_imag), previous), basis), cosine),
            cosine_squared,
        ),
        next,
    ) in same_real_vectors
        .iter()
        .zip(same_imag_vectors.iter())
        .zip(opposite_real_vectors.iter())
        .zip(opposite_imag_vectors.iter())
        .zip(previous_vectors.iter())
        .zip(current_vectors.iter())
        .zip(cosine_vectors.iter())
        .zip(cosine_squared_vectors.iter())
        .zip(next_vectors.iter_mut())
    {
        let basis = *basis;
        same_re = simd.mul_add_f64s(*same_real, basis, same_re);
        same_im = simd.mul_add_f64s(*same_imag, basis, same_im);
        if OPPOSITE_COSINE_WEIGHTED {
            opposite_re = simd.mul_add_f64s(*opposite_real, basis, opposite_re);
        } else {
            opposite_re = simd.add_f64s(
                opposite_re,
                simd.mul_f64s(simd.mul_f64s(*opposite_real, basis), *cosine),
            );
        }
        if OPPOSITE_COSINE_WEIGHTED {
            opposite_im = simd.mul_add_f64s(*opposite_imag, basis, opposite_im);
        } else {
            opposite_im = simd.add_f64s(
                opposite_im,
                simd.mul_f64s(simd.mul_f64s(*opposite_imag, basis), *cosine),
            );
        }

        let leading = simd.mul_add_f64s(a, *cosine_squared, b);
        let next_value = simd.mul_add_f64s(leading, basis, *previous);
        *next = next_value;
        let magnitude = simd.abs_f64s(next_value);
        let is_zero = simd.equal_f64s(magnitude, zero);
        let above_minimum = simd.greater_than_or_equal_f64s(magnitude, minimum);
        let in_range = simd.or_m64s(is_zero, above_minimum);
        all_vectors_in_range = simd.and_m64s(all_vectors_in_range, in_range);
    }

    let mut same_re_sum = simd.reduce_sum_f64s(same_re);
    let mut same_im_sum = simd.reduce_sum_f64s(same_im);
    let mut opposite_re_sum = simd.reduce_sum_f64s(opposite_re);
    let mut opposite_im_sum = simd.reduce_sum_f64s(opposite_im);
    for (
        (
            ((((((same_real, same_imag), opposite_real), opposite_imag), previous), basis), cosine),
            cosine_squared,
        ),
        next,
    ) in same_real_tail
        .iter()
        .zip(same_imag_tail.iter())
        .zip(opposite_real_tail.iter())
        .zip(opposite_imag_tail.iter())
        .zip(previous_tail.iter())
        .zip(current_tail.iter())
        .zip(cosine_tail.iter())
        .zip(cosine_squared_tail.iter())
        .zip(next_tail.iter_mut())
    {
        let basis = *basis;
        let cosine = *cosine;
        same_re_sum = (*same_real).mul_add(basis, same_re_sum);
        same_im_sum = (*same_imag).mul_add(basis, same_im_sum);
        if OPPOSITE_COSINE_WEIGHTED {
            opposite_re_sum = (*opposite_real).mul_add(basis, opposite_re_sum);
            opposite_im_sum = (*opposite_imag).mul_add(basis, opposite_im_sum);
        } else {
            opposite_re_sum += (*opposite_real * basis) * cosine;
            opposite_im_sum += (*opposite_imag * basis) * cosine;
        }

        let leading = coefficient_a.mul_add(*cosine_squared, coefficient_b);
        let next_value = leading.mul_add(basis, *previous);
        *next = next_value;
        let magnitude = next_value.abs();
        all_tails_in_range &= magnitude == 0.0 || magnitude >= f64::MIN_POSITIVE;
    }

    (
        Complex64::new(same_re_sum, same_im_sum),
        Complex64::new(opposite_re_sum, opposite_im_sum),
        all_vectors_in_range,
        all_tails_in_range,
    )
}

#[allow(unexpected_cfgs)]
#[inline(always)]
fn fused_analysis_two_pair_steps_with_simd<
    S: Simd,
    const OPPOSITE_COSINE_WEIGHTED: bool,
    const CHECK_GROUP_RANGE: bool,
    const TRACK_STATE_RANGE: bool,
    const ACCUMULATE: bool,
    const ROTATE_SEQUENTIAL_STATES: bool,
>(
    simd: S,
    step: FusedAnalysisTwoPairSteps<'_, OPPOSITE_COSINE_WEIGHTED>,
) -> Option<FusedAnalysisPairGroup> {
    let FusedAnalysisTwoPairSteps {
        same_real,
        same_imag,
        opposite_real,
        opposite_imag,
        basis_previous,
        basis_current,
        basis_rotation_scratch,
        cos_theta,
        cos_theta_squared,
        first_coefficient_a,
        first_coefficient_b,
        second_coefficient_a,
        second_coefficient_b,
    } = step;
    if same_real.len() != basis_current.len()
        || same_imag.len() != basis_current.len()
        || opposite_real.len() != basis_current.len()
        || opposite_imag.len() != basis_current.len()
        || basis_previous.len() != basis_current.len()
        || cos_theta.len() != basis_current.len()
        || cos_theta_squared.len() != basis_current.len()
    {
        return None;
    }

    let (same_real_vectors, same_real_tail) = S::as_simd_f64s(same_real);
    let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(same_imag);
    let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(opposite_real);
    let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(opposite_imag);
    let (previous_vectors, previous_tail) = S::as_mut_simd_f64s(basis_previous);
    let (current_vectors, current_tail) = S::as_mut_simd_f64s(basis_current);
    let (snapshot_vectors, snapshot_tail) = S::as_mut_simd_f64s(basis_rotation_scratch);
    let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
    let (cosine_squared_vectors, cosine_squared_tail) = S::as_simd_f64s(cos_theta_squared);
    if same_real_vectors.len() != current_vectors.len()
        || same_imag_vectors.len() != current_vectors.len()
        || opposite_real_vectors.len() != current_vectors.len()
        || opposite_imag_vectors.len() != current_vectors.len()
        || previous_vectors.len() != current_vectors.len()
        || (ROTATE_SEQUENTIAL_STATES && snapshot_vectors.len() != current_vectors.len())
        || (!ROTATE_SEQUENTIAL_STATES && !snapshot_vectors.is_empty())
        || cosine_vectors.len() != current_vectors.len()
        || cosine_squared_vectors.len() != current_vectors.len()
        || same_real_tail.len() != current_tail.len()
        || same_imag_tail.len() != current_tail.len()
        || opposite_real_tail.len() != current_tail.len()
        || opposite_imag_tail.len() != current_tail.len()
        || previous_tail.len() != current_tail.len()
        || (ROTATE_SEQUENTIAL_STATES && snapshot_tail.len() != current_tail.len())
        || (!ROTATE_SEQUENTIAL_STATES && !snapshot_tail.is_empty())
        || cosine_tail.len() != current_tail.len()
        || cosine_squared_tail.len() != current_tail.len()
    {
        return None;
    }

    let zero = simd.splat_f64s(0.0);
    // The experiment re-materializes coefficient splats at the recurrence
    // expressions; the default keeps the established broadcast lifetimes.
    #[cfg(not(rustsht_rematerialize_analysis_coefficients))]
    let first_a = simd.splat_f64s(first_coefficient_a);
    #[cfg(not(rustsht_rematerialize_analysis_coefficients))]
    let first_b = simd.splat_f64s(first_coefficient_b);
    #[cfg(not(rustsht_rematerialize_analysis_coefficients))]
    let second_a = simd.splat_f64s(second_coefficient_a);
    #[cfg(not(rustsht_rematerialize_analysis_coefficients))]
    let second_b = simd.splat_f64s(second_coefficient_b);
    let minimum = simd.splat_f64s(f64::MIN_POSITIVE);
    let mut first_same_re = zero;
    let mut first_same_im = zero;
    let mut first_opposite_re = zero;
    let mut first_opposite_im = zero;
    let mut second_same_re = zero;
    let mut second_same_im = zero;
    let mut second_opposite_re = zero;
    let mut second_opposite_im = zero;
    let mut vector_states_in_range = simd.equal_f64s(zero, zero);
    let mut scalar_states_in_range = true;
    let mut tails_in_range = true;

    macro_rules! update_analysis_pair_vector {
        ($previous:expr, $current:expr, $same_real:expr, $same_imag:expr,
         $opposite_real:expr, $opposite_imag:expr, $cosine:expr,
         $cosine_squared:expr, $snapshot:expr) => {{
            let previous_value = *($previous);
            let current_value = *($current);
            if ACCUMULATE {
                first_same_re = simd.mul_add_f64s(*($same_real), current_value, first_same_re);
                first_same_im = simd.mul_add_f64s(*($same_imag), current_value, first_same_im);
                if OPPOSITE_COSINE_WEIGHTED {
                    first_opposite_re =
                        simd.mul_add_f64s(*($opposite_real), current_value, first_opposite_re);
                    first_opposite_im =
                        simd.mul_add_f64s(*($opposite_imag), current_value, first_opposite_im);
                } else {
                    first_opposite_re = simd.add_f64s(
                        first_opposite_re,
                        simd.mul_f64s(simd.mul_f64s(*($opposite_real), current_value), *($cosine)),
                    );
                    first_opposite_im = simd.add_f64s(
                        first_opposite_im,
                        simd.mul_f64s(simd.mul_f64s(*($opposite_imag), current_value), *($cosine)),
                    );
                }
            }

            #[cfg(rustsht_rematerialize_analysis_coefficients)]
            let first_leading = simd.mul_add_f64s(
                simd.splat_f64s(first_coefficient_a),
                *($cosine_squared),
                simd.splat_f64s(first_coefficient_b),
            );
            #[cfg(not(rustsht_rematerialize_analysis_coefficients))]
            let first_leading = simd.mul_add_f64s(first_a, *($cosine_squared), first_b);
            let first_next = simd.mul_add_f64s(first_leading, current_value, previous_value);

            if ACCUMULATE {
                second_same_re = simd.mul_add_f64s(*($same_real), first_next, second_same_re);
                second_same_im = simd.mul_add_f64s(*($same_imag), first_next, second_same_im);
                if OPPOSITE_COSINE_WEIGHTED {
                    second_opposite_re =
                        simd.mul_add_f64s(*($opposite_real), first_next, second_opposite_re);
                    second_opposite_im =
                        simd.mul_add_f64s(*($opposite_imag), first_next, second_opposite_im);
                } else {
                    second_opposite_re = simd.add_f64s(
                        second_opposite_re,
                        simd.mul_f64s(simd.mul_f64s(*($opposite_real), first_next), *($cosine)),
                    );
                    second_opposite_im = simd.add_f64s(
                        second_opposite_im,
                        simd.mul_f64s(simd.mul_f64s(*($opposite_imag), first_next), *($cosine)),
                    );
                }
            }

            #[cfg(rustsht_rematerialize_analysis_coefficients)]
            let second_leading = simd.mul_add_f64s(
                simd.splat_f64s(second_coefficient_a),
                *($cosine_squared),
                simd.splat_f64s(second_coefficient_b),
            );
            #[cfg(not(rustsht_rematerialize_analysis_coefficients))]
            let second_leading = simd.mul_add_f64s(second_a, *($cosine_squared), second_b);
            let second_next = simd.mul_add_f64s(second_leading, first_next, current_value);
            if CHECK_GROUP_RANGE && TRACK_STATE_RANGE {
                let previous_magnitude = simd.abs_f64s(first_next);
                let current_magnitude = simd.abs_f64s(second_next);
                let previous_in_range = simd.or_m64s(
                    simd.equal_f64s(previous_magnitude, zero),
                    simd.greater_than_or_equal_f64s(previous_magnitude, minimum),
                );
                let current_in_range = simd.or_m64s(
                    simd.equal_f64s(current_magnitude, zero),
                    simd.greater_than_or_equal_f64s(current_magnitude, minimum),
                );
                let states_in_range = simd.and_m64s(previous_in_range, current_in_range);
                // The scalar-reduction candidate shortens the vector-mask live
                // range at the cost of one horizontal reduction per vector chunk.
                if cfg!(rustsht_scalar_fused_analysis_pair_range_check) {
                    scalar_states_in_range &= analysis_mask_all(simd, states_in_range);
                } else {
                    vector_states_in_range = simd.and_m64s(vector_states_in_range, states_in_range);
                }
            }
            if ROTATE_SEQUENTIAL_STATES {
                *($snapshot) = first_next;
                *($previous) = second_next;
            } else {
                *($previous) = first_next;
                *($current) = second_next;
            }
        }};
    }

    macro_rules! pair_vector_inputs {
        () => {
            previous_vectors
                .iter_mut()
                .zip(current_vectors.iter_mut())
                .zip(same_real_vectors.iter())
                .zip(same_imag_vectors.iter())
                .zip(opposite_real_vectors.iter())
                .zip(opposite_imag_vectors.iter())
                .zip(cosine_vectors.iter())
                .zip(cosine_squared_vectors.iter())
        };
    }

    macro_rules! update_pair_vector {
        ($values:expr, $snapshot:expr) => {{
            let (
                (
                    (((((previous, current), same_real), same_imag), opposite_real), opposite_imag),
                    cosine,
                ),
                cosine_squared,
            ) = $values;
            update_analysis_pair_vector!(
                previous,
                current,
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                cosine,
                cosine_squared,
                $snapshot
            );
        }};
    }

    if ROTATE_SEQUENTIAL_STATES {
        for (values, snapshot) in pair_vector_inputs!().zip(snapshot_vectors.iter_mut()) {
            update_pair_vector!(values, snapshot);
        }
    } else {
        for values in pair_vector_inputs!() {
            let mut unused_snapshot = zero;
            update_pair_vector!(values, &mut unused_snapshot);
        }
    }

    let (first_same, first_opposite, second_same, second_opposite) = if ACCUMULATE {
        (
            reduce_analysis_complex_pair(simd, first_same_re, first_same_im),
            reduce_analysis_complex_pair(simd, first_opposite_re, first_opposite_im),
            reduce_analysis_complex_pair(simd, second_same_re, second_same_im),
            reduce_analysis_complex_pair(simd, second_opposite_re, second_opposite_im),
        )
    } else {
        let zero = Complex64::new(0.0, 0.0);
        (zero, zero, zero, zero)
    };
    let mut first_same_re_sum = first_same.re;
    let mut first_same_im_sum = first_same.im;
    let mut first_opposite_re_sum = first_opposite.re;
    let mut first_opposite_im_sum = first_opposite.im;
    let mut second_same_re_sum = second_same.re;
    let mut second_same_im_sum = second_same.im;
    let mut second_opposite_re_sum = second_opposite.re;
    let mut second_opposite_im_sum = second_opposite.im;

    let mut snapshot_tail = snapshot_tail.iter_mut();
    for (
        ((((((previous, current), same_real), same_imag), opposite_real), opposite_imag), cosine),
        cosine_squared,
    ) in previous_tail
        .iter_mut()
        .zip(current_tail.iter_mut())
        .zip(same_real_tail.iter())
        .zip(same_imag_tail.iter())
        .zip(opposite_real_tail.iter())
        .zip(opposite_imag_tail.iter())
        .zip(cosine_tail.iter())
        .zip(cosine_squared_tail.iter())
    {
        let previous_value = *previous;
        let current_value = *current;
        if ACCUMULATE {
            first_same_re_sum = (*same_real).mul_add(current_value, first_same_re_sum);
            first_same_im_sum = (*same_imag).mul_add(current_value, first_same_im_sum);
            if OPPOSITE_COSINE_WEIGHTED {
                first_opposite_re_sum =
                    (*opposite_real).mul_add(current_value, first_opposite_re_sum);
                first_opposite_im_sum =
                    (*opposite_imag).mul_add(current_value, first_opposite_im_sum);
            } else {
                first_opposite_re_sum += (*opposite_real * current_value) * *cosine;
                first_opposite_im_sum += (*opposite_imag * current_value) * *cosine;
            }
        }

        let first_next = first_coefficient_a
            .mul_add(*cosine_squared, first_coefficient_b)
            .mul_add(current_value, previous_value);
        let first_magnitude = first_next.abs();
        tails_in_range &= first_magnitude == 0.0 || first_magnitude >= f64::MIN_POSITIVE;

        if ACCUMULATE {
            second_same_re_sum = (*same_real).mul_add(first_next, second_same_re_sum);
            second_same_im_sum = (*same_imag).mul_add(first_next, second_same_im_sum);
            if OPPOSITE_COSINE_WEIGHTED {
                second_opposite_re_sum =
                    (*opposite_real).mul_add(first_next, second_opposite_re_sum);
                second_opposite_im_sum =
                    (*opposite_imag).mul_add(first_next, second_opposite_im_sum);
            } else {
                second_opposite_re_sum += (*opposite_real * first_next) * *cosine;
                second_opposite_im_sum += (*opposite_imag * first_next) * *cosine;
            }
        }

        let second_next = second_coefficient_a
            .mul_add(*cosine_squared, second_coefficient_b)
            .mul_add(first_next, current_value);
        let second_magnitude = second_next.abs();
        tails_in_range &= second_magnitude == 0.0 || second_magnitude >= f64::MIN_POSITIVE;
        if ROTATE_SEQUENTIAL_STATES {
            let snapshot = snapshot_tail.next()?;
            *snapshot = first_next;
            *previous = second_next;
        } else {
            *previous = first_next;
            *current = second_next;
        }
    }

    let states_in_range = if CHECK_GROUP_RANGE && TRACK_STATE_RANGE {
        let vectors_in_range = if cfg!(rustsht_scalar_fused_analysis_pair_range_check) {
            scalar_states_in_range
        } else {
            analysis_mask_all(simd, vector_states_in_range)
        };
        let tails_in_range = if ROTATE_SEQUENTIAL_STATES {
            tails_in_range
        } else {
            previous_tail.iter().zip(current_tail.iter()).all(
                |(&previous_value, &current_value)| {
                    (previous_value == 0.0 || previous_value.abs() >= f64::MIN_POSITIVE)
                        && (current_value == 0.0 || current_value.abs() >= f64::MIN_POSITIVE)
                },
            )
        };
        vectors_in_range && tails_in_range
    } else {
        true
    };
    Some(FusedAnalysisPairGroup {
        first_same: Complex64::new(first_same_re_sum, first_same_im_sum),
        first_opposite: Complex64::new(first_opposite_re_sum, first_opposite_im_sum),
        second_same: Complex64::new(second_same_re_sum, second_same_im_sum),
        second_opposite: Complex64::new(second_opposite_re_sum, second_opposite_im_sum),
        states_in_range,
        tails_in_range,
    })
}

/// Checks the two recurrence states produced by a fused pair group after its
/// register-heavy accumulation pass has completed. This keeps the safety
/// check out of the multi-accumulator loop while preserving fallback on any
/// nonzero subnormal lane.
#[allow(unexpected_cfgs)]
#[inline(always)]
fn analysis_pair_group_state_in_range<S: Simd>(simd: S, previous: &[f64], current: &[f64]) -> bool {
    if previous.len() != current.len() {
        return false;
    }
    let (previous_vectors, previous_tail) = S::as_simd_f64s(previous);
    let (current_vectors, current_tail) = S::as_simd_f64s(current);
    if previous_vectors.len() != current_vectors.len() || previous_tail.len() != current_tail.len()
    {
        return false;
    }

    let zero = simd.splat_f64s(0.0);
    let minimum = simd.splat_f64s(f64::MIN_POSITIVE);
    #[cfg(not(rustsht_split_analysis_range_masks))]
    let mut vectors_in_range = simd.equal_f64s(zero, zero);
    #[cfg(not(rustsht_split_analysis_range_masks))]
    for (&previous_value, &current_value) in previous_vectors.iter().zip(current_vectors) {
        vectors_in_range = simd.and_m64s(
            vectors_in_range,
            analysis_pair_state_range_mask(simd, previous_value, current_value, zero, minimum),
        );
    }
    // Keep two independent mask chains in the scan loop. The AVX2 loop used
    // one loop-carried YMM AND chain after checking each recurrence group.
    #[cfg(rustsht_split_analysis_range_masks)]
    let mut first_vectors_in_range = simd.equal_f64s(zero, zero);
    #[cfg(rustsht_split_analysis_range_masks)]
    let mut second_vectors_in_range = simd.equal_f64s(zero, zero);
    #[cfg(rustsht_split_analysis_range_masks)]
    for pair in 0..previous_vectors.len() / 2 {
        let first = pair * 2;
        first_vectors_in_range = simd.and_m64s(
            first_vectors_in_range,
            analysis_pair_state_range_mask(
                simd,
                previous_vectors[first],
                current_vectors[first],
                zero,
                minimum,
            ),
        );
        let second = first + 1;
        second_vectors_in_range = simd.and_m64s(
            second_vectors_in_range,
            analysis_pair_state_range_mask(
                simd,
                previous_vectors[second],
                current_vectors[second],
                zero,
                minimum,
            ),
        );
    }
    #[cfg(rustsht_split_analysis_range_masks)]
    if previous_vectors.len() % 2 != 0 {
        let last = previous_vectors.len() - 1;
        first_vectors_in_range = simd.and_m64s(
            first_vectors_in_range,
            analysis_pair_state_range_mask(
                simd,
                previous_vectors[last],
                current_vectors[last],
                zero,
                minimum,
            ),
        );
    }
    #[cfg(rustsht_split_analysis_range_masks)]
    let vectors_in_range = simd.and_m64s(first_vectors_in_range, second_vectors_in_range);
    if !analysis_mask_all(simd, vectors_in_range) {
        return false;
    }

    previous_tail
        .iter()
        .zip(current_tail)
        .all(|(&previous_value, &current_value)| {
            (previous_value == 0.0 || previous_value.abs() >= f64::MIN_POSITIVE)
                && (current_value == 0.0 || current_value.abs() >= f64::MIN_POSITIVE)
        })
}

#[inline(always)]
fn analysis_pair_state_range_mask<S: Simd>(
    simd: S,
    previous: S::f64s,
    current: S::f64s,
    zero: S::f64s,
    minimum: S::f64s,
) -> S::m64s {
    let previous_magnitude = simd.abs_f64s(previous);
    let current_magnitude = simd.abs_f64s(current);
    let previous_in_range = simd.or_m64s(
        simd.equal_f64s(previous_magnitude, zero),
        simd.greater_than_or_equal_f64s(previous_magnitude, minimum),
    );
    let current_in_range = simd.or_m64s(
        simd.equal_f64s(current_magnitude, zero),
        simd.greater_than_or_equal_f64s(current_magnitude, minimum),
    );
    simd.and_m64s(previous_in_range, current_in_range)
}

#[inline(always)]
fn store_analysis_pair(
    coefficients: &mut [Complex64],
    offset: usize,
    has_odd: bool,
    same: Complex64,
    opposite: Complex64,
) {
    coefficients[offset] = same;
    if has_odd {
        coefficients[offset + 1] = opposite;
    }
}

impl<
    const OPPOSITE_COSINE_WEIGHTED: bool,
    const CHECK_PAIR_GROUP_RANGE: bool,
    const FUSE_PAIR_GROUP_RANGE_CHECK: bool,
    const ACCUMULATE: bool,
    const SEQUENTIAL_SECOND_CHANNEL: bool,
> WithSimd
    for FusedScalarAnalysisPair<
        '_,
        OPPOSITE_COSINE_WEIGHTED,
        CHECK_PAIR_GROUP_RANGE,
        FUSE_PAIR_GROUP_RANGE_CHECK,
        ACCUMULATE,
        SEQUENTIAL_SECOND_CHANNEL,
    >
{
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedScalarAnalysisPair {
            m,
            lmax,
            range_check_was_exercised,
            pair_start,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            second_channel,
            previous,
            current,
            next,
        } = self;
        let mut previous = previous;
        let mut current = current;
        let mut next = next;
        let mut second_channel = second_channel;
        let lane_count = current.len();
        if previous.len() != lane_count
            || next.len() != lane_count
            || cos_theta.len() != lane_count
            || cos_theta_squared.len() != lane_count
            || same_real.len() != lane_count
            || same_imag.len() != lane_count
            || opposite_real.len() != lane_count
            || opposite_imag.len() != lane_count
        {
            return false;
        }
        if SEQUENTIAL_SECOND_CHANNEL != second_channel.is_some() {
            return false;
        }
        if let Some(second) = second_channel.as_ref()
            && (second.same_real.len() != lane_count
                || second.same_imag.len() != lane_count
                || second.opposite_real.len() != lane_count
                || second.opposite_imag.len() != lane_count
                || second.staged_coefficients.len() < lmax - m + 1)
        {
            return false;
        }
        let zero = simd.splat_f64s(0.0);
        let mut all_vectors_in_range = simd.equal_f64s(zero, zero);
        let mut all_tails_in_range = true;

        let pair_count = (lmax - m + 2) / 2;
        let recurrence_stop = pair_count.saturating_sub(1);
        if CHECK_PAIR_GROUP_RANGE && pair_start < recurrence_stop.saturating_sub(1) {
            *range_check_was_exercised = true;
        }
        let mut pair_index = pair_start;
        while pair_index < recurrence_stop.saturating_sub(1) {
            let Some(group) = fused_analysis_two_pair_steps_with_simd::<
                S,
                OPPOSITE_COSINE_WEIGHTED,
                CHECK_PAIR_GROUP_RANGE,
                FUSE_PAIR_GROUP_RANGE_CHECK,
                ACCUMULATE,
                SEQUENTIAL_SECOND_CHANNEL,
            >(
                simd,
                FusedAnalysisTwoPairSteps {
                    same_real,
                    same_imag,
                    opposite_real,
                    opposite_imag,
                    basis_previous: previous,
                    basis_current: current,
                    basis_rotation_scratch: if SEQUENTIAL_SECOND_CHANNEL {
                        &mut next[..]
                    } else {
                        &mut []
                    },
                    cos_theta,
                    cos_theta_squared,
                    first_coefficient_a: coefficient_a[pair_index],
                    first_coefficient_b: coefficient_b[pair_index],
                    second_coefficient_a: coefficient_a[pair_index + 1],
                    second_coefficient_b: coefficient_b[pair_index + 1],
                },
            ) else {
                return false;
            };
            if SEQUENTIAL_SECOND_CHANNEL {
                // The sequential Q/U pass needs both pre-step basis values.
                // Rotate the three recurrence buffers so the old current basis
                // remains available in `next`, while the two next states become
                // `previous` and `current`. The SIMD step writes its first next
                // state to `next` and its second next state to the old `previous`,
                // avoiding a separate per-lane copy of the old current state.
                std::mem::swap(&mut previous, &mut next);
                std::mem::swap(&mut current, &mut next);
            }
            if CHECK_PAIR_GROUP_RANGE {
                let group_in_range = if FUSE_PAIR_GROUP_RANGE_CHECK {
                    group.states_in_range
                } else {
                    analysis_pair_group_state_in_range(simd, previous, current)
                };
                if !group_in_range {
                    #[cfg(test)]
                    ANALYSIS_PAIR_GROUP_RANGE_DECLINES
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return false;
                }
            }
            all_tails_in_range &= group.tails_in_range;

            let first_l = m + 2 * pair_index;
            let first_offset = first_l - m;
            store_analysis_pair(
                staged_coefficients,
                first_offset,
                first_l < lmax,
                group.first_same,
                group.first_opposite,
            );

            let second_l = first_l + 2;
            let second_offset = second_l - m;
            store_analysis_pair(
                staged_coefficients,
                second_offset,
                second_l < lmax,
                group.second_same,
                group.second_opposite,
            );
            if let Some(second) = second_channel.as_mut() {
                let [(first_same, first_opposite), (second_same, second_opposite)] =
                    AnalysisTwoBasisAccumulation {
                        same_real: second.same_real,
                        same_imag: second.same_imag,
                        opposite_real: second.opposite_real,
                        opposite_imag: second.opposite_imag,
                        first_basis: next,
                        second_basis: previous,
                    }
                    .with_simd(simd);
                store_analysis_pair(
                    second.staged_coefficients,
                    first_offset,
                    first_l < lmax,
                    first_same,
                    first_opposite,
                );
                store_analysis_pair(
                    second.staged_coefficients,
                    second_offset,
                    second_l < lmax,
                    second_same,
                    second_opposite,
                );
            }
            pair_index += 2;
        }

        while pair_index < recurrence_stop {
            let l = m + 2 * pair_index;
            let offset = l - m;
            if let Some(second) = second_channel.as_mut() {
                let (same_sum, opposite_sum) =
                    AnalysisPairAccumulation::<OPPOSITE_COSINE_WEIGHTED> {
                        same_real: second.same_real,
                        same_imag: second.same_imag,
                        opposite_real: second.opposite_real,
                        opposite_imag: second.opposite_imag,
                        basis: current,
                        cos_theta,
                    }
                    .with_simd(simd);
                store_analysis_pair(
                    second.staged_coefficients,
                    offset,
                    l < lmax,
                    same_sum,
                    opposite_sum,
                );
            }
            let (same_sum, opposite_sum, pair_vectors_in_range, pair_tails_in_range) =
                fused_analysis_pair_step_with_simd::<S, OPPOSITE_COSINE_WEIGHTED>(
                    simd,
                    FusedAnalysisPairStep {
                        same_real,
                        same_imag,
                        opposite_real,
                        opposite_imag,
                        basis_previous: previous,
                        basis_current: current,
                        cos_theta,
                        cos_theta_squared,
                        coefficient_a: coefficient_a[pair_index],
                        coefficient_b: coefficient_b[pair_index],
                        basis_next: next,
                    },
                );
            all_vectors_in_range = simd.and_m64s(all_vectors_in_range, pair_vectors_in_range);
            all_tails_in_range &= pair_tails_in_range;
            if ACCUMULATE {
                store_analysis_pair(
                    staged_coefficients,
                    offset,
                    l < lmax,
                    same_sum,
                    opposite_sum,
                );
            } else {
                store_analysis_pair(
                    staged_coefficients,
                    offset,
                    l < lmax,
                    Complex64::new(0.0, 0.0),
                    Complex64::new(0.0, 0.0),
                );
            }
            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
            pair_index += 1;
        }

        let last_pair = pair_count - 1;
        let l = m + 2 * last_pair;
        let offset = l - m;
        let (same_sum, opposite_sum) = if ACCUMULATE {
            AnalysisPairAccumulation::<OPPOSITE_COSINE_WEIGHTED> {
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                basis: current,
                cos_theta,
            }
            .with_simd(simd)
        } else {
            let zero = Complex64::new(0.0, 0.0);
            (zero, zero)
        };
        if let Some(second) = second_channel.as_mut() {
            let (same_sum, opposite_sum) = AnalysisPairAccumulation::<OPPOSITE_COSINE_WEIGHTED> {
                same_real: second.same_real,
                same_imag: second.same_imag,
                opposite_real: second.opposite_real,
                opposite_imag: second.opposite_imag,
                basis: current,
                cos_theta,
            }
            .with_simd(simd);
            store_analysis_pair(
                second.staged_coefficients,
                offset,
                l < lmax,
                same_sum,
                opposite_sum,
            );
        }
        store_analysis_pair(
            staged_coefficients,
            offset,
            l < lmax,
            same_sum,
            opposite_sum,
        );
        analysis_mask_all(simd, all_vectors_in_range) && all_tails_in_range
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The scalar analysis dispatch binds one channel's spectra to shared recurrence geometry and scratch."
)]
#[inline(always)]
fn dispatch_fused_scalar_analysis_pair<const CHECK_PAIR_GROUP_RANGE: bool>(
    arch: Arch,
    m: usize,
    lmax: usize,
    range_check_was_exercised: &mut bool,
    opposite_preweighted: bool,
    cos_theta: &[f64],
    cos_theta_squared: &[f64],
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &[f64],
    opposite_imag: &[f64],
    staged_coefficients: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> bool {
    if opposite_preweighted {
        dispatch_fused_scalar_analysis_pair_weighted::<true, CHECK_PAIR_GROUP_RANGE>(
            arch,
            m,
            lmax,
            range_check_was_exercised,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            previous,
            current,
            next,
        )
    } else {
        dispatch_fused_scalar_analysis_pair_weighted::<false, CHECK_PAIR_GROUP_RANGE>(
            arch,
            m,
            lmax,
            range_check_was_exercised,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            previous,
            current,
            next,
        )
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The scalar analysis dispatch binds one channel's spectra to shared recurrence geometry and scratch."
)]
#[inline(always)]
fn dispatch_fused_scalar_analysis_pair_weighted<
    const OPPOSITE_COSINE_WEIGHTED: bool,
    const CHECK_PAIR_GROUP_RANGE: bool,
>(
    arch: Arch,
    m: usize,
    lmax: usize,
    range_check_was_exercised: &mut bool,
    cos_theta: &[f64],
    cos_theta_squared: &[f64],
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &[f64],
    opposite_imag: &[f64],
    staged_coefficients: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> bool {
    arch.dispatch(
        FusedScalarAnalysisPair::<OPPOSITE_COSINE_WEIGHTED, CHECK_PAIR_GROUP_RANGE> {
            m,
            lmax,
            range_check_was_exercised,
            pair_start: 0,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            second_channel: None,
            previous,
            current,
            next,
        },
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The shared recurrence keeps Q inputs and staged U reductions in separate channel rows."
)]
#[allow(unexpected_cfgs)]
#[inline(always)]
fn dispatch_fused_scalar_analysis_pair_two_channels<
    const OPPOSITE_COSINE_WEIGHTED: bool,
    const CHECK_PAIR_GROUP_RANGE: bool,
>(
    arch: Arch,
    m: usize,
    lmax: usize,
    range_check_was_exercised: &mut bool,
    cos_theta: &[f64],
    cos_theta_squared: &[f64],
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    q_same_real: &[f64],
    q_same_imag: &[f64],
    q_opposite_real: &[f64],
    q_opposite_imag: &[f64],
    u_same_real: &[f64],
    u_same_imag: &[f64],
    u_opposite_real: &[f64],
    u_opposite_imag: &[f64],
    staged_q: &mut [Complex64],
    staged_u: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> bool {
    arch.dispatch(FusedScalarAnalysisPair::<
        OPPOSITE_COSINE_WEIGHTED,
        CHECK_PAIR_GROUP_RANGE,
        { cfg!(rustsht_fused_analysis_pair_range_check) },
        true,
        true,
    > {
        m,
        lmax,
        range_check_was_exercised,
        pair_start: 0,
        cos_theta,
        cos_theta_squared,
        coefficient_a,
        coefficient_b,
        same_real: q_same_real,
        same_imag: q_same_imag,
        opposite_real: q_opposite_real,
        opposite_imag: q_opposite_imag,
        staged_coefficients: staged_q,
        second_channel: Some(FusedScalarAnalysisPairSecondChannel {
            same_real: u_same_real,
            same_imag: u_same_imag,
            opposite_real: u_opposite_real,
            opposite_imag: u_opposite_imag,
            staged_coefficients: staged_u,
        }),
        previous,
        current,
        next,
    })
}

/// Computes a normal-range scalar analysis block for two spin-conversion channels.
///
/// Returns `false` when the recurrence requires binary scaling. Any staged
/// coefficients must then be discarded by the caller.
#[expect(
    clippy::too_many_arguments,
    reason = "The two-channel analysis block receives lane spectra and worker-owned recurrence scratch explicitly."
)]
#[cfg(test)]
pub(crate) fn fused_scalar_analysis_pair_channels_block_dispatched(
    arch: Arch,
    sequential_channels: bool,
    opposite_preweighted: bool,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: [&[f64]; 2],
    same_imag: [&[f64]; 2],
    opposite_real: [&[f64]; 2],
    opposite_imag: [&[f64]; 2],
    staged_first: &mut [Complex64],
    staged_second: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    let mut range_check_was_exercised = false;
    fused_scalar_analysis_pair_channels_block_with_range_proof(
        arch,
        sequential_channels,
        opposite_preweighted,
        false,
        false,
        &mut range_check_was_exercised,
        m,
        lmax,
        sin_theta,
        cos_theta,
        sectoral_norm,
        coefficient_a,
        coefficient_b,
        same_real,
        same_imag,
        opposite_real,
        opposite_imag,
        staged_first,
        staged_second,
        previous,
        current,
        next,
        exponents,
    )
}

struct NativeFusedAnalysisPairChannels<'a, T, const CHECK_RANGE: bool> {
    m: usize,
    lmax: usize,
    cos_theta: &'a [T],
    cos_theta_squared: &'a [T],
    coefficient_a: &'a [T],
    coefficient_b: &'a [T],
    same_real: [&'a [T]; 2],
    same_imag: [&'a [T]; 2],
    opposite_real: [&'a [T]; 2],
    opposite_imag: [&'a [T]; 2],
    staged_first: &'a mut [Complex<T>],
    staged_second: &'a mut [Complex<T>],
    previous: &'a mut [T],
    current: &'a mut [T],
    next: &'a mut [T],
}

impl<T: FusedAnalysisPrecision, const CHECK_RANGE: bool> WithSimd
    for NativeFusedAnalysisPairChannels<'_, T, CHECK_RANGE>
{
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> bool {
        let Self {
            m,
            lmax,
            cos_theta,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_first,
            staged_second,
            previous,
            current,
            next,
        } = self;
        let (cosine_vectors, cosine_tail) = T::as_simd::<S>(cos_theta);
        let (cosine_squared_vectors, cosine_squared_tail) = T::as_simd::<S>(cos_theta_squared);
        let (same_real_0, same_real_tail_0) = T::as_simd::<S>(same_real[0]);
        let (same_imag_0, same_imag_tail_0) = T::as_simd::<S>(same_imag[0]);
        let (opposite_real_0, opposite_real_tail_0) = T::as_simd::<S>(opposite_real[0]);
        let (opposite_imag_0, opposite_imag_tail_0) = T::as_simd::<S>(opposite_imag[0]);
        let (same_real_1, same_real_tail_1) = T::as_simd::<S>(same_real[1]);
        let (same_imag_1, same_imag_tail_1) = T::as_simd::<S>(same_imag[1]);
        let (opposite_real_1, opposite_real_tail_1) = T::as_simd::<S>(opposite_real[1]);
        let (opposite_imag_1, opposite_imag_tail_1) = T::as_simd::<S>(opposite_imag[1]);
        let (previous_vectors, previous_tail) = T::as_mut_simd::<S>(previous);
        let (current_vectors, current_tail) = T::as_mut_simd::<S>(current);
        let (next_vectors, next_tail) = T::as_mut_simd::<S>(next);
        let vector_count = current_vectors.len();
        if cosine_vectors.len() != vector_count
            || cosine_squared_vectors.len() != vector_count
            || same_real_0.len() != vector_count
            || same_imag_0.len() != vector_count
            || opposite_real_0.len() != vector_count
            || opposite_imag_0.len() != vector_count
            || same_real_1.len() != vector_count
            || same_imag_1.len() != vector_count
            || opposite_real_1.len() != vector_count
            || opposite_imag_1.len() != vector_count
            || previous_vectors.len() != vector_count
            || next_vectors.len() != vector_count
            || cosine_tail.len() != current_tail.len()
            || cosine_squared_tail.len() != current_tail.len()
            || same_real_tail_0.len() != current_tail.len()
            || same_imag_tail_0.len() != current_tail.len()
            || opposite_real_tail_0.len() != current_tail.len()
            || opposite_imag_tail_0.len() != current_tail.len()
            || same_real_tail_1.len() != current_tail.len()
            || same_imag_tail_1.len() != current_tail.len()
            || opposite_real_tail_1.len() != current_tail.len()
            || opposite_imag_tail_1.len() != current_tail.len()
            || previous_tail.len() != current_tail.len()
            || next_tail.len() != current_tail.len()
        {
            return false;
        }

        let zero = T::splat(simd, T::zero());
        let pair_count = (lmax - m + 2) / 2;
        let coefficient_count = lmax - m + 1;
        for pair in 0..pair_count {
            let mut same_re_0 = zero;
            let mut same_im_0 = zero;
            let mut opposite_re_0 = zero;
            let mut opposite_im_0 = zero;
            let mut same_re_1 = zero;
            let mut same_im_1 = zero;
            let mut opposite_re_1 = zero;
            let mut opposite_im_1 = zero;
            for index in 0..vector_count {
                let basis = current_vectors[index];
                same_re_0 = T::simd_fma(simd, same_real_0[index], basis, same_re_0);
                same_im_0 = T::simd_fma(simd, same_imag_0[index], basis, same_im_0);
                same_re_1 = T::simd_fma(simd, same_real_1[index], basis, same_re_1);
                same_im_1 = T::simd_fma(simd, same_imag_1[index], basis, same_im_1);
                let opposite_basis = T::simd_mul(simd, basis, cosine_vectors[index]);
                opposite_re_0 =
                    T::simd_fma(simd, opposite_real_0[index], opposite_basis, opposite_re_0);
                opposite_im_0 =
                    T::simd_fma(simd, opposite_imag_0[index], opposite_basis, opposite_im_0);
                opposite_re_1 =
                    T::simd_fma(simd, opposite_real_1[index], opposite_basis, opposite_re_1);
                opposite_im_1 =
                    T::simd_fma(simd, opposite_imag_1[index], opposite_basis, opposite_im_1);
            }
            if CHECK_RANGE
                && (T::vector_pair_needs_scaling(simd, same_re_0, same_im_0)
                    || T::vector_pair_needs_scaling(simd, opposite_re_0, opposite_im_0)
                    || T::vector_pair_needs_scaling(simd, same_re_1, same_im_1)
                    || T::vector_pair_needs_scaling(simd, opposite_re_1, opposite_im_1))
            {
                return false;
            }
            let mut same_re_0_sum = T::reduce_sum(simd, same_re_0);
            let mut same_im_0_sum = T::reduce_sum(simd, same_im_0);
            let mut opposite_re_0_sum = T::reduce_sum(simd, opposite_re_0);
            let mut opposite_im_0_sum = T::reduce_sum(simd, opposite_im_0);
            let mut same_re_1_sum = T::reduce_sum(simd, same_re_1);
            let mut same_im_1_sum = T::reduce_sum(simd, same_im_1);
            let mut opposite_re_1_sum = T::reduce_sum(simd, opposite_re_1);
            let mut opposite_im_1_sum = T::reduce_sum(simd, opposite_im_1);
            for lane in 0..current_tail.len() {
                let basis = current_tail[lane];
                same_re_0_sum = same_real_tail_0[lane].mul_add(basis, same_re_0_sum);
                same_im_0_sum = same_imag_tail_0[lane].mul_add(basis, same_im_0_sum);
                same_re_1_sum = same_real_tail_1[lane].mul_add(basis, same_re_1_sum);
                same_im_1_sum = same_imag_tail_1[lane].mul_add(basis, same_im_1_sum);
                let opposite_basis = basis * cosine_tail[lane];
                opposite_re_0_sum =
                    opposite_real_tail_0[lane].mul_add(opposite_basis, opposite_re_0_sum);
                opposite_im_0_sum =
                    opposite_imag_tail_0[lane].mul_add(opposite_basis, opposite_im_0_sum);
                opposite_re_1_sum =
                    opposite_real_tail_1[lane].mul_add(opposite_basis, opposite_re_1_sum);
                opposite_im_1_sum =
                    opposite_imag_tail_1[lane].mul_add(opposite_basis, opposite_im_1_sum);
            }
            if CHECK_RANGE
                && ![
                    same_re_0_sum,
                    same_im_0_sum,
                    opposite_re_0_sum,
                    opposite_im_0_sum,
                    same_re_1_sum,
                    same_im_1_sum,
                    opposite_re_1_sum,
                    opposite_im_1_sum,
                ]
                .into_iter()
                .all(T::scalar_in_fast_range)
            {
                return false;
            }

            let degree = m + 2 * pair;
            let offset = degree - m;
            let first = Complex::new(same_re_0_sum, same_im_0_sum);
            let first_odd = Complex::new(opposite_re_0_sum, opposite_im_0_sum);
            let second = Complex::new(same_re_1_sum, same_im_1_sum);
            let second_odd = Complex::new(opposite_re_1_sum, opposite_im_1_sum);
            if ![
                first.re,
                first.im,
                first_odd.re,
                first_odd.im,
                second.re,
                second.im,
                second_odd.re,
                second_odd.im,
            ]
            .into_iter()
            .all(T::scalar_in_fast_range)
            {
                return false;
            }
            staged_first[offset] = first;
            staged_second[offset] = second;
            if offset + 1 < coefficient_count {
                staged_first[offset + 1] = first_odd;
                staged_second[offset + 1] = second_odd;
            }

            if pair + 1 == pair_count {
                continue;
            }
            let coefficient_a_value = coefficient_a[pair];
            let coefficient_b_value = coefficient_b[pair];
            let coefficient_a_vector = T::splat(simd, coefficient_a_value);
            let coefficient_b_vector = T::splat(simd, coefficient_b_value);
            for index in 0..vector_count {
                let leading = T::simd_fma(
                    simd,
                    coefficient_a_vector,
                    cosine_squared_vectors[index],
                    coefficient_b_vector,
                );
                let basis = current_vectors[index];
                let next_value = T::simd_fma(simd, leading, basis, previous_vectors[index]);
                if CHECK_RANGE
                    && (T::vector_pair_needs_scaling(simd, leading, zero)
                        || T::vector_pair_needs_scaling(simd, basis, next_value))
                {
                    return false;
                }
                previous_vectors[index] = basis;
                current_vectors[index] = next_value;
                next_vectors[index] = next_value;
            }
            for lane in 0..current_tail.len() {
                let leading =
                    coefficient_a_value.mul_add(cosine_squared_tail[lane], coefficient_b_value);
                let basis = current_tail[lane];
                let next_value = leading.mul_add(basis, previous_tail[lane]);
                if CHECK_RANGE
                    && (!T::scalar_in_fast_range(leading)
                        || !T::scalar_in_fast_range(basis)
                        || !T::scalar_in_fast_range(next_value))
                {
                    return false;
                }
                previous_tail[lane] = basis;
                current_tail[lane] = next_value;
                next_tail[lane] = next_value;
            }
        }
        true
    }
}

/// Scaled-sectoral counterpart to the ordinary fused block. It keeps both
/// channel accumulators fused while the shared recurrence carries lane-wise
/// binary exponents, then uses the native precision's guarded scaled product
/// leaf for each contribution.
struct NativeScaledFusedAnalysisPairChannels<'a, T> {
    m: usize,
    lmax: usize,
    cos_theta: &'a [T],
    coefficient_a: &'a [T],
    coefficient_b: &'a [T],
    same_real: [&'a [T]; 2],
    same_imag: [&'a [T]; 2],
    opposite_real: [&'a [T]; 2],
    opposite_imag: [&'a [T]; 2],
    staged_first: &'a mut [Complex<T>],
    staged_second: &'a mut [Complex<T>],
    previous: &'a mut [T],
    current: &'a mut [T],
    next: &'a mut [T],
    exponents: &'a mut [i64],
}

impl<T: FusedAnalysisPrecision> WithSimd for NativeScaledFusedAnalysisPairChannels<'_, T> {
    type Output = Result<bool, ShtError>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self {
            m,
            lmax,
            cos_theta,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_first,
            staged_second,
            previous,
            current,
            next,
            exponents,
        } = self;
        let lane_count = cos_theta.len();
        if lane_count == 0
            || same_real.iter().any(|values| values.len() != lane_count)
            || same_imag.iter().any(|values| values.len() != lane_count)
            || opposite_real
                .iter()
                .any(|values| values.len() != lane_count)
            || opposite_imag
                .iter()
                .any(|values| values.len() != lane_count)
            || previous.len() != lane_count
            || current.len() != lane_count
            || next.len() != lane_count
            || exponents.len() != lane_count
        {
            return Ok(false);
        }

        let mut previous = previous;
        let mut current = current;
        let mut next = next;
        T::normalize_pair_state(previous, current, exponents)?;
        let mut scaled_states = [
            T::prepare_scaled_state(
                same_real[0],
                same_imag[0],
                opposite_real[0],
                opposite_imag[0],
            ),
            T::prepare_scaled_state(
                same_real[1],
                same_imag[1],
                opposite_real[1],
                opposite_imag[1],
            ),
        ];
        let pair_count = (lmax - m + 2) / 2;
        let coefficient_count = lmax - m + 1;

        for pair in 0..pair_count {
            let mut sums = [[T::zero(); 4]; 2];
            for channel in 0..2 {
                let (same, opposite) = T::accumulate_scaled::<S, false>(
                    simd,
                    same_real[channel],
                    same_imag[channel],
                    opposite_real[channel],
                    opposite_imag[channel],
                    &current[..lane_count],
                    exponents,
                    cos_theta,
                    &mut scaled_states[channel],
                );
                sums[channel] = [same.re, same.im, opposite.re, opposite.im];
            }

            if !sums
                .iter()
                .flat_map(|channel| channel.iter().copied())
                .all(|value| value.is_finite())
            {
                return Ok(false);
            }
            let degree = m + 2 * pair;
            let offset = degree - m;
            staged_first[offset] = Complex::new(sums[0][0], sums[0][1]);
            staged_second[offset] = Complex::new(sums[1][0], sums[1][1]);
            if offset + 1 < coefficient_count {
                staged_first[offset + 1] = Complex::new(sums[0][2], sums[0][3]);
                staged_second[offset + 1] = Complex::new(sums[1][2], sums[1][3]);
            }

            if pair + 1 == pair_count {
                continue;
            }
            T::step_pair(
                simd,
                coefficient_a[pair],
                coefficient_b[pair],
                cos_theta,
                previous,
                current,
                next,
            );
            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
            if T::pair_state_needs_scaling(simd, previous, current) {
                T::normalize_pair_state(previous, current, exponents)?;
                for state in &mut scaled_states {
                    T::invalidate_scaled_state(state);
                }
            }
        }
        Ok(true)
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The native paired kernel receives reflected channels and caller-owned recurrence scratch."
)]
pub(crate) fn fused_scalar_analysis_pair_channels_block_native<T: FusedAnalysisPrecision>(
    arch: Arch,
    range_check_is_proven_safe: bool,
    range_check_was_exercised: &mut bool,
    m: usize,
    lmax: usize,
    sin_theta: &[T],
    cos_theta: &[T],
    sectoral_norm: T,
    coefficient_a: &[T],
    coefficient_b: &[T],
    same_real: [&[T]; 2],
    same_imag: [&[T]; 2],
    opposite_real: [&[T]; 2],
    opposite_imag: [&[T]; 2],
    staged_first: &mut [Complex<T>],
    staged_second: &mut [Complex<T>],
    previous: &mut [T],
    current: &mut [T],
    next: &mut [T],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    *range_check_was_exercised = false;
    if sin_theta.is_empty()
        || sin_theta.len() != cos_theta.len()
        || m > lmax
        || sin_theta.len() > LATITUDE_BLOCK_SIZE
        || previous.len() < sin_theta.len()
        || current.len() < sin_theta.len()
        || next.len() < sin_theta.len()
        || exponents.len() < sin_theta.len()
    {
        return Ok(false);
    }
    let pair_count = (lmax - m + 2) / 2;
    let coefficient_count = lmax - m + 1;
    if coefficient_a.len() < pair_count
        || coefficient_b.len() < pair_count
        || staged_first.len() < coefficient_count
        || staged_second.len() < coefficient_count
    {
        return Ok(false);
    }
    for channel in 0..2 {
        if same_real[channel].len() != sin_theta.len()
            || same_imag[channel].len() != sin_theta.len()
            || opposite_real[channel].len() != sin_theta.len()
            || opposite_imag[channel].len() != sin_theta.len()
        {
            return Ok(false);
        }
        if !same_real[channel]
            .iter()
            .chain(same_imag[channel])
            .chain(opposite_real[channel])
            .chain(opposite_imag[channel])
            .all(|&value| T::scalar_in_fast_range(value))
        {
            return Ok(false);
        }
    }
    if !cos_theta
        .iter()
        .all(|&value| value.is_finite() && value.abs() <= T::one())
    {
        return Ok(false);
    }
    T::initialize_pair_state(
        m,
        sin_theta,
        sectoral_norm,
        &mut previous[..sin_theta.len()],
        &mut current[..sin_theta.len()],
        &mut exponents[..sin_theta.len()],
    )?;
    let mut ordinary_seed = true;
    for ring in 0..sin_theta.len() {
        let exponent = exponents[ring];
        ordinary_seed &= T::materialize_scaled_seed(previous[ring], exponent)
            .is_some_and(T::scalar_in_fast_range)
            && T::materialize_scaled_seed(current[ring], exponent)
                .is_some_and(T::scalar_in_fast_range);
    }
    staged_first[..coefficient_count].fill(Complex::new(T::zero(), T::zero()));
    staged_second[..coefficient_count].fill(Complex::new(T::zero(), T::zero()));
    if !ordinary_seed {
        return arch.dispatch(NativeScaledFusedAnalysisPairChannels {
            m,
            lmax,
            cos_theta,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_first,
            staged_second,
            previous: &mut previous[..sin_theta.len()],
            current: &mut current[..sin_theta.len()],
            next: &mut next[..sin_theta.len()],
            exponents: &mut exponents[..sin_theta.len()],
        });
    }
    for ring in 0..sin_theta.len() {
        let exponent = exponents[ring];
        previous[ring] = T::materialize_scaled_seed(previous[ring], exponent)
            .expect("ordinary seed was validated");
        current[ring] = T::materialize_scaled_seed(current[ring], exponent)
            .expect("ordinary seed was validated");
        exponents[ring] = 0;
    }
    let mut cos_theta_squared = [T::zero(); LATITUDE_BLOCK_SIZE];
    for (squared, &cosine) in cos_theta_squared.iter_mut().zip(cos_theta) {
        *squared = cosine * cosine;
    }
    let succeeded = if range_check_is_proven_safe {
        arch.dispatch(NativeFusedAnalysisPairChannels::<T, false> {
            m,
            lmax,
            cos_theta,
            cos_theta_squared: &cos_theta_squared[..sin_theta.len()],
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_first,
            staged_second,
            previous: &mut previous[..sin_theta.len()],
            current: &mut current[..sin_theta.len()],
            next: &mut next[..sin_theta.len()],
        })
    } else {
        arch.dispatch(NativeFusedAnalysisPairChannels::<T, true> {
            m,
            lmax,
            cos_theta,
            cos_theta_squared: &cos_theta_squared[..sin_theta.len()],
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_first,
            staged_second,
            previous: &mut previous[..sin_theta.len()],
            current: &mut current[..sin_theta.len()],
            next: &mut next[..sin_theta.len()],
        })
    };
    if succeeded && !range_check_is_proven_safe {
        *range_check_was_exercised = true;
    }
    Ok(succeeded)
}

#[expect(
    clippy::too_many_arguments,
    reason = "The checked shared-recurrence trial stages both scalar channels independently."
)]
#[inline(never)]
fn try_fused_scalar_analysis_pair_shared_recurrence(
    arch: Arch,
    range_check_is_proven_safe: bool,
    range_check_was_exercised: &mut bool,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: [&[f64]; 2],
    same_imag: [&[f64]; 2],
    opposite_real: [&[f64]; 2],
    opposite_imag: [&[f64]; 2],
    staged_first: &mut [Complex64],
    staged_second: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
) -> Result<Option<bool>, ShtError> {
    let mut q_opposite_real = [0.0; LATITUDE_BLOCK_SIZE];
    let mut q_opposite_imag = [0.0; LATITUDE_BLOCK_SIZE];
    let mut u_opposite_real = [0.0; LATITUDE_BLOCK_SIZE];
    let mut u_opposite_imag = [0.0; LATITUDE_BLOCK_SIZE];
    q_opposite_real[..sin_theta.len()].copy_from_slice(opposite_real[0]);
    q_opposite_imag[..sin_theta.len()].copy_from_slice(opposite_imag[0]);
    u_opposite_real[..sin_theta.len()].copy_from_slice(opposite_real[1]);
    u_opposite_imag[..sin_theta.len()].copy_from_slice(opposite_imag[1]);
    let q_opposite_prepared = prepare_analysis_opposite(
        &mut q_opposite_real[..sin_theta.len()],
        &mut q_opposite_imag[..sin_theta.len()],
        cos_theta,
    );
    let u_opposite_prepared = prepare_analysis_opposite(
        &mut u_opposite_real[..sin_theta.len()],
        &mut u_opposite_imag[..sin_theta.len()],
        cos_theta,
    );
    if !q_opposite_prepared || !u_opposite_prepared {
        return Ok(None);
    }
    let mut cos_theta_squared = [0.0; LATITUDE_BLOCK_SIZE];
    for (squared, &cosine) in cos_theta_squared.iter_mut().zip(cos_theta) {
        *squared = cosine * cosine;
    }
    let initialized_normal = initialize_analysis_sectoral_lanes(
        arch,
        m,
        lmax,
        sin_theta,
        sectoral_norm,
        &mut previous[..sin_theta.len()],
        &mut current[..sin_theta.len()],
        &mut exponents[..sin_theta.len()],
    )?;
    if !initialized_normal {
        let recurrence_succeeded = if range_check_is_proven_safe {
            arch.dispatch(FusedCheckedScaledAnalysisPairChannels::<false> {
                m,
                lmax,
                range_check_was_exercised,
                cos_theta,
                cos_theta_squared: &cos_theta_squared[..sin_theta.len()],
                coefficient_a,
                coefficient_b,
                same_real,
                same_imag,
                ordinary_opposite_real: [
                    &q_opposite_real[..sin_theta.len()],
                    &u_opposite_real[..sin_theta.len()],
                ],
                ordinary_opposite_imag: [
                    &q_opposite_imag[..sin_theta.len()],
                    &u_opposite_imag[..sin_theta.len()],
                ],
                staged_first,
                staged_second,
                previous: &mut previous[..sin_theta.len()],
                current: &mut current[..sin_theta.len()],
                next: &mut next[..sin_theta.len()],
                exponents: &mut exponents[..sin_theta.len()],
            })?
        } else {
            arch.dispatch(FusedCheckedScaledAnalysisPairChannels::<true> {
                m,
                lmax,
                range_check_was_exercised,
                cos_theta,
                cos_theta_squared: &cos_theta_squared[..sin_theta.len()],
                coefficient_a,
                coefficient_b,
                same_real,
                same_imag,
                ordinary_opposite_real: [
                    &q_opposite_real[..sin_theta.len()],
                    &u_opposite_real[..sin_theta.len()],
                ],
                ordinary_opposite_imag: [
                    &q_opposite_imag[..sin_theta.len()],
                    &u_opposite_imag[..sin_theta.len()],
                ],
                staged_first,
                staged_second,
                previous: &mut previous[..sin_theta.len()],
                current: &mut current[..sin_theta.len()],
                next: &mut next[..sin_theta.len()],
                exponents: &mut exponents[..sin_theta.len()],
            })?
        };
        let pair_count = (lmax - m + 2) / 2;
        let single_pair_tail_writes_next = !pair_count.saturating_sub(1).is_multiple_of(2);
        let state_within_upper_bound = previous[..sin_theta.len()]
            .iter()
            .chain(&current[..sin_theta.len()])
            .all(|value| value.is_finite() && value.abs() <= scalar::BIG)
            && (!single_pair_tail_writes_next
                || next[..sin_theta.len()]
                    .iter()
                    .all(|value| value.is_finite() && value.abs() <= scalar::BIG));
        return Ok(Some(recurrence_succeeded && state_within_upper_bound));
    }

    let mut shared_range_check_was_exercised = false;
    let recurrence_succeeded = if range_check_is_proven_safe {
        dispatch_fused_scalar_analysis_pair_two_channels::<true, false>(
            arch,
            m,
            lmax,
            &mut shared_range_check_was_exercised,
            cos_theta,
            &cos_theta_squared[..sin_theta.len()],
            coefficient_a,
            coefficient_b,
            same_real[0],
            same_imag[0],
            &q_opposite_real[..sin_theta.len()],
            &q_opposite_imag[..sin_theta.len()],
            same_real[1],
            same_imag[1],
            &u_opposite_real[..sin_theta.len()],
            &u_opposite_imag[..sin_theta.len()],
            staged_first,
            staged_second,
            &mut previous[..sin_theta.len()],
            &mut current[..sin_theta.len()],
            &mut next[..sin_theta.len()],
        )
    } else {
        dispatch_fused_scalar_analysis_pair_two_channels::<
            true,
            { !cfg!(rustsht_unchecked_analysis_pair_range) },
        >(
            arch,
            m,
            lmax,
            &mut shared_range_check_was_exercised,
            cos_theta,
            &cos_theta_squared[..sin_theta.len()],
            coefficient_a,
            coefficient_b,
            same_real[0],
            same_imag[0],
            &q_opposite_real[..sin_theta.len()],
            &q_opposite_imag[..sin_theta.len()],
            same_real[1],
            same_imag[1],
            &u_opposite_real[..sin_theta.len()],
            &u_opposite_imag[..sin_theta.len()],
            staged_first,
            staged_second,
            &mut previous[..sin_theta.len()],
            &mut current[..sin_theta.len()],
            &mut next[..sin_theta.len()],
        )
    };
    *range_check_was_exercised |= shared_range_check_was_exercised;

    let pair_count = (lmax - m + 2) / 2;
    let single_pair_tail_writes_next = !pair_count.saturating_sub(1).is_multiple_of(2);
    let state_within_upper_bound = previous[..sin_theta.len()]
        .iter()
        .chain(&current[..sin_theta.len()])
        .all(|value| value.is_finite() && value.abs() <= scalar::BIG)
        && (!single_pair_tail_writes_next
            || next[..sin_theta.len()]
                .iter()
                .all(|value| value.is_finite() && value.abs() <= scalar::BIG));
    Ok(Some(recurrence_succeeded && state_within_upper_bound))
}

#[expect(
    clippy::too_many_arguments,
    reason = "The two-channel analysis block receives lane spectra and worker-owned recurrence scratch explicitly."
)]
pub(crate) fn fused_scalar_analysis_pair_channels_block_with_range_proof(
    arch: Arch,
    sequential_channels: bool,
    opposite_preweighted: bool,
    shared_recurrence_sequential_accumulation: bool,
    range_check_is_proven_safe: bool,
    range_check_was_exercised: &mut bool,
    m: usize,
    lmax: usize,
    sin_theta: &[f64],
    cos_theta: &[f64],
    sectoral_norm: f64,
    coefficient_a: &[f64],
    coefficient_b: &[f64],
    same_real: [&[f64]; 2],
    same_imag: [&[f64]; 2],
    opposite_real: [&[f64]; 2],
    opposite_imag: [&[f64]; 2],
    staged_first: &mut [Complex64],
    staged_second: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
    exponents: &mut [i64],
) -> Result<bool, ShtError> {
    *range_check_was_exercised = false;
    debug_assert!(m <= lmax);
    debug_assert!(!opposite_preweighted || sequential_channels);
    debug_assert!(
        !shared_recurrence_sequential_accumulation || (sequential_channels && opposite_preweighted)
    );
    let coefficient_count = lmax - m + 1;
    debug_assert!(sin_theta.len() <= LATITUDE_BLOCK_SIZE);
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    for channel in 0..2 {
        debug_assert_eq!(sin_theta.len(), same_real[channel].len());
        debug_assert_eq!(sin_theta.len(), same_imag[channel].len());
        debug_assert_eq!(sin_theta.len(), opposite_real[channel].len());
        debug_assert_eq!(sin_theta.len(), opposite_imag[channel].len());
    }
    debug_assert!(staged_first.len() >= coefficient_count);
    debug_assert!(staged_second.len() >= coefficient_count);
    debug_assert!(previous.len() >= sin_theta.len());
    debug_assert!(current.len() >= sin_theta.len());
    debug_assert!(next.len() >= sin_theta.len());
    debug_assert!(exponents.len() >= sin_theta.len());

    if sin_theta.is_empty() {
        return Ok(false);
    }

    if shared_recurrence_sequential_accumulation
        && let Some(fused) = try_fused_scalar_analysis_pair_shared_recurrence(
            arch,
            range_check_is_proven_safe,
            range_check_was_exercised,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            &mut staged_first[..coefficient_count],
            &mut staged_second[..coefficient_count],
            previous,
            current,
            next,
            exponents,
        )?
    {
        return Ok(fused);
    }

    if !sequential_channels {
        if !initialize_analysis_sectoral_lanes(
            arch,
            m,
            lmax,
            sin_theta,
            sectoral_norm,
            &mut previous[..sin_theta.len()],
            &mut current[..sin_theta.len()],
            &mut exponents[..sin_theta.len()],
        )? {
            return Ok(false);
        }
        return Ok(arch.dispatch(FusedScalarAnalysisPairChannels {
            m,
            lmax,
            cos_theta,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_first: &mut staged_first[..coefficient_count],
            staged_second: &mut staged_second[..coefficient_count],
            previous: &mut previous[..sin_theta.len()],
            current: &mut current[..sin_theta.len()],
            next: &mut next[..sin_theta.len()],
        }));
    }

    let mut cos_theta_squared = [0.0; LATITUDE_BLOCK_SIZE];
    for (squared, &cosine) in cos_theta_squared.iter_mut().zip(cos_theta) {
        *squared = cosine * cosine;
    }

    // DUCC's spin-1 analysis runs one scalar recurrence per converted channel.
    // Keep the reflected inputs packed once, then reuse the checked single-channel
    // kernel with its smaller accumulator set for each row.
    let mut channels_succeeded = true;
    let mut first_channel_proved_recurrence_range = range_check_is_proven_safe;
    // Weight each opposite input once per lane; retain the originals for fallback.
    let mut weighted_opposite_real = [0.0; LATITUDE_BLOCK_SIZE];
    let mut weighted_opposite_imag = [0.0; LATITUDE_BLOCK_SIZE];
    for (channel, staged) in [
        (0, &mut staged_first[..coefficient_count]),
        (1, &mut staged_second[..coefficient_count]),
    ] {
        if !initialize_analysis_sectoral_lanes(
            arch,
            m,
            lmax,
            sin_theta,
            sectoral_norm,
            &mut previous[..sin_theta.len()],
            &mut current[..sin_theta.len()],
            &mut exponents[..sin_theta.len()],
        )? {
            channels_succeeded = false;
            continue;
        }
        let (opposite_real, opposite_imag, channel_opposite_preweighted) = if opposite_preweighted {
            weighted_opposite_real[..sin_theta.len()].copy_from_slice(opposite_real[channel]);
            weighted_opposite_imag[..sin_theta.len()].copy_from_slice(opposite_imag[channel]);
            if prepare_analysis_opposite(
                &mut weighted_opposite_real[..sin_theta.len()],
                &mut weighted_opposite_imag[..sin_theta.len()],
                cos_theta,
            ) {
                (
                    &weighted_opposite_real[..sin_theta.len()],
                    &weighted_opposite_imag[..sin_theta.len()],
                    true,
                )
            } else {
                (opposite_real[channel], opposite_imag[channel], false)
            }
        } else {
            (opposite_real[channel], opposite_imag[channel], false)
        };
        let mut channel_range_check_was_exercised = false;
        let recurrence_succeeded = if first_channel_proved_recurrence_range {
            dispatch_fused_scalar_analysis_pair::<false>(
                arch,
                m,
                lmax,
                &mut channel_range_check_was_exercised,
                channel_opposite_preweighted,
                cos_theta,
                &cos_theta_squared[..sin_theta.len()],
                coefficient_a,
                coefficient_b,
                same_real[channel],
                same_imag[channel],
                opposite_real,
                opposite_imag,
                staged,
                &mut previous[..sin_theta.len()],
                &mut current[..sin_theta.len()],
                &mut next[..sin_theta.len()],
            )
        } else {
            dispatch_fused_scalar_analysis_pair::<{ !cfg!(rustsht_unchecked_analysis_pair_range) }>(
                arch,
                m,
                lmax,
                &mut channel_range_check_was_exercised,
                channel_opposite_preweighted,
                cos_theta,
                &cos_theta_squared[..sin_theta.len()],
                coefficient_a,
                coefficient_b,
                same_real[channel],
                same_imag[channel],
                opposite_real,
                opposite_imag,
                staged,
                &mut previous[..sin_theta.len()],
                &mut current[..sin_theta.len()],
                &mut next[..sin_theta.len()],
            )
        };
        *range_check_was_exercised |= channel_range_check_was_exercised;
        // The two-pair groups advance `previous` and `current` directly and
        // leave `next` untouched. Only the single-pair tail writes that slot.
        let pair_count = (lmax - m + 2) / 2;
        let single_pair_tail_writes_next = !pair_count.saturating_sub(1).is_multiple_of(2);
        let state_within_upper_bound = previous[..sin_theta.len()]
            .iter()
            .chain(&current[..sin_theta.len()])
            .all(|value| value.is_finite() && value.abs() <= scalar::BIG)
            && (!single_pair_tail_writes_next
                || next[..sin_theta.len()]
                    .iter()
                    .all(|value| value.is_finite() && value.abs() <= scalar::BIG));
        let channel_succeeded = recurrence_succeeded && state_within_upper_bound;
        if channel == 0 {
            // The basis recurrence depends only on order and latitude geometry.
            // A completed checked first-channel traversal therefore proves that
            // the reinitialized recurrence for channel two stays in range too.
            first_channel_proved_recurrence_range = channel_succeeded
                && (channel_range_check_was_exercised || range_check_is_proven_safe);
        }
        channels_succeeded &= channel_succeeded;
    }
    Ok(channels_succeeded)
}

struct FusedScalarAnalysisPairChannels<'a> {
    m: usize,
    lmax: usize,
    cos_theta: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    same_real: [&'a [f64]; 2],
    same_imag: [&'a [f64]; 2],
    opposite_real: [&'a [f64]; 2],
    opposite_imag: [&'a [f64]; 2],
    staged_first: &'a mut [Complex64],
    staged_second: &'a mut [Complex64],
    previous: &'a mut [f64],
    current: &'a mut [f64],
    next: &'a mut [f64],
}

impl WithSimd for FusedScalarAnalysisPairChannels<'_> {
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedScalarAnalysisPairChannels {
            m,
            lmax,
            cos_theta,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_first,
            staged_second,
            previous,
            current,
            next,
        } = self;
        let mut previous = previous;
        let mut current = current;
        let mut next = next;

        for (pair_index, l) in (m..=lmax).step_by(2).enumerate() {
            let offset = l - m;
            let channel_sums = AnalysisPairChannelsAccumulation {
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                basis: current,
                cos_theta,
            }
            .with_simd(simd);
            staged_first[offset] = channel_sums[0].0;
            staged_second[offset] = channel_sums[1].0;
            if l < lmax {
                staged_first[offset + 1] = channel_sums[0].1;
                staged_second[offset + 1] = channel_sums[1].1;
            }

            if l + 2 > lmax {
                break;
            }

            let (current_vectors, current_tail) = S::as_simd_f64s(current);
            let (previous_vectors, previous_tail) = S::as_simd_f64s(previous);
            let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
            let (next_vectors, next_tail) = S::as_mut_simd_f64s(next);
            let a = simd.splat_f64s(coefficient_a[pair_index]);
            let b = simd.splat_f64s(coefficient_b[pair_index]);
            let big = simd.splat_f64s(scalar::BIG);
            let zeros = simd.splat_f64s(0.0);
            debug_assert_eq!(current_vectors.len(), previous_vectors.len());
            debug_assert_eq!(current_vectors.len(), cosine_vectors.len());
            debug_assert_eq!(current_vectors.len(), next_vectors.len());
            // The previous/current pair is normal on entry. Check each new
            // next value before it becomes current on the following iteration.
            for index in 0..current_vectors.len() {
                let cosine_squared = simd.mul_f64s(cosine_vectors[index], cosine_vectors[index]);
                let leading = simd.mul_add_f64s(a, cosine_squared, b);
                let next_value =
                    simd.mul_add_f64s(leading, current_vectors[index], previous_vectors[index]);
                next_vectors[index] = next_value;

                let magnitude = simd.abs_f64s(next_value);
                let below_upper_bound = simd.less_than_or_equal_f64s(magnitude, big);
                let is_zero = simd.equal_f64s(magnitude, zeros);
                let lower_check = simd.select_f64s(is_zero, big, magnitude);
                let in_range = simd.select_f64s(below_upper_bound, lower_check, zeros);
                if simd.reduce_min_f64s(in_range) < scalar::SMALL {
                    return false;
                }
            }
            for index in 0..current_tail.len() {
                let cosine_squared = cosine_tail[index] * cosine_tail[index];
                let leading =
                    coefficient_a[pair_index].mul_add(cosine_squared, coefficient_b[pair_index]);
                let next_value = leading.mul_add(current_tail[index], previous_tail[index]);
                next_tail[index] = next_value;
                let magnitude = next_value.abs();
                if !next_value.is_finite()
                    || magnitude > scalar::BIG
                    || (magnitude != 0.0 && magnitude < scalar::SMALL)
                {
                    return false;
                }
            }

            std::mem::swap(&mut previous, &mut current);
            std::mem::swap(&mut current, &mut next);
        }
        true
    }
}

struct AnalysisPairAccumulation<'a, const OPPOSITE_COSINE_WEIGHTED: bool = false> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis: &'a [f64],
    cos_theta: &'a [f64],
}

struct AnalysisTwoBasisAccumulation<'a> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    first_basis: &'a [f64],
    second_basis: &'a [f64],
}

impl WithSimd for AnalysisTwoBasisAccumulation<'_> {
    type Output = [(Complex64, Complex64); 2];

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let (same_real_vectors, same_real_tail) = S::as_simd_f64s(self.same_real);
        let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(self.same_imag);
        let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(self.opposite_real);
        let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(self.opposite_imag);
        let (first_basis_vectors, first_basis_tail) = S::as_simd_f64s(self.first_basis);
        let (second_basis_vectors, second_basis_tail) = S::as_simd_f64s(self.second_basis);
        debug_assert_eq!(same_real_vectors.len(), same_imag_vectors.len());
        debug_assert_eq!(same_real_vectors.len(), opposite_real_vectors.len());
        debug_assert_eq!(same_real_vectors.len(), opposite_imag_vectors.len());
        debug_assert_eq!(same_real_vectors.len(), first_basis_vectors.len());
        debug_assert_eq!(same_real_vectors.len(), second_basis_vectors.len());

        let zero = simd.splat_f64s(0.0);
        let mut first_sums = [zero; 4];
        let mut second_sums = [zero; 4];
        for (
            ((((same_real, same_imag), opposite_real), opposite_imag), first_basis),
            second_basis,
        ) in same_real_vectors
            .iter()
            .zip(same_imag_vectors)
            .zip(opposite_real_vectors)
            .zip(opposite_imag_vectors)
            .zip(first_basis_vectors)
            .zip(second_basis_vectors)
        {
            first_sums[0] = simd.mul_add_f64s(*same_real, *first_basis, first_sums[0]);
            first_sums[1] = simd.mul_add_f64s(*same_imag, *first_basis, first_sums[1]);
            first_sums[2] = simd.mul_add_f64s(*opposite_real, *first_basis, first_sums[2]);
            first_sums[3] = simd.mul_add_f64s(*opposite_imag, *first_basis, first_sums[3]);
            second_sums[0] = simd.mul_add_f64s(*same_real, *second_basis, second_sums[0]);
            second_sums[1] = simd.mul_add_f64s(*same_imag, *second_basis, second_sums[1]);
            second_sums[2] = simd.mul_add_f64s(*opposite_real, *second_basis, second_sums[2]);
            second_sums[3] = simd.mul_add_f64s(*opposite_imag, *second_basis, second_sums[3]);
        }

        let mut first = [
            simd.reduce_sum_f64s(first_sums[0]),
            simd.reduce_sum_f64s(first_sums[1]),
            simd.reduce_sum_f64s(first_sums[2]),
            simd.reduce_sum_f64s(first_sums[3]),
        ];
        let mut second = [
            simd.reduce_sum_f64s(second_sums[0]),
            simd.reduce_sum_f64s(second_sums[1]),
            simd.reduce_sum_f64s(second_sums[2]),
            simd.reduce_sum_f64s(second_sums[3]),
        ];
        debug_assert_eq!(same_real_tail.len(), same_imag_tail.len());
        debug_assert_eq!(same_real_tail.len(), opposite_real_tail.len());
        debug_assert_eq!(same_real_tail.len(), opposite_imag_tail.len());
        debug_assert_eq!(same_real_tail.len(), first_basis_tail.len());
        debug_assert_eq!(same_real_tail.len(), second_basis_tail.len());
        for lane in 0..same_real_tail.len() {
            first[0] = same_real_tail[lane].mul_add(first_basis_tail[lane], first[0]);
            first[1] = same_imag_tail[lane].mul_add(first_basis_tail[lane], first[1]);
            first[2] = opposite_real_tail[lane].mul_add(first_basis_tail[lane], first[2]);
            first[3] = opposite_imag_tail[lane].mul_add(first_basis_tail[lane], first[3]);
            second[0] = same_real_tail[lane].mul_add(second_basis_tail[lane], second[0]);
            second[1] = same_imag_tail[lane].mul_add(second_basis_tail[lane], second[1]);
            second[2] = opposite_real_tail[lane].mul_add(second_basis_tail[lane], second[2]);
            second[3] = opposite_imag_tail[lane].mul_add(second_basis_tail[lane], second[3]);
        }

        [
            (
                Complex64::new(first[0], first[1]),
                Complex64::new(first[2], first[3]),
            ),
            (
                Complex64::new(second[0], second[1]),
                Complex64::new(second[2], second[3]),
            ),
        ]
    }
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
            let opposite_real_scaled = simd.mul_f64s(*opposite_real, basis);
            let opposite_imag_scaled = simd.mul_f64s(*opposite_imag, basis);
            let opposite_real_1_scaled = simd.mul_f64s(*opposite_real_1, basis);
            let opposite_imag_1_scaled = simd.mul_f64s(*opposite_imag_1, basis);
            sums_0[0] = simd.mul_add_f64s(*same_real, basis, sums_0[0]);
            sums_0[1] = simd.mul_add_f64s(*same_imag, basis, sums_0[1]);
            sums_0[2] = simd.mul_add_f64s(opposite_real_scaled, cosine, sums_0[2]);
            sums_0[3] = simd.mul_add_f64s(opposite_imag_scaled, cosine, sums_0[3]);
            sums_1[0] = simd.mul_add_f64s(*same_real_1, basis, sums_1[0]);
            sums_1[1] = simd.mul_add_f64s(*same_imag_1, basis, sums_1[1]);
            sums_1[2] = simd.mul_add_f64s(opposite_real_1_scaled, cosine, sums_1[2]);
            sums_1[3] = simd.mul_add_f64s(opposite_imag_1_scaled, cosine, sums_1[3]);
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
                let opposite_real_scaled = opposite_real[lane] * basis_tail[lane];
                let opposite_imag_scaled = opposite_imag[lane] * basis_tail[lane];
                results[channel].0.re =
                    same_real[lane].mul_add(basis_tail[lane], results[channel].0.re);
                results[channel].0.im =
                    same_imag[lane].mul_add(basis_tail[lane], results[channel].0.im);
                results[channel].1.re =
                    opposite_real_scaled.mul_add(cosine_tail[lane], results[channel].1.re);
                results[channel].1.im =
                    opposite_imag_scaled.mul_add(cosine_tail[lane], results[channel].1.im);
            }
        }
        results
    }
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool> WithSimd
    for AnalysisPairAccumulation<'_, OPPOSITE_COSINE_WEIGHTED>
{
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
            same_re_even = simd.mul_add_f64s(same_real[0], basis[0], same_re_even);
            same_im_even = simd.mul_add_f64s(same_imag[0], basis[0], same_im_even);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_re_even = simd.mul_add_f64s(opposite_real[0], basis[0], opposite_re_even);
            } else {
                opposite_re_even = simd.add_f64s(
                    opposite_re_even,
                    simd.mul_f64s(simd.mul_f64s(opposite_real[0], basis[0]), cosine[0]),
                );
            }
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_im_even = simd.mul_add_f64s(opposite_imag[0], basis[0], opposite_im_even);
            } else {
                opposite_im_even = simd.add_f64s(
                    opposite_im_even,
                    simd.mul_f64s(simd.mul_f64s(opposite_imag[0], basis[0]), cosine[0]),
                );
            }
            same_re_odd = simd.mul_add_f64s(same_real[1], basis[1], same_re_odd);
            same_im_odd = simd.mul_add_f64s(same_imag[1], basis[1], same_im_odd);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_re_odd = simd.mul_add_f64s(opposite_real[1], basis[1], opposite_re_odd);
                opposite_im_odd = simd.mul_add_f64s(opposite_imag[1], basis[1], opposite_im_odd);
            } else {
                opposite_re_odd = simd.add_f64s(
                    opposite_re_odd,
                    simd.mul_f64s(simd.mul_f64s(opposite_real[1], basis[1]), cosine[1]),
                );
                opposite_im_odd = simd.add_f64s(
                    opposite_im_odd,
                    simd.mul_f64s(simd.mul_f64s(opposite_imag[1], basis[1]), cosine[1]),
                );
            }
        }
        if !basis_remainder.is_empty() {
            let basis = basis_remainder[0];
            let cosine = cosine_remainder[0];
            same_re_even = simd.mul_add_f64s(same_real_remainder[0], basis, same_re_even);
            same_im_even = simd.mul_add_f64s(same_imag_remainder[0], basis, same_im_even);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_re_even =
                    simd.mul_add_f64s(opposite_real_remainder[0], basis, opposite_re_even);
            } else {
                opposite_re_even = simd.add_f64s(
                    opposite_re_even,
                    simd.mul_f64s(simd.mul_f64s(opposite_real_remainder[0], basis), cosine),
                );
            }
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_im_even =
                    simd.mul_add_f64s(opposite_imag_remainder[0], basis, opposite_im_even);
            } else {
                opposite_im_even = simd.add_f64s(
                    opposite_im_even,
                    simd.mul_f64s(simd.mul_f64s(opposite_imag_remainder[0], basis), cosine),
                );
            }
        }
        let mut same_re = simd.reduce_sum_f64s(simd.add_f64s(same_re_even, same_re_odd));
        let mut same_im = simd.reduce_sum_f64s(simd.add_f64s(same_im_even, same_im_odd));
        let mut opposite_re =
            simd.reduce_sum_f64s(simd.add_f64s(opposite_re_even, opposite_re_odd));
        let mut opposite_im =
            simd.reduce_sum_f64s(simd.add_f64s(opposite_im_even, opposite_im_odd));
        for lane in 0..basis_tail.len() {
            same_re = same_real_tail[lane].mul_add(basis_tail[lane], same_re);
            same_im = same_imag_tail[lane].mul_add(basis_tail[lane], same_im);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_re = opposite_real_tail[lane].mul_add(basis_tail[lane], opposite_re);
                opposite_im = opposite_imag_tail[lane].mul_add(basis_tail[lane], opposite_im);
            } else {
                opposite_re += (opposite_real_tail[lane] * basis_tail[lane]) * cosine_tail[lane];
                opposite_im += (opposite_imag_tail[lane] * basis_tail[lane]) * cosine_tail[lane];
            }
        }
        (
            Complex64::new(same_re, same_im),
            Complex64::new(opposite_re, opposite_im),
        )
    }
}

struct MixedAnalysisPairAccumulation<'a, const OPPOSITE_COSINE_WEIGHTED: bool = false> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis: &'a [f64],
    exponents: &'a [i64],
    cos_theta: &'a [f64],
    allow_insignificant_scaled_lanes: bool,
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool> WithSimd
    for MixedAnalysisPairAccumulation<'_, OPPOSITE_COSINE_WEIGHTED>
{
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
            allow_insignificant_scaled_lanes,
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
        let mut vector_needs_scaled_fallback = false;
        for index in 0..basis_vectors.len() {
            let exponent_is_zero = simd.equal_i64s(exponent_vectors[index], zero_exponent);
            let exponent_is_positive =
                simd.greater_than_i64s(exponent_vectors[index], zero_exponent);
            let basis_magnitude = simd.abs_f64s(basis_vectors[index]);
            let basis_is_zero = simd.equal_f64s(basis_magnitude, zeros);
            let basis_is_large_enough = simd.greater_than_or_equal_f64s(basis_magnitude, small);
            let basis_is_small_enough = simd.less_than_or_equal_f64s(basis_magnitude, big);
            let basis_is_normal_or_zero = simd.and_m64s(
                basis_is_small_enough,
                simd.or_m64s(basis_is_zero, basis_is_large_enough),
            );
            if allow_insignificant_scaled_lanes {
                let zero_exponent_needs_fallback =
                    simd.and_m64s(exponent_is_zero, simd.not_m64s(basis_is_normal_or_zero));
                vector_needs_scaled_fallback |= analysis_mask_any(
                    simd,
                    simd.or_m64s(exponent_is_positive, zero_exponent_needs_fallback),
                );
            }
            let normal_lanes = simd.and_m64s(exponent_is_zero, basis_is_normal_or_zero);
            let basis = simd.select_f64s(normal_lanes, basis_vectors[index], zeros);
            let same_real = simd.select_f64s(normal_lanes, same_real_vectors[index], zeros);
            let same_imag = simd.select_f64s(normal_lanes, same_imag_vectors[index], zeros);
            let opposite_real = simd.select_f64s(normal_lanes, opposite_real_vectors[index], zeros);
            let opposite_imag = simd.select_f64s(normal_lanes, opposite_imag_vectors[index], zeros);
            same_re = simd.add_f64s(same_re, simd.mul_f64s(same_real, basis));
            same_im = simd.add_f64s(same_im, simd.mul_f64s(same_imag, basis));
            let opposite_re_product = if OPPOSITE_COSINE_WEIGHTED {
                simd.mul_f64s(opposite_real, basis)
            } else {
                simd.mul_f64s(simd.mul_f64s(opposite_real, basis), cosine_vectors[index])
            };
            let opposite_im_product = if OPPOSITE_COSINE_WEIGHTED {
                simd.mul_f64s(opposite_imag, basis)
            } else {
                simd.mul_f64s(simd.mul_f64s(opposite_imag, basis), cosine_vectors[index])
            };
            opposite_re = simd.add_f64s(opposite_re, opposite_re_product);
            opposite_im = simd.add_f64s(opposite_im, opposite_im_product);
        }

        let mut same = Complex64::new(simd.reduce_sum_f64s(same_re), simd.reduce_sum_f64s(same_im));
        let mut opposite = Complex64::new(
            simd.reduce_sum_f64s(opposite_re),
            simd.reduce_sum_f64s(opposite_im),
        );
        let vector_lane_count = basis.len() - basis_tail.len();
        if !allow_insignificant_scaled_lanes || vector_needs_scaled_fallback {
            for lane in 0..vector_lane_count {
                let needs_scaled_fallback = if allow_insignificant_scaled_lanes {
                    exponents[lane] > 0
                        || (exponents[lane] == 0 && !scalar_value_in_normal_range(basis[lane]))
                } else {
                    exponents[lane] != 0 || !scalar_value_in_normal_range(basis[lane])
                };
                if needs_scaled_fallback {
                    same += scalar::multiply_complex_scaled(
                        Complex64::new(same_real[lane], same_imag[lane]),
                        basis[lane],
                        exponents[lane],
                    );
                    let opposite_value = Complex64::new(opposite_real[lane], opposite_imag[lane]);
                    opposite += if OPPOSITE_COSINE_WEIGHTED {
                        scalar::multiply_complex_scaled(
                            opposite_value,
                            basis[lane],
                            exponents[lane],
                        )
                    } else {
                        scalar::multiply_complex_scaled_product(
                            opposite_value,
                            basis[lane],
                            cos_theta[lane],
                            exponents[lane],
                        )
                    };
                }
            }
        }
        for lane in 0..basis_tail.len() {
            if allow_insignificant_scaled_lanes && exponent_tail[lane] < 0 {
                // The normalized mantissa is at most 2^400 and negative scale
                // epochs are multiples of 800, so this basis contribution is
                // below 2^-400. The caller's latitude-wide budget covers its
                // omission after coefficient accumulation.
                continue;
            } else if exponent_tail[lane] == 0 && scalar_value_in_normal_range(basis_tail[lane]) {
                same.re += same_real_tail[lane] * basis_tail[lane];
                same.im += same_imag_tail[lane] * basis_tail[lane];
                if OPPOSITE_COSINE_WEIGHTED {
                    opposite.re += opposite_real_tail[lane] * basis_tail[lane];
                    opposite.im += opposite_imag_tail[lane] * basis_tail[lane];
                } else {
                    opposite.re +=
                        (opposite_real_tail[lane] * basis_tail[lane]) * cosine_tail[lane];
                    opposite.im +=
                        (opposite_imag_tail[lane] * basis_tail[lane]) * cosine_tail[lane];
                }
            } else {
                same += scalar::multiply_complex_scaled(
                    Complex64::new(same_real_tail[lane], same_imag_tail[lane]),
                    basis_tail[lane],
                    exponent_tail[lane],
                );
                let opposite_value =
                    Complex64::new(opposite_real_tail[lane], opposite_imag_tail[lane]);
                opposite += if OPPOSITE_COSINE_WEIGHTED {
                    scalar::multiply_complex_scaled(
                        opposite_value,
                        basis_tail[lane],
                        exponent_tail[lane],
                    )
                } else {
                    scalar::multiply_complex_scaled_product(
                        opposite_value,
                        basis_tail[lane],
                        cosine_tail[lane],
                        exponent_tail[lane],
                    )
                };
            }
        }
        (same, opposite)
    }
}

struct ScaledAnalysisExceptionVector<'a, S: Simd, const OPPOSITE_COSINE_WEIGHTED: bool> {
    same_real: [S::f64s; 2],
    same_imag: [S::f64s; 2],
    opposite_real: [S::f64s; 2],
    opposite_imag: [S::f64s; 2],
    basis: S::f64s,
    exponents: &'a [i64],
    cosine: S::f64s,
}

#[inline(always)]
fn try_accumulate_scaled_exception_vector<S: Simd, const OPPOSITE_COSINE_WEIGHTED: bool>(
    simd: S,
    vector: ScaledAnalysisExceptionVector<'_, S, OPPOSITE_COSINE_WEIGHTED>,
    results: &mut [(Complex64, Complex64); 2],
) -> bool {
    let ScaledAnalysisExceptionVector {
        same_real,
        same_imag,
        opposite_real,
        opposite_imag,
        basis,
        exponents,
        cosine,
    } = vector;

    if S::F64_LANES != 4 || exponents.len() != 4 {
        return false;
    }

    let mut same_products = [simd.splat_f64s(0.0); 4];
    let mut opposite_products = [simd.splat_f64s(0.0); 4];
    for channel in 0..2 {
        let offset = channel * 2;
        same_products[offset] = simd.mul_f64s(same_real[channel], basis);
        same_products[offset + 1] = simd.mul_f64s(same_imag[channel], basis);
        let opposite_real_product = simd.mul_f64s(opposite_real[channel], basis);
        let opposite_imag_product = simd.mul_f64s(opposite_imag[channel], basis);
        if OPPOSITE_COSINE_WEIGHTED {
            opposite_products[offset] = opposite_real_product;
            opposite_products[offset + 1] = opposite_imag_product;
        } else {
            opposite_products[offset] = simd.mul_f64s(opposite_real_product, cosine);
            opposite_products[offset + 1] = simd.mul_f64s(opposite_imag_product, cosine);
        }
    }

    let zero = simd.splat_f64s(0.0);
    let minimum = simd.splat_f64s(f64::MIN_POSITIVE);
    let maximum = simd.splat_f64s(f64::MAX);
    let mut all_products_normal = simd.equal_f64s(zero, zero);
    for &product in same_products.iter().chain(&opposite_products) {
        let magnitude = simd.abs_f64s(product);
        let normal = simd.and_m64s(
            simd.greater_than_or_equal_f64s(magnitude, minimum),
            simd.less_than_or_equal_f64s(magnitude, maximum),
        );
        all_products_normal = simd.and_m64s(all_products_normal, normal);
    }
    if !analysis_mask_all(simd, all_products_normal) {
        return false;
    }

    let Some(scaled_same) = scale_normal_prefix_products_simd::<S>(same_products, exponents) else {
        return false;
    };
    let Some(scaled_opposite) =
        scale_normal_prefix_products_simd::<S>(opposite_products, exponents)
    else {
        return false;
    };
    let mut same_values = [[0.0; 4]; 4];
    let mut opposite_values = [[0.0; 4]; 4];
    for component in 0..4 {
        simd.partial_store_f64s(&mut same_values[component], scaled_same[component]);
        simd.partial_store_f64s(&mut opposite_values[component], scaled_opposite[component]);
    }

    for lane in 0..4 {
        for (channel, result) in results.iter_mut().enumerate() {
            let component = channel * 2;
            result.0 += Complex64::new(
                same_values[component][lane],
                same_values[component + 1][lane],
            );
            result.1 += Complex64::new(
                opposite_values[component][lane],
                opposite_values[component + 1][lane],
            );
        }
    }
    true
}

/// Accumulates mixed scaled lanes for Q and U while classifying ordinary lanes
/// once and updating both channels in the same vector traversal. When
/// `OPPOSITE_COSINE_WEIGHTED` is true, callers pass spectra already weighted
/// and checked by `prepare_analysis_opposite`, matching the ordinary suffix
/// operation. Each channel keeps four SIMD sums; nonordinary lanes use the
/// existing checked scalar/vector fallback below.
struct MixedAnalysisPairChannelsAccumulation<'a, const OPPOSITE_COSINE_WEIGHTED: bool = false> {
    same_real: [&'a [f64]; 2],
    same_imag: [&'a [f64]; 2],
    opposite_real: [&'a [f64]; 2],
    opposite_imag: [&'a [f64]; 2],
    basis: &'a [f64],
    exponents: &'a [i64],
    cos_theta: &'a [f64],
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool> WithSimd
    for MixedAnalysisPairChannelsAccumulation<'_, OPPOSITE_COSINE_WEIGHTED>
{
    type Output = [(Complex64, Complex64); 2];

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let MixedAnalysisPairChannelsAccumulation {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis,
            exponents,
            cos_theta,
        } = self;
        debug_assert_eq!(basis.len(), exponents.len());
        debug_assert_eq!(basis.len(), cos_theta.len());
        for channel in 0..2 {
            debug_assert_eq!(basis.len(), same_real[channel].len());
            debug_assert_eq!(basis.len(), same_imag[channel].len());
            debug_assert_eq!(basis.len(), opposite_real[channel].len());
            debug_assert_eq!(basis.len(), opposite_imag[channel].len());
        }
        let (basis_vectors, basis_tail) = S::as_simd_f64s(basis);
        let (exponent_vectors, exponent_tail) = S::as_simd_i64s(exponents);
        debug_assert!(basis.len() <= SCALAR_ANALYSIS_PAIR_TILE_SIZE);
        debug_assert_eq!(basis_vectors.len(), exponent_vectors.len());
        debug_assert_eq!(basis_tail.len(), exponent_tail.len());

        let (same_real_0, _) = S::as_simd_f64s(same_real[0]);
        let (same_imag_0, _) = S::as_simd_f64s(same_imag[0]);
        let (opposite_real_0, _) = S::as_simd_f64s(opposite_real[0]);
        let (opposite_imag_0, _) = S::as_simd_f64s(opposite_imag[0]);
        let (same_real_1, _) = S::as_simd_f64s(same_real[1]);
        let (same_imag_1, _) = S::as_simd_f64s(same_imag[1]);
        let (opposite_real_1, _) = S::as_simd_f64s(opposite_real[1]);
        let (opposite_imag_1, _) = S::as_simd_f64s(opposite_imag[1]);
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
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

        let zero_f64 = simd.splat_f64s(0.0);
        let minimum = simd.splat_f64s(scalar::SMALL);
        let maximum = simd.splat_f64s(scalar::BIG);
        let zero_i64 = simd.splat_i64s(0);
        let mut exceptional_vector_lanes = 0_u64;
        let mut has_avx2_lane_masks = true;
        let zeros = simd.splat_f64s(0.0);
        let mut sums_0 = [zeros; 4];
        let mut sums_1 = [zeros; 4];
        for index in 0..basis_vectors.len() {
            let magnitude = simd.abs_f64s(basis_vectors[index]);
            let in_range = simd.and_m64s(
                simd.less_than_or_equal_f64s(magnitude, maximum),
                simd.or_m64s(
                    simd.equal_f64s(magnitude, zero_f64),
                    simd.greater_than_or_equal_f64s(magnitude, minimum),
                ),
            );
            let ordinary =
                simd.and_m64s(simd.equal_i64s(exponent_vectors[index], zero_i64), in_range);
            if let Some(ordinary_lanes) = analysis_mask_lane_bits::<S>(ordinary) {
                exceptional_vector_lanes |= u64::from(!ordinary_lanes & 0b1111) << (index * 4);
            } else {
                has_avx2_lane_masks = false;
            }
            let ordinary_basis = simd.select_f64s(ordinary, basis_vectors[index], zero_f64);
            let cosine = cosine_vectors[index];
            sums_0[0] = simd.add_f64s(sums_0[0], simd.mul_f64s(same_real_0[index], ordinary_basis));
            sums_0[1] = simd.add_f64s(sums_0[1], simd.mul_f64s(same_imag_0[index], ordinary_basis));
            let opposite_real_product = simd.mul_f64s(opposite_real_0[index], ordinary_basis);
            let opposite_imag_product = simd.mul_f64s(opposite_imag_0[index], ordinary_basis);
            let (opposite_real_product, opposite_imag_product) = if OPPOSITE_COSINE_WEIGHTED {
                (opposite_real_product, opposite_imag_product)
            } else {
                (
                    simd.mul_f64s(opposite_real_product, cosine),
                    simd.mul_f64s(opposite_imag_product, cosine),
                )
            };
            sums_0[2] = simd.add_f64s(sums_0[2], opposite_real_product);
            sums_0[3] = simd.add_f64s(sums_0[3], opposite_imag_product);

            sums_1[0] = simd.add_f64s(sums_1[0], simd.mul_f64s(same_real_1[index], ordinary_basis));
            sums_1[1] = simd.add_f64s(sums_1[1], simd.mul_f64s(same_imag_1[index], ordinary_basis));
            let opposite_real_product = simd.mul_f64s(opposite_real_1[index], ordinary_basis);
            let opposite_imag_product = simd.mul_f64s(opposite_imag_1[index], ordinary_basis);
            let (opposite_real_product, opposite_imag_product) = if OPPOSITE_COSINE_WEIGHTED {
                (opposite_real_product, opposite_imag_product)
            } else {
                (
                    simd.mul_f64s(opposite_real_product, cosine),
                    simd.mul_f64s(opposite_imag_product, cosine),
                )
            };
            sums_1[2] = simd.add_f64s(sums_1[2], opposite_real_product);
            sums_1[3] = simd.add_f64s(sums_1[3], opposite_imag_product);
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
            (Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)),
        ];

        results[1] = (
            Complex64::new(
                simd.reduce_sum_f64s(sums_1[0]),
                simd.reduce_sum_f64s(sums_1[1]),
            ),
            Complex64::new(
                simd.reduce_sum_f64s(sums_1[2]),
                simd.reduce_sum_f64s(sums_1[3]),
            ),
        );

        let vector_lane_count = basis.len() - basis_tail.len();
        let accumulate_scaled_lane = |lane: usize, results: &mut [(Complex64, Complex64); 2]| {
            for channel in 0..2 {
                results[channel].0 += scalar::multiply_complex_scaled(
                    Complex64::new(same_real[channel][lane], same_imag[channel][lane]),
                    basis[lane],
                    exponents[lane],
                );
                let opposite =
                    Complex64::new(opposite_real[channel][lane], opposite_imag[channel][lane]);
                results[channel].1 += if OPPOSITE_COSINE_WEIGHTED {
                    scalar::multiply_complex_scaled(opposite, basis[lane], exponents[lane])
                } else {
                    scalar::multiply_complex_scaled_product(
                        opposite,
                        basis[lane],
                        cos_theta[lane],
                        exponents[lane],
                    )
                };
            }
        };
        if has_avx2_lane_masks {
            for vector_index in 0..basis_vectors.len() {
                let vector_lane_start = vector_index * 4;
                let mut vector_exception_mask =
                    ((exceptional_vector_lanes >> (vector_lane_start)) & 0b1111) as u8;
                if vector_exception_mask == 0b1111 {
                    let completed =
                        try_accumulate_scaled_exception_vector::<S, OPPOSITE_COSINE_WEIGHTED>(
                            simd,
                            ScaledAnalysisExceptionVector::<S, OPPOSITE_COSINE_WEIGHTED> {
                                same_real: [same_real_0[vector_index], same_real_1[vector_index]],
                                same_imag: [same_imag_0[vector_index], same_imag_1[vector_index]],
                                opposite_real: [
                                    opposite_real_0[vector_index],
                                    opposite_real_1[vector_index],
                                ],
                                opposite_imag: [
                                    opposite_imag_0[vector_index],
                                    opposite_imag_1[vector_index],
                                ],
                                basis: basis_vectors[vector_index],
                                exponents: &exponents[vector_lane_start..vector_lane_start + 4],
                                cosine: cosine_vectors[vector_index],
                            },
                            &mut results,
                        );
                    if !completed {
                        for lane in vector_lane_start..vector_lane_start + 4 {
                            debug_assert!(lane < vector_lane_count);
                            accumulate_scaled_lane(lane, &mut results);
                        }
                    }
                } else {
                    while vector_exception_mask != 0 {
                        let lane_offset = vector_exception_mask.trailing_zeros() as usize;
                        vector_exception_mask &= vector_exception_mask - 1;
                        let lane = vector_lane_start + lane_offset;
                        debug_assert!(lane < vector_lane_count);
                        accumulate_scaled_lane(lane, &mut results);
                    }
                }
            }
        } else {
            for lane in 0..vector_lane_count {
                if exponents[lane] == 0 && scalar_value_in_normal_range(basis[lane]) {
                    continue;
                }
                accumulate_scaled_lane(lane, &mut results);
            }
        }
        for lane in 0..basis_tail.len() {
            if exponent_tail[lane] == 0 && scalar_value_in_normal_range(basis_tail[lane]) {
                for channel in 0..2 {
                    results[channel].0.re +=
                        same_real[channel][vector_lane_count + lane] * basis_tail[lane];
                    results[channel].0.im +=
                        same_imag[channel][vector_lane_count + lane] * basis_tail[lane];
                    if OPPOSITE_COSINE_WEIGHTED {
                        results[channel].1.re +=
                            opposite_real[channel][vector_lane_count + lane] * basis_tail[lane];
                        results[channel].1.im +=
                            opposite_imag[channel][vector_lane_count + lane] * basis_tail[lane];
                    } else {
                        results[channel].1.re += (opposite_real[channel][vector_lane_count + lane]
                            * basis_tail[lane])
                            * cosine_tail[lane];
                        results[channel].1.im += (opposite_imag[channel][vector_lane_count + lane]
                            * basis_tail[lane])
                            * cosine_tail[lane];
                    }
                }
            } else {
                for channel in 0..2 {
                    results[channel].0 += scalar::multiply_complex_scaled(
                        Complex64::new(
                            same_real[channel][vector_lane_count + lane],
                            same_imag[channel][vector_lane_count + lane],
                        ),
                        basis_tail[lane],
                        exponent_tail[lane],
                    );
                    let opposite = Complex64::new(
                        opposite_real[channel][vector_lane_count + lane],
                        opposite_imag[channel][vector_lane_count + lane],
                    );
                    results[channel].1 += if OPPOSITE_COSINE_WEIGHTED {
                        scalar::multiply_complex_scaled(
                            opposite,
                            basis_tail[lane],
                            exponent_tail[lane],
                        )
                    } else {
                        scalar::multiply_complex_scaled_product(
                            opposite,
                            basis_tail[lane],
                            cosine_tail[lane],
                            exponent_tail[lane],
                        )
                    };
                }
            }
        }
        results
    }
}

struct FusedMixedAnalysisPairStep<'a, const OPPOSITE_COSINE_WEIGHTED: bool> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis_previous: &'a [f64],
    basis_current: &'a [f64],
    cos_theta: &'a [f64],
    cos_theta_squared: &'a [f64],
    scale_corrections: &'a [f64],
    coefficient_a: f64,
    coefficient_b: f64,
    basis_next: &'a mut [f64],
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool> WithSimd
    for FusedMixedAnalysisPairStep<'_, OPPOSITE_COSINE_WEIGHTED>
{
    type Output = (Complex64, Complex64);

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedMixedAnalysisPairStep {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis_previous,
            basis_current,
            cos_theta,
            cos_theta_squared,
            scale_corrections,
            coefficient_a,
            coefficient_b,
            basis_next,
        } = self;
        let (same_real_vectors, same_real_tail) = S::as_simd_f64s(same_real);
        let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(same_imag);
        let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(opposite_real);
        let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(opposite_imag);
        let (previous_vectors, previous_tail) = S::as_simd_f64s(basis_previous);
        let (current_vectors, current_tail) = S::as_simd_f64s(basis_current);
        let (correction_vectors, correction_tail) = S::as_simd_f64s(scale_corrections);
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
        let (cosine_squared_vectors, cosine_squared_tail) = S::as_simd_f64s(cos_theta_squared);
        let (next_vectors, next_tail) = S::as_mut_simd_f64s(basis_next);
        debug_assert_eq!(same_real_vectors.len(), current_vectors.len());
        debug_assert_eq!(same_imag_vectors.len(), current_vectors.len());
        debug_assert_eq!(opposite_real_vectors.len(), current_vectors.len());
        debug_assert_eq!(opposite_imag_vectors.len(), current_vectors.len());
        debug_assert_eq!(previous_vectors.len(), current_vectors.len());
        debug_assert_eq!(correction_vectors.len(), current_vectors.len());
        debug_assert_eq!(cosine_vectors.len(), current_vectors.len());
        debug_assert_eq!(cosine_squared_vectors.len(), current_vectors.len());
        debug_assert_eq!(next_vectors.len(), current_vectors.len());
        debug_assert_eq!(same_real_tail.len(), current_tail.len());
        debug_assert_eq!(same_imag_tail.len(), current_tail.len());
        debug_assert_eq!(opposite_real_tail.len(), current_tail.len());
        debug_assert_eq!(opposite_imag_tail.len(), current_tail.len());
        debug_assert_eq!(previous_tail.len(), current_tail.len());
        debug_assert_eq!(correction_tail.len(), current_tail.len());
        debug_assert_eq!(cosine_tail.len(), current_tail.len());
        debug_assert_eq!(cosine_squared_tail.len(), current_tail.len());
        debug_assert_eq!(next_tail.len(), current_tail.len());

        let zero = simd.splat_f64s(0.0);
        let a = simd.splat_f64s(coefficient_a);
        let b = simd.splat_f64s(coefficient_b);
        let mut same_re = zero;
        let mut same_im = zero;
        let mut opposite_re = zero;
        let mut opposite_im = zero;

        // Apply the per-lane scale correction while accumulating this degree
        // pair, then advance the recurrence state for the next pair.
        for index in 0..current_vectors.len() {
            let basis = simd.mul_f64s(current_vectors[index], correction_vectors[index]);
            same_re = simd.mul_add_f64s(same_real_vectors[index], basis, same_re);
            same_im = simd.mul_add_f64s(same_imag_vectors[index], basis, same_im);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_re = simd.mul_add_f64s(opposite_real_vectors[index], basis, opposite_re);
                opposite_im = simd.mul_add_f64s(opposite_imag_vectors[index], basis, opposite_im);
            } else {
                opposite_re = simd.mul_add_f64s(
                    simd.mul_f64s(opposite_real_vectors[index], cosine_vectors[index]),
                    basis,
                    opposite_re,
                );
                opposite_im = simd.mul_add_f64s(
                    simd.mul_f64s(opposite_imag_vectors[index], cosine_vectors[index]),
                    basis,
                    opposite_im,
                );
            }
            let leading = simd.mul_add_f64s(a, cosine_squared_vectors[index], b);
            next_vectors[index] =
                simd.mul_add_f64s(leading, current_vectors[index], previous_vectors[index]);
        }

        let mut same = Complex64::new(simd.reduce_sum_f64s(same_re), simd.reduce_sum_f64s(same_im));
        let mut opposite = Complex64::new(
            simd.reduce_sum_f64s(opposite_re),
            simd.reduce_sum_f64s(opposite_im),
        );
        for lane in 0..current_tail.len() {
            let basis = current_tail[lane] * correction_tail[lane];
            same.re = same_real_tail[lane].mul_add(basis, same.re);
            same.im = same_imag_tail[lane].mul_add(basis, same.im);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite.re = opposite_real_tail[lane].mul_add(basis, opposite.re);
                opposite.im = opposite_imag_tail[lane].mul_add(basis, opposite.im);
            } else {
                opposite.re =
                    (opposite_real_tail[lane] * cosine_tail[lane]).mul_add(basis, opposite.re);
                opposite.im =
                    (opposite_imag_tail[lane] * cosine_tail[lane]).mul_add(basis, opposite.im);
            }
            let leading = coefficient_a.mul_add(cosine_squared_tail[lane], coefficient_b);
            next_tail[lane] = leading.mul_add(current_tail[lane], previous_tail[lane]);
        }
        (same, opposite)
    }
}

struct FusedMixedAnalysisPairAccumulation<'a, const OPPOSITE_COSINE_WEIGHTED: bool> {
    same_real: &'a [f64],
    same_imag: &'a [f64],
    opposite_real: &'a [f64],
    opposite_imag: &'a [f64],
    basis_current: &'a [f64],
    cos_theta: &'a [f64],
    scale_corrections: &'a [f64],
}

impl<const OPPOSITE_COSINE_WEIGHTED: bool> WithSimd
    for FusedMixedAnalysisPairAccumulation<'_, OPPOSITE_COSINE_WEIGHTED>
{
    type Output = (Complex64, Complex64);

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let FusedMixedAnalysisPairAccumulation {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis_current,
            cos_theta,
            scale_corrections,
        } = self;
        let (same_real_vectors, same_real_tail) = S::as_simd_f64s(same_real);
        let (same_imag_vectors, same_imag_tail) = S::as_simd_f64s(same_imag);
        let (opposite_real_vectors, opposite_real_tail) = S::as_simd_f64s(opposite_real);
        let (opposite_imag_vectors, opposite_imag_tail) = S::as_simd_f64s(opposite_imag);
        let (basis_vectors, basis_tail) = S::as_simd_f64s(basis_current);
        let (correction_vectors, correction_tail) = S::as_simd_f64s(scale_corrections);
        let (cosine_vectors, cosine_tail) = S::as_simd_f64s(cos_theta);
        let zero = simd.splat_f64s(0.0);
        let mut same_re = zero;
        let mut same_im = zero;
        let mut opposite_re = zero;
        let mut opposite_im = zero;
        for index in 0..basis_vectors.len() {
            let correction = correction_vectors[index];
            let basis = simd.mul_f64s(basis_vectors[index], correction);
            same_re = simd.mul_add_f64s(same_real_vectors[index], basis, same_re);
            same_im = simd.mul_add_f64s(same_imag_vectors[index], basis, same_im);
            let opposite_re_coefficient = opposite_real_vectors[index];
            let opposite_im_coefficient = opposite_imag_vectors[index];
            if OPPOSITE_COSINE_WEIGHTED {
                opposite_re = simd.mul_add_f64s(opposite_re_coefficient, basis, opposite_re);
                opposite_im = simd.mul_add_f64s(opposite_im_coefficient, basis, opposite_im);
            } else {
                opposite_re = simd.mul_add_f64s(
                    simd.mul_f64s(opposite_re_coefficient, cosine_vectors[index]),
                    basis,
                    opposite_re,
                );
                opposite_im = simd.mul_add_f64s(
                    simd.mul_f64s(opposite_im_coefficient, cosine_vectors[index]),
                    basis,
                    opposite_im,
                );
            }
        }
        let mut same = Complex64::new(simd.reduce_sum_f64s(same_re), simd.reduce_sum_f64s(same_im));
        let mut opposite = Complex64::new(
            simd.reduce_sum_f64s(opposite_re),
            simd.reduce_sum_f64s(opposite_im),
        );
        for lane in 0..basis_tail.len() {
            let correction = correction_tail[lane];
            let basis = basis_tail[lane] * correction;
            same.re = same_real_tail[lane].mul_add(basis, same.re);
            same.im = same_imag_tail[lane].mul_add(basis, same.im);
            if OPPOSITE_COSINE_WEIGHTED {
                opposite.re = opposite_real_tail[lane].mul_add(basis, opposite.re);
                opposite.im = opposite_imag_tail[lane].mul_add(basis, opposite.im);
            } else {
                opposite.re =
                    (opposite_real_tail[lane] * cosine_tail[lane]).mul_add(basis, opposite.re);
                opposite.im =
                    (opposite_imag_tail[lane] * cosine_tail[lane]).mul_add(basis, opposite.im);
            }
        }
        (same, opposite)
    }
}

#[cfg(test)]
mod tests;
