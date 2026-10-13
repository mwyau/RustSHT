// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-FileCopyrightText: 2010-2026 Max-Planck-Society (fft.h)
// SPDX-FileCopyrightText: 2010-2023 Max-Planck-Society (fft1d_impl.h)
// SPDX-FileCopyrightText: 2019 Peter Bell (fft1d_impl.h)
// SPDX-License-Identifier: BSD-3-Clause
//
// The radix order follows DUCC's rfftpass::factorize at
// c1ed01b8fda69d2a378190f0915f1a19762dddbf. The implementation recursively
// combines real half-spectra and uses the public Complex layout internally.
// Radices 2, 3, 4, and 5 use fixed butterflies derived from rfftp2/3/4/5.

use num_complex::Complex;
use num_traits::{Float, FromPrimitive};

use crate::FftError;
use crate::complex::{check_len, try_vec, unity_root};

/// An immutable one-dimensional real FFT plan.
///
/// Forward transforms return the nonnegative-frequency bins in ordinary
/// interleaved complex form and are unnormalized. Inverse transforms return
/// real samples normalized by `1/n`. Imaginary DC and even-length Nyquist
/// components supplied to [`Self::inverse`] are ignored.
pub struct RealFftPlan<T> {
    len: usize,
    factors: Vec<usize>,
    roots: Vec<Complex<T>>,
    scratch_len: usize,
}

/// Reusable scratch for a [`RealFftPlan`].
///
/// A workspace may be reused after an operation finishes. Concurrent calls
/// need separate workspaces.
pub struct RealFftWorkspace<T> {
    len: usize,
    scratch: Vec<Complex<T>>,
}

impl<T> RealFftWorkspace<T> {
    /// Returns the transform length for which this workspace was allocated.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if this workspace has no elements.
    pub fn is_empty(&self) -> bool {
        false
    }
}

impl<T: Float + FromPrimitive> RealFftPlan<T> {
    fn try_new(len: usize) -> Result<Self, FftError> {
        if len == 0 {
            return Err(FftError::InvalidLength);
        }

        let mut roots = try_vec(len, Complex::new(T::zero(), T::zero()))?;
        let factors = factorize(len)?;
        let scratch_len =
            forward_scratch_len(len, &factors)?.max(inverse_scratch_len(len, &factors)?);
        for (index, root) in roots.iter_mut().enumerate() {
            *root = unity_root(index, len);
        }

        Ok(Self {
            len,
            factors,
            roots,
            scratch_len,
        })
    }

    fn try_workspace(&self) -> Result<RealFftWorkspace<T>, FftError> {
        Ok(RealFftWorkspace {
            len: self.len,
            scratch: try_vec(self.scratch_len, Complex::new(T::zero(), T::zero()))?,
        })
    }

    /// Returns the real transform length.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the number of nonnegative-frequency complex bins.
    pub fn spectrum_len(&self) -> usize {
        self.len / 2 + 1
    }

    /// Returns `true` if the plan length is zero. Plans reject zero lengths.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Allocates reusable execution scratch for this plan.
    ///
    /// # Errors
    ///
    /// Returns [`FftError::AllocationFailed`] if scratch cannot be reserved.
    pub fn workspace(&self) -> Result<RealFftWorkspace<T>, FftError> {
        self.try_workspace()
    }

    /// Computes an unnormalized real-to-complex transform.
    ///
    /// The output contains bins `0..=floor(n/2)`. The plan and workspace can
    /// be reused without allocating during execution.
    ///
    /// # Errors
    ///
    /// Returns [`FftError::BufferLength`] if the input, spectrum, or workspace
    /// does not match this plan.
    pub fn forward(
        &self,
        input: &[T],
        spectrum: &mut [Complex<T>],
        workspace: &mut RealFftWorkspace<T>,
    ) -> Result<(), FftError> {
        check_len("input", self.len, input.len())?;
        check_len("spectrum", self.spectrum_len(), spectrum.len())?;
        check_len("workspace", self.len, workspace.len)?;

        RealForwardExecutor {
            total_len: self.len,
            roots: &self.roots,
        }
        .recurse(
            input,
            Layout {
                offset: 0,
                stride: 1,
            },
            self.len,
            &self.factors,
            spectrum,
            &mut workspace.scratch,
        );
        Ok(())
    }

    /// Computes the normalized complex-to-real inverse transform.
    ///
    /// Imaginary DC and, for even lengths, imaginary Nyquist values are
    /// ignored. Other positive-frequency bins imply their negative-frequency
    /// partners by conjugation.
    ///
    /// # Errors
    ///
    /// Returns [`FftError::BufferLength`] if the spectrum, output, or
    /// workspace does not match this plan.
    pub fn inverse(
        &self,
        spectrum: &[Complex<T>],
        output: &mut [T],
        workspace: &mut RealFftWorkspace<T>,
    ) -> Result<(), FftError> {
        check_len("spectrum", self.spectrum_len(), spectrum.len())?;
        check_len("output", self.len, output.len())?;
        check_len("workspace", self.len, workspace.len)?;

        RealInverseExecutor {
            total_len: self.len,
            roots: &self.roots,
        }
        .recurse(
            spectrum,
            self.len,
            output,
            Layout {
                offset: 0,
                stride: 1,
            },
            &self.factors,
            &mut workspace.scratch,
        );
        Ok(())
    }
}

macro_rules! implement_public_api {
    ($scalar:ty) => {
        impl RealFftPlan<$scalar> {
            /// Constructs a plan for a positive transform length.
            ///
            /// # Errors
            ///
            /// Returns [`FftError::InvalidLength`] for zero and
            /// [`FftError::AllocationFailed`] if plan storage cannot be
            /// reserved.
            pub fn new(len: usize) -> Result<Self, FftError> {
                Self::try_new(len)
            }
        }
    };
}

implement_public_api!(f32);
implement_public_api!(f64);

#[derive(Clone, Copy)]
struct Layout {
    offset: usize,
    stride: usize,
}

struct RealForwardExecutor<'a, T> {
    total_len: usize,
    roots: &'a [Complex<T>],
}

impl<T: Float + FromPrimitive> RealForwardExecutor<'_, T> {
    fn recurse(
        &self,
        input: &[T],
        layout: Layout,
        len: usize,
        factors: &[usize],
        output: &mut [Complex<T>],
        scratch: &mut [Complex<T>],
    ) {
        if len == 1 {
            output[0] = Complex::new(input[layout.offset], T::zero());
            return;
        }

        let radix = factors[0];
        let child_len = len / radix;
        let child_bins = child_len / 2 + 1;
        let child_total = radix * child_bins;
        let (children, recursive_scratch) = scratch.split_at_mut(child_total);

        for branch in 0..radix {
            let start = branch * child_bins;
            self.recurse(
                input,
                Layout {
                    offset: layout.offset + branch * layout.stride,
                    stride: layout.stride * radix,
                },
                child_len,
                &factors[1..],
                &mut children[start..start + child_bins],
                recursive_scratch,
            );
        }

        let root_stride = self.total_len / len;
        for (frequency, value) in output.iter_mut().enumerate() {
            let child_frequency = frequency % child_len;
            let radix_frequency = frequency / child_len;
            if radix <= 5 {
                let zero = Complex::new(T::zero(), T::zero());
                let mut radix_values = [zero; 5];
                for (branch, radix_value) in radix_values.iter_mut().enumerate().take(radix) {
                    let child = &children[branch * child_bins..(branch + 1) * child_bins];
                    let child_value = if child_frequency <= child_len / 2 {
                        child[child_frequency]
                    } else {
                        child[child_len - child_frequency].conj()
                    };
                    let root_index = root_stride * modular_product(branch, child_frequency, len);
                    *radix_value = child_value * self.roots[root_index].conj();
                }
                *value = small_radix_dft(&radix_values, radix, radix_frequency, false);
            } else {
                let mut sum = Complex::new(T::zero(), T::zero());
                for branch in 0..radix {
                    let child = &children[branch * child_bins..(branch + 1) * child_bins];
                    let child_value = if child_frequency <= child_len / 2 {
                        child[child_frequency]
                    } else {
                        child[child_len - child_frequency].conj()
                    };
                    let root_index = root_stride * modular_product(branch, frequency, len);
                    sum = sum + child_value * self.roots[root_index].conj();
                }
                *value = sum;
            }
        }
    }
}

struct RealInverseExecutor<'a, T> {
    total_len: usize,
    roots: &'a [Complex<T>],
}

impl<T: Float + FromPrimitive> RealInverseExecutor<'_, T> {
    fn recurse(
        &self,
        spectrum: &[Complex<T>],
        len: usize,
        output: &mut [T],
        layout: Layout,
        factors: &[usize],
        scratch: &mut [Complex<T>],
    ) {
        if len == 1 {
            output[layout.offset] = spectrum[0].re;
            return;
        }

        let radix = factors[0];
        let child_len = len / radix;
        let child_bins = child_len / 2 + 1;
        let (child_spectrum, recursive_scratch) = scratch.split_at_mut(child_bins);
        let radix_scale = scalar::<T>(1.0 / radix as f64);
        let radix_value = scalar::<T>(radix as f64);
        let root_stride = self.total_len / len;

        for branch in 0..radix {
            for (frequency, child_value) in child_spectrum.iter_mut().enumerate() {
                let child_root_index = root_stride * modular_product(frequency, branch, len);
                if radix <= 5 {
                    let zero = Complex::new(T::zero(), T::zero());
                    let mut scaled_values = [zero; 5];
                    let mut residuals = [zero; 5];
                    for branch_frequency in 0..radix {
                        let full_frequency = frequency + branch_frequency * child_len;
                        let source = full_spectrum_value(spectrum, full_frequency, len);
                        let scaled_source = source * radix_scale;
                        scaled_values[branch_frequency] = scaled_source;

                        // Correct values rounded away by 1/radix before the radix
                        // sum. Applying the correction after summing preserves terms
                        // that become representable only in combination.
                        let reconstructed = scaled_source * radix_value;
                        residuals[branch_frequency] = Complex::new(
                            prescale_residual(source.re, reconstructed.re),
                            prescale_residual(source.im, reconstructed.im),
                        );
                    }
                    let transformed = small_radix_dft(&scaled_values, radix, branch, true)
                        * self.roots[child_root_index];
                    let correction = small_radix_dft(&residuals, radix, branch, true)
                        * self.roots[child_root_index];
                    *child_value = transformed + correction * radix_scale;
                } else {
                    let mut sum = Complex::new(T::zero(), T::zero());
                    let mut correction = Complex::new(T::zero(), T::zero());
                    for branch_frequency in 0..radix {
                        let full_frequency = frequency + branch_frequency * child_len;
                        let source = full_spectrum_value(spectrum, full_frequency, len);
                        let radix_root_index = root_stride
                            * modular_product(branch_frequency, branch, radix)
                            * child_len;
                        let twiddle = self.roots[radix_root_index] * self.roots[child_root_index];
                        let scaled_source = source * radix_scale;
                        sum = sum + scaled_source * twiddle;

                        let reconstructed = scaled_source * radix_value;
                        let residual = Complex::new(
                            prescale_residual(source.re, reconstructed.re),
                            prescale_residual(source.im, reconstructed.im),
                        );
                        correction = correction + residual * twiddle;
                    }
                    *child_value = sum + correction * radix_scale;
                }
            }

            self.recurse(
                child_spectrum,
                child_len,
                output,
                Layout {
                    offset: layout.offset + branch * layout.stride,
                    stride: layout.stride * radix,
                },
                &factors[1..],
                recursive_scratch,
            );
        }
    }
}

fn full_spectrum_value<T: Float>(
    spectrum: &[Complex<T>],
    frequency: usize,
    len: usize,
) -> Complex<T> {
    if frequency <= len / 2 {
        let value = spectrum[frequency];
        if frequency == 0 || (len.is_multiple_of(2) && frequency == len / 2) {
            Complex::new(value.re, T::zero())
        } else {
            value
        }
    } else {
        spectrum[len - frequency].conj()
    }
}

fn forward_scratch_len(len: usize, factors: &[usize]) -> Result<usize, FftError> {
    if len == 1 {
        return Ok(0);
    }
    let radix = factors[0];
    let child_len = len / radix;
    let child_total = radix
        .checked_mul(child_len / 2 + 1)
        .ok_or(FftError::AllocationFailed)?;
    child_total
        .checked_add(forward_scratch_len(child_len, &factors[1..])?)
        .ok_or(FftError::AllocationFailed)
}

fn inverse_scratch_len(len: usize, factors: &[usize]) -> Result<usize, FftError> {
    if len == 1 {
        return Ok(0);
    }
    let radix = factors[0];
    let child_len = len / radix;
    (child_len / 2 + 1)
        .checked_add(inverse_scratch_len(child_len, &factors[1..])?)
        .ok_or(FftError::AllocationFailed)
}

fn factorize(len: usize) -> Result<Vec<usize>, FftError> {
    let mut factors = Vec::new();
    factors
        .try_reserve_exact(usize::BITS as usize)
        .map_err(|_| FftError::AllocationFailed)?;

    let mut remaining = len;
    while remaining.is_multiple_of(4) {
        factors.push(4);
        remaining /= 4;
    }
    if remaining.is_multiple_of(2) {
        remaining /= 2;
        factors.push(2);
        let last = factors.len() - 1;
        factors.swap(0, last);
    }
    let mut divisor = 3;
    while divisor <= remaining / divisor {
        while remaining.is_multiple_of(divisor) {
            factors.push(divisor);
            remaining /= divisor;
        }
        divisor += 2;
    }
    if remaining > 1 {
        factors.push(remaining);
    }
    Ok(factors)
}

fn modular_product(left: usize, right: usize, modulus: usize) -> usize {
    ((left as u128 * right as u128) % modulus as u128) as usize
}

fn scalar<T: FromPrimitive>(value: f64) -> T {
    T::from_f64(value).expect("f32 and f64 represent positive radix scales")
}

fn prescale_residual<T: Float>(original: T, reconstructed: T) -> T {
    if reconstructed.is_finite() {
        original - reconstructed
    } else {
        T::zero()
    }
}

fn small_radix_dft<T: Float + FromPrimitive>(
    values: &[Complex<T>; 5],
    radix: usize,
    frequency: usize,
    inverse: bool,
) -> Complex<T> {
    let frequency = if inverse {
        (radix - frequency % radix) % radix
    } else {
        frequency
    };
    match radix {
        2 => {
            if frequency == 0 {
                values[0] + values[1]
            } else {
                values[0] - values[1]
            }
        }
        3 => {
            let pair = values[1] + values[2];
            if frequency == 0 {
                values[0] + pair
            } else {
                let half = scalar::<T>(0.5);
                let tau = scalar::<T>(0.866_025_403_784_438_6);
                let delta = values[1] - values[2];
                let rotated = Complex::new(delta.im * tau, -delta.re * tau);
                let base = values[0] - pair * half;
                if frequency == 1 {
                    base + rotated
                } else {
                    base - rotated
                }
            }
        }
        4 => {
            let even_sum = values[0] + values[2];
            let even_difference = values[0] - values[2];
            let odd_sum = values[1] + values[3];
            let odd_difference = values[1] - values[3];
            match frequency {
                0 => even_sum + odd_sum,
                1 => even_difference + Complex::new(odd_difference.im, -odd_difference.re),
                2 => even_sum - odd_sum,
                _ => even_difference + Complex::new(-odd_difference.im, odd_difference.re),
            }
        }
        5 => {
            let pair_1 = values[1] + values[4];
            let pair_2 = values[2] + values[3];
            let difference_1 = values[1] - values[4];
            let difference_2 = values[2] - values[3];
            let cosine_72 = scalar::<T>(0.309_016_994_374_947_45);
            let cosine_144 = scalar::<T>(-0.809_016_994_374_947_5);
            let sine_72 = scalar::<T>(0.951_056_516_295_153_5);
            let sine_144 = scalar::<T>(0.587_785_252_292_473_1);
            let symmetric = values[0] + pair_1 * cosine_72 + pair_2 * cosine_144;
            let symmetric_swapped = values[0] + pair_1 * cosine_144 + pair_2 * cosine_72;
            let antisymmetric_1 = difference_1 * sine_72 + difference_2 * sine_144;
            let antisymmetric_2 = difference_1 * sine_144 - difference_2 * sine_72;
            let rotate_negative = |value: Complex<T>| Complex::new(value.im, -value.re);
            let rotate_positive = |value: Complex<T>| Complex::new(-value.im, value.re);
            match frequency {
                0 => values[0] + pair_1 + pair_2,
                1 => symmetric + rotate_negative(antisymmetric_1),
                2 => symmetric_swapped + rotate_negative(antisymmetric_2),
                3 => symmetric_swapped + rotate_positive(antisymmetric_2),
                _ => symmetric + rotate_positive(antisymmetric_1),
            }
        }
        _ => unreachable!("small radix butterfly requires radix 2 through 5"),
    }
}
