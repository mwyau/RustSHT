// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex64;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use crate::error::ShtError;
use crate::legendre::{
    fill_spin_factors, for_each_spin_lm_dispatched, polar_order_limit, scaled_basis_value,
    spin_normalization,
};
use crate::plan::ShtPlan;
use crate::workspace::{RecurrenceWorkspace, ShtWorkspace};

impl ShtPlan {
    /// Synthesizes the two real map components for a spin-1 or spin-2 field.
    ///
    /// `e_alm` and `b_alm` are packed coefficient channels. The output slices
    /// are the two map components in DUCC order, each stored north-to-south
    /// with eastward longitude samples. Both coefficient channels must have
    /// the plan's packed length; their `m=0` values must be real and their
    /// degrees below the spin must be zero.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is not 1 or 2, an input coefficient
    /// is invalid, a buffer length differs from the plan, synthesis longitude
    /// sampling is insufficient, or the workspace is incompatible.
    pub fn synthesis_spin_into(
        &self,
        e_alm: &[Complex64],
        b_alm: &[Complex64],
        first_component: &mut [f64],
        second_component: &mut [f64],
        workspace: &mut ShtWorkspace,
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
    /// Returns an error if the plan spin is not 1 or 2, an input coefficient
    /// is invalid, a buffer length differs from the plan, synthesis longitude
    /// sampling is insufficient, or the workspace is incompatible.
    pub fn synthesis_gradient_into(
        &self,
        e_alm: &[Complex64],
        first_component: &mut [f64],
        second_component: &mut [f64],
        workspace: &mut ShtWorkspace,
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
        alm: &[Complex64],
        theta_derivative: &mut [f64],
        phi_derivative: &mut [f64],
        workspace: &mut ShtWorkspace,
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
            theta_derivative.fill(0.0);
            phi_derivative.fill(0.0);
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

    /// Analyzes two real spin-1 or spin-2 map components into E/B coefficients.
    ///
    /// Exact GL analysis requires `ntheta >= lmax + 1` and
    /// `mmax <= (nphi - 1) / 2`. Each output coefficient slice has the plan's
    /// packed length; degrees below the spin are returned as zero.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is not 1 or 2, exact-analysis
    /// sampling is insufficient, a buffer length differs from the plan, or
    /// the workspace is incompatible.
    pub fn analysis_spin_into(
        &self,
        first_component: &[f64],
        second_component: &[f64],
        e_alm: &mut [Complex64],
        b_alm: &mut [Complex64],
        workspace: &mut ShtWorkspace,
    ) -> Result<(), ShtError> {
        let spin = self.supported_spin()?;
        self.check_analysis_sampling()?;
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
        check_len("B coefficient output", self.harmonics.nalm(), b_alm.len())?;
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

        e_alm.fill(Complex64::new(0.0, 0.0));
        b_alm.fill(Complex64::new(0.0, 0.0));
        self.prepare_spin_latitudes(workspace, spin);
        let arch = pulp::Arch::new();
        let order_count = self.harmonics.mmax() + 1;

        #[cfg(feature = "parallel")]
        {
            let worker_count = workspace.recurrence_workers.len().min(order_count);
            let use_parallel = rayon::current_num_threads() > 1 && worker_count > 1;
            if use_parallel {
                self.analyze_spin_order_groups_parallel(
                    &workspace.spectra,
                    &workspace.spin_spectra,
                    nphi,
                    spectrum_len,
                    self.grid.phi0(),
                    self.grid.ring_weights(),
                    &workspace.theta_sin,
                    &workspace.theta_cos,
                    &workspace.ring_order_limits,
                    arch,
                    0,
                    order_count,
                    e_alm,
                    b_alm,
                    &mut workspace.recurrence_workers[..worker_count],
                )?;
            } else {
                self.analyze_spin_order_group(
                    &workspace.spectra,
                    &workspace.spin_spectra,
                    nphi,
                    spectrum_len,
                    self.grid.phi0(),
                    self.grid.ring_weights(),
                    &workspace.theta_sin,
                    &workspace.theta_cos,
                    &workspace.ring_order_limits,
                    arch,
                    0,
                    order_count,
                    e_alm,
                    b_alm,
                    &mut workspace.recurrence_workers[0],
                )?;
            }
        }
        #[cfg(not(feature = "parallel"))]
        self.analyze_spin_order_group(
            &workspace.spectra,
            &workspace.spin_spectra,
            nphi,
            spectrum_len,
            self.grid.phi0(),
            self.grid.ring_weights(),
            &workspace.theta_sin,
            &workspace.theta_cos,
            &workspace.ring_order_limits,
            arch,
            0,
            order_count,
            e_alm,
            b_alm,
            &mut workspace.recurrence_workers[0],
        )?;

        for degree in 0..=self.harmonics.lmax() {
            let index = self
                .harmonics
                .index(degree, 0)
                .expect("order zero contains every degree");
            e_alm[index].im = 0.0;
            b_alm[index].im = 0.0;
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The shared STANDARD, GRAD_ONLY and DERIV1 driver receives two channels, outputs and recurrence mode."
    )]
    fn synthesis_spin_core(
        &self,
        e_alm: &[Complex64],
        b_alm: Option<&[Complex64]>,
        first_component: &mut [f64],
        second_component: &mut [f64],
        recurrence_spin: usize,
        derivative_norm: bool,
        workspace: &mut ShtWorkspace,
    ) -> Result<(), ShtError> {
        if !matches!(recurrence_spin, 1 | 2) {
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
        workspace.spectra.fill(Complex64::new(0.0, 0.0));
        workspace.spin_spectra.fill(Complex64::new(0.0, 0.0));
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
                    &workspace.theta_sin,
                    &workspace.theta_cos,
                    &workspace.ring_order_limits,
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
            &workspace.theta_sin,
            &workspace.theta_cos,
            &workspace.ring_order_limits,
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
        e_alm: &[Complex64],
        b_alm: Option<&[Complex64]>,
        spin: usize,
        derivative_norm: bool,
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_start: usize,
        ring_end: usize,
        q_spectra: &mut [Complex64],
        u_spectra: &mut [Complex64],
        theta_sin: &[f64],
        theta_cos: &[f64],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace,
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let mmax = self.harmonics.mmax();
        debug_assert_eq!(ring_end - ring_start, theta_sin.len());
        debug_assert_eq!(q_spectra.len(), (ring_end - ring_start) * spectrum_len);
        debug_assert_eq!(u_spectra.len(), q_spectra.len());

        for m in 0..=mmax {
            let initial = fill_spin_factors(
                spin,
                m,
                lmax,
                &mut recurrence.spin_alpha,
                &mut recurrence.spin_coefficient_a,
                &mut recurrence.spin_coefficient_b,
            )?;
            let phase = Complex64::from_polar(1.0, m as f64 * phi0);
            for_each_spin_lm_dispatched(
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
                        .unwrap_or_default();
                    let alpha_norm = recurrence.spin_alpha[degree]
                        * spin_normalization(spin, degree, derivative_norm);
                    for lane in 0..plus.len() {
                        let group_ring = group_ring_start + lane;
                        let global_ring = ring_start + group_ring;
                        if m > ring_order_limits[global_ring] {
                            continue;
                        }
                        let plus_value = scaled_basis_value(plus[lane], plus_exp[lane], alpha_norm);
                        let minus_value =
                            scaled_basis_value(minus[lane], minus_exp[lane], alpha_norm);
                        let even_basis = plus_value + minus_value;
                        let odd_basis = plus_value - minus_value;
                        let q_value = e_coefficient * even_basis
                            + Complex64::new(0.0, -1.0) * b_coefficient * odd_basis;
                        let u_value = Complex64::new(0.0, 1.0) * e_coefficient * odd_basis
                            + b_coefficient * even_basis;
                        let spectrum_index = group_ring * spectrum_len + m;
                        let phased_q = q_value * phase;
                        let phased_u = u_value * phase;
                        if nphi.is_multiple_of(2) && m == nphi / 2 {
                            q_spectra[spectrum_index] +=
                                Complex64::new(2.0 * nphi as f64 * phased_q.re, 0.0);
                            u_spectra[spectrum_index] +=
                                Complex64::new(2.0 * nphi as f64 * phased_u.re, 0.0);
                        } else {
                            q_spectra[spectrum_index] += phased_q * nphi as f64;
                            u_spectra[spectrum_index] += phased_u * nphi as f64;
                        }
                    }
                    Ok(())
                },
            )?;
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The analysis group shares spectra and grid metadata and owns two coefficient slices plus worker scratch."
    )]
    fn analyze_spin_order_group(
        &self,
        q_spectra: &[Complex64],
        u_spectra: &[Complex64],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_weights: &[f64],
        theta_sin: &[f64],
        theta_cos: &[f64],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        m_start: usize,
        m_end: usize,
        e_alm: &mut [Complex64],
        b_alm: &mut [Complex64],
        recurrence: &mut RecurrenceWorkspace,
    ) -> Result<(), ShtError> {
        let spin = self.harmonics.spin();
        let lmax = self.harmonics.lmax();
        let mut coefficient_start = 0;
        for m in m_start..m_end {
            let coefficient_count = lmax - m + 1;
            let e_coefficients =
                &mut e_alm[coefficient_start..coefficient_start + coefficient_count];
            let b_coefficients =
                &mut b_alm[coefficient_start..coefficient_start + coefficient_count];
            let initial = fill_spin_factors(
                spin,
                m,
                lmax,
                &mut recurrence.spin_alpha,
                &mut recurrence.spin_coefficient_a,
                &mut recurrence.spin_coefficient_b,
            )?;
            let phase = Complex64::from_polar(1.0, -(m as f64 * phi0));
            for_each_spin_lm_dispatched(
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
                |degree, ring_start, plus, plus_exp, minus, minus_exp| {
                    let alpha_norm =
                        recurrence.spin_alpha[degree] * spin_normalization(spin, degree, false);
                    let coefficient = &mut e_coefficients[degree - m];
                    let b_coefficient = &mut b_coefficients[degree - m];
                    for lane in 0..plus.len() {
                        let ring = ring_start + lane;
                        if m > ring_order_limits[ring] {
                            continue;
                        }
                        let plus_value = scaled_basis_value(plus[lane], plus_exp[lane], alpha_norm);
                        let minus_value =
                            scaled_basis_value(minus[lane], minus_exp[lane], alpha_norm);
                        let even_basis = plus_value + minus_value;
                        let odd_basis = plus_value - minus_value;
                        let spectrum_start = ring * spectrum_len;
                        let q_fourier = q_spectra[spectrum_start + m] * phase / nphi as f64;
                        let u_fourier = u_spectra[spectrum_start + m] * phase / nphi as f64;
                        let weight = ring_weights[ring];
                        *coefficient += (q_fourier * even_basis
                            + Complex64::new(0.0, -1.0) * u_fourier * odd_basis)
                            * weight;
                        *b_coefficient += (Complex64::new(0.0, 1.0) * q_fourier * odd_basis
                            + u_fourier * even_basis)
                            * weight;
                    }
                    Ok(())
                },
            )?;
            coefficient_start += coefficient_count;
        }
        debug_assert_eq!(coefficient_start, e_alm.len());
        debug_assert_eq!(coefficient_start, b_alm.len());
        Ok(())
    }

    #[cfg(feature = "parallel")]
    #[expect(
        clippy::too_many_arguments,
        reason = "Recursive analysis splits coefficient outputs and recurrence scratch while sharing immutable grid inputs."
    )]
    fn analyze_spin_order_groups_parallel(
        &self,
        q_spectra: &[Complex64],
        u_spectra: &[Complex64],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_weights: &[f64],
        theta_sin: &[f64],
        theta_cos: &[f64],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        m_start: usize,
        m_end: usize,
        e_alm: &mut [Complex64],
        b_alm: &mut [Complex64],
        recurrence: &mut [RecurrenceWorkspace],
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
        let left_order_count = order_count.div_ceil(2);
        let m_mid = m_start + left_order_count;
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
        let (left_b, right_b) = b_alm.split_at_mut(coefficient_split);
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

    fn prepare_spin_latitudes(&self, workspace: &mut ShtWorkspace, spin: usize) {
        for (ring, &theta) in self.grid.theta().iter().enumerate() {
            (workspace.theta_sin[ring], workspace.theta_cos[ring]) = theta.sin_cos();
            workspace.ring_order_limits[ring] = polar_order_limit(
                self.harmonics.lmax(),
                spin,
                workspace.theta_sin[ring],
                workspace.theta_cos[ring],
            );
        }
    }

    fn supported_spin(&self) -> Result<usize, ShtError> {
        match self.harmonics.spin() {
            spin @ (1 | 2) => Ok(spin),
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

    fn check_analysis_sampling(&self) -> Result<(), ShtError> {
        if self.grid.ntheta() <= self.harmonics.lmax() {
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
mod tests {
    use super::*;

    use crate::{GridKind, GridSpec, HarmonicSpec};

    fn make_plan(spin: usize, lmax: usize, mmax: usize, ntheta: usize, nphi: usize) -> ShtPlan {
        ShtPlan::new(
            GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap(),
            HarmonicSpec::new(lmax, mmax, spin).unwrap(),
        )
        .unwrap()
    }

    fn coefficients(plan: &ShtPlan) -> (Vec<Complex64>, Vec<Complex64>) {
        let spec = plan.harmonics();
        let mut e = vec![Complex64::new(0.0, 0.0); spec.nalm()];
        let mut b = vec![Complex64::new(0.0, 0.0); spec.nalm()];
        for m in 0..=spec.mmax() {
            for l in spec.spin().max(m)..=spec.lmax() {
                let index = spec.index(l, m).unwrap();
                e[index] = Complex64::new(
                    0.03 * (1 + l + 2 * m) as f64,
                    if m == 0 { 0.0 } else { 0.017 * (l - m) as f64 },
                );
                b[index] = Complex64::new(
                    -0.02 * (2 + l + m) as f64,
                    if m == 0 {
                        0.0
                    } else {
                        0.013 * (2 * l - m) as f64
                    },
                );
            }
        }
        (e, b)
    }

    fn arbitrary_maps(plan: &ShtPlan) -> (Vec<f64>, Vec<f64>) {
        let mut first = Vec::with_capacity(plan.sample_count);
        let mut second = Vec::with_capacity(plan.sample_count);
        for i in 0..plan.grid.ntheta() {
            for j in 0..plan.grid.nphi() {
                first.push(
                    (0.17 * i as f64 + 0.23 * j as f64).sin()
                        + 0.2 * (0.11 * i as f64 - 0.31 * j as f64).cos(),
                );
                second.push(
                    (0.19 * i as f64 - 0.13 * j as f64).cos()
                        - 0.15 * (0.29 * i as f64 + 0.07 * j as f64).sin(),
                );
            }
        }
        (first, second)
    }

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.17e}, expected={expected:.17e}, error={:.3e}",
            (actual - expected).abs()
        );
    }

    fn assert_complex_close(actual: Complex64, expected: Complex64, tolerance: f64) {
        assert!(
            (actual - expected).norm() <= tolerance,
            "actual={actual:?}, expected={expected:?}, error={:.3e}",
            (actual - expected).norm()
        );
    }

    #[test]
    #[expect(
        clippy::excessive_precision,
        reason = "These decimal fixtures retain the printed DUCC reference values."
    )]
    fn spin_one_gl_matches_ducc_0410_and_weighted_analysis() {
        // Static outputs from ducc0==0.41.0, GL 3x7, phi0=0.37, one thread.
        let plan = make_plan(1, 2, 2, 3, 7);
        let (e, b) = coefficients(&plan);
        let mut workspace = plan.workspace().unwrap();
        let mut first = vec![0.0; plan.sample_count];
        let mut second = vec![0.0; plan.sample_count];
        plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
            .unwrap();
        for (index, expected) in [
            (0, -0.074_965_830_511_175_502),
            (2, 0.094_092_382_651_284_556),
            (10, -0.078_411_578_146_920_513),
            (20, 0.073_425_512_817_354_946),
        ] {
            assert_close(first[index], expected, 4e-14);
        }
        for (index, expected) in [
            (0, 0.053_094_402_343_952_324),
            (2, 0.231_182_369_405_343_7),
            (10, 0.107_080_931_286_266_61),
            (20, 0.051_955_952_175_169_004),
        ] {
            assert_close(second[index], expected, 4e-14);
        }

        let (input_first, input_second) = arbitrary_maps(&plan);
        let mut recovered_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut recovered_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.analysis_spin_into(
            &input_first,
            &input_second,
            &mut recovered_e,
            &mut recovered_b,
            &mut workspace,
        )
        .unwrap();
        let expected_e = [
            Complex64::new(0.0, 0.0),
            Complex64::new(-2.719_595_782_603_398_5, 0.0),
            Complex64::new(0.296_440_629_674_238_01, 0.0),
            Complex64::new(0.116_869_914_995_230_15, -0.085_692_903_571_819_773),
            Complex64::new(-0.043_265_294_485_276_44, 0.153_766_887_626_409_58),
            Complex64::new(-0.070_050_301_995_330_216, 0.017_601_979_788_444_126),
        ];
        let expected_b = [
            Complex64::new(0.0, 0.0),
            Complex64::new(-3.005_376_860_452_039_4, 0.0),
            Complex64::new(-0.002_667_162_341_653_368_6, 0.0),
            Complex64::new(0.336_942_654_702_059_75, 0.250_544_257_552_348_33),
            Complex64::new(0.026_827_651_643_820_543, -0.065_628_053_641_931_43),
            Complex64::new(-0.185_782_400_932_178_02, -0.085_451_169_345_024_267),
        ];
        for ((&actual_e, &actual_b), (&expected_e, &expected_b)) in recovered_e
            .iter()
            .zip(&recovered_b)
            .zip(expected_e.iter().zip(&expected_b))
        {
            assert_complex_close(actual_e, expected_e, 8e-13);
            assert_complex_close(actual_b, expected_b, 8e-13);
        }
    }

    #[test]
    #[expect(
        clippy::excessive_precision,
        reason = "These decimal fixtures retain the printed DUCC reference values."
    )]
    fn spin_two_gl_matches_ducc_0410_and_weighted_analysis() {
        // Static outputs from ducc0==0.41.0, GL 3x7, phi0=0.37, one thread.
        let plan = make_plan(2, 2, 2, 3, 7);
        let (e, b) = coefficients(&plan);
        let mut workspace = plan.workspace().unwrap();
        let mut first = vec![0.0; plan.sample_count];
        let mut second = vec![0.0; plan.sample_count];
        plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
            .unwrap();
        for (index, expected) in [
            (0, -0.103_404_620_030_628_45),
            (2, 0.048_002_344_434_381_797),
            (10, -0.070_686_531_243_965_053),
            (20, 0.000_284_063_513_471_682_9),
        ] {
            assert_close(first[index], expected, 5e-14);
        }
        for (index, expected) in [
            (0, 0.196_243_499_903_941_38),
            (2, -0.079_139_202_179_877_077),
            (10, 0.063_750_908_470_770_57),
            (20, 0.075_170_514_907_193_37),
        ] {
            assert_close(second[index], expected, 5e-14);
        }

        let (input_first, input_second) = arbitrary_maps(&plan);
        let mut recovered_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut recovered_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.analysis_spin_into(
            &input_first,
            &input_second,
            &mut recovered_e,
            &mut recovered_b,
            &mut workspace,
        )
        .unwrap();
        let expected_e = [
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(-2.550_847_468_393_931_3, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.122_163_424_850_977_95, -0.080_290_382_013_937_858),
            Complex64::new(0.083_928_217_095_939_02, -0.152_913_590_740_669_4),
        ];
        let expected_b = [
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(-2.819_769_059_231_437_5, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.347_681_620_237_764_94, 0.249_694_162_014_288_01),
            Complex64::new(0.026_574_652_518_373_161, 0.058_868_006_368_103_853),
        ];
        for ((&actual_e, &actual_b), (&expected_e, &expected_b)) in recovered_e
            .iter()
            .zip(&recovered_b)
            .zip(expected_e.iter().zip(&expected_b))
        {
            assert_complex_close(actual_e, expected_e, 8e-13);
            assert_complex_close(actual_b, expected_b, 8e-13);
        }
    }

    #[test]
    fn spin_gradient_and_derivative_modes_match_their_definitions() {
        for spin in [1, 2] {
            let plan = make_plan(spin, spin, spin, 5, 7);
            let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            e[plan.harmonics.index(spin, 0).unwrap()] = Complex64::new(1.0, 0.0);
            let b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            let mut workspace = plan.workspace().unwrap();
            let mut standard_first = vec![0.0; plan.sample_count];
            let mut standard_second = vec![0.0; plan.sample_count];
            let mut gradient_first = vec![0.0; plan.sample_count];
            let mut gradient_second = vec![0.0; plan.sample_count];
            plan.synthesis_spin_into(
                &e,
                &b,
                &mut standard_first,
                &mut standard_second,
                &mut workspace,
            )
            .unwrap();
            plan.synthesis_gradient_into(
                &e,
                &mut gradient_first,
                &mut gradient_second,
                &mut workspace,
            )
            .unwrap();
            for (&actual, &expected) in gradient_first.iter().zip(&standard_first) {
                assert_close(actual, expected, 2e-14);
            }
            for (&actual, &expected) in gradient_second.iter().zip(&standard_second) {
                assert_close(actual, expected, 2e-14);
            }

            let expected_factor = if spin == 1 {
                (3.0 / (8.0 * std::f64::consts::PI)).sqrt()
            } else {
                (30.0 / (64.0 * std::f64::consts::PI)).sqrt()
            };
            for ring in 0..plan.grid.ntheta() {
                let theta = plan.grid.theta()[ring];
                let expected = -expected_factor * theta.sin().powi(spin as i32);
                assert_close(standard_first[ring * plan.grid.nphi()], expected, 3e-14);
                assert_close(standard_second[ring * plan.grid.nphi()], 0.0, 3e-14);
            }
        }

        let plan = ShtPlan::new(
            GridSpec::new(GridKind::GaussLegendre, 4, 7, 0.21).unwrap(),
            HarmonicSpec::scalar(1, 1).unwrap(),
        )
        .unwrap();
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        alm[plan.harmonics.index(1, 0).unwrap()] = Complex64::new(1.0, 0.0);
        let mut theta_derivative = vec![0.0; plan.sample_count];
        let mut phi_derivative = vec![0.0; plan.sample_count];
        plan.synthesis_deriv1_into(
            &alm,
            &mut theta_derivative,
            &mut phi_derivative,
            &mut plan.workspace().unwrap(),
        )
        .unwrap();
        let norm = (3.0 / (4.0 * std::f64::consts::PI)).sqrt();
        for ring in 0..plan.grid.ntheta() {
            let expected = -norm * plan.grid.theta()[ring].sin();
            assert_close(theta_derivative[ring * plan.grid.nphi()], expected, 3e-14);
            assert_close(phi_derivative[ring * plan.grid.nphi()], 0.0, 3e-14);
        }

        alm.fill(Complex64::new(0.0, 0.0));
        alm[plan.harmonics.index(1, 1).unwrap()] = Complex64::new(1.0, 0.0);
        plan.synthesis_deriv1_into(
            &alm,
            &mut theta_derivative,
            &mut phi_derivative,
            &mut plan.workspace().unwrap(),
        )
        .unwrap();
        let derivative_norm = (3.0 / (2.0 * std::f64::consts::PI)).sqrt();
        for ring in 0..plan.grid.ntheta() {
            let theta = plan.grid.theta()[ring];
            for longitude in 0..plan.grid.nphi() {
                let phi = plan.grid.phi0()
                    + 2.0 * std::f64::consts::PI * longitude as f64 / plan.grid.nphi() as f64;
                let index = ring * plan.grid.nphi() + longitude;
                assert_close(
                    theta_derivative[index],
                    -derivative_norm * theta.cos() * phi.cos(),
                    5e-14,
                );
                assert_close(phi_derivative[index], derivative_norm * phi.sin(), 5e-14);
            }
        }

        let constant_plan = ShtPlan::new(
            GridSpec::new(GridKind::GaussLegendre, 2, 3, 0.0).unwrap(),
            HarmonicSpec::scalar(0, 0).unwrap(),
        )
        .unwrap();
        let mut zero_derivatives = [1.0; 6];
        let mut zero_phi = [1.0; 6];
        constant_plan
            .synthesis_deriv1_into(
                &[Complex64::new(2.0, 0.0)],
                &mut zero_derivatives,
                &mut zero_phi,
                &mut constant_plan.workspace().unwrap(),
            )
            .unwrap();
        assert_eq!(zero_derivatives, [0.0; 6]);
        assert_eq!(zero_phi, [0.0; 6]);
    }

    #[test]
    fn spin_analysis_accepts_minimum_odd_even_longitudes_and_rectangular_layouts() {
        for spin in [1, 2] {
            for nphi in [3, 4] {
                let plan = make_plan(spin, 3, 1, 4, nphi);
                let (e, b) = coefficients(&plan);
                let mut first = vec![0.0; plan.sample_count];
                let mut second = vec![0.0; plan.sample_count];
                let mut workspace = plan.workspace().unwrap();
                plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
                    .unwrap();
                let mut recovered_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
                let mut recovered_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
                plan.analysis_spin_into(
                    &first,
                    &second,
                    &mut recovered_e,
                    &mut recovered_b,
                    &mut workspace,
                )
                .unwrap();
                for (&actual, &expected) in recovered_e.iter().zip(&e) {
                    assert_complex_close(actual, expected, 8e-13);
                }
                for (&actual, &expected) in recovered_b.iter().zip(&b) {
                    assert_complex_close(actual, expected, 8e-13);
                }
            }
        }
    }

    #[test]
    fn spin_operations_reject_unsupported_spin_and_invalid_coefficients() {
        let unsupported = make_plan(3, 3, 1, 4, 5);
        let mut unsupported_workspace = unsupported.workspace().unwrap();
        let unsupported_coefficients = vec![Complex64::new(0.0, 0.0); unsupported.harmonics.nalm()];
        let mut unsupported_first = vec![0.0; unsupported.sample_count];
        let mut unsupported_second = vec![0.0; unsupported.sample_count];
        let mut unsupported_e = vec![Complex64::new(0.0, 0.0); unsupported.harmonics.nalm()];
        let mut unsupported_b = vec![Complex64::new(0.0, 0.0); unsupported.harmonics.nalm()];
        assert_eq!(
            unsupported.synthesis_spin_into(
                &unsupported_coefficients,
                &unsupported_coefficients,
                &mut unsupported_first,
                &mut unsupported_second,
                &mut unsupported_workspace,
            ),
            Err(ShtError::UnsupportedSpin { spin: 3 })
        );
        assert_eq!(
            unsupported.analysis_spin_into(
                &unsupported_first,
                &unsupported_second,
                &mut unsupported_e,
                &mut unsupported_b,
                &mut unsupported_workspace,
            ),
            Err(ShtError::UnsupportedSpin { spin: 3 })
        );

        let plan = make_plan(1, 1, 1, 2, 3);
        let mut workspace = plan.workspace().unwrap();
        let mut coefficients = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut first = vec![0.0; plan.sample_count];
        let mut second = vec![0.0; plan.sample_count];
        coefficients[plan.harmonics.index(1, 0).unwrap()].im = 1e-12;
        assert_eq!(
            plan.synthesis_gradient_into(&coefficients, &mut first, &mut second, &mut workspace,),
            Err(ShtError::NonRealMZero { degree: 1 })
        );

        coefficients.fill(Complex64::new(0.0, 0.0));
        coefficients[plan.harmonics.index(0, 0).unwrap()].re = 1.0;
        let zero_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        assert_eq!(
            plan.synthesis_spin_into(
                &coefficients,
                &zero_b,
                &mut first,
                &mut second,
                &mut workspace,
            ),
            Err(ShtError::NonzeroBelowSpin {
                spin: 1,
                degree: 0,
                order: 0
            })
        );
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn spin_transform_matches_between_one_and_two_rayon_threads() {
        let plan = make_plan(2, 4, 4, 131, 11);
        let (e, b) = coefficients(&plan);
        let one_thread = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let two_threads = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let mut one_workspace = one_thread.install(|| plan.workspace()).unwrap();
        let mut two_workspace = two_threads.install(|| plan.workspace()).unwrap();
        let mut one_first = vec![0.0; plan.sample_count];
        let mut one_second = vec![0.0; plan.sample_count];
        let mut two_first = vec![0.0; plan.sample_count];
        let mut two_second = vec![0.0; plan.sample_count];
        one_thread
            .install(|| {
                plan.synthesis_spin_into(
                    &e,
                    &b,
                    &mut one_first,
                    &mut one_second,
                    &mut one_workspace,
                )
            })
            .unwrap();
        two_threads
            .install(|| {
                plan.synthesis_spin_into(
                    &e,
                    &b,
                    &mut two_first,
                    &mut two_second,
                    &mut two_workspace,
                )
            })
            .unwrap();
        for (&actual, &expected) in two_first.iter().zip(&one_first) {
            assert_close(actual, expected, 3e-14);
        }
        for (&actual, &expected) in two_second.iter().zip(&one_second) {
            assert_close(actual, expected, 3e-14);
        }

        let mut one_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut one_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut two_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut two_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        one_thread
            .install(|| {
                plan.analysis_spin_into(
                    &one_first,
                    &one_second,
                    &mut one_e,
                    &mut one_b,
                    &mut one_workspace,
                )
            })
            .unwrap();
        two_threads
            .install(|| {
                plan.analysis_spin_into(
                    &one_first,
                    &one_second,
                    &mut two_e,
                    &mut two_b,
                    &mut two_workspace,
                )
            })
            .unwrap();
        for (&actual, &expected) in two_e.iter().zip(&one_e) {
            assert_complex_close(actual, expected, 3e-14);
        }
        for (&actual, &expected) in two_b.iter().zip(&one_b) {
            assert_complex_close(actual, expected, 3e-14);
        }
    }

    #[test]
    #[expect(
        clippy::excessive_precision,
        reason = "These high-degree decimal fixtures retain the printed DUCC reference values."
    )]
    fn spin_t255_gl_matches_ducc_samples_and_coefficients() {
        // Static outputs from ducc0==0.41.0, GL 256x513, phi0=0.37, one thread.
        for spin in [1, 2] {
            let plan = make_plan(spin, 255, 255, 256, 513);
            let (mut e, mut b) = (
                vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()],
                vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()],
            );
            for m in 0..=255 {
                for l in spin.max(m)..=255 {
                    let index = plan.harmonics.index(l, m).unwrap();
                    let real_e = ((7 * l + 11 * m) % 19) as i32 - 9;
                    let imag_e = ((13 * l + 5 * m) % 23) as i32 - 11;
                    let real_b = ((5 * l + 3 * m) % 17) as i32 - 8;
                    let imag_b = ((3 * l + 7 * m) % 13) as i32 - 6;
                    e[index] = Complex64::new(
                        1e-5 * f64::from(real_e),
                        if m == 0 {
                            0.0
                        } else {
                            1e-5 * f64::from(imag_e)
                        },
                    );
                    b[index] = Complex64::new(
                        0.7e-5 * f64::from(real_b),
                        if m == 0 {
                            0.0
                        } else {
                            0.9e-5 * f64::from(imag_b)
                        },
                    );
                }
            }
            let mut workspace = plan.workspace().unwrap();
            let mut first = vec![0.0; plan.sample_count];
            let mut second = vec![0.0; plan.sample_count];
            plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
                .unwrap();
            let sample_indices = [0, 626, 127 * 513 + 256, 255 * 513 + 512];
            let (expected_first, expected_second) = if spin == 1 {
                (
                    [
                        -0.000_171_618_524_796_108,
                        0.000_240_781_297_514_088_88,
                        0.002_306_027_070_480_382_8,
                        0.000_643_793_990_142_110_37,
                    ],
                    [
                        0.000_365_376_736_155_249_38,
                        -0.000_105_662_296_816_821_9,
                        0.000_513_609_667_700_186_31,
                        0.000_393_166_009_911_476_18,
                    ],
                )
            } else {
                (
                    [
                        -0.000_191_889_393_857_919_31,
                        -0.000_072_843_722_054_503_326,
                        0.001_119_711_711_133_881_2,
                        -0.001_035_586_726_648_950_9,
                    ],
                    [
                        0.000_039_856_788_143_593_73,
                        -0.000_631_962_191_664_244_82,
                        -0.002_267_234_955_300_86,
                        0.000_809_307_858_647_371_52,
                    ],
                )
            };
            for (&index, &expected) in sample_indices.iter().zip(&expected_first) {
                assert_close(first[index], expected, 2e-12);
            }
            for (&index, &expected_second) in sample_indices.iter().zip(&expected_second) {
                assert_close(second[index], expected_second, 2e-12);
            }

            e.fill(Complex64::new(0.0, 0.0));
            b.fill(Complex64::new(0.0, 0.0));
            e[plan.harmonics.index(255, 255).unwrap()] = Complex64::new(1.0, 0.0);
            plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
                .unwrap();
            let sectoral_indices = [
                0,
                513,
                127 * 513,
                127 * 513 + 128,
                128 * 513 + 256,
                255 * 513 + 512,
            ];
            let (sectoral_first, sectoral_second) = if spin == 1 {
                (
                    [
                        0.0,
                        0.0,
                        -0.014_502_544_345_968_933,
                        0.009_150_897_969_309_892_6,
                        -0.001_620_795_756_960_675_1,
                        0.0,
                    ],
                    [
                        0.0,
                        0.0,
                        0.242_923_273_826_296_67,
                        -1.853_213_608_652_077_8,
                        2.365_851_500_895_088_7,
                        0.0,
                    ],
                )
            } else {
                (
                    [
                        0.0,
                        0.0,
                        2.354_452_652_343_165_9,
                        -1.485_625_934_400_402_1,
                        -0.263_132_232_375_715_2,
                        0.0,
                    ],
                    [
                        0.0,
                        0.0,
                        -0.002_957_920_717_880_142_2,
                        0.022_565_392_114_750_955,
                        0.028_807_454_550_153_003,
                        0.0,
                    ],
                )
            };
            for ((&index, &expected_first), &expected_second) in sectoral_indices
                .iter()
                .zip(&sectoral_first)
                .zip(&sectoral_second)
            {
                assert_close(first[index], expected_first, 3e-11);
                assert_close(second[index], expected_second, 3e-11);
            }

            let mut map_first = vec![0.0; plan.sample_count];
            let mut map_second = vec![0.0; plan.sample_count];
            for i in 0..256 {
                for j in 0..513 {
                    map_first[i * 513 + j] = (0.013 * i as f64 + 0.017 * j as f64).sin()
                        + 0.3 * (0.019 * i as f64 - 0.011 * j as f64).cos();
                    map_second[i * 513 + j] = (0.007 * i as f64 - 0.023 * j as f64).cos()
                        - 0.2 * (0.031 * i as f64 + 0.003 * j as f64).sin();
                }
            }
            let mut recovered_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            let mut recovered_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            plan.analysis_spin_into(
                &map_first,
                &map_second,
                &mut recovered_e,
                &mut recovered_b,
                &mut workspace,
            )
            .unwrap();
            let coefficients = [(spin, 0), (255, 0), (128, 1), (255, 128), (255, 255)];
            let (expected_e, expected_b) = if spin == 1 {
                (
                    [
                        Complex64::new(-0.235_214_804_154_751_46, 0.0),
                        Complex64::new(4.730_955_291_022_578_7e-5, 0.0),
                        Complex64::new(0.000_364_309_409_390_762_61, -0.000_310_485_887_894_616_06),
                        Complex64::new(-4.874_854_665_548_187_6e-5, -3.189_712_094_041_813_8e-5),
                        Complex64::new(9.320_378_481_313_17e-5, 0.000_822_533_069_129_493_44),
                    ],
                    [
                        Complex64::new(-0.133_655_294_352_013_43, 0.0),
                        Complex64::new(0.000_214_779_425_271_175_49, 0.0),
                        Complex64::new(-0.001_140_372_173_998_606_1, -0.001_170_201_806_136_655_1),
                        Complex64::new(5.480_444_806_653_299e-5, 3.575_031_575_613_239e-5),
                        Complex64::new(-0.000_235_951_559_473_843_78, -0.002_141_500_278_908_898_3),
                    ],
                )
            } else {
                (
                    [
                        Complex64::new(-0.237_673_805_510_415_23, 0.0),
                        Complex64::new(-0.000_486_172_907_263_484_44, 0.0),
                        Complex64::new(0.001_050_541_489_860_348_4, -0.001_127_992_219_361_837_9),
                        Complex64::new(-3.476_958_601_947_758e-5, 5.540_041_297_517_928e-5),
                        Complex64::new(0.002_141_824_249_688_625_3, -0.000_232_315_219_513_176_68),
                    ],
                    [
                        Complex64::new(-0.197_211_989_032_294_25, 0.0),
                        Complex64::new(0.000_119_337_432_581_061_4, 0.0),
                        Complex64::new(
                            -0.000_345_143_456_478_464_04,
                            -0.000_663_935_366_580_957_58,
                        ),
                        Complex64::new(-3.440_626_319_093_324e-5, 4.723_233_723_130_058e-5),
                        Complex64::new(0.000_822_384_251_765_275_73, -9.439_566_581_699_641e-5),
                    ],
                )
            };
            for (slot, &(degree, order)) in coefficients.iter().enumerate() {
                let index = plan.harmonics.index(degree, order).unwrap();
                assert_complex_close(recovered_e[index], expected_e[slot], 2e-10);
                assert_complex_close(recovered_b[index], expected_b[slot], 2e-10);
            }
        }
    }
}
