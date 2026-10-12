// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::f64::consts::PI;
use std::sync::Arc;

use num_complex::Complex64;
use rustfft::{Fft, FftPlanner};

use crate::error::ShtError;

enum Engine {
    Even {
        forward: Arc<dyn Fft<f64>>,
        inverse: Arc<dyn Fft<f64>>,
        twiddles: Vec<Complex64>,
    },
    Odd {
        forward: Arc<dyn Fft<f64>>,
        inverse: Arc<dyn Fft<f64>>,
    },
}

/// Reusable real-input FFT plan for one longitude length.
pub(crate) struct RealFftPlan {
    n: usize,
    transform_len: usize,
    scratch_len: usize,
    engine: Engine,
}

/// Mutable storage for one real FFT execution.
pub(crate) struct RealFftWorkspace {
    transform: Vec<Complex64>,
    scratch: Vec<Complex64>,
}

impl RealFftPlan {
    pub(crate) fn new(n: usize) -> Result<Self, ShtError> {
        if n == 0 {
            return Err(ShtError::InvalidPhiCount);
        }

        let even = n.is_multiple_of(2);
        let transform_len = if even { n / 2 } else { n };
        let mut planner = FftPlanner::<f64>::new();
        let forward = planner.plan_fft_forward(transform_len);
        let inverse = planner.plan_fft_inverse(transform_len);
        let scratch_len = forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len());

        let engine = if even {
            let twiddle_count = transform_len
                .checked_add(1)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "real FFT twiddle count",
                })?;
            let mut twiddles = Vec::new();
            twiddles
                .try_reserve_exact(twiddle_count)
                .map_err(|_| ShtError::AllocationFailed {
                    array: "real FFT twiddle",
                })?;
            for k in 0..twiddle_count {
                let angle = -PI * (k as f64) / (transform_len as f64);
                let (sin, cos) = angle.sin_cos();
                twiddles.push(Complex64::new(cos, sin));
            }
            Engine::Even {
                forward,
                inverse,
                twiddles,
            }
        } else {
            Engine::Odd { forward, inverse }
        };

        Ok(Self {
            n,
            transform_len,
            scratch_len,
            engine,
        })
    }

    pub(crate) fn workspace(&self) -> Result<RealFftWorkspace, ShtError> {
        Ok(RealFftWorkspace {
            transform: allocate_complex(self.transform_len, "real FFT transform")?,
            scratch: allocate_complex(self.scratch_len, "real FFT scratch")?,
        })
    }

    pub(crate) const fn spectrum_len(&self) -> usize {
        self.n / 2 + 1
    }

    /// Computes the unnormalized DFT modes `0..=floor(n/2)` of a real row.
    pub(crate) fn forward(
        &self,
        input: &[f64],
        output: &mut [Complex64],
        workspace: &mut RealFftWorkspace,
    ) -> Result<(), ShtError> {
        check_len("real FFT input", self.n, input.len())?;
        check_len("real FFT spectrum", self.spectrum_len(), output.len())?;
        self.check_workspace(workspace)?;

        match &self.engine {
            Engine::Even {
                forward, twiddles, ..
            } => {
                for (j, pair) in input.as_chunks::<2>().0.iter().enumerate() {
                    workspace.transform[j] = Complex64::new(pair[0], pair[1]);
                }
                forward.process_with_scratch(&mut workspace.transform, &mut workspace.scratch);

                let half = self.transform_len;
                for (k, value) in output.iter_mut().enumerate() {
                    let z = workspace.transform[k % half];
                    let mirror = workspace.transform[(half - (k % half)) % half].conj();
                    let difference = z - mirror;
                    *value =
                        (z + mirror) * 0.5 + Complex64::new(0.0, -0.5) * twiddles[k] * difference;
                }
                output[0].im = 0.0;
                output[half].im = 0.0;
            }
            Engine::Odd { forward, .. } => {
                for (slot, &sample) in workspace.transform.iter_mut().zip(input) {
                    *slot = Complex64::new(sample, 0.0);
                }
                forward.process_with_scratch(&mut workspace.transform, &mut workspace.scratch);
                output.copy_from_slice(&workspace.transform[..self.spectrum_len()]);
                output[0].im = 0.0;
            }
        }

        Ok(())
    }

    /// Reconstructs a real row from its unnormalized nonnegative Fourier modes.
    pub(crate) fn inverse(
        &self,
        input: &[Complex64],
        output: &mut [f64],
        workspace: &mut RealFftWorkspace,
    ) -> Result<(), ShtError> {
        check_len("real FFT spectrum", self.spectrum_len(), input.len())?;
        check_len("real FFT output", self.n, output.len())?;
        self.check_workspace(workspace)?;

        match &self.engine {
            Engine::Even {
                inverse, twiddles, ..
            } => {
                let half = self.transform_len;
                if input[0].im != 0.0 {
                    return Err(ShtError::NonRealFftEdge { index: 0 });
                }
                if input[half].im != 0.0 {
                    return Err(ShtError::NonRealFftEdge { index: half });
                }

                for k in 0..half {
                    let a = input[k];
                    let b = if k == 0 {
                        input[half]
                    } else {
                        input[half - k].conj()
                    };
                    let difference = a - b;
                    workspace.transform[k] =
                        (a + b) * 0.5 + Complex64::new(0.0, 0.5) * twiddles[k].conj() * difference;
                }

                inverse.process_with_scratch(&mut workspace.transform, &mut workspace.scratch);
                let scale = 1.0 / (half as f64);
                for (j, pair) in output.as_chunks_mut::<2>().0.iter_mut().enumerate() {
                    pair[0] = workspace.transform[j].re * scale;
                    pair[1] = workspace.transform[j].im * scale;
                }
            }
            Engine::Odd { inverse, .. } => {
                if input[0].im != 0.0 {
                    return Err(ShtError::NonRealFftEdge { index: 0 });
                }
                workspace.transform[0] = Complex64::new(input[0].re, 0.0);
                let highest_positive_mode = self.n / 2;
                for (k, &mode) in input
                    .iter()
                    .enumerate()
                    .take(highest_positive_mode + 1)
                    .skip(1)
                {
                    workspace.transform[k] = mode;
                    workspace.transform[self.n - k] = mode.conj();
                }

                inverse.process_with_scratch(&mut workspace.transform, &mut workspace.scratch);
                let scale = 1.0 / (self.n as f64);
                for (value, sample) in output.iter_mut().zip(&workspace.transform) {
                    *value = sample.re * scale;
                }
            }
        }

        Ok(())
    }

    fn check_workspace(&self, workspace: &RealFftWorkspace) -> Result<(), ShtError> {
        check_len(
            "real FFT workspace transform",
            self.transform_len,
            workspace.transform.len(),
        )?;
        check_len(
            "real FFT workspace scratch",
            self.scratch_len,
            workspace.scratch.len(),
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

fn allocate_complex(len: usize, array: &'static str) -> Result<Vec<Complex64>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed { array })?;
    values.resize(len, Complex64::new(0.0, 0.0));
    Ok(values)
}

#[cfg(test)]
mod tests {
    use num_complex::Complex64;
    use rustfft::FftPlanner;

    use super::*;

    fn close_complex(actual: Complex64, expected: Complex64, tolerance: f64) {
        let error = (actual - expected).norm();
        assert!(
            error <= tolerance * expected.norm().max(1.0),
            "actual={actual:?}, expected={expected:?}, error={error:.3e}"
        );
    }

    fn sample_rows(n: usize) -> Vec<Vec<f64>> {
        let mut rows = Vec::new();
        rows.push((0..n).map(|i| if i == n / 3 { 1.0 } else { 0.0 }).collect());
        rows.push(vec![1.25; n]);
        rows.push(
            (0..n)
                .map(|i| (0.37 * i as f64).sin() + (0.61 * i as f64).cos())
                .collect(),
        );
        for k in 1..=n / 2 {
            rows.push(
                (0..n)
                    .map(|i| (2.0 * PI * k as f64 * i as f64 / n as f64).cos())
                    .collect(),
            );
        }
        rows
    }

    fn full_complex_forward(input: &[f64]) -> Vec<Complex64> {
        let n = input.len();
        let mut planner = FftPlanner::<f64>::new();
        let fft = planner.plan_fft_forward(n);
        let mut values: Vec<_> = input.iter().map(|&x| Complex64::new(x, 0.0)).collect();
        let mut scratch = vec![Complex64::new(0.0, 0.0); fft.get_inplace_scratch_len()];
        fft.process_with_scratch(&mut values, &mut scratch);
        values
    }

    #[test]
    fn real_fft_modes_match_full_complex_rustfft() {
        for n in [1, 2, 3, 4, 5, 6, 7, 8, 31, 32, 33] {
            let plan = RealFftPlan::new(n).unwrap();
            let mut workspace = plan.workspace().unwrap();
            for input in sample_rows(n) {
                let mut actual = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
                plan.forward(&input, &mut actual, &mut workspace).unwrap();
                let expected = full_complex_forward(&input);
                for k in 0..plan.spectrum_len() {
                    close_complex(actual[k], expected[k], 4e-13);
                }
                assert_eq!(actual[0].im, 0.0);
                if n % 2 == 0 {
                    assert_eq!(actual[n / 2].im, 0.0);
                }
            }
        }
    }

    #[test]
    fn real_fft_preserves_phase_for_positive_modes() {
        let phase = 0.73_f64;
        for n in [3, 4, 5, 8, 31, 32, 33] {
            let plan = RealFftPlan::new(n).unwrap();
            let mut workspace = plan.workspace().unwrap();
            let input: Vec<_> = (0..n)
                .map(|j| (2.0 * PI * j as f64 / n as f64 + phase).cos())
                .collect();
            let mut spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
            plan.forward(&input, &mut spectrum, &mut workspace).unwrap();
            close_complex(
                spectrum[1],
                Complex64::from_polar(n as f64 / 2.0, phase),
                4e-13,
            );
        }
    }

    #[test]
    fn inverse_reconstruction_matches_real_samples_and_fft_scaling() {
        for n in [1, 2, 3, 4, 5, 6, 7, 8, 31, 32, 33] {
            let plan = RealFftPlan::new(n).unwrap();
            let mut workspace = plan.workspace().unwrap();
            for input in sample_rows(n) {
                let mut spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
                plan.forward(&input, &mut spectrum, &mut workspace).unwrap();
                let mut actual = vec![0.0; n];
                plan.inverse(&spectrum, &mut actual, &mut workspace)
                    .unwrap();
                for (&value, &expected) in actual.iter().zip(&input) {
                    assert!((value - expected).abs() <= 3e-13 * expected.abs().max(1.0));
                }
            }
        }
    }

    #[test]
    fn inverse_matches_direct_hermitian_sum_for_arbitrary_spectra() {
        for n in [1, 2, 3, 4, 5, 6, 7, 8, 31, 32, 33] {
            let plan = RealFftPlan::new(n).unwrap();
            let mut workspace = plan.workspace().unwrap();
            let mut spectrum: Vec<_> = (0..plan.spectrum_len())
                .map(|k| Complex64::new(0.21 * (k + 1) as f64, 0.17 * (k as f64 - 2.0)))
                .collect();
            spectrum[0].im = 0.0;
            if n.is_multiple_of(2) {
                spectrum[n / 2].im = 0.0;
            }

            let mut actual = vec![0.0; n];
            plan.inverse(&spectrum, &mut actual, &mut workspace)
                .unwrap();
            for (j, value) in actual.iter().enumerate() {
                let mut expected = spectrum[0].re;
                let last_positive = if n.is_multiple_of(2) {
                    spectrum.len() - 2
                } else {
                    spectrum.len() - 1
                };
                for (k, mode) in spectrum.iter().enumerate().take(last_positive + 1).skip(1) {
                    let angle = 2.0 * PI * (k * j) as f64 / n as f64;
                    expected += 2.0 * (mode.re * angle.cos() - mode.im * angle.sin());
                }
                if n.is_multiple_of(2) {
                    expected += spectrum[n / 2].re * (PI * j as f64).cos();
                }
                expected /= n as f64;
                assert!(
                    (value - expected).abs() <= 4e-13 * expected.abs().max(1.0),
                    "n={n}, j={j}, actual={value:.16e}, expected={expected:.16e}"
                );
            }
        }
    }

    #[test]
    fn real_fft_validates_shapes_and_spectrum_edges() {
        let plan = RealFftPlan::new(4).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
        assert_eq!(
            plan.forward(&[0.0; 3], &mut spectrum, &mut workspace),
            Err(ShtError::BufferLength {
                buffer: "real FFT input",
                expected: 4,
                actual: 3
            })
        );

        spectrum[0].im = 1.0;
        assert_eq!(
            plan.inverse(&spectrum, &mut [0.0; 4], &mut workspace),
            Err(ShtError::NonRealFftEdge { index: 0 })
        );
    }
}
