// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex64;

use crate::error::ShtError;
use crate::fft::{RealFftPlan, RealFftWorkspace};
use crate::legendre::LATITUDE_BLOCK_SIZE;

/// Mutable scratch for one transform execution.
///
/// Create a workspace with [`crate::ShtPlan::workspace`]. Use a separate workspace
/// for each concurrent execution.
pub struct ShtWorkspace {
    pub(crate) spectra: Vec<Complex64>,
    pub(crate) row: Vec<f64>,
    pub(crate) epsilon: Vec<f64>,
    pub(crate) theta_sin: Vec<f64>,
    pub(crate) theta_cos: Vec<f64>,
    pub(crate) active_rings: Vec<bool>,
    pub(crate) recurrence_previous: Vec<f64>,
    pub(crate) recurrence_current: Vec<f64>,
    pub(crate) recurrence_next: Vec<f64>,
    pub(crate) recurrence_exponents: Vec<i64>,
    pub(crate) fft: RealFftWorkspace,
}

impl ShtWorkspace {
    pub(crate) fn new(
        spectrum_count: usize,
        nphi: usize,
        ntheta: usize,
        recurrence_factor_count: usize,
        fft: &RealFftPlan,
    ) -> Result<Self, ShtError> {
        let recurrence_scratch_len = ntheta.min(LATITUDE_BLOCK_SIZE);
        Ok(Self {
            spectra: allocate_complex(spectrum_count, "Fourier spectrum")?,
            row: allocate_real(nphi, "longitude row")?,
            epsilon: allocate_real(recurrence_factor_count, "scalar recurrence factors")?,
            theta_sin: allocate_real(ntheta, "latitude sine values")?,
            theta_cos: allocate_real(ntheta, "latitude cosine values")?,
            active_rings: allocate_bools(ntheta)?,
            recurrence_previous: allocate_real(
                recurrence_scratch_len,
                "previous recurrence values",
            )?,
            recurrence_current: allocate_real(recurrence_scratch_len, "current recurrence values")?,
            recurrence_next: allocate_real(recurrence_scratch_len, "next recurrence values")?,
            recurrence_exponents: allocate_exponents(recurrence_scratch_len)?,
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

fn allocate_exponents(len: usize) -> Result<Vec<i64>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed {
            array: "recurrence exponents",
        })?;
    values.resize(len, 0);
    Ok(values)
}

fn allocate_bools(len: usize) -> Result<Vec<bool>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed {
            array: "active latitude rings",
        })?;
    values.resize(len, false);
    Ok(values)
}
