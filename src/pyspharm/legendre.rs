// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::f64::consts::PI;

use pulp::Arch;

use crate::error::ShtError;
use crate::harmonics::HarmonicSpec;
use crate::legendre::ShtScalar;

/// Evaluates pyspharm-normalized associated Legendre functions at one latitude.
///
/// The result contains all values through degree `lmax`, ordered by increasing
/// order `m`, then increasing degree `l = m..=lmax`. The normalization is the
/// one returned by pyspharm's `legendre`; in particular, the degree-zero value
/// is `1/sqrt(2)`. Latitude is in degrees and may lie outside the usual
/// geographic range; finite values are evaluated using the same trigonometric
/// formula as the source API.
///
/// # Errors
///
/// Returns an error if the latitude is non-finite, the packed size overflows,
/// or the result cannot be allocated.
pub fn associated_legendre_f32(latitude_degrees: f32, lmax: usize) -> Result<Vec<f32>, ShtError> {
    legendre_table(&[latitude_degrees], lmax, lmax)
}

/// Evaluates pyspharm-normalized associated Legendre functions at one latitude.
///
/// The result contains all values through degree `lmax`, ordered by increasing
/// order `m`, then increasing degree `l = m..=lmax`. The normalization is the
/// one returned by pyspharm's `legendre`; in particular, the degree-zero value
/// is `1/sqrt(2)`. Latitude is in degrees and may lie outside the usual
/// geographic range; finite values are evaluated using the same trigonometric
/// formula as the source API.
///
/// # Errors
///
/// Returns an error if the latitude is non-finite, the packed size overflows,
/// or the result cannot be allocated.
pub fn associated_legendre_f64(latitude_degrees: f64, lmax: usize) -> Result<Vec<f64>, ShtError> {
    legendre_table(&[latitude_degrees], lmax, lmax)
}

/// Visits each normalized basis value over a set of arbitrary latitudes.
///
/// The recurrence is shared with the regular-grid transform implementation.
/// Its callback receives degree, order, point offset, mantissas, binary
/// exponents and the pyspharm normalization/phase factor for the order.
pub(crate) fn for_each_pyspharm_legendre<T, F>(
    latitudes_degrees: &[T],
    lmax: usize,
    mmax: usize,
    mut visit: F,
) -> Result<(), ShtError>
where
    T: ShtScalar,
    F: FnMut(usize, usize, usize, &[T], &[i64], T) -> Result<(), ShtError>,
{
    let layout = HarmonicSpec::scalar(lmax, mmax)?;
    for &latitude in latitudes_degrees {
        if !latitude.is_finite() {
            return Err(ShtError::NonFiniteLatitude);
        }
    }
    if latitudes_degrees.is_empty() {
        return Ok(());
    }

    let degrees_to_radians = T::from_f64(PI / 180.0).expect("f32 and f64 represent pi/180");
    let half_pi = T::from_f64(PI / 2.0).expect("f32 and f64 represent pi/2");
    let ninety = T::from_f64(90.0).expect("f32 and f64 represent 90");
    let minus_ninety = -ninety;
    let mut sin_theta = reserve_filled(latitudes_degrees.len(), T::zero(), "latitude sine")?;
    let mut cos_theta = reserve_filled(latitudes_degrees.len(), T::zero(), "latitude cosine")?;
    for (point, &latitude) in latitudes_degrees.iter().enumerate() {
        if latitude == ninety {
            sin_theta[point] = T::zero();
            cos_theta[point] = T::one();
        } else if latitude == minus_ninety {
            sin_theta[point] = T::zero();
            cos_theta[point] = -T::one();
        } else {
            let theta = half_pi - latitude * degrees_to_radians;
            let (sine, cosine) = theta.sin_cos();
            sin_theta[point] = sine;
            cos_theta[point] = cosine;
        }
    }

    let epsilon_len = lmax.checked_add(1).ok_or(ShtError::IntegerOverflow {
        quantity: "associated Legendre recurrence factors",
    })?;
    let mut epsilon = reserve_filled(epsilon_len, T::zero(), "associated Legendre factors")?;
    let sectoral_norms = T::sectoral_normalizations(mmax)?;
    let scratch_len = latitudes_degrees
        .len()
        .min(crate::legendre::LATITUDE_BLOCK_SIZE);
    let mut previous = reserve_filled(scratch_len, T::zero(), "Legendre previous-degree scratch")?;
    let mut current = reserve_filled(scratch_len, T::zero(), "Legendre current-degree scratch")?;
    let mut next = reserve_filled(scratch_len, T::zero(), "Legendre next-degree scratch")?;
    let mut exponents = reserve_filled(scratch_len, 0_i64, "Legendre exponent scratch")?;
    let sqrt_two_pi = T::from_f64((2.0 * PI).sqrt()).expect("f32 and f64 represent sqrt(2 pi)");
    let arch = Arch::new();

    for (m, &sectoral_norm) in sectoral_norms.iter().take(mmax + 1).enumerate() {
        T::fill_epsilon(m, lmax, &mut epsilon);
        let phase_sign = if m.is_multiple_of(2) {
            T::one()
        } else {
            -T::one()
        };
        // RustSHT's recurrence returns Y_lm with the Condon-Shortley phase.
        // pyspharm's pbar is (-1)^m * sqrt(2*pi) * Y_lm.
        let pyspharm_scale = phase_sign * sqrt_two_pi;
        T::for_each_y_lm_dispatched(
            arch,
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &epsilon,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
            |degree, point_start, mantissas, lane_exponents| {
                debug_assert!(layout.index(degree, m).is_some());
                visit(
                    degree,
                    m,
                    point_start,
                    mantissas,
                    lane_exponents,
                    pyspharm_scale,
                )
            },
        )?;
    }
    Ok(())
}

pub(crate) fn legendre_table<T: ShtScalar>(
    latitudes_degrees: &[T],
    lmax: usize,
    mmax: usize,
) -> Result<Vec<T>, ShtError> {
    let layout = HarmonicSpec::scalar(lmax, mmax)?;
    let output_len =
        latitudes_degrees
            .len()
            .checked_mul(layout.nalm())
            .ok_or(ShtError::IntegerOverflow {
                quantity: "arbitrary-latitude Legendre table length",
            })?;
    let mut output = reserve_filled(output_len, T::zero(), "arbitrary-latitude Legendre table")?;

    for_each_pyspharm_legendre(
        latitudes_degrees,
        lmax,
        mmax,
        |degree, order, point_start, values, exponents, normalization| {
            let packed_index = layout
                .index(degree, order)
                .expect("the recurrence visits only valid harmonic indices");
            for lane in 0..values.len() {
                let point = point_start + lane;
                output[point * layout.nalm() + packed_index] =
                    T::scaled_basis_value(values[lane], exponents[lane], normalization);
            }
            Ok(())
        },
    )?;
    Ok(output)
}

pub(crate) fn reserve_filled<T: Clone>(
    len: usize,
    value: T,
    array: &'static str,
) -> Result<Vec<T>, ShtError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed { array })?;
    result.resize(len, value);
    Ok(result)
}

#[cfg(test)]
#[path = "legendre_tests.rs"]
mod tests;
