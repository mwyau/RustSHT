// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex64;

use crate::error::ShtError;
use crate::legendre::{fill_epsilon, for_each_y_lm, multiply_complex_scaled, polar_order_limit};
use crate::plan::ShtPlan;
use crate::workspace::ShtWorkspace;

impl ShtPlan {
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
        alm: &[Complex64],
        map: &mut [f64],
        workspace: &mut ShtWorkspace,
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
        let mmax = self.harmonics.mmax();
        let phi0 = self.grid.phi0();
        let order_offsets = &self.harmonics;
        let (spectra, row, epsilon, fft_workspace) = (
            &mut workspace.spectra,
            &mut workspace.row,
            &mut workspace.epsilon,
            &mut workspace.fft,
        );

        spectra.fill(Complex64::new(0.0, 0.0));
        for m in 0..=mmax {
            fill_epsilon(m, lmax, epsilon);
            let coefficient_start = order_offsets
                .index(m, m)
                .expect("every packed order starts at its diagonal degree");
            let phase = if m == 0 {
                Complex64::new(1.0, 0.0)
            } else {
                Complex64::from_polar(1.0, m as f64 * phi0)
            };
            for ring in 0..ntheta {
                let (sin_theta, cos_theta) = self.grid.theta()[ring].sin_cos();
                if m > polar_order_limit(lmax, 0, sin_theta, cos_theta) {
                    continue;
                }
                let mut mode = Complex64::new(0.0, 0.0);
                for_each_y_lm(
                    m,
                    lmax,
                    sin_theta,
                    cos_theta,
                    self.sectoral_norms[m],
                    epsilon,
                    |l, mantissa, exponent| {
                        let coefficient = alm[coefficient_start + (l - m)];
                        mode += multiply_complex_scaled(coefficient, mantissa, exponent);
                    },
                )?;

                let phased = mode * phase;
                let spectrum_index = ring * spectrum_len + m;
                if nphi.is_multiple_of(2) && m == nphi / 2 {
                    spectra[spectrum_index] = Complex64::new(2.0 * nphi as f64 * phased.re, 0.0);
                } else {
                    spectra[spectrum_index] = phased * nphi as f64;
                }
            }
        }

        for ring in 0..ntheta {
            let start = ring * spectrum_len;
            let ring_spectrum = &spectra[start..start + spectrum_len];
            self.fft.inverse(ring_spectrum, row, fft_workspace)?;
            let output_start = ring * nphi;
            map[output_start..output_start + nphi].copy_from_slice(row);
        }
        Ok(())
    }

    /// Analyzes GL samples into real-field scalar harmonic coefficients.
    ///
    /// Exact analysis requires `ntheta >= lmax + 1` and
    /// `mmax <= (nphi - 1) / 2`.
    ///
    /// # Errors
    ///
    /// Returns an error for non-scalar spin, insufficient GL sampling, a
    /// buffer length mismatch, or an incompatible workspace.
    pub fn analysis_into(
        &self,
        map: &[f64],
        alm: &mut [Complex64],
        workspace: &mut ShtWorkspace,
    ) -> Result<(), ShtError> {
        self.check_scalar_spin()?;
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
        check_len("map input", self.sample_count, map.len())?;
        check_len("coefficient output", self.harmonics.nalm(), alm.len())?;
        self.check_workspace(workspace)?;

        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        let (spectra, epsilon, fft_workspace) = (
            &mut workspace.spectra,
            &mut workspace.epsilon,
            &mut workspace.fft,
        );
        for ring in 0..ntheta {
            let input_start = ring * nphi;
            let spectrum_start = ring * spectrum_len;
            self.fft.forward(
                &map[input_start..input_start + nphi],
                &mut spectra[spectrum_start..spectrum_start + spectrum_len],
                fft_workspace,
            )?;
        }

        alm.fill(Complex64::new(0.0, 0.0));
        let lmax = self.harmonics.lmax();
        let mmax = self.harmonics.mmax();
        let phi0 = self.grid.phi0();
        for m in 0..=mmax {
            fill_epsilon(m, lmax, epsilon);
            let coefficient_start = self
                .harmonics
                .index(m, m)
                .expect("every packed order starts at its diagonal degree");
            let phase = if m == 0 {
                Complex64::new(1.0, 0.0)
            } else {
                Complex64::from_polar(1.0, -(m as f64 * phi0))
            };
            for (ring, (&theta, &ring_weight)) in self
                .grid
                .theta()
                .iter()
                .zip(self.grid.ring_weights())
                .enumerate()
            {
                let (sin_theta, cos_theta) = theta.sin_cos();
                if m > polar_order_limit(lmax, 0, sin_theta, cos_theta) {
                    continue;
                }
                let spectrum_start = ring * spectrum_len;
                let fourier = spectra[spectrum_start + m] * phase / nphi as f64;
                let weighted = fourier * ring_weight;
                for_each_y_lm(
                    m,
                    lmax,
                    sin_theta,
                    cos_theta,
                    self.sectoral_norms[m],
                    epsilon,
                    |l, mantissa, exponent| {
                        alm[coefficient_start + (l - m)] +=
                            multiply_complex_scaled(weighted, mantissa, exponent);
                    },
                )?;
            }
        }
        for l in 0..=lmax {
            let index = self
                .harmonics
                .index(l, 0)
                .expect("order zero contains every degree");
            alm[index].im = 0.0;
        }
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

    fn check_workspace(&self, workspace: &ShtWorkspace) -> Result<(), ShtError> {
        check_len(
            "workspace Fourier spectra",
            self.spectrum_count,
            workspace.spectra.len(),
        )?;
        check_len(
            "workspace longitude row",
            self.grid.nphi(),
            workspace.row.len(),
        )?;
        check_len(
            "workspace scalar recurrence factors",
            self.recurrence_factor_count,
            workspace.epsilon.len(),
        )
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
    use std::f64::consts::PI;

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

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance * expected.abs().max(1.0),
            "actual={actual:.16e}, expected={expected:.16e}"
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

        let nyquist_analysis = make_plan(2, 2, 3, 4, 0.0);
        let mut nyquist_workspace = nyquist_analysis.workspace().unwrap();
        let nyquist_map = vec![0.0; nyquist_analysis.sample_count];
        let mut nyquist_alm = vec![Complex64::new(0.0, 0.0); nyquist_analysis.harmonics().nalm()];
        assert_eq!(
            nyquist_analysis.analysis_into(&nyquist_map, &mut nyquist_alm, &mut nyquist_workspace),
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
}
