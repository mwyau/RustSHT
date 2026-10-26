// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use super::{associated_legendre_f32, associated_legendre_f64};
use crate::ShtError;

#[test]
fn pointwise_normalization_phase_and_packed_order_match_closed_forms() {
    let latitude = 30.0_f64;
    let values = associated_legendre_f64(latitude, 2).unwrap();
    let x = latitude.to_radians().sin();
    let sin_theta = latitude.to_radians().cos();

    assert_eq!(values.len(), 6);
    assert!((values[0] - 1.0 / 2.0_f64.sqrt()).abs() < 2.0e-16);
    assert!((values[1] - (1.5_f64).sqrt() * x).abs() < 3.0e-16);
    assert!((values[2] - (5.0_f64 / 8.0).sqrt() * (3.0 * x * x - 1.0)).abs() < 4.0e-16);
    assert!((values[3] - (3.0_f64 / 4.0).sqrt() * sin_theta).abs() < 3.0e-16);
    assert!((values[5] - (15.0_f64 / 16.0).sqrt() * sin_theta * sin_theta).abs() < 5.0e-16);
}

#[test]
fn poles_are_exact_and_follow_degree_parity() {
    let north = associated_legendre_f64(90.0, 12).unwrap();
    let south = associated_legendre_f64(-90.0, 12).unwrap();
    assert!((north[0] - 1.0 / 2.0_f64.sqrt()).abs() < 2.0e-16);
    assert!((north[1] - (3.0_f64 / 2.0).sqrt()).abs() < 4.0e-16);
    assert_eq!(south[1], -north[1]);
    for m in 1..=12 {
        let first = (m * 13) - m * (m - 1) / 2;
        assert!(
            north[first..first + (13 - m)]
                .iter()
                .all(|&value| value == 0.0)
        );
        assert!(
            south[first..first + (13 - m)]
                .iter()
                .all(|&value| value == 0.0)
        );
    }
}

#[test]
fn f32_values_track_f64_and_high_degree_values_remain_finite() {
    let single = associated_legendre_f32(-37.25, 64).unwrap();
    let double = associated_legendre_f64(-37.25, 64).unwrap();
    assert_eq!(single.len(), double.len());
    let mut maximum_absolute_error = 0.0_f64;
    for (&single_value, &double_value) in single.iter().zip(&double) {
        assert!(single_value.is_finite());
        let error = (single_value as f64 - double_value).abs();
        maximum_absolute_error = maximum_absolute_error.max(error);
    }
    assert!(maximum_absolute_error <= 6.0e-6);

    let high = associated_legendre_f64(89.999, 1023).unwrap();
    assert!(high.iter().all(|value| value.is_finite()));
}

#[test]
fn nonfinite_latitude_is_rejected() {
    assert_eq!(
        associated_legendre_f32(f32::NAN, 2),
        Err(ShtError::NonFiniteLatitude)
    );
}
