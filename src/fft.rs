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
mod tests;
