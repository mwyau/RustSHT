// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use super::geodesic_points;
use crate::ShtError;

#[test]
fn subdivision_counts_poles_and_unit_vectors_are_valid() {
    for subdivision in 1..=7 {
        let points = geodesic_points(subdivision).unwrap();
        assert_eq!(points.len(), 10 * (subdivision - 1).pow(2) + 2);
        assert_eq!(points[0].unit_vector(), [0.0, 0.0, 1.0]);
        assert_eq!(points[1].unit_vector(), [0.0, 0.0, -1.0]);
        for point in &points {
            let [x, y, z] = point.unit_vector();
            assert!((x.mul_add(x, y.mul_add(y, z * z)) - 1.0).abs() < 2.0e-15);
            assert!(point.latitude_radians().is_finite());
            assert!(point.longitude_radians().is_finite());
        }
        for (index, point) in points.iter().enumerate() {
            for other in &points[..index] {
                let left = point.unit_vector();
                let right = other.unit_vector();
                let distance_squared = (left[0] - right[0]).powi(2)
                    + (left[1] - right[1]).powi(2)
                    + (left[2] - right[2]).powi(2);
                assert!(distance_squared > 1.0e-16);
            }
        }
        assert_eq!(points, geodesic_points(subdivision).unwrap());
    }
}

#[test]
fn subdivision_two_matches_the_reference_vertex_order_and_coordinates() {
    let points = geodesic_points(2).unwrap();
    let expected = [
        (90.0, 0.0),
        (-90.0, 0.0),
        (26.565_05, 180.0),
        (26.565_05, 252.0),
        (26.565_05, 324.0),
        (26.565_05, 36.0),
        (26.565_05, 108.0),
        (-26.565_05, 216.0),
        (-26.565_05, 288.0),
        (-26.565_05, 360.0),
        (-26.565_05, 72.0),
        (-26.565_05, 144.0),
    ];
    assert_eq!(points.len(), expected.len());
    for (point, (expected_lat, expected_lon)) in points.iter().zip(expected) {
        let (latitude, longitude) = point.pyspharm_lat_lon_degrees();
        assert!((latitude - expected_lat).abs() < 2.0e-4);
        assert!((longitude - expected_lon).abs() < 2.0e-4);
    }
}

#[test]
fn invalid_and_overflowing_subdivisions_are_rejected() {
    assert_eq!(
        geodesic_points(0),
        Err(ShtError::InvalidGeodesicSubdivision { subdivision: 0 })
    );
    assert_eq!(
        geodesic_points(usize::MAX),
        Err(ShtError::IntegerOverflow {
            quantity: "geodesic point count"
        })
    );
}
