// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex64;

use crate::error::ShtError;
use crate::fft::{RealFftPlan, RealFftWorkspace};
use crate::legendre::LATITUDE_BLOCK_SIZE;
use crate::meridional::CCResamplingPlan;

/// Mutable scratch for one transform execution.
///
/// Create a workspace with [`crate::ShtPlan::workspace`]. Use a separate workspace
/// for each concurrent execution.
pub struct ShtWorkspace {
    pub(crate) spectra: Vec<Complex64>,
    pub(crate) spin_spectra: Vec<Complex64>,
    pub(crate) prepared_spectra: Vec<Complex64>,
    pub(crate) prepared_spin_spectra: Vec<Complex64>,
    pub(crate) cc_profile: Vec<Complex64>,
    pub(crate) cc_fft_scratch: Vec<Complex64>,
    pub(crate) row: Vec<f64>,
    pub(crate) theta_sin: Vec<f64>,
    pub(crate) theta_cos: Vec<f64>,
    pub(crate) ring_order_limits: Vec<usize>,
    pub(crate) recurrence_workers: Vec<RecurrenceWorkspace>,
    pub(crate) ring_group_size: usize,
    pub(crate) fft: RealFftWorkspace,
}

pub(crate) struct RecurrenceWorkspace {
    pub(crate) epsilon: Vec<f64>,
    pub(crate) previous: Vec<f64>,
    pub(crate) current: Vec<f64>,
    pub(crate) next: Vec<f64>,
    pub(crate) exponents: Vec<i64>,
    pub(crate) spin_alpha: Vec<f64>,
    pub(crate) spin_coefficient_a: Vec<f64>,
    pub(crate) spin_coefficient_b: Vec<f64>,
    pub(crate) spin_previous_plus: Vec<f64>,
    pub(crate) spin_current_plus: Vec<f64>,
    pub(crate) spin_next_plus: Vec<f64>,
    pub(crate) spin_exponents_plus: Vec<i64>,
    pub(crate) spin_previous_minus: Vec<f64>,
    pub(crate) spin_current_minus: Vec<f64>,
    pub(crate) spin_next_minus: Vec<f64>,
    pub(crate) spin_exponents_minus: Vec<i64>,
}

impl RecurrenceWorkspace {
    fn new(
        recurrence_factor_count: usize,
        spin_recurrence_factor_count: usize,
        recurrence_scratch_len: usize,
    ) -> Result<Self, ShtError> {
        Ok(Self {
            epsilon: allocate_real(recurrence_factor_count, "scalar recurrence factors")?,
            previous: allocate_real(recurrence_scratch_len, "previous recurrence values")?,
            current: allocate_real(recurrence_scratch_len, "current recurrence values")?,
            next: allocate_real(recurrence_scratch_len, "next recurrence values")?,
            exponents: allocate_exponents(recurrence_scratch_len)?,
            spin_alpha: allocate_real(spin_recurrence_factor_count, "spin recurrence alpha")?,
            spin_coefficient_a: allocate_real(
                spin_recurrence_factor_count,
                "spin recurrence coefficient A",
            )?,
            spin_coefficient_b: allocate_real(
                spin_recurrence_factor_count,
                "spin recurrence coefficient B",
            )?,
            spin_previous_plus: allocate_real(
                recurrence_scratch_len,
                "positive-spin previous recurrence values",
            )?,
            spin_current_plus: allocate_real(
                recurrence_scratch_len,
                "positive-spin current recurrence values",
            )?,
            spin_next_plus: allocate_real(
                recurrence_scratch_len,
                "positive-spin next recurrence values",
            )?,
            spin_exponents_plus: allocate_exponents(recurrence_scratch_len)?,
            spin_previous_minus: allocate_real(
                recurrence_scratch_len,
                "negative-spin previous recurrence values",
            )?,
            spin_current_minus: allocate_real(
                recurrence_scratch_len,
                "negative-spin current recurrence values",
            )?,
            spin_next_minus: allocate_real(
                recurrence_scratch_len,
                "negative-spin next recurrence values",
            )?,
            spin_exponents_minus: allocate_exponents(recurrence_scratch_len)?,
        })
    }
}

impl ShtWorkspace {
    pub(crate) fn new(
        spectrum_count: usize,
        nphi: usize,
        ntheta: usize,
        recurrence_factor_count: usize,
        spin_recurrence_factor_count: usize,
        fft: &RealFftPlan,
        cc_resampling: Option<&CCResamplingPlan>,
    ) -> Result<Self, ShtError> {
        let prepared_ntheta = cc_resampling.map_or(0, CCResamplingPlan::prepared_ntheta);
        let latitude_count = ntheta.max(prepared_ntheta);
        let prepared_spectrum_count =
            prepared_ntheta
                .checked_mul(fft.spectrum_len())
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "prepared CC Fourier spectrum count",
                })?;
        let recurrence_scratch_len = latitude_count.min(LATITUDE_BLOCK_SIZE);
        let ring_block_count = ntheta.div_ceil(LATITUDE_BLOCK_SIZE);
        #[cfg(feature = "parallel")]
        let available_workers = rayon::current_num_threads();
        #[cfg(not(feature = "parallel"))]
        let available_workers = 1;
        let worker_limit = available_workers.min(ring_block_count).max(1);
        let blocks_per_group = ring_block_count.div_ceil(worker_limit);
        let ring_group_size =
            blocks_per_group
                .checked_mul(LATITUDE_BLOCK_SIZE)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "parallel latitude group size",
                })?;
        let worker_count = ring_block_count.div_ceil(blocks_per_group);
        let mut recurrence_workers = Vec::new();
        recurrence_workers
            .try_reserve_exact(worker_count)
            .map_err(|_| ShtError::AllocationFailed {
                array: "recurrence worker scratch",
            })?;
        for _ in 0..worker_count {
            recurrence_workers.push(RecurrenceWorkspace::new(
                recurrence_factor_count,
                spin_recurrence_factor_count,
                recurrence_scratch_len,
            )?);
        }

        Ok(Self {
            spectra: allocate_complex(spectrum_count, "Fourier spectrum")?,
            spin_spectra: allocate_complex(spectrum_count, "spin Fourier spectrum")?,
            prepared_spectra: allocate_complex(
                prepared_spectrum_count,
                "prepared CC Fourier spectrum",
            )?,
            prepared_spin_spectra: allocate_complex(
                prepared_spectrum_count,
                "prepared spin CC Fourier spectrum",
            )?,
            cc_profile: allocate_complex(
                cc_resampling.map_or(0, CCResamplingPlan::workspace_profile_len),
                "CC meridional profile",
            )?,
            cc_fft_scratch: allocate_complex(
                cc_resampling.map_or(0, CCResamplingPlan::workspace_fft_scratch_len),
                "CC meridional FFT scratch",
            )?,
            row: allocate_real(nphi, "longitude row")?,
            theta_sin: allocate_real(latitude_count, "latitude sine values")?,
            theta_cos: allocate_real(latitude_count, "latitude cosine values")?,
            ring_order_limits: allocate_usizes(latitude_count)?,
            recurrence_workers,
            ring_group_size,
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

fn allocate_usizes(len: usize) -> Result<Vec<usize>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed {
            array: "latitude order limits",
        })?;
    values.resize(len, 0);
    Ok(values)
}
