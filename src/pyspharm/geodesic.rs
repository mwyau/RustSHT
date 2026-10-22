// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::f64::consts::PI;

use crate::error::ShtError;

/// One unit direction in an icosahedral geodesic point set.
///
/// The Cartesian coordinates use the usual right-handed eastward convention.
/// Spherical coordinate accessors return north-positive latitude and longitude
/// in radians. The pyspharm conversion method preserves its historical degree
/// convention and is intended for compatibility adapters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeodesicPoint {
    xyz: [f64; 3],
}

impl GeodesicPoint {
    /// Returns the unit Cartesian direction as x, y, z.
    #[must_use]
    pub const fn unit_vector(self) -> [f64; 3] {
        self.xyz
    }

    /// Returns geographic latitude in radians, positive northward.
    #[must_use]
    pub fn latitude_radians(self) -> f64 {
        self.xyz[2].atan2(self.xyz[0].hypot(self.xyz[1]))
    }

    /// Returns eastward longitude in radians, in the range [-pi, pi].
    ///
    /// Longitude is zero at either pole, where it is otherwise undefined.
    #[must_use]
    pub fn longitude_radians(self) -> f64 {
        if self.xyz[0] == 0.0 && self.xyz[1] == 0.0 {
            0.0
        } else {
            self.xyz[1].atan2(self.xyz[0])
        }
    }

    /// Returns the latitude/longitude pair used by pyspharm, in degrees.
    ///
    /// Pyspharm places the poles first and assigns them longitudes of zero. For
    /// other points its wrapper returns south-positive latitude and adds 180
    /// degrees to the Cartesian azimuth. The result is ordered latitude first.
    #[must_use]
    pub fn pyspharm_lat_lon_degrees(self) -> (f32, f32) {
        if self.xyz[0] == 0.0 && self.xyz[1] == 0.0 {
            let latitude = if self.xyz[2] >= 0.0 { 90.0 } else { -90.0 };
            return (latitude_f32(latitude), 0.0);
        }
        let latitude = -self.latitude_radians().to_degrees();
        let longitude = self.longitude_radians().to_degrees() + 180.0;
        (latitude_f32(latitude), latitude_f32(longitude))
    }
}

/// Generates pyspharm's deterministic icosahedral geodesic point ordering.
///
/// Subdivision one contains only the north and south poles. For subdivision m,
/// the output count is 10*(m-1)^2+2. Points are ordered as the two poles, then
/// the row-major unfolded icosahedron order with the five longitudinal sectors
/// interleaved in the innermost position. Coordinates are stored as f64 unit
/// vectors; callers can obtain standard spherical coordinates from each point.
///
/// # Errors
///
/// Returns an error if subdivision is zero, the point count overflows, or the
/// point array cannot be allocated.
pub fn geodesic_points(subdivision: usize) -> Result<Vec<GeodesicPoint>, ShtError> {
    if subdivision == 0 {
        return Err(ShtError::InvalidGeodesicSubdivision { subdivision });
    }
    let edge_steps = subdivision - 1;
    let edge_square = edge_steps
        .checked_mul(edge_steps)
        .ok_or(ShtError::IntegerOverflow {
            quantity: "geodesic point count",
        })?;
    let point_count = edge_square
        .checked_mul(10)
        .and_then(|count| count.checked_add(2))
        .ok_or(ShtError::IntegerOverflow {
            quantity: "geodesic point count",
        })?;
    let mut points = Vec::new();
    points
        .try_reserve_exact(point_count)
        .map_err(|_| ShtError::AllocationFailed {
            array: "geodesic points",
        })?;
    points.push(GeodesicPoint {
        xyz: [0.0, 0.0, 1.0],
    });
    points.push(GeodesicPoint {
        xyz: [0.0, 0.0, -1.0],
    });
    if subdivision == 1 {
        return Ok(points);
    }

    let row_count = edge_steps.checked_mul(2).ok_or(ShtError::IntegerOverflow {
        quantity: "geodesic row count",
    })?;
    let layout = IcosahedronLayout::new(subdivision);

    for row in 0..row_count {
        for column in 0..edge_steps {
            for sector in 0..5 {
                let phi = sector as f64 * layout.dphi;
                let point = layout.unfolded_point(row, column, phi);
                points.push(GeodesicPoint {
                    xyz: normalize(point),
                });
            }
        }
    }
    debug_assert_eq!(points.len(), point_count);
    Ok(points)
}

struct IcosahedronLayout {
    subdivision: usize,
    dphi: f64,
    half_dphi: f64,
    theta_north: f64,
    theta_south: f64,
}

impl IcosahedronLayout {
    fn new(subdivision: usize) -> Self {
        let dphi = 2.0 * PI / 5.0;
        let half_dphi = dphi * 0.5;
        let beta = dphi.cos();
        let theta_north = (beta / (1.0 - beta)).acos();
        Self {
            subdivision,
            dphi,
            half_dphi,
            theta_north,
            theta_south: PI - theta_north,
        }
    }

    fn unfolded_point(&self, row: usize, column: usize, phi: f64) -> [f64; 3] {
        let x1 = spherical(self.theta_south, phi);
        let x2 = [0.0, 0.0, -1.0];
        let x3 = spherical(self.theta_south, phi + self.dphi);
        let x4 = spherical(self.theta_north, phi + self.half_dphi);
        let x5 = spherical(self.theta_north, phi + 3.0 * self.half_dphi);
        let x6 = [0.0, 0.0, 1.0];
        let denominator = (self.subdivision - 1) as f64;

        if row < self.subdivision - 1 {
            let row_fraction = row as f64 / denominator;
            let column_fraction = column as f64 / denominator;
            if column <= row {
                add_scaled(
                    add_scaled(x1, subtract(x4, x1), row_fraction),
                    subtract(x3, x4),
                    column_fraction,
                )
            } else {
                add_scaled(
                    add_scaled(x1, subtract(x2, x1), column_fraction),
                    subtract(x3, x2),
                    row_fraction,
                )
            }
        } else {
            let upper_row = row - (self.subdivision - 1);
            let row_fraction = upper_row as f64 / denominator;
            let column_fraction = column as f64 / denominator;
            if column <= upper_row {
                add_scaled(
                    add_scaled(x4, subtract(x6, x4), row_fraction),
                    subtract(x5, x6),
                    column_fraction,
                )
            } else {
                add_scaled(
                    add_scaled(x4, subtract(x3, x4), column_fraction),
                    subtract(x5, x3),
                    row_fraction,
                )
            }
        }
    }
}

fn spherical(theta: f64, phi: f64) -> [f64; 3] {
    let (sin_theta, cos_theta) = theta.sin_cos();
    let (sin_phi, cos_phi) = phi.sin_cos();
    [sin_theta * cos_phi, sin_theta * sin_phi, cos_theta]
}

fn add_scaled(origin: [f64; 3], delta: [f64; 3], scale: f64) -> [f64; 3] {
    [
        origin[0] + scale * delta[0],
        origin[1] + scale * delta[1],
        origin[2] + scale * delta[2],
    ]
}

fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn normalize(vector: [f64; 3]) -> [f64; 3] {
    let norm = vector[0]
        .mul_add(
            vector[0],
            vector[1].mul_add(vector[1], vector[2] * vector[2]),
        )
        .sqrt();
    [vector[0] / norm, vector[1] / norm, vector[2] / norm]
}

fn latitude_f32(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
#[path = "geodesic_tests.rs"]
mod tests;
