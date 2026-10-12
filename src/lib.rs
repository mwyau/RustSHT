// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

//! Spherical harmonic transforms on regular latitude-longitude grids.
//!
//! The first implemented geometry is the Gauss-Legendre grid. Latitude rings
//! run north to south; longitude samples run eastward from `phi0` in radians.

mod error;
mod grid;

pub use error::ShtError;
pub use grid::{GridKind, GridSpec};
