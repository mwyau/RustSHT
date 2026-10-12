// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex64;

use crate::error::ShtError;
use crate::fft::{RealFftPlan, RealFftWorkspace};

/// Mutable scratch for one transform execution.
///
/// Create a workspace with [`crate::ShtPlan::workspace`]. Use a separate workspace
/// for each concurrent execution.
pub struct ShtWorkspace {
    pub(crate) spectra: Vec<Complex64>,
    pub(crate) row: Vec<f64>,
    pub(crate) epsilon: Vec<f64>,
    pub(crate) fft: RealFftWorkspace,
}

impl ShtWorkspace {
    pub(crate) fn new(
        spectrum_count: usize,
        nphi: usize,
        recurrence_factor_count: usize,
        fft: &RealFftPlan,
    ) -> Result<Self, ShtError> {
        Ok(Self {
            spectra: allocate_complex(spectrum_count, "Fourier spectrum")?,
            row: allocate_real(nphi, "longitude row")?,
            epsilon: allocate_real(recurrence_factor_count, "scalar recurrence factors")?,
            fft: fft.workspace()?,
        })
    }
}

fn allocate_complex(len: usize, array: &'static str) -> Result<Vec<Complex64>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed { array })?;
    values.resize(len, Complex64::new(0.0, 0.0));
    Ok(values)
}

fn allocate_real(len: usize, array: &'static str) -> Result<Vec<f64>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed { array })?;
    values.resize(len, 0.0);
    Ok(values)
}
