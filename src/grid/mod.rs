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
mod tests;
