// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::f64::consts::PI;
use std::sync::Arc;
#[cfg(all(test, feature = "parallel"))]
use std::time::Instant;

use num_complex::Complex;
use rustfft::algorithm::MixedRadix;
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
        inverse_mixed_radix: Option<Arc<dyn Fft<T>>>,
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
    #[cfg(all(test, feature = "parallel"))]
    odd_inverse_phase_ns: [u64; 3],
    #[cfg(all(test, feature = "parallel"))]
    odd_inverse_phase_enabled: bool,
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
        // This alternative has been numerically validated for f64. Preserve
        // the existing f32 planner until a separate accuracy/performance
        // comparison justifies changing that route.
        let inverse_mixed_radix = if even || T::epsilon().to_f64() != Some(f64::EPSILON) {
            None
        } else {
            odd_square_inverse_plan(&mut planner, transform_len)
        };
        let scratch_len = forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len())
            .max(
                inverse_mixed_radix
                    .as_ref()
                    .map_or(0, |plan| plan.get_inplace_scratch_len()),
            );

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
            Engine::Odd {
                forward,
                inverse,
                inverse_mixed_radix,
            }
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
            #[cfg(all(test, feature = "parallel"))]
            odd_inverse_phase_ns: [0; 3],
            #[cfg(all(test, feature = "parallel"))]
            odd_inverse_phase_enabled: false,
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
        self.forward_modes(input, output, workspace, self.spectrum_len())
    }

    /// Computes only the requested leading nonnegative Fourier modes.
    /// The real-input FFT still evaluates its full transform; this avoids
    /// recombining and storing bins that a truncated harmonic transform will
    /// not consume.
    #[inline]
    pub(crate) fn forward_truncated(
        &self,
        input: &[T],
        output: &mut [Complex<T>],
        workspace: &mut RealFftWorkspace<T>,
    ) -> Result<(), ShtError> {
        let output_modes = output.len();
        self.forward_modes(input, output, workspace, output_modes)
    }

    /// Computes two adjacent odd-length real transforms with one complex FFT.
    /// Returns `false` when headroom or subnormal checks require the ordinary
    /// independent-row path.
    pub(crate) fn forward_paired_truncated(
        &self,
        first_input: &[T],
        second_input: &[T],
        first_output: &mut [Complex<T>],
        second_output: &mut [Complex<T>],
        workspace: &mut RealFftWorkspace<T>,
    ) -> Result<bool, ShtError> {
        check_len("paired real FFT first input", self.n, first_input.len())?;
        check_len("paired real FFT second input", self.n, second_input.len())?;
        if first_output.is_empty()
            || first_output.len() > self.spectrum_len()
            || second_output.len() != first_output.len()
        {
            return Err(ShtError::BufferLength {
                buffer: "paired real FFT spectrum",
                expected: self.spectrum_len(),
                actual: first_output.len().min(second_output.len()),
            });
        }
        self.check_workspace(workspace)?;
        if self.n.is_multiple_of(2) {
            return Ok(false);
        }
        let Engine::Odd { forward, .. } = &self.engine else {
            return Ok(false);
        };

        let component_bound = paired_component_bound::<T>(self.n);
        let mut safe = true;
        let mut first_scale = T::zero();
        let mut second_scale = T::zero();
        for (j, (&first, &second)) in first_input.iter().zip(second_input).enumerate() {
            safe &= paired_component_safe(first, component_bound)
                && paired_component_safe(second, component_bound);
            if first.is_finite() {
                first_scale = first_scale.max(first.abs());
            }
            if second.is_finite() {
                second_scale = second_scale.max(second.abs());
            }
            workspace.transform[j] = Complex::new(first, second);
        }
        if !safe || !paired_scales_safe(first_scale, second_scale) {
            return Ok(false);
        }

        forward.process_with_scratch(&mut workspace.transform, &mut workspace.scratch);
        let half = T::from_f64(0.5).expect("half is representable");
        for k in 0..first_output.len() {
            let value = workspace.transform[k];
            let mirror = workspace.transform[(self.n - k) % self.n].conj();
            let sum = value + mirror;
            let difference = value - mirror;
            first_output[k] = sum * half;
            second_output[k] = Complex::new(difference.im * half, -difference.re * half);
            if complex_has_subnormal(first_output[k]) || complex_has_subnormal(second_output[k]) {
                return Ok(false);
            }
        }
        first_output[0].im = T::zero();
        second_output[0].im = T::zero();
        Ok(true)
    }

    /// Reconstructs two adjacent odd-length real rows from Hermitian spectra
    /// with one complex inverse FFT. Returns `false` when the inputs or outputs
    /// need the independent-row path to preserve numerical safeguards.
    pub(crate) fn inverse_paired_series(
        &self,
        first_input: &[Complex<T>],
        second_input: &[Complex<T>],
        first_output: &mut [T],
        second_output: &mut [T],
        workspace: &mut RealFftWorkspace<T>,
    ) -> Result<bool, ShtError> {
        check_len(
            "paired real FFT first spectrum",
            self.spectrum_len(),
            first_input.len(),
        )?;
        check_len(
            "paired real FFT second spectrum",
            self.spectrum_len(),
            second_input.len(),
        )?;
        check_len("paired real FFT first output", self.n, first_output.len())?;
        check_len("paired real FFT second output", self.n, second_output.len())?;
        self.check_workspace(workspace)?;
        if first_input[0].im != T::zero() {
            return Err(ShtError::NonRealFftEdge { index: 0 });
        }
        if second_input[0].im != T::zero() {
            return Err(ShtError::NonRealFftEdge { index: 0 });
        }
        if self.n.is_multiple_of(2) {
            return Ok(false);
        }
        let Engine::Odd {
            inverse,
            inverse_mixed_radix,
            ..
        } = &self.engine
        else {
            return Ok(false);
        };

        let component_bound = paired_component_bound::<T>(self.n);
        let mut safe = paired_complex_safe(first_input[0], component_bound)
            && paired_complex_safe(second_input[0], component_bound);
        let mut first_scale = first_input[0].re.abs();
        let mut second_scale = second_input[0].re.abs();
        workspace.transform[0] = Complex::new(first_input[0].re, second_input[0].re);
        for k in 1..=self.n / 2 {
            let first = first_input[k];
            let second = second_input[k];
            safe &= paired_complex_safe(first, component_bound)
                && paired_complex_safe(second, component_bound);
            if first.re.is_finite() && first.im.is_finite() {
                first_scale = first_scale.max(first.re.abs()).max(first.im.abs());
            }
            if second.re.is_finite() && second.im.is_finite() {
                second_scale = second_scale.max(second.re.abs()).max(second.im.abs());
            }
            workspace.transform[k] = Complex::new(first.re - second.im, first.im + second.re);
            workspace.transform[self.n - k] =
                Complex::new(first.re + second.im, -first.im + second.re);
        }
        if !safe || !paired_scales_safe(first_scale, second_scale) {
            return Ok(false);
        }

        inverse_mixed_radix
            .as_deref()
            .unwrap_or(inverse.as_ref())
            .process_with_scratch(&mut workspace.transform, &mut workspace.scratch);
        for value in &workspace.transform {
            if value.re.is_subnormal() || value.im.is_subnormal() {
                return Ok(false);
            }
        }
        for ((first, second), value) in first_output
            .iter_mut()
            .zip(second_output)
            .zip(&workspace.transform)
        {
            *first = value.re;
            *second = value.im;
        }
        Ok(true)
    }

    #[inline]
    fn forward_modes(
        &self,
        input: &[T],
        output: &mut [Complex<T>],
        workspace: &mut RealFftWorkspace<T>,
        output_modes: usize,
    ) -> Result<(), ShtError> {
        check_len("real FFT input", self.n, input.len())?;
        if output_modes == 0 || output_modes > self.spectrum_len() {
            return Err(ShtError::BufferLength {
                buffer: "real FFT mode count",
                expected: self.spectrum_len(),
                actual: output_modes,
            });
        }
        check_len("real FFT spectrum", output_modes, output.len())?;
        self.check_workspace(workspace)?;

        match &self.engine {
            Engine::Even {
                forward, twiddles, ..
            } => {
                let component_bound = T::max_value()
                    / T::from_f64(8.0).expect("eight is representable")
                    / T::from_usize(self.n).expect("usize converts to the FFT scalar");
                let mut bounded_row = true;
                for (j, pair) in input.as_chunks::<2>().0.iter().enumerate() {
                    bounded_row &= pair
                        .iter()
                        .all(|&sample| sample.is_finite() && sample.abs() <= component_bound);
                    workspace.transform[j] = Complex::new(pair[0], pair[1]);
                }
                forward.process_with_scratch(&mut workspace.transform, &mut workspace.scratch);

                let half = self.transform_len;
                let half_scalar = T::from_f64(0.5).expect("half is representable");
                if bounded_row {
                    for (k, value) in output.iter_mut().enumerate() {
                        let z = workspace.transform[k % half];
                        let mirror = workspace.transform[(half - (k % half)) % half].conj();
                        *value = ordinary_recombine_half(z, mirror, twiddles[k], -half_scalar);
                    }
                } else {
                    for (k, value) in output.iter_mut().enumerate() {
                        let z = workspace.transform[k % half];
                        let mirror = workspace.transform[(half - (k % half)) % half].conj();
                        *value = recombine_half(z, mirror, twiddles[k], -half_scalar);
                    }
                }
                output[0].im = T::zero();
                if output_modes > half {
                    output[half].im = T::zero();
                }
            }
            Engine::Odd { forward, .. } => {
                for (slot, &sample) in workspace.transform.iter_mut().zip(input) {
                    *slot = Complex::new(sample, T::zero());
                }
                forward.process_with_scratch(&mut workspace.transform, &mut workspace.scratch);
                output.copy_from_slice(&workspace.transform[..output_modes]);
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
        self.inverse_impl::<true, true>(input, output, workspace)
    }

    /// Reconstructs a real row using the ordinary half-spectrum expression.
    /// The caller must establish finite headroom for every intermediate value.
    pub(crate) fn inverse_bounded(
        &self,
        input: &[Complex<T>],
        output: &mut [T],
        workspace: &mut RealFftWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.inverse_impl::<true, false>(input, output, workspace)
    }

    /// Evaluates a Hermitian Fourier series without a `1/n` factor.
    /// DC and even-length Nyquist bins each occur once in the series.
    pub(crate) fn inverse_series(
        &self,
        input: &[Complex<T>],
        output: &mut [T],
        workspace: &mut RealFftWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.inverse_impl::<false, false>(input, output, workspace)
    }

    fn inverse_impl<const NORMALIZE: bool, const ROBUST: bool>(
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
                    workspace.transform[k] = if NORMALIZE && ROBUST {
                        recombine_half(
                            a,
                            b,
                            twiddles[k].conj(),
                            T::from_f64(0.5).expect("half is representable"),
                        )
                    } else if NORMALIZE {
                        ordinary_recombine_half(
                            a,
                            b,
                            twiddles[k].conj(),
                            T::from_f64(0.5).expect("half is representable"),
                        )
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
            Engine::Odd {
                inverse,
                inverse_mixed_radix,
                ..
            } => {
                if input[0].im != T::zero() {
                    return Err(ShtError::NonRealFftEdge { index: 0 });
                }
                #[cfg(all(test, feature = "parallel"))]
                let phase_started = workspace.odd_inverse_phase_enabled.then(Instant::now);
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
                #[cfg(all(test, feature = "parallel"))]
                if let Some(phase_started) = phase_started {
                    workspace.odd_inverse_phase_ns[0] += phase_started.elapsed().as_nanos() as u64;
                }

                #[cfg(all(test, feature = "parallel"))]
                let phase_started = workspace.odd_inverse_phase_enabled.then(Instant::now);
                // The ordinary RustFFT planner treats repeated unsupported
                // prime factors (for example 19 x 19) as one Bluestein size.
                // A mixed-radix decomposition reuses its direct odd-radix
                // butterflies. Keep the original plan for the checked route:
                // the mixed-radix path is only selected when the caller has
                // already established the ordinary-transform headroom.
                let inverse = if ROBUST {
                    inverse.as_ref()
                } else {
                    inverse_mixed_radix.as_deref().unwrap_or(inverse.as_ref())
                };
                inverse.process_with_scratch(&mut workspace.transform, &mut workspace.scratch);
                #[cfg(all(test, feature = "parallel"))]
                if let Some(phase_started) = phase_started {
                    workspace.odd_inverse_phase_ns[1] += phase_started.elapsed().as_nanos() as u64;
                }
                let scale = if NORMALIZE {
                    T::one() / T::from_usize(self.n).expect("usize converts to the FFT scalar")
                } else {
                    T::one()
                };
                #[cfg(all(test, feature = "parallel"))]
                let phase_started = workspace.odd_inverse_phase_enabled.then(Instant::now);
                for (value, sample) in output.iter_mut().zip(&workspace.transform) {
                    *value = sample.re * scale;
                }
                #[cfg(all(test, feature = "parallel"))]
                if let Some(phase_started) = phase_started {
                    workspace.odd_inverse_phase_ns[2] += phase_started.elapsed().as_nanos() as u64;
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
fn paired_component_bound<T: FftNum + Float>(n: usize) -> T {
    T::max_value()
        / T::from_f64(8.0).expect("eight is representable")
        / T::from_usize(n).expect("usize converts to the FFT scalar")
}

#[inline]
fn paired_scales_safe<T: FftNum + Float>(first: T, second: T) -> bool {
    if first == T::zero() || second == T::zero() {
        return first == T::zero() && second == T::zero();
    }
    let smaller = first.min(second);
    let larger = first.max(second);
    // The forward split subtracts spectra produced by one complex transform,
    // so roundoff from a much larger row can leak into its neighbor. Keep the
    // measured fast path to pairs whose component scales differ by at most 8.
    let maximum_ratio = T::from_f64(8.0).expect("8 is representable");
    smaller >= larger / maximum_ratio
}

#[inline]
fn paired_component_safe<T: Float>(value: T, bound: T) -> bool {
    value.is_finite() && !value.is_subnormal() && value.abs() <= bound
}

#[inline]
fn paired_complex_safe<T: Float>(value: Complex<T>, bound: T) -> bool {
    paired_component_safe(value.re, bound) && paired_component_safe(value.im, bound)
}

#[inline]
fn complex_has_subnormal<T: Float>(value: Complex<T>) -> bool {
    value.re.is_subnormal() || value.im.is_subnormal()
}

fn odd_square_inverse_plan<T: FftNum>(
    planner: &mut FftPlanner<T>,
    transform_len: usize,
) -> Option<Arc<dyn Fft<T>>> {
    // RustFFT 6.4.1 leaves N=19² in its Bluestein base even though it has a
    // direct radix-19 butterfly. Composing that measured T720 odd longitude
    // length from the existing butterflies avoids the expensive Bluestein
    // convolution. Other lengths remain on RustFFT's normal planner until a
    // matched workload establishes that the alternative helps there too.
    if transform_len == 19 * 19 {
        let one_dimensional = planner.plan_fft_inverse(19);
        return Some(Arc::new(MixedRadix::new(
            Arc::clone(&one_dimensional),
            one_dimensional,
        )));
    }
    None
}

#[cfg(all(test, feature = "parallel"))]
impl<T: FftNum> RealFftWorkspace<T> {
    pub(crate) fn set_odd_inverse_phase_enabled(&mut self, enabled: bool) {
        self.odd_inverse_phase_enabled = enabled;
    }

    pub(crate) fn take_odd_inverse_phase_ns(&mut self) -> [u64; 3] {
        std::mem::take(&mut self.odd_inverse_phase_ns)
    }
}

#[inline]
fn ordinary_recombine_half<T: FftNum + Float>(
    a: Complex<T>,
    b: Complex<T>,
    twiddle: Complex<T>,
    signed_half: T,
) -> Complex<T> {
    let half = T::from_f64(0.5).expect("half is representable");
    let difference = a - b;
    (a + b) * half + Complex::new(T::zero(), signed_half) * twiddle * difference
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
    let result = ordinary_recombine_half(a, b, twiddle, signed_half);
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
    fn paired_odd_fft_matches_independent_rows_and_guards_unsafe_inputs() {
        for n in [129, 257, 361, 509, 513] {
            let plan = RealFftPlan::new(n).unwrap();
            let mut workspace = plan.workspace().unwrap();
            let first: Vec<_> = (0..n)
                .map(|j| 8.0 * ((0.37 * j as f64).sin() + (0.61 * j as f64).cos()))
                .collect();
            let second: Vec<_> = (0..n)
                .map(|j| (0.37 * j as f64).sin() + (0.61 * j as f64).cos())
                .collect();
            let mut first_expected = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
            let mut second_expected = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
            plan.forward(&first, &mut first_expected, &mut workspace)
                .unwrap();
            plan.forward(&second, &mut second_expected, &mut workspace)
                .unwrap();
            let mut first_actual = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
            let mut second_actual = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
            assert!(
                plan.forward_paired_truncated(
                    &first,
                    &second,
                    &mut first_actual,
                    &mut second_actual,
                    &mut workspace,
                )
                .unwrap()
            );
            for (actual, expected) in first_actual.iter().zip(&first_expected) {
                close_complex(*actual, *expected, 4e-13);
            }
            for (actual, expected) in second_actual.iter().zip(&second_expected) {
                close_complex(*actual, *expected, 4e-13);
            }

            let mut first_inverse_expected = vec![0.0; n];
            let mut second_inverse_expected = vec![0.0; n];
            plan.inverse_series(&first_expected, &mut first_inverse_expected, &mut workspace)
                .unwrap();
            plan.inverse_series(
                &second_expected,
                &mut second_inverse_expected,
                &mut workspace,
            )
            .unwrap();

            let opposite: Vec<_> = second.iter().map(|value| -*value).collect();
            assert!(
                plan.forward_paired_truncated(
                    &second,
                    &opposite,
                    &mut first_actual,
                    &mut second_actual,
                    &mut workspace,
                )
                .unwrap()
            );
            for (actual, expected) in first_actual.iter().zip(&second_expected) {
                close_complex(*actual, *expected, 4e-13);
            }
            for (actual, expected) in second_actual.iter().zip(&second_expected) {
                close_complex(*actual, -*expected, 4e-13);
            }

            let mut first_reconstructed = vec![0.0; n];
            let mut second_reconstructed = vec![0.0; n];
            assert!(
                plan.inverse_paired_series(
                    &first_expected,
                    &second_expected,
                    &mut first_reconstructed,
                    &mut second_reconstructed,
                    &mut workspace,
                )
                .unwrap()
            );
            let relative_l2 = |actual: &[f64], expected: &[f64]| {
                let (error, norm) = actual.iter().zip(expected).fold(
                    (0.0, 0.0),
                    |(error, norm), (&actual, &expected)| {
                        (error + (actual - expected).powi(2), norm + expected.powi(2))
                    },
                );
                (error / norm).sqrt()
            };
            assert!(
                relative_l2(&first_reconstructed, &first_inverse_expected) <= 4e-13,
                "N={n}, first inverse relative L2={:.3e}",
                relative_l2(&first_reconstructed, &first_inverse_expected)
            );
            assert!(
                relative_l2(&second_reconstructed, &second_inverse_expected) <= 4e-13,
                "N={n}, second inverse relative L2={:.3e}",
                relative_l2(&second_reconstructed, &second_inverse_expected)
            );

            let zero = vec![0.0; n];
            assert!(
                plan.forward_paired_truncated(
                    &zero,
                    &zero,
                    &mut first_actual,
                    &mut second_actual,
                    &mut workspace,
                )
                .unwrap()
            );
            assert!(
                !plan
                    .forward_paired_truncated(
                        &first,
                        &zero,
                        &mut first_actual,
                        &mut second_actual,
                        &mut workspace,
                    )
                    .unwrap()
            );

            let zero_spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
            let mut zero_output = vec![0.0; n];
            assert!(
                plan.inverse_paired_series(
                    &zero_spectrum,
                    &zero_spectrum,
                    &mut zero_output,
                    &mut first_reconstructed,
                    &mut workspace,
                )
                .unwrap()
            );
            assert!(
                !plan
                    .inverse_paired_series(
                        &first_expected,
                        &zero_spectrum,
                        &mut first_reconstructed,
                        &mut second_reconstructed,
                        &mut workspace,
                    )
                    .unwrap()
            );
            let very_unequal_spectrum: Vec<_> = first_expected.iter().map(|v| *v * 1.0e4).collect();
            assert!(
                !plan
                    .inverse_paired_series(
                        &very_unequal_spectrum,
                        &second_expected,
                        &mut first_reconstructed,
                        &mut second_reconstructed,
                        &mut workspace,
                    )
                    .unwrap()
            );
            let very_unequal: Vec<_> = second.iter().map(|value| value * 1.0e4).collect();
            assert!(
                !plan
                    .forward_paired_truncated(
                        &very_unequal,
                        &second,
                        &mut first_actual,
                        &mut second_actual,
                        &mut workspace,
                    )
                    .unwrap()
            );
        }

        let plan = RealFftPlan::new(129).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let normal = vec![1.0; 129];
        let mut exceptional = normal.clone();
        exceptional[0] = f64::NAN;
        let mut first_spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
        let mut second_spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
        assert!(
            !plan
                .forward_paired_truncated(
                    &exceptional,
                    &normal,
                    &mut first_spectrum,
                    &mut second_spectrum,
                    &mut workspace,
                )
                .unwrap()
        );
        exceptional[0] = f64::from_bits(1);
        assert!(
            !plan
                .forward_paired_truncated(
                    &exceptional,
                    &normal,
                    &mut first_spectrum,
                    &mut second_spectrum,
                    &mut workspace,
                )
                .unwrap()
        );
        exceptional[0] = f64::MAX / 2.0;
        assert!(
            !plan
                .forward_paired_truncated(
                    &exceptional,
                    &normal,
                    &mut first_spectrum,
                    &mut second_spectrum,
                    &mut workspace,
                )
                .unwrap()
        );
    }

    #[test]
    fn truncated_real_fft_matches_full_spectrum_prefix() {
        for n in [1, 2, 3, 4, 5, 6, 7, 8, 31, 32, 33] {
            let plan = RealFftPlan::new(n).unwrap();
            let mut workspace = plan.workspace().unwrap();
            let mut inputs = sample_rows(n);
            inputs.push(vec![f64::from_bits(1); n]);
            inputs.push(
                (0..n)
                    .map(|index| {
                        if index.is_multiple_of(2) {
                            1.0e307
                        } else {
                            -1.0e307
                        }
                    })
                    .collect(),
            );

            for input in inputs {
                let mut full = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
                plan.forward(&input, &mut full, &mut workspace).unwrap();

                let spectrum_len = plan.spectrum_len();
                let mut mode_counts = vec![1, spectrum_len.div_ceil(2), spectrum_len];
                mode_counts.sort_unstable();
                mode_counts.dedup();
                for mode_count in mode_counts {
                    let sentinel = Complex64::new(17.0, -29.0);
                    let mut truncated = vec![sentinel; mode_count];
                    plan.forward_truncated(&input, &mut truncated, &mut workspace)
                        .unwrap();
                    assert_eq!(truncated, full[..mode_count], "n={n}, modes={mode_count}");
                }
            }
        }
    }

    #[test]
    fn truncated_real_fft_rejects_invalid_mode_counts() {
        let plan = RealFftPlan::new(8).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let input = [0.0; 8];
        let mut empty = [];
        assert!(
            plan.forward_truncated(&input, &mut empty, &mut workspace)
                .is_err()
        );
        let mut too_many = vec![Complex64::new(0.0, 0.0); plan.spectrum_len() + 1];
        assert!(
            plan.forward_truncated(&input, &mut too_many, &mut workspace)
                .is_err()
        );
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
    fn bounded_inverse_matches_robust_inverse_for_ordinary_spectra() {
        for n in [1, 2, 3, 4, 5, 8, 31, 32, 33] {
            let plan = RealFftPlan::new(n).unwrap();
            let mut workspace = plan.workspace().unwrap();
            for row in sample_rows(n) {
                let mut spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
                plan.forward(&row, &mut spectrum, &mut workspace).unwrap();

                let mut robust = vec![0.0; n];
                let mut bounded = vec![0.0; n];
                plan.inverse(&spectrum, &mut robust, &mut workspace)
                    .unwrap();
                plan.inverse_bounded(&spectrum, &mut bounded, &mut workspace)
                    .unwrap();
                assert_eq!(bounded, robust, "n={n}");
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
    fn mixed_radix_odd_inverse_series_matches_full_complex_and_keeps_dc_edges() {
        let n = 361;
        let plan = RealFftPlan::<f64>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut spectrum: Vec<_> = (0..plan.spectrum_len())
            .map(|k| Complex64::new(0.21 * (k + 1) as f64, 0.17 * (k as f64 - 2.0)))
            .collect();
        spectrum[0].im = 0.0;
        let mut actual = vec![0.0; n];
        plan.inverse_series(&spectrum, &mut actual, &mut workspace)
            .unwrap();

        let mut reference: Vec<_> = (0..n).map(|_| Complex64::new(0.0, 0.0)).collect();
        reference[0] = spectrum[0];
        for k in 1..=n / 2 {
            reference[k] = spectrum[k];
            reference[n - k] = spectrum[k].conj();
        }
        let mut planner = FftPlanner::<f64>::new();
        let inverse = planner.plan_fft_inverse(n);
        let mut scratch = vec![Complex64::new(0.0, 0.0); inverse.get_inplace_scratch_len()];
        inverse.process_with_scratch(&mut reference, &mut scratch);
        for (&value, transformed) in actual.iter().zip(&reference) {
            let error = (value - transformed.re).abs();
            assert!(
                error <= 5e-14 * transformed.re.abs().max(1.0),
                "actual={value:.16e}, reference={:.16e}, error={error:.3e}",
                transformed.re
            );
        }

        // The checked inverse deliberately keeps RustFFT's original full-size
        // plan. Its normalized output must agree with the candidate series
        // after applying the same 1/N scaling.
        let mut robust = vec![0.0; n];
        plan.inverse(&spectrum, &mut robust, &mut workspace)
            .unwrap();
        for (&series_value, &robust_value) in actual.iter().zip(&robust) {
            let normalized = series_value / n as f64;
            assert!(
                (normalized - robust_value).abs() <= 5e-14 * robust_value.abs().max(1.0),
                "series={normalized:.16e}, robust={robust_value:.16e}"
            );
        }

        for dc in [f64::from_bits(1), -f64::from_bits(1), f64::MAX] {
            spectrum.fill(Complex64::new(0.0, 0.0));
            spectrum[0].re = dc;
            plan.inverse_series(&spectrum, &mut actual, &mut workspace)
                .unwrap();
            assert!(actual.iter().all(|&value| value == dc));
        }
    }

    #[test]
    fn only_measured_odd_square_selects_mixed_inverse() {
        for n in [169, 289, 529, 841, 961, 255, 359, 363] {
            let plan = RealFftPlan::<f64>::new(n).unwrap();
            let Engine::Odd {
                inverse_mixed_radix,
                ..
            } = &plan.engine
            else {
                panic!("odd transform should use the odd engine");
            };
            assert!(inverse_mixed_radix.is_none(), "n={n}");
        }

        let plan = RealFftPlan::<f64>::new(361).unwrap();
        let Engine::Odd {
            inverse_mixed_radix,
            ..
        } = &plan.engine
        else {
            panic!("odd transform should use the odd engine");
        };
        assert!(inverse_mixed_radix.is_some());
    }

    #[test]
    fn f32_odd_square_keeps_existing_inverse_planner() {
        let n = 361;
        let plan = RealFftPlan::<f32>::new(n).unwrap();
        let Engine::Odd {
            inverse_mixed_radix,
            ..
        } = &plan.engine
        else {
            panic!("odd transform should use the odd engine");
        };
        assert!(inverse_mixed_radix.is_none());
        let mut workspace = plan.workspace().unwrap();
        let mut spectrum: Vec<_> = (0..plan.spectrum_len())
            .map(|k| Complex32::new(0.21 * (k + 1) as f32, 0.17 * (k as f32 - 2.0)))
            .collect();
        spectrum[0].im = 0.0;
        let mut output = vec![0.0_f32; n];
        plan.inverse_series(&spectrum, &mut output, &mut workspace)
            .unwrap();

        let mut reference: Vec<_> = (0..n).map(|_| Complex32::new(0.0, 0.0)).collect();
        reference[0] = spectrum[0];
        for k in 1..=n / 2 {
            reference[k] = spectrum[k];
            reference[n - k] = spectrum[k].conj();
        }
        let mut planner = FftPlanner::<f32>::new();
        let inverse = planner.plan_fft_inverse(n);
        let mut scratch = vec![Complex32::new(0.0, 0.0); inverse.get_inplace_scratch_len()];
        inverse.process_with_scratch(&mut reference, &mut scratch);
        for (&actual, transformed) in output.iter().zip(&reference) {
            assert!(
                (actual - transformed.re).abs() <= 5e-6 * transformed.re.abs().max(1.0),
                "actual={actual:.8e}, reference={:.8e}",
                transformed.re
            );
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
