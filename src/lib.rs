// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

//! Spherical harmonic transforms on regular latitude-longitude grids.
//!
//! The first implemented geometry is the Gauss-Legendre grid. Latitude rings
//! run north to south; longitude samples run eastward from `phi0` in radians.

mod error;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "The adapter is staged before the transform kernels that consume it."
    )
)]
mod fft;
mod grid;
mod harmonics;

pub use error::ShtError;
pub use grid::{GridKind, GridSpec};
pub use harmonics::HarmonicSpec;

/// Complex double-precision harmonic coefficient.
pub type Complex64 = num_complex::Complex64;
