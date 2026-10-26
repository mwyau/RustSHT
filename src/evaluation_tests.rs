// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::{Complex32, Complex64};

use super::{
    interpolate_spectrum_at_longitude_f32, interpolate_spectrum_at_longitude_f64,
    synthesize_at_point_f32, synthesize_at_point_f64, synthesize_at_points_f32,
    synthesize_at_points_f64,
};
use crate::ShtError;
use crate::legendre::pointwise::{associated_legendre_f32, associated_legendre_f64};

#[test]
fn spectrum_interpolation_uses_real_field_phase_and_packed_order() {
    let latitude = 23.0_f64;
    let longitude = -41.0_f64;
    let basis = associated_legendre_f64(latitude, 2).unwrap();
    let mut coefficients = vec![Complex64::new(0.0, 0.0); 6];
    coefficients[0] = Complex64::new(0.5, 17.0);
    coefficients[3] = Complex64::new(2.0, -3.0);
    coefficients[5] = Complex64::new(-0.75, 0.25);
    let lon = longitude.to_radians();
    let expected = 0.5 * basis[0]
        + 2.0 * (2.0 * basis[3] * lon.cos() + 3.0 * basis[3] * lon.sin())
        + 2.0 * (-0.75 * basis[5] * (2.0 * lon).cos() - 0.25 * basis[5] * (2.0 * lon).sin());
    let observed = interpolate_spectrum_at_longitude_f64(longitude, &coefficients, &basis).unwrap();
    assert!((observed - expected).abs() < 2.0e-15);
}

#[test]
fn point_and_batch_synthesis_agree_with_precomputed_legendre_values() {
    let mut latitudes = vec![90.0_f64, 23.0, -61.5, -90.0];
    let mut longitudes = vec![0.0_f64, 47.0, -211.0, 123.0];
    for point in 4..131 {
        latitudes.push(-89.0 + (point as f64 * 31.7).rem_euclid(178.0));
        longitudes.push(-720.0 + point as f64 * 19.3);
    }
    let mut coefficients = vec![Complex64::new(0.0, 0.0); 15];
    for (index, coefficient) in coefficients.iter_mut().enumerate() {
        coefficient.re = (index as f64 - 4.0) / 19.0;
        coefficient.im = (index as f64 - 7.0) / 23.0;
    }
    let batch = synthesize_at_points_f64(&latitudes, &longitudes, &coefficients).unwrap();
    assert_eq!(batch.len(), latitudes.len());
    for point in 0..latitudes.len() {
        let legendre = associated_legendre_f64(latitudes[point], 4).unwrap();
        let expected =
            interpolate_spectrum_at_longitude_f64(longitudes[point], &coefficients, &legendre)
                .unwrap();
        let computed =
            synthesize_at_point_f64(latitudes[point], longitudes[point], &coefficients, None)
                .unwrap();
        let precomputed = synthesize_at_point_f64(
            latitudes[point],
            longitudes[point],
            &coefficients,
            Some(&legendre),
        )
        .unwrap();
        assert!((batch[point] - expected).abs() < 3.0e-14);
        assert!((computed - expected).abs() < 3.0e-14);
        assert_eq!(precomputed, expected);
    }
}

#[test]
fn f32_point_and_batch_paths_preserve_single_precision_coefficients() {
    let latitudes = [0.0_f32, 39.0, -72.0];
    let longitudes = [0.0_f32, 135.0, -23.0];
    let coefficients = (0..10)
        .map(|index| Complex32::new(index as f32 / 9.0, index as f32 / 13.0))
        .collect::<Vec<_>>();
    let batch = synthesize_at_points_f32(&latitudes, &longitudes, &coefficients).unwrap();
    for point in 0..latitudes.len() {
        let basis = associated_legendre_f32(latitudes[point], 3).unwrap();
        let expected =
            interpolate_spectrum_at_longitude_f32(longitudes[point], &coefficients, &basis)
                .unwrap();
        let observed =
            synthesize_at_point_f32(latitudes[point], longitudes[point], &coefficients, None)
                .unwrap();
        assert!((batch[point] - expected).abs() <= 2.0e-5);
        assert!((observed - expected).abs() <= 2.0e-5);
    }
}

#[test]
fn invalid_shapes_and_nonfinite_coordinates_are_reported() {
    let coefficients = [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)];
    assert_eq!(
        interpolate_spectrum_at_longitude_f64(0.0, &coefficients, &[0.0, 0.0]),
        Err(ShtError::InvalidTriangularLength { length: 2 })
    );
    assert_eq!(
        synthesize_at_points_f64(&[0.0], &[0.0, 1.0], &[Complex64::new(1.0, 0.0)]),
        Err(ShtError::PointCountMismatch {
            latitudes: 1,
            longitudes: 2
        })
    );
    assert_eq!(
        synthesize_at_point_f64(f64::NAN, 0.0, &[Complex64::new(1.0, 0.0)], None),
        Err(ShtError::NonFiniteLatitude)
    );
    assert_eq!(
        synthesize_at_point_f64(0.0, f64::INFINITY, &[Complex64::new(1.0, 0.0)], None),
        Err(ShtError::NonFiniteLongitude)
    );
}
