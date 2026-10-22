// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

//! Spherical harmonic transforms on regular latitude-longitude grids.
//!
//! Gauss-Legendre, Clenshaw-Curtis and Fejer first-rule grids use
//! north-to-south latitude rings; longitude samples run eastward from `phi0`
//! in radians.

mod error;
mod fft;
mod grid;
mod harmonics;
mod legendre;
mod meridional;
mod plan;
pub mod pyspharm;
mod spin_transform;
mod transform;
mod workspace;

pub use error::ShtError;
pub use grid::{GridKind, GridSpec};
pub use harmonics::HarmonicSpec;
pub use plan::{LegendreReuseEstimate, ShtPlan};
pub use workspace::ShtWorkspace;

/// Complex double-precision harmonic coefficient.
pub type Complex64 = num_complex::Complex64;

/// Complex single-precision harmonic coefficient.
pub type Complex32 = num_complex::Complex32;
