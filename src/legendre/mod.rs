// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

mod scalar;
mod scalar_f32;
mod spin;
mod spin_f32;

use num_complex::Complex;
use pulp::Arch;
use rustfft::FftNum;
use rustfft::num_traits::{Float, FromPrimitive};

use crate::error::ShtError;

pub(crate) fn longitude_phase<T: Float + FromPrimitive>(
    order: usize,
    phi0: f64,
    inverse: bool,
) -> Complex<T> {
    let angle = order as f64 * phi0;
    let phase = if angle.is_finite() {
        let angle = if inverse { -angle } else { angle };
        let (sin, cos) = angle.sin_cos();
        Complex::<f64>::new(cos, sin)
    } else {
        let (sin, cos) = phi0.sin_cos();
        let mut base = Complex::<f64>::new(cos, sin);
        let mut result = Complex::<f64>::new(1.0, 0.0);
        let mut exponent = order;
        while exponent > 0 {
            if exponent & 1 == 1 {
                result *= base;
            }
            exponent >>= 1;
            if exponent > 0 {
                base *= base;
            }
        }
        if inverse { result.conj() } else { result }
    };
    Complex::new(
        T::from_f64(phase.re).expect("longitude phase is representable"),
        T::from_f64(phase.im).expect("longitude phase is representable"),
    )
}

pub(crate) fn sin_cos_from_f64<T: Float + FromPrimitive>(angle: f64) -> (T, T) {
    let (sin, cos) = if angle == 0.0 {
        (0.0, 1.0)
    } else if angle == std::f64::consts::PI {
        (0.0, -1.0)
    } else {
        angle.sin_cos()
    };
    (
        T::from_f64(sin).expect("latitude sine is representable"),
        T::from_f64(cos).expect("latitude cosine is representable"),
    )
}

/// Numerical operations needed by the f32 and f64 transform implementations.
///
/// This remains crate-private so the public API exposes only the supported
/// scalar types through its generic plan and workspace types.
pub(crate) trait ShtScalar:
    FftNum
    + Float
    + FromPrimitive
    + std::ops::AddAssign
    + std::ops::DivAssign
    + std::ops::MulAssign
    + std::ops::RemAssign
    + std::ops::SubAssign
{
    type SpinInitial;

    fn sectoral_normalizations(mmax: usize) -> Result<Vec<Self>, ShtError>;
    fn polar_order_limit(lmax: usize, spin: usize, sin_theta: Self, cos_theta: Self) -> usize;
    fn fill_epsilon(m: usize, lmax: usize, output: &mut [Self]);
    #[expect(
        clippy::too_many_arguments,
        reason = "The recurrence receives grid lanes and caller-owned recurrence scratch explicitly."
    )]
    fn for_each_y_lm_dispatched<F>(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        epsilon: &[Self],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        visit: F,
    ) -> Result<(), ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64]) -> Result<(), ShtError>;
    fn multiply_complex_scaled(value: Complex<Self>, factor: Self, exponent: i64) -> Complex<Self>;
    fn fill_spin_factors(
        spin: usize,
        m: usize,
        lmax: usize,
        alpha: &mut [Self],
        coefficient_a: &mut [Self],
        coefficient_b: &mut [Self],
    ) -> Result<Self::SpinInitial, ShtError>;
    #[expect(
        clippy::too_many_arguments,
        reason = "The spin recurrence receives plan factors and caller-owned lane scratch explicitly."
    )]
    fn for_each_spin_lm_dispatched<F>(
        arch: Arch,
        initial: &Self::SpinInitial,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        previous_plus: &mut [Self],
        current_plus: &mut [Self],
        next_plus: &mut [Self],
        exponent_plus: &mut [i64],
        previous_minus: &mut [Self],
        current_minus: &mut [Self],
        next_minus: &mut [Self],
        exponent_minus: &mut [i64],
        visit: F,
    ) -> Result<(), ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64], &[Self], &[i64]) -> Result<(), ShtError>;
    fn scaled_basis_value(value: Self, exponent: i64, alpha_norm: Self) -> Self;
    fn spin_normalization(spin: usize, degree: usize, derivative: bool) -> Self;
}

impl ShtScalar for f64 {
    type SpinInitial = spin::SpinInitial;

    fn sectoral_normalizations(mmax: usize) -> Result<Vec<Self>, ShtError> {
        scalar::sectoral_normalizations(mmax)
    }

    fn polar_order_limit(lmax: usize, spin: usize, sin_theta: Self, cos_theta: Self) -> usize {
        scalar::polar_order_limit(lmax, spin, sin_theta, cos_theta)
    }

    fn fill_epsilon(m: usize, lmax: usize, output: &mut [Self]) {
        scalar::fill_epsilon(m, lmax, output);
    }

    fn for_each_y_lm_dispatched<F>(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        epsilon: &[Self],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        visit: F,
    ) -> Result<(), ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64]) -> Result<(), ShtError>,
    {
        scalar::for_each_y_lm_dispatched(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            epsilon,
            previous,
            current,
            next,
            exponents,
            visit,
        )
    }

    fn multiply_complex_scaled(value: Complex<Self>, factor: Self, exponent: i64) -> Complex<Self> {
        scalar::multiply_complex_scaled(value, factor, exponent)
    }

    fn fill_spin_factors(
        spin: usize,
        m: usize,
        lmax: usize,
        alpha: &mut [Self],
        coefficient_a: &mut [Self],
        coefficient_b: &mut [Self],
    ) -> Result<Self::SpinInitial, ShtError> {
        spin::fill_spin_factors(spin, m, lmax, alpha, coefficient_a, coefficient_b)
    }

    fn for_each_spin_lm_dispatched<F>(
        arch: Arch,
        initial: &Self::SpinInitial,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        previous_plus: &mut [Self],
        current_plus: &mut [Self],
        next_plus: &mut [Self],
        exponent_plus: &mut [i64],
        previous_minus: &mut [Self],
        current_minus: &mut [Self],
        next_minus: &mut [Self],
        exponent_minus: &mut [i64],
        visit: F,
    ) -> Result<(), ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64], &[Self], &[i64]) -> Result<(), ShtError>,
    {
        spin::for_each_spin_lm_dispatched(
            arch,
            initial,
            lmax,
            sin_theta,
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
            visit,
        )
    }

    fn scaled_basis_value(value: Self, exponent: i64, alpha_norm: Self) -> Self {
        spin::scaled_basis_value(value, exponent, alpha_norm)
    }

    fn spin_normalization(spin: usize, degree: usize, derivative: bool) -> Self {
        spin::spin_normalization(spin, degree, derivative)
    }
}

impl ShtScalar for f32 {
    type SpinInitial = spin_f32::SpinInitial32;

    fn sectoral_normalizations(mmax: usize) -> Result<Vec<Self>, ShtError> {
        scalar_f32::sectoral_normalizations(mmax)
    }

    fn polar_order_limit(lmax: usize, spin: usize, sin_theta: Self, cos_theta: Self) -> usize {
        scalar_f32::polar_order_limit(lmax, spin, sin_theta, cos_theta)
    }

    fn fill_epsilon(m: usize, lmax: usize, output: &mut [Self]) {
        scalar_f32::fill_epsilon(m, lmax, output);
    }

    fn for_each_y_lm_dispatched<F>(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        epsilon: &[Self],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        visit: F,
    ) -> Result<(), ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64]) -> Result<(), ShtError>,
    {
        scalar_f32::for_each_y_lm_dispatched(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            epsilon,
            previous,
            current,
            next,
            exponents,
            visit,
        )
    }

    fn multiply_complex_scaled(value: Complex<Self>, factor: Self, exponent: i64) -> Complex<Self> {
        scalar_f32::multiply_complex_scaled(value, factor, exponent)
    }

    fn fill_spin_factors(
        spin: usize,
        m: usize,
        lmax: usize,
        alpha: &mut [Self],
        coefficient_a: &mut [Self],
        coefficient_b: &mut [Self],
    ) -> Result<Self::SpinInitial, ShtError> {
        spin_f32::fill_spin_factors(spin, m, lmax, alpha, coefficient_a, coefficient_b)
    }

    fn for_each_spin_lm_dispatched<F>(
        arch: Arch,
        initial: &Self::SpinInitial,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        previous_plus: &mut [Self],
        current_plus: &mut [Self],
        next_plus: &mut [Self],
        exponent_plus: &mut [i64],
        previous_minus: &mut [Self],
        current_minus: &mut [Self],
        next_minus: &mut [Self],
        exponent_minus: &mut [i64],
        visit: F,
    ) -> Result<(), ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64], &[Self], &[i64]) -> Result<(), ShtError>,
    {
        spin_f32::for_each_spin_lm_dispatched(
            arch,
            initial,
            lmax,
            sin_theta,
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
            visit,
        )
    }

    fn scaled_basis_value(value: Self, exponent: i64, alpha_norm: Self) -> Self {
        spin_f32::scaled_basis_value(value, exponent, alpha_norm)
    }

    fn spin_normalization(spin: usize, degree: usize, derivative: bool) -> Self {
        spin_f32::spin_normalization(spin, degree, derivative)
    }
}

pub(crate) use scalar::{LATITUDE_BLOCK_SIZE, scale_binary};
