// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
#[cfg(feature = "parallel")]
use std::collections::VecDeque;
#[cfg(feature = "parallel")]
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{ShtError, check_len};
use crate::fft::RealFftWorkspace;
use crate::grid::GridKind;
use crate::legendre::{
    LATITUDE_BLOCK_SIZE, LegendreOperation, LegendreStrategy, SCALAR_ANALYSIS_PAIR_TILE_SIZE,
    ShtScalar, longitude_phase, recurrence_scratch_len, sin_cos_from_f64,
};
use crate::plan::ShtPlan;
use crate::workspace::{RecurrenceWorkspace, ShtWorkspace};

const SCALAR_TRANSITION_MIN_LMAX: usize = 1023;
const FUSED_SCALAR_ANALYSIS_MIN_LMAX: usize = 255;

#[cfg(feature = "parallel")]
#[derive(Clone, Copy)]
struct ScalarSynthesisCoefficientCacheLayout {
    coefficient_count: usize,
    row_stride: usize,
    half_stride: usize,
}

#[derive(Clone, Copy)]
struct ScalarSynthesisCoefficientCache<'a, T> {
    values: &'a [Complex<T>],
    row_stride: usize,
    half_stride: usize,
}

fn scalar_ordered_scratch<T: ShtScalar>(
    recurrence: &mut RecurrenceWorkspace<T>,
) -> crate::legendre::scalar_ordered::ScalarOrderedScratch<'_, T> {
    crate::legendre::scalar_ordered::ScalarOrderedScratch {
        epsilon: &mut recurrence.epsilon,
        alpha: &mut recurrence.spin_alpha,
        coefficient_a: &mut recurrence.spin_coefficient_a,
        coefficient_b: &mut recurrence.spin_coefficient_b,
        coefficient_even: &mut recurrence.fast_spin_first,
        coefficient_odd: &mut recurrence.fast_spin_second,
        packed_sin: &mut recurrence.compact_theta_sin,
        packed_cos: &mut recurrence.compact_theta_cos,
        packed_csq: &mut recurrence.spin_current_plus,
        packed_north: &mut recurrence.compact_ring_indices,
        previous: &mut recurrence.previous,
        current: &mut recurrence.current,
        scale: &mut recurrence.exponents,
        corfac: &mut recurrence.scalar_order_correction,
        sum_even_re: &mut recurrence.analysis_same_real,
        sum_even_im: &mut recurrence.analysis_same_imag,
        sum_odd_re: &mut recurrence.analysis_opposite_real,
        sum_odd_im: &mut recurrence.analysis_opposite_imag,
    }
}

#[inline]
fn finalize_paired_scalar_analysis_coefficients<T: ShtScalar>(
    coefficients: &mut [Complex<T>],
    m: usize,
    lmax: usize,
    epsilon: &[T],
    alpha: &[T],
) {
    debug_assert_eq!(coefficients.len(), lmax - m + 1);
    let mut previous_even = Complex::new(T::zero(), T::zero());
    let mut previous_alpha = T::zero();
    for (pair_index, l) in (m..=lmax).step_by(2).enumerate() {
        let offset = l - m;
        let even = coefficients[offset];
        let odd = if l < lmax {
            coefficients[offset + 1]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        let previous = previous_even * (previous_alpha * epsilon[l]);
        let pair_alpha = alpha[pair_index];
        coefficients[offset] = even * (pair_alpha * epsilon[l + 1]) + previous;
        if l < lmax {
            coefficients[offset + 1] = odd * pair_alpha;
        }
        previous_even = even;
        previous_alpha = pair_alpha;
    }
}

struct ScalarAnalysisOrderPreparation<'a, T> {
    analysis_spectra: &'a [Complex<T>],
    spectrum_len: usize,
    phi0: f64,
    ring_order_limits: &'a [usize],
    scalar_table_selected: bool,
    ring_weights: Option<&'a [f64]>,
    longitude_count: T,
    scalar_table_input_limit: Option<T>,
}

impl<T: ShtScalar> ScalarAnalysisOrderPreparation<'_, T> {
    #[inline]
    fn prepare_order(&self, order: usize, order_spectra: &mut [Complex<T>]) -> bool {
        debug_assert_eq!(order_spectra.len(), self.ring_order_limits.len());
        if let Some(ring_weights) = self.ring_weights {
            debug_assert_eq!(ring_weights.len(), order_spectra.len());
        }

        let phase = if order == 0 {
            Complex::new(T::one(), T::zero())
        } else {
            longitude_phase::<T>(order, self.phi0, true)
        };
        let mut inputs_safe = true;
        for (ring, output) in order_spectra.iter_mut().enumerate() {
            let mut weighted = if self.scalar_table_selected && order > self.ring_order_limits[ring]
            {
                Complex::new(T::zero(), T::zero())
            } else {
                self.analysis_spectra[ring * self.spectrum_len + order] * phase
            };
            if let Some(ring_weights) = self.ring_weights {
                let weight = T::from_f64(ring_weights[ring]).expect("ring weight is representable");
                weighted *= weight / self.longitude_count;
            }
            if let Some(input_limit) = self.scalar_table_input_limit {
                inputs_safe &= crate::legendre::ScalarLegendreTable::<T>::input_value_is_safe(
                    weighted.re,
                    input_limit,
                ) && crate::legendre::ScalarLegendreTable::<T>::input_value_is_safe(
                    weighted.im,
                    input_limit,
                );
            }
            *output = weighted;
        }
        inputs_safe
    }

    fn prepare_serial(&self, order_spectra: &mut [Complex<T>]) -> bool {
        let latitude_count = self.ring_order_limits.len();
        debug_assert!(latitude_count > 0);
        debug_assert_eq!(order_spectra.len() % latitude_count, 0);
        let mut inputs_safe = true;
        for (order, order_row) in order_spectra.chunks_mut(latitude_count).enumerate() {
            let order_safe = self.prepare_order(order, order_row);
            inputs_safe &= order_safe;
        }
        inputs_safe
    }
}

#[cfg(feature = "parallel")]
pub(crate) fn weighted_order_midpoint(lmax: usize, m_start: usize, m_end: usize) -> usize {
    debug_assert!(m_start < m_end);
    debug_assert!(m_end - 1 <= lmax);
    debug_assert!(m_end - m_start > 1);

    let total_work = (m_start..m_end)
        .map(|m| (lmax - m + 1) as u128)
        .sum::<u128>();
    let mut left_work = 0_u128;
    let mut best_mid = m_start + 1;
    let mut best_difference = u128::MAX;
    for m in m_start..(m_end - 1) {
        left_work += (lmax - m + 1) as u128;
        let right_work = total_work - left_work;
        let difference = left_work.abs_diff(right_work);
        if difference < best_difference {
            best_difference = difference;
            best_mid = m + 1;
        }
    }
    best_mid
}

fn accumulate_precomputed_synthesis_order<T: ShtScalar>(
    plan: &ShtPlan<T>,
    table: &crate::legendre::ScalarLegendreTable<T>,
    alm: &[Complex<T>],
    ring_order_limits: &[usize],
    m: usize,
    order_row: &mut [Complex<T>],
) -> bool {
    let ntheta = order_row.len();
    let nphi = plan.grid.nphi();
    let lmax = plan.harmonics.lmax();
    for degree_offset in 0..=(lmax - m) {
        let l = m + degree_offset;
        let coefficient = alm[plan
            .harmonics
            .index(l, m)
            .expect("precomputed scalar order is within the harmonic layout")];
        if !table.input_is_safe(coefficient.re) || !table.input_is_safe(coefficient.im) {
            return false;
        }
        let parity = if (l + m).is_multiple_of(2) {
            T::one()
        } else {
            -T::one()
        };
        for (north_ring, &basis) in table.degree_lanes(m, degree_offset).iter().enumerate() {
            let south_ring = ntheta - 1 - north_ring;
            if m <= ring_order_limits[north_ring] {
                order_row[north_ring] += coefficient * basis;
            }
            if south_ring != north_ring && m <= ring_order_limits[south_ring] {
                order_row[south_ring] += coefficient * (parity * basis);
            }
        }
    }

    let phase = if m == 0 {
        Complex::new(T::one(), T::zero())
    } else {
        longitude_phase::<T>(m, plan.grid.phi0(), false)
    };
    let scale = phase * T::from_usize(nphi).expect("longitude count converts");
    for value in order_row {
        *value *= scale;
    }
    true
}

#[inline]
#[expect(
    clippy::too_many_arguments,
    reason = "The block preparation maps mirrored inputs into existing worker scratch."
)]
fn prepare_symmetric_analysis_pair_block<T: ShtScalar>(
    m: usize,
    latitude_count: usize,
    block_start: usize,
    block_len: usize,
    compact: bool,
    order_spectra: &[Complex<T>],
    ring_order_limits: &[usize],
    theta_sin: &[T],
    theta_cos: &[T],
    theta_cos_squared: &[T],
    same_real: &mut [T],
    same_imag: &mut [T],
    opposite_real: &mut [T],
    opposite_imag: &mut [T],
    packed_north_ring: &mut [usize],
    compact_theta_sin: &mut [T],
    compact_theta_cos: &mut [T],
    compact_theta_cos_squared: &mut [T],
) -> usize {
    debug_assert_eq!(latitude_count, theta_cos_squared.len());
    let mut output_lane = 0;
    for lane in 0..block_len {
        let north_ring = block_start + lane;
        let south_ring = latitude_count - 1 - north_ring;
        let north_active = m <= ring_order_limits[north_ring];
        let south_active = south_ring != north_ring && m <= ring_order_limits[south_ring];
        if compact && !north_active && !south_active {
            continue;
        }
        let north = if north_active {
            order_spectra[north_ring]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        // The equatorial singleton has no reflected sample and contributes once to both parities.
        let south = if south_active {
            order_spectra[south_ring]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        same_real[output_lane] = north.re + south.re;
        same_imag[output_lane] = north.im + south.im;
        opposite_real[output_lane] = north.re - south.re;
        opposite_imag[output_lane] = north.im - south.im;
        if compact {
            packed_north_ring[output_lane] = north_ring;
            compact_theta_sin[output_lane] = theta_sin[north_ring];
            compact_theta_cos[output_lane] = theta_cos[north_ring];
            compact_theta_cos_squared[output_lane] = theta_cos_squared[north_ring];
        }
        output_lane += 1;
    }

    if compact { output_lane } else { block_len }
}

#[inline]
fn restore_symmetric_analysis_pair_opposite<T: ShtScalar>(
    m: usize,
    latitude_count: usize,
    order_spectra: &[Complex<T>],
    ring_order_limits: &[usize],
    packed_north_ring: &[usize],
    opposite_real: &mut [T],
    opposite_imag: &mut [T],
) {
    debug_assert_eq!(order_spectra.len(), latitude_count);
    debug_assert_eq!(ring_order_limits.len(), latitude_count);
    debug_assert_eq!(packed_north_ring.len(), opposite_real.len());
    debug_assert_eq!(opposite_real.len(), opposite_imag.len());
    for (lane, &north_ring) in packed_north_ring.iter().enumerate() {
        let south_ring = latitude_count - 1 - north_ring;
        let north_active = m <= ring_order_limits[north_ring];
        let south_active = south_ring != north_ring && m <= ring_order_limits[south_ring];
        let north = if north_active {
            order_spectra[north_ring]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        let south = if south_active {
            order_spectra[south_ring]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        opposite_real[lane] = north.re - south.re;
        opposite_imag[lane] = north.im - south.im;
    }
}

#[inline]
fn stable_analysis_cosine_squared<T: ShtScalar>(sine: T, cosine: T) -> T {
    if cosine.abs() > T::from_f64(0.99).expect("0.99 is representable") {
        (T::one() - sine) * (T::one() + sine)
    } else {
        cosine * cosine
    }
}

fn first_active_symmetric_pair(
    m: usize,
    latitude_count: usize,
    ring_order_limits: &[usize],
) -> usize {
    // On supported symmetric grids, the scalar cutoff rises toward the equator;
    // the maximum limit of each reflected pair is therefore monotone as well.
    let pair_lane_count = latitude_count.div_ceil(2);
    let pair_limit =
        |pair: usize| ring_order_limits[pair].max(ring_order_limits[latitude_count - 1 - pair]);
    let mut lower = 0;
    let mut upper = pair_lane_count;
    while lower < upper {
        let midpoint = lower + (upper - lower) / 2;
        if pair_limit(midpoint) < m {
            lower = midpoint + 1;
        } else {
            upper = midpoint;
        }
    }
    lower
}

#[allow(private_bounds)]
impl<T: ShtScalar> ShtPlan<T> {
    pub(crate) fn forward_analysis_rows(
        &self,
        input: &[T],
        output: &mut [Complex<T>],
        serial_workspace: &mut RealFftWorkspace<T>,
        #[cfg(feature = "parallel")] worker_workspaces: &mut [RealFftWorkspace<T>],
        #[cfg(feature = "parallel")] execution_capacity: usize,
    ) -> Result<(), ShtError> {
        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();

        #[cfg(feature = "parallel")]
        {
            let worker_count = execution_capacity
                .min(rayon::current_num_threads())
                .min(worker_workspaces.len())
                .min(ntheta);
            if worker_count > 1 {
                let rings_per_group = ntheta.div_ceil(worker_count);
                let output_chunk_len =
                    rings_per_group
                        .checked_mul(spectrum_len)
                        .ok_or(ShtError::IntegerOverflow {
                            quantity: "analysis FFT output chunk length",
                        })?;
                output
                    .par_chunks_mut(output_chunk_len)
                    .zip(worker_workspaces[..worker_count].par_iter_mut())
                    .enumerate()
                    .try_for_each(|(group, (spectra, fft_workspace))| {
                        let first_ring = group * rings_per_group;
                        for (offset, spectrum) in spectra.chunks_mut(spectrum_len).enumerate() {
                            let ring = first_ring + offset;
                            let input_start = ring * nphi;
                            self.fft.forward(
                                &input[input_start..input_start + nphi],
                                spectrum,
                                fft_workspace,
                            )?;
                        }
                        Ok(())
                    })?;
                return Ok(());
            }
        }

        for ring in 0..ntheta {
            let input_start = ring * nphi;
            let spectrum_start = ring * spectrum_len;
            self.fft.forward(
                &input[input_start..input_start + nphi],
                &mut output[spectrum_start..spectrum_start + spectrum_len],
                serial_workspace,
            )?;
        }
        Ok(())
    }

    fn inverse_synthesis_rows(
        &self,
        spectra: &[Complex<T>],
        output: &mut [T],
        serial_workspace: &mut RealFftWorkspace<T>,
        #[cfg(feature = "parallel")] worker_workspaces: &mut [RealFftWorkspace<T>],
        #[cfg(feature = "parallel")] execution_capacity: usize,
    ) -> Result<(), ShtError> {
        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        debug_assert_eq!(spectra.len(), ntheta * spectrum_len);
        debug_assert_eq!(output.len(), ntheta * nphi);

        #[cfg(feature = "parallel")]
        {
            let worker_count = execution_capacity
                .min(rayon::current_num_threads())
                .min(worker_workspaces.len())
                .min(ntheta);
            if self.row_fft_parallelism_supported() && worker_count > 1 {
                let rings_per_group = ntheta.div_ceil(worker_count);
                let output_chunk_len =
                    rings_per_group
                        .checked_mul(nphi)
                        .ok_or(ShtError::IntegerOverflow {
                            quantity: "synthesis FFT output chunk length",
                        })?;
                output
                    .par_chunks_mut(output_chunk_len)
                    .zip(worker_workspaces[..worker_count].par_iter_mut())
                    .enumerate()
                    .try_for_each(|(group, (output_rows, fft_workspace))| {
                        let first_ring = group * rings_per_group;
                        for (offset, output_row) in output_rows.chunks_mut(nphi).enumerate() {
                            let ring = first_ring + offset;
                            let spectrum_start = ring * spectrum_len;
                            self.fft.inverse_series(
                                &spectra[spectrum_start..spectrum_start + spectrum_len],
                                output_row,
                                fft_workspace,
                            )?;
                        }
                        Ok(())
                    })?;
                return Ok(());
            }
        }

        for ring in 0..ntheta {
            let spectrum_start = ring * spectrum_len;
            let output_start = ring * nphi;
            self.fft.inverse_series(
                &spectra[spectrum_start..spectrum_start + spectrum_len],
                &mut output[output_start..output_start + nphi],
                serial_workspace,
            )?;
        }
        Ok(())
    }

    #[cfg(feature = "parallel")]
    fn synthesize_scalar_ordered_parallel(
        &self,
        alm: &[Complex<T>],
        arch: pulp::Arch,
        workspace: &mut ShtWorkspace<T>,
    ) -> bool {
        let lmax = self.harmonics.lmax();
        let mmax = self.harmonics.mmax();
        let Some(order_count) = mmax.checked_add(1) else {
            return false;
        };
        let ntheta = self.grid.ntheta();
        let spectrum_len = self.fft.spectrum_len();
        let Some(order_spectrum_len) = ntheta.checked_mul(order_count) else {
            return false;
        };
        if mmax >= spectrum_len || order_spectrum_len > workspace.spin_spectra.len() {
            return false;
        }
        let worker_count = rayon::current_num_threads()
            .min(workspace.execution_capacity)
            .min(workspace.recurrence_workers.len())
            .min(order_count);
        if worker_count <= 1 {
            return false;
        }

        #[cfg(all(test, feature = "parallel"))]
        let phase_probe = workspace.transform_phase_probe();
        let order_spectra = &mut workspace.spin_spectra[..order_spectrum_len];
        order_spectra.fill(Complex::new(T::zero(), T::zero()));
        // Match DUCC's one-order dynamic schedule. The queue owns disjoint
        // output rows, while each Rayon task reuses one recurrence workspace.
        let mut order_jobs = VecDeque::new();
        if order_jobs.try_reserve_exact(order_count).is_err() {
            return false;
        }
        for (order, spectra) in order_spectra.chunks_mut(ntheta).enumerate() {
            order_jobs.push_back((order, spectra));
        }
        if order_jobs.len() != order_count {
            return false;
        }
        let order_jobs = Mutex::new(order_jobs);
        let failed = AtomicBool::new(false);
        #[cfg(all(test, feature = "parallel"))]
        let order_started = std::time::Instant::now();
        workspace.recurrence_workers[..worker_count]
            .par_iter_mut()
            .for_each(|recurrence| {
                #[cfg(all(test, feature = "parallel"))]
                recurrence.record_active_worker();
                loop {
                    if failed.load(Ordering::Relaxed) {
                        break;
                    }
                    let job = order_jobs
                        .lock()
                        .expect("scalar synthesis order queue is not poisoned")
                        .pop_front();
                    let Some((order, spectra)) = job else {
                        break;
                    };
                    #[cfg(all(test, feature = "parallel"))]
                    let leaf_started = recurrence.scalar_order_leaf_started();
                    let succeeded = {
                        let mut scratch = scalar_ordered_scratch(recurrence);
                        T::synthesize_scalar_ordered_range_dispatched(
                            arch,
                            alm,
                            lmax,
                            mmax,
                            order,
                            order + 1,
                            self.grid.nphi(),
                            spectrum_len,
                            self.grid.phi0(),
                            &self.sectoral_norms,
                            &workspace.theta_sin[..ntheta],
                            &workspace.theta_cos[..ntheta],
                            &workspace.ring_order_limits[..ntheta],
                            spectra,
                            &mut scratch,
                        )
                    };
                    #[cfg(all(test, feature = "parallel"))]
                    recurrence.record_scalar_order_leaf(
                        lmax,
                        order,
                        order + 1,
                        &workspace.ring_order_limits[..ntheta],
                        leaf_started,
                    );
                    if !succeeded {
                        failed.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            });
        drop(order_jobs);
        let succeeded = !failed.load(Ordering::Relaxed);
        #[cfg(all(test, feature = "parallel"))]
        crate::workspace::record_transform_phase(
            &phase_probe,
            "ordered scalar recurrence",
            order_started,
        );
        if !succeeded {
            return false;
        }

        // Keep row-major spectra uncommitted until every dynamic order succeeds.
        const TRANSPOSE_TILE: usize = 32;
        let Some(scatter_chunk_len) = spectrum_len.checked_mul(TRANSPOSE_TILE) else {
            return false;
        };
        let Some(output_len) = ntheta.checked_mul(spectrum_len) else {
            return false;
        };
        if output_len > workspace.spectra.len() {
            return false;
        }
        #[cfg(all(test, feature = "parallel"))]
        let scatter_started = std::time::Instant::now();
        let order_spectra: &[Complex<T>] = order_spectra;
        workspace.spectra[..output_len]
            .par_chunks_mut(scatter_chunk_len)
            .enumerate()
            .for_each(|(ring_tile, rows)| {
                let ring_start = ring_tile * TRANSPOSE_TILE;
                let ring_count = rows.len() / spectrum_len;
                for order_start in (0..order_count).step_by(TRANSPOSE_TILE) {
                    let order_end = (order_start + TRANSPOSE_TILE).min(order_count);
                    for local_ring in 0..ring_count {
                        let ring = ring_start + local_ring;
                        let row_start = local_ring * spectrum_len;
                        let output_row = &mut rows[row_start..row_start + spectrum_len];
                        for order in order_start..order_end {
                            output_row[order] = order_spectra[order * ntheta + ring];
                        }
                    }
                }
            });
        #[cfg(all(test, feature = "parallel"))]
        crate::workspace::record_transform_phase(
            &phase_probe,
            "ordered spectrum scatter",
            scatter_started,
        );
        true
    }

    /// Synthesizes one real scalar field into north-to-south row-major samples.
    ///
    /// For even longitude counts, the Nyquist order is allowed and its
    /// positive and negative modes are combined into the real Nyquist bin.
    ///
    /// # Errors
    ///
    /// Returns an error for non-scalar spin, invalid coefficients, insufficient
    /// longitude sampling, a buffer length mismatch, or an incompatible workspace.
    pub fn synthesis_into(
        &self,
        alm: &[Complex<T>],
        map: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.check_scalar_spin()?;
        self.harmonics.validate_coefficients(alm)?;
        self.check_synthesis_order()?;
        check_len("map output", self.sample_count, map.len())?;
        self.check_workspace(workspace)?;

        #[cfg(all(test, feature = "parallel"))]
        let phase_probe = workspace.transform_phase_probe();
        #[cfg(all(test, feature = "parallel"))]
        let mut phase_started = std::time::Instant::now();

        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        let lmax = self.harmonics.lmax();
        let phi0 = self.grid.phi0();
        for (ring, &theta) in self.grid.theta().iter().enumerate() {
            (workspace.theta_sin[ring], workspace.theta_cos[ring]) = sin_cos_from_f64(theta);
            workspace.ring_order_limits[ring] = T::polar_order_limit(
                lmax,
                0,
                workspace.theta_sin[ring],
                workspace.theta_cos[ring],
            );
        }
        workspace.spectra.fill(Complex::new(T::zero(), T::zero()));
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "latitude setup and spectrum clear",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }
        let arch = pulp::Arch::new();
        if self.scalar_legendre_strategy_with_capacity(
            LegendreOperation::Synthesis,
            workspace.execution_capacity,
        ) == LegendreStrategy::CompactTable
            && let Some(table) = self.get_safe_scalar_legendre_table_for_synthesis(alm)
            && self.synthesize_scalar_table(alm, map, workspace, table)?
        {
            return Ok(());
        }
        let ordered_scalar_used =
            if T::supports_scalar_ordered() && self.grid.kind() == GridKind::GaussLegendre {
                if workspace.execution_capacity == 1 {
                    let recurrence = &mut workspace.recurrence_workers[0];
                    #[cfg(all(test, feature = "parallel"))]
                    recurrence.record_active_worker();
                    let mut scratch = scalar_ordered_scratch(recurrence);
                    T::synthesize_scalar_ordered_dispatched(
                        arch,
                        alm,
                        lmax,
                        self.harmonics.mmax(),
                        nphi,
                        spectrum_len,
                        phi0,
                        &self.sectoral_norms,
                        &workspace.theta_sin[..ntheta],
                        &workspace.theta_cos[..ntheta],
                        &workspace.ring_order_limits[..ntheta],
                        &mut workspace.spectra,
                        &mut scratch,
                    )
                } else {
                    #[cfg(feature = "parallel")]
                    {
                        self.synthesize_scalar_ordered_parallel(alm, arch, workspace)
                    }
                    #[cfg(not(feature = "parallel"))]
                    {
                        false
                    }
                }
            } else {
                false
            };
        if !ordered_scalar_used {
            #[cfg(feature = "parallel")]
            {
                let execution_threads =
                    rayon::current_num_threads().min(workspace.execution_capacity);
                let group_ring_count = self
                    .fine_synthesis_ring_group_size(execution_threads)
                    .unwrap_or(workspace.ring_group_size)
                    .min(ntheta);
                let parallel_groups = ntheta.div_ceil(group_ring_count);
                let use_parallel = execution_threads > 1 && parallel_groups > 1;
                if use_parallel {
                    let cached_scalar_coefficients = self
                        .prepare_scalar_synthesis_coefficient_cache(
                            alm,
                            &mut workspace.spin_spectra,
                            &mut workspace.recurrence_workers[..parallel_groups],
                            execution_threads,
                            parallel_groups,
                        )
                        .map(|layout| ScalarSynthesisCoefficientCache {
                            values: &workspace.spin_spectra[..layout.coefficient_count],
                            row_stride: layout.row_stride,
                            half_stride: layout.half_stride,
                        });
                    let spectrum_group_len = group_ring_count.checked_mul(spectrum_len).ok_or(
                        ShtError::IntegerOverflow {
                            quantity: "parallel Fourier spectrum group size",
                        },
                    )?;
                    debug_assert!(parallel_groups <= workspace.recurrence_workers.len());
                    workspace
                        .spectra
                        .par_chunks_mut(spectrum_group_len)
                        .zip(workspace.recurrence_workers[..parallel_groups].par_iter_mut())
                        .enumerate()
                        .with_min_len(parallel_groups.div_ceil(execution_threads))
                        .map(|(group_index, (group_spectra, recurrence))| {
                            let ring_start = group_index * group_ring_count;
                            let ring_end =
                                (ring_start + group_spectra.len() / spectrum_len).min(ntheta);
                            self.synthesize_ring_group_with_scalar_coefficients(
                                alm,
                                nphi,
                                spectrum_len,
                                phi0,
                                ring_start,
                                &mut group_spectra[..(ring_end - ring_start) * spectrum_len],
                                &workspace.theta_sin[ring_start..ring_end],
                                &workspace.theta_cos[ring_start..ring_end],
                                &workspace.ring_order_limits,
                                arch,
                                recurrence,
                                cached_scalar_coefficients,
                            )
                        })
                        .try_for_each(|result| result)?;
                } else {
                    self.synthesize_ring_group(
                        alm,
                        nphi,
                        spectrum_len,
                        phi0,
                        0,
                        &mut workspace.spectra,
                        &workspace.theta_sin[..ntheta],
                        &workspace.theta_cos[..ntheta],
                        &workspace.ring_order_limits[..ntheta],
                        arch,
                        &mut workspace.recurrence_workers[0],
                    )?;
                }
            }
            #[cfg(not(feature = "parallel"))]
            {
                self.synthesize_ring_group(
                    alm,
                    nphi,
                    spectrum_len,
                    phi0,
                    0,
                    &mut workspace.spectra,
                    &workspace.theta_sin[..ntheta],
                    &workspace.theta_cos[..ntheta],
                    &workspace.ring_order_limits[..ntheta],
                    arch,
                    &mut workspace.recurrence_workers[0],
                )?;
            }
        }

        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "Legendre recurrence and spectral accumulation",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }

        self.inverse_synthesis_rows(
            &workspace.spectra,
            map,
            &mut workspace.fft,
            #[cfg(feature = "parallel")]
            &mut workspace.fft_worker_workspaces,
            #[cfg(feature = "parallel")]
            workspace.execution_capacity,
        )?;
        #[cfg(all(test, feature = "parallel"))]
        crate::workspace::record_transform_phase(
            &phase_probe,
            "inverse longitude FFT and map writeback",
            phase_started,
        );
        Ok(())
    }

    #[cfg(feature = "parallel")]
    fn prepare_scalar_synthesis_coefficient_cache(
        &self,
        alm: &[Complex<T>],
        cache: &mut [Complex<T>],
        recurrence_workers: &mut [RecurrenceWorkspace<T>],
        execution_threads: usize,
        parallel_groups: usize,
    ) -> Option<ScalarSynthesisCoefficientCacheLayout> {
        let lmax = self.harmonics.lmax();
        let order_count = self.harmonics.mmax().checked_add(1)?;
        // Keep the cache on the paired-transition path and avoid preparing it
        // when the ring-group schedule cannot keep each worker occupied.
        if self.harmonics.spin() != 0
            || self.grid.kind() != GridKind::GaussLegendre
            || lmax < SCALAR_TRANSITION_MIN_LMAX
            || parallel_groups < execution_threads
            || !T::supports_paired_scalar()
        {
            return None;
        }

        let half_stride = lmax.checked_add(2)?.checked_div(2)?;
        let row_stride = half_stride.checked_mul(2)?;
        let coefficient_count = row_stride.checked_mul(order_count)?;
        if coefficient_count > cache.len() || recurrence_workers.len() < parallel_groups {
            return None;
        }

        let worker_count = execution_threads
            .min(order_count)
            .min(recurrence_workers.len())
            .max(1);
        let orders_per_chunk = order_count.div_ceil(worker_count);
        let chunk_len = orders_per_chunk.checked_mul(row_stride)?;
        let chunk_count = order_count.div_ceil(orders_per_chunk);
        let prepared = cache[..coefficient_count]
            .par_chunks_mut(chunk_len)
            .zip(recurrence_workers[..chunk_count].par_iter_mut())
            .enumerate()
            .all(|(chunk_index, (coefficient_chunk, recurrence))| {
                for (local_order, order_coefficients) in
                    coefficient_chunk.chunks_mut(row_stride).enumerate()
                {
                    let m = chunk_index * orders_per_chunk + local_order;
                    if m >= order_count {
                        break;
                    }
                    let pair_count = (lmax - m + 2) / 2;
                    T::fill_epsilon(m, lmax, &mut recurrence.epsilon);
                    if !T::fill_paired_scalar_factors(
                        m,
                        lmax,
                        &recurrence.epsilon,
                        &mut recurrence.spin_alpha,
                        &mut recurrence.spin_coefficient_a,
                        &mut recurrence.spin_coefficient_b,
                    ) {
                        return false;
                    }
                    let coefficient_start = self
                        .harmonics
                        .index(m, m)
                        .expect("every packed order starts at its diagonal degree");
                    let coefficient_len = lmax - m + 1;
                    let (coefficient_even, coefficient_odd) =
                        order_coefficients.split_at_mut(half_stride);
                    if !T::prepare_paired_scalar_synthesis_coefficients(
                        m,
                        lmax,
                        &recurrence.epsilon,
                        &recurrence.spin_alpha,
                        &alm[coefficient_start..coefficient_start + coefficient_len],
                        &mut coefficient_even[..pair_count],
                        &mut coefficient_odd[..pair_count],
                    ) {
                        return false;
                    }
                }
                true
            });
        prepared.then_some(ScalarSynthesisCoefficientCacheLayout {
            coefficient_count,
            row_stride,
            half_stride,
        })
    }

    fn synthesize_scalar_table(
        &self,
        alm: &[Complex<T>],
        map: &mut [T],
        workspace: &mut ShtWorkspace<T>,
        table: &crate::legendre::ScalarLegendreTable<T>,
    ) -> Result<bool, ShtError> {
        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        let mmax = self.harmonics.mmax();
        let order_count = mmax + 1;
        let order_spectrum_count =
            order_count
                .checked_mul(ntheta)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "precomputed scalar synthesis spectrum count",
                })?;
        let order_spectra = &mut workspace.spin_spectra[..order_spectrum_count];
        order_spectra.fill(Complex::new(T::zero(), T::zero()));

        #[cfg(feature = "parallel")]
        let execution_threads = rayon::current_num_threads().min(workspace.execution_capacity);
        #[cfg(feature = "parallel")]
        let all_inputs_safe = if execution_threads > 1 && order_count > 1 {
            order_spectra
                .par_chunks_mut(ntheta)
                .enumerate()
                .with_min_len(order_count.div_ceil(execution_threads))
                .all(|(m, order_row)| {
                    accumulate_precomputed_synthesis_order(
                        self,
                        table,
                        alm,
                        &workspace.ring_order_limits[..ntheta],
                        m,
                        order_row,
                    )
                })
        } else {
            order_spectra
                .chunks_mut(ntheta)
                .enumerate()
                .all(|(m, order_row)| {
                    accumulate_precomputed_synthesis_order(
                        self,
                        table,
                        alm,
                        &workspace.ring_order_limits[..ntheta],
                        m,
                        order_row,
                    )
                })
        };
        #[cfg(not(feature = "parallel"))]
        let all_inputs_safe = order_spectra
            .chunks_mut(ntheta)
            .enumerate()
            .all(|(m, order_row)| {
                accumulate_precomputed_synthesis_order(
                    self,
                    table,
                    alm,
                    &workspace.ring_order_limits[..ntheta],
                    m,
                    order_row,
                )
            });

        if !all_inputs_safe {
            return Ok(false);
        }

        workspace.spectra.fill(Complex::new(T::zero(), T::zero()));
        for m in 0..order_count {
            for ring in 0..ntheta {
                workspace.spectra[ring * spectrum_len + m] = order_spectra[m * ntheta + ring];
            }
        }
        for ring in 0..ntheta {
            let spectrum_start = ring * spectrum_len;
            self.fft.inverse_bounded(
                &workspace.spectra[spectrum_start..spectrum_start + spectrum_len],
                &mut workspace.row,
                &mut workspace.fft,
            )?;
            let map_start = ring * nphi;
            map[map_start..map_start + nphi].copy_from_slice(&workspace.row);
        }
        Ok(true)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Each synthesis group uses plan geometry and its caller-owned worker scratch."
    )]
    fn synthesize_ring_group(
        &self,
        alm: &[Complex<T>],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        group_ring_start: usize,
        spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.synthesize_ring_group_impl::<false>(
            alm,
            nphi,
            spectrum_len,
            phi0,
            group_ring_start,
            spectra,
            theta_sin,
            theta_cos,
            ring_order_limits,
            arch,
            recurrence,
            None,
        )
    }

    #[cfg(feature = "parallel")]
    #[expect(
        clippy::too_many_arguments,
        reason = "The cached synthesis group receives plan geometry, optional shared coefficients, and worker scratch."
    )]
    fn synthesize_ring_group_with_scalar_coefficients(
        &self,
        alm: &[Complex<T>],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        group_ring_start: usize,
        spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
        scalar_coefficients: Option<ScalarSynthesisCoefficientCache<'_, T>>,
    ) -> Result<(), ShtError> {
        if let Some(scalar_coefficients) = scalar_coefficients {
            self.synthesize_ring_group_impl::<true>(
                alm,
                nphi,
                spectrum_len,
                phi0,
                group_ring_start,
                spectra,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                recurrence,
                Some(scalar_coefficients),
            )
        } else {
            self.synthesize_ring_group(
                alm,
                nphi,
                spectrum_len,
                phi0,
                group_ring_start,
                spectra,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                recurrence,
            )
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The synthesis group receives plan geometry, optional shared coefficients, and worker scratch."
    )]
    fn synthesize_ring_group_impl<const USE_SCALAR_COEFFICIENT_CACHE: bool>(
        &self,
        alm: &[Complex<T>],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        group_ring_start: usize,
        spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
        scalar_coefficients: Option<ScalarSynthesisCoefficientCache<'_, T>>,
    ) -> Result<(), ShtError> {
        #[cfg(all(test, feature = "parallel"))]
        recurrence.record_active_worker();
        let lmax = self.harmonics.lmax();
        let mmax = self.harmonics.mmax();
        debug_assert_eq!(spectra.len(), theta_sin.len() * spectrum_len);
        debug_assert_eq!(theta_sin.len(), theta_cos.len());

        let nyquist_scale = T::from_f64(2.0).expect("two is representable");
        for m in 0..=mmax {
            T::fill_epsilon(m, lmax, &mut recurrence.epsilon);
            let coefficient_start = self
                .harmonics
                .index(m, m)
                .expect("every packed order starts at its diagonal degree");
            let transition_enabled =
                lmax >= SCALAR_TRANSITION_MIN_LMAX && T::supports_paired_scalar();
            let mut transition_coefficients_ready = false;
            let paired_factors_ready = T::fill_paired_scalar_factors(
                m,
                lmax,
                &recurrence.epsilon,
                &mut recurrence.spin_alpha,
                &mut recurrence.spin_coefficient_a,
                &mut recurrence.spin_coefficient_b,
            );
            // Extreme finite coefficients can overflow DUCC's paired-degree
            // coefficient transform even when the final scaled contribution
            // is representable. Keep those orders on the unpaired scaled path.
            let coefficient_pair_count = (lmax - m + 2) / 2;
            let cached_pair = if USE_SCALAR_COEFFICIENT_CACHE {
                scalar_coefficients.map(|cache| {
                    let row_start = m * cache.row_stride;
                    let row = &cache.values[row_start..row_start + cache.row_stride];
                    (
                        &row[..coefficient_pair_count],
                        &row[cache.half_stride..cache.half_stride + coefficient_pair_count],
                    )
                })
            } else {
                None
            };
            let coefficients_prepared = paired_factors_ready
                && if cached_pair.is_some() {
                    true
                } else {
                    recurrence.fast_spin_first.len() >= coefficient_pair_count
                        && recurrence.fast_spin_second.len() >= coefficient_pair_count
                        && T::prepare_paired_scalar_synthesis_coefficients(
                            m,
                            lmax,
                            &recurrence.epsilon,
                            &recurrence.spin_alpha,
                            &alm[coefficient_start..],
                            &mut recurrence.fast_spin_first[..coefficient_pair_count],
                            &mut recurrence.fast_spin_second[..coefficient_pair_count],
                        )
                };
            let (fast_first, fast_second) = match cached_pair {
                Some((first, second)) => (first, second),
                None => (
                    recurrence
                        .fast_spin_first
                        .get(..coefficient_pair_count)
                        .unwrap_or(&[]),
                    recurrence
                        .fast_spin_second
                        .get(..coefficient_pair_count)
                        .unwrap_or(&[]),
                ),
            };
            let phase = if m == 0 {
                Complex::new(T::one(), T::zero())
            } else {
                longitude_phase::<T>(m, phi0, false)
            };
            let paired_used = if paired_factors_ready && coefficients_prepared {
                for block_start in (0..theta_sin.len()).step_by(LATITUDE_BLOCK_SIZE) {
                    let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(theta_sin.len());
                    let block_len = block_end - block_start;
                    let ring_limits = &ring_order_limits
                        [group_ring_start + block_start..group_ring_start + block_end];
                    if ring_limits.iter().all(|&limit| m > limit) {
                        continue;
                    }
                    let spectrum_start = block_start * spectrum_len;
                    let spectrum_end = block_end * spectrum_len;
                    let fused = T::synthesize_paired_scalar_block_partitioned_dispatched(
                        arch,
                        m,
                        lmax,
                        &theta_sin[block_start..block_end],
                        &theta_cos[block_start..block_end],
                        self.sectoral_norms[m],
                        &recurrence.spin_coefficient_a,
                        &recurrence.spin_coefficient_b,
                        fast_first,
                        fast_second,
                        ring_limits,
                        spectrum_len,
                        &mut spectra[spectrum_start..spectrum_end],
                        &mut recurrence.previous[..block_len],
                        &mut recurrence.current[..block_len],
                        &mut recurrence.next[..block_len],
                        &mut recurrence.exponents[..block_len],
                        &mut recurrence.compact_theta_sin[..block_len],
                        &mut recurrence.compact_theta_cos[..block_len],
                        &mut recurrence.compact_ring_indices[..block_len],
                    )?;
                    if !fused {
                        let transition =
                            if transition_enabled && ring_limits.iter().all(|&limit| m <= limit) {
                                let coefficient_count = lmax - m + 1;
                                if !transition_coefficients_ready {
                                    recurrence.fast_scalar_second[..coefficient_count]
                                        .fill(Complex::new(T::zero(), T::zero()));
                                    transition_coefficients_ready = true;
                                }
                                T::try_synthesize_paired_scalar_transition_dispatched(
                                    arch,
                                    m,
                                    lmax,
                                    &theta_sin[block_start..block_end],
                                    &theta_cos[block_start..block_end],
                                    self.sectoral_norms[m],
                                    &recurrence.epsilon,
                                    &recurrence.spin_alpha,
                                    &recurrence.spin_coefficient_a,
                                    &recurrence.spin_coefficient_b,
                                    &alm[coefficient_start..coefficient_start + coefficient_count],
                                    &recurrence.fast_scalar_second[..coefficient_count],
                                    &mut recurrence.previous[..block_len],
                                    &mut recurrence.current[..block_len],
                                    &mut recurrence.next[..block_len],
                                    &mut recurrence.exponents[..block_len],
                                    &mut recurrence.analysis_same_real[..block_len],
                                    &mut recurrence.analysis_same_imag[..block_len],
                                    &mut recurrence.analysis_opposite_real[..block_len],
                                    &mut recurrence.analysis_opposite_imag[..block_len],
                                )?
                            } else {
                                false
                            };
                        if transition {
                            for lane in 0..block_len {
                                spectra[spectrum_start + lane * spectrum_len + m] += Complex::new(
                                    recurrence.analysis_same_real[lane],
                                    recurrence.analysis_same_imag[lane],
                                );
                            }
                        } else {
                            let compact_mixed_block =
                                lmax >= 255 && ring_limits.iter().any(|&limit| m > limit);
                            if compact_mixed_block {
                                let mut compact_len = 0;
                                for (ring, &limit) in ring_limits.iter().enumerate() {
                                    if m <= limit {
                                        recurrence.compact_theta_sin[compact_len] =
                                            theta_sin[block_start + ring];
                                        recurrence.compact_theta_cos[compact_len] =
                                            theta_cos[block_start + ring];
                                        recurrence.compact_ring_indices[compact_len] = ring;
                                        compact_len += 1;
                                    }
                                }
                                debug_assert!(compact_len > 0);
                                T::for_each_y_lm_paired_dispatched(
                                    arch,
                                    m,
                                    lmax,
                                    &recurrence.compact_theta_sin[..compact_len],
                                    &recurrence.compact_theta_cos[..compact_len],
                                    self.sectoral_norms[m],
                                    &recurrence.spin_coefficient_a,
                                    &recurrence.spin_coefficient_b,
                                    &mut recurrence.previous[..compact_len],
                                    &mut recurrence.current[..compact_len],
                                    &mut recurrence.next[..compact_len],
                                    &mut recurrence.exponents[..compact_len],
                                    |l, ring_start, current, exponents| {
                                        let pair_index = (l - m) / 2;
                                        let coefficient_offset = l - m;
                                        let next_even_coefficient = if l + 2 <= lmax {
                                            alm[coefficient_start + coefficient_offset + 2]
                                        } else {
                                            Complex::new(T::zero(), T::zero())
                                        };
                                        let coefficient_even = (alm
                                            [coefficient_start + coefficient_offset]
                                            * recurrence.epsilon[l + 1]
                                            + next_even_coefficient * recurrence.epsilon[l + 2])
                                            * recurrence.spin_alpha[pair_index];
                                        let coefficient_odd = if l < lmax {
                                            alm[coefficient_start + coefficient_offset + 1]
                                                * recurrence.spin_alpha[pair_index]
                                        } else {
                                            Complex::new(T::zero(), T::zero())
                                        };
                                        for lane in 0..current.len() {
                                            let fallback_ring = ring_start + lane;
                                            let local_ring =
                                                recurrence.compact_ring_indices[fallback_ring];
                                            let group_ring = block_start + local_ring;
                                            let contribution = T::multiply_complex_scaled(
                                                coefficient_even,
                                                current[lane],
                                                exponents[lane],
                                            )
                                                + T::multiply_complex_scaled_product(
                                                    coefficient_odd,
                                                    current[lane],
                                                    recurrence.compact_theta_cos[fallback_ring],
                                                    exponents[lane],
                                                );
                                            let spectrum_index = group_ring * spectrum_len + m;
                                            spectra[spectrum_index] += contribution;
                                        }
                                        Ok(())
                                    },
                                )?;
                            } else {
                                T::for_each_y_lm_paired_dispatched(
                                    arch,
                                    m,
                                    lmax,
                                    &theta_sin[block_start..block_end],
                                    &theta_cos[block_start..block_end],
                                    self.sectoral_norms[m],
                                    &recurrence.spin_coefficient_a,
                                    &recurrence.spin_coefficient_b,
                                    &mut recurrence.previous[..block_len],
                                    &mut recurrence.current[..block_len],
                                    &mut recurrence.next[..block_len],
                                    &mut recurrence.exponents[..block_len],
                                    |l, ring_start, current, exponents| {
                                        let pair_index = (l - m) / 2;
                                        let coefficient_offset = l - m;
                                        let next_even_coefficient = if l + 2 <= lmax {
                                            alm[coefficient_start + coefficient_offset + 2]
                                        } else {
                                            Complex::new(T::zero(), T::zero())
                                        };
                                        let coefficient_even = (alm
                                            [coefficient_start + coefficient_offset]
                                            * recurrence.epsilon[l + 1]
                                            + next_even_coefficient * recurrence.epsilon[l + 2])
                                            * recurrence.spin_alpha[pair_index];
                                        let coefficient_odd = if l < lmax {
                                            alm[coefficient_start + coefficient_offset + 1]
                                                * recurrence.spin_alpha[pair_index]
                                        } else {
                                            Complex::new(T::zero(), T::zero())
                                        };
                                        for lane in 0..current.len() {
                                            let group_ring = block_start + ring_start + lane;
                                            let global_ring = group_ring_start + group_ring;
                                            if m > ring_order_limits[global_ring] {
                                                continue;
                                            }
                                            let contribution = T::multiply_complex_scaled(
                                                coefficient_even,
                                                current[lane],
                                                exponents[lane],
                                            )
                                                + T::multiply_complex_scaled_product(
                                                    coefficient_odd,
                                                    current[lane],
                                                    theta_cos[group_ring],
                                                    exponents[lane],
                                                );
                                            let spectrum_index = group_ring * spectrum_len + m;
                                            spectra[spectrum_index] += contribution;
                                        }
                                        Ok(())
                                    },
                                )?;
                            }
                        }
                    }
                }
                true
            } else {
                false
            };
            if !paired_used {
                T::for_each_y_lm_dispatched(
                    arch,
                    m,
                    lmax,
                    theta_sin,
                    theta_cos,
                    self.sectoral_norms[m],
                    &recurrence.epsilon,
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                    |l, ring_start, current, exponents| {
                        let coefficient = alm[coefficient_start + (l - m)];
                        for lane in 0..current.len() {
                            let group_ring = ring_start + lane;
                            let global_ring = group_ring_start + group_ring;
                            if m > ring_order_limits[global_ring] {
                                continue;
                            }
                            let spectrum_index = group_ring * spectrum_len + m;
                            spectra[spectrum_index] += T::multiply_complex_scaled(
                                coefficient,
                                current[lane],
                                exponents[lane],
                            );
                        }
                        Ok(())
                    },
                )?;
            }
            for group_ring in 0..theta_sin.len() {
                let global_ring = group_ring_start + group_ring;
                if m > ring_order_limits[global_ring] {
                    continue;
                }
                let spectrum_index = group_ring * spectrum_len + m;
                let phased = spectra[spectrum_index] * phase;
                if nphi.is_multiple_of(2) && m == nphi / 2 {
                    spectra[spectrum_index] = Complex::new(nyquist_scale * phased.re, T::zero());
                } else {
                    spectra[spectrum_index] = phased;
                }
            }
        }
        Ok(())
    }

    /// Analyzes samples into real-field scalar harmonic coefficients.
    ///
    /// Exact GL and F1 analysis require `ntheta >= lmax + 1`; exact CC analysis
    /// requires `ntheta >= lmax + 2`. All require
    /// `mmax <= (nphi - 1) / 2`.
    ///
    /// # Errors
    ///
    /// Returns an error for non-scalar spin, insufficient exact-analysis
    /// sampling, a buffer length mismatch, or an incompatible workspace.
    pub fn analysis_into(
        &self,
        map: &[T],
        alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.check_scalar_spin()?;
        self.check_analysis_sampling()?;
        check_len("map input", self.sample_count, map.len())?;
        check_len("coefficient output", self.harmonics.nalm(), alm.len())?;
        self.check_workspace(workspace)?;

        self.analyze_map_to_coefficients(map, alm, workspace, Some(self.grid.ring_weights()))
    }

    /// Applies the adjoint of scalar synthesis under the Euclidean map dot
    /// product and the real harmonic inner product.
    ///
    /// Unlike exact analysis, this operation has no latitude sampling floor.
    /// It accepts the real Nyquist order for even longitude counts.
    ///
    /// # Errors
    ///
    /// Returns an error for non-scalar spin, insufficient longitude sampling,
    /// a buffer length mismatch, or an incompatible workspace.
    pub fn adjoint_synthesis_into(
        &self,
        map: &[T],
        alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.check_scalar_spin()?;
        self.check_synthesis_order()?;
        check_len("map input", self.sample_count, map.len())?;
        check_len("coefficient output", self.harmonics.nalm(), alm.len())?;
        self.check_workspace(workspace)?;

        self.analyze_map_to_coefficients(map, alm, workspace, None)
    }

    /// Applies the adjoint of exact scalar analysis.
    ///
    /// Exact GL and F1 analysis require `ntheta >= lmax + 1`; exact CC analysis
    /// requires `ntheta >= lmax + 2`. All require
    /// `mmax <= (nphi - 1) / 2`. GL output is STANDARD synthesis scaled on
    /// each ring by `ring_weight / nphi`; CC output uses the adjoint of the
    /// prepared-grid resampling operator.
    ///
    /// # Errors
    ///
    /// Returns an error for non-scalar spin, insufficient exact-analysis
    /// sampling, invalid coefficients, a buffer length mismatch, or an
    /// incompatible workspace.
    pub fn adjoint_analysis_into(
        &self,
        alm: &[Complex<T>],
        map: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.check_scalar_spin()?;
        self.check_analysis_sampling()?;
        self.harmonics.validate_coefficients(alm)?;
        check_len("coefficient input", self.harmonics.nalm(), alm.len())?;
        check_len("map output", self.sample_count, map.len())?;
        self.check_workspace(workspace)?;

        if self.cc_resampling.is_some() {
            self.adjoint_analysis_cc_into(alm, map, workspace)
        } else {
            self.synthesis_into(alm, map, workspace)?;
            self.apply_analysis_pixel_weights(map);
            Ok(())
        }
    }

    fn adjoint_analysis_cc_into(
        &self,
        alm: &[Complex<T>],
        map: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let resampling = self
            .cc_resampling
            .as_ref()
            .expect("CC adjoint analysis requires the prepared CC operator");
        let prepared_ntheta = resampling.prepared_ntheta();
        for (ring, &theta) in resampling.prepared_theta().iter().enumerate() {
            (workspace.theta_sin[ring], workspace.theta_cos[ring]) = sin_cos_from_f64::<T>(theta);
            workspace.ring_order_limits[ring] = T::polar_order_limit(
                self.harmonics.lmax(),
                0,
                workspace.theta_sin[ring],
                workspace.theta_cos[ring],
            );
        }
        workspace
            .prepared_spectra
            .fill(Complex::new(T::zero(), T::zero()));
        self.synthesize_ring_group(
            alm,
            1,
            self.fft.spectrum_len(),
            self.grid.phi0(),
            0,
            &mut workspace.prepared_spectra,
            &workspace.theta_sin[..prepared_ntheta],
            &workspace.theta_cos[..prepared_ntheta],
            &workspace.ring_order_limits[..prepared_ntheta],
            pulp::Arch::new(),
            &mut workspace.recurrence_workers[0],
        )?;
        resampling.restore_original(
            &workspace.prepared_spectra,
            &mut workspace.spectra,
            self.fft.spectrum_len(),
            self.harmonics.mmax(),
            0,
            &mut workspace.cc_profile,
            &mut workspace.cc_fft_scratch,
        )?;
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        for ring in 0..self.grid.ntheta() {
            let start = ring * spectrum_len;
            workspace.spectra[start].im = T::zero();
            self.fft.inverse(
                &workspace.spectra[start..start + spectrum_len],
                &mut workspace.row,
                &mut workspace.fft,
            )?;
            let output_start = ring * nphi;
            map[output_start..output_start + nphi].copy_from_slice(&workspace.row);
        }
        Ok(())
    }

    fn analyze_map_to_coefficients(
        &self,
        map: &[T],
        alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
        ring_weights: Option<&[f64]>,
    ) -> Result<(), ShtError> {
        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        #[cfg(all(test, feature = "parallel"))]
        let mut phase_started = std::time::Instant::now();
        #[cfg(all(test, feature = "parallel"))]
        let phase_probe = workspace.transform_phase_probe();
        let scalar_table_selected = self.scalar_legendre_strategy_with_capacity(
            LegendreOperation::Analysis,
            workspace.execution_capacity,
        ) == LegendreStrategy::CompactTable;
        let scalar_table_input_limit = scalar_table_selected
            .then(crate::legendre::ScalarLegendreTable::<T>::input_scale_limit);
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "strategy selection",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }
        #[cfg(feature = "parallel")]
        let parallel_fft = self.row_fft_parallelism_supported()
            && workspace
                .execution_capacity
                .min(rayon::current_num_threads())
                > 1;
        #[cfg(not(feature = "parallel"))]
        let parallel_fft = false;
        if parallel_fft {
            self.forward_analysis_rows(
                map,
                &mut workspace.spectra,
                &mut workspace.fft,
                #[cfg(feature = "parallel")]
                &mut workspace.fft_worker_workspaces,
                #[cfg(feature = "parallel")]
                workspace.execution_capacity,
            )?;
        } else {
            for ring in 0..ntheta {
                let input_start = ring * nphi;
                let spectrum_start = ring * spectrum_len;
                self.fft.forward(
                    &map[input_start..input_start + nphi],
                    &mut workspace.spectra[spectrum_start..spectrum_start + spectrum_len],
                    &mut workspace.fft,
                )?;
            }
        }
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(&phase_probe, "forward FFT", phase_started);
            phase_started = std::time::Instant::now();
        }

        let (analysis_spectra, analysis_ntheta, analysis_ring_weights) = if let Some(resampling) =
            self.cc_resampling
                .as_ref()
                .filter(|_| ring_weights.is_some())
        {
            for value in &mut workspace.spectra {
                *value /= T::from_usize(nphi).expect("longitude count converts");
            }
            resampling.to_prepared(
                &workspace.spectra,
                &mut workspace.prepared_spectra,
                spectrum_len,
                self.harmonics.mmax(),
                0,
                &mut workspace.cc_profile,
                &mut workspace.cc_fft_scratch,
            )?;
            for (ring, &theta) in resampling.prepared_theta().iter().enumerate() {
                (workspace.theta_sin[ring], workspace.theta_cos[ring]) =
                    sin_cos_from_f64::<T>(theta);
                workspace.theta_cos_squared[ring] = stable_analysis_cosine_squared(
                    workspace.theta_sin[ring],
                    workspace.theta_cos[ring],
                );
                workspace.ring_order_limits[ring] = T::polar_order_limit(
                    self.harmonics.lmax(),
                    0,
                    workspace.theta_sin[ring],
                    workspace.theta_cos[ring],
                );
            }
            (
                workspace.prepared_spectra.as_slice(),
                resampling.prepared_ntheta(),
                None,
            )
        } else {
            for (ring, &theta) in self.grid.theta().iter().enumerate() {
                (workspace.theta_sin[ring], workspace.theta_cos[ring]) =
                    sin_cos_from_f64::<T>(theta);
                workspace.theta_cos_squared[ring] = stable_analysis_cosine_squared(
                    workspace.theta_sin[ring],
                    workspace.theta_cos[ring],
                );
                workspace.ring_order_limits[ring] = T::polar_order_limit(
                    self.harmonics.lmax(),
                    0,
                    workspace.theta_sin[ring],
                    workspace.theta_cos[ring],
                );
            }
            (workspace.spectra.as_slice(), ntheta, ring_weights)
        };
        let allow_insignificant_prefix = analysis_ring_weights.is_some()
            && self.grid.kind() == GridKind::GaussLegendre
            && T::scalar_analysis_prefix_skip_allowed(analysis_ntheta);
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "resampling and latitude setup",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }
        let theta_sin = &workspace.theta_sin[..analysis_ntheta];
        let theta_cos = &workspace.theta_cos[..analysis_ntheta];
        let ring_order_limits = &workspace.ring_order_limits[..analysis_ntheta];

        // Scalar plans do not use their spin-spectrum buffers, so analysis can
        // use them for a full order-major copy without growing the workspace.
        let order_count = self.harmonics.mmax() + 1;
        let order_spectrum_len =
            order_count
                .checked_mul(analysis_ntheta)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "order-major analysis spectrum count",
                })?;
        let order_spectra = if analysis_ntheta == ntheta {
            &mut workspace.spin_spectra[..order_spectrum_len]
        } else {
            &mut workspace.prepared_spin_spectra[..order_spectrum_len]
        };
        let longitude_count = T::from_usize(nphi).expect("longitude count converts");
        let order_preparation = ScalarAnalysisOrderPreparation {
            analysis_spectra,
            spectrum_len,
            phi0: self.grid.phi0(),
            ring_order_limits,
            scalar_table_selected,
            ring_weights: analysis_ring_weights,
            longitude_count,
            scalar_table_input_limit,
        };
        #[cfg(feature = "parallel")]
        let scalar_table_inputs_safe = {
            let worker_count = rayon::current_num_threads()
                .min(workspace.execution_capacity)
                .min(order_count);
            if !scalar_table_selected && worker_count > 1 {
                order_spectra
                    .par_chunks_mut(analysis_ntheta)
                    .enumerate()
                    .with_min_len(order_count.div_ceil(worker_count))
                    .for_each(|(order, order_row)| {
                        let inputs_safe = order_preparation.prepare_order(order, order_row);
                        debug_assert!(inputs_safe);
                    });
                true
            } else {
                order_preparation.prepare_serial(order_spectra)
            }
        };
        #[cfg(not(feature = "parallel"))]
        let scalar_table_inputs_safe = order_preparation.prepare_serial(order_spectra);
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "order spectrum preparation",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }
        let scalar_table = if scalar_table_selected && scalar_table_inputs_safe {
            self.get_scalar_legendre_table()
        } else {
            None
        };
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "Legendre table retrieval",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }

        alm.fill(Complex::new(T::zero(), T::zero()));
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "coefficient initialization",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }
        let lmax = self.harmonics.lmax();
        let mmax = self.harmonics.mmax();
        if let Some(table) = scalar_table {
            self.analyze_scalar_table(order_spectra, alm, table, workspace.execution_capacity);
            #[cfg(all(test, feature = "parallel"))]
            {
                crate::workspace::record_transform_phase(
                    &phase_probe,
                    "Legendre recurrence and accumulation",
                    phase_started,
                );
                phase_started = std::time::Instant::now();
            }
            for l in 0..=lmax {
                let index = self
                    .harmonics
                    .index(l, 0)
                    .expect("order zero contains every degree");
                alm[index].im = T::zero();
            }
            #[cfg(all(test, feature = "parallel"))]
            crate::workspace::record_transform_phase(
                &phase_probe,
                "coefficient cleanup",
                phase_started,
            );
            return Ok(());
        }
        let arch = pulp::Arch::new();
        let selected_arch = std::mem::discriminant(&arch);
        let same_plan = workspace
            .analysis_pair_range_cache_id
            .as_ref()
            .is_some_and(|cache_id| {
                std::sync::Arc::ptr_eq(cache_id, &self.analysis_pair_range_cache_id)
            });
        let cache_geometry = (analysis_ntheta, allow_insignificant_prefix);
        if !same_plan
            || workspace.analysis_pair_range_arch != Some(selected_arch)
            || workspace.analysis_pair_range_geometry != Some(cache_geometry)
        {
            for cached in &workspace.analysis_pair_range_safe {
                cached.store(false, Ordering::Relaxed);
            }
            workspace.analysis_pair_range_cache_id =
                Some(std::sync::Arc::clone(&self.analysis_pair_range_cache_id));
            workspace.analysis_pair_range_arch = Some(selected_arch);
            workspace.analysis_pair_range_geometry = Some(cache_geometry);
        }

        #[cfg(feature = "parallel")]
        {
            let order_count = mmax + 1;
            let worker_count = rayon::current_num_threads()
                .min(workspace.execution_capacity)
                .min(workspace.recurrence_workers.len())
                .min(order_count);
            let use_parallel = rayon::current_num_threads() > 1 && worker_count > 1;
            if use_parallel {
                self.analyze_order_jobs_dynamic(
                    order_spectra,
                    analysis_ntheta,
                    theta_sin,
                    theta_cos,
                    &workspace.theta_cos_squared[..analysis_ntheta],
                    ring_order_limits,
                    arch,
                    allow_insignificant_prefix,
                    &workspace.analysis_pair_range_safe,
                    alm,
                    &mut workspace.recurrence_workers[..worker_count],
                )?;
                #[cfg(all(test, feature = "parallel"))]
                {
                    crate::workspace::record_transform_phase(
                        &phase_probe,
                        "Legendre recurrence and accumulation",
                        phase_started,
                    );
                    phase_started = std::time::Instant::now();
                }
            } else {
                #[cfg(all(test, feature = "parallel"))]
                {
                    phase_started = std::time::Instant::now();
                }
                self.analyze_order_group(
                    order_spectra,
                    analysis_ntheta,
                    theta_sin,
                    theta_cos,
                    &workspace.theta_cos_squared[..analysis_ntheta],
                    ring_order_limits,
                    arch,
                    allow_insignificant_prefix,
                    &workspace.analysis_pair_range_safe,
                    0,
                    order_count,
                    alm,
                    &mut workspace.recurrence_workers[0],
                )?;
                #[cfg(all(test, feature = "parallel"))]
                {
                    crate::workspace::record_transform_phase(
                        &phase_probe,
                        "Legendre recurrence and accumulation",
                        phase_started,
                    );
                    phase_started = std::time::Instant::now();
                }
            }
        }
        #[cfg(not(feature = "parallel"))]
        {
            let order_count = mmax + 1;
            self.analyze_order_group(
                order_spectra,
                analysis_ntheta,
                theta_sin,
                theta_cos,
                &workspace.theta_cos_squared[..analysis_ntheta],
                ring_order_limits,
                arch,
                allow_insignificant_prefix,
                &workspace.analysis_pair_range_safe,
                0,
                order_count,
                alm,
                &mut workspace.recurrence_workers[0],
            )?;
        }
        for l in 0..=lmax {
            let index = self
                .harmonics
                .index(l, 0)
                .expect("order zero contains every degree");
            alm[index].im = T::zero();
        }
        #[cfg(all(test, feature = "parallel"))]
        crate::workspace::record_transform_phase(
            &phase_probe,
            "coefficient cleanup",
            phase_started,
        );
        Ok(())
    }

    fn analyze_scalar_table(
        &self,
        order_spectra: &[Complex<T>],
        alm: &mut [Complex<T>],
        table: &crate::legendre::ScalarLegendreTable<T>,
        execution_capacity: usize,
    ) {
        let order_count = self.harmonics.mmax() + 1;
        #[cfg(not(feature = "parallel"))]
        let _ = execution_capacity;
        #[cfg(feature = "parallel")]
        {
            let workers = rayon::current_num_threads()
                .min(execution_capacity)
                .min(order_count)
                .max(1);
            self.analyze_scalar_table_groups(order_spectra, table, 0, order_count, alm, workers);
        }
        #[cfg(not(feature = "parallel"))]
        self.analyze_scalar_table_group(order_spectra, table, 0, order_count, alm);
    }

    #[cfg(feature = "parallel")]
    fn analyze_scalar_table_groups(
        &self,
        order_spectra: &[Complex<T>],
        table: &crate::legendre::ScalarLegendreTable<T>,
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<T>],
        worker_count: usize,
    ) {
        if m_end - m_start <= 1 || worker_count <= 1 {
            self.analyze_scalar_table_group(order_spectra, table, m_start, m_end, alm);
            return;
        }
        let m_mid = weighted_order_midpoint(self.harmonics.lmax(), m_start, m_end);
        let coefficient_start = self
            .harmonics
            .index(m_start, m_start)
            .expect("every packed order starts at its diagonal degree");
        let coefficient_mid = self
            .harmonics
            .index(m_mid, m_mid)
            .expect("the end of an order group starts the next order");
        let (left_alm, right_alm) = alm.split_at_mut(coefficient_mid - coefficient_start);
        let left_worker_count = worker_count.div_ceil(2);
        rayon::join(
            || {
                self.analyze_scalar_table_groups(
                    order_spectra,
                    table,
                    m_start,
                    m_mid,
                    left_alm,
                    left_worker_count,
                );
            },
            || {
                self.analyze_scalar_table_groups(
                    order_spectra,
                    table,
                    m_mid,
                    m_end,
                    right_alm,
                    worker_count - left_worker_count,
                );
            },
        );
    }

    fn analyze_scalar_table_group(
        &self,
        order_spectra: &[Complex<T>],
        table: &crate::legendre::ScalarLegendreTable<T>,
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<T>],
    ) {
        let ntheta = self.grid.ntheta();
        let lmax = self.harmonics.lmax();
        let mut coefficient_start = 0;
        for m in m_start..m_end {
            let coefficient_count = lmax - m + 1;
            let coefficients = &mut alm[coefficient_start..coefficient_start + coefficient_count];
            for (degree_offset, coefficient) in coefficients.iter_mut().enumerate() {
                let l = m + degree_offset;
                let parity = if (l + m).is_multiple_of(2) {
                    T::one()
                } else {
                    -T::one()
                };
                let mut sum = Complex::new(T::zero(), T::zero());
                for (north_ring, &basis) in table.degree_lanes(m, degree_offset).iter().enumerate()
                {
                    let south_ring = ntheta - 1 - north_ring;
                    let north = order_spectra[m * ntheta + north_ring];
                    let south = if south_ring != north_ring {
                        order_spectra[m * ntheta + south_ring] * parity
                    } else {
                        Complex::new(T::zero(), T::zero())
                    };
                    sum += (north + south) * basis;
                }
                *coefficient = sum;
            }
            coefficient_start += coefficient_count;
        }
    }

    #[cfg(feature = "parallel")]
    #[expect(
        clippy::too_many_arguments,
        reason = "Dynamic analysis jobs share plan geometry and own disjoint coefficient slices."
    )]
    fn analyze_order_jobs_dynamic(
        &self,
        spectra: &[Complex<T>],
        latitude_count: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        theta_cos_squared: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        allow_insignificant_prefix: bool,
        analysis_pair_range_safe: &[AtomicBool],
        alm: &mut [Complex<T>],
        recurrence: &mut [RecurrenceWorkspace<T>],
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let order_count = self.harmonics.mmax() + 1;
        if order_count <= 1 || recurrence.len() <= 1 {
            return self.analyze_order_group(
                spectra,
                latitude_count,
                theta_sin,
                theta_cos,
                theta_cos_squared,
                ring_order_limits,
                arch,
                allow_insignificant_prefix,
                analysis_pair_range_safe,
                0,
                order_count,
                alm,
                recurrence
                    .first_mut()
                    .expect("each analysis group has at least one recurrence workspace"),
            );
        }

        let mut order_jobs = VecDeque::new();
        if order_jobs.try_reserve_exact(order_count).is_err() {
            return self.analyze_order_group(
                spectra,
                latitude_count,
                theta_sin,
                theta_cos,
                theta_cos_squared,
                ring_order_limits,
                arch,
                allow_insignificant_prefix,
                analysis_pair_range_safe,
                0,
                order_count,
                alm,
                &mut recurrence[0],
            );
        }
        let mut remaining = alm;
        for m in 0..order_count {
            let coefficient_count = lmax - m + 1;
            let (coefficients, tail) = remaining.split_at_mut(coefficient_count);
            order_jobs.push_back((m, coefficients));
            remaining = tail;
        }
        debug_assert!(remaining.is_empty());

        let order_jobs = Mutex::new(order_jobs);
        let failed = AtomicBool::new(false);
        let failure = Mutex::new(None::<ShtError>);
        recurrence.par_iter_mut().for_each(|worker| {
            loop {
                if failed.load(Ordering::Relaxed) {
                    break;
                }
                let job = order_jobs
                    .lock()
                    .expect("scalar analysis order queue is not poisoned")
                    .pop_front();
                let Some((m, coefficients)) = job else {
                    break;
                };
                if let Err(error) = self.analyze_order_group(
                    spectra,
                    latitude_count,
                    theta_sin,
                    theta_cos,
                    theta_cos_squared,
                    ring_order_limits,
                    arch,
                    allow_insignificant_prefix,
                    analysis_pair_range_safe,
                    m,
                    m + 1,
                    coefficients,
                    worker,
                ) {
                    failed.store(true, Ordering::Relaxed);
                    let mut first_error = failure
                        .lock()
                        .expect("scalar analysis error slot is not poisoned");
                    first_error.get_or_insert(error);
                    break;
                }
            }
        });

        if failed.load(Ordering::Relaxed) {
            return Err(failure
                .into_inner()
                .expect("scalar analysis error slot is not poisoned")
                .expect("a failed analysis job records its error"));
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The recurrence group uses plan geometry, grid weights and caller-owned worker scratch."
    )]
    fn analyze_order_group(
        &self,
        spectra: &[Complex<T>],
        latitude_count: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        theta_cos_squared: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        allow_insignificant_prefix: bool,
        analysis_pair_range_safe: &[AtomicBool],
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<T>],
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        #[cfg(all(test, feature = "parallel"))]
        recurrence.record_active_worker();
        #[cfg(all(test, feature = "parallel"))]
        let profile_started = recurrence
            .analysis_leaf_profile_enabled()
            .then(std::time::Instant::now);
        #[cfg(all(test, feature = "parallel"))]
        recurrence.reset_analysis_leaf_block_counts();
        let result = self.analyze_symmetric_order_group(
            spectra,
            latitude_count,
            theta_sin,
            theta_cos,
            theta_cos_squared,
            ring_order_limits,
            arch,
            allow_insignificant_prefix,
            analysis_pair_range_safe,
            m_start,
            m_end,
            alm,
            recurrence,
        );
        #[cfg(all(test, feature = "parallel"))]
        if let Some(started) = profile_started {
            recurrence.record_analysis_leaf(
                false,
                self.harmonics.lmax(),
                m_start,
                m_end,
                ring_order_limits,
                started,
            );
        }
        result
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The symmetric analysis group receives plan geometry and worker scratch explicitly."
    )]
    #[allow(unexpected_cfgs)]
    fn analyze_symmetric_order_group(
        &self,
        spectra: &[Complex<T>],
        latitude_count: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        theta_cos_squared: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        allow_insignificant_prefix: bool,
        analysis_pair_range_safe: &[AtomicBool],
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<T>],
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        // GL, CC, F1 and prepared meridional grids use mirrored colatitudes.
        // Scalar harmonics satisfy Y_lm(pi-theta) = (-1)^(l+m) Y_lm(theta).
        debug_assert_eq!(theta_sin.len(), latitude_count);
        debug_assert_eq!(theta_cos.len(), latitude_count);
        debug_assert_eq!(theta_cos_squared.len(), latitude_count);
        debug_assert_eq!(ring_order_limits.len(), latitude_count);
        let lmax = self.harmonics.lmax();
        let pair_lane_count = latitude_count.div_ceil(2);
        let pair_block_size = if T::supports_paired_scalar() {
            SCALAR_ANALYSIS_PAIR_TILE_SIZE
        } else {
            LATITUDE_BLOCK_SIZE
        };
        let range_cache_block_count = pair_lane_count.div_ceil(pair_block_size);
        debug_assert!(recurrence.previous.len() >= pair_block_size);
        let mut coefficient_start = 0;
        #[cfg(all(test, feature = "parallel"))]
        let profiling = recurrence.analysis_leaf_profile_enabled();

        for m in m_start..m_end {
            let coefficient_count = lmax - m + 1;
            let coefficients = &mut alm[coefficient_start..coefficient_start + coefficient_count];
            let first_active_pair =
                first_active_symmetric_pair(m, latitude_count, ring_order_limits);
            T::fill_epsilon(m, lmax, &mut recurrence.epsilon);
            let mut paired_used = T::fill_paired_scalar_factors(
                m,
                lmax,
                &recurrence.epsilon,
                &mut recurrence.spin_alpha,
                &mut recurrence.spin_coefficient_a,
                &mut recurrence.spin_coefficient_b,
            );
            for block_start in (first_active_pair..pair_lane_count).step_by(pair_block_size) {
                let block_end = (block_start + pair_block_size).min(pair_lane_count);
                let block_len = block_end - block_start;
                let order_spectra = &spectra[m * latitude_count..(m + 1) * latitude_count];
                let compact_candidate = T::supports_paired_scalar() && lmax >= 127;
                let lane_count = prepare_symmetric_analysis_pair_block(
                    m,
                    latitude_count,
                    block_start,
                    block_len,
                    compact_candidate,
                    order_spectra,
                    ring_order_limits,
                    theta_sin,
                    theta_cos,
                    theta_cos_squared,
                    &mut recurrence.analysis_same_real,
                    &mut recurrence.analysis_same_imag,
                    &mut recurrence.analysis_opposite_real,
                    &mut recurrence.analysis_opposite_imag,
                    &mut recurrence.compact_ring_indices,
                    &mut recurrence.compact_theta_sin,
                    &mut recurrence.compact_theta_cos,
                    &mut recurrence.compact_theta_cos_squared,
                );
                if lane_count == 0 {
                    continue;
                }
                let cache_index = m
                    .checked_mul(range_cache_block_count)
                    .and_then(|order_index| order_index.checked_add(block_start / pair_block_size));
                let range_cache_enabled = !cfg!(rustsht_disable_analysis_pair_range_cache);
                let range_check_is_proven_safe = range_cache_enabled
                    && cache_index
                        .and_then(|index| analysis_pair_range_safe.get(index))
                        .is_some_and(|safe| safe.load(Ordering::Relaxed));
                // Recurrence state depends on this plan's fixed factors and
                // active geometry, not the map spectrum. Cache each block only
                // after its checked fused traversal has exercised group checks.
                let mut may_cache_range_safe = range_cache_enabled
                    && !range_check_is_proven_safe
                    && !cfg!(rustsht_unchecked_analysis_pair_range);
                let mut saw_successful_fused_block = false;
                #[cfg(all(test, feature = "parallel"))]
                let (mut block_saw_normal, mut block_saw_scaled) = (false, false);
                let compact_mixed_block = compact_candidate && lane_count < block_len;
                let pair_sin = if compact_mixed_block {
                    &recurrence.compact_theta_sin[..lane_count]
                } else {
                    &theta_sin[block_start..block_end]
                };
                let pair_cos = if compact_mixed_block {
                    &recurrence.compact_theta_cos[..lane_count]
                } else {
                    &theta_cos[block_start..block_end]
                };
                let pair_cos_squared = if compact_mixed_block {
                    &recurrence.compact_theta_cos_squared[..lane_count]
                } else {
                    &theta_cos_squared[block_start..block_end]
                };
                if paired_used {
                    // Paired degrees l=m+2k use north+south for the even term;
                    // the intervening odd term uses north-south and cos(theta).
                    let staged_count = coefficient_count;
                    let fast_path_candidate = lmax >= FUSED_SCALAR_ANALYSIS_MIN_LMAX
                        && recurrence.fast_scalar_first.len() >= staged_count;
                    // DUCC's fill_m2a checks the packed spectra and cosine-weights
                    // opposite parity in one pass. Preserve the transformed
                    // values only when both range checks succeed.
                    let packed_preparation = if fast_path_candidate {
                        T::prepare_scalar_analysis_pair_inputs(
                            &recurrence.analysis_same_real[..lane_count],
                            &recurrence.analysis_same_imag[..lane_count],
                            &mut recurrence.analysis_opposite_real[..lane_count],
                            &mut recurrence.analysis_opposite_imag[..lane_count],
                            pair_cos,
                        )
                    } else {
                        None
                    };
                    let (product_range_safe, opposite_preparation) =
                        if let Some((products_safe, opposite_safe)) = packed_preparation {
                            (products_safe, Some(opposite_safe))
                        } else {
                            let products_safe = T::scalar_analysis_products_safe(
                                &recurrence.analysis_same_real[..lane_count],
                                &recurrence.analysis_same_imag[..lane_count],
                                &recurrence.analysis_opposite_real[..lane_count],
                                &recurrence.analysis_opposite_imag[..lane_count],
                                pair_cos,
                            );
                            let opposite_preparation = if fast_path_candidate && products_safe {
                                T::prepare_scalar_analysis_opposite(
                                    &mut recurrence.analysis_opposite_real[..lane_count],
                                    &mut recurrence.analysis_opposite_imag[..lane_count],
                                    pair_cos,
                                )
                            } else {
                                None
                            };
                            (products_safe, opposite_preparation)
                        };
                    let fast_path_eligible = opposite_preparation == Some(true);
                    let mut range_check_was_exercised = false;
                    let fused_block = if fast_path_eligible {
                        T::fused_scalar_analysis_pair_block_preweighted(
                            arch,
                            m,
                            lmax,
                            allow_insignificant_prefix,
                            range_check_is_proven_safe,
                            &mut range_check_was_exercised,
                            pair_sin,
                            pair_cos,
                            pair_cos_squared,
                            self.sectoral_norms[m],
                            &recurrence.spin_coefficient_a,
                            &recurrence.spin_coefficient_b,
                            &recurrence.analysis_same_real[..lane_count],
                            &recurrence.analysis_same_imag[..lane_count],
                            &recurrence.analysis_opposite_real[..lane_count],
                            &recurrence.analysis_opposite_imag[..lane_count],
                            &mut recurrence.fast_scalar_first[..staged_count],
                            &mut recurrence.previous,
                            &mut recurrence.current,
                            &mut recurrence.next,
                            &mut recurrence.analysis_scale_epochs,
                            &mut recurrence.analysis_scale_corrections,
                            &mut recurrence.exponents,
                        )?
                    } else {
                        false
                    };
                    if fused_block {
                        saw_successful_fused_block = true;
                        if !range_check_is_proven_safe && !range_check_was_exercised {
                            may_cache_range_safe = false;
                        }
                        #[cfg(all(test, feature = "parallel"))]
                        {
                            block_saw_normal = true;
                        }
                        for (coefficient, contribution) in coefficients
                            .iter_mut()
                            .zip(&recurrence.fast_scalar_first[..staged_count])
                        {
                            *coefficient += *contribution;
                        }
                    } else {
                        may_cache_range_safe = false;
                        if opposite_preparation.is_some() {
                            restore_symmetric_analysis_pair_opposite(
                                m,
                                latitude_count,
                                order_spectra,
                                ring_order_limits,
                                &recurrence.compact_ring_indices[..lane_count],
                                &mut recurrence.analysis_opposite_real[..lane_count],
                                &mut recurrence.analysis_opposite_imag[..lane_count],
                            );
                        }
                        let same_real = &recurrence.analysis_same_real[..lane_count];
                        let same_imag = &recurrence.analysis_same_imag[..lane_count];
                        let opposite_real = &recurrence.analysis_opposite_real[..lane_count];
                        let opposite_imag = &recurrence.analysis_opposite_imag[..lane_count];
                        paired_used = T::for_each_y_lm_paired_status_dispatched(
                            arch,
                            m,
                            lmax,
                            pair_sin,
                            pair_cos,
                            self.sectoral_norms[m],
                            &recurrence.spin_coefficient_a,
                            &recurrence.spin_coefficient_b,
                            &mut recurrence.previous,
                            &mut recurrence.current,
                            &mut recurrence.next,
                            &mut recurrence.exponents,
                            |l, ring_start, current, exponents, normal_range| {
                                #[cfg(all(test, feature = "parallel"))]
                                if profiling {
                                    if normal_range {
                                        block_saw_normal = true;
                                    } else {
                                        block_saw_scaled = true;
                                    }
                                }
                                let offset = l - m;
                                let mut even_coefficient = coefficients[offset];
                                let mut odd_coefficient = if l < lmax {
                                    coefficients[offset + 1]
                                } else {
                                    Complex::new(T::zero(), T::zero())
                                };
                                let lane_end = ring_start + current.len();
                                let (same_sum, opposite_sum) = T::accumulate_scalar_analysis_pair(
                                    arch,
                                    product_range_safe,
                                    normal_range,
                                    &same_real[ring_start..lane_end],
                                    &same_imag[ring_start..lane_end],
                                    &opposite_real[ring_start..lane_end],
                                    &opposite_imag[ring_start..lane_end],
                                    current,
                                    exponents,
                                    &pair_cos[ring_start..lane_end],
                                );
                                even_coefficient += same_sum;
                                if l < lmax {
                                    odd_coefficient += opposite_sum;
                                }
                                coefficients[offset] = even_coefficient;
                                if l < lmax {
                                    coefficients[offset + 1] = odd_coefficient;
                                }
                                Ok(())
                            },
                        )?;
                    }
                    if !fast_path_eligible {
                        may_cache_range_safe = false;
                    }
                } else {
                    may_cache_range_safe = false;
                }
                if !paired_used {
                    let same_real = &recurrence.analysis_same_real[..lane_count];
                    let same_imag = &recurrence.analysis_same_imag[..lane_count];
                    let opposite_real = &recurrence.analysis_opposite_real[..lane_count];
                    let opposite_imag = &recurrence.analysis_opposite_imag[..lane_count];
                    T::for_each_y_lm_dispatched(
                        arch,
                        m,
                        lmax,
                        pair_sin,
                        pair_cos,
                        self.sectoral_norms[m],
                        &recurrence.epsilon,
                        &mut recurrence.previous,
                        &mut recurrence.current,
                        &mut recurrence.next,
                        &mut recurrence.exponents,
                        |l, ring_start, current, exponents| {
                            #[cfg(all(test, feature = "parallel"))]
                            if profiling {
                                if exponents.iter().any(|&exponent| exponent != 0) {
                                    block_saw_scaled = true;
                                } else {
                                    block_saw_normal = true;
                                }
                            }
                            let coefficient = &mut coefficients[l - m];
                            let (spectrum_real, spectrum_imag) = if (l + m).is_multiple_of(2) {
                                (same_real, same_imag)
                            } else {
                                (opposite_real, opposite_imag)
                            };
                            for lane in 0..current.len() {
                                let ring = ring_start + lane;
                                *coefficient += T::multiply_complex_scaled(
                                    Complex::new(spectrum_real[ring], spectrum_imag[ring]),
                                    current[lane],
                                    exponents[lane],
                                );
                            }
                            Ok(())
                        },
                    )?;
                }
                #[cfg(all(test, feature = "parallel"))]
                if profiling {
                    recurrence.record_analysis_leaf_block(block_saw_normal, block_saw_scaled);
                }
                if may_cache_range_safe
                    && saw_successful_fused_block
                    && let Some(index) = cache_index
                    && let Some(safe) = analysis_pair_range_safe.get(index)
                {
                    safe.store(true, Ordering::Relaxed);
                }
            }
            if paired_used {
                finalize_paired_scalar_analysis_coefficients(
                    coefficients,
                    m,
                    lmax,
                    &recurrence.epsilon,
                    &recurrence.spin_alpha,
                );
            }
            coefficient_start += coefficient_count;
        }
        debug_assert_eq!(coefficient_start, alm.len());
        Ok(())
    }

    fn check_scalar_spin(&self) -> Result<(), ShtError> {
        if self.harmonics.spin() == 0 {
            Ok(())
        } else {
            Err(ShtError::UnsupportedSpin {
                spin: self.harmonics.spin(),
            })
        }
    }

    fn check_synthesis_order(&self) -> Result<(), ShtError> {
        if self.harmonics.mmax() <= self.grid.nphi() / 2 {
            Ok(())
        } else {
            Err(ShtError::SynthesisLongitudeUndersampled {
                mmax: self.harmonics.mmax(),
                nphi: self.grid.nphi(),
            })
        }
    }

    pub(crate) fn check_analysis_sampling(&self) -> Result<(), ShtError> {
        let minimum_ntheta = match self.grid.kind() {
            GridKind::GaussLegendre => self.harmonics.lmax().checked_add(1),
            GridKind::Fejer1 => self.harmonics.lmax().checked_add(1),
            GridKind::ClenshawCurtis => self.harmonics.lmax().checked_add(2),
        }
        .ok_or(ShtError::IntegerOverflow {
            quantity: "exact-analysis latitude count",
        })?;
        if self.grid.ntheta() < minimum_ntheta {
            return Err(ShtError::AnalysisLatitudeUndersampled {
                lmax: self.harmonics.lmax(),
                ntheta: self.grid.ntheta(),
            });
        }
        if self.harmonics.mmax() > (self.grid.nphi() - 1) / 2 {
            return Err(ShtError::AnalysisLongitudeUndersampled {
                mmax: self.harmonics.mmax(),
                nphi: self.grid.nphi(),
            });
        }
        Ok(())
    }

    pub(crate) fn apply_analysis_pixel_weights(&self, map: &mut [T]) {
        let nphi = self.grid.nphi();
        for (ring, &ring_weight) in self.grid.ring_weights().iter().enumerate() {
            let scale = T::from_f64(ring_weight / nphi as f64)
                .expect("analysis pixel weight is representable");
            let start = ring * nphi;
            for sample in &mut map[start..start + nphi] {
                *sample *= scale;
            }
        }
    }

    pub(crate) fn check_workspace(&self, workspace: &ShtWorkspace<T>) -> Result<(), ShtError> {
        let prepared_ntheta = self
            .cc_resampling
            .as_ref()
            .map_or(0, |resampling| resampling.prepared_ntheta());
        let latitude_count = self.grid.ntheta().max(prepared_ntheta);
        let recurrence_scratch_len = recurrence_scratch_len(latitude_count);
        if workspace.ring_group_size == 0 {
            return Err(ShtError::BufferLength {
                buffer: "workspace latitude group size",
                expected: 1,
                actual: 0,
            });
        }
        check_len(
            "workspace Fourier spectra",
            self.spectrum_count,
            workspace.spectra.len(),
        )?;
        check_len(
            "workspace spin Fourier spectra",
            self.spectrum_count,
            workspace.spin_spectra.len(),
        )?;
        let prepared_spectrum_count = prepared_ntheta.checked_mul(self.fft.spectrum_len()).ok_or(
            ShtError::IntegerOverflow {
                quantity: "prepared CC Fourier spectrum count",
            },
        )?;
        check_len(
            "workspace prepared CC Fourier spectra",
            prepared_spectrum_count,
            workspace.prepared_spectra.len(),
        )?;
        check_len(
            "workspace prepared spin CC Fourier spectra",
            prepared_spectrum_count,
            workspace.prepared_spin_spectra.len(),
        )?;
        check_len(
            "workspace CC meridional profile",
            self.cc_resampling
                .as_ref()
                .map_or(0, |resampling| resampling.workspace_profile_len()),
            workspace.cc_profile.len(),
        )?;
        check_len(
            "workspace CC meridional FFT scratch",
            self.cc_resampling
                .as_ref()
                .map_or(0, |resampling| resampling.workspace_fft_scratch_len()),
            workspace.cc_fft_scratch.len(),
        )?;
        check_len(
            "workspace longitude row",
            self.grid.nphi(),
            workspace.row.len(),
        )?;
        for (buffer, actual) in [
            ("workspace latitude sine values", workspace.theta_sin.len()),
            (
                "workspace latitude cosine values",
                workspace.theta_cos.len(),
            ),
            (
                "workspace latitude cosine-square values",
                workspace.theta_cos_squared.len(),
            ),
            (
                "workspace analysis ring scales",
                workspace.analysis_ring_scales.len(),
            ),
            (
                "workspace latitude order limits",
                workspace.ring_order_limits.len(),
            ),
        ] {
            check_len(buffer, latitude_count, actual)?;
        }
        let expected_worker_count = self
            .grid
            .ntheta()
            .div_ceil(workspace.ring_group_size.min(self.grid.ntheta()));
        if workspace.recurrence_workers.len() < expected_worker_count {
            return Err(ShtError::BufferLength {
                buffer: "workspace recurrence worker groups",
                expected: expected_worker_count,
                actual: workspace.recurrence_workers.len(),
            });
        }
        for recurrence in &workspace.recurrence_workers {
            for (buffer, expected, actual) in [
                (
                    "workspace scalar recurrence factors",
                    self.recurrence_factor_count,
                    recurrence.epsilon.len(),
                ),
                (
                    "workspace previous recurrence values",
                    recurrence_scratch_len,
                    recurrence.previous.len(),
                ),
                (
                    "workspace current recurrence values",
                    recurrence_scratch_len,
                    recurrence.current.len(),
                ),
                (
                    "workspace next recurrence values",
                    recurrence_scratch_len,
                    recurrence.next.len(),
                ),
                (
                    "workspace recurrence exponents",
                    recurrence_scratch_len,
                    recurrence.exponents.len(),
                ),
                (
                    "workspace scalar-analysis scale epochs",
                    recurrence_scratch_len,
                    recurrence.analysis_scale_epochs.len(),
                ),
                (
                    "workspace scalar-analysis scale corrections",
                    recurrence_scratch_len,
                    recurrence.analysis_scale_corrections.len(),
                ),
                (
                    "workspace compact latitude cosine-square values",
                    recurrence_scratch_len,
                    recurrence.compact_theta_cos_squared.len(),
                ),
                (
                    "workspace spin recurrence alpha factors",
                    self.spin_recurrence_factor_count,
                    recurrence.spin_alpha.len(),
                ),
                (
                    "workspace spin recurrence A factors",
                    self.spin_recurrence_factor_count,
                    recurrence.spin_coefficient_a.len(),
                ),
                (
                    "workspace spin recurrence B factors",
                    self.spin_recurrence_factor_count,
                    recurrence.spin_coefficient_b.len(),
                ),
                (
                    "workspace same-parity analysis real spectrum",
                    recurrence_scratch_len,
                    recurrence.analysis_same_real.len(),
                ),
                (
                    "workspace same-parity analysis imaginary spectrum",
                    recurrence_scratch_len,
                    recurrence.analysis_same_imag.len(),
                ),
                (
                    "workspace opposite-parity analysis real spectrum",
                    recurrence_scratch_len,
                    recurrence.analysis_opposite_real.len(),
                ),
                (
                    "workspace opposite-parity analysis imaginary spectrum",
                    recurrence_scratch_len,
                    recurrence.analysis_opposite_imag.len(),
                ),
                (
                    "workspace ordered scalar scale correction",
                    recurrence_scratch_len,
                    recurrence.scalar_order_correction.len(),
                ),
                (
                    "workspace spin-1 fast scalar first coefficients",
                    self.spin_fast_coefficient_count,
                    recurrence.fast_scalar_first.len(),
                ),
                (
                    "workspace spin-1 fast scalar second coefficients",
                    self.spin_fast_coefficient_count,
                    recurrence.fast_scalar_second.len(),
                ),
                (
                    "workspace spin-1 fast E coefficients",
                    self.spin_fast_coefficient_count,
                    recurrence.fast_spin_first.len(),
                ),
                (
                    "workspace spin-1 fast B coefficients",
                    self.spin_fast_coefficient_count,
                    recurrence.fast_spin_second.len(),
                ),
                (
                    "workspace positive-spin previous values",
                    recurrence_scratch_len,
                    recurrence.spin_previous_plus.len(),
                ),
                (
                    "workspace positive-spin current values",
                    recurrence_scratch_len,
                    recurrence.spin_current_plus.len(),
                ),
                (
                    "workspace positive-spin next values",
                    recurrence_scratch_len,
                    recurrence.spin_next_plus.len(),
                ),
                (
                    "workspace positive-spin exponents",
                    recurrence_scratch_len,
                    recurrence.spin_exponents_plus.len(),
                ),
                (
                    "workspace negative-spin previous values",
                    recurrence_scratch_len,
                    recurrence.spin_previous_minus.len(),
                ),
                (
                    "workspace negative-spin current values",
                    recurrence_scratch_len,
                    recurrence.spin_current_minus.len(),
                ),
                (
                    "workspace negative-spin next values",
                    recurrence_scratch_len,
                    recurrence.spin_next_minus.len(),
                ),
                (
                    "workspace negative-spin exponents",
                    recurrence_scratch_len,
                    recurrence.spin_exponents_minus.len(),
                ),
            ] {
                check_len(buffer, expected, actual)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
