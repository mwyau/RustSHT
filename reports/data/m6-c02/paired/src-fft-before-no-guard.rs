// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::f64::consts::PI;
use std::sync::Arc;

use num_complex::Complex;
use rustfft::num_traits::{Float, Zero};
use rustfft::{Fft, FftNum, FftPlanner};

use crate::error::ShtError;

enum Engine<T: FftNum> {
    Even {
        forward: Arc<dyn Fft<T>>,
        inverse: Arc<dyn Fft<T>>,
        twiddles: Vec<Complex<T>>,
    },
    Odd {
        forward: Arc<dyn Fft<T>>,
        inverse: Arc<dyn Fft<T>>,
    },
}

/// Reusable real-input FFT plan for one longitude length.
pub(crate) struct RealFftPlan<T: FftNum = f64> {
    n: usize,
    transform_len: usize,
    scratch_len: usize,
    engine: Engine<T>,
}

/// Mutable storage for one real FFT execution.
pub(crate) struct RealFftWorkspace<T: FftNum = f64> {
    transform: Vec<Complex<T>>,
    scratch: Vec<Complex<T>>,
}

impl<T: FftNum + Float> RealFftPlan<T> {
    pub(crate) fn new(n: usize) -> Result<Self, ShtError> {
        if n == 0 {
            return Err(ShtError::InvalidPhiCount);
        }

        let even = n.is_multiple_of(2);
        let transform_len = if even { n / 2 } else { n };
        let mut planner = FftPlanner::<T>::new();
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
                let angle = -T::from_f64(PI).expect("pi is representable")
                    * T::from_usize(k).expect("usize converts to the FFT scalar")
                    / T::from_usize(transform_len).expect("usize converts to the FFT scalar");
                let (sin, cos) = angle.sin_cos();
                twiddles.push(Complex::new(cos, sin));
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

    pub(crate) fn workspace(&self) -> Result<RealFftWorkspace<T>, ShtError> {
        Ok(RealFftWorkspace::<T> {
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
        input: &[T],
        output: &mut [Complex<T>],
        workspace: &mut RealFftWorkspace<T>,
    ) -> Result<(), ShtError> {
        check_len("real FFT input", self.n, input.len())?;
        check_len("real FFT spectrum", self.spectrum_len(), output.len())?;
        self.check_workspace(workspace)?;

        match &self.engine {
            Engine::Even {
                forward, twiddles, ..
            } => {
                for (j, pair) in input.as_chunks::<2>().0.iter().enumerate() {
                    workspace.transform[j] = Complex::new(pair[0], pair[1]);
                }
                forward.process_with_scratch(&mut workspace.transform, &mut workspace.scratch);

                let half = self.transform_len;
                let half_scalar = T::from_f64(0.5).expect("half is representable");
                for (k, value) in output.iter_mut().enumerate() {
                    let z = workspace.transform[k % half];
                    let mirror = workspace.transform[(half - (k % half)) % half].conj();
                    *value = recombine_half(z, mirror, twiddles[k], -half_scalar);
                }
                output[0].im = T::zero();
                output[half].im = T::zero();
            }
            Engine::Odd { forward, .. } => {
                for (slot, &sample) in workspace.transform.iter_mut().zip(input) {
                    *slot = Complex::new(sample, T::zero());
                }
                forward.process_with_scratch(&mut workspace.transform, &mut workspace.scratch);
                output.copy_from_slice(&workspace.transform[..self.spectrum_len()]);
                output[0].im = T::zero();
            }
        }

        Ok(())
    }

    /// Reconstructs a real row from its unnormalized nonnegative Fourier modes.
    pub(crate) fn inverse(
        &self,
        input: &[Complex<T>],
        output: &mut [T],
        workspace: &mut RealFftWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.inverse_impl::<true>(input, output, workspace)
    }

    /// Evaluates a Hermitian Fourier series without a `1/n` factor.
    /// DC and even-length Nyquist bins each occur once in the series.
    pub(crate) fn inverse_series(
        &self,
        input: &[Complex<T>],
        output: &mut [T],
        workspace: &mut RealFftWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.inverse_impl::<false>(input, output, workspace)
    }

    fn inverse_impl<const NORMALIZE: bool>(
        &self,
        input: &[Complex<T>],
        output: &mut [T],
        workspace: &mut RealFftWorkspace<T>,
    ) -> Result<(), ShtError> {
        check_len("real FFT spectrum", self.spectrum_len(), input.len())?;
        check_len("real FFT output", self.n, output.len())?;
        self.check_workspace(workspace)?;

        match &self.engine {
            Engine::Even {
                inverse, twiddles, ..
            } => {
                let half = self.transform_len;
                if input[0].im != T::zero() {
                    return Err(ShtError::NonRealFftEdge { index: 0 });
                }
                if input[half].im != T::zero() {
                    return Err(ShtError::NonRealFftEdge { index: half });
                }

                for k in 0..half {
                    let a = input[k];
                    let b = if k == 0 {
                        input[half]
                    } else {
                        input[half - k].conj()
                    };
                    workspace.transform[k] = if NORMALIZE {
                        recombine_half(a, b, twiddles[k].conj(), T::from_f64(0.5).unwrap())
                    } else {
                        (a + b) + Complex::new(T::zero(), T::one()) * twiddles[k].conj() * (a - b)
                    };
                }

                inverse.process_with_scratch(&mut workspace.transform, &mut workspace.scratch);
                let scale = if NORMALIZE {
                    T::one() / T::from_usize(half).expect("usize converts to the FFT scalar")
                } else {
                    T::one()
                };
                for (j, pair) in output.as_chunks_mut::<2>().0.iter_mut().enumerate() {
                    pair[0] = workspace.transform[j].re * scale;
                    pair[1] = workspace.transform[j].im * scale;
                }
            }
            Engine::Odd { inverse, .. } => {
                if input[0].im != T::zero() {
                    return Err(ShtError::NonRealFftEdge { index: 0 });
                }
                workspace.transform[0] = Complex::new(input[0].re, T::zero());
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
                let scale = if NORMALIZE {
                    T::one() / T::from_usize(self.n).expect("usize converts to the FFT scalar")
                } else {
                    T::one()
                };
                for (value, sample) in output.iter_mut().zip(&workspace.transform) {
                    *value = sample.re * scale;
                }
            }
        }

        Ok(())
    }

    fn check_workspace(&self, workspace: &RealFftWorkspace<T>) -> Result<(), ShtError> {
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

#[inline]
fn half_sum<T: FftNum + Float>(a: T, b: T) -> T {
    let half = T::from_f64(0.5).expect("half is representable");
    let sum = a + b;
    if sum.is_infinite() && a.is_finite() && b.is_finite() {
        a * half + b * half
    } else {
        sum * half
    }
}

#[inline]
fn recombine_half<T: FftNum + Float>(
    a: Complex<T>,
    b: Complex<T>,
    twiddle: Complex<T>,
    signed_half: T,
) -> Complex<T> {
    let half = T::from_f64(0.5).expect("half is representable");
    let result = (a + b) * half + Complex::new(T::zero(), signed_half) * twiddle * (a - b);
    if result.re.is_finite() && result.im.is_finite() {
        return result;
    }
    // Sum before halving preserves subnormals in the ordinary path. Only an
    // overflowing finite sum is halved first in the exceptional path.
    let even = Complex::new(half_sum(a.re, b.re), half_sum(a.im, b.im));
    let odd = Complex::new(half_sum(a.re, -b.re), half_sum(a.im, -b.im));
    even + Complex::new(T::zero(), signed_half + signed_half) * twiddle * odd
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

#[cfg(test)]
mod tests {
    use num_complex::{Complex32, Complex64};
    use rustfft::FftPlanner;

    use super::*;

    #[test]
    fn length_two_extreme_finite_forward() {
        for input in [[1.0e308, 0.0], [0.0, 1.0e308]] {
            let plan = RealFftPlan::new(2).unwrap();
            let mut workspace = plan.workspace().unwrap();
            let mut spectrum = [Complex64::new(0.0, 0.0); 2];
            plan.forward(&input, &mut spectrum, &mut workspace).unwrap();
            assert_eq!(spectrum[0], Complex64::new(1.0e308, 0.0));
            assert_eq!(spectrum[1], Complex64::new(input[0] - input[1], 0.0));
        }
    }

    #[test]
    fn length_two_extreme_finite_inverse() {
        let plan = RealFftPlan::new(2).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let spectrum = [Complex64::new(1.0e308, 0.0); 2];
        let mut output = [0.0; 2];
        plan.inverse(&spectrum, &mut output, &mut workspace)
            .unwrap();
        assert_eq!(output, [1.0e308, 0.0]);
    }

    #[test]
    fn length_two_extreme_finite_f32() {
        let plan = RealFftPlan::<f32>::new(2).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let input = [2.0e38, 0.0];
        let mut spectrum = [Complex32::new(0.0, 0.0); 2];
        plan.forward(&input, &mut spectrum, &mut workspace).unwrap();
        assert_eq!(spectrum, [Complex32::new(2.0e38, 0.0); 2]);
        let mut output = [0.0; 2];
        plan.inverse(
            &[Complex32::new(2.0e38, 0.0); 2],
            &mut output,
            &mut workspace,
        )
        .unwrap();
        assert_eq!(output, input);
    }

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
    fn f32_real_fft_matches_full_complex_rustfft() {
        for n in [1, 2, 3, 4, 5, 8, 31, 32, 33] {
            let plan = RealFftPlan::<f32>::new(n).unwrap();
            let mut workspace = plan.workspace().unwrap();
            for input in sample_rows(n) {
                let input: Vec<f32> = input.into_iter().map(|value| value as f32).collect();
                let mut actual = vec![Complex32::new(0.0, 0.0); plan.spectrum_len()];
                plan.forward(&input, &mut actual, &mut workspace).unwrap();

                let mut planner = FftPlanner::<f32>::new();
                let fft = planner.plan_fft_forward(n);
                let mut expected: Vec<_> = input
                    .iter()
                    .map(|&value| Complex32::new(value, 0.0))
                    .collect();
                let mut scratch = vec![Complex32::new(0.0, 0.0); fft.get_inplace_scratch_len()];
                fft.process_with_scratch(&mut expected, &mut scratch);
                for k in 0..plan.spectrum_len() {
                    let error = (actual[k] - expected[k]).norm();
                    assert!(
                        error <= 8e-5 * expected[k].norm().max(1.0),
                        "n={n}, k={k}, actual={:?}, expected={:?}, error={error:.3e}",
                        actual[k],
                        expected[k]
                    );
                }
                assert_eq!(actual[0].im, 0.0);
                if n.is_multiple_of(2) {
                    assert_eq!(actual[n / 2].im, 0.0);
                }

                let mut reconstructed = vec![0.0_f32; n];
                plan.inverse(&actual, &mut reconstructed, &mut workspace)
                    .unwrap();
                for (&value, &reference) in reconstructed.iter().zip(&input) {
                    assert!((value - reference).abs() <= 4e-6 * reference.abs().max(1.0));
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
    fn inverse_series_matches_direct_sum_and_preserves_tiny_dc() {
        for n in [1, 2, 3, 4, 5, 8, 31, 32, 33] {
            let plan = RealFftPlan::new(n).unwrap();
            let mut workspace = plan.workspace().unwrap();
            let mut spectrum: Vec<_> = (0..plan.spectrum_len())
                .map(|k| Complex64::new(0.21 * (k + 1) as f64, 0.17 * (k as f64 - 2.0)))
                .collect();
            spectrum[0].im = 0.0;
            if n.is_multiple_of(2) {
                spectrum[n / 2].im = 0.0;
            }
            let mut output = vec![0.0; n];
            plan.inverse_series(&spectrum, &mut output, &mut workspace)
                .unwrap();
            for (j, &actual) in output.iter().enumerate() {
                let mut expected = spectrum[0].re;
                for (k, &mode) in spectrum.iter().enumerate().skip(1) {
                    let angle = 2.0 * PI * (k * j) as f64 / n as f64;
                    let weight = if 2 * k == n { 1.0 } else { 2.0 };
                    expected += weight * (mode.re * angle.cos() - mode.im * angle.sin());
                }
                assert!((actual - expected).abs() <= 4e-13 * expected.abs().max(1.0));
            }
            for dc in [f64::from_bits(1), -f64::from_bits(1), 1.0e308] {
                spectrum.fill(Complex64::new(0.0, 0.0));
                spectrum[0].re = dc;
                plan.inverse_series(&spectrum, &mut output, &mut workspace)
                    .unwrap();
                assert!(
                    output.iter().all(|&value| value == dc),
                    "n={n}, dc={dc}, {output:?}"
                );
            }
        }
    }

    #[test]
    fn half_recombination_preserves_subnormals_and_cancellation() {
        for a in [f64::from_bits(1), f64::MIN_POSITIVE, 1.0e308, -1.0e308] {
            assert_eq!(half_sum(a, a), a);
            assert_eq!(half_sum(a, -a), 0.0);
            assert_eq!(half_sum(a, 0.0), a * 0.5);
        }
        for a in [f32::from_bits(1), f32::MIN_POSITIVE, 2.0e38, -2.0e38] {
            assert_eq!(half_sum(a, a), a);
            assert_eq!(half_sum(a, -a), 0.0);
            assert_eq!(half_sum(a, 0.0), a * 0.5);
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
