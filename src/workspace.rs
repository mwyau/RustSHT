// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex;
use rustfft::FftNum;
use rustfft::num_traits::Zero;

use crate::error::ShtError;
use crate::fft::{RealFftPlan, RealFftWorkspace};
use crate::legendre::{LATITUDE_BLOCK_SIZE, ShtScalar};
use crate::meridional::CCResamplingPlan;

/// Mutable scratch for one transform execution.
///
/// Create a workspace with [`crate::ShtPlan::workspace`]. Use a separate workspace
/// for each concurrent execution.
#[allow(private_bounds)]
pub struct ShtWorkspace<T: ShtScalar = f64> {
    pub(crate) spectra: Vec<Complex<T>>,
    pub(crate) spin_spectra: Vec<Complex<T>>,
    pub(crate) prepared_spectra: Vec<Complex<T>>,
    pub(crate) prepared_spin_spectra: Vec<Complex<T>>,
    pub(crate) cc_profile: Vec<Complex<T>>,
    pub(crate) cc_fft_scratch: Vec<Complex<T>>,
    pub(crate) row: Vec<T>,
    pub(crate) theta_sin: Vec<T>,
    pub(crate) theta_cos: Vec<T>,
    pub(crate) ring_order_limits: Vec<usize>,
    pub(crate) recurrence_workers: Vec<RecurrenceWorkspace<T>>,
    pub(crate) ring_group_size: usize,
    pub(crate) fft: RealFftWorkspace<T>,
}

pub(crate) struct RecurrenceWorkspace<T: ShtScalar> {
    pub(crate) epsilon: Vec<T>,
    pub(crate) previous: Vec<T>,
    pub(crate) current: Vec<T>,
    pub(crate) next: Vec<T>,
    pub(crate) exponents: Vec<i64>,
    pub(crate) spin_alpha: Vec<T>,
    pub(crate) spin_coefficient_a: Vec<T>,
    pub(crate) spin_coefficient_b: Vec<T>,
    pub(crate) spin_previous_plus: Vec<T>,
    pub(crate) spin_current_plus: Vec<T>,
    pub(crate) spin_next_plus: Vec<T>,
    pub(crate) spin_exponents_plus: Vec<i64>,
    pub(crate) spin_previous_minus: Vec<T>,
    pub(crate) spin_current_minus: Vec<T>,
    pub(crate) spin_next_minus: Vec<T>,
    pub(crate) spin_exponents_minus: Vec<i64>,
    pub(crate) weighted_spin_q_spectrum: Vec<Complex<T>>,
    pub(crate) weighted_spin_u_spectrum: Vec<Complex<T>>,
    pub(crate) analysis_same_real: Vec<T>,
    pub(crate) analysis_same_imag: Vec<T>,
    pub(crate) analysis_opposite_real: Vec<T>,
    pub(crate) analysis_opposite_imag: Vec<T>,
    pub(crate) fast_scalar_first: Vec<Complex<T>>,
    pub(crate) fast_scalar_second: Vec<Complex<T>>,
    pub(crate) fast_spin_first: Vec<Complex<T>>,
    pub(crate) fast_spin_second: Vec<Complex<T>>,
}

impl<T: ShtScalar> RecurrenceWorkspace<T> {
    fn new(
        recurrence_factor_count: usize,
        spin_recurrence_factor_count: usize,
        spin_fast_coefficient_count: usize,
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
            weighted_spin_q_spectrum: allocate_complex(
                recurrence_scratch_len,
                "weighted spin Q Fourier modes",
            )?,
            weighted_spin_u_spectrum: allocate_complex(
                recurrence_scratch_len,
                "weighted spin U Fourier modes",
            )?,
            analysis_same_real: allocate_real(
                recurrence_scratch_len,
                "same-parity analysis real spectrum",
            )?,
            analysis_same_imag: allocate_real(
                recurrence_scratch_len,
                "same-parity analysis imaginary spectrum",
            )?,
            analysis_opposite_real: allocate_real(
                recurrence_scratch_len,
                "opposite-parity analysis real spectrum",
            )?,
            analysis_opposite_imag: allocate_real(
                recurrence_scratch_len,
                "opposite-parity analysis imaginary spectrum",
            )?,
            fast_scalar_first: allocate_complex(
                spin_fast_coefficient_count,
                "spin fast scalar first coefficients",
            )?,
            fast_scalar_second: allocate_complex(
                spin_fast_coefficient_count,
                "spin fast scalar second coefficients",
            )?,
            fast_spin_first: allocate_complex(
                spin_fast_coefficient_count,
                "spin fast E coefficients",
            )?,
            fast_spin_second: allocate_complex(
                spin_fast_coefficient_count,
                "spin fast B coefficients",
            )?,
        })
    }
}

#[allow(private_bounds)]
impl<T: ShtScalar> ShtWorkspace<T> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        spectrum_count: usize,
        nphi: usize,
        ntheta: usize,
        analysis_order_count: usize,
        recurrence_factor_count: usize,
        spin_recurrence_factor_count: usize,
        spin_fast_coefficient_count: usize,
        fft: &RealFftPlan<T>,
        cc_resampling: Option<&CCResamplingPlan<T>>,
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
        let ring_worker_count = ring_block_count.div_ceil(blocks_per_group);
        let analysis_worker_count = available_workers.min(analysis_order_count).max(1);
        let worker_count = ring_worker_count.max(analysis_worker_count);
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
                spin_fast_coefficient_count,
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

fn allocate_complex<T: FftNum + Zero>(
    len: usize,
    array: &'static str,
) -> Result<Vec<Complex<T>>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed { array })?;
    values.resize(len, Complex::new(T::zero(), T::zero()));
    Ok(values)
}

fn allocate_real<T: FftNum + Zero>(len: usize, array: &'static str) -> Result<Vec<T>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed { array })?;
    values.resize(len, T::zero());
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
