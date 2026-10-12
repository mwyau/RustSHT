// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use crate::error::ShtError;
use crate::legendre::{LATITUDE_BLOCK_SIZE, ShtScalar, longitude_phase, sin_cos_from_f64};
use crate::plan::ShtPlan;
#[cfg(feature = "parallel")]
use crate::transform::weighted_order_midpoint;
use crate::workspace::{RecurrenceWorkspace, ShtWorkspace};

const MAX_SUPPORTED_SPIN: usize = 4;

#[allow(private_bounds)]
impl<T: ShtScalar> ShtPlan<T> {
    /// Synthesizes two real map components for a spin-1 through spin-4 field.
    ///
    /// `e_alm` and `b_alm` are packed coefficient channels. The output slices
    /// are the two map components in DUCC order, each stored north-to-south
    /// with eastward longitude samples. Both coefficient channels must have
    /// the plan's packed length; their `m=0` values must be real and their
    /// degrees below the spin must be zero.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is outside `1..=4`, an input
    /// coefficient is invalid, a buffer length differs from the plan,
    /// synthesis longitude sampling is insufficient, or the workspace is
    /// incompatible.
    pub fn synthesis_spin_into(
        &self,
        e_alm: &[Complex<T>],
        b_alm: &[Complex<T>],
        first_component: &mut [T],
        second_component: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let spin = self.supported_spin()?;
        self.synthesis_spin_core(
            e_alm,
            Some(b_alm),
            first_component,
            second_component,
            spin,
            false,
            workspace,
        )
    }

    /// Synthesizes a spin field with only its first coefficient channel set.
    ///
    /// This is the DUCC `GRAD_ONLY` operation. It uses the plan's spin and
    /// writes both real map components.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is outside `1..=4`, an input
    /// coefficient is invalid, a buffer length differs from the plan,
    /// synthesis longitude sampling is insufficient, or the workspace is
    /// incompatible.
    pub fn synthesis_gradient_into(
        &self,
        e_alm: &[Complex<T>],
        first_component: &mut [T],
        second_component: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let spin = self.supported_spin()?;
        self.synthesis_spin_core(
            e_alm,
            None,
            first_component,
            second_component,
            spin,
            false,
            workspace,
        )
    }

    /// Synthesizes the angular derivatives of a scalar field.
    ///
    /// The first output is `d/dtheta`; the second is
    /// `(1/sin(theta))*d/dphi`. This is the DUCC `DERIV1` operation and uses
    /// the scalar plan's coefficients with a spin-1 recurrence.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan has nonzero spin, an input coefficient
    /// is invalid, a buffer length differs from the plan, synthesis longitude
    /// sampling is insufficient, or the workspace is incompatible.
    pub fn synthesis_deriv1_into(
        &self,
        alm: &[Complex<T>],
        theta_derivative: &mut [T],
        phi_derivative: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        if self.harmonics.spin() != 0 {
            return Err(ShtError::UnsupportedSpin {
                spin: self.harmonics.spin(),
            });
        }
        if self.harmonics.lmax() == 0 {
            self.harmonics.validate_coefficients(alm)?;
            check_len("scalar coefficient input", self.harmonics.nalm(), alm.len())?;
            check_len(
                "theta derivative output",
                self.sample_count,
                theta_derivative.len(),
            )?;
            check_len(
                "phi derivative output",
                self.sample_count,
                phi_derivative.len(),
            )?;
            self.check_synthesis_longitude()?;
            self.check_workspace(workspace)?;
            theta_derivative.fill(T::zero());
            phi_derivative.fill(T::zero());
            return Ok(());
        }
        self.synthesis_spin_core(
            alm,
            None,
            theta_derivative,
            phi_derivative,
            1,
            true,
            workspace,
        )
    }

    /// Analyzes two real map components into spin-1 through spin-4 E/B
    /// coefficients.
    ///
    /// Exact GL and F1 analysis require `ntheta >= lmax + 1`; exact CC analysis
    /// requires `ntheta >= lmax + 2`. All require
    /// `mmax <= (nphi - 1) / 2`. Each output coefficient slice has the plan's
    /// packed length; degrees below the spin are returned as zero.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is outside `1..=4`, exact-analysis
    /// sampling is insufficient, a buffer length differs from the plan, or
    /// the workspace is incompatible.
    pub fn analysis_spin_into(
        &self,
        first_component: &[T],
        second_component: &[T],
        e_alm: &mut [Complex<T>],
        b_alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let spin = self.supported_spin()?;
        self.check_analysis_sampling()?;
        self.map_to_spin_coefficients(
            first_component,
            second_component,
            e_alm,
            Some(b_alm),
            spin,
            false,
            Some(self.grid.ring_weights()),
            true,
            workspace,
        )
    }

    /// Applies the adjoint of spin-1 through spin-4 STANDARD synthesis.
    ///
    /// The map dot product is an unweighted pixel sum. The returned E/B
    /// coefficients use the real harmonic inner product, including weight
    /// two for `m > 0`. This operation accepts the even longitude Nyquist
    /// order and has no latitude sampling floor.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is outside `1..=4`, longitude
    /// sampling is insufficient, a buffer length differs from the plan, or
    /// the workspace is incompatible.
    pub fn adjoint_synthesis_spin_into(
        &self,
        first_component: &[T],
        second_component: &[T],
        e_alm: &mut [Complex<T>],
        b_alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let spin = self.supported_spin()?;
        self.check_synthesis_longitude()?;
        self.map_to_spin_coefficients(
            first_component,
            second_component,
            e_alm,
            Some(b_alm),
            spin,
            false,
            None,
            false,
            workspace,
        )
    }

    /// Applies the adjoint of spin-1 through spin-4 `GRAD_ONLY` synthesis.
    ///
    /// The two input maps produce only the first (gradient) coefficient
    /// channel. The map dot product is an unweighted pixel sum; the real
    /// harmonic inner product weights `m > 0` by two.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is outside `1..=4`, longitude
    /// sampling is insufficient, a buffer length differs from the plan, or
    /// the workspace is incompatible.
    pub fn adjoint_synthesis_gradient_into(
        &self,
        first_component: &[T],
        second_component: &[T],
        e_alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let spin = self.supported_spin()?;
        self.check_synthesis_longitude()?;
        self.map_to_spin_coefficients(
            first_component,
            second_component,
            e_alm,
            None,
            spin,
            false,
            None,
            false,
            workspace,
        )
    }

    /// Applies the adjoint of scalar `DERIV1` synthesis.
    ///
    /// The input slices contain the theta and phi derivative map components.
    /// The output is a scalar coefficient channel, with degree zero equal to
    /// zero. The map dot product is an unweighted pixel sum.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan is not scalar, longitude sampling is
    /// insufficient, a buffer length differs from the plan, or the workspace
    /// is incompatible.
    pub fn adjoint_synthesis_deriv1_into(
        &self,
        theta_derivative: &[T],
        phi_derivative: &[T],
        alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        if self.harmonics.spin() != 0 {
            return Err(ShtError::UnsupportedSpin {
                spin: self.harmonics.spin(),
            });
        }
        self.check_synthesis_longitude()?;
        check_len(
            "theta derivative input",
            self.sample_count,
            theta_derivative.len(),
        )?;
        check_len(
            "phi derivative input",
            self.sample_count,
            phi_derivative.len(),
        )?;
        check_len(
            "scalar coefficient output",
            self.harmonics.nalm(),
            alm.len(),
        )?;
        self.check_workspace(workspace)?;
        if self.harmonics.lmax() == 0 {
            alm.fill(Complex::<T>::new(T::zero(), T::zero()));
            return Ok(());
        }
        self.map_to_spin_coefficients(
            theta_derivative,
            phi_derivative,
            alm,
            None,
            1,
            true,
            None,
            false,
            workspace,
        )
    }

    /// Applies the adjoint of exact spin-1 through spin-4 analysis.
    ///
    /// The output maps are STANDARD spin synthesis. GL analysis scales each
    /// ring by `ring_weight / nphi`; CC analysis uses the adjoint of its
    /// prepared-grid resampling path.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is outside `1..=4`, exact-analysis
    /// sampling is insufficient, a coefficient or output buffer is invalid,
    /// or the workspace is incompatible.
    pub fn adjoint_analysis_spin_into(
        &self,
        e_alm: &[Complex<T>],
        b_alm: &[Complex<T>],
        first_component: &mut [T],
        second_component: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.supported_spin()?;
        self.check_analysis_sampling()?;
        self.harmonics.validate_coefficients(e_alm)?;
        self.harmonics.validate_coefficients(b_alm)?;
        check_len("E coefficient input", self.harmonics.nalm(), e_alm.len())?;
        check_len("B coefficient input", self.harmonics.nalm(), b_alm.len())?;
        check_len(
            "first spin map output",
            self.sample_count,
            first_component.len(),
        )?;
        check_len(
            "second spin map output",
            self.sample_count,
            second_component.len(),
        )?;
        self.check_workspace(workspace)?;
        if self.cc_resampling.is_some() {
            self.adjoint_analysis_spin_cc_into(
                e_alm,
                b_alm,
                first_component,
                second_component,
                workspace,
            )
        } else {
            self.synthesis_spin_into(e_alm, b_alm, first_component, second_component, workspace)?;
            self.apply_analysis_pixel_weights(first_component);
            self.apply_analysis_pixel_weights(second_component);
            Ok(())
        }
    }

    fn adjoint_analysis_spin_cc_into(
        &self,
        e_alm: &[Complex<T>],
        b_alm: &[Complex<T>],
        first_component: &mut [T],
        second_component: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let spin = self.supported_spin()?;
        let resampling = self
            .cc_resampling
            .as_ref()
            .expect("CC adjoint analysis requires the prepared CC operator");
        let prepared_ntheta = resampling.prepared_ntheta();
        for (ring, &theta) in resampling.prepared_theta().iter().enumerate() {
            (workspace.theta_sin[ring], workspace.theta_cos[ring]) = sin_cos_from_f64::<T>(theta);
            workspace.ring_order_limits[ring] = T::polar_order_limit(
                self.harmonics.lmax(),
                spin,
                workspace.theta_sin[ring],
                workspace.theta_cos[ring],
            );
        }
        workspace
            .prepared_spectra
            .fill(Complex::<T>::new(T::zero(), T::zero()));
        workspace
            .prepared_spin_spectra
            .fill(Complex::<T>::new(T::zero(), T::zero()));
        self.synthesize_spin_ring_group(
            e_alm,
            Some(b_alm),
            spin,
            false,
            1,
            self.fft.spectrum_len(),
            self.grid.phi0(),
            0,
            prepared_ntheta,
            &mut workspace.prepared_spectra,
            &mut workspace.prepared_spin_spectra,
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
            spin,
            &mut workspace.cc_profile,
            &mut workspace.cc_fft_scratch,
        )?;
        resampling.restore_original(
            &workspace.prepared_spin_spectra,
            &mut workspace.spin_spectra,
            self.fft.spectrum_len(),
            self.harmonics.mmax(),
            spin,
            &mut workspace.cc_profile,
            &mut workspace.cc_fft_scratch,
        )?;
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        for ring in 0..self.grid.ntheta() {
            let start = ring * spectrum_len;
            workspace.spectra[start].im = T::zero();
            workspace.spin_spectra[start].im = T::zero();
            self.fft.inverse(
                &workspace.spectra[start..start + spectrum_len],
                &mut workspace.row,
                &mut workspace.fft,
            )?;
            let output_start = ring * nphi;
            first_component[output_start..output_start + nphi].copy_from_slice(&workspace.row);
            self.fft.inverse(
                &workspace.spin_spectra[start..start + spectrum_len],
                &mut workspace.row,
                &mut workspace.fft,
            )?;
            second_component[output_start..output_start + nphi].copy_from_slice(&workspace.row);
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The map-to-coefficient driver covers weighted analysis and the four synthesis adjoints."
    )]
    fn map_to_spin_coefficients(
        &self,
        first_component: &[T],
        second_component: &[T],
        e_alm: &mut [Complex<T>],
        mut b_alm: Option<&mut [Complex<T>]>,
        recurrence_spin: usize,
        derivative_norm: bool,
        ring_weights: Option<&[f64]>,
        normalize_fourier: bool,
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        if !matches!(recurrence_spin, 1..=MAX_SUPPORTED_SPIN) {
            return Err(ShtError::UnsupportedSpin {
                spin: recurrence_spin,
            });
        }
        if !derivative_norm && self.harmonics.spin() != recurrence_spin {
            return Err(ShtError::UnsupportedSpin {
                spin: self.harmonics.spin(),
            });
        }
        check_len(
            "first spin map input",
            self.sample_count,
            first_component.len(),
        )?;
        check_len(
            "second spin map input",
            self.sample_count,
            second_component.len(),
        )?;
        check_len("E coefficient output", self.harmonics.nalm(), e_alm.len())?;
        if let Some(b_alm) = b_alm.as_deref() {
            check_len("B coefficient output", self.harmonics.nalm(), b_alm.len())?;
        }
        self.check_workspace(workspace)?;

        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        for ring in 0..ntheta {
            let input_start = ring * nphi;
            let spectrum_start = ring * spectrum_len;
            let end = spectrum_start + spectrum_len;
            self.fft.forward(
                &first_component[input_start..input_start + nphi],
                &mut workspace.spectra[spectrum_start..end],
                &mut workspace.fft,
            )?;
            self.fft.forward(
                &second_component[input_start..input_start + nphi],
                &mut workspace.spin_spectra[spectrum_start..end],
                &mut workspace.fft,
            )?;
        }

        e_alm.fill(Complex::<T>::new(T::zero(), T::zero()));
        if let Some(b_alm) = b_alm.as_deref_mut() {
            b_alm.fill(Complex::<T>::new(T::zero(), T::zero()));
        }
        let (
            q_spectra,
            u_spectra,
            analysis_ntheta,
            analysis_ring_weights,
            analysis_normalize_fourier,
        ) = if let Some(resampling) = self
            .cc_resampling
            .as_ref()
            .filter(|_| ring_weights.is_some())
        {
            for value in &mut workspace.spectra {
                *value /= T::from_usize(nphi).expect("longitude count converts");
            }
            for value in &mut workspace.spin_spectra {
                *value /= T::from_usize(nphi).expect("longitude count converts");
            }
            resampling.to_prepared(
                &workspace.spectra,
                &mut workspace.prepared_spectra,
                spectrum_len,
                self.harmonics.mmax(),
                recurrence_spin,
                &mut workspace.cc_profile,
                &mut workspace.cc_fft_scratch,
            )?;
            resampling.to_prepared(
                &workspace.spin_spectra,
                &mut workspace.prepared_spin_spectra,
                spectrum_len,
                self.harmonics.mmax(),
                recurrence_spin,
                &mut workspace.cc_profile,
                &mut workspace.cc_fft_scratch,
            )?;
            for (ring, &theta) in resampling.prepared_theta().iter().enumerate() {
                (workspace.theta_sin[ring], workspace.theta_cos[ring]) =
                    sin_cos_from_f64::<T>(theta);
                workspace.ring_order_limits[ring] = T::polar_order_limit(
                    self.harmonics.lmax(),
                    recurrence_spin,
                    workspace.theta_sin[ring],
                    workspace.theta_cos[ring],
                );
            }
            (
                workspace.prepared_spectra.as_slice(),
                workspace.prepared_spin_spectra.as_slice(),
                resampling.prepared_ntheta(),
                None,
                false,
            )
        } else {
            self.prepare_spin_latitudes(workspace, recurrence_spin);
            (
                workspace.spectra.as_slice(),
                workspace.spin_spectra.as_slice(),
                ntheta,
                ring_weights,
                normalize_fourier,
            )
        };
        let theta_sin = &workspace.theta_sin[..analysis_ntheta];
        let theta_cos = &workspace.theta_cos[..analysis_ntheta];
        let ring_order_limits = &workspace.ring_order_limits[..analysis_ntheta];
        let arch = pulp::Arch::new();
        let order_count = self.harmonics.mmax() + 1;

        #[cfg(feature = "parallel")]
        {
            let worker_count = workspace.recurrence_workers.len().min(order_count);
            let use_parallel = rayon::current_num_threads() > 1 && worker_count > 1;
            if use_parallel {
                self.analyze_spin_order_groups_parallel(
                    q_spectra,
                    u_spectra,
                    nphi,
                    spectrum_len,
                    self.grid.phi0(),
                    analysis_ring_weights,
                    analysis_normalize_fourier,
                    recurrence_spin,
                    derivative_norm,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    0,
                    order_count,
                    e_alm,
                    b_alm.as_deref_mut(),
                    &mut workspace.recurrence_workers[..worker_count],
                )?;
            } else {
                self.analyze_spin_order_group(
                    q_spectra,
                    u_spectra,
                    nphi,
                    spectrum_len,
                    self.grid.phi0(),
                    analysis_ring_weights,
                    analysis_normalize_fourier,
                    recurrence_spin,
                    derivative_norm,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    0,
                    order_count,
                    e_alm,
                    b_alm.as_deref_mut(),
                    &mut workspace.recurrence_workers[0],
                )?;
            }
        }
        #[cfg(not(feature = "parallel"))]
        self.analyze_spin_order_group(
            q_spectra,
            u_spectra,
            nphi,
            spectrum_len,
            self.grid.phi0(),
            analysis_ring_weights,
            analysis_normalize_fourier,
            recurrence_spin,
            derivative_norm,
            theta_sin,
            theta_cos,
            ring_order_limits,
            arch,
            0,
            order_count,
            e_alm,
            b_alm.as_deref_mut(),
            &mut workspace.recurrence_workers[0],
        )?;

        for degree in recurrence_spin..=self.harmonics.lmax() {
            let index = self
                .harmonics
                .index(degree, 0)
                .expect("order zero contains every degree");
            e_alm[index].im = T::zero();
            if let Some(b_alm) = b_alm.as_deref_mut() {
                b_alm[index].im = T::zero();
            }
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The shared STANDARD, GRAD_ONLY and DERIV1 driver receives two channels, outputs and recurrence mode."
    )]
    fn synthesis_spin_core(
        &self,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        first_component: &mut [T],
        second_component: &mut [T],
        recurrence_spin: usize,
        derivative_norm: bool,
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        if !matches!(recurrence_spin, 1..=MAX_SUPPORTED_SPIN) {
            return Err(ShtError::UnsupportedSpin {
                spin: recurrence_spin,
            });
        }
        if !derivative_norm && self.harmonics.spin() != recurrence_spin {
            return Err(ShtError::UnsupportedSpin {
                spin: self.harmonics.spin(),
            });
        }
        self.harmonics.validate_coefficients(e_alm)?;
        if let Some(b_alm) = b_alm {
            self.harmonics.validate_coefficients(b_alm)?;
        }
        check_len(
            "first spin map output",
            self.sample_count,
            first_component.len(),
        )?;
        check_len(
            "second spin map output",
            self.sample_count,
            second_component.len(),
        )?;
        self.check_synthesis_longitude()?;
        self.check_workspace(workspace)?;

        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        let phi0 = self.grid.phi0();
        self.prepare_spin_latitudes(workspace, recurrence_spin);
        workspace
            .spectra
            .fill(Complex::<T>::new(T::zero(), T::zero()));
        workspace
            .spin_spectra
            .fill(Complex::<T>::new(T::zero(), T::zero()));
        let arch = pulp::Arch::new();

        #[cfg(feature = "parallel")]
        {
            let group_ring_count = workspace.ring_group_size.min(ntheta);
            let use_parallel =
                rayon::current_num_threads() > 1 && workspace.recurrence_workers.len() > 1;
            if use_parallel {
                let group_spectrum_len = group_ring_count.checked_mul(spectrum_len).ok_or(
                    ShtError::IntegerOverflow {
                        quantity: "spin Fourier spectrum group size",
                    },
                )?;
                workspace
                    .spectra
                    .par_chunks_mut(group_spectrum_len)
                    .zip(workspace.spin_spectra.par_chunks_mut(group_spectrum_len))
                    .zip(workspace.recurrence_workers.par_iter_mut())
                    .enumerate()
                    .try_for_each(|(group_index, ((q_spectra, u_spectra), recurrence))| {
                        let ring_start = group_index * group_ring_count;
                        let ring_end = (ring_start + q_spectra.len() / spectrum_len).min(ntheta);
                        self.synthesize_spin_ring_group(
                            e_alm,
                            b_alm,
                            recurrence_spin,
                            derivative_norm,
                            nphi,
                            spectrum_len,
                            phi0,
                            ring_start,
                            ring_end,
                            q_spectra,
                            u_spectra,
                            &workspace.theta_sin[ring_start..ring_end],
                            &workspace.theta_cos[ring_start..ring_end],
                            &workspace.ring_order_limits,
                            arch,
                            recurrence,
                        )
                    })?;
            } else {
                self.synthesize_spin_ring_group(
                    e_alm,
                    b_alm,
                    recurrence_spin,
                    derivative_norm,
                    nphi,
                    spectrum_len,
                    phi0,
                    0,
                    ntheta,
                    &mut workspace.spectra,
                    &mut workspace.spin_spectra,
                    &workspace.theta_sin[..ntheta],
                    &workspace.theta_cos[..ntheta],
                    &workspace.ring_order_limits[..ntheta],
                    arch,
                    &mut workspace.recurrence_workers[0],
                )?;
            }
        }
        #[cfg(not(feature = "parallel"))]
        self.synthesize_spin_ring_group(
            e_alm,
            b_alm,
            recurrence_spin,
            derivative_norm,
            nphi,
            spectrum_len,
            phi0,
            0,
            ntheta,
            &mut workspace.spectra,
            &mut workspace.spin_spectra,
            &workspace.theta_sin[..ntheta],
            &workspace.theta_cos[..ntheta],
            &workspace.ring_order_limits[..ntheta],
            arch,
            &mut workspace.recurrence_workers[0],
        )?;

        for ring in 0..ntheta {
            let spectrum_start = ring * spectrum_len;
            let spectrum_end = spectrum_start + spectrum_len;
            self.fft.inverse(
                &workspace.spectra[spectrum_start..spectrum_end],
                &mut workspace.row,
                &mut workspace.fft,
            )?;
            let output_start = ring * nphi;
            first_component[output_start..output_start + nphi].copy_from_slice(&workspace.row);
            self.fft.inverse(
                &workspace.spin_spectra[spectrum_start..spectrum_end],
                &mut workspace.row,
                &mut workspace.fft,
            )?;
            second_component[output_start..output_start + nphi].copy_from_slice(&workspace.row);
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Each ring group receives plan inputs and its own caller-owned recurrence workspace."
    )]
    fn synthesize_spin_ring_group(
        &self,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        spin: usize,
        derivative_norm: bool,
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_start: usize,
        ring_end: usize,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let mmax = self.harmonics.mmax();
        debug_assert_eq!(ring_end - ring_start, theta_sin.len());
        debug_assert_eq!(q_spectra.len(), (ring_end - ring_start) * spectrum_len);
        debug_assert_eq!(u_spectra.len(), q_spectra.len());

        let fourier_scale = T::from_usize(nphi).expect("longitude count converts");
        let nyquist_scale = fourier_scale * T::from_f64(2.0).expect("two is representable");
        for m in 0..=mmax {
            let initial = T::fill_spin_factors(
                spin,
                m,
                lmax,
                &mut recurrence.spin_alpha,
                &mut recurrence.spin_coefficient_a,
                &mut recurrence.spin_coefficient_b,
            )?;
            let phase = longitude_phase::<T>(m, phi0, false);
            T::for_each_spin_lm_dispatched(
                arch,
                &initial,
                lmax,
                theta_sin,
                theta_cos,
                &recurrence.spin_coefficient_a,
                &recurrence.spin_coefficient_b,
                &mut recurrence.spin_previous_plus,
                &mut recurrence.spin_current_plus,
                &mut recurrence.spin_next_plus,
                &mut recurrence.spin_exponents_plus,
                &mut recurrence.spin_previous_minus,
                &mut recurrence.spin_current_minus,
                &mut recurrence.spin_next_minus,
                &mut recurrence.spin_exponents_minus,
                |degree, group_ring_start, plus, plus_exp, minus, minus_exp| {
                    let coefficient_index = self
                        .harmonics
                        .index(degree, m)
                        .expect("the recurrence visits degrees in the packed order");
                    let e_coefficient = e_alm[coefficient_index];
                    let b_coefficient = b_alm
                        .map(|values| values[coefficient_index])
                        .unwrap_or_else(|| Complex::new(T::zero(), T::zero()));
                    let alpha_norm = recurrence.spin_alpha[degree]
                        * T::spin_normalization(spin, degree, derivative_norm);
                    for lane in 0..plus.len() {
                        let group_ring = group_ring_start + lane;
                        let global_ring = ring_start + group_ring;
                        if m > ring_order_limits[global_ring] {
                            continue;
                        }
                        let plus_value =
                            T::scaled_basis_value(plus[lane], plus_exp[lane], alpha_norm);
                        let minus_value =
                            T::scaled_basis_value(minus[lane], minus_exp[lane], alpha_norm);
                        let even_basis = plus_value + minus_value;
                        let odd_basis = plus_value - minus_value;
                        let q_value = e_coefficient * even_basis
                            + Complex::<T>::new(T::zero(), -T::one()) * b_coefficient * odd_basis;
                        let u_value =
                            Complex::<T>::new(T::zero(), T::one()) * e_coefficient * odd_basis
                                + b_coefficient * even_basis;
                        let spectrum_index = group_ring * spectrum_len + m;
                        q_spectra[spectrum_index] += q_value;
                        u_spectra[spectrum_index] += u_value;
                    }
                    Ok(())
                },
            )?;
            for group_ring in 0..ring_end - ring_start {
                let spectrum_index = group_ring * spectrum_len + m;
                let phased_q = q_spectra[spectrum_index] * phase;
                let phased_u = u_spectra[spectrum_index] * phase;
                if nphi.is_multiple_of(2) && m == nphi / 2 {
                    q_spectra[spectrum_index] =
                        Complex::new(nyquist_scale * phased_q.re, T::zero());
                    u_spectra[spectrum_index] =
                        Complex::new(nyquist_scale * phased_u.re, T::zero());
                } else {
                    q_spectra[spectrum_index] = phased_q * fourier_scale;
                    u_spectra[spectrum_index] = phased_u * fourier_scale;
                }
            }
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The analysis group shares spectra and grid metadata and owns two coefficient slices plus worker scratch."
    )]
    fn analyze_spin_order_group(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_weights: Option<&[f64]>,
        normalize_fourier: bool,
        recurrence_spin: usize,
        derivative_norm: bool,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        m_start: usize,
        m_end: usize,
        e_alm: &mut [Complex<T>],
        mut b_alm: Option<&mut [Complex<T>]>,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let longitude_count = T::from_usize(nphi).expect("longitude count converts");
        let mut coefficient_start = 0;
        for m in m_start..m_end {
            let coefficient_count = lmax - m + 1;
            let e_coefficients =
                &mut e_alm[coefficient_start..coefficient_start + coefficient_count];
            let mut b_coefficients = b_alm.as_deref_mut().map(|values| {
                &mut values[coefficient_start..coefficient_start + coefficient_count]
            });
            let initial = T::fill_spin_factors(
                recurrence_spin,
                m,
                lmax,
                &mut recurrence.spin_alpha,
                &mut recurrence.spin_coefficient_a,
                &mut recurrence.spin_coefficient_b,
            )?;
            let phase = longitude_phase::<T>(m, phi0, true);
            for block_start in (0..theta_sin.len()).step_by(LATITUDE_BLOCK_SIZE) {
                let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(theta_sin.len());
                let block_len = block_end - block_start;
                for lane in 0..block_len {
                    let ring = block_start + lane;
                    let spectrum_start = ring * spectrum_len;
                    let mut q_weighted = q_spectra[spectrum_start + m] * phase;
                    let mut u_weighted = u_spectra[spectrum_start + m] * phase;
                    let mut scale = ring_weights.map_or(T::one(), |weights| {
                        T::from_f64(weights[ring]).expect("ring weight is representable")
                    });
                    if normalize_fourier {
                        scale /= longitude_count;
                    }
                    q_weighted *= scale;
                    u_weighted *= scale;
                    recurrence.weighted_spin_q_spectrum[lane] = q_weighted;
                    recurrence.weighted_spin_u_spectrum[lane] = u_weighted;
                }
                let weighted_q_spectrum = &recurrence.weighted_spin_q_spectrum[..block_len];
                let weighted_u_spectrum = &recurrence.weighted_spin_u_spectrum[..block_len];
                T::for_each_spin_lm_dispatched(
                    arch,
                    &initial,
                    lmax,
                    &theta_sin[block_start..block_end],
                    &theta_cos[block_start..block_end],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &mut recurrence.spin_previous_plus,
                    &mut recurrence.spin_current_plus,
                    &mut recurrence.spin_next_plus,
                    &mut recurrence.spin_exponents_plus,
                    &mut recurrence.spin_previous_minus,
                    &mut recurrence.spin_current_minus,
                    &mut recurrence.spin_next_minus,
                    &mut recurrence.spin_exponents_minus,
                    |degree, ring_start, plus, plus_exp, minus, minus_exp| {
                        let alpha_norm = recurrence.spin_alpha[degree]
                            * T::spin_normalization(recurrence_spin, degree, derivative_norm);
                        let coefficient = &mut e_coefficients[degree - m];
                        let mut b_coefficient = b_coefficients
                            .as_deref_mut()
                            .map(|values| &mut values[degree - m]);
                        for lane in 0..plus.len() {
                            let ring = ring_start + lane;
                            if m > ring_order_limits[block_start + ring] {
                                continue;
                            }
                            let plus_value =
                                T::scaled_basis_value(plus[lane], plus_exp[lane], alpha_norm);
                            let minus_value =
                                T::scaled_basis_value(minus[lane], minus_exp[lane], alpha_norm);
                            let even_basis = plus_value + minus_value;
                            let odd_basis = plus_value - minus_value;
                            let q_fourier = weighted_q_spectrum[ring];
                            let u_fourier = weighted_u_spectrum[ring];
                            *coefficient += q_fourier * even_basis
                                + Complex::<T>::new(T::zero(), -T::one()) * u_fourier * odd_basis;
                            if let Some(b_coefficient) = b_coefficient.as_mut() {
                                **b_coefficient +=
                                    Complex::<T>::new(T::zero(), T::one()) * q_fourier * odd_basis
                                        + u_fourier * even_basis;
                            }
                        }
                        Ok(())
                    },
                )?;
            }
            coefficient_start += coefficient_count;
        }
        debug_assert_eq!(coefficient_start, e_alm.len());
        if let Some(b_alm) = b_alm {
            debug_assert_eq!(coefficient_start, b_alm.len());
        }
        Ok(())
    }

    #[cfg(feature = "parallel")]
    #[expect(
        clippy::too_many_arguments,
        reason = "Recursive analysis splits coefficient outputs and recurrence scratch while sharing immutable grid inputs."
    )]
    fn analyze_spin_order_groups_parallel(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_weights: Option<&[f64]>,
        normalize_fourier: bool,
        recurrence_spin: usize,
        derivative_norm: bool,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        m_start: usize,
        m_end: usize,
        e_alm: &mut [Complex<T>],
        b_alm: Option<&mut [Complex<T>]>,
        recurrence: &mut [RecurrenceWorkspace<T>],
    ) -> Result<(), ShtError> {
        let order_count = m_end - m_start;
        if order_count <= 1 || recurrence.len() <= 1 {
            return self.analyze_spin_order_group(
                q_spectra,
                u_spectra,
                nphi,
                spectrum_len,
                phi0,
                ring_weights,
                normalize_fourier,
                recurrence_spin,
                derivative_norm,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                m_start,
                m_end,
                e_alm,
                b_alm,
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
        let (left_e, right_e) = e_alm.split_at_mut(coefficient_split);
        let (left_b, right_b) = if let Some(b_alm) = b_alm {
            let (left, right) = b_alm.split_at_mut(coefficient_split);
            (Some(left), Some(right))
        } else {
            (None, None)
        };
        let left_worker_count = recurrence.len().div_ceil(2);
        let (left_recurrence, right_recurrence) = recurrence.split_at_mut(left_worker_count);
        let (left_result, right_result) = rayon::join(
            || {
                self.analyze_spin_order_groups_parallel(
                    q_spectra,
                    u_spectra,
                    nphi,
                    spectrum_len,
                    phi0,
                    ring_weights,
                    normalize_fourier,
                    recurrence_spin,
                    derivative_norm,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    m_start,
                    m_mid,
                    left_e,
                    left_b,
                    left_recurrence,
                )
            },
            || {
                self.analyze_spin_order_groups_parallel(
                    q_spectra,
                    u_spectra,
                    nphi,
                    spectrum_len,
                    phi0,
                    ring_weights,
                    normalize_fourier,
                    recurrence_spin,
                    derivative_norm,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    m_mid,
                    m_end,
                    right_e,
                    right_b,
                    right_recurrence,
                )
            },
        );
        left_result?;
        right_result
    }

    fn prepare_spin_latitudes(&self, workspace: &mut ShtWorkspace<T>, spin: usize) {
        for (ring, &theta) in self.grid.theta().iter().enumerate() {
            (workspace.theta_sin[ring], workspace.theta_cos[ring]) = sin_cos_from_f64::<T>(theta);
            workspace.ring_order_limits[ring] = T::polar_order_limit(
                self.harmonics.lmax(),
                spin,
                workspace.theta_sin[ring],
                workspace.theta_cos[ring],
            );
        }
    }

    fn supported_spin(&self) -> Result<usize, ShtError> {
        match self.harmonics.spin() {
            spin @ 1..=MAX_SUPPORTED_SPIN => Ok(spin),
            spin => Err(ShtError::UnsupportedSpin { spin }),
        }
    }

    fn check_synthesis_longitude(&self) -> Result<(), ShtError> {
        if self.harmonics.mmax() <= self.grid.nphi() / 2 {
            Ok(())
        } else {
            Err(ShtError::SynthesisLongitudeUndersampled {
                mmax: self.harmonics.mmax(),
                nphi: self.grid.nphi(),
            })
        }
    }
}

fn check_len(buffer: &'static str, expected: usize, actual: usize) -> Result<(), ShtError> {
    if expected == actual {
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
mod tests;
