// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

//! Numerical operations used by the pyspharm API.
//!
//! These functions use pyspharm's normalized, non-Condon-Shortley associated
//! Legendre convention. They do not change the normalization used by the
//! regular RustSHT transform plans.

mod geodesic;
mod interpolation;
mod legendre;

pub use geodesic::{GeodesicPoint, geodesic_points};
pub use interpolation::{
    specintrp_f32, specintrp_f64, synthesize_at_point_f32, synthesize_at_point_f64,
    synthesize_at_points_f32, synthesize_at_points_f64,
};
pub use legendre::{associated_legendre_f32, associated_legendre_f64};
