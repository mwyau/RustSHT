// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use crate::error::ShtError;
use crate::grid::GridKind;
use crate::legendre::{LATITUDE_BLOCK_SIZE, ShtScalar, longitude_phase, sin_cos_from_f64};
use crate::plan::ShtPlan;
use crate::workspace::{RecurrenceWorkspace, ShtWorkspace};

#[cfg(feature = "parallel")]
pub(crate) fn weighted_order_midpoint(lmax: usize, m_start: usize, m_end: usize) -> usize {
    debug_assert!(m_start < m_end);
    debug_assert!(m_end - 1 <= lmax);
    debug_assert!(m_end - m_start > 1);

    let total_work = (m_start..m_end).map(|m| lmax - m + 1).sum::<usize>();
    let mut left_work = 0;
    let mut best_mid = m_start + 1;
    let mut best_difference = usize::MAX;
    for m in m_start..(m_end - 1) {
        left_work += lmax - m + 1;
        let right_work = total_work - left_work;
        let difference = left_work.abs_diff(right_work);
        if difference < best_difference {
            best_difference = difference;
            best_mid = m + 1;
        }
    }
    best_mid
}

#[allow(private_bounds)]
impl<T: ShtScalar> ShtPlan<T> {
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
        let arch = pulp::Arch::new();
        #[cfg(feature = "parallel")]
        {
            let group_ring_count = workspace.ring_group_size.min(ntheta);
            let parallel_groups = workspace.recurrence_workers.len();
            let use_parallel = rayon::current_num_threads() > 1 && parallel_groups > 1;
            if use_parallel {
                let spectrum_group_len = group_ring_count.checked_mul(spectrum_len).ok_or(
                    ShtError::IntegerOverflow {
                        quantity: "parallel Fourier spectrum group size",
                    },
                )?;
                let expected_groups = ntheta.div_ceil(group_ring_count);
                debug_assert_eq!(expected_groups, parallel_groups);
                workspace
                    .spectra
                    .par_chunks_mut(spectrum_group_len)
                    .zip(workspace.recurrence_workers.par_iter_mut())
                    .enumerate()
                    .map(|(group_index, (group_spectra, recurrence))| {
                        let ring_start = group_index * group_ring_count;
                        let ring_end =
                            (ring_start + group_spectra.len() / spectrum_len).min(ntheta);
                        self.synthesize_ring_group(
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

        for ring in 0..ntheta {
            let start = ring * spectrum_len;
            let ring_spectrum = &workspace.spectra[start..start + spectrum_len];
            self.fft
                .inverse(ring_spectrum, &mut workspace.row, &mut workspace.fft)?;
            let output_start = ring * nphi;
            map[output_start..output_start + nphi].copy_from_slice(&workspace.row);
        }
        Ok(())
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
        let lmax = self.harmonics.lmax();
        let mmax = self.harmonics.mmax();
        debug_assert_eq!(spectra.len(), theta_sin.len() * spectrum_len);
        debug_assert_eq!(theta_sin.len(), theta_cos.len());

        let fourier_scale = T::from_usize(nphi).expect("longitude count converts");
        let nyquist_scale = fourier_scale * T::from_f64(2.0).expect("two is representable");
        for m in 0..=mmax {
            T::fill_epsilon(m, lmax, &mut recurrence.epsilon);
            let coefficient_start = self
                .harmonics
                .index(m, m)
                .expect("every packed order starts at its diagonal degree");
            let phase = if m == 0 {
                Complex::new(T::one(), T::zero())
            } else {
                longitude_phase::<T>(m, phi0, false)
            };
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
                        spectra[spectrum_index] +=
                            T::multiply_complex_scaled(coefficient, current[lane], exponents[lane]);
                    }
                    Ok(())
                },
            )?;
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
                    spectra[spectrum_index] = phased * fourier_scale;
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
        let spectra = &mut workspace.spectra;
        for ring in 0..ntheta {
            let input_start = ring * nphi;
            let spectrum_start = ring * spectrum_len;
            self.fft.forward(
                &map[input_start..input_start + nphi],
                &mut spectra[spectrum_start..spectrum_start + spectrum_len],
                &mut workspace.fft,
            )?;
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
                workspace.ring_order_limits[ring] = T::polar_order_limit(
                    self.harmonics.lmax(),
                    0,
                    workspace.theta_sin[ring],
                    workspace.theta_cos[ring],
                );
            }
            (workspace.spectra.as_slice(), ntheta, ring_weights)
        };
        let theta_sin = &workspace.theta_sin[..analysis_ntheta];
        let theta_cos = &workspace.theta_cos[..analysis_ntheta];
        let ring_order_limits = &workspace.ring_order_limits[..analysis_ntheta];

        alm.fill(Complex::new(T::zero(), T::zero()));
        let lmax = self.harmonics.lmax();
        let mmax = self.harmonics.mmax();
        let arch = pulp::Arch::new();

        #[cfg(feature = "parallel")]
        {
            let order_count = mmax + 1;
            let worker_count = workspace.recurrence_workers.len().min(order_count);
            let use_parallel = rayon::current_num_threads() > 1 && worker_count > 1;
            if use_parallel {
                self.analyze_order_groups_parallel(
                    analysis_spectra,
                    nphi,
                    spectrum_len,
                    self.grid.phi0(),
                    analysis_ring_weights,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    0,
                    order_count,
                    alm,
                    &mut workspace.recurrence_workers[..worker_count],
                )?;
            } else {
                self.analyze_order_group(
                    analysis_spectra,
                    nphi,
                    spectrum_len,
                    self.grid.phi0(),
                    analysis_ring_weights,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    0,
                    order_count,
                    alm,
                    &mut workspace.recurrence_workers[0],
                )?;
            }
        }
        #[cfg(not(feature = "parallel"))]
        {
            let order_count = mmax + 1;
            self.analyze_order_group(
                analysis_spectra,
                nphi,
                spectrum_len,
                self.grid.phi0(),
                analysis_ring_weights,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
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
        Ok(())
    }

    #[cfg(feature = "parallel")]
    #[expect(
        clippy::too_many_arguments,
        reason = "The parallel analysis groups share plan geometry and split caller-owned outputs and scratch."
    )]
    fn analyze_order_groups_parallel(
        &self,
        spectra: &[Complex<T>],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_weights: Option<&[f64]>,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<T>],
        recurrence: &mut [RecurrenceWorkspace<T>],
    ) -> Result<(), ShtError> {
        let order_count = m_end - m_start;
        if order_count <= 1 || recurrence.len() <= 1 {
            return self.analyze_order_group(
                spectra,
                nphi,
                spectrum_len,
                phi0,
                ring_weights,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                m_start,
                m_end,
                alm,
                recurrence
                    .first_mut()
                    .expect("each analysis group has at least one recurrence workspace"),
            );
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
        let coefficient_split = coefficient_mid - coefficient_start;
        let (left_alm, right_alm) = alm.split_at_mut(coefficient_split);
        let left_worker_count = recurrence.len().div_ceil(2);
        let (left_recurrence, right_recurrence) = recurrence.split_at_mut(left_worker_count);

        let (left_result, right_result) = rayon::join(
            || {
                self.analyze_order_groups_parallel(
                    spectra,
                    nphi,
                    spectrum_len,
                    phi0,
                    ring_weights,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    m_start,
                    m_mid,
                    left_alm,
                    left_recurrence,
                )
            },
            || {
                self.analyze_order_groups_parallel(
                    spectra,
                    nphi,
                    spectrum_len,
                    phi0,
                    ring_weights,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    m_mid,
                    m_end,
                    right_alm,
                    right_recurrence,
                )
            },
        );
        left_result?;
        right_result
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The recurrence group uses plan geometry, grid weights and caller-owned worker scratch."
    )]
    fn analyze_order_group(
        &self,
        spectra: &[Complex<T>],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_weights: Option<&[f64]>,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<T>],
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let longitude_count = T::from_usize(nphi).expect("longitude count converts");
        let mut coefficient_start = 0;
        for m in m_start..m_end {
            let coefficient_count = lmax - m + 1;
            let coefficients = &mut alm[coefficient_start..coefficient_start + coefficient_count];
            T::fill_epsilon(m, lmax, &mut recurrence.epsilon);
            let phase = if m == 0 {
                Complex::new(T::one(), T::zero())
            } else {
                longitude_phase::<T>(m, phi0, true)
            };
            for block_start in (0..theta_sin.len()).step_by(LATITUDE_BLOCK_SIZE) {
                let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(theta_sin.len());
                let block_len = block_end - block_start;
                for (lane, weighted) in recurrence.weighted_spectrum[..block_len]
                    .iter_mut()
                    .enumerate()
                {
                    let ring = block_start + lane;
                    let spectrum_start = ring * spectrum_len;
                    *weighted = spectra[spectrum_start + m] * phase;
                    if let Some(ring_weights) = ring_weights {
                        let weight =
                            T::from_f64(ring_weights[ring]).expect("ring weight is representable");
                        *weighted *= weight / longitude_count;
                    }
                }
                let weighted_spectrum = &recurrence.weighted_spectrum[..block_len];
                T::for_each_y_lm_dispatched(
                    arch,
                    m,
                    lmax,
                    &theta_sin[block_start..block_end],
                    &theta_cos[block_start..block_end],
                    self.sectoral_norms[m],
                    &recurrence.epsilon,
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                    |l, ring_start, current, exponents| {
                        let coefficient = &mut coefficients[l - m];
                        for lane in 0..current.len() {
                            let ring = ring_start + lane;
                            if m > ring_order_limits[block_start + ring] {
                                continue;
                            }
                            *coefficient += T::multiply_complex_scaled(
                                weighted_spectrum[ring],
                                current[lane],
                                exponents[lane],
                            );
                        }
                        Ok(())
                    },
                )?;
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
        let recurrence_scratch_len = latitude_count.min(LATITUDE_BLOCK_SIZE);
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
        check_len(
            "workspace recurrence worker groups",
            expected_worker_count,
            workspace.recurrence_workers.len(),
        )?;
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

fn check_len(buffer: &'static str, expected: usize, actual: usize) -> Result<(), ShtError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ShtError::BufferLength {
            buffer,
            expected,
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use num_complex::Complex64;
    use std::f64::consts::{PI, TAU};

    use super::*;
    use crate::grid::{GridKind, GridSpec};
    use crate::harmonics::HarmonicSpec;

    fn make_plan(lmax: usize, mmax: usize, ntheta: usize, nphi: usize, phi0: f64) -> ShtPlan {
        ShtPlan::new(
            GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, phi0).unwrap(),
            HarmonicSpec::scalar(lmax, mmax).unwrap(),
        )
        .unwrap()
    }

    fn make_cc_plan(lmax: usize, mmax: usize, ntheta: usize, nphi: usize, phi0: f64) -> ShtPlan {
        ShtPlan::new(
            GridSpec::new(GridKind::ClenshawCurtis, ntheta, nphi, phi0).unwrap(),
            HarmonicSpec::scalar(lmax, mmax).unwrap(),
        )
        .unwrap()
    }

    fn make_f1_plan(lmax: usize, mmax: usize, ntheta: usize, nphi: usize, phi0: f64) -> ShtPlan {
        ShtPlan::new(
            GridSpec::new(GridKind::Fejer1, ntheta, nphi, phi0).unwrap(),
            HarmonicSpec::scalar(lmax, mmax).unwrap(),
        )
        .unwrap()
    }

    fn coefficients(spec: &HarmonicSpec) -> Vec<Complex64> {
        let mut alm = vec![Complex64::new(0.0, 0.0); spec.nalm()];
        for (l, m, value) in [
            (0, 0, Complex64::new(0.7, 0.0)),
            (1, 0, Complex64::new(-0.2, 0.0)),
            (2, 0, Complex64::new(0.1, 0.0)),
            (1, 1, Complex64::new(0.4, 0.3)),
            (2, 1, Complex64::new(-0.2, 0.15)),
            (2, 2, Complex64::new(0.1, -0.6)),
        ] {
            if let Some(index) = spec.index(l, m) {
                alm[index] = value;
            }
        }
        alm
    }

    fn analytic_y(theta: f64, phi: f64, l: usize, m: usize) -> Complex64 {
        let (sin_theta, cos_theta) = theta.sin_cos();
        let amplitude = match (l, m) {
            (0, 0) => 1.0 / (4.0 * PI).sqrt(),
            (1, 0) => (3.0 / (4.0 * PI)).sqrt() * cos_theta,
            (2, 0) => (5.0 / (16.0 * PI)).sqrt() * (3.0 * cos_theta * cos_theta - 1.0),
            (1, 1) => -(3.0 / (8.0 * PI)).sqrt() * sin_theta,
            (2, 1) => -(15.0 / (8.0 * PI)).sqrt() * sin_theta * cos_theta,
            (2, 2) => (15.0 / (32.0 * PI)).sqrt() * sin_theta * sin_theta,
            _ => 0.0,
        };
        Complex64::from_polar(amplitude, m as f64 * phi)
    }

    fn analytic_map(plan: &ShtPlan, alm: &[Complex64]) -> Vec<f64> {
        let mut map = vec![0.0; plan.sample_count];
        for (ring, &theta) in plan.grid.theta().iter().enumerate() {
            for j in 0..plan.grid.nphi() {
                let phi = plan.grid.longitude(j).unwrap();
                let mut value = 0.0;
                for m in 0..=plan.harmonics.mmax() {
                    for l in m..=2.min(plan.harmonics.lmax()) {
                        if let Some(index) = plan.harmonics.index(l, m) {
                            let mode = alm[index] * analytic_y(theta, phi, l, m);
                            value += if m == 0 { mode.re } else { 2.0 * mode.re };
                        }
                    }
                }
                map[ring * plan.grid.nphi() + j] = value;
            }
        }
        map
    }

    fn arbitrary_map(plan: &ShtPlan) -> Vec<f64> {
        let mut map = Vec::with_capacity(plan.sample_count);
        for i in 0..plan.grid.ntheta() {
            for j in 0..plan.grid.nphi() {
                map.push(
                    (0.17 * i as f64 + 0.23 * j as f64).sin()
                        + 0.2 * (0.11 * i as f64 - 0.31 * j as f64).cos(),
                );
            }
        }
        map
    }

    fn map_inner_product(left: &[f64], right: &[f64]) -> f64 {
        left.iter().zip(right).map(|(x, y)| x * y).sum()
    }

    fn harmonic_inner_product(spec: &HarmonicSpec, left: &[Complex64], right: &[Complex64]) -> f64 {
        let mut result = 0.0;
        for m in 0..=spec.mmax() {
            let weight = if m == 0 { 1.0 } else { 2.0 };
            for l in m..=spec.lmax() {
                let index = spec.index(l, m).unwrap();
                result += weight * (left[index].conj() * right[index]).re;
            }
        }
        result
    }

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance * expected.abs().max(1.0),
            "actual={actual:.16e}, expected={expected:.16e}"
        );
    }

    fn assert_complex_close(actual: Complex64, expected: Complex64, tolerance: f64) {
        assert!(
            (actual - expected).norm() <= tolerance,
            "actual={actual:?}, expected={expected:?}, error={:.3e}",
            (actual - expected).norm()
        );
    }

    fn assert_reference_close(actual: f64, expected: f64) {
        if expected == 0.0 {
            assert_eq!(actual, 0.0, "expected exact zero, got {actual:.16e}");
        } else {
            let relative_error = (actual - expected).abs() / expected.abs();
            assert!(
                relative_error <= 1e-12,
                "actual={actual:.16e}, expected={expected:.16e}, relative error={relative_error:.3e}"
            );
        }
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn weighted_order_midpoint_minimizes_coefficient_work() {
        for (lmax, m_start, m_end) in [
            (511, 0, 512),
            (511, 0, 128),
            (511, 300, 512),
            (20, 4, 11),
            (2, 0, 3),
            (511, 0, 2),
        ] {
            let midpoint = weighted_order_midpoint(lmax, m_start, m_end);
            assert!(m_start < midpoint && midpoint < m_end);
            let work = |start: usize, end: usize| (start..end).map(|m| lmax - m + 1).sum::<usize>();
            let difference = work(m_start, midpoint).abs_diff(work(midpoint, m_end));
            for candidate in (m_start + 1)..m_end {
                let candidate_difference =
                    work(m_start, candidate).abs_diff(work(candidate, m_end));
                assert!(
                    difference <= candidate_difference,
                    "lmax={lmax}, m_start={m_start}, m_end={m_end}, midpoint={midpoint}, candidate={candidate}"
                );
            }
        }
    }

    #[test]
    fn scalar_synthesis_matches_independent_low_degree_harmonics() {
        let plan = make_plan(3, 2, 5, 9, 0.23);
        let alm = coefficients(plan.harmonics());
        let expected = analytic_map(&plan, &alm);
        let mut map = vec![0.0; plan.sample_count];
        let mut workspace = plan.workspace().unwrap();

        plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();
        for (&actual, &reference) in map.iter().zip(&expected) {
            assert_close(actual, reference, 3e-14);
        }
    }

    #[test]
    fn scalar_adjoint_operators_match_ducc_and_inner_products() {
        // Reference: ducc0 0.41.0, GL 3x7, phi0=0.37, mstart for packed Rust layout.
        let plan = make_plan(2, 2, 3, 7, 0.37);
        let alm = coefficients(plan.harmonics());
        let input = arbitrary_map(&plan);
        let mut workspace = plan.workspace().unwrap();

        let mut synthesized = vec![0.0; plan.sample_count];
        plan.synthesis_into(&alm, &mut synthesized, &mut workspace)
            .unwrap();
        let mut synthesis_adjoint = vec![Complex64::new(0.0, 0.0); alm.len()];
        plan.adjoint_synthesis_into(&input, &mut synthesis_adjoint, &mut workspace)
            .unwrap();
        for (&actual, expected) in synthesis_adjoint.iter().zip([
            Complex64::new(4.648_960_499_291_433, 0.0),
            Complex64::new(-0.594_467_893_478_704_9, 0.0),
            Complex64::new(1.011_778_214_432_334_2, 0.0),
            Complex64::new(0.378_992_673_732_543_3, -0.607_778_178_500_533),
            Complex64::new(-0.033_014_359_575_363_2, -0.149_134_620_282_385_1),
            Complex64::new(-0.105_833_761_840_617_6, 0.290_989_521_058_551_9),
        ]) {
            assert!((actual - expected).norm() < 2e-14);
        }
        assert_close(
            map_inner_product(&synthesized, &input),
            harmonic_inner_product(plan.harmonics(), &alm, &synthesis_adjoint),
            2e-13,
        );

        let mut analyzed = vec![Complex64::new(0.0, 0.0); alm.len()];
        plan.analysis_into(&input, &mut analyzed, &mut workspace)
            .unwrap();
        let mut analysis_adjoint = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace)
            .unwrap();
        for (index, expected) in [
            (0, 0.180_256_485_957_303_8),
            (1, 0.226_281_635_669_034_06),
            (3, 0.095_259_119_859_373_66),
            (7, 0.281_486_856_056_618_55),
            (10, 0.368_079_725_958_397_8),
            (20, -0.068_996_291_908_175_74),
        ] {
            assert_close(analysis_adjoint[index], expected, 2e-14);
        }
        assert_close(
            harmonic_inner_product(plan.harmonics(), &analyzed, &alm),
            map_inner_product(&input, &analysis_adjoint),
            2e-13,
        );
    }

    #[test]
    fn scalar_synthesis_adjoint_accepts_even_longitude_nyquist_order() {
        // Reference: ducc0 0.41.0, GL 3x4, phi0=0.31, one thread.
        let plan = make_plan(2, 2, 3, 4, 0.31);
        let alm = coefficients(plan.harmonics());
        let input = arbitrary_map(&plan);
        let mut workspace = plan.workspace().unwrap();
        let mut synthesized = vec![0.0; plan.sample_count];
        plan.synthesis_into(&alm, &mut synthesized, &mut workspace)
            .unwrap();
        let mut adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.adjoint_synthesis_into(&input, &mut adjoint, &mut workspace)
            .unwrap();

        for (&actual, expected) in adjoint.iter().zip([
            Complex64::new(2.192_070_200_391_801, 0.0),
            Complex64::new(-0.452_936_406_865_718_34, 0.0),
            Complex64::new(0.478_150_955_162_161_2, 0.0),
            Complex64::new(0.219_369_995_115_942_18, -0.321_778_433_740_079_75),
            Complex64::new(0.004_395_968_203_849_482_5, -0.028_237_897_221_759_366),
            Complex64::new(-0.195_608_850_028_266_7, 0.139_646_919_817_436_15),
        ]) {
            assert!((actual - expected).norm() < 3e-14);
        }
        assert!(adjoint[plan.harmonics().index(2, 2).unwrap()].im.abs() > 0.1);
        assert_close(
            map_inner_product(&synthesized, &input),
            harmonic_inner_product(plan.harmonics(), &alm, &adjoint),
            2e-13,
        );
    }

    #[test]
    #[expect(
        clippy::excessive_precision,
        reason = "The literals retain DUCC's printed reference digits."
    )]
    fn scalar_synthesis_matches_ducc_0410_reference() {
        // Reference: ducc0 0.41.0, GL, ntheta=3, nphi=5, phi0=0.37, nthreads=1.
        let plan = make_plan(2, 2, 3, 5, 0.37);
        let alm = coefficients(plan.harmonics());
        let expected = [
            3.61477755632444442e-1,
            3.41202806618260746e-1,
            1.67425809619942578e-1,
            1.10137639969044035e-1,
            -2.45225492833134107e-1,
            3.52800193384860350e-1,
            2.59893066238583326e-1,
            3.24314581387524226e-1,
            5.31628824359529850e-1,
            -6.39000676788184396e-1,
            1.48463124212611458e-1,
            2.82761224552306900e-1,
            5.53534560205396930e-1,
            6.16420556489261751e-1,
            -1.09221189846971128e-1,
        ];
        let mut map = vec![0.0; plan.sample_count];
        let mut workspace = plan.workspace().unwrap();

        plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();
        for (&actual, &reference) in map.iter().zip(&expected) {
            assert_close(actual, reference, 4e-14);
        }
    }

    #[test]
    #[expect(
        clippy::excessive_precision,
        reason = "The literals retain DUCC's printed reference digits."
    )]
    fn t255_sectoral_synthesis_matches_ducc_0410_samples() {
        // Reference: ducc0 0.41.0, GL, T255, phi0=0.37, nthreads=1.
        let plan = make_plan(255, 255, 256, 513, 0.37);
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        alm[plan.harmonics().index(255, 255).unwrap()] = Complex64::new(1.0, 0.0);
        let mut map = vec![0.0; plan.sample_count];
        let mut workspace = plan.workspace().unwrap();

        plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();
        for (ring, longitude, reference) in [
            (0, 0, 0.0),
            (63, 1, 3.06179051888323780e-39),
            (127, 0, -2.37277732245063966),
            (127, 1, 2.37684826227152968),
            (128, 256, 2.65180186641488569e-1),
            (191, 512, 6.86455637290427111e-38),
            (255, 0, 0.0),
        ] {
            assert_reference_close(map[ring * 513 + longitude], reference);
        }
    }

    #[test]
    #[expect(
        clippy::excessive_precision,
        reason = "The literals retain DUCC's printed reference digits."
    )]
    fn t255_mixed_synthesis_matches_ducc_0410_samples() {
        // Reference: ducc0 0.41.0, GL, T255, phi0=0.37, nthreads=1.
        let plan = make_plan(255, 255, 256, 513, 0.37);
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        for m in 0..=255 {
            for l in m..=255 {
                let re = ((l * 7 + m * 11) % 19) as f64 - 9.0;
                let im = if m == 0 {
                    0.0
                } else {
                    ((l * 13 + m * 5) % 23) as f64 - 11.0
                };
                let index = plan.harmonics().index(l, m).unwrap();
                alm[index] = Complex64::new(re * 1e-5, im * 1e-5);
            }
        }
        let mut map = vec![0.0; plan.sample_count];
        let mut workspace = plan.workspace().unwrap();

        plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();
        for (ring, longitude, reference) in [
            (0, 0, -7.73911562867147081e-5),
            (63, 1, 7.59821699150580896e-4),
            (127, 0, -4.01554944939784119e-4),
            (127, 1, 4.51924072746787359e-4),
            (128, 256, -2.10693373424372839e-3),
            (191, 512, -2.32708782503306599e-4),
            (217, 283, -5.04792555199083300e-2),
            (255, 512, -8.23852167514639881e-4),
        ] {
            assert_close(map[ring * 513 + longitude], reference, 2e-14);
        }
    }

    #[test]
    fn scalar_analysis_recovers_closed_form_fields_at_minimum_sampling() {
        for nphi in [5, 6] {
            let plan = make_plan(3, 2, 4, nphi, 0.47);
            let expected = coefficients(plan.harmonics());
            let map = analytic_map(&plan, &expected);
            let mut actual = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let mut workspace = plan.workspace().unwrap();

            plan.analysis_into(&map, &mut actual, &mut workspace)
                .unwrap();
            for (actual, expected) in actual.iter().zip(&expected) {
                assert!((*actual - *expected).norm() < 2e-13);
            }
        }
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn rayon_groups_match_one_thread_for_synthesis_and_analysis() {
        let plan = make_plan(3, 2, 131, 9, 0.41);
        let input_alm = coefficients(plan.harmonics());
        let expected_map = analytic_map(&plan, &input_alm);
        let one_thread = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let two_threads = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let mut one_thread_workspace = one_thread.install(|| plan.workspace()).unwrap();
        let mut two_thread_workspace = two_threads.install(|| plan.workspace()).unwrap();
        let mut one_thread_map = vec![0.0; plan.sample_count];
        let mut two_thread_map = vec![0.0; plan.sample_count];

        one_thread
            .install(|| {
                plan.synthesis_into(&input_alm, &mut one_thread_map, &mut one_thread_workspace)
            })
            .unwrap();
        two_threads
            .install(|| {
                plan.synthesis_into(&input_alm, &mut two_thread_map, &mut two_thread_workspace)
            })
            .unwrap();
        for (&one_thread, &two_threads) in one_thread_map.iter().zip(&two_thread_map) {
            assert_close(two_threads, one_thread, 2e-14);
        }
        for (&actual, &expected) in two_thread_map.iter().zip(&expected_map) {
            assert_close(actual, expected, 3e-14);
        }

        let mut one_thread_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut two_thread_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        one_thread
            .install(|| {
                plan.analysis_into(
                    &expected_map,
                    &mut one_thread_alm,
                    &mut one_thread_workspace,
                )
            })
            .unwrap();
        two_threads
            .install(|| {
                plan.analysis_into(
                    &expected_map,
                    &mut two_thread_alm,
                    &mut two_thread_workspace,
                )
            })
            .unwrap();
        for (&one_thread, &two_threads) in one_thread_alm.iter().zip(&two_thread_alm) {
            assert!((one_thread - two_threads).norm() < 2e-14);
        }
        for (&actual, &expected) in two_thread_alm.iter().zip(&input_alm) {
            assert!((actual - expected).norm() < 2e-13);
        }

        let input_map = arbitrary_map(&plan);
        let mut one_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut two_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        one_thread
            .install(|| {
                plan.adjoint_synthesis_into(&input_map, &mut one_adjoint, &mut one_thread_workspace)
            })
            .unwrap();
        two_threads
            .install(|| {
                plan.adjoint_synthesis_into(&input_map, &mut two_adjoint, &mut two_thread_workspace)
            })
            .unwrap();
        for (&one, &two) in one_adjoint.iter().zip(&two_adjoint) {
            assert!((one - two).norm() < 2e-14);
        }

        let mut one_analysis_adjoint = vec![0.0; plan.sample_count];
        let mut two_analysis_adjoint = vec![0.0; plan.sample_count];
        one_thread
            .install(|| {
                plan.adjoint_analysis_into(
                    &input_alm,
                    &mut one_analysis_adjoint,
                    &mut one_thread_workspace,
                )
            })
            .unwrap();
        two_threads
            .install(|| {
                plan.adjoint_analysis_into(
                    &input_alm,
                    &mut two_analysis_adjoint,
                    &mut two_thread_workspace,
                )
            })
            .unwrap();
        for (&one, &two) in one_analysis_adjoint.iter().zip(&two_analysis_adjoint) {
            assert_close(two, one, 2e-14);
        }

        let mut one_derivative_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut two_derivative_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        one_thread
            .install(|| {
                plan.adjoint_synthesis_deriv1_into(
                    &input_map,
                    &expected_map,
                    &mut one_derivative_adjoint,
                    &mut one_thread_workspace,
                )
            })
            .unwrap();
        two_threads
            .install(|| {
                plan.adjoint_synthesis_deriv1_into(
                    &input_map,
                    &expected_map,
                    &mut two_derivative_adjoint,
                    &mut two_thread_workspace,
                )
            })
            .unwrap();
        for (&one, &two) in one_derivative_adjoint.iter().zip(&two_derivative_adjoint) {
            assert!((one - two).norm() < 2e-14);
        }
    }

    #[test]
    #[expect(
        clippy::excessive_precision,
        reason = "The literals retain DUCC's printed reference digits."
    )]
    fn scalar_analysis_matches_ducc_0410_reference() {
        // Reference: ducc0 0.41.0, GL, ntheta=3, nphi=5, phi0=0.37, nthreads=1.
        let plan = make_plan(2, 2, 3, 5, 0.37);
        let map: Vec<_> = (0..3)
            .flat_map(|i| {
                (0..5).map(move |j| {
                    let i = i as f64;
                    let j = j as f64;
                    (0.27 * i + 0.41 * j).sin() + 0.1 * i * j
                })
            })
            .collect();
        let expected = [
            Complex64::new(3.29516125962815387e0, 0.0),
            Complex64::new(-8.02149484639918087e-1, 0.0),
            Complex64::new(-4.75137285362010608e-2, 0.0),
            Complex64::new(4.74880729100706500e-1, -7.04706284735987509e-1),
            Complex64::new(-3.49490137183501304e-2, 1.41321010292757131e-2),
            Complex64::new(-2.49437516790312230e-1, 3.67573142006427211e-1),
        ];
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut workspace = plan.workspace().unwrap();

        plan.analysis_into(&map, &mut alm, &mut workspace).unwrap();
        for (&actual, &reference) in alm.iter().zip(&expected) {
            assert!((actual - reference).norm() < 4e-14);
        }
    }

    #[test]
    #[expect(
        clippy::excessive_precision,
        reason = "The literals retain DUCC's printed reference digits."
    )]
    fn t255_analysis_matches_ducc_0410_coefficients() {
        // Reference: ducc0 0.41.0, GL, T255, phi0=0.37, nthreads=1.
        let plan = make_plan(255, 255, 256, 513, 0.37);
        let map: Vec<_> = (0..256)
            .flat_map(|i| {
                (0..513).map(move |j| {
                    let i = i as f64;
                    let j = j as f64;
                    (0.019 * i + 0.037 * j).sin() + 0.3 * (0.071 * i - 0.013 * j).cos()
                })
            })
            .collect();
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut workspace = plan.workspace().unwrap();

        plan.analysis_into(&map, &mut alm, &mut workspace).unwrap();
        for (l, m, reference) in [
            (0, 0, Complex64::new(6.76004192896470860e-3, 0.0)),
            (1, 0, Complex64::new(1.29270045097557157e-2, 0.0)),
            (255, 0, Complex64::new(-5.40717096686140754e-9, 0.0)),
            (
                128,
                64,
                Complex64::new(-3.85208519952169484e-5, -9.74532652404812478e-6),
            ),
            (
                200,
                128,
                Complex64::new(2.69942915754136812e-7, -3.89128455741458547e-7),
            ),
            (
                255,
                254,
                Complex64::new(-6.43558637760604195e-5, -1.66103146151738605e-5),
            ),
            (
                255,
                255,
                Complex64::new(-5.16216786170599655e-5, 5.78457625404296755e-6),
            ),
        ] {
            let index = plan.harmonics().index(l, m).unwrap();
            assert!(
                (alm[index] - reference).norm() < 4e-13,
                "(l={l}, m={m}): actual={:?}, expected={reference:?}",
                alm[index]
            );
        }

        let mut adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.adjoint_synthesis_into(&map, &mut adjoint, &mut workspace)
            .unwrap();
        let mut synthesized = vec![0.0; plan.sample_count];
        plan.synthesis_into(&alm, &mut synthesized, &mut workspace)
            .unwrap();
        assert_close(
            map_inner_product(&synthesized, &map),
            harmonic_inner_product(plan.harmonics(), &alm, &adjoint),
            3e-10,
        );

        let mut analysis_adjoint = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace)
            .unwrap();
        assert_close(
            harmonic_inner_product(plan.harmonics(), &alm, &alm),
            map_inner_product(&map, &analysis_adjoint),
            3e-10,
        );

        let second_component: Vec<_> = (0..256)
            .flat_map(|i| {
                (0..513).map(move |j| {
                    let i = i as f64;
                    let j = j as f64;
                    (0.19 * i - 0.13 * j).cos() - 0.15 * (0.29 * i + 0.07 * j).sin()
                })
            })
            .collect();
        let mut derivative_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.adjoint_synthesis_deriv1_into(
            &map,
            &second_component,
            &mut derivative_adjoint,
            &mut workspace,
        )
        .unwrap();
        let mut derivative_theta = vec![0.0; plan.sample_count];
        let mut derivative_phi = vec![0.0; plan.sample_count];
        plan.synthesis_deriv1_into(
            &alm,
            &mut derivative_theta,
            &mut derivative_phi,
            &mut workspace,
        )
        .unwrap();
        assert_close(
            map_inner_product(&derivative_theta, &map)
                + map_inner_product(&derivative_phi, &second_component),
            harmonic_inner_product(plan.harmonics(), &alm, &derivative_adjoint),
            3e-10,
        );
    }

    #[test]
    fn synthesis_combines_the_even_longitude_nyquist_mode() {
        let plan = make_plan(2, 2, 4, 4, 0.31);
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        alm[plan.harmonics().index(2, 2).unwrap()] = Complex64::new(0.6, -0.4);
        let expected = analytic_map(&plan, &alm);
        let mut map = vec![0.0; plan.sample_count];
        let mut workspace = plan.workspace().unwrap();

        plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();
        for (&actual, &reference) in map.iter().zip(&expected) {
            assert_close(actual, reference, 3e-14);
        }
    }

    #[test]
    fn operations_reject_undersampling_buffers_and_unimplemented_spin() {
        let too_few_rings = make_plan(3, 1, 3, 5, 0.0);
        let mut workspace = too_few_rings.workspace().unwrap();
        let map = vec![0.0; too_few_rings.sample_count];
        let mut alm = vec![Complex64::new(0.0, 0.0); too_few_rings.harmonics().nalm()];
        assert_eq!(
            too_few_rings.analysis_into(&map, &mut alm, &mut workspace),
            Err(ShtError::AnalysisLatitudeUndersampled { lmax: 3, ntheta: 3 })
        );
        let mut adjoint_map = vec![0.0; too_few_rings.sample_count];
        assert_eq!(
            too_few_rings.adjoint_analysis_into(&alm, &mut adjoint_map, &mut workspace),
            Err(ShtError::AnalysisLatitudeUndersampled { lmax: 3, ntheta: 3 })
        );

        let nyquist_analysis = make_plan(2, 2, 3, 4, 0.0);
        let mut nyquist_workspace = nyquist_analysis.workspace().unwrap();
        let nyquist_map = vec![0.0; nyquist_analysis.sample_count];
        let mut nyquist_alm = vec![Complex64::new(0.0, 0.0); nyquist_analysis.harmonics().nalm()];
        assert_eq!(
            nyquist_analysis.analysis_into(&nyquist_map, &mut nyquist_alm, &mut nyquist_workspace),
            Err(ShtError::AnalysisLongitudeUndersampled { mmax: 2, nphi: 4 })
        );
        let mut nyquist_adjoint_map = vec![0.0; nyquist_analysis.sample_count];
        assert_eq!(
            nyquist_analysis.adjoint_analysis_into(
                &nyquist_alm,
                &mut nyquist_adjoint_map,
                &mut nyquist_workspace
            ),
            Err(ShtError::AnalysisLongitudeUndersampled { mmax: 2, nphi: 4 })
        );

        let too_many_orders = make_plan(3, 3, 4, 4, 0.0);
        let mut order_workspace = too_many_orders.workspace().unwrap();
        let order_alm = vec![Complex64::new(0.0, 0.0); too_many_orders.harmonics().nalm()];
        let mut order_map = vec![0.0; too_many_orders.sample_count];
        assert_eq!(
            too_many_orders.synthesis_into(&order_alm, &mut order_map, &mut order_workspace),
            Err(ShtError::SynthesisLongitudeUndersampled { mmax: 3, nphi: 4 })
        );

        assert_eq!(
            nyquist_analysis.synthesis_into(
                &nyquist_alm,
                &mut order_map[..3],
                &mut nyquist_workspace
            ),
            Err(ShtError::BufferLength {
                buffer: "map output",
                expected: nyquist_analysis.sample_count,
                actual: 3
            })
        );

        let spin_plan = ShtPlan::new(
            GridSpec::new(GridKind::GaussLegendre, 3, 5, 0.0).unwrap(),
            HarmonicSpec::new(2, 1, 1).unwrap(),
        )
        .unwrap();
        let mut spin_workspace = spin_plan.workspace().unwrap();
        let spin_alm = vec![Complex64::new(0.0, 0.0); spin_plan.harmonics().nalm()];
        let mut spin_map = vec![0.0; spin_plan.sample_count];
        assert_eq!(
            spin_plan.synthesis_into(&spin_alm, &mut spin_map, &mut spin_workspace),
            Err(ShtError::UnsupportedSpin { spin: 1 })
        );
    }

    #[test]
    #[expect(
        clippy::excessive_precision,
        reason = "The fixture values retain printed ducc0 0.41.0 outputs."
    )]
    fn clenshaw_curtis_scalar_operations_match_ducc_and_adjoint_identities() {
        // DUCC reference: ducc0==0.41.0, CC 4x5, phi0=0.37, one thread.
        let plan = make_cc_plan(2, 2, 4, 5, 0.37);
        let alm = coefficients(plan.harmonics());
        let map = arbitrary_map(&plan);
        let mut workspace = plan.workspace().unwrap();

        let mut synthesized = vec![0.0; plan.sample_count];
        plan.synthesis_into(&alm, &mut synthesized, &mut workspace)
            .unwrap();
        let analytic = analytic_map(&plan, &alm);
        for (&actual, &expected) in synthesized.iter().zip(&analytic) {
            assert_close(actual, expected, 5e-14);
        }
        for (index, expected) in [
            (0, 0.162_824_164_911_634_76),
            (1, 0.162_824_164_911_634_76),
            (2, 0.162_824_164_911_634_76),
            (3, 0.162_824_164_911_634_76),
            (4, 0.162_824_164_911_634_76),
            (5, 0.420_719_530_114_416_26),
            (10, 0.196_350_610_822_396_35),
            (15, 0.358_265_169_672_802_75),
            (19, 0.358_265_169_672_802_75),
        ] {
            assert_close(synthesized[index], expected, 3e-14);
        }

        let mut analyzed = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.analysis_into(&map, &mut analyzed, &mut workspace)
            .unwrap();
        let expected_analysis = [
            Complex64::new(2.763_651_652_288_861_4, 0.0),
            Complex64::new(-0.327_153_874_969_685_84, 0.0),
            Complex64::new(-0.027_926_626_089_303_408, 0.0),
            Complex64::new(0.199_456_242_909_442_36, -0.367_684_272_467_260_32),
            Complex64::new(-0.000_170_607_730_796_683_42, -0.025_218_840_386_616_675),
            Complex64::new(-0.129_749_726_124_326_17, 0.200_921_712_854_862_38),
        ];
        for (&actual, expected) in analyzed.iter().zip(expected_analysis) {
            assert_complex_close(actual, expected, 5e-13);
        }

        let mut synthesis_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.adjoint_synthesis_into(&map, &mut synthesis_adjoint, &mut workspace)
            .unwrap();
        let expected_synthesis_adjoint = [
            Complex64::new(4.355_007_000_427_095, 0.0),
            Complex64::new(-1.105_228_186_479_379_1, 0.0),
            Complex64::new(4.190_099_197_329_241, 0.0),
            Complex64::new(0.178_562_514_316_362_64, -0.329_168_078_214_253_7),
            Complex64::new(-0.000_190_919_979_042_051_46, -0.028_221_349_968_109_447),
            Complex64::new(-0.115_918_089_919_755_52, 0.179_490_470_259_712_76),
        ];
        for (&actual, expected) in synthesis_adjoint.iter().zip(expected_synthesis_adjoint) {
            assert_complex_close(actual, expected, 5e-13);
        }

        let mut analysis_adjoint = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace)
            .unwrap();
        for (index, expected) in [
            (0, 0.001_918_749_569_064_584_5),
            (1, 0.029_734_322_064_137_043),
            (2, 0.031_597_426_064_432_131),
            (3, 0.000_767_287_972_306_324_17),
            (4, 0.048_788_395_282_923_627),
            (5, 0.460_244_027_175_822_19),
            (10, 0.259_746_036_479_518_88),
            (15, 0.040_122_946_665_733_537),
            (19, 0.086_992_592_379_592_532),
        ] {
            assert_close(analysis_adjoint[index], expected, 3e-13);
        }
        assert_close(
            map_inner_product(&synthesized, &map),
            harmonic_inner_product(plan.harmonics(), &alm, &synthesis_adjoint),
            5e-13,
        );
        assert_close(
            harmonic_inner_product(plan.harmonics(), &analyzed, &alm),
            map_inner_product(&map, &analysis_adjoint),
            5e-13,
        );
    }

    #[test]
    #[allow(clippy::excessive_precision)]
    fn clenshaw_curtis_deriv1_and_adjoint_match_ducc() {
        // DUCC references: ducc0==0.41.0, CC 4x5, phi0=0.37, one thread.
        let plan = make_cc_plan(2, 2, 4, 5, 0.37);
        let alm = coefficients(plan.harmonics());
        let mut theta_derivative = vec![0.0; plan.sample_count];
        let mut phi_derivative = vec![0.0; plan.sample_count];
        let mut workspace = plan.workspace().unwrap();

        plan.synthesis_deriv1_into(
            &alm,
            &mut theta_derivative,
            &mut phi_derivative,
            &mut workspace,
        )
        .unwrap();
        for (index, expected_theta, expected_phi) in [
            (0, 0.189_187_523_633_865_48, 0.397_551_229_793_163_65),
            (5, 0.045_449_220_099_812_365, 0.848_041_919_092_327_8),
            (10, -0.248_109_173_647_102_92, 0.743_707_688_160_535_6),
            (19, 0.351_033_268_398_765_7, -0.469_131_834_353_264_5),
        ] {
            assert_close(theta_derivative[index], expected_theta, 8e-14);
            assert_close(phi_derivative[index], expected_phi, 8e-14);
        }

        let mut first_input = vec![0.0; plan.sample_count];
        let mut second_input = vec![0.0; plan.sample_count];
        for i in 0..plan.grid.ntheta() {
            for j in 0..plan.grid.nphi() {
                let index = i * plan.grid.nphi() + j;
                first_input[index] = (0.17 * i as f64 + 0.23 * j as f64).sin()
                    + 0.2 * (0.11 * i as f64 - 0.31 * j as f64).cos();
                second_input[index] = (0.19 * i as f64 - 0.13 * j as f64).cos()
                    - 0.15 * (0.29 * i as f64 + 0.07 * j as f64).sin();
            }
        }
        let mut adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.adjoint_synthesis_deriv1_into(
            &first_input,
            &second_input,
            &mut adjoint,
            &mut workspace,
        )
        .unwrap();
        let expected_adjoint = [
            Complex64::new(0.0, 0.0),
            Complex64::new(-3.308_183_563_850_672, 0.0),
            Complex64::new(0.533_803_701_067_148_1, 0.0),
            Complex64::new(0.045_294_896_873_528_77, -0.155_975_012_277_048_55),
            Complex64::new(0.505_392_537_836_259_8, -0.323_413_117_328_060_7),
            Complex64::new(-0.028_628_937_979_647_68, 0.013_394_692_285_793_142),
        ];
        for (&actual, expected) in adjoint.iter().zip(expected_adjoint) {
            assert_complex_close(actual, expected, 8e-13);
        }
        assert_close(
            map_inner_product(&theta_derivative, &first_input)
                + map_inner_product(&phi_derivative, &second_input),
            harmonic_inner_product(plan.harmonics(), &alm, &adjoint),
            2e-12,
        );
    }

    #[test]
    #[allow(clippy::excessive_precision)]
    fn fejer1_scalar_operations_match_ducc_and_adjoint_identities() {
        // DUCC references: ducc0==0.41.0, F1 3x5, phi0=0.37, one thread.
        let plan = make_f1_plan(2, 2, 3, 5, 0.37);
        let alm = coefficients(plan.harmonics());
        let map = arbitrary_map(&plan);
        let mut workspace = plan.workspace().unwrap();
        let mut synthesized = vec![0.0; plan.sample_count];
        plan.synthesis_into(&alm, &mut synthesized, &mut workspace)
            .unwrap();
        for (&actual, &expected) in synthesized.iter().zip(analytic_map(&plan, &alm).iter()) {
            assert_close(actual, expected, 5e-14);
        }
        for (index, expected) in [
            (0, 0.314_342_495_875_079_34),
            (3, 0.080_714_992_574_932_31),
            (7, 0.324_314_581_387_524_2),
            (14, 0.029_654_993_002_577_9),
        ] {
            assert_close(synthesized[index], expected, 5e-14);
        }

        let mut analyzed = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.analysis_into(&map, &mut analyzed, &mut workspace)
            .unwrap();
        let expected_analysis = [
            Complex64::new(2.526_335_455_435_708_7, 0.0),
            Complex64::new(-0.329_194_184_006_445_57, 0.0),
            Complex64::new(-0.012_691_648_946_562_495, 0.0),
            Complex64::new(0.217_297_712_402_338_9, -0.431_291_149_674_174_2),
            Complex64::new(-0.002_838_643_247_449_916, -0.044_675_978_762_751_395),
            Complex64::new(-0.135_929_960_056_900_4, 0.213_978_455_917_123_7),
        ];
        for (&actual, expected) in analyzed.iter().zip(expected_analysis) {
            assert_complex_close(actual, expected, 8e-13);
        }

        let mut synthesis_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.adjoint_synthesis_into(&map, &mut synthesis_adjoint, &mut workspace)
            .unwrap();
        let expected_synthesis_adjoint = [
            Complex64::new(3.007_121_966_993_341, 0.0),
            Complex64::new(-0.589_419_918_244_447_6, 0.0),
            Complex64::new(1.659_728_243_091_294, 0.0),
            Complex64::new(0.204_545_838_622_175_46, -0.405_907_874_492_659_64),
            Complex64::new(-0.003_176_606_984_808_496_3, -0.049_995_020_092_221_82),
            Complex64::new(-0.121_563_609_841_810_32, 0.191_357_035_312_117_95),
        ];
        for (&actual, expected) in synthesis_adjoint.iter().zip(expected_synthesis_adjoint) {
            assert_complex_close(actual, expected, 8e-13);
        }
        assert_close(
            map_inner_product(&synthesized, &map),
            harmonic_inner_product(plan.harmonics(), &alm, &synthesis_adjoint),
            2e-12,
        );

        let mut analysis_adjoint = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace)
            .unwrap();
        for (index, expected) in [
            (0, 0.221_949_056_029_639_9),
            (3, 0.013_177_515_252_603_91),
            (7, 0.442_143_828_934_253_97),
            (14, -0.089_168_661_536_922_03),
        ] {
            assert_close(analysis_adjoint[index], expected, 8e-13);
        }
        assert_close(
            harmonic_inner_product(plan.harmonics(), &analyzed, &alm),
            map_inner_product(&map, &analysis_adjoint),
            2e-12,
        );

        let mut first_input = map.clone();
        let mut second_input = vec![0.0; plan.sample_count];
        for i in 0..plan.grid.ntheta() {
            for j in 0..plan.grid.nphi() {
                let index = i * plan.grid.nphi() + j;
                first_input[index] = (0.17 * i as f64 + 0.23 * j as f64).sin()
                    + 0.2 * (0.11 * i as f64 - 0.31 * j as f64).cos();
                second_input[index] = (0.19 * i as f64 - 0.13 * j as f64).cos()
                    - 0.15 * (0.29 * i as f64 + 0.07 * j as f64).sin();
            }
        }
        let mut theta_derivative = vec![0.0; plan.sample_count];
        let mut phi_derivative = vec![0.0; plan.sample_count];
        plan.synthesis_deriv1_into(
            &alm,
            &mut theta_derivative,
            &mut phi_derivative,
            &mut workspace,
        )
        .unwrap();
        for (index, expected_theta, expected_phi) in [
            (0, 0.314_714_294_731_826_5, 0.673_782_712_223_989_2),
            (3, 0.102_519_439_228_165_21, -0.490_236_302_038_752_24),
            (7, 0.337_281_414_970_432_6, 0.752_572_816_780_536_8),
            (14, 0.835_739_806_187_284_8, -0.434_961_404_658_846_7),
        ] {
            assert_close(theta_derivative[index], expected_theta, 8e-14);
            assert_close(phi_derivative[index], expected_phi, 8e-14);
        }
        let mut derivative_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.adjoint_synthesis_deriv1_into(
            &first_input,
            &second_input,
            &mut derivative_adjoint,
            &mut workspace,
        )
        .unwrap();
        let expected_derivative_adjoint = [
            Complex64::new(0.0, 0.0),
            Complex64::new(-3.479_659_593_771_595, 0.0),
            Complex64::new(1.141_406_763_641_841_2, 0.0),
            Complex64::new(0.087_222_449_069_814_26, -0.071_936_864_630_359_4),
            Complex64::new(0.137_370_444_355_236_3, 0.044_720_398_234_314_444),
            Complex64::new(-0.081_778_044_950_258_63, -0.000_976_861_073_657_549_6),
        ];
        for (&actual, expected) in derivative_adjoint.iter().zip(expected_derivative_adjoint) {
            assert_complex_close(actual, expected, 8e-13);
        }
        assert_close(
            map_inner_product(&theta_derivative, &first_input)
                + map_inner_product(&phi_derivative, &second_input),
            harmonic_inner_product(plan.harmonics(), &alm, &derivative_adjoint),
            2e-12,
        );

        let undersampled = make_f1_plan(2, 2, 2, 5, 0.37);
        let mut undersampled_alm = vec![Complex64::new(0.0, 0.0); undersampled.harmonics().nalm()];
        let mut undersampled_workspace = undersampled.workspace().unwrap();
        assert_eq!(
            undersampled.analysis_into(
                &arbitrary_map(&undersampled),
                &mut undersampled_alm,
                &mut undersampled_workspace,
            ),
            Err(ShtError::AnalysisLatitudeUndersampled { lmax: 2, ntheta: 2 })
        );
    }

    #[test]
    #[allow(clippy::excessive_precision)]
    fn fejer1_analysis_accepts_minimum_even_longitudes() {
        // DUCC references: ducc0==0.41.0, F1 3x6, phi0=0.37, one thread.
        let plan = make_f1_plan(2, 2, 3, 6, 0.37);
        let map = arbitrary_map(&plan);
        let alm = coefficients(plan.harmonics());
        let mut workspace = plan.workspace().unwrap();
        let mut analyzed = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.analysis_into(&map, &mut analyzed, &mut workspace)
            .unwrap();
        let expected = [
            Complex64::new(2.689_316_937_951_824, 0.0),
            Complex64::new(-0.299_440_230_160_603_4, 0.0),
            Complex64::new(-0.013_893_099_564_934_452, 0.0),
            Complex64::new(0.224_633_317_762_154_5, -0.435_041_605_183_639_2),
            Complex64::new(-0.010_149_034_800_194_424, -0.063_517_653_564_240_91),
            Complex64::new(-0.093_868_080_667_352_89, 0.212_305_006_847_431_7),
        ];
        for (&actual, expected) in analyzed.iter().zip(expected) {
            assert_complex_close(actual, expected, 1e-12);
        }

        let mut analysis_adjoint = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace)
            .unwrap();
        for (index, expected) in [
            (0, 0.184_957_546_691_366_58),
            (6, 0.397_620_363_614_759_6),
            (17, -0.097_552_061_176_245_71),
        ] {
            assert_close(analysis_adjoint[index], expected, 1e-12);
        }
        assert_close(
            harmonic_inner_product(plan.harmonics(), &analyzed, &alm),
            map_inner_product(&map, &analysis_adjoint),
            2e-12,
        );
    }

    #[test]
    #[allow(clippy::excessive_precision)]
    fn fejer1_oversampled_analysis_matches_ducc() {
        // DUCC references: ducc0==0.41.0, F1 7x5, phi0=0.37, one thread.
        let plan = make_f1_plan(2, 2, 7, 5, 0.37);
        let map = arbitrary_map(&plan);
        let alm = coefficients(plan.harmonics());
        let mut workspace = plan.workspace().unwrap();
        let mut analyzed = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.analysis_into(&map, &mut analyzed, &mut workspace)
            .unwrap();
        let expected = [
            Complex64::new(3.286_660_215_255_482_7, 0.0),
            Complex64::new(-0.509_258_770_448_473_7, 0.0),
            Complex64::new(-0.107_215_381_801_019_3, 0.0),
            Complex64::new(0.194_947_339_035_520_2, -0.271_488_412_725_603_3),
            Complex64::new(0.011_389_941_302_963_157, -0.106_062_635_167_317_38),
            Complex64::new(-0.102_728_760_424_834_82, 0.148_405_483_952_625_12),
        ];
        for (&actual, expected) in analyzed.iter().zip(expected) {
            assert_complex_close(actual, expected, 1e-12);
        }

        let mut analysis_adjoint = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace)
            .unwrap();
        for (index, expected) in [
            (0, 0.029_402_401_338_101_222),
            (5, 0.123_113_649_028_543_33),
            (34, 0.022_831_874_150_001_888),
        ] {
            assert_close(analysis_adjoint[index], expected, 1e-12);
        }
        assert_close(
            harmonic_inner_product(plan.harmonics(), &analyzed, &alm),
            map_inner_product(&map, &analysis_adjoint),
            2e-12,
        );
    }

    #[test]
    #[allow(clippy::excessive_precision)]
    fn clenshaw_curtis_analysis_matches_ducc_without_initial_resampling() {
        // DUCC reference: ducc0==0.41.0, CC 7x5, phi0=0.37, one thread.
        let plan = make_cc_plan(2, 2, 7, 5, 0.37);
        let map = arbitrary_map(&plan);
        let mut coefficients = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut workspace = plan.workspace().unwrap();
        plan.analysis_into(&map, &mut coefficients, &mut workspace)
            .unwrap();
        let expected = [
            Complex64::new(3.312_543_485_375_359_7, 0.0),
            Complex64::new(-0.440_646_384_701_871_9, 0.0),
            Complex64::new(-0.080_662_065_710_822_795, 0.0),
            Complex64::new(0.194_847_818_808_176_12, -0.271_708_531_406_952_56),
            Complex64::new(0.009_293_865_922_364_808, -0.086_372_734_621_443_25),
            Complex64::new(-0.103_237_895_255_551_15, 0.149_175_081_850_113_86),
        ];
        for (&actual, expected) in coefficients.iter().zip(expected) {
            assert_complex_close(actual, expected, 8e-13);
        }

        let alm = self::coefficients(plan.harmonics());
        let mut analysis_adjoint = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace)
            .unwrap();
        for (index, expected) in [
            (0, 0.005_846_025_146_437_542),
            (1, 0.005_846_025_146_437_542),
            (2, 0.005_846_025_146_437_542),
            (3, 0.005_846_025_146_437_542),
            (4, 0.005_846_025_146_437_542),
            (5, 0.100_321_125_155_782_02),
            (10, 0.241_687_658_976_767_94),
            (15, 0.230_819_221_966_427_18),
            (34, 0.012_863_122_572_356_323),
        ] {
            assert_close(analysis_adjoint[index], expected, 8e-13);
        }
    }

    #[test]
    #[expect(
        clippy::excessive_precision,
        reason = "The fixture values retain printed ducc0 0.41.0 outputs."
    )]
    fn clenshaw_curtis_minimum_even_longitude_count_matches_ducc() {
        // DUCC reference: ducc0==0.41.0, CC 4x6, phi0=0.37, one thread.
        let plan = make_cc_plan(2, 2, 4, 6, 0.37);
        let map = arbitrary_map(&plan);
        let mut coefficients = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut workspace = plan.workspace().unwrap();
        plan.analysis_into(&map, &mut coefficients, &mut workspace)
            .unwrap();
        let expected = [
            Complex64::new(2.903_561_518_322_166_4, 0.0),
            Complex64::new(-0.293_423_837_509_477_2, 0.0),
            Complex64::new(-0.030_031_189_191_877_417, 0.0),
            Complex64::new(0.210_231_077_576_941_93, -0.360_541_545_558_988_21),
            Complex64::new(-0.003_831_592_732_144_799_6, -0.034_803_087_930_531_502),
            Complex64::new(-0.089_972_128_440_015_786, 0.193_588_058_937_762_71),
        ];
        for (&actual, expected) in coefficients.iter().zip(expected) {
            assert_complex_close(actual, expected, 8e-13);
        }
        let test_alm = self::coefficients(plan.harmonics());
        let mut analysis_adjoint = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_into(&test_alm, &mut analysis_adjoint, &mut workspace)
            .unwrap();
        assert_close(
            harmonic_inner_product(plan.harmonics(), &coefficients, &test_alm),
            map_inner_product(&map, &analysis_adjoint),
            8e-13,
        );

        let undersampled_longitudes = make_cc_plan(2, 2, 4, 4, 0.37);
        let mut undersampled_workspace = undersampled_longitudes.workspace().unwrap();
        let mut undersampled_coefficients =
            vec![Complex64::new(0.0, 0.0); undersampled_longitudes.harmonics().nalm()];
        let undersampled_map = vec![0.0; undersampled_longitudes.sample_count];
        assert_eq!(
            undersampled_longitudes.analysis_into(
                &undersampled_map,
                &mut undersampled_coefficients,
                &mut undersampled_workspace,
            ),
            Err(ShtError::AnalysisLongitudeUndersampled { mmax: 2, nphi: 4 })
        );

        let undersampled_latitudes = make_cc_plan(2, 2, 3, 5, 0.37);
        let mut latitude_workspace = undersampled_latitudes.workspace().unwrap();
        let mut latitude_coefficients =
            vec![Complex64::new(0.0, 0.0); undersampled_latitudes.harmonics().nalm()];
        let latitude_map = vec![0.0; undersampled_latitudes.sample_count];
        assert_eq!(
            undersampled_latitudes.analysis_into(
                &latitude_map,
                &mut latitude_coefficients,
                &mut latitude_workspace,
            ),
            Err(ShtError::AnalysisLatitudeUndersampled { lmax: 2, ntheta: 3 })
        );
    }

    #[test]
    fn clenshaw_curtis_t255_sectoral_analysis_matches_closed_form() {
        let plan = make_cc_plan(255, 255, 257, 513, 0.37);
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let sectoral_index = plan.harmonics().index(255, 255).unwrap();
        alm[sectoral_index] = Complex64::new(1.0, 0.0);
        let mut workspace = plan.workspace().unwrap();
        let mut map = vec![0.0; plan.sample_count];
        plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();

        let mut sectoral_norm = 1.0 / (4.0 * PI).sqrt();
        for degree in 1..=255 {
            sectoral_norm *= ((2 * degree + 1) as f64 / (2 * degree) as f64).sqrt();
        }
        for (ring, &theta) in plan.grid.theta().iter().enumerate() {
            let amplitude = -2.0 * sectoral_norm * theta.sin().powi(255);
            for longitude in 0..plan.grid.nphi() {
                let phi = plan.grid.phi0() + TAU * longitude as f64 / plan.grid.nphi() as f64;
                let expected = amplitude * (255.0 * phi).cos();
                assert_close(map[ring * plan.grid.nphi() + longitude], expected, 4e-12);
            }
        }

        let mut recovered = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.analysis_into(&map, &mut recovered, &mut workspace)
            .unwrap();
        for (index, &actual) in recovered.iter().enumerate() {
            let expected = if index == sectoral_index { 1.0 } else { 0.0 };
            assert!(
                (actual.re - expected).abs() <= 2e-9 && actual.im.abs() <= 2e-9,
                "coefficient {index}: actual={actual:?}, expected real={expected}"
            );
        }
    }

    #[test]
    fn fejer1_t255_sectoral_analysis_matches_closed_form() {
        let plan = make_f1_plan(255, 255, 256, 513, 0.37);
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let sectoral_index = plan.harmonics().index(255, 255).unwrap();
        alm[sectoral_index] = Complex64::new(1.0, 0.0);
        let mut workspace = plan.workspace().unwrap();
        let mut map = vec![0.0; plan.sample_count];
        plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();

        let mut sectoral_norm = 1.0 / (4.0 * PI).sqrt();
        for degree in 1..=255 {
            sectoral_norm *= ((2 * degree + 1) as f64 / (2 * degree) as f64).sqrt();
        }
        for (ring, &theta) in plan.grid.theta().iter().enumerate() {
            let amplitude = -2.0 * sectoral_norm * theta.sin().powi(255);
            for longitude in 0..plan.grid.nphi() {
                let phi = plan.grid.phi0() + TAU * longitude as f64 / plan.grid.nphi() as f64;
                let expected = amplitude * (255.0 * phi).cos();
                assert_close(map[ring * plan.grid.nphi() + longitude], expected, 4e-12);
            }
        }

        let mut recovered = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.analysis_into(&map, &mut recovered, &mut workspace)
            .unwrap();
        for (index, &actual) in recovered.iter().enumerate() {
            let expected = if index == sectoral_index { 1.0 } else { 0.0 };
            assert!(
                (actual.re - expected).abs() <= 2e-9 && actual.im.abs() <= 2e-9,
                "coefficient {index}: actual={actual:?}, expected real={expected}"
            );
        }
    }

    #[test]
    #[ignore = "M3 high-bandlimit acceptance; run this check in release mode"]
    #[expect(
        clippy::excessive_precision,
        reason = "The literals retain printed DUCC 0.41.0 reference values."
    )]
    fn m3_scalar_t511_t1023_t2047_all_grids_matches_ducc_and_closed_form() {
        // DUCC 0.41.0 references use a_(L,L)=1, phi0=0.37, nthreads=1.
        // The five values are the middle-ring samples at j=[0,1,17,nphi/2,nphi-1].
        for degree in [511, 1023, 2047] {
            let nphi = 2 * degree + 3;
            for kind in [
                GridKind::GaussLegendre,
                GridKind::ClenshawCurtis,
                GridKind::Fejer1,
            ] {
                let ntheta = degree + usize::from(kind == GridKind::ClenshawCurtis) + 1;
                let plan = ShtPlan::new(
                    GridSpec::new(kind, ntheta, nphi, 0.37).unwrap(),
                    HarmonicSpec::scalar(degree, degree).unwrap(),
                )
                .unwrap();
                let sectoral_index = plan.harmonics().index(degree, degree).unwrap();
                let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
                alm[sectoral_index] = Complex64::new(1.0, 0.0);
                let mut workspace = plan.workspace().unwrap();
                let mut map = vec![0.0; plan.sample_count];
                plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();

                let expected_ducc = match (degree, kind) {
                    (511, GridKind::GaussLegendre) => [
                        -2.3872256264278287,
                        2.4013318896018663,
                        2.5986634372168815,
                        1.5560946744052795,
                        2.3729175336200483,
                    ],
                    (511, GridKind::ClenshawCurtis) => [
                        -2.3929622710868896,
                        2.4071024324473411,
                        2.6049081794661149,
                        1.5598340621297142,
                        2.3786197950841248,
                    ],
                    (511, GridKind::Fejer1) => [
                        -2.3872144245647005,
                        2.4013206215462395,
                        2.598651243198931,
                        1.5560873725569058,
                        2.3729063988964887,
                    ],
                    (1023, GridKind::GaussLegendre) => [
                        -0.17570114687664429,
                        0.19125253260213948,
                        0.43930098463756251,
                        3.3817703089638673,
                        0.1601460438121452,
                    ],
                    (1023, GridKind::ClenshawCurtis) => [
                        -0.17591254311800286,
                        0.19148263961772272,
                        0.4398295331338507,
                        3.3858391129822953,
                        0.1603387248067325,
                    ],
                    (1023, GridKind::Fejer1) => [
                        -0.17570094045838802,
                        0.19125230791371206,
                        0.43930046853539056,
                        3.3817663359725523,
                        0.16014585566842582,
                    ],
                    (2047, GridKind::GaussLegendre) => [
                        3.887238035280272,
                        -3.8896657567270916,
                        -3.9257015210558133,
                        -1.0642856103322809,
                        -3.884789743029231,
                    ],
                    (2047, GridKind::ClenshawCurtis) => [
                        3.889578098030304,
                        -3.8920072809313684,
                        -3.9280647382845477,
                        -1.0649262953352392,
                        -3.8871283319416836,
                    ],
                    (2047, GridKind::Fejer1) => [
                        3.887236892735931,
                        -3.889664613469189,
                        -3.925700367206211,
                        -1.0642852975154442,
                        -3.8847886012044954,
                    ],
                    _ => unreachable!(),
                };
                let middle_ring = ntheta / 2;
                let longitudes = [0, 1, 17, nphi / 2, nphi - 1];
                let mut ducc_max_error: f64 = 0.0;
                for (longitude, expected) in longitudes.into_iter().zip(expected_ducc) {
                    let error = (map[middle_ring * nphi + longitude] - expected).abs();
                    ducc_max_error = ducc_max_error.max(error);
                    assert!(error <= 5e-11, "{kind:?} T{degree} DUCC error={error:.3e}");
                }

                let mut sectoral_norm = 1.0 / (4.0 * PI).sqrt();
                for l in 1..=degree {
                    sectoral_norm *= ((2 * l + 1) as f64 / (2 * l) as f64).sqrt();
                }
                let sign = if degree.is_multiple_of(2) { 1.0 } else { -1.0 };
                let (mut analytic_max_error, mut analytic_error_squares) = (0.0_f64, 0.0_f64);
                let mut analytic_sample_count = 0usize;
                for (ring, &theta) in plan.grid.theta().iter().enumerate() {
                    let amplitude = 2.0 * sign * sectoral_norm * theta.sin().powi(degree as i32);
                    for longitude in 0..nphi {
                        let phi = plan.grid.longitude(longitude).unwrap();
                        let expected = amplitude * (degree as f64 * phi).cos();
                        let error = (map[ring * nphi + longitude] - expected).abs();
                        analytic_max_error = analytic_max_error.max(error);
                        analytic_error_squares += error * error;
                        analytic_sample_count += 1;
                        assert!(
                            error <= 5e-10,
                            "{kind:?} T{degree} analytic map error={error:.3e}"
                        );
                    }
                }

                let mut recovered = vec![Complex64::new(0.0, 0.0); alm.len()];
                plan.analysis_into(&map, &mut recovered, &mut workspace)
                    .unwrap();
                let (mut coefficient_max_error, mut coefficient_error_squares) = (0.0_f64, 0.0_f64);
                for (index, actual) in recovered.iter().enumerate() {
                    let expected = if index == sectoral_index { 1.0 } else { 0.0 };
                    let error = (actual - Complex64::new(expected, 0.0)).norm();
                    coefficient_max_error = coefficient_max_error.max(error);
                    coefficient_error_squares += error * error;
                    assert!(
                        error <= 2e-7,
                        "{kind:?} T{degree} coefficient {index}: actual={actual:?}, expected={expected}"
                    );
                }
                eprintln!(
                    "M3 scalar T{degree} {kind:?}: DUCC middle-row max abs={ducc_max_error:.3e}; analytic map max abs={analytic_max_error:.3e}, RMS={:.3e}; analysis coefficient max abs={coefficient_max_error:.3e}, RMS={:.3e}",
                    (analytic_error_squares / analytic_sample_count as f64).sqrt(),
                    (coefficient_error_squares / recovered.len() as f64).sqrt(),
                );
            }
        }
    }
}
