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
use crate::complex::{
    BluesteinPlan, ComplexFftPlan, ComplexFftWorkspace, Direction, check_len, check_vec_len,
    try_vec, unity_root,
};

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
    bluestein: Vec<Option<BluesteinPlan<T>>>,
    complexify: Option<ComplexFftPlan<T>>,
}

/// Reusable scratch for a [`RealFftPlan`].
///
/// A workspace may be reused after an operation finishes. Concurrent calls
/// need separate workspaces.
pub struct RealFftWorkspace<T> {
    len: usize,
    scratch: Vec<Complex<T>>,
    radix_input: Vec<Complex<T>>,
    radix_output: Vec<Complex<T>>,
    radix_saved: Vec<Complex<T>>,
    bluestein_buffer: Vec<Complex<T>>,
    bluestein_workspaces: Vec<Option<ComplexFftWorkspace<T>>>,
    complexify_workspace: Option<ComplexFftWorkspace<T>>,
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

    /// Returns the byte size of all reusable scratch buffers in this
    /// workspace, including nested Bluestein and complexify workspaces.
    ///
    /// The count includes initialized buffer elements and excludes allocator
    /// metadata and the plan's precomputed tables. Returns `None` only if the
    /// total byte count overflows `usize`.
    pub fn scratch_bytes(&self) -> Option<usize> {
        let local_elements = [
            self.scratch.len(),
            self.radix_input.len(),
            self.radix_output.len(),
            self.radix_saved.len(),
            self.bluestein_buffer.len(),
        ]
        .into_iter()
        .try_fold(0usize, usize::checked_add)?;
        let local_bytes = local_elements.checked_mul(std::mem::size_of::<Complex<T>>())?;
        let blue_bytes = self
            .bluestein_workspaces
            .iter()
            .flatten()
            .try_fold(local_bytes, |total, workspace| {
                total.checked_add(workspace.scratch_bytes()?)
            })?;
        if let Some(workspace) = self.complexify_workspace.as_ref() {
            blue_bytes.checked_add(workspace.scratch_bytes()?)
        } else {
            Some(blue_bytes)
        }
    }
}

impl<T: Float + FromPrimitive> RealFftPlan<T> {
    fn try_new(len: usize) -> Result<Self, FftError> {
        if len == 0 {
            return Err(FftError::InvalidLength);
        }

        check_vec_len::<Complex<T>>(len)?;
        let factors = factorize(len)?;
        let use_complexify = len.is_multiple_of(2)
            && len > 1000
            && (len > 10000 || factors.iter().any(|&factor| factor > 5));
        let complexify = if use_complexify {
            Some(ComplexFftPlan::try_new(len / 2)?)
        } else {
            None
        };
        let mut roots = try_vec(len, Complex::new(T::zero(), T::zero()))?;
        let mut bluestein = Vec::new();
        bluestein
            .try_reserve_exact(factors.len())
            .map_err(|_| crate::FftError::AllocationFailed)?;
        for &radix in &factors {
            bluestein.push(if !use_complexify && radix >= 135 {
                Some(BluesteinPlan::try_new(radix)?)
            } else {
                None
            });
        }
        let scratch_len = if use_complexify {
            len
        } else {
            forward_scratch_len(len, &factors)?.max(inverse_scratch_len(len, &factors, &bluestein)?)
        };
        for (index, root) in roots.iter_mut().enumerate() {
            *root = unity_root(index, len);
        }

        Ok(Self {
            len,
            factors,
            roots,
            scratch_len,
            bluestein,
            complexify,
        })
    }

    fn try_workspace(&self) -> Result<RealFftWorkspace<T>, FftError> {
        let zero = Complex::new(T::zero(), T::zero());
        let max_radix = if self.complexify.is_some() {
            1
        } else {
            self.factors.iter().copied().max().unwrap_or(1)
        };
        let mut bluestein_buffer_len = 0;
        let mut bluestein_workspaces = Vec::new();
        bluestein_workspaces
            .try_reserve_exact(self.bluestein.len())
            .map_err(|_| crate::FftError::AllocationFailed)?;
        for plan in &self.bluestein {
            if let Some(plan) = plan {
                bluestein_buffer_len = bluestein_buffer_len.max(plan.convolution_len());
                bluestein_workspaces.push(Some(plan.convolution_workspace()?));
            } else {
                bluestein_workspaces.push(None);
            }
        }
        let complexify_workspace = self
            .complexify
            .as_ref()
            .map(ComplexFftPlan::try_workspace)
            .transpose()?;
        Ok(RealFftWorkspace {
            len: self.len,
            scratch: try_vec(self.scratch_len, zero)?,
            radix_input: try_vec(max_radix, zero)?,
            radix_output: try_vec(max_radix, zero)?,
            radix_saved: try_vec(max_radix, zero)?,
            bluestein_buffer: try_vec(bluestein_buffer_len, zero)?,
            bluestein_workspaces,
            complexify_workspace,
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

        if let Some(complexify) = self.complexify.as_ref() {
            let complex_len = self.len / 2;
            let values = &mut workspace.scratch[..complex_len];
            let (input_pairs, remainder) = input.as_chunks::<2>();
            debug_assert!(remainder.is_empty());
            for (value, pair) in values.iter_mut().zip(input_pairs) {
                *value = Complex::new(pair[0], pair[1]);
            }
            complexify.execute(
                values,
                workspace
                    .complexify_workspace
                    .as_mut()
                    .expect("complexified plan has a complex workspace"),
                Direction::Forward,
            )?;

            let (maximum, all_finite) =
                values
                    .iter()
                    .fold((T::zero(), true), |(maximum, all_finite), value| {
                        (
                            maximum.max(value.re.abs()).max(value.im.abs()),
                            all_finite && value.re.is_finite() && value.im.is_finite(),
                        )
                    });
            let scaled = all_finite && maximum > T::max_value() / scalar::<T>(8.0);
            if scaled {
                // The postprocessing equations average pairs of complex FFT
                // bins. Scale large finite values by an exact power of two so
                // the sums and root multiplication stay finite, then recover
                // any subnormal terms lost by that scaling separately.
                let scale = scalar::<T>(0.125);
                let unscale = scalar::<T>(8.0);
                let value = values[0];
                let main = value * scale;
                let residual = complex_prescale_residual(value, main, unscale);
                spectrum[0] = Complex::new(
                    (main.re + main.im) * unscale + residual.re + residual.im,
                    T::zero(),
                );
                spectrum[complex_len] = Complex::new(
                    (main.re - main.im) * unscale + residual.re - residual.im,
                    T::zero(),
                );

                let mut index = 1;
                let mut mirror = complex_len - 1;
                while index <= mirror {
                    let left = values[index];
                    let mirror_value = values[mirror];
                    let scaled_left = left * scale;
                    let scaled_mirror = mirror_value * scale;
                    let residual_left = complex_prescale_residual(left, scaled_left, unscale);
                    let residual_mirror =
                        complex_prescale_residual(mirror_value, scaled_mirror, unscale);
                    let (left_main, mirror_main) =
                        complexify_forward_pair(scaled_left, scaled_mirror, self.roots[index]);
                    let (left_residual, mirror_residual) =
                        complexify_forward_pair(residual_left, residual_mirror, self.roots[index]);
                    spectrum[index] = left_main * unscale + left_residual;
                    spectrum[mirror] = mirror_main * unscale + mirror_residual;
                    index += 1;
                    mirror -= 1;
                }
            } else {
                spectrum[0] = Complex::new(values[0].re + values[0].im, T::zero());
                let mut index = 1;
                let mut mirror = complex_len - 1;
                while index <= mirror {
                    let left = values[index];
                    let mirror_value = values[mirror];
                    let (left_value, mirror_value) =
                        complexify_forward_pair(left, mirror_value, self.roots[index]);
                    spectrum[index] = left_value;
                    spectrum[mirror] = mirror_value;
                    index += 1;
                    mirror -= 1;
                }
                spectrum[complex_len] = Complex::new(values[0].re - values[0].im, T::zero());
            }
            return Ok(());
        }

        RealForwardExecutor {
            total_len: self.len,
            roots: &self.roots,
            bluestein_plans: &self.bluestein,
            bluestein_buffer: &mut workspace.bluestein_buffer,
            bluestein_workspaces: &mut workspace.bluestein_workspaces,
            radix_input: &mut workspace.radix_input,
            radix_output: &mut workspace.radix_output,
        }
        .recurse(
            input,
            Layout {
                offset: 0,
                stride: 1,
                stage: 0,
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

        if let Some(complexify) = self.complexify.as_ref() {
            let complex_len = self.len / 2;
            let (maximum, all_finite) = spectrum.iter().enumerate().fold(
                (T::zero(), true),
                |(maximum, all_finite), (index, value)| {
                    let maximum = maximum.max(value.re.abs());
                    let all_finite = all_finite && value.re.is_finite();
                    if index == 0 || index == complex_len {
                        (maximum, all_finite)
                    } else {
                        (
                            maximum.max(value.im.abs()),
                            all_finite && value.im.is_finite(),
                        )
                    }
                },
            );
            let scaled = all_finite && maximum > T::max_value() / scalar::<T>(8.0);
            if scaled {
                // Keep the packed sums below overflow by scaling every
                // spectrum component by 1/8. The inverse FFT is linear, so
                // multiply its result by four to retain the final 1/2 factor.
                // A second transform recovers subnormal terms lost to this
                // exact power-of-two prescale.
                let scale = scalar::<T>(0.125);
                let unscale = scalar::<T>(8.0);
                let (values, residual_values) =
                    workspace.scratch[..self.len].split_at_mut(complex_len);
                let dc = spectrum[0].re;
                let nyquist = spectrum[complex_len].re;
                let scaled_dc = dc * scale;
                let scaled_nyquist = nyquist * scale;
                values[0] = Complex::new(scaled_dc + scaled_nyquist, scaled_dc - scaled_nyquist);
                let residual_dc = prescale_residual(dc, scaled_dc * unscale);
                let residual_nyquist = prescale_residual(nyquist, scaled_nyquist * unscale);
                residual_values[0] = Complex::new(
                    residual_dc + residual_nyquist,
                    residual_dc - residual_nyquist,
                );
                let mut has_residual = residual_dc != T::zero() || residual_nyquist != T::zero();

                let mut index = 1;
                let mut mirror = complex_len - 1;
                while index <= mirror {
                    let left = spectrum[index];
                    let mirror_value = spectrum[mirror];
                    let left = Complex::new(left.re, left.im);
                    let right = Complex::new(mirror_value.re, -mirror_value.im);
                    let scaled_left = left * scale;
                    let scaled_right = right * scale;
                    let residual_left = complex_prescale_residual(left, scaled_left, unscale);
                    let residual_right = complex_prescale_residual(right, scaled_right, unscale);
                    has_residual |= residual_left.re != T::zero()
                        || residual_left.im != T::zero()
                        || residual_right.re != T::zero()
                        || residual_right.im != T::zero();
                    let (left_main, mirror_main) =
                        complexify_inverse_pair(scaled_left, scaled_right, self.roots[index]);
                    let (left_residual, mirror_residual) =
                        complexify_inverse_pair(residual_left, residual_right, self.roots[index]);
                    values[index] = left_main;
                    values[mirror] = mirror_main;
                    residual_values[index] = left_residual;
                    residual_values[mirror] = mirror_residual;
                    index += 1;
                    mirror -= 1;
                }

                let complex_workspace = workspace
                    .complexify_workspace
                    .as_mut()
                    .expect("complexified plan has a complex workspace");
                let main_scale = scalar::<T>(4.0);
                complexify.execute_inverse_scaled(
                    values,
                    complex_workspace,
                    Direction::Inverse,
                    main_scale,
                )?;
                for (index, value) in values.iter().enumerate() {
                    output[2 * index] = value.re;
                    output[2 * index + 1] = value.im;
                }
                if has_residual {
                    let half = scalar::<T>(0.5);
                    complexify.execute_inverse_scaled(
                        residual_values,
                        complex_workspace,
                        Direction::Inverse,
                        half,
                    )?;
                    for (index, value) in residual_values.iter().enumerate() {
                        output[2 * index] = output[2 * index] + value.re;
                        output[2 * index + 1] = output[2 * index + 1] + value.im;
                    }
                }
            } else {
                let values = &mut workspace.scratch[..complex_len];
                values[0] = Complex::new(
                    spectrum[0].re + spectrum[complex_len].re,
                    spectrum[0].re - spectrum[complex_len].re,
                );
                let mut index = 1;
                let mut mirror = complex_len - 1;
                while index <= mirror {
                    let left = Complex::new(spectrum[index].re, spectrum[index].im);
                    let right = Complex::new(spectrum[mirror].re, -spectrum[mirror].im);
                    let (left_value, mirror_value) =
                        complexify_inverse_pair(left, right, self.roots[index]);
                    values[index] = left_value;
                    values[mirror] = mirror_value;
                    index += 1;
                    mirror -= 1;
                }
                complexify.execute_inverse_scaled(
                    values,
                    workspace
                        .complexify_workspace
                        .as_mut()
                        .expect("complexified plan has a complex workspace"),
                    Direction::Inverse,
                    scalar::<T>(0.5),
                )?;
                for (index, value) in values.iter().enumerate() {
                    output[2 * index] = value.re;
                    output[2 * index + 1] = value.im;
                }
            }
            return Ok(());
        }

        let defer_inverse_normalization = inverse_normalization_is_safe(spectrum, self.len);
        RealInverseExecutor {
            total_len: self.len,
            defer_inverse_normalization,
            roots: &self.roots,
            bluestein_plans: &self.bluestein,
            bluestein_buffer: &mut workspace.bluestein_buffer,
            bluestein_workspaces: &mut workspace.bluestein_workspaces,
            radix_input: &mut workspace.radix_input,
            radix_output: &mut workspace.radix_output,
            radix_saved: &mut workspace.radix_saved,
        }
        .recurse(
            spectrum,
            self.len,
            output,
            Layout {
                offset: 0,
                stride: 1,
                stage: 0,
            },
            &self.factors,
            &mut workspace.scratch,
        );
        if defer_inverse_normalization {
            let normalization = scalar::<T>(1.0 / self.len as f64);
            for value in output {
                *value = *value * normalization;
            }
        }
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
    stage: usize,
}

struct RealForwardExecutor<'a, T> {
    total_len: usize,
    roots: &'a [Complex<T>],
    bluestein_plans: &'a [Option<BluesteinPlan<T>>],
    bluestein_buffer: &'a mut [Complex<T>],
    bluestein_workspaces: &'a mut [Option<ComplexFftWorkspace<T>>],
    radix_input: &'a mut [Complex<T>],
    radix_output: &'a mut [Complex<T>],
}

impl<T: Float + FromPrimitive> RealForwardExecutor<'_, T> {
    fn recurse(
        &mut self,
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
                    stage: layout.stage + 1,
                },
                child_len,
                &factors[1..],
                &mut children[start..start + child_bins],
                recursive_scratch,
            );
        }

        let root_stride = self.total_len / len;
        // For each supported child frequency, `branch * child_frequency` is
        // less than `len`; the stage-root lookup needs no remainder.
        if let Some(plan) = self.bluestein_plans[layout.stage].as_ref() {
            for child_frequency in 0..child_len {
                for branch in 0..radix {
                    let child = &children[branch * child_bins..(branch + 1) * child_bins];
                    let child_value = child_spectrum_value(child, child_len, child_frequency);
                    let root_index = root_stride * branch * child_frequency;
                    self.radix_input[branch] = child_value * self.roots[root_index].conj();
                }
                plan.execute(
                    &self.radix_input[..radix],
                    &mut self.radix_output[..radix],
                    Direction::Forward,
                    self.bluestein_buffer,
                    self.bluestein_workspaces[layout.stage]
                        .as_mut()
                        .expect("Bluestein stage has a workspace"),
                );
                for radix_frequency in 0..radix {
                    let frequency = child_frequency + radix_frequency * child_len;
                    if frequency < output.len() {
                        output[frequency] = self.radix_output[radix_frequency];
                    }
                }
            }
        } else {
            let radix_root_stride = self.total_len / radix;
            for child_frequency in 0..child_len {
                for branch in 0..radix {
                    let child = &children[branch * child_bins..(branch + 1) * child_bins];
                    let child_value = child_spectrum_value(child, child_len, child_frequency);
                    let root_index = root_stride * branch * child_frequency;
                    self.radix_input[branch] = child_value * self.roots[root_index].conj();
                }
                if radix <= 5 {
                    let zero = Complex::new(T::zero(), T::zero());
                    let mut radix_values = [zero; 5];
                    radix_values[..radix].copy_from_slice(&self.radix_input[..radix]);
                    let mut transformed = [zero; 5];
                    small_radix_dft_all(&radix_values, radix, false, &mut transformed);
                    for (radix_frequency, value) in transformed.iter().enumerate().take(radix) {
                        let frequency = child_frequency + radix_frequency * child_len;
                        if frequency < output.len() {
                            output[frequency] = *value;
                        }
                    }
                } else {
                    let first = self.radix_input[0];
                    for branch in 1..=(radix - 1) / 2 {
                        let opposite = radix - branch;
                        let left = self.radix_input[branch];
                        let right = self.radix_input[opposite];
                        self.radix_input[branch] = left + right;
                        self.radix_output[branch] = left - right;
                    }
                    for radix_frequency in 0..radix {
                        let frequency = child_frequency + radix_frequency * child_len;
                        if frequency >= output.len() {
                            continue;
                        }
                        let mut sum = first;
                        for branch in 1..=(radix - 1) / 2 {
                            let phase_index =
                                radix_root_stride * modular_product(branch, radix_frequency, radix);
                            let root = self.roots[phase_index].conj();
                            sum = sum
                                + self.radix_input[branch] * root.re
                                + imaginary(self.radix_output[branch]) * root.im;
                        }
                        output[frequency] = sum;
                    }
                }
            }
        }
    }
}

struct RealInverseExecutor<'a, T> {
    total_len: usize,
    defer_inverse_normalization: bool,
    roots: &'a [Complex<T>],
    bluestein_plans: &'a [Option<BluesteinPlan<T>>],
    bluestein_buffer: &'a mut [Complex<T>],
    bluestein_workspaces: &'a mut [Option<ComplexFftWorkspace<T>>],
    radix_input: &'a mut [Complex<T>],
    radix_output: &'a mut [Complex<T>],
    radix_saved: &'a mut [Complex<T>],
}

impl<T: Float + FromPrimitive> RealInverseExecutor<'_, T> {
    fn recurse(
        &mut self,
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
        let radix_scale = if self.defer_inverse_normalization {
            T::one()
        } else {
            scalar::<T>(1.0 / radix as f64)
        };
        let radix_value = scalar::<T>(radix as f64);
        let root_stride = self.total_len / len;
        // The half-spectrum limits `frequency` further, so the branch and
        // frequency product is below `len` and the root lookup needs no mod.

        if let Some(plan) = self.bluestein_plans[layout.stage].as_ref() {
            let child_total = radix * child_bins;
            let (children, recursive_scratch) = scratch.split_at_mut(child_total);
            let overflow_threshold = T::max_value() / scalar::<T>(4.0 * radix as f64);

            for frequency in 0..child_bins {
                let mut max_component = T::zero();
                for branch_frequency in 0..radix {
                    let full_frequency = frequency + branch_frequency * child_len;
                    let source = full_spectrum_value(spectrum, full_frequency, len);
                    if !self.defer_inverse_normalization {
                        max_component = max_component.max(source.re.abs()).max(source.im.abs());
                    }
                    self.radix_input[branch_frequency] = source;
                }
                let prescale =
                    !self.defer_inverse_normalization && max_component > overflow_threshold;
                if prescale {
                    for value in &mut self.radix_input[..radix] {
                        *value = *value * radix_scale;
                    }
                }
                plan.execute(
                    &self.radix_input[..radix],
                    &mut self.radix_output[..radix],
                    Direction::Inverse,
                    self.bluestein_buffer,
                    self.bluestein_workspaces[layout.stage]
                        .as_mut()
                        .expect("Bluestein stage has a workspace"),
                );

                if prescale {
                    self.radix_saved[..radix].copy_from_slice(&self.radix_output[..radix]);
                    for branch_frequency in 0..radix {
                        let full_frequency = frequency + branch_frequency * child_len;
                        let source = full_spectrum_value(spectrum, full_frequency, len);
                        let scaled = source * radix_scale;
                        self.radix_input[branch_frequency] = Complex::new(
                            prescale_residual(source.re, scaled.re * radix_value),
                            prescale_residual(source.im, scaled.im * radix_value),
                        );
                    }
                    plan.execute(
                        &self.radix_input[..radix],
                        &mut self.radix_output[..radix],
                        Direction::Inverse,
                        self.bluestein_buffer,
                        self.bluestein_workspaces[layout.stage]
                            .as_mut()
                            .expect("Bluestein stage has a workspace"),
                    );
                }

                for branch in 0..radix {
                    let root_index = root_stride * branch * frequency;
                    let mut child_value = if prescale {
                        self.radix_saved[branch] + self.radix_output[branch] * radix_scale
                    } else {
                        self.radix_output[branch] * radix_scale
                    };
                    child_value = child_value * self.roots[root_index];
                    children[branch * child_bins + frequency] = child_value;
                }
            }

            for branch in 0..radix {
                let start = branch * child_bins;
                self.recurse(
                    &children[start..start + child_bins],
                    child_len,
                    output,
                    Layout {
                        offset: layout.offset + branch * layout.stride,
                        stride: layout.stride * radix,
                        stage: layout.stage + 1,
                    },
                    &factors[1..],
                    recursive_scratch,
                );
            }
        } else {
            let child_total = radix * child_bins;
            let (children, recursive_scratch) = scratch.split_at_mut(child_total);
            let overflow_threshold = T::max_value() / scalar::<T>(4.0 * radix as f64);
            let radix_root_stride = self.total_len / radix;
            for frequency in 0..child_bins {
                let mut max_component = T::zero();
                let mut all_finite = true;
                if radix <= 5 {
                    let zero = Complex::new(T::zero(), T::zero());
                    let mut scaled_values = [zero; 5];
                    let mut residuals = [zero; 5];
                    let mut sources = [zero; 5];
                    let mut transformed = [zero; 5];
                    for (branch_frequency, slot) in sources.iter_mut().enumerate().take(radix) {
                        let full_frequency = frequency + branch_frequency * child_len;
                        let source = full_spectrum_value(spectrum, full_frequency, len);
                        *slot = source;
                        if !self.defer_inverse_normalization {
                            max_component = max_component.max(source.re.abs()).max(source.im.abs());
                            all_finite &= source.re.is_finite() && source.im.is_finite();
                        }
                    }
                    // Four-radix headroom bounds the radix sum and its complex
                    // root multiplication. The unscaled butterfly also avoids
                    // losing small terms before cancellation. Groups without
                    // this bound keep the residual-corrected path.
                    let fast_path = self.defer_inverse_normalization
                        || (all_finite && max_component <= overflow_threshold);
                    if fast_path {
                        small_radix_dft_all(&sources, radix, true, &mut transformed);
                        for branch in 0..radix {
                            let child_root_index = root_stride * branch * frequency;
                            children[branch * child_bins + frequency] =
                                transformed[branch] * self.roots[child_root_index] * radix_scale;
                        }
                    } else {
                        for branch_frequency in 0..radix {
                            let source = sources[branch_frequency];
                            let scaled = source * radix_scale;
                            scaled_values[branch_frequency] = scaled;
                            let reconstructed = scaled * radix_value;
                            residuals[branch_frequency] = Complex::new(
                                prescale_residual(source.re, reconstructed.re),
                                prescale_residual(source.im, reconstructed.im),
                            );
                        }
                        small_radix_dft_all(&scaled_values, radix, true, &mut transformed);
                        for branch in 0..radix {
                            let child_root_index = root_stride * branch * frequency;
                            children[branch * child_bins + frequency] =
                                transformed[branch] * self.roots[child_root_index];
                        }
                        small_radix_dft_all(&residuals, radix, true, &mut transformed);
                        for branch in 0..radix {
                            let child_root_index = root_stride * branch * frequency;
                            children[branch * child_bins + frequency] = children
                                [branch * child_bins + frequency]
                                + transformed[branch] * self.roots[child_root_index] * radix_scale;
                        }
                    }
                } else {
                    for branch_frequency in 0..radix {
                        let full_frequency = frequency + branch_frequency * child_len;
                        let source = full_spectrum_value(spectrum, full_frequency, len);
                        self.radix_input[branch_frequency] = source;
                        if !self.defer_inverse_normalization {
                            max_component = max_component.max(source.re.abs()).max(source.im.abs());
                            all_finite &= source.re.is_finite() && source.im.is_finite();
                        }
                    }
                    let fast_path = self.defer_inverse_normalization
                        || (all_finite && max_component <= overflow_threshold);
                    if fast_path {
                        let first = self.radix_input[0];
                        for branch_frequency in 1..=(radix - 1) / 2 {
                            let opposite_frequency = radix - branch_frequency;
                            let left = self.radix_input[branch_frequency];
                            let right = self.radix_input[opposite_frequency];
                            self.radix_input[branch_frequency] = left + right;
                            self.radix_output[branch_frequency] = left - right;
                        }
                        for branch in 0..radix {
                            let child_root_index = root_stride * branch * frequency;
                            let mut sum = first;
                            for branch_frequency in 1..=(radix - 1) / 2 {
                                let phase_index = radix_root_stride
                                    * modular_product(branch_frequency, branch, radix);
                                let phase = self.roots[phase_index];
                                sum = sum
                                    + self.radix_input[branch_frequency] * phase.re
                                    + imaginary(self.radix_output[branch_frequency]) * phase.im;
                            }
                            children[branch * child_bins + frequency] =
                                sum * self.roots[child_root_index] * radix_scale;
                        }
                    } else {
                        for branch_frequency in 0..radix {
                            let source = self.radix_input[branch_frequency];
                            let scaled = source * radix_scale;
                            self.radix_saved[branch_frequency] = scaled;
                            let reconstructed = scaled * radix_value;
                            self.radix_input[branch_frequency] = Complex::new(
                                prescale_residual(source.re, reconstructed.re),
                                prescale_residual(source.im, reconstructed.im),
                            );
                        }
                        let first_scaled = self.radix_saved[0];
                        for branch_frequency in 1..=(radix - 1) / 2 {
                            let opposite_frequency = radix - branch_frequency;
                            let left = self.radix_saved[branch_frequency];
                            let right = self.radix_saved[opposite_frequency];
                            self.radix_saved[branch_frequency] = left + right;
                            self.radix_output[branch_frequency] = left - right;
                        }
                        for branch in 0..radix {
                            let child_root_index = root_stride * branch * frequency;
                            let mut sum = first_scaled;
                            for branch_frequency in 1..=(radix - 1) / 2 {
                                let phase_index = radix_root_stride
                                    * modular_product(branch_frequency, branch, radix);
                                let phase = self.roots[phase_index];
                                sum = sum
                                    + self.radix_saved[branch_frequency] * phase.re
                                    + imaginary(self.radix_output[branch_frequency]) * phase.im;
                            }
                            children[branch * child_bins + frequency] =
                                sum * self.roots[child_root_index];
                        }
                        let first_residual = self.radix_input[0];
                        for branch_frequency in 1..=(radix - 1) / 2 {
                            let opposite_frequency = radix - branch_frequency;
                            let left = self.radix_input[branch_frequency];
                            let right = self.radix_input[opposite_frequency];
                            self.radix_saved[branch_frequency] = left + right;
                            self.radix_output[branch_frequency] = left - right;
                        }
                        for branch in 0..radix {
                            let child_root_index = root_stride * branch * frequency;
                            let mut correction = first_residual;
                            for branch_frequency in 1..=(radix - 1) / 2 {
                                let phase_index = radix_root_stride
                                    * modular_product(branch_frequency, branch, radix);
                                let phase = self.roots[phase_index];
                                correction = correction
                                    + self.radix_saved[branch_frequency] * phase.re
                                    + imaginary(self.radix_output[branch_frequency]) * phase.im;
                            }
                            children[branch * child_bins + frequency] = children
                                [branch * child_bins + frequency]
                                + correction * self.roots[child_root_index] * radix_scale;
                        }
                    }
                }
            }

            for branch in 0..radix {
                let start = branch * child_bins;
                self.recurse(
                    &children[start..start + child_bins],
                    child_len,
                    output,
                    Layout {
                        offset: layout.offset + branch * layout.stride,
                        stride: layout.stride * radix,
                        stage: layout.stage + 1,
                    },
                    &factors[1..],
                    recursive_scratch,
                );
            }
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

fn child_spectrum_value<T: Float>(
    spectrum: &[Complex<T>],
    len: usize,
    frequency: usize,
) -> Complex<T> {
    if frequency <= len / 2 {
        spectrum[frequency]
    } else {
        spectrum[len - frequency].conj()
    }
}

fn imaginary<T: Float>(value: Complex<T>) -> Complex<T> {
    Complex::new(-value.im, value.re)
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

fn inverse_scratch_len<T>(
    len: usize,
    factors: &[usize],
    bluestein: &[Option<BluesteinPlan<T>>],
) -> Result<usize, FftError> {
    if len == 1 {
        return Ok(0);
    }
    let radix = factors[0];
    let child_len = len / radix;
    let child_bins = child_len / 2 + 1;
    let current = radix
        .checked_mul(child_bins)
        .ok_or(FftError::AllocationFailed)?;
    current
        .checked_add(inverse_scratch_len(
            child_len,
            &factors[1..],
            &bluestein[1..],
        )?)
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
    debug_assert!(left < modulus && right < modulus);
    // General real radices are below the Bluestein threshold, so the product
    // remains bounded well below usize::MAX.
    left * right % modulus
}

fn complexify_forward_pair<T: Float + FromPrimitive>(
    left: Complex<T>,
    mirror: Complex<T>,
    root: Complex<T>,
) -> (Complex<T>, Complex<T>) {
    let half = scalar::<T>(0.5);
    let even = Complex::new(left.re + mirror.re, left.im - mirror.im);
    let odd = Complex::new(left.im + mirror.im, mirror.re - left.re) * root.conj();
    (
        (even + odd) * half,
        Complex::new((even.re - odd.re) * half, (odd.im - even.im) * half),
    )
}

fn complexify_inverse_pair<T: Float>(
    left: Complex<T>,
    right: Complex<T>,
    root: Complex<T>,
) -> (Complex<T>, Complex<T>) {
    let even = left + right;
    let odd = (left - right) * root;
    (
        even + Complex::new(-odd.im, odd.re),
        even.conj() + Complex::new(odd.im, odd.re),
    )
}

fn complex_prescale_residual<T: Float>(
    original: Complex<T>,
    scaled: Complex<T>,
    unscale: T,
) -> Complex<T> {
    Complex::new(
        prescale_residual(original.re, scaled.re * unscale),
        prescale_residual(original.im, scaled.im * unscale),
    )
}

fn scalar<T: FromPrimitive>(value: f64) -> T {
    T::from_f64(value).expect("f32 and f64 represent positive radix scales")
}

fn inverse_normalization_is_safe<T: Float + FromPrimitive>(
    spectrum: &[Complex<T>],
    len: usize,
) -> bool {
    let nyquist = len / 2;
    let mut maximum = T::zero();
    for (index, value) in spectrum.iter().enumerate() {
        if !value.re.is_finite() {
            return false;
        }
        maximum = maximum.max(value.re.abs());
        if index != 0 && index != nyquist {
            if !value.im.is_finite() {
                return false;
            }
            maximum = maximum.max(value.im.abs());
        }
    }
    maximum <= T::max_value() / scalar::<T>(4.0 * len as f64)
}

fn prescale_residual<T: Float>(original: T, reconstructed: T) -> T {
    if reconstructed.is_finite() {
        original - reconstructed
    } else {
        T::zero()
    }
}

fn small_radix_dft_all<T: Float + FromPrimitive>(
    values: &[Complex<T>; 5],
    radix: usize,
    inverse: bool,
    output: &mut [Complex<T>; 5],
) {
    let zero = Complex::new(T::zero(), T::zero());
    let mut forward = [zero; 5];
    match radix {
        2 => {
            forward[0] = values[0] + values[1];
            forward[1] = values[0] - values[1];
        }
        3 => {
            let pair = values[1] + values[2];
            let difference = values[1] - values[2];
            let base = values[0] - pair * scalar::<T>(0.5);
            let rotated = Complex::new(
                difference.im * scalar::<T>(0.866_025_403_784_438_6),
                -difference.re * scalar::<T>(0.866_025_403_784_438_6),
            );
            forward[0] = values[0] + pair;
            forward[1] = base + rotated;
            forward[2] = base - rotated;
        }
        4 => {
            let even_sum = values[0] + values[2];
            let even_difference = values[0] - values[2];
            let odd_sum = values[1] + values[3];
            let odd_difference = values[1] - values[3];
            forward[0] = even_sum + odd_sum;
            forward[1] = even_difference + Complex::new(odd_difference.im, -odd_difference.re);
            forward[2] = even_sum - odd_sum;
            forward[3] = even_difference + Complex::new(-odd_difference.im, odd_difference.re);
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
            let base_1 = values[0] + pair_1 * cosine_72 + pair_2 * cosine_144;
            let base_2 = values[0] + pair_1 * cosine_144 + pair_2 * cosine_72;
            let turn_1 = difference_1 * sine_72 + difference_2 * sine_144;
            let turn_2 = difference_1 * sine_144 - difference_2 * sine_72;
            forward[0] = values[0] + pair_1 + pair_2;
            forward[1] = base_1 + Complex::new(turn_1.im, -turn_1.re);
            forward[4] = base_1 + Complex::new(-turn_1.im, turn_1.re);
            forward[2] = base_2 + Complex::new(turn_2.im, -turn_2.re);
            forward[3] = base_2 + Complex::new(-turn_2.im, turn_2.re);
        }
        _ => unreachable!("small radix butterfly requires radix 2 through 5"),
    }
    for (frequency, value) in output.iter_mut().enumerate().take(radix) {
        let forward_frequency = if inverse {
            (radix - frequency) % radix
        } else {
            frequency
        };
        *value = forward[forward_frequency];
    }
}
