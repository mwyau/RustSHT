// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;

#[allow(private_bounds)]
impl<T: ShtScalar> ShtPlan<T> {
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
            false,
            false,
            None,
            None,
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
}
