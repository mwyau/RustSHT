// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Latitude grid descriptors, geometry, and longitude sampling.

use rustfft::num_traits::{Float, FromPrimitive, ToPrimitive};
use std::f64::consts::TAU;

use crate::error::ShtError;
use quadrature::{clenshaw_curtis_geometry, fejer1_geometry, gauss_legendre_geometry};

pub mod geodesic;
mod quadrature;
pub(crate) mod resampling;

/// Returns the maximum usable harmonic order for one latitude and spin.
///
/// The cutoff is computed in the transform's native precision and is shared
/// by the f32 and f64 Legendre workers.
pub(crate) fn polar_order_limit<T: Float + FromPrimitive + ToPrimitive>(
    lmax: usize,
    spin: usize,
    sin_theta: T,
    cos_theta: T,
) -> usize {
    let lmax_float = T::from_usize(lmax).expect("lmax is representable in f32 and f64");
    let spin_float = T::from_usize(spin).expect("spin is representable in f32 and f64");
    let zero = T::zero();
    let half = T::from_f64(0.5).expect("0.5 is representable in f32 and f64");
    let offset = (T::from_f64(0.01).expect("0.01 is representable") * lmax_float)
        .max(T::from_f64(100.0).expect("100 is representable in f32 and f64"));
    let b = -T::from_f64(2.0).expect("2 is representable in f32 and f64")
        * spin_float
        * cos_theta.abs();
    let t = lmax_float * sin_theta + offset;
    let c = spin_float.powi(2) - t * t;
    let discriminant = b * b - T::from_f64(4.0).expect("4 is representable in f32 and f64") * c;
    if discriminant <= zero {
        return lmax;
    }
    let cutoff = ((-b + discriminant.sqrt()) * half).min(lmax_float) + half;
    if cutoff.is_nan() || cutoff <= zero {
        0
    } else {
        cutoff.to_usize().unwrap_or(usize::MAX)
    }
}

/// Latitude grid used by a rectangular spherical harmonic transform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridKind {
    /// Gauss-Legendre colatitudes and quadrature weights.
    GaussLegendre,
    /// Equally spaced Clenshaw-Curtis colatitudes, including both poles.
    ClenshawCurtis,
    /// Equally spaced Fejer first-rule colatitudes, excluding both poles.
    Fejer1,
}

/// A regular latitude-longitude grid in canonical core order.
///
/// Latitude rings run from north to south. Longitude samples run eastward from
/// `phi0`, in radians. `ring_weights` include the azimuthal factor `2π` and sum
/// to `4π` for the complete sphere. Exact CC and F1 analysis uses
/// prepared-grid meridional resampling in addition to these grid weights.
#[derive(Clone, Debug, PartialEq)]
pub struct GridSpec {
    kind: GridKind,
    ntheta: usize,
    nphi: usize,
    phi0: f64,
    theta: Vec<f64>,
    ring_weights: Vec<f64>,
}

impl GridSpec {
    /// Suggests the smallest longitude count that exactly samples orders
    /// through `mmax` and has no prime factors greater than seven.
    ///
    /// This is an optional grid-sizing suggestion based on common efficient
    /// RustFFT lengths. It does not change the longitude count supplied to
    /// [`GridSpec::new`], and it does not add oversampling for nonlinear
    /// products or other application-specific requirements.
    ///
    /// # Errors
    ///
    /// Returns [`ShtError::IntegerOverflow`] if the sampling limit or a
    /// suitable longitude count cannot be represented as `usize`.
    pub fn fft_friendly_nphi(mmax: usize) -> Result<usize, ShtError> {
        let minimum = mmax
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(ShtError::IntegerOverflow {
                quantity: "minimum longitude count",
            })?;
        let upper_bound = minimum.checked_next_power_of_two().unwrap_or(usize::MAX);
        let mut best = minimum.checked_next_power_of_two();
        find_fft_friendly_nphi(0, 1, minimum, upper_bound, &mut best);
        best.ok_or(ShtError::IntegerOverflow {
            quantity: "FFT-friendly longitude count",
        })
    }

    /// Constructs a grid of the selected kind.
    ///
    /// # Errors
    ///
    /// Returns an error when a dimension is too small for the selected grid,
    /// `phi0` is not finite, memory allocation fails, or quadrature setup fails.
    pub fn new(kind: GridKind, ntheta: usize, nphi: usize, phi0: f64) -> Result<Self, ShtError> {
        if ntheta == 0 || (kind == GridKind::ClenshawCurtis && ntheta < 2) {
            return Err(ShtError::InvalidThetaCount);
        }
        if nphi == 0 {
            return Err(ShtError::InvalidPhiCount);
        }
        if !phi0.is_finite() {
            return Err(ShtError::NonFinitePhi0);
        }

        let (theta, ring_weights) = match kind {
            GridKind::GaussLegendre => gauss_legendre_geometry(ntheta)?,
            GridKind::ClenshawCurtis => clenshaw_curtis_geometry(ntheta)?,
            GridKind::Fejer1 => fejer1_geometry(ntheta)?,
        };
        Ok(Self {
            kind,
            ntheta,
            nphi,
            phi0,
            theta,
            ring_weights,
        })
    }

    /// Returns the grid kind.
    #[must_use]
    pub const fn kind(&self) -> GridKind {
        self.kind
    }

    /// Returns the number of latitude rings.
    #[must_use]
    pub const fn ntheta(&self) -> usize {
        self.ntheta
    }

    /// Returns the number of longitude samples on each ring.
    #[must_use]
    pub const fn nphi(&self) -> usize {
        self.nphi
    }

    /// Returns the longitude of the first sample, in radians.
    #[must_use]
    pub const fn phi0(&self) -> f64 {
        self.phi0
    }

    /// Returns north-to-south colatitudes in radians.
    #[must_use]
    pub fn theta(&self) -> &[f64] {
        &self.theta
    }

    /// Returns latitude-ring quadrature weights, including the `2π` factor.
    ///
    /// Exact CC and F1 analysis uses prepared-grid resampling, not these
    /// weights as a direct per-ring multiplier.
    #[must_use]
    pub fn ring_weights(&self) -> &[f64] {
        &self.ring_weights
    }

    /// Returns the eastward longitude of sample `j`, or `None` if it is out of range.
    #[must_use]
    pub fn longitude(&self, j: usize) -> Option<f64> {
        (j < self.nphi).then(|| self.phi0 + TAU * (j as f64) / (self.nphi as f64))
    }
}

fn find_fft_friendly_nphi(
    factor_index: usize,
    current: usize,
    minimum: usize,
    upper_bound: usize,
    best: &mut Option<usize>,
) {
    const FACTORS: [usize; 4] = [2, 3, 5, 7];

    if current > upper_bound || best.is_some_and(|value| current >= value) {
        return;
    }
    if factor_index == FACTORS.len() {
        if current >= minimum {
            *best = Some(current);
        }
        return;
    }

    let factor = FACTORS[factor_index];
    let mut candidate = current;
    loop {
        find_fft_friendly_nphi(factor_index + 1, candidate, minimum, upper_bound, best);
        let Some(next) = candidate.checked_mul(factor) else {
            break;
        };
        if next > upper_bound {
            break;
        }
        candidate = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn reference_polar_order_limit_f64(
        lmax: usize,
        spin: usize,
        sin_theta: f64,
        cos_theta: f64,
    ) -> usize {
        let lmax_float = lmax as f64;
        let offset = (0.01 * lmax_float).max(100.0);
        let b = -2.0 * spin as f64 * cos_theta.abs();
        let t = lmax_float * sin_theta + offset;
        let c = (spin as f64).powi(2) - t * t;
        let discriminant = b * b - 4.0 * c;
        if discriminant <= 0.0 {
            return lmax;
        }
        (((-b + discriminant.sqrt()) * 0.5).min(lmax_float) + 0.5) as usize
    }

    fn reference_polar_order_limit_f32(
        lmax: usize,
        spin: usize,
        sin_theta: f32,
        cos_theta: f32,
    ) -> usize {
        let lmax_float = lmax as f32;
        let offset = (0.01 * lmax_float).max(100.0);
        let b = -2.0 * spin as f32 * cos_theta.abs();
        let t = lmax_float * sin_theta + offset;
        let c = (spin as f32).powi(2) - t * t;
        let discriminant = b * b - 4.0 * c;
        if discriminant <= 0.0 {
            return lmax;
        }
        (((-b + discriminant.sqrt()) * 0.5).min(lmax_float) + 0.5) as usize
    }

    #[test]
    fn shared_polar_order_limit_matches_native_precision_formulas() {
        fn check(lmax: usize, spin: usize, sin_theta: f64, cos_theta: f64) {
            let expected_f64 = reference_polar_order_limit_f64(lmax, spin, sin_theta, cos_theta);
            assert_eq!(
                polar_order_limit(lmax, spin, sin_theta, cos_theta),
                expected_f64
            );
            assert_eq!(
                <f64 as crate::legendre::ShtScalar>::polar_order_limit(
                    lmax, spin, sin_theta, cos_theta
                ),
                expected_f64
            );
            let sin_theta = sin_theta as f32;
            let cos_theta = cos_theta as f32;
            let expected_f32 = reference_polar_order_limit_f32(lmax, spin, sin_theta, cos_theta);
            assert_eq!(
                polar_order_limit(lmax, spin, sin_theta, cos_theta),
                expected_f32
            );
            assert_eq!(
                <f32 as crate::legendre::ShtScalar>::polar_order_limit(
                    lmax, spin, sin_theta, cos_theta
                ),
                expected_f32
            );
        }

        let angles = [0.0, 1.0e-5, 0.1, 0.5, PI / 2.0, PI - 1.0e-5, PI];
        for lmax in [0, 1, 2, 15, 99, 100, 101, 255, 511, 720, 1023, 2048, 10_000] {
            for spin in [0, 1, 2, 4] {
                for angle in angles {
                    let (sin_theta, cos_theta) = angle.sin_cos();
                    check(lmax, spin, sin_theta, cos_theta);
                }
            }
        }

        for kind in [
            GridKind::GaussLegendre,
            GridKind::ClenshawCurtis,
            GridKind::Fejer1,
        ] {
            for lmax in [255, 511, 720, 1023] {
                let ntheta = lmax + 1;
                let grid = GridSpec::new(kind, ntheta, 1, 0.0).unwrap();
                for spin in [0, 1, 2, 4] {
                    for &theta in grid.theta() {
                        check(lmax, spin, theta.sin(), theta.cos());
                    }
                }
            }
        }
    }

    fn is_fft_friendly(mut value: usize) -> bool {
        for factor in [2, 3, 5, 7] {
            while value.is_multiple_of(factor) {
                value /= factor;
            }
        }
        value == 1
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "grid value differs from reference beyond tolerance"
        );
    }

    #[test]
    fn fft_friendly_nphi_is_minimal_and_sampling_safe() {
        for mmax in 0..=1024 {
            let minimum = 2 * mmax + 1;
            let actual = GridSpec::fft_friendly_nphi(mmax).unwrap();
            assert!(actual >= minimum);
            assert!(is_fft_friendly(actual));
            assert!((minimum..actual).all(|candidate| !is_fft_friendly(candidate)));
        }

        for (mmax, expected) in [
            (63, 128),
            (72, 147),
            (127, 256),
            (255, 512),
            (511, 1024),
            (720, 1458),
            (1023, 2048),
        ] {
            assert_eq!(GridSpec::fft_friendly_nphi(mmax), Ok(expected));
        }

        let error = GridSpec::fft_friendly_nphi(usize::MAX / 2).unwrap_err();
        assert!(matches!(error, ShtError::IntegerOverflow { .. }));
    }

    #[test]
    fn small_rules_match_closed_form() {
        let one = GridSpec::new(GridKind::GaussLegendre, 1, 1, 0.0).unwrap();
        close(one.theta()[0], PI / 2.0, 2e-16);
        close(one.ring_weights()[0], 4.0 * PI, 2e-15);

        let two = GridSpec::new(GridKind::GaussLegendre, 2, 3, 0.0).unwrap();
        close(two.theta()[0], (1.0_f64 / 3.0).sqrt().acos(), 3e-16);
        close(two.theta()[1], PI - (1.0_f64 / 3.0).sqrt().acos(), 3e-16);
        close(two.ring_weights()[0], 2.0 * PI, 2e-15);

        let three = GridSpec::new(GridKind::GaussLegendre, 3, 4, 0.0).unwrap();
        close(three.theta()[0].cos(), (3.0_f64 / 5.0).sqrt(), 3e-16);
        close(three.theta()[1], PI / 2.0, 3e-16);
        close(three.ring_weights()[0], 10.0 * PI / 9.0, 3e-15);
        close(three.ring_weights()[1], 16.0 * PI / 9.0, 3e-15);
    }

    #[test]
    fn gauss_legendre_moments_are_exact_for_small_rules() {
        for ntheta in 1..=16 {
            let grid = GridSpec::new(GridKind::GaussLegendre, ntheta, 1, 0.0).unwrap();
            for degree in 0..2 * ntheta {
                let actual: f64 = grid
                    .theta()
                    .iter()
                    .zip(grid.ring_weights())
                    .map(|(&theta, &weight)| theta.cos().powi(degree as i32) * weight / TAU)
                    .sum();
                let expected = if degree % 2 == 0 {
                    2.0 / (degree as f64 + 1.0)
                } else {
                    0.0
                };
                close(actual, expected, 3e-14);
            }
        }
    }

    #[test]
    fn both_quadrature_algorithms_are_symmetric_and_normalized() {
        for ntheta in [99, 100, 101, 127, 255, 1024] {
            let grid = GridSpec::new(GridKind::GaussLegendre, ntheta, 1, 0.0).unwrap();
            for i in 0..ntheta {
                close(grid.theta()[i], PI - grid.theta()[ntheta - 1 - i], 3e-15);
                close(
                    grid.ring_weights()[i],
                    grid.ring_weights()[ntheta - 1 - i],
                    3e-15,
                );
                assert!(grid.ring_weights()[i] > 0.0);
            }
            close(grid.ring_weights().iter().sum(), 4.0 * PI, 2e-13);
        }
    }

    #[test]
    #[allow(clippy::excessive_precision)]
    fn selected_nodes_and_weights_match_pinned_ducc() {
        // Values from GL_Integrator at DUCC revision
        // c1ed01b8fda69d2a378190f0915f1a19762dddbf, with weights multiplied by 2π.
        let reference = [
            (99, 0, 0.024168999361183122, 0.004709081805298769),
            (100, 0, 0.023928513799576875, 0.0046158446368925986),
            (101, 0, 0.023692766811651021, 0.004525349275429071),
            (127, 31, 0.782325906824648, 0.10913384776173643),
            (255, 63, 0.7838631013671257, 0.05454494109708152),
            (1024, 256, 0.7873148222088625, 0.013650025993590937),
        ];

        for (ntheta, ring, expected_theta, expected_weight) in reference {
            let grid = GridSpec::new(GridKind::GaussLegendre, ntheta, 1, 0.0).unwrap();
            close(grid.theta()[ring], expected_theta, 2.5e-15);
            close(grid.ring_weights()[ring], expected_weight, 2e-15);
        }
    }

    #[test]
    #[allow(clippy::excessive_precision)]
    fn clenshaw_curtis_nodes_and_weights_match_ducc_0410() {
        let references = [
            (2, vec![TAU, TAU]),
            (
                3,
                vec![
                    2.094_395_102_393_195_7,
                    8.377_580_409_572_783,
                    2.094_395_102_393_195_7,
                ],
            ),
            (
                4,
                vec![
                    0.698_131_700_797_731_6,
                    5.585_053_606_381_853,
                    5.585_053_606_381_853,
                    0.698_131_700_797_731_6,
                ],
            ),
            (
                5,
                vec![
                    0.418_879_020_478_639_34,
                    3.351_032_163_829_112_5,
                    5.026_548_245_743_669,
                    3.351_032_163_829_112_5,
                    0.418_879_020_478_639_34,
                ],
            ),
        ];
        for (ntheta, expected_weights) in references {
            let grid = GridSpec::new(GridKind::ClenshawCurtis, ntheta, 1, 0.0).unwrap();
            for (ring, (&theta, &weight)) in
                grid.theta().iter().zip(grid.ring_weights()).enumerate()
            {
                let north_ring = ring.min(ntheta - 1 - ring);
                let north_theta = PI * north_ring as f64 / (ntheta - 1) as f64;
                let expected_theta = if ring <= ntheta - 1 - ring {
                    north_theta
                } else {
                    PI - north_theta
                };
                close(theta, expected_theta, 0.0);
                close(weight, expected_weights[ring], 2e-15);
            }
            close(grid.ring_weights().iter().sum(), 4.0 * PI, 2e-14);
        }

        for ntheta in 2..=20 {
            let grid = GridSpec::new(GridKind::ClenshawCurtis, ntheta, 1, 0.0).unwrap();
            for degree in 0..ntheta {
                let actual: f64 = grid
                    .theta()
                    .iter()
                    .zip(grid.ring_weights())
                    .map(|(&theta, &weight)| theta.cos().powi(degree as i32) * weight / TAU)
                    .sum();
                let expected = if degree % 2 == 0 {
                    2.0 / (degree as f64 + 1.0)
                } else {
                    0.0
                };
                close(actual, expected, 4e-14);
            }
        }
    }

    #[test]
    #[allow(clippy::excessive_precision)]
    fn fejer1_nodes_and_weights_match_ducc_0410() {
        let references = [
            (1, vec![12.566_370_614_359_172]),
            (2, vec![TAU, TAU]),
            (
                3,
                vec![
                    2.792_526_803_190_927,
                    6.981_317_007_977_318,
                    2.792_526_803_190_927,
                ],
            ),
            (
                4,
                vec![
                    1.660_631_674_203_671_5,
                    4.622_553_632_975_915,
                    4.622_553_632_975_915,
                    1.660_631_674_203_671_5,
                ],
            ),
            (
                5,
                vec![
                    1.054_200_549_522_407,
                    3.302_141_263_455_439_5,
                    3.853_686_988_403_479_2,
                    3.302_141_263_455_439_5,
                    1.054_200_549_522_407,
                ],
            ),
            (
                6,
                vec![
                    0.745_569_186_077_503_9,
                    2.373_647_782_712_288,
                    3.163_968_338_389_794,
                    3.163_968_338_389_794,
                    2.373_647_782_712_288,
                    0.745_569_186_077_503_9,
                ],
            ),
        ];
        for (ntheta, expected_weights) in references {
            let grid = GridSpec::new(GridKind::Fejer1, ntheta, 1, 0.0).unwrap();
            for (ring, (&theta, &weight)) in
                grid.theta().iter().zip(grid.ring_weights()).enumerate()
            {
                let north_ring = ring.min(ntheta - 1 - ring);
                let north_theta = PI * (north_ring as f64 + 0.5) / ntheta as f64;
                let expected_theta = if ring <= ntheta - 1 - ring {
                    north_theta
                } else {
                    PI - north_theta
                };
                close(theta, expected_theta, 0.0);
                close(weight, expected_weights[ring], 2e-15);
            }
            close(grid.ring_weights().iter().sum(), 4.0 * PI, 2e-14);
        }

        for ntheta in 1..=20 {
            let grid = GridSpec::new(GridKind::Fejer1, ntheta, 1, 0.0).unwrap();
            for degree in 0..ntheta {
                let actual: f64 = grid
                    .theta()
                    .iter()
                    .zip(grid.ring_weights())
                    .map(|(&theta, &weight)| theta.cos().powi(degree as i32) * weight / TAU)
                    .sum();
                let expected = if degree % 2 == 0 {
                    2.0 / (degree as f64 + 1.0)
                } else {
                    0.0
                };
                close(actual, expected, 5e-14);
            }
        }
    }

    #[test]
    fn grid_rejects_invalid_shapes_and_longitude_origin() {
        assert_eq!(
            GridSpec::new(GridKind::GaussLegendre, 0, 1, 0.0),
            Err(ShtError::InvalidThetaCount)
        );
        assert_eq!(
            GridSpec::new(GridKind::GaussLegendre, 1, 0, 0.0),
            Err(ShtError::InvalidPhiCount)
        );
        assert_eq!(
            GridSpec::new(GridKind::GaussLegendre, 1, 1, f64::NAN),
            Err(ShtError::NonFinitePhi0)
        );
    }

    #[test]
    fn longitude_samples_follow_phi0_and_eastward_order() {
        let grid = GridSpec::new(GridKind::GaussLegendre, 2, 4, 0.25).unwrap();
        close(grid.longitude(0).unwrap(), 0.25, 0.0);
        close(grid.longitude(1).unwrap(), 0.25 + PI / 2.0, 2e-16);
        assert_eq!(grid.longitude(4), None);
    }
}
