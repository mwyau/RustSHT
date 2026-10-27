// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::f64::consts::PI;

use num_complex::Complex;

use crate::error::ShtError;
use crate::harmonics::HarmonicSpec;
use crate::legendre::ShtScalar;

use crate::legendre::pointwise::{
    for_each_pointwise_legendre, pointwise_legendre_table, reserve_filled,
};

/// Evaluates a packed real-field scalar spectrum at a longitude using
/// precomputed pointwise Legendre values.
///
/// The coefficient and Legendre arrays use the increasing-order, then
/// increasing-degree triangular layout produced by
/// [`crate::legendre::pointwise::associated_legendre_f32`]. Longitude is in
/// degrees; only the real part of the zonal-order sum contributes to the real
/// output.
///
/// # Errors
///
/// Returns an error if the arrays do not have equal triangular lengths, the
/// longitude is non-finite, or the packed layout overflows.
pub fn interpolate_spectrum_at_longitude_f32(
    longitude_degrees: f32,
    coefficients: &[Complex<f32>],
    legendre: &[f32],
) -> Result<f32, ShtError> {
    interpolate(longitude_degrees, coefficients, legendre)
}

/// Evaluates a packed real-field scalar spectrum at a longitude using
/// precomputed pointwise Legendre values.
///
/// The coefficient and Legendre arrays use the increasing-order, then
/// increasing-degree triangular layout. Longitude is in degrees. As in
/// only the real part of the zonal-order sum contributes to the real output.
///
/// # Errors
///
/// Returns an error if the arrays do not have equal triangular lengths, the
/// longitude is non-finite, or the packed layout overflows.
pub fn interpolate_spectrum_at_longitude_f64(
    longitude_degrees: f64,
    coefficients: &[Complex<f64>],
    legendre: &[f64],
) -> Result<f64, ShtError> {
    interpolate(longitude_degrees, coefficients, legendre)
}

/// Synthesizes one real scalar value at an arbitrary latitude and longitude.
///
/// Coordinates are in degrees. Supplying a precomputed Legendre array avoids
/// recomputing the latitude recurrence and has the same effect as calling
/// [`interpolate_spectrum_at_longitude_f32`] directly. The coefficient array
/// must contain a complete triangular truncation in increasing-order, then
/// increasing-degree packing order.
///
/// # Errors
///
/// Returns an error for non-finite coordinates, a non-triangular coefficient
/// length, a mismatched precomputed Legendre array, integer overflow, or an
/// allocation failure.
pub fn synthesize_at_point_f32(
    latitude_degrees: f32,
    longitude_degrees: f32,
    coefficients: &[Complex<f32>],
    precomputed_legendre: Option<&[f32]>,
) -> Result<f32, ShtError> {
    synthesize_point(
        latitude_degrees,
        longitude_degrees,
        coefficients,
        precomputed_legendre,
    )
}

/// Synthesizes one real scalar value at an arbitrary latitude and longitude.
///
/// Coordinates are in degrees. Supplying a precomputed Legendre array avoids
/// recomputing the latitude recurrence. The coefficient array must contain a
/// complete triangular truncation in increasing-order, then increasing-degree
/// packing order.
///
/// # Errors
///
/// Returns an error for non-finite coordinates, a non-triangular coefficient
/// length, a mismatched precomputed Legendre array, integer overflow, or an
/// allocation failure.
pub fn synthesize_at_point_f64(
    latitude_degrees: f64,
    longitude_degrees: f64,
    coefficients: &[Complex<f64>],
    precomputed_legendre: Option<&[f64]>,
) -> Result<f64, ShtError> {
    synthesize_point(
        latitude_degrees,
        longitude_degrees,
        coefficients,
        precomputed_legendre,
    )
}

/// Synthesizes a packed real-field scalar spectrum at paired coordinates.
///
/// Coordinates are in degrees. The arrays must have equal lengths. The
/// operation evaluates the scaled latitude recurrence directly at the supplied
/// points and does not construct a longitude grid or use an FFT. Its temporary
/// memory is linear in the number of points.
///
/// # Errors
///
/// Returns an error for non-finite coordinates, a non-triangular coefficient
/// length, unequal coordinate counts, integer overflow, or allocation failure.
pub fn synthesize_at_points_f32(
    latitudes_degrees: &[f32],
    longitudes_degrees: &[f32],
    coefficients: &[Complex<f32>],
) -> Result<Vec<f32>, ShtError> {
    synthesize_points(latitudes_degrees, longitudes_degrees, coefficients)
}

/// Synthesizes a packed real-field scalar spectrum at paired coordinates.
///
/// Coordinates are in degrees. The arrays must have equal lengths. The
/// operation evaluates the scaled latitude recurrence directly at the supplied
/// points and does not construct a longitude grid or use an FFT. Its temporary
/// memory is linear in the number of points.
///
/// # Errors
///
/// Returns an error for non-finite coordinates, a non-triangular coefficient
/// length, unequal coordinate counts, integer overflow, or allocation failure.
pub fn synthesize_at_points_f64(
    latitudes_degrees: &[f64],
    longitudes_degrees: &[f64],
    coefficients: &[Complex<f64>],
) -> Result<Vec<f64>, ShtError> {
    synthesize_points(latitudes_degrees, longitudes_degrees, coefficients)
}

fn synthesize_point<T: ShtScalar>(
    latitude_degrees: T,
    longitude_degrees: T,
    coefficients: &[Complex<T>],
    precomputed_legendre: Option<&[T]>,
) -> Result<T, ShtError> {
    if !latitude_degrees.is_finite() {
        return Err(ShtError::NonFiniteLatitude);
    }
    if !longitude_degrees.is_finite() {
        return Err(ShtError::NonFiniteLongitude);
    }
    let lmax = triangular_lmax(coefficients.len())?;
    let owned_legendre;
    let legendre = if let Some(values) = precomputed_legendre {
        if values.len() != coefficients.len() {
            return Err(ShtError::BufferLength {
                buffer: "precomputed Legendre values",
                expected: coefficients.len(),
                actual: values.len(),
            });
        }
        values
    } else {
        owned_legendre =
            pointwise_legendre_table(std::slice::from_ref(&latitude_degrees), lmax, lmax)?;
        &owned_legendre
    };
    interpolate(longitude_degrees, coefficients, legendre)
}

fn interpolate<T: ShtScalar>(
    longitude_degrees: T,
    coefficients: &[Complex<T>],
    legendre: &[T],
) -> Result<T, ShtError> {
    if !longitude_degrees.is_finite() {
        return Err(ShtError::NonFiniteLongitude);
    }
    if coefficients.len() != legendre.len() {
        return Err(ShtError::BufferLength {
            buffer: "precomputed Legendre values",
            expected: coefficients.len(),
            actual: legendre.len(),
        });
    }
    let lmax = triangular_lmax(coefficients.len())?;
    let layout = HarmonicSpec::scalar(lmax, lmax)?;
    let radians =
        longitude_degrees * T::from_f64(PI / 180.0).expect("f32 and f64 represent pi/180");
    let two = T::one() + T::one();
    let mut result = T::zero();

    for m in 0..=lmax {
        let mut real_sum = T::zero();
        let mut imag_sum = T::zero();
        for l in m..=lmax {
            let index = layout
                .index(l, m)
                .expect("triangular layout contains every requested harmonic");
            let coefficient = coefficients[index];
            let basis = legendre[index];
            real_sum += coefficient.re * basis;
            imag_sum += coefficient.im * basis;
        }
        if m == 0 {
            result += real_sum;
        } else {
            let phase = T::from_usize(m).expect("harmonic order is representable") * radians;
            let (sine, cosine) = phase.sin_cos();
            result += two * (real_sum * cosine - imag_sum * sine);
        }
    }
    Ok(result)
}

fn synthesize_points<T: ShtScalar>(
    latitudes_degrees: &[T],
    longitudes_degrees: &[T],
    coefficients: &[Complex<T>],
) -> Result<Vec<T>, ShtError> {
    if latitudes_degrees.len() != longitudes_degrees.len() {
        return Err(ShtError::PointCountMismatch {
            latitudes: latitudes_degrees.len(),
            longitudes: longitudes_degrees.len(),
        });
    }
    for &longitude in longitudes_degrees {
        if !longitude.is_finite() {
            return Err(ShtError::NonFiniteLongitude);
        }
    }
    let lmax = triangular_lmax(coefficients.len())?;
    let layout = HarmonicSpec::scalar(lmax, lmax)?;
    let degrees_to_radians = T::from_f64(PI / 180.0).expect("f32 and f64 represent pi/180");
    let mut longitudes = reserve_filled(longitudes_degrees.len(), T::zero(), "point longitudes")?;
    for (longitude, &degrees) in longitudes.iter_mut().zip(longitudes_degrees) {
        *longitude = degrees * degrees_to_radians;
    }
    let mut output = reserve_filled(latitudes_degrees.len(), T::zero(), "point synthesis output")?;
    let mut real_sum = reserve_filled(
        latitudes_degrees.len(),
        T::zero(),
        "point synthesis real scratch",
    )?;
    let mut imag_sum = reserve_filled(
        latitudes_degrees.len(),
        T::zero(),
        "point synthesis imaginary scratch",
    )?;
    let two = T::one() + T::one();

    for_each_pointwise_legendre(
        latitudes_degrees,
        lmax,
        lmax,
        |degree, order, point_start, values, exponents, normalization| {
            let point_end = point_start + values.len();
            if degree == order {
                real_sum[point_start..point_end].fill(T::zero());
                imag_sum[point_start..point_end].fill(T::zero());
            }
            let index = layout
                .index(degree, order)
                .expect("the recurrence visits only valid harmonic indices");
            let coefficient = coefficients[index];
            for lane in 0..values.len() {
                let point = point_start + lane;
                let basis = T::scaled_basis_value(values[lane], exponents[lane], normalization);
                real_sum[point] += coefficient.re * basis;
                imag_sum[point] += coefficient.im * basis;
            }
            if degree == lmax {
                if order == 0 {
                    for point in point_start..point_end {
                        output[point] += real_sum[point];
                    }
                } else {
                    let order_value =
                        T::from_usize(order).expect("harmonic order is representable");
                    for point in point_start..point_end {
                        let phase = order_value * longitudes[point];
                        let (sine, cosine) = phase.sin_cos();
                        output[point] += two * (real_sum[point] * cosine - imag_sum[point] * sine);
                    }
                }
            }
            Ok(())
        },
    )?;
    Ok(output)
}

fn triangular_lmax(length: usize) -> Result<usize, ShtError> {
    let triangular_length = |lmax: usize| {
        lmax.checked_add(1)
            .and_then(|first| {
                lmax.checked_add(2)
                    .and_then(|second| first.checked_mul(second))
            })
            .map(|product| product / 2)
    };
    let mut low = 0usize;
    let mut high = 1usize;
    while triangular_length(high).is_none_or(|value| value < length) {
        high = high
            .checked_mul(2)
            .ok_or(ShtError::InvalidTriangularLength { length })?;
    }
    while low <= high {
        let middle = low + (high - low) / 2;
        match triangular_length(middle) {
            Some(value) if value == length => return Ok(middle),
            Some(value) if value < length => low = middle + 1,
            _ => {
                if middle == 0 {
                    break;
                }
                high = middle - 1;
            }
        }
    }
    Err(ShtError::InvalidTriangularLength { length })
}

#[cfg(test)]
mod tests;
