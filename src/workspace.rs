// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex;
use rustfft::FftNum;
use rustfft::num_traits::Zero;

use crate::error::ShtError;
use crate::fft::{RealFftPlan, RealFftWorkspace};
use crate::legendre::{LATITUDE_BLOCK_SIZE, SCALAR_ORDER_BATCH_SIZE, ShtScalar};
use crate::meridional::CCResamplingPlan;

#[cfg(all(test, feature = "parallel"))]
use std::collections::BTreeSet;
#[cfg(all(test, feature = "parallel"))]
use std::sync::{Arc, Mutex};
#[cfg(all(test, feature = "parallel"))]
use std::time::Instant;

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
    #[cfg(feature = "parallel")]
    pub(crate) spin1_scalar_coefficients: Vec<Complex<T>>,
    pub(crate) cc_profile: Vec<Complex<T>>,
    pub(crate) cc_fft_scratch: Vec<Complex<T>>,
    pub(crate) row: Vec<T>,
    pub(crate) theta_sin: Vec<T>,
    pub(crate) theta_cos: Vec<T>,
    pub(crate) ring_order_limits: Vec<usize>,
    #[cfg(feature = "parallel")]
    pub(crate) order_work_prefix: Vec<u128>,
    pub(crate) recurrence_workers: Vec<RecurrenceWorkspace<T>>,
    pub(crate) ring_group_size: usize,
    pub(crate) execution_capacity: usize,
    pub(crate) fft: RealFftWorkspace<T>,
    #[cfg(feature = "parallel")]
    pub(crate) fft_worker_workspaces: Vec<RealFftWorkspace<T>>,
    #[cfg(all(test, feature = "parallel"))]
    transform_phase_probe: Option<TransformPhaseProbe>,
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
    pub(crate) scalar_order_correction: Vec<T>,
    pub(crate) compact_theta_sin: Vec<T>,
    pub(crate) compact_theta_cos: Vec<T>,
    pub(crate) compact_ring_indices: Vec<usize>,
    pub(crate) fast_scalar_first: Vec<Complex<T>>,
    pub(crate) fast_scalar_second: Vec<Complex<T>>,
    pub(crate) fast_spin_first: Vec<Complex<T>>,
    pub(crate) fast_spin_second: Vec<Complex<T>>,
    pub(crate) fast_spin_pair_first: Vec<Complex<T>>,
    pub(crate) fast_spin_pair_second: Vec<Complex<T>>,
    #[cfg(all(test, feature = "parallel"))]
    worker_probe: Option<Arc<Mutex<BTreeSet<usize>>>>,
    #[cfg(all(test, feature = "parallel"))]
    analysis_leaf_probe: Option<Arc<Mutex<Vec<AnalysisLeafObservation>>>>,
    #[cfg(all(test, feature = "parallel"))]
    scalar_order_leaf_probe: Option<Arc<Mutex<Vec<ScalarOrderLeafObservation>>>>,
    #[cfg(all(test, feature = "parallel"))]
    profile_normal_blocks: usize,
    #[cfg(all(test, feature = "parallel"))]
    profile_scaled_blocks: usize,
    #[cfg(all(test, feature = "parallel"))]
    profile_mixed_blocks: usize,
}

#[cfg(all(test, feature = "parallel"))]
#[derive(Clone, Debug)]
pub(crate) struct AnalysisLeafObservation {
    pub(crate) spin: bool,
    pub(crate) m_start: usize,
    pub(crate) m_end: usize,
    pub(crate) active_ring_order_pairs: u128,
    pub(crate) estimated_work: u128,
    pub(crate) elapsed_ns: u128,
    pub(crate) started: Instant,
    pub(crate) finished: Instant,
    pub(crate) worker: Option<usize>,
    pub(crate) normal_blocks: usize,
    pub(crate) scaled_blocks: usize,
    pub(crate) mixed_blocks: usize,
}

#[cfg(all(test, feature = "parallel"))]
#[derive(Clone, Debug)]
pub(crate) struct ScalarOrderLeafObservation {
    pub(crate) m_start: usize,
    pub(crate) m_end: usize,
    pub(crate) active_ring_order_pairs: u128,
    pub(crate) estimated_work: u128,
    pub(crate) elapsed_ns: u128,
    pub(crate) worker: Option<usize>,
}

#[cfg(all(test, feature = "parallel"))]
#[derive(Clone, Debug)]
pub(crate) struct TransformPhaseObservation {
    pub(crate) phase: &'static str,
    pub(crate) elapsed_ns: u128,
}

#[cfg(all(test, feature = "parallel"))]
pub(crate) type TransformPhaseProbe = Arc<Mutex<Vec<TransformPhaseObservation>>>;

#[cfg(all(test, feature = "parallel"))]
pub(crate) fn record_transform_phase(
    probe: &Option<TransformPhaseProbe>,
    phase: &'static str,
    started: Instant,
) {
    let Some(probe) = probe else {
        return;
    };
    let observation = TransformPhaseObservation {
        phase,
        elapsed_ns: started.elapsed().as_nanos(),
    };
    probe
        .lock()
        .expect("transform phase profile is not poisoned")
        .push(observation);
}

impl<T: ShtScalar> RecurrenceWorkspace<T> {
    fn new(
        recurrence_factor_count: usize,
        spin_recurrence_factor_count: usize,
        spin_fast_coefficient_count: usize,
        spin_pair_fast_coefficient_count: usize,
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
            scalar_order_correction: allocate_real(
                recurrence_scratch_len,
                "ordered scalar scale correction",
            )?,
            compact_theta_sin: allocate_real(
                recurrence_scratch_len,
                "compacted latitude sine values",
            )?,
            compact_theta_cos: allocate_real(
                recurrence_scratch_len,
                "compacted latitude cosine values",
            )?,
            compact_ring_indices: allocate_usizes(
                recurrence_scratch_len,
                "compacted latitude ring indices",
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
            fast_spin_pair_first: allocate_complex(
                spin_pair_fast_coefficient_count,
                "spin paired fast first channel coefficients",
            )?,
            fast_spin_pair_second: allocate_complex(
                spin_pair_fast_coefficient_count,
                "spin paired fast second channel coefficients",
            )?,
            #[cfg(all(test, feature = "parallel"))]
            worker_probe: None,
            #[cfg(all(test, feature = "parallel"))]
            analysis_leaf_probe: None,
            #[cfg(all(test, feature = "parallel"))]
            scalar_order_leaf_probe: None,
            #[cfg(all(test, feature = "parallel"))]
            profile_normal_blocks: 0,
            #[cfg(all(test, feature = "parallel"))]
            profile_scaled_blocks: 0,
            #[cfg(all(test, feature = "parallel"))]
            profile_mixed_blocks: 0,
        })
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn record_active_worker(&self) {
        let Some(probe) = &self.worker_probe else {
            return;
        };
        if let Some(index) = rayon::current_thread_index() {
            probe
                .lock()
                .expect("worker probe is not poisoned")
                .insert(index);
        }
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn analysis_leaf_profile_enabled(&self) -> bool {
        self.analysis_leaf_probe.is_some()
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn scalar_order_leaf_started(&self) -> Option<Instant> {
        self.scalar_order_leaf_probe
            .as_ref()
            .map(|_| Instant::now())
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn record_scalar_order_leaf(
        &self,
        lmax: usize,
        m_start: usize,
        m_end: usize,
        ring_order_limits: &[usize],
        started: Option<Instant>,
    ) {
        let (Some(probe), Some(started)) = (&self.scalar_order_leaf_probe, started) else {
            return;
        };
        let finished = Instant::now();
        let mut active_ring_order_pairs = 0_u128;
        let mut estimated_work = 0_u128;
        for m in m_start..m_end {
            let active_rings = ring_order_limits
                .iter()
                .filter(|&&limit| m <= limit)
                .count() as u128;
            active_ring_order_pairs += active_rings;
            estimated_work += active_rings * (lmax - m + 1) as u128;
        }
        probe
            .lock()
            .expect("scalar-order leaf profile is not poisoned")
            .push(ScalarOrderLeafObservation {
                m_start,
                m_end,
                active_ring_order_pairs,
                estimated_work,
                elapsed_ns: finished.duration_since(started).as_nanos(),
                worker: rayon::current_thread_index(),
            });
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn reset_analysis_leaf_block_counts(&mut self) {
        self.profile_normal_blocks = 0;
        self.profile_scaled_blocks = 0;
        self.profile_mixed_blocks = 0;
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn record_analysis_leaf_block(&mut self, normal: bool, scaled: bool) {
        if self.analysis_leaf_probe.is_none() {
            return;
        }
        match (normal, scaled) {
            (true, false) => self.profile_normal_blocks += 1,
            (false, true) => self.profile_scaled_blocks += 1,
            (true, true) => self.profile_mixed_blocks += 1,
            (false, false) => {}
        }
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn record_analysis_leaf(
        &mut self,
        spin: bool,
        lmax: usize,
        m_start: usize,
        m_end: usize,
        ring_order_limits: &[usize],
        started: Instant,
    ) {
        let Some(probe) = &self.analysis_leaf_probe else {
            return;
        };
        let finished = Instant::now();
        let mut active_ring_order_pairs = 0_u128;
        let mut estimated_work = 0_u128;
        for m in m_start..m_end {
            let active_rings = ring_order_limits
                .iter()
                .filter(|&&limit| m <= limit)
                .count() as u128;
            active_ring_order_pairs += active_rings;
            estimated_work += active_rings * (lmax - m + 1) as u128;
        }
        let observation = AnalysisLeafObservation {
            spin,
            m_start,
            m_end,
            active_ring_order_pairs,
            estimated_work,
            elapsed_ns: finished.duration_since(started).as_nanos(),
            started,
            finished,
            worker: rayon::current_thread_index(),
            normal_blocks: self.profile_normal_blocks,
            scaled_blocks: self.profile_scaled_blocks,
            mixed_blocks: self.profile_mixed_blocks,
        };
        probe
            .lock()
            .expect("analysis leaf profile is not poisoned")
            .push(observation);
        self.reset_analysis_leaf_block_counts();
    }
}

#[allow(private_bounds)]
impl<T: ShtScalar> ShtWorkspace<T> {
    /// Maximum execution capacity accepted by this build.
    ///
    /// Builds with the `parallel` feature accept capacities through 64. Builds
    /// without it accept only 1.
    pub const MAX_EXECUTION_CAPACITY: usize = if cfg!(feature = "parallel") { 64 } else { 1 };

    /// Returns the requested maximum number of Rayon workers for transforms
    /// that use this workspace.
    ///
    /// The executing pool may have fewer threads. Increasing this value
    /// requires [`crate::ShtPlan::prepare_workspace`]; changing Rayon pools
    /// does not change it.
    #[must_use]
    pub const fn execution_capacity(&self) -> usize {
        self.execution_capacity
    }

    /// Returns the number of recurrence scratch slots currently allocated.
    ///
    /// The slot count can exceed the execution capacity because synthesis
    /// groups may be overpartitioned for load balancing. Reducing execution
    /// capacity retains allocated scratch.
    #[must_use]
    pub fn allocated_worker_count(&self) -> usize {
        self.recurrence_workers.len()
    }

    pub(crate) fn ring_group_size_for_capacity(
        ntheta: usize,
        execution_capacity: usize,
    ) -> Result<usize, ShtError> {
        let ring_block_count = ntheta.div_ceil(LATITUDE_BLOCK_SIZE);
        let worker_limit = execution_capacity.min(ring_block_count).max(1);
        let blocks_per_group = ring_block_count.div_ceil(worker_limit);
        let mut ring_group_size =
            blocks_per_group
                .checked_mul(LATITUDE_BLOCK_SIZE)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "parallel latitude group size",
                })?;
        let existing_ring_worker_count = ntheta.div_ceil(ring_group_size);
        // Increase synthesis granularity when block grouping leaves workers
        // idle. Keep multi-block groups aligned to recurrence block boundaries.
        if execution_capacity > 1
            && ntheta >= 64
            && (ring_block_count <= 9 || existing_ring_worker_count < worker_limit)
        {
            let candidate_blocks_per_group = blocks_per_group.div_ceil(2);
            ring_group_size = if blocks_per_group == 1 {
                if ntheta < 256 {
                    LATITUDE_BLOCK_SIZE / 4
                } else {
                    LATITUDE_BLOCK_SIZE / 2
                }
            } else {
                candidate_blocks_per_group
                    .checked_mul(LATITUDE_BLOCK_SIZE)
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "parallel latitude group size",
                    })?
            };
        }
        Ok(ring_group_size)
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn observe_active_workers(&mut self) -> Arc<Mutex<BTreeSet<usize>>> {
        let probe = Arc::new(Mutex::new(BTreeSet::new()));
        for recurrence in &mut self.recurrence_workers {
            recurrence.worker_probe = Some(Arc::clone(&probe));
        }
        probe
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn observe_analysis_leaves(&mut self) -> Arc<Mutex<Vec<AnalysisLeafObservation>>> {
        let probe = Arc::new(Mutex::new(Vec::new()));
        for recurrence in &mut self.recurrence_workers {
            recurrence.analysis_leaf_probe = Some(Arc::clone(&probe));
        }
        probe
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn observe_scalar_order_leaves(
        &mut self,
    ) -> Arc<Mutex<Vec<ScalarOrderLeafObservation>>> {
        let probe = Arc::new(Mutex::new(Vec::new()));
        for recurrence in &mut self.recurrence_workers {
            recurrence.scalar_order_leaf_probe = Some(Arc::clone(&probe));
        }
        probe
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn observe_transform_phases(
        &mut self,
    ) -> Arc<Mutex<Vec<TransformPhaseObservation>>> {
        let probe = Arc::new(Mutex::new(Vec::new()));
        self.transform_phase_probe = Some(Arc::clone(&probe));
        probe
    }

    #[cfg(all(test, feature = "parallel"))]
    pub(crate) fn transform_phase_probe(&self) -> Option<TransformPhaseProbe> {
        self.transform_phase_probe.clone()
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn ensure_recurrence_worker_count(
        &mut self,
        worker_count: usize,
        recurrence_factor_count: usize,
        spin_recurrence_factor_count: usize,
        spin_fast_coefficient_count: usize,
        spin_pair_fast_coefficient_count: usize,
        recurrence_scratch_len: usize,
    ) -> Result<(), ShtError> {
        if worker_count <= self.recurrence_workers.len() {
            return Ok(());
        }
        self.recurrence_workers
            .try_reserve_exact(worker_count - self.recurrence_workers.len())
            .map_err(|_| ShtError::AllocationFailed {
                array: "recurrence worker scratch",
            })?;
        while self.recurrence_workers.len() < worker_count {
            self.recurrence_workers.push(RecurrenceWorkspace::new(
                recurrence_factor_count,
                spin_recurrence_factor_count,
                spin_fast_coefficient_count,
                spin_pair_fast_coefficient_count,
                recurrence_scratch_len,
            )?);
        }
        Ok(())
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn ensure_fft_worker_count(
        &mut self,
        worker_count: usize,
        fft: &RealFftPlan<T>,
    ) -> Result<(), ShtError> {
        if worker_count <= self.fft_worker_workspaces.len() {
            return Ok(());
        }
        self.fft_worker_workspaces
            .try_reserve_exact(worker_count - self.fft_worker_workspaces.len())
            .map_err(|_| ShtError::AllocationFailed {
                array: "FFT worker scratch slots",
            })?;
        while self.fft_worker_workspaces.len() < worker_count {
            self.fft_worker_workspaces.push(fft.workspace()?);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        spectrum_count: usize,
        nphi: usize,
        ntheta: usize,
        analysis_order_count: usize,
        recurrence_factor_count: usize,
        spin_recurrence_factor_count: usize,
        spin_fast_coefficient_count: usize,
        spin_pair_fast_coefficient_count: usize,
        execution_capacity: usize,
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
        let recurrence_scratch_len = latitude_count
            .min(LATITUDE_BLOCK_SIZE)
            .max(SCALAR_ORDER_BATCH_SIZE);
        let ring_group_size = Self::ring_group_size_for_capacity(ntheta, execution_capacity)?;
        let ring_worker_count = ntheta.div_ceil(ring_group_size);
        let analysis_worker_count = execution_capacity.min(analysis_order_count).max(1);
        #[cfg(feature = "parallel")]
        let order_work_prefix_len =
            analysis_order_count
                .checked_add(1)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "analysis order work prefix length",
                })?;
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
                spin_pair_fast_coefficient_count,
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
            #[cfg(feature = "parallel")]
            spin1_scalar_coefficients: Vec::new(),
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
            ring_order_limits: allocate_usizes(latitude_count, "latitude order limits")?,
            #[cfg(feature = "parallel")]
            order_work_prefix: allocate_u128(order_work_prefix_len, "analysis order work prefix")?,
            recurrence_workers,
            ring_group_size,
            execution_capacity,
            fft: fft.workspace()?,
            #[cfg(feature = "parallel")]
            fft_worker_workspaces: Vec::new(),
            #[cfg(all(test, feature = "parallel"))]
            transform_phase_probe: None,
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

fn allocate_usizes(len: usize, array: &'static str) -> Result<Vec<usize>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed { array })?;
    values.resize(len, 0);
    Ok(values)
}

#[cfg(feature = "parallel")]
fn allocate_u128(len: usize, array: &'static str) -> Result<Vec<u128>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed { array })?;
    values.resize(len, 0);
    Ok(values)
}
