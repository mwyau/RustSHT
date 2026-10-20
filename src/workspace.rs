// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex;
use rustfft::FftNum;
use rustfft::num_traits::Zero;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::error::ShtError;
use crate::fft::{RealFftPlan, RealFftWorkspace};
use crate::legendre::{
    LATITUDE_BLOCK_SIZE, SCALAR_ANALYSIS_PAIR_TILE_SIZE, ShtScalar, recurrence_scratch_len,
};
use crate::meridional::CCResamplingPlan;

const ANALYSIS_VECTOR_BUFFER_CAPACITY: usize = crate::legendre::SCALAR_ORDER_BATCH_SIZE;

#[cfg(all(test, feature = "parallel"))]
use std::collections::BTreeSet;
#[cfg(all(test, feature = "parallel"))]
use std::sync::Mutex;
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
    pub(crate) theta_cos_squared: Vec<T>,
    pub(crate) analysis_ring_scales: Vec<T>,
    pub(crate) ring_order_limits: Vec<usize>,
    pub(crate) recurrence_workers: Vec<RecurrenceWorkspace<T>>,
    // Order/tile pairs whose ordinary recurrence passed checked range validation.
    pub(crate) analysis_pair_range_safe: Vec<AtomicBool>,
    // Entries are cleared when plan identity, backend or geometry changes.
    pub(crate) analysis_pair_range_arch: Option<std::mem::Discriminant<pulp::Arch>>,
    pub(crate) analysis_pair_range_cache_id: Option<Arc<u8>>,
    // Analysis and adjoint-synthesis may use different latitude geometry and
    // insignificant-prefix rules on the same plan.
    pub(crate) analysis_pair_range_geometry: Option<(usize, bool)>,
    pub(crate) ring_group_size: usize,
    pub(crate) execution_capacity: usize,
    pub(crate) fft: RealFftWorkspace<T>,
    #[cfg(feature = "parallel")]
    pub(crate) fft_worker_workspaces: Vec<RealFftWorkspace<T>>,
    #[cfg(all(test, feature = "parallel"))]
    transform_phase_probe: Option<TransformPhaseProbe>,
}

#[allow(unexpected_cfgs)]
#[cfg_attr(rustsht_scalar_analysis_fixed_simd_arrays, repr(C))]
pub(crate) struct RecurrenceWorkspace<T: ShtScalar> {
    // Keep the scalar recurrence state and the four packed analysis spectra
    // together. The fixed-array experiment matches DUCC's per-order SIMD
    // storage while preserving slice-based Pulp dispatch.
    #[allow(unexpected_cfgs)]
    pub(crate) previous: AnalysisVectorBuffer<T>,
    #[allow(unexpected_cfgs)]
    pub(crate) current: AnalysisVectorBuffer<T>,
    #[allow(unexpected_cfgs)]
    pub(crate) next: AnalysisVectorBuffer<T>,
    #[allow(unexpected_cfgs)]
    pub(crate) analysis_same_real: AnalysisVectorBuffer<T>,
    #[allow(unexpected_cfgs)]
    pub(crate) analysis_same_imag: AnalysisVectorBuffer<T>,
    #[allow(unexpected_cfgs)]
    pub(crate) analysis_opposite_real: AnalysisVectorBuffer<T>,
    #[allow(unexpected_cfgs)]
    pub(crate) analysis_opposite_imag: AnalysisVectorBuffer<T>,
    pub(crate) epsilon: Vec<T>,
    pub(crate) exponents: Vec<i64>,
    pub(crate) analysis_scale_epochs: Vec<T>,
    pub(crate) analysis_scale_corrections: Vec<T>,
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
    pub(crate) scalar_order_correction: Vec<T>,
    pub(crate) compact_theta_sin: Vec<T>,
    pub(crate) compact_theta_cos: Vec<T>,
    pub(crate) compact_theta_cos_squared: Vec<T>,
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

#[allow(unexpected_cfgs)]
#[cfg(rustsht_scalar_analysis_fixed_simd_arrays)]
#[repr(align(64))]
pub(crate) struct FixedAnalysisVectorBuffer<T: ShtScalar>([T; ANALYSIS_VECTOR_BUFFER_CAPACITY]);

#[allow(unexpected_cfgs)]
#[cfg(rustsht_scalar_analysis_fixed_simd_arrays)]
impl<T: ShtScalar> FixedAnalysisVectorBuffer<T> {
    fn new(length: usize, name: &'static str) -> Result<Self, ShtError> {
        if length > ANALYSIS_VECTOR_BUFFER_CAPACITY {
            return Err(ShtError::BufferLength {
                buffer: name,
                expected: length,
                actual: ANALYSIS_VECTOR_BUFFER_CAPACITY,
            });
        }
        Ok(Self([T::zero(); ANALYSIS_VECTOR_BUFFER_CAPACITY]))
    }

    #[cfg(test)]
    pub(crate) fn capacity(&self) -> usize {
        ANALYSIS_VECTOR_BUFFER_CAPACITY
    }
}

#[allow(unexpected_cfgs)]
#[cfg(rustsht_scalar_analysis_fixed_simd_arrays)]
impl<T: ShtScalar> std::ops::Deref for FixedAnalysisVectorBuffer<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[allow(unexpected_cfgs)]
#[cfg(rustsht_scalar_analysis_fixed_simd_arrays)]
impl<T: ShtScalar> std::ops::DerefMut for FixedAnalysisVectorBuffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[allow(unexpected_cfgs)]
#[cfg(rustsht_scalar_analysis_fixed_simd_arrays)]
type AnalysisVectorBuffer<T> = FixedAnalysisVectorBuffer<T>;

#[allow(unexpected_cfgs)]
#[cfg(not(rustsht_scalar_analysis_fixed_simd_arrays))]
type AnalysisVectorBuffer<T> = Vec<T>;

#[allow(unexpected_cfgs)]
#[cfg(rustsht_scalar_analysis_fixed_simd_arrays)]
fn allocate_analysis_vector_buffer<T: ShtScalar>(
    length: usize,
    name: &'static str,
) -> Result<AnalysisVectorBuffer<T>, ShtError> {
    FixedAnalysisVectorBuffer::new(length, name)
}

#[allow(unexpected_cfgs)]
#[cfg(not(rustsht_scalar_analysis_fixed_simd_arrays))]
fn allocate_analysis_vector_buffer<T: ShtScalar>(
    length: usize,
    name: &'static str,
) -> Result<AnalysisVectorBuffer<T>, ShtError> {
    debug_assert!(length <= ANALYSIS_VECTOR_BUFFER_CAPACITY);
    allocate_real(length, name)
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
            previous: allocate_analysis_vector_buffer(
                recurrence_scratch_len,
                "previous recurrence values",
            )?,
            current: allocate_analysis_vector_buffer(
                recurrence_scratch_len,
                "current recurrence values",
            )?,
            next: allocate_analysis_vector_buffer(
                recurrence_scratch_len,
                "next recurrence values",
            )?,
            exponents: allocate_exponents(recurrence_scratch_len)?,
            analysis_scale_epochs: allocate_real(
                recurrence_scratch_len,
                "scalar-analysis scale epochs",
            )?,
            analysis_scale_corrections: allocate_real(
                recurrence_scratch_len,
                "scalar-analysis scale corrections",
            )?,
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
            analysis_same_real: allocate_analysis_vector_buffer(
                recurrence_scratch_len,
                "same-parity analysis real spectrum",
            )?,
            analysis_same_imag: allocate_analysis_vector_buffer(
                recurrence_scratch_len,
                "same-parity analysis imaginary spectrum",
            )?,
            analysis_opposite_real: allocate_analysis_vector_buffer(
                recurrence_scratch_len,
                "opposite-parity analysis real spectrum",
            )?,
            analysis_opposite_imag: allocate_analysis_vector_buffer(
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
            compact_theta_cos_squared: allocate_real(
                recurrence_scratch_len,
                "compacted latitude cosine-square values",
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
        let recurrence_scratch_len = recurrence_scratch_len(latitude_count);
        let analysis_pair_block_size = if T::supports_paired_scalar() {
            SCALAR_ANALYSIS_PAIR_TILE_SIZE
        } else {
            LATITUDE_BLOCK_SIZE
        };
        let analysis_pair_block_count = latitude_count
            .div_ceil(2)
            .div_ceil(analysis_pair_block_size);
        let analysis_pair_range_cache_len = analysis_order_count
            .checked_mul(analysis_pair_block_count)
            .ok_or(ShtError::IntegerOverflow {
                quantity: "scalar-analysis range cache count",
            })?;
        let ring_group_size = Self::ring_group_size_for_capacity(ntheta, execution_capacity)?;
        let ring_worker_count = ntheta.div_ceil(ring_group_size);
        let analysis_worker_count = execution_capacity.min(analysis_order_count).max(1);
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
            theta_cos_squared: allocate_real(latitude_count, "latitude cosine-square values")?,
            analysis_ring_scales: allocate_real(latitude_count, "analysis ring scales")?,
            ring_order_limits: allocate_usizes(latitude_count, "latitude order limits")?,
            recurrence_workers,
            analysis_pair_range_safe: allocate_atomic_bools(analysis_pair_range_cache_len)?,
            analysis_pair_range_arch: None,
            analysis_pair_range_cache_id: None,
            analysis_pair_range_geometry: None,
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

fn allocate_atomic_bools(len: usize) -> Result<Vec<AtomicBool>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed {
            array: "scalar-analysis range cache",
        })?;
    values.extend((0..len).map(|_| AtomicBool::new(false)));
    Ok(values)
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

#[allow(unexpected_cfgs)]
#[cfg(all(test, rustsht_scalar_analysis_fixed_simd_arrays))]
mod fixed_analysis_vector_tests {
    use super::{ANALYSIS_VECTOR_BUFFER_CAPACITY, FixedAnalysisVectorBuffer, RecurrenceWorkspace};
    use pulp::{Arch, Simd, WithSimd};

    struct VectorSpan<'a>(&'a [f64]);

    impl WithSimd for VectorSpan<'_> {
        type Output = (usize, usize, usize);

        fn with_simd<S: Simd>(self, _simd: S) -> Self::Output {
            let (vectors, tail) = S::as_simd_f64s(self.0);
            (S::F64_LANES, vectors.len(), tail.len())
        }
    }

    #[test]
    fn analysis_arrays_are_contiguous_aligned_and_vector_complete() {
        let workspace =
            RecurrenceWorkspace::<f64>::new(4, 4, 4, 4, ANALYSIS_VECTOR_BUFFER_CAPACITY)
                .expect("fixed-array recurrence workspace");
        let bases = [
            workspace.previous.as_ptr() as usize,
            workspace.current.as_ptr() as usize,
            workspace.next.as_ptr() as usize,
            workspace.analysis_same_real.as_ptr() as usize,
            workspace.analysis_same_imag.as_ptr() as usize,
            workspace.analysis_opposite_real.as_ptr() as usize,
            workspace.analysis_opposite_imag.as_ptr() as usize,
        ];
        let stride = std::mem::size_of::<FixedAnalysisVectorBuffer<f64>>();
        assert_eq!(
            stride,
            ANALYSIS_VECTOR_BUFFER_CAPACITY * std::mem::size_of::<f64>()
        );
        for (index, base) in bases.iter().enumerate() {
            assert_eq!(base % 64, 0, "buffer {index} alignment");
            if index > 0 {
                assert_eq!(*base - bases[index - 1], stride, "buffer {index} stride");
            }
        }

        let (width, vector_count, tail_count) =
            Arch::new().dispatch(VectorSpan(&workspace.analysis_same_real));
        assert_eq!(
            vector_count * width + tail_count,
            ANALYSIS_VECTOR_BUFFER_CAPACITY
        );
        assert_eq!(tail_count, 0);
    }
}
