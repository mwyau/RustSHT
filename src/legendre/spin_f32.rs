// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * The f32 spin recurrence follows DUCC's normalized spin recurrence in
 * src/ducc0/sht/sht_inner_loop.h, revision
 * c1ed01b8fda69d2a378190f0915f1a19762dddbf.
 * Its binary scaling keeps recurrence values representable in f32 while
 * preserving the positive and negative spin sequences.
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

use std::f32::consts::PI;
#[cfg(all(test, rustsht_profile_spin32_normalizer))]
use std::sync::atomic::{AtomicU64, Ordering};

use num_complex::Complex32;
use pulp::{Arch, Simd, WithSimd};

use crate::error::ShtError;
use crate::legendre::{LATITUDE_BLOCK_SIZE, scalar_f32::scale_binary};

const BIG: f32 = f32::from_bits(167_u32 << 23);
const SMALL: f32 = f32::from_bits(87_u32 << 23);
const SCALE_UP: f32 = f32::from_bits(207_u32 << 23);
const SCALE_DOWN: f32 = f32::from_bits(47_u32 << 23);

#[cfg(all(test, rustsht_profile_spin32_normalizer))]
static NORMALIZER_SCALED_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(all(test, rustsht_profile_spin32_normalizer))]
static NORMALIZER_SCALED_LANES: AtomicU64 = AtomicU64::new(0);
#[cfg(all(test, rustsht_profile_spin32_normalizer))]
static NORMALIZER_SCALE_STEPS: AtomicU64 = AtomicU64::new(0);
#[cfg(all(test, rustsht_profile_spin32_normalizer))]
static NORMALIZER_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(all(test, rustsht_profile_spin32_normalizer))]
static NORMALIZER_LANES: AtomicU64 = AtomicU64::new(0);

#[cfg(all(test, rustsht_profile_spin32_normalizer))]
pub(crate) fn reset_normalizer_profile() {
    NORMALIZER_SCALED_CALLS.store(0, Ordering::Relaxed);
    NORMALIZER_SCALED_LANES.store(0, Ordering::Relaxed);
    NORMALIZER_SCALE_STEPS.store(0, Ordering::Relaxed);
    NORMALIZER_CALLS.store(0, Ordering::Relaxed);
    NORMALIZER_LANES.store(0, Ordering::Relaxed);
}

#[cfg(all(test, rustsht_profile_spin32_normalizer))]
pub(crate) fn normalizer_profile() -> (u64, u64, u64, u64, u64) {
    (
        NORMALIZER_SCALED_CALLS.load(Ordering::Relaxed),
        NORMALIZER_SCALED_LANES.load(Ordering::Relaxed),
        NORMALIZER_SCALE_STEPS.load(Ordering::Relaxed),
        NORMALIZER_CALLS.load(Ordering::Relaxed),
        NORMALIZER_LANES.load(Ordering::Relaxed),
    )
}

#[derive(Clone, Copy)]
struct ScaledValue {
    mantissa: f32,
    exponent: i64,
}

pub(crate) struct SpinInitial32 {
    pub(crate) start_degree: usize,
    cos_power: usize,
    sin_power: usize,
    plus_sign: f32,
    minus_sign: f32,
    prefactor: ScaledValue,
}

pub(crate) fn fill_spin_factors(
    spin: usize,
    m: usize,
    lmax: usize,
    alpha: &mut [f32],
    coefficient_a: &mut [f32],
    coefficient_b: &mut [f32],
) -> Result<SpinInitial32, ShtError> {
    debug_assert!(spin > 0 && spin <= lmax && m <= lmax);
    debug_assert!(alpha.len() > lmax + 1);
    debug_assert!(coefficient_a.len() > lmax + 1);
    debug_assert!(coefficient_b.len() > lmax + 1);

    let mlo = spin.min(m);
    let mhi = spin.max(m);
    alpha.fill(0.0);
    coefficient_a.fill(0.0);
    coefficient_b.fill(0.0);
    alpha[mhi] = 1.0;

    for l in mhi..=lmax {
        let lf = l as f32;
        let mf = m as f32;
        let sf = spin as f32;
        let inv_product = ((lf + mf + 1.0).recip()
            * (lf - mf + 1.0).recip()
            * (lf + sf + 1.0).recip()
            * (lf - sf + 1.0).recip())
        .sqrt();
        let leading = (lf + 1.0) * (2.0 * lf + 1.0) * inv_product;
        let coupling = if l == 0 {
            0.0
        } else {
            mf * sf / (lf * (lf + 1.0))
        };

        if l > mhi {
            let ratio = (((lf + mf) * (lf - mf) * (lf + sf) * (lf - sf))
                / ((lf + mf + 1.0) * (lf - mf + 1.0) * (lf + sf + 1.0) * (lf - sf + 1.0)))
                .sqrt()
                * ((lf + 1.0) / lf);
            alpha[l + 1] = alpha[l - 1] * ratio;
        } else {
            alpha[l + 1] = 1.0;
        }

        let a = leading * alpha[l] / alpha[l + 1];
        coefficient_a[l + 1] = a;
        coefficient_b[l + 1] = coupling * a;
    }

    let mut prefactor = ScaledValue {
        mantissa: 1.0,
        exponent: 0,
    };
    let binomial_lower = mhi - mlo;
    for i in 1..=binomial_lower {
        let numerator = mhi
            .checked_add(mlo)
            .and_then(|value| value.checked_add(i))
            .ok_or(ShtError::IntegerOverflow {
                quantity: "spin recurrence binomial factor",
            })?;
        prefactor = normalize(
            prefactor.mantissa * ((numerator as f32) / (i as f32)).sqrt(),
            prefactor.exponent,
        )?;
    }

    let (cos_power, sin_power, plus_sign, minus_sign) = if mhi == m {
        (
            mhi.checked_add(spin).ok_or(ShtError::IntegerOverflow {
                quantity: "spin recurrence half-angle power",
            })?,
            mhi - spin,
            parity_sign(m),
            parity_sign(mhi - spin),
        )
    } else {
        let combined = mhi.checked_add(m).ok_or(ShtError::IntegerOverflow {
            quantity: "spin recurrence half-angle power",
        })?;
        (combined, mhi - m, parity_sign(spin), parity_sign(combined))
    };

    Ok(SpinInitial32 {
        start_degree: mhi,
        cos_power,
        sin_power,
        plus_sign,
        minus_sign,
        prefactor,
    })
}

pub(crate) fn analysis_inputs_vectorizable(
    m: usize,
    q_real: &[f32],
    q_imag: &[f32],
    u_real: &[f32],
    u_imag: &[f32],
    ring_order_limits: &[usize],
) -> bool {
    debug_assert_eq!(q_real.len(), q_imag.len());
    debug_assert_eq!(q_real.len(), u_real.len());
    debug_assert_eq!(q_real.len(), u_imag.len());
    debug_assert_eq!(q_real.len(), ring_order_limits.len());
    ring_order_limits.iter().all(|&limit| m <= limit)
        && q_real.iter().all(|&value| is_normal_or_zero(value))
        && q_imag.iter().all(|&value| is_normal_or_zero(value))
        && u_real.iter().all(|&value| is_normal_or_zero(value))
        && u_imag.iter().all(|&value| is_normal_or_zero(value))
}

#[inline]
fn is_normal_or_zero(value: f32) -> bool {
    value == 0.0 || value.is_normal()
}

#[expect(
    clippy::too_many_arguments,
    reason = "Spin analysis accumulation receives lane spectra, two scaled bases and cutoff metadata."
)]
pub(crate) fn accumulate_analysis_dispatched(
    arch: Arch,
    vectorizable_inputs: bool,
    m: usize,
    q_real: &[f32],
    q_imag: &[f32],
    u_real: &[f32],
    u_imag: &[f32],
    plus: &[f32],
    plus_exponents: &[i64],
    minus: &[f32],
    minus_exponents: &[i64],
    alpha_norm: f32,
    ring_order_limits: &[usize],
    include_b: bool,
) -> (Complex32, Complex32) {
    debug_assert_eq!(q_real.len(), plus.len());
    debug_assert_eq!(q_imag.len(), plus.len());
    debug_assert_eq!(u_real.len(), plus.len());
    debug_assert_eq!(u_imag.len(), plus.len());
    debug_assert_eq!(plus_exponents.len(), plus.len());
    debug_assert_eq!(minus.len(), plus.len());
    debug_assert_eq!(minus_exponents.len(), plus.len());
    debug_assert_eq!(ring_order_limits.len(), plus.len());

    let vectorizable = vectorizable_inputs
        && alpha_norm.is_normal()
        && plus_exponents.iter().all(|&exponent| exponent == 0)
        && minus_exponents.iter().all(|&exponent| exponent == 0)
        && plus.iter().all(|&value| is_normal_or_zero(value))
        && minus.iter().all(|&value| is_normal_or_zero(value));
    if vectorizable {
        let simd_sums = arch.dispatch(SpinAnalysisAccumulation32 {
            q_real,
            q_imag,
            u_real,
            u_imag,
            plus,
            minus,
            alpha_norm,
            include_b,
        });
        if [
            simd_sums.0.re,
            simd_sums.0.im,
            simd_sums.1.re,
            simd_sums.1.im,
        ]
        .into_iter()
        .all(f32::is_finite)
        {
            return simd_sums;
        }
    }

    // SIMD lane partials can overflow even when each input and basis value is
    // finite. Recompute nonfinite results in the scalar accumulation order.
    let mut e_sum = Complex32::new(0.0, 0.0);
    let mut b_sum = Complex32::new(0.0, 0.0);
    for lane in 0..plus.len() {
        if m > ring_order_limits[lane] {
            continue;
        }
        let plus_value = scaled_basis_value(plus[lane], plus_exponents[lane], alpha_norm);
        let minus_value = scaled_basis_value(minus[lane], minus_exponents[lane], alpha_norm);
        let even_basis = plus_value + minus_value;
        let odd_basis = plus_value - minus_value;
        e_sum.re += q_real[lane] * even_basis + u_imag[lane] * odd_basis;
        e_sum.im += q_imag[lane] * even_basis - u_real[lane] * odd_basis;
        if include_b {
            b_sum.re += u_real[lane] * even_basis - q_imag[lane] * odd_basis;
            b_sum.im += u_imag[lane] * even_basis + q_real[lane] * odd_basis;
        }
    }
    (e_sum, b_sum)
}

struct SpinAnalysisAccumulation32<'a> {
    q_real: &'a [f32],
    q_imag: &'a [f32],
    u_real: &'a [f32],
    u_imag: &'a [f32],
    plus: &'a [f32],
    minus: &'a [f32],
    alpha_norm: f32,
    include_b: bool,
}

impl WithSimd for SpinAnalysisAccumulation32<'_> {
    type Output = (Complex32, Complex32);

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self {
            q_real,
            q_imag,
            u_real,
            u_imag,
            plus,
            minus,
            alpha_norm,
            include_b,
        } = self;
        let (q_real_vectors, q_real_tail) = S::as_simd_f32s(q_real);
        let (q_imag_vectors, q_imag_tail) = S::as_simd_f32s(q_imag);
        let (u_real_vectors, u_real_tail) = S::as_simd_f32s(u_real);
        let (u_imag_vectors, u_imag_tail) = S::as_simd_f32s(u_imag);
        let (plus_vectors, plus_tail) = S::as_simd_f32s(plus);
        let (minus_vectors, minus_tail) = S::as_simd_f32s(minus);
        let vector_count = plus_vectors.len();
        debug_assert_eq!(q_real_vectors.len(), vector_count);
        debug_assert_eq!(q_imag_vectors.len(), vector_count);
        debug_assert_eq!(u_real_vectors.len(), vector_count);
        debug_assert_eq!(u_imag_vectors.len(), vector_count);
        debug_assert_eq!(minus_vectors.len(), vector_count);

        let alpha = simd.splat_f32s(alpha_norm);
        let mut e_real_sum = simd.splat_f32s(0.0);
        let mut e_imag_sum = simd.splat_f32s(0.0);
        let mut b_real_sum = simd.splat_f32s(0.0);
        let mut b_imag_sum = simd.splat_f32s(0.0);
        for index in 0..vector_count {
            let plus_value = simd.mul_f32s(plus_vectors[index], alpha);
            let minus_value = simd.mul_f32s(minus_vectors[index], alpha);
            let even_basis = simd.add_f32s(plus_value, minus_value);
            let odd_basis = simd.sub_f32s(plus_value, minus_value);
            e_real_sum = simd.add_f32s(
                e_real_sum,
                simd.add_f32s(
                    simd.mul_f32s(q_real_vectors[index], even_basis),
                    simd.mul_f32s(u_imag_vectors[index], odd_basis),
                ),
            );
            e_imag_sum = simd.add_f32s(
                e_imag_sum,
                simd.sub_f32s(
                    simd.mul_f32s(q_imag_vectors[index], even_basis),
                    simd.mul_f32s(u_real_vectors[index], odd_basis),
                ),
            );
            if include_b {
                b_real_sum = simd.add_f32s(
                    b_real_sum,
                    simd.sub_f32s(
                        simd.mul_f32s(u_real_vectors[index], even_basis),
                        simd.mul_f32s(q_imag_vectors[index], odd_basis),
                    ),
                );
                b_imag_sum = simd.add_f32s(
                    b_imag_sum,
                    simd.add_f32s(
                        simd.mul_f32s(u_imag_vectors[index], even_basis),
                        simd.mul_f32s(q_real_vectors[index], odd_basis),
                    ),
                );
            }
        }

        let mut e_real_tail_sum = 0.0_f32;
        let mut e_imag_tail_sum = 0.0_f32;
        let mut b_real_tail_sum = 0.0_f32;
        let mut b_imag_tail_sum = 0.0_f32;
        for lane in 0..plus_tail.len() {
            let plus_value = plus_tail[lane] * alpha_norm;
            let minus_value = minus_tail[lane] * alpha_norm;
            let even_basis = plus_value + minus_value;
            let odd_basis = plus_value - minus_value;
            e_real_tail_sum += q_real_tail[lane] * even_basis + u_imag_tail[lane] * odd_basis;
            e_imag_tail_sum += q_imag_tail[lane] * even_basis - u_real_tail[lane] * odd_basis;
            if include_b {
                b_real_tail_sum += u_real_tail[lane] * even_basis - q_imag_tail[lane] * odd_basis;
                b_imag_tail_sum += u_imag_tail[lane] * even_basis + q_real_tail[lane] * odd_basis;
            }
        }

        (
            Complex32::new(
                simd.reduce_sum_f32s(e_real_sum) + e_real_tail_sum,
                simd.reduce_sum_f32s(e_imag_sum) + e_imag_tail_sum,
            ),
            Complex32::new(
                simd.reduce_sum_f32s(b_real_sum) + b_real_tail_sum,
                simd.reduce_sum_f32s(b_imag_sum) + b_imag_tail_sum,
            ),
        )
    }
}

const fn parity_sign(value: usize) -> f32 {
    if value.is_multiple_of(2) { 1.0 } else { -1.0 }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The recurrence receives its plan factors and caller-owned lane scratch explicitly."
)]
pub(crate) fn for_each_spin_lm_dispatched<F>(
    arch: Arch,
    initial: &SpinInitial32,
    lmax: usize,
    sin_theta: &[f32],
    cos_theta: &[f32],
    coefficient_a: &[f32],
    coefficient_b: &[f32],
    previous_plus: &mut [f32],
    current_plus: &mut [f32],
    next_plus: &mut [f32],
    exponent_plus: &mut [i64],
    previous_minus: &mut [f32],
    current_minus: &mut [f32],
    next_minus: &mut [f32],
    exponent_minus: &mut [i64],
    mut visit: F,
) -> Result<(), ShtError>
where
    F: FnMut(usize, usize, &[f32], &[i64], &[f32], &[i64]) -> Result<(), ShtError>,
{
    debug_assert_eq!(sin_theta.len(), cos_theta.len());
    let scratch_len = sin_theta.len().min(LATITUDE_BLOCK_SIZE);
    for values in [
        &*previous_plus,
        &*current_plus,
        &*next_plus,
        &*previous_minus,
        &*current_minus,
        &*next_minus,
    ] {
        debug_assert!(values.len() >= scratch_len);
    }
    debug_assert!(exponent_plus.len() >= scratch_len);
    debug_assert!(exponent_minus.len() >= scratch_len);

    for ring_start in (0..sin_theta.len()).step_by(LATITUDE_BLOCK_SIZE) {
        let ring_end = (ring_start + LATITUDE_BLOCK_SIZE).min(sin_theta.len());
        let block_len = ring_end - ring_start;
        arch.dispatch(ForEachSpin32 {
            ring_start,
            initial,
            lmax,
            cos_theta: &cos_theta[ring_start..ring_end],
            coefficient_a,
            coefficient_b,
            previous_plus: &mut previous_plus[..block_len],
            current_plus: &mut current_plus[..block_len],
            next_plus: &mut next_plus[..block_len],
            exponent_plus: &mut exponent_plus[..block_len],
            previous_minus: &mut previous_minus[..block_len],
            current_minus: &mut current_minus[..block_len],
            next_minus: &mut next_minus[..block_len],
            exponent_minus: &mut exponent_minus[..block_len],
            visit: &mut visit,
        })?;
    }
    Ok(())
}

struct ForEachSpin32<'a, F> {
    ring_start: usize,
    initial: &'a SpinInitial32,
    lmax: usize,
    cos_theta: &'a [f32],
    coefficient_a: &'a [f32],
    coefficient_b: &'a [f32],
    previous_plus: &'a mut [f32],
    current_plus: &'a mut [f32],
    next_plus: &'a mut [f32],
    exponent_plus: &'a mut [i64],
    previous_minus: &'a mut [f32],
    current_minus: &'a mut [f32],
    next_minus: &'a mut [f32],
    exponent_minus: &'a mut [i64],
    visit: F,
}

impl<F> WithSimd for ForEachSpin32<'_, F>
where
    F: FnMut(usize, usize, &[f32], &[i64], &[f32], &[i64]) -> Result<(), ShtError>,
{
    type Output = Result<(), ShtError>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let ForEachSpin32 {
            ring_start,
            initial,
            lmax,
            cos_theta,
            coefficient_a,
            coefficient_b,
            previous_plus,
            current_plus,
            next_plus,
            exponent_plus,
            previous_minus,
            current_minus,
            next_minus,
            exponent_minus,
            mut visit,
        } = self;
        debug_assert_eq!(cos_theta.len(), previous_plus.len());
        debug_assert_eq!(cos_theta.len(), current_plus.len());
        debug_assert_eq!(cos_theta.len(), next_plus.len());
        debug_assert_eq!(cos_theta.len(), exponent_plus.len());
        debug_assert_eq!(cos_theta.len(), previous_minus.len());
        debug_assert_eq!(cos_theta.len(), current_minus.len());
        debug_assert_eq!(cos_theta.len(), next_minus.len());
        debug_assert_eq!(cos_theta.len(), exponent_minus.len());

        for lane in 0..cos_theta.len() {
            let half_cosine = ((1.0 + cos_theta[lane]) * 0.5).sqrt().max(1e-15);
            let half_sine = ((1.0 - cos_theta[lane]) * 0.5).sqrt().max(1e-15);
            let plus = multiply(
                initial.prefactor,
                multiply(
                    power(half_cosine, initial.cos_power)?,
                    power(half_sine, initial.sin_power)?,
                )?,
            )?;
            let minus = multiply(
                initial.prefactor,
                multiply(
                    power(half_cosine, initial.sin_power)?,
                    power(half_sine, initial.cos_power)?,
                )?,
            )?;
            previous_plus[lane] = 0.0;
            current_plus[lane] = initial.plus_sign * plus.mantissa;
            exponent_plus[lane] = plus.exponent;
            previous_minus[lane] = 0.0;
            current_minus[lane] = initial.minus_sign * minus.mantissa;
            exponent_minus[lane] = minus.exponent;
        }

        let mut previous_plus = previous_plus;
        let mut current_plus = current_plus;
        let mut next_plus = next_plus;
        let mut previous_minus = previous_minus;
        let mut current_minus = current_minus;
        let mut next_minus = next_minus;
        for l in initial.start_degree..=lmax {
            visit(
                l,
                ring_start,
                current_plus,
                exponent_plus,
                current_minus,
                exponent_minus,
            )?;
            if l == lmax {
                break;
            }
            recurrence_step(
                simd,
                cos_theta,
                coefficient_a[l + 1],
                coefficient_b[l + 1],
                current_plus,
                previous_plus,
                next_plus,
                false,
            );
            recurrence_step(
                simd,
                cos_theta,
                coefficient_a[l + 1],
                coefficient_b[l + 1],
                current_minus,
                previous_minus,
                next_minus,
                true,
            );
            normalize_pair_lanes(current_plus, next_plus, exponent_plus)?;
            normalize_pair_lanes(current_minus, next_minus, exponent_minus)?;
            std::mem::swap(&mut previous_plus, &mut current_plus);
            std::mem::swap(&mut current_plus, &mut next_plus);
            std::mem::swap(&mut previous_minus, &mut current_minus);
            std::mem::swap(&mut current_minus, &mut next_minus);
        }
        Ok(())
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The recurrence step receives lane vectors and independent spin coefficients explicitly."
)]
#[inline(always)]
fn recurrence_step<S: Simd>(
    simd: S,
    cosine: &[f32],
    coefficient_a: f32,
    coefficient_b: f32,
    current: &[f32],
    previous: &[f32],
    next: &mut [f32],
    add_coupling: bool,
) {
    let coefficient_b = if add_coupling {
        -coefficient_b
    } else {
        coefficient_b
    };
    let (current_vectors, current_tail) = S::as_simd_f32s(current);
    let (previous_vectors, previous_tail) = S::as_simd_f32s(previous);
    let (cosine_vectors, cosine_tail) = S::as_simd_f32s(cosine);
    let (next_vectors, next_tail) = S::as_mut_simd_f32s(next);
    let coupling = simd.splat_f32s(-coefficient_b);
    for index in 0..current_vectors.len() {
        let leading = simd.mul_add_f32s(
            simd.splat_f32s(coefficient_a),
            cosine_vectors[index],
            coupling,
        );
        next_vectors[index] = simd.mul_add_f32s(
            leading,
            current_vectors[index],
            simd.neg_f32s(previous_vectors[index]),
        );
    }
    for index in 0..current_tail.len() {
        let leading = coefficient_a.mul_add(cosine_tail[index], -coefficient_b);
        next_tail[index] = leading.mul_add(current_tail[index], -previous_tail[index]);
    }
}

fn normalize_pair_lanes(
    previous: &mut [f32],
    current: &mut [f32],
    exponents: &mut [i64],
) -> Result<(), ShtError> {
    #[cfg(all(test, rustsht_profile_spin32_normalizer))]
    let mut scaled_lanes = 0_u64;
    #[cfg(all(test, rustsht_profile_spin32_normalizer))]
    let mut scale_steps = 0_u64;
    #[cfg(all(test, rustsht_profile_spin32_normalizer))]
    {
        NORMALIZER_CALLS.fetch_add(1, Ordering::Relaxed);
        NORMALIZER_LANES.fetch_add(previous.len() as u64, Ordering::Relaxed);
    }
    for lane in 0..previous.len() {
        let previous_bits = previous[lane].to_bits() & 0x7fff_ffff;
        let current_bits = current[lane].to_bits() & 0x7fff_ffff;
        debug_assert!(previous_bits < 0x7f80_0000 && current_bits < 0x7f80_0000);
        let mut maximum_bits = previous_bits.max(current_bits);
        let mut maximum = f32::from_bits(maximum_bits);
        #[cfg(all(test, rustsht_profile_spin32_normalizer))]
        let mut lane_scaled = false;
        while maximum_bits > BIG.to_bits() {
            previous[lane] *= SCALE_DOWN;
            current[lane] *= SCALE_DOWN;
            exponents[lane] = exponents[lane]
                .checked_add(80)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "spin recurrence exponent",
                })?;
            maximum *= SCALE_DOWN;
            maximum_bits = maximum.to_bits() & 0x7fff_ffff;
            #[cfg(all(test, rustsht_profile_spin32_normalizer))]
            {
                lane_scaled = true;
                scale_steps += 1;
            }
        }
        while maximum_bits != 0 && maximum_bits < SMALL.to_bits() {
            previous[lane] *= SCALE_UP;
            current[lane] *= SCALE_UP;
            exponents[lane] = exponents[lane]
                .checked_sub(80)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "spin recurrence exponent",
                })?;
            maximum *= SCALE_UP;
            maximum_bits = maximum.to_bits() & 0x7fff_ffff;
            #[cfg(all(test, rustsht_profile_spin32_normalizer))]
            {
                lane_scaled = true;
                scale_steps += 1;
            }
        }
        #[cfg(all(test, rustsht_profile_spin32_normalizer))]
        if lane_scaled {
            scaled_lanes += 1;
        }
    }
    #[cfg(all(test, rustsht_profile_spin32_normalizer))]
    if scaled_lanes != 0 {
        NORMALIZER_SCALED_CALLS.fetch_add(1, Ordering::Relaxed);
        NORMALIZER_SCALED_LANES.fetch_add(scaled_lanes, Ordering::Relaxed);
        NORMALIZER_SCALE_STEPS.fetch_add(scale_steps, Ordering::Relaxed);
    }
    Ok(())
}

fn power(base: f32, mut exponent: usize) -> Result<ScaledValue, ShtError> {
    let mut result = ScaledValue {
        mantissa: 1.0,
        exponent: 0,
    };
    let mut factor = normalize(base, 0)?;
    while exponent != 0 {
        if !exponent.is_multiple_of(2) {
            result = multiply(result, factor)?;
        }
        exponent /= 2;
        if exponent != 0 {
            factor = multiply(factor, factor)?;
        }
    }
    Ok(result)
}

fn multiply(left: ScaledValue, right: ScaledValue) -> Result<ScaledValue, ShtError> {
    let exponent = left
        .exponent
        .checked_add(right.exponent)
        .ok_or(ShtError::IntegerOverflow {
            quantity: "spin recurrence exponent",
        })?;
    normalize(left.mantissa * right.mantissa, exponent)
}

fn normalize(mut value: f32, mut exponent: i64) -> Result<ScaledValue, ShtError> {
    while value.abs() > BIG {
        value *= SCALE_DOWN;
        exponent = exponent.checked_add(80).ok_or(ShtError::IntegerOverflow {
            quantity: "spin recurrence exponent",
        })?;
    }
    while value != 0.0 && value.abs() < SMALL {
        value *= SCALE_UP;
        exponent = exponent.checked_sub(80).ok_or(ShtError::IntegerOverflow {
            quantity: "spin recurrence exponent",
        })?;
    }
    Ok(ScaledValue {
        mantissa: value,
        exponent,
    })
}

pub(crate) fn scaled_basis_value(value: f32, exponent: i64, alpha_norm: f32) -> f32 {
    if value == 0.0 || alpha_norm == 0.0 {
        return 0.0;
    }
    scale_binary(value * alpha_norm, exponent)
}

pub(crate) fn spin_normalization(spin: usize, degree: usize, derivative: bool) -> f32 {
    if derivative {
        if degree == 0 {
            return 0.0;
        }
        let degree = degree as f32;
        0.5 * (degree * (degree + 1.0) * (2.0 * degree + 1.0) / (4.0 * PI)).sqrt()
    } else {
        let sign = if spin.is_multiple_of(2) { -1.0 } else { 1.0 };
        sign * 0.5 * ((2 * degree + 1) as f32 / (4.0 * PI)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AnalysisLanes32 {
        q_real: Vec<f32>,
        q_imag: Vec<f32>,
        u_real: Vec<f32>,
        u_imag: Vec<f32>,
        plus: Vec<f32>,
        plus_exponents: Vec<i64>,
        minus: Vec<f32>,
        minus_exponents: Vec<i64>,
        limits: Vec<usize>,
    }

    impl AnalysisLanes32 {
        fn new(count: usize) -> Self {
            Self {
                q_real: (0..count).map(|lane| 0.25 + lane as f32 * 0.007).collect(),
                q_imag: (0..count).map(|lane| -0.4 + lane as f32 * 0.003).collect(),
                u_real: (0..count).map(|lane| 0.15 - lane as f32 * 0.002).collect(),
                u_imag: (0..count).map(|lane| 0.55 + lane as f32 * 0.004).collect(),
                plus: (0..count).map(|lane| 0.31 + lane as f32 * 0.001).collect(),
                plus_exponents: vec![0; count],
                minus: (0..count).map(|lane| -0.17 + lane as f32 * 0.002).collect(),
                minus_exponents: vec![0; count],
                limits: vec![64; count],
            }
        }

        fn accumulate(
            &self,
            arch: Arch,
            m: usize,
            alpha_norm: f32,
            include_b: bool,
        ) -> (Complex32, Complex32) {
            let vectorizable_inputs = analysis_inputs_vectorizable(
                m,
                &self.q_real,
                &self.q_imag,
                &self.u_real,
                &self.u_imag,
                &self.limits,
            );
            accumulate_analysis_dispatched(
                arch,
                vectorizable_inputs,
                m,
                &self.q_real,
                &self.q_imag,
                &self.u_real,
                &self.u_imag,
                &self.plus,
                &self.plus_exponents,
                &self.minus,
                &self.minus_exponents,
                alpha_norm,
                &self.limits,
                include_b,
            )
        }

        fn scalar_reference(
            &self,
            m: usize,
            alpha_norm: f32,
            include_b: bool,
        ) -> (Complex32, Complex32) {
            let mut e_sum = Complex32::new(0.0, 0.0);
            let mut b_sum = Complex32::new(0.0, 0.0);
            for lane in 0..self.plus.len() {
                if m > self.limits[lane] {
                    continue;
                }
                let plus_value =
                    scaled_basis_value(self.plus[lane], self.plus_exponents[lane], alpha_norm);
                let minus_value =
                    scaled_basis_value(self.minus[lane], self.minus_exponents[lane], alpha_norm);
                let even_basis = plus_value + minus_value;
                let odd_basis = plus_value - minus_value;
                e_sum.re += self.q_real[lane] * even_basis + self.u_imag[lane] * odd_basis;
                e_sum.im += self.q_imag[lane] * even_basis - self.u_real[lane] * odd_basis;
                if include_b {
                    b_sum.re += self.u_real[lane] * even_basis - self.q_imag[lane] * odd_basis;
                    b_sum.im += self.u_imag[lane] * even_basis + self.q_real[lane] * odd_basis;
                }
            }
            (e_sum, b_sum)
        }
    }

    #[test]
    fn f32_spin_analysis_simd_accumulation_matches_scalar_blocks_and_tails() {
        for count in [64, 61] {
            let lanes = AnalysisLanes32::new(count);
            let expected = lanes.scalar_reference(4, 0.75, true);
            assert!(analysis_inputs_vectorizable(
                4,
                &lanes.q_real,
                &lanes.q_imag,
                &lanes.u_real,
                &lanes.u_imag,
                &lanes.limits,
            ));
            for arch in [Arch::Scalar, Arch::new()] {
                let actual = lanes.accumulate(arch, 4, 0.75, true);
                assert!((actual.0 - expected.0).norm() < 1.0e-4);
                assert!((actual.1 - expected.1).norm() < 1.0e-4);

                let e_only = lanes.accumulate(arch, 4, 0.75, false);
                assert!((e_only.0 - expected.0).norm() < 1.0e-4);
                assert_eq!(e_only.1, Complex32::new(0.0, 0.0));
            }
        }
    }

    #[test]
    fn f32_spin_analysis_falls_back_when_simd_lane_partials_can_overflow() {
        let mut lanes = AnalysisLanes32::new(16);
        for lane in 0..lanes.q_real.len() {
            lanes.q_real[lane] = if lane.is_multiple_of(2) {
                f32::MAX
            } else {
                -f32::MAX
            };
            lanes.q_imag[lane] = 0.0;
            lanes.u_real[lane] = 0.0;
            lanes.u_imag[lane] = 0.0;
            lanes.plus[lane] = 0.5;
            lanes.minus[lane] = 0.5;
        }

        assert!(analysis_inputs_vectorizable(
            4,
            &lanes.q_real,
            &lanes.q_imag,
            &lanes.u_real,
            &lanes.u_imag,
            &lanes.limits,
        ));
        let expected = lanes.scalar_reference(4, 1.0, false);
        assert_eq!(expected.0, Complex32::new(0.0, 0.0));
        for arch in [Arch::Scalar, Arch::new()] {
            assert_eq!(lanes.accumulate(arch, 4, 1.0, false), expected);
        }
    }

    #[test]
    fn f32_spin_analysis_zero_inputs_use_simd_and_scaled_lanes_use_scalar_accumulation() {
        let mut lanes = AnalysisLanes32::new(64);
        lanes.plus_exponents[11] = -800;
        lanes.limits[7] = 3;
        let expected = lanes.scalar_reference(4, 0.75, true);
        assert!(!analysis_inputs_vectorizable(
            4,
            &lanes.q_real,
            &lanes.q_imag,
            &lanes.u_real,
            &lanes.u_imag,
            &lanes.limits,
        ));
        for arch in [Arch::Scalar, Arch::new()] {
            let actual = lanes.accumulate(arch, 4, 0.75, true);
            assert_eq!(actual, expected);
        }

        lanes.limits.fill(64);
        lanes.q_real[4] = 0.0;
        lanes.u_imag[9] = -0.0;
        lanes.plus[12] = 0.0;
        lanes.minus[17] = -0.0;
        assert!(lanes.plus.iter().all(|&value| is_normal_or_zero(value)));
        assert!(lanes.minus.iter().all(|&value| is_normal_or_zero(value)));
        let zero_expected = lanes.scalar_reference(4, 0.75, true);
        assert!(analysis_inputs_vectorizable(
            4,
            &lanes.q_real,
            &lanes.q_imag,
            &lanes.u_real,
            &lanes.u_imag,
            &lanes.limits,
        ));
        for arch in [Arch::Scalar, Arch::new()] {
            let actual = lanes.accumulate(arch, 4, 0.75, true);
            assert!((actual.0 - zero_expected.0).norm() < 1.0e-4);
            assert!((actual.1 - zero_expected.1).norm() < 1.0e-4);
        }

        lanes.q_real[4] = f32::from_bits(1);
        lanes.plus[4] = f32::from_bits(1);
        assert!(!lanes.plus.iter().all(|&value| is_normal_or_zero(value)));
        assert!(!analysis_inputs_vectorizable(
            4,
            &lanes.q_real,
            &lanes.q_imag,
            &lanes.u_real,
            &lanes.u_imag,
            &lanes.limits,
        ));
    }

    fn reference_normalize_pair_lanes(
        previous: &mut [f32],
        current: &mut [f32],
        exponents: &mut [i64],
    ) {
        for lane in 0..previous.len() {
            let mut maximum = previous[lane].abs().max(current[lane].abs());
            while maximum > BIG {
                previous[lane] *= SCALE_DOWN;
                current[lane] *= SCALE_DOWN;
                exponents[lane] += 80;
                maximum *= SCALE_DOWN;
            }
            while maximum != 0.0 && maximum < SMALL {
                previous[lane] *= SCALE_UP;
                current[lane] *= SCALE_UP;
                exponents[lane] -= 80;
                maximum *= SCALE_UP;
            }
        }
    }

    #[test]
    fn f32_spin_pair_normalization_matches_float_magnitude_reference() {
        let initial_previous = [
            0.0,
            f32::from_bits(1),
            f32::from_bits(SMALL.to_bits() - 1),
            -f32::from_bits(BIG.to_bits() + 1),
            f32::MAX,
            f32::MIN_POSITIVE,
            -0.5,
            0.0,
        ];
        let initial_current = [
            -0.0,
            0.0,
            0.0,
            BIG,
            f32::MIN_POSITIVE,
            -f32::from_bits(1),
            0.25,
            SMALL,
        ];
        let mut actual_previous = initial_previous;
        let mut actual_current = initial_current;
        let mut actual_exponents = [0; 8];
        let mut expected_previous = initial_previous;
        let mut expected_current = initial_current;
        let mut expected_exponents = [0; 8];

        normalize_pair_lanes(
            &mut actual_previous,
            &mut actual_current,
            &mut actual_exponents,
        )
        .unwrap();
        reference_normalize_pair_lanes(
            &mut expected_previous,
            &mut expected_current,
            &mut expected_exponents,
        );

        for lane in 0..initial_previous.len() {
            assert_eq!(
                actual_previous[lane].to_bits(),
                expected_previous[lane].to_bits()
            );
            assert_eq!(
                actual_current[lane].to_bits(),
                expected_current[lane].to_bits()
            );
            assert_eq!(actual_exponents[lane], expected_exponents[lane]);
        }
    }

    fn values_over_rings(
        arch: Arch,
        spin: usize,
        m: usize,
        lmax: usize,
        theta: &[f32],
    ) -> (Vec<Vec<f32>>, Vec<Vec<f32>>) {
        let count = lmax + 2;
        let mut alpha = vec![0.0; count];
        let mut coefficient_a = vec![0.0; count];
        let mut coefficient_b = vec![0.0; count];
        let initial = fill_spin_factors(
            spin,
            m,
            lmax,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        )
        .unwrap();
        let sin_theta = theta.iter().map(|value| value.sin()).collect::<Vec<_>>();
        let cos_theta = theta.iter().map(|value| value.cos()).collect::<Vec<_>>();
        let scratch_len = theta.len().min(LATITUDE_BLOCK_SIZE);
        let mut previous_plus = vec![0.0; scratch_len];
        let mut current_plus = vec![0.0; scratch_len];
        let mut next_plus = vec![0.0; scratch_len];
        let mut exponent_plus = vec![0; scratch_len];
        let mut previous_minus = vec![0.0; scratch_len];
        let mut current_minus = vec![0.0; scratch_len];
        let mut next_minus = vec![0.0; scratch_len];
        let mut exponent_minus = vec![0; scratch_len];
        let degree_count = lmax - initial.start_degree + 1;
        let mut plus_values = vec![vec![0.0; theta.len()]; degree_count];
        let mut minus_values = vec![vec![0.0; theta.len()]; degree_count];
        for_each_spin_lm_dispatched(
            arch,
            &initial,
            lmax,
            &sin_theta,
            &cos_theta,
            &coefficient_a,
            &coefficient_b,
            &mut previous_plus,
            &mut current_plus,
            &mut next_plus,
            &mut exponent_plus,
            &mut previous_minus,
            &mut current_minus,
            &mut next_minus,
            &mut exponent_minus,
            |degree, ring_start, plus, plus_exponents, minus, minus_exponents| {
                let factor = spin_normalization(spin, degree, false) * alpha[degree];
                for lane in 0..plus.len() {
                    plus_values[degree - initial.start_degree][ring_start + lane] =
                        scaled_basis_value(plus[lane], plus_exponents[lane], factor);
                    minus_values[degree - initial.start_degree][ring_start + lane] =
                        scaled_basis_value(minus[lane], minus_exponents[lane], factor);
                }
                Ok(())
            },
        )
        .unwrap();
        (plus_values, minus_values)
    }

    #[test]
    fn runtime_simd_recurrence_matches_scalar_fallback_across_blocks_and_tails() {
        let mut theta = (0..129)
            .map(|ring| PI * (ring as f32 + 0.5) / 129.0)
            .collect::<Vec<_>>();
        theta[0] = 1.0e-6;
        theta[1] = 1.0e-3;
        theta[127] = PI - 1.0e-3;
        theta[128] = PI - 1.0e-6;

        for spin in 1..=4 {
            for m in [0, spin, 96] {
                let expected = values_over_rings(Arch::Scalar, spin, m, 96, &theta);
                let actual = values_over_rings(Arch::new(), spin, m, 96, &theta);
                for (sign, expected_values, actual_values) in [
                    ("plus", &expected.0, &actual.0),
                    ("minus", &expected.1, &actual.1),
                ] {
                    for (degree_offset, (expected_rings, actual_rings)) in
                        expected_values.iter().zip(actual_values).enumerate()
                    {
                        for (ring, (&expected, &actual)) in
                            expected_rings.iter().zip(actual_rings).enumerate()
                        {
                            assert!(
                                actual.is_finite(),
                                "spin={spin}, m={m}, sign={sign}, l={}, ring={ring}: non-finite result {actual}",
                                spin.max(m) + degree_offset
                            );
                            assert!(
                                (actual - expected).abs() <= 5.0e-6,
                                "spin={spin}, m={m}, sign={sign}, l={}, ring={ring}: actual {actual:.9e}, scalar {expected:.9e}",
                                spin.max(m) + degree_offset
                            );
                        }
                    }
                }
            }
        }
    }
}
