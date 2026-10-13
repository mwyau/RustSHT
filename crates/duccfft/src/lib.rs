// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause

//! Standalone one-dimensional FFT crate under development.
//!
//! The crate uses immutable plans with caller-owned execution workspaces.

mod complex;
mod real;
mod simd;

pub use complex::{ComplexFftPlan, ComplexFftWorkspace, FftError};
pub use num_complex::{Complex, Complex64};
pub use real::{RealFftPlan, RealFftWorkspace};
