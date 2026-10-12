// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
//
// This file contains a Rust translation of DUCC's Gauss-Legendre integrator.
/*
 *  This file is part of the MR utility library.
 *
 *  This code is free software; you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation; either version 2 of the License, or
 *  (at your option) any later version.
 *
 *  This code is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  Copyright (C) 2019-2026 Max-Planck-Society, Ignace Bogaert
 *  Author: Martin Reinecke
 *
 *  The FastGL approximations are modified from Ignace Bogaert's code:
 *  https://sourceforge.net/projects/fastgausslegendrequadrature/
 *  Source revision: c1ed01b8fda69d2a378190f0915f1a19762dddbf.
 */

use std::f64::consts::{PI, TAU};

use crate::error::ShtError;

/// Latitude grid used by a rectangular spherical harmonic transform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridKind {
    /// Gauss-Legendre colatitudes and quadrature weights.
    GaussLegendre,
}

/// A regular latitude-longitude grid in canonical core order.
///
/// Latitude rings run from north to south. Longitude samples run eastward from
/// `phi0`, in radians. `ring_weights` include the azimuthal factor `2π` and sum
/// to `4π` for the complete sphere.
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
    /// Constructs a Gauss-Legendre grid.
    ///
    /// # Errors
    ///
    /// Returns an error when either dimension is zero, `phi0` is not finite,
    /// memory allocation fails, or the small-grid root iteration fails.
    pub fn new(kind: GridKind, ntheta: usize, nphi: usize, phi0: f64) -> Result<Self, ShtError> {
        if ntheta == 0 {
            return Err(ShtError::InvalidThetaCount);
        }
        if nphi == 0 {
            return Err(ShtError::InvalidPhiCount);
        }
        if !phi0.is_finite() {
            return Err(ShtError::NonFinitePhi0);
        }

        let (theta, ring_weights) = gauss_legendre_geometry(ntheta)?;
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

fn allocate_f64(len: usize, array: &'static str) -> Result<Vec<f64>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed { array })?;
    values.resize(len, 0.0);
    Ok(values)
}

fn gauss_legendre_geometry(ntheta: usize) -> Result<(Vec<f64>, Vec<f64>), ShtError> {
    let half = ntheta / 2 + ntheta % 2;
    let mut positive_weights = allocate_f64(half, "GL half-weight")?;
    let mut positive_thetas = allocate_f64(half, "GL half-colatitude")?;

    for i in 0..half {
        let root = half - i;
        let (weight, theta) = if ntheta <= 100 {
            iterative_gl_root(ntheta, root)?
        } else {
            bogaert_gl_root(ntheta, root)
        };
        positive_weights[i] = weight;
        positive_thetas[i] = theta;
    }

    let mut theta = allocate_f64(ntheta, "GL colatitude")?;
    let mut ring_weights = allocate_f64(ntheta, "GL ring-weight")?;
    let northern_count = ntheta / 2;

    for i in 0..ntheta {
        let (half_index, ring_theta) = if i < northern_count {
            let half_index = half - 1 - i;
            (half_index, positive_thetas[half_index])
        } else {
            let half_index = i - northern_count;
            (half_index, PI - positive_thetas[half_index])
        };
        theta[i] = ring_theta;
        ring_weights[i] = TAU * positive_weights[half_index];
    }

    Ok((theta, ring_weights))
}

fn one_minus_x2(x: f64) -> f64 {
    let x = x.abs();
    if x > 0.1 {
        (1.0 + x) * (1.0 - x)
    } else {
        1.0 - x * x
    }
}

fn iterative_gl_root(ntheta: usize, root: usize) -> Result<(f64, f64), ShtError> {
    let n = ntheta as f64;
    let t0 = 1.0 - (1.0 - 1.0 / n) / (8.0 * n * n);
    let t1 = 1.0 / (4.0 * n + 2.0);
    let mut x = (PI * (4.0 * root as f64 - 1.0) * t1).cos() * t0;
    let mut derivative = 0.0;
    let mut redo = false;
    let mut converged = false;

    for _ in 0..100 {
        let mut p_prev = 1.0;
        let mut p = x;
        for k in 2..=ntheta {
            let degree = k as f64;
            let p_next = x * p + ((degree - 1.0) / degree) * (x * p - p_prev);
            p_prev = p;
            p = p_next;
        }

        derivative = (p_prev - x * p) * n / one_minus_x2(x);
        let next = x - p / derivative;
        let delta = x - next;
        x = next;

        if redo {
            converged = true;
            break;
        }
        if delta.abs() <= 3e-14 {
            redo = true;
        }
    }

    if !converged || !x.is_finite() || !derivative.is_finite() {
        return Err(ShtError::QuadratureConvergence { ntheta, root });
    }

    let weight = 2.0 / (one_minus_x2(x) * derivative * derivative);
    Ok((weight, x.acos()))
}

// The next three functions translate the modified FastGL approximations in
// DUCC. FastGL is by Ignace Bogaert:
// https://sourceforge.net/projects/fastgausslegendrequadrature/
// See https://doi.org/10.1137/140954969 for the asymptotic expansions.
#[allow(clippy::excessive_precision)]
fn bessel_j0_zero(k: usize) -> f64 {
    const ZEROS: [f64; 12] = [
        2.40482555769577276862163187933,
        5.52007811028631064959660411281,
        8.65372791291101221695419871266,
        11.7915344390142816137430449119,
        14.9309177084877859477625939974,
        18.0710639679109225431478829756,
        21.2116366298792589590783933505,
        24.3524715307493027370579447632,
        27.4934791320402547958772882346,
        30.6346064684319751175495789269,
        33.7758202135735686842385463467,
        36.9170983536640439797694930633,
    ];

    if k <= ZEROS.len() {
        return ZEROS[k - 1];
    }

    let z = PI * (k as f64 - 0.25);
    let r = 1.0 / z;
    let r2 = r * r;
    z + r
        * (0.125
            + r2 * (-31.0 / 384.0
                + r2 * (3779.0 / 15_360.0
                    + r2 * (-6_277_237.0 / 3_440_640.0 + r2 * (2_092_163_573.0 / 82_575_360.0)))))
}

#[allow(clippy::excessive_precision)]
fn bessel_j1_squared_at_j0_zero(k: usize) -> f64 {
    const VALUES: [f64; 13] = [
        0.269514123941916926139021992911,
        0.115780138582203695807812836182,
        0.0736863511364082151406476811985,
        0.0540375731981162820417749182758,
        0.0426614290172430912655106063495,
        0.0352421034909961013587473033648,
        0.0300210701030546726750888157688,
        0.0261473914953080885904584675399,
        0.0231591218246913922652676382178,
        0.0207838291222678576039808057297,
        0.0188504506693176678161056800214,
        0.0172461575696650082995240053542,
        0.0158935181059235978027065594287,
    ];

    if k <= VALUES.len() {
        return VALUES[k - 1];
    }

    let x = 1.0 / (k as f64 - 0.25);
    let x2 = x * x;
    x * (0.202_642_367_284_675_54
        + x2 * x2
            * (-0.000_303_380_429_711_290_25
                + x2 * (0.000_198_924_364_245_969_3
                    + x2 * (-0.000_228_969_902_772_111_65 + x2 * 0.000_433_710_719_130_746_3))))
}

fn horner(x: f64, coefficients: &[f64]) -> f64 {
    coefficients
        .iter()
        .copied()
        .reduce(|value, coefficient| value * x + coefficient)
        .unwrap_or(0.0)
}

#[allow(clippy::excessive_precision)]
fn bogaert_gl_root(ntheta: usize, root0: usize) -> (f64, f64) {
    let half = ntheta / 2 + ntheta % 2;
    let root = if root0 <= half {
        root0
    } else {
        ntheta - root0 + 1
    };
    let inv_nu = 1.0 / (ntheta as f64 + 0.5);
    let nu = bessel_j0_zero(root);
    let mut theta = inv_nu * nu;
    let x = theta * theta;
    let bessel = bessel_j1_squared_at_j0_zero(root);

    let sf1 = horner(
        x,
        &[
            -1.29052996274280508473467968379e-12,
            2.40724685864330121825976175184e-10,
            -3.13148654635992041468855740012e-8,
            0.275573168962061235623801563453e-5,
            -0.148809523713909147898955880165e-3,
            0.416666666665193394525296923981e-2,
            -0.416666666666662959639712457549e-1,
        ],
    );
    let sf2 = horner(
        x,
        &[
            2.20639421781871003734786884322e-9,
            -7.53036771373769326811030753538e-8,
            0.161969259453836261731700382098e-5,
            -0.253300326008232025914059965302e-4,
            0.282116886057560434805998583817e-3,
            -0.209022248387852902722635654229e-2,
            0.815972221772932265640401128517e-2,
        ],
    );
    let sf3 = horner(
        x,
        &[
            -2.97058225375526229899781956673e-8,
            5.55845330223796209655886325712e-7,
            -0.567797841356833081642185432056e-5,
            0.418498100329504574443885193835e-4,
            -0.251395293283965914823026348764e-3,
            0.128654198542845137196151147483e-2,
            -0.416012165620204364833694266818e-2,
        ],
    );
    let wsf1 = horner(
        x,
        &[
            -2.20902861044616638398573427475e-14,
            2.30365726860377376873232578871e-12,
            -1.75257700735423807659851042318e-10,
            1.03756066927916795821098009353e-8,
            -4.63968647553221331251529631098e-7,
            0.149644593625028648361395938176e-4,
            -0.326278659594412170300449074873e-3,
            0.436507936507598105249726413120e-2,
            -0.305555555555553028279487898503e-1,
            0.833333333333333302184063103900e-1,
        ],
    );
    let wsf2 = horner(
        x,
        &[
            3.63117412152654783455929483029e-12,
            7.67643545069893130779501844323e-11,
            -7.12912857233642220650643150625e-9,
            2.11483880685947151466370130277e-7,
            -0.381817918680045468483009307090e-5,
            0.465969530694968391417927388162e-4,
            -0.407297185611335764191683161117e-3,
            0.268959435694729660779984493795e-2,
            -0.111111111111214923138249347172e-1,
        ],
    );
    let wsf3 = horner(
        x,
        &[
            2.01826791256703301806643264922e-9,
            -4.38647122520206649251063212545e-8,
            5.08898347288671653137451093208e-7,
            -0.397933316519135275712977531366e-5,
            0.200559326396458326778521795392e-4,
            -0.422888059282921161626339411388e-4,
            -0.105646050254076140548678457002e-3,
            -0.947969308958577323145923317955e-4,
            0.656966489926484797412985260842e-2,
        ],
    );

    let nu_over_sine = nu / theta.sin();
    let bessel_over_sine = bessel * nu_over_sine;
    let inverse_sinc = inv_nu * inv_nu * nu_over_sine;
    let inverse_sinc2 = inverse_sinc * inverse_sinc;
    theta =
        inv_nu * (nu + theta * inverse_sinc * (sf1 + inverse_sinc2 * (sf2 + inverse_sinc2 * sf3)));
    let denominator = bessel_over_sine
        + bessel_over_sine * inverse_sinc2 * (wsf1 + inverse_sinc2 * (wsf2 + inverse_sinc2 * wsf3));
    let weight = (2.0 * inv_nu) / denominator;
    (weight, theta)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.17e}, expected={expected:.17e}, error={:.3e}",
            (actual - expected).abs()
        );
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
