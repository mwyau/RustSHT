// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

//! Spherical harmonic transforms on regular latitude-longitude grids.
//!
//! The first implemented geometry is the Gauss-Legendre grid. Latitude rings
//! run north to south; longitude samples run eastward from `phi0` in radians.

mod error;
mod fft;
mod grid;
mod harmonics;
mod legendre;
mod plan;
mod spin_transform;
mod transform;
mod workspace;

pub use error::ShtError;
pub use grid::{GridKind, GridSpec};
pub use harmonics::HarmonicSpec;
pub use plan::ShtPlan;
pub use workspace::ShtWorkspace;

/// Complex double-precision harmonic coefficient.
pub type Complex64 = num_complex::Complex64;
