// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-FileCopyrightText: 2010-2026 Max-Planck-Society (fft.h)
// SPDX-FileCopyrightText: 2019-2026 Max-Planck-Society (unity_roots.h)
// SPDX-FileCopyrightText: 2010-2023 Max-Planck-Society
// SPDX-FileCopyrightText: 2019 Peter Bell
// SPDX-License-Identifier: BSD-3-Clause
//
// The factor order follows DUCC's cfftpass::factorize in fft.h at
// c1ed01b8fda69d2a378190f0915f1a19762dddbf. Execution uses an independent
// multipass schedule with caller-owned scratch and alternating buffers.

use std::fmt;

use num_complex::Complex;
use num_traits::{Float, FromPrimitive};

use crate::simd::{
    ComplexGeneralPrimeIdo1Kernel, ComplexGeneralPrimeIdo1Params, ComplexPassKernel, SimdPass,
    SimdScalar,
};

mod radix11_ido1;

pub(crate) const P241_BLUESTEIN_CONVOLUTION_LEN: usize = 512;

/// Errors returned by FFT plan construction and execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FftError {
    /// The transform length must be positive.
    InvalidLength,
    /// The requested plan or workspace could not be allocated.
    AllocationFailed,
    /// An input, output, or workspace length did not match the plan.
    BufferLength {
        /// The name of the buffer with an invalid length.
        buffer: &'static str,
        /// The length required by the plan.
        expected: usize,
        /// The length supplied by the caller.
        actual: usize,
    },
    /// A flat batch buffer does not contain an integral number of rows.
    BatchBufferLength {
        /// The name of the buffer with an invalid length.
        buffer: &'static str,
        /// The required length of each row.
        row_len: usize,
        /// The length supplied by the caller.
        actual: usize,
    },
    /// A requested flat batch element count overflows `usize`.
    BatchLengthOverflow {
        /// The name of the buffer whose required length overflowed.
        buffer: &'static str,
        /// The number of rows in the batch.
        rows: usize,
        /// The length required for each row.
        row_len: usize,
    },
}

impl fmt::Display for FftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength => write!(f, "FFT length must be positive"),
            Self::AllocationFailed => write!(f, "FFT plan or workspace allocation failed"),
            Self::BufferLength {
                buffer,
                expected,
                actual,
            } => write!(
                f,
                "{buffer} length is {actual}; the plan requires {expected}"
            ),
            Self::BatchBufferLength {
                buffer,
                row_len,
                actual,
            } => write!(
                f,
                "{buffer} length is {actual}; batch rows must be a multiple of {row_len}"
            ),
            Self::BatchLengthOverflow {
                buffer,
                rows,
                row_len,
            } => write!(
                f,
                "{buffer} element count overflows for {rows} batch rows of length {row_len}"
            ),
        }
    }
}

impl std::error::Error for FftError {}

/// An immutable one-dimensional complex FFT plan.
///
/// Plans support `f32` and `f64`. Forward transforms use the negative
/// exponential and are unnormalized. Inverse transforms use the positive
/// exponential and include a `1/n` scale. A plan can be shared between
/// threads; each concurrent call needs a separate [`ComplexFftWorkspace`].
pub struct ComplexFftPlan<T> {
    data: ComplexPlanData<T>,
}

struct ComplexPlanData<T> {
    len: usize,
    passes: Vec<ComplexPass<T>>,
    roots: Vec<Complex<T>>,
    max_radix: usize,
    bluestein: Vec<Option<BluesteinPlan<T>>>,
}

struct ComplexPass<T> {
    radix: usize,
    l1: usize,
    ido: usize,
    twiddles: Vec<Complex<T>>,
    prime_phase_indices: Vec<u8>,
}

struct PassExecution<'a, T> {
    roots: &'a [Complex<T>],
    total_len: usize,
    direction: Direction,
    defer_inverse_normalization: bool,
    inverse_output_scale: T,
    radix_input: &'a mut [Complex<T>],
    radix_output: &'a mut [Complex<T>],
    blue_plan: Option<&'a BluesteinPlan<T>>,
    prime_phase_indices: &'a [u8],
    blue_buffer: &'a mut [Complex<T>],
    blue_workspace: Option<&'a mut ComplexFftWorkspace<T>>,
    simd: Option<SimdKernels<T>>,
}

type FirstPassKernel<T> = unsafe fn(&mut [Complex<T>], &SimdPass<'_, T>);
#[derive(Clone, Copy)]
struct SimdKernels<T> {
    complex_lanes: usize,
    first_pass: FirstPassKernel<T>,
    pass: ComplexPassKernel<T>,
    ido1_radix8_pass: Option<ComplexPassKernel<T>>,
    ido1_radix4_pass: Option<ComplexPassKernel<T>>,
    radix11_pass: Option<ComplexPassKernel<T>>,
}

/// Mutable scratch for repeated execution of a [`ComplexFftPlan`].
///
/// A workspace can be reused after each operation completes. It must not be
/// used by concurrent operations at the same time.
pub struct ComplexFftWorkspace<T> {
    len: usize,
    output: Vec<Complex<T>>,
    radix_input: Vec<Complex<T>>,
    radix_output: Vec<Complex<T>>,
    bluestein_buffer: Vec<Complex<T>>,
    bluestein_workspaces: Vec<Option<ComplexFftWorkspace<T>>>,
}

impl<T> ComplexFftWorkspace<T> {
    /// Returns the transform length for which this workspace was allocated.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if this workspace has no elements.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Returns the byte size of all reusable scratch buffers in this
    /// workspace, including nested Bluestein workspaces.
    ///
    /// The count includes initialized buffer elements and excludes allocator
    /// metadata and the plan's precomputed tables. Returns `None` only if the
    /// total byte count overflows `usize`.
    pub fn scratch_bytes(&self) -> Option<usize> {
        let local_elements = [
            self.output.len(),
            self.radix_input.len(),
            self.radix_output.len(),
            self.bluestein_buffer.len(),
        ]
        .into_iter()
        .try_fold(0usize, usize::checked_add)?;
        let local_bytes = local_elements.checked_mul(std::mem::size_of::<Complex<T>>())?;
        self.bluestein_workspaces
            .iter()
            .flatten()
            .try_fold(local_bytes, |total, workspace| {
                total.checked_add(workspace.scratch_bytes()?)
            })
    }
}

impl<T: Float + FromPrimitive> ComplexFftPlan<T> {
    pub(crate) fn try_new(len: usize) -> Result<Self, FftError> {
        Self::try_new_with_bluestein(len, true)
    }

    fn try_new_with_bluestein(len: usize, enable_bluestein: bool) -> Result<Self, FftError> {
        if len == 0 {
            return Err(FftError::InvalidLength);
        }

        check_vec_len::<Complex<T>>(len)?;
        let factors = factorize(len)?;
        let mut roots = try_vec(len, Complex::new(T::zero(), T::zero()))?;
        let max_radix = factors.iter().copied().max().unwrap_or(1);
        for (index, root) in roots.iter_mut().enumerate() {
            *root = unity_root(index, len);
        }
        let mut passes = Vec::new();
        passes
            .try_reserve_exact(factors.len())
            .map_err(|_| FftError::AllocationFailed)?;
        let mut l1 = 1_usize;
        for &radix in &factors {
            let pass_span = l1.checked_mul(radix).ok_or(FftError::AllocationFailed)?;
            let ido = len / pass_span;
            let twiddle_count = radix.checked_mul(ido).ok_or(FftError::AllocationFailed)?;
            let mut twiddles = try_vec(twiddle_count, Complex::new(T::zero(), T::zero()))?;
            for branch in 0..radix {
                for inner in 0..ido {
                    // The factorization guarantees branch*l1*inner < len.
                    let root_index = branch * l1 * inner;
                    twiddles[branch * ido + inner] = roots[root_index];
                }
            }
            let mut prime_phase_indices = Vec::new();
            if (13..110).contains(&radix) && !radix.is_multiple_of(2) {
                let phase_count = ((radix - 1) / 2)
                    .checked_mul(radix)
                    .ok_or(FftError::AllocationFailed)?;
                prime_phase_indices = try_vec(phase_count, 0_u8)?;
                for branch in 1..=(radix - 1) / 2 {
                    for frequency in 0..radix {
                        let phase = bounded_product_mod(branch, frequency, radix);
                        prime_phase_indices[(branch - 1) * radix + frequency] =
                            u8::try_from(phase).map_err(|_| FftError::AllocationFailed)?;
                    }
                }
            }
            passes.push(ComplexPass {
                radix,
                l1,
                ido,
                twiddles,
                prime_phase_indices,
            });
            l1 = pass_span;
        }
        let mut bluestein = Vec::new();
        bluestein
            .try_reserve_exact(factors.len())
            .map_err(|_| FftError::AllocationFailed)?;
        for &radix in &factors {
            bluestein.push(if enable_bluestein && radix >= 110 {
                let plan = if radix == 241 {
                    BluesteinPlan::try_new_with_convolution_len(
                        radix,
                        P241_BLUESTEIN_CONVOLUTION_LEN,
                    )?
                } else {
                    BluesteinPlan::try_new(radix)?
                };
                Some(plan)
            } else {
                None
            });
        }
        Ok(Self {
            data: ComplexPlanData {
                len,
                passes,
                roots,
                max_radix,
                bluestein,
            },
        })
    }

    #[cfg(test)]
    pub(crate) fn m0_pass_report(&self) -> Vec<String> {
        self.data
            .passes
            .iter()
            .enumerate()
            .map(|(stage, pass)| {
                let route = if let Some(blue) = self.data.bluestein[stage].as_ref() {
                    format!("bluestein conv={}", blue.convolution_len)
                } else if matches!(pass.radix, 2 | 3 | 4 | 5 | 7 | 8 | 11) {
                    String::from("fixed butterfly")
                } else if pass.radix.is_multiple_of(2) {
                    String::from("direct even radix")
                } else {
                    String::from("general prime")
                };
                format!(
                    "n={} stage={} radix={} l1={} ido={} route={}",
                    self.data.len, stage, pass.radix, pass.l1, pass.ido, route
                )
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn m0_retained_plan_bytes(&self) -> usize {
        let mut bytes = self.data.roots.capacity() * std::mem::size_of::<Complex<T>>();
        bytes += self.data.passes.capacity() * std::mem::size_of::<ComplexPass<T>>();
        bytes += self.data.bluestein.capacity() * std::mem::size_of::<Option<BluesteinPlan<T>>>();
        for pass in &self.data.passes {
            bytes += pass.twiddles.capacity() * std::mem::size_of::<Complex<T>>();
            bytes += pass.prime_phase_indices.capacity() * std::mem::size_of::<u8>();
        }
        for blue in self.data.bluestein.iter().flatten() {
            bytes += blue.m0_retained_plan_bytes();
        }
        bytes
    }

    pub(crate) fn try_workspace(&self) -> Result<ComplexFftWorkspace<T>, FftError> {
        let zero = Complex::new(T::zero(), T::zero());
        let mut bluestein_buffer_len = 0;
        let mut bluestein_workspaces = Vec::new();
        bluestein_workspaces
            .try_reserve_exact(self.data.bluestein.len())
            .map_err(|_| FftError::AllocationFailed)?;
        for plan in &self.data.bluestein {
            if let Some(plan) = plan {
                bluestein_buffer_len = bluestein_buffer_len.max(plan.convolution_len);
                bluestein_workspaces.push(Some(plan.convolution.try_workspace()?));
            } else {
                bluestein_workspaces.push(None);
            }
        }
        Ok(ComplexFftWorkspace {
            len: self.data.len,
            output: try_vec(self.data.len, zero)?,
            radix_input: try_vec(self.data.max_radix, zero)?,
            radix_output: try_vec(self.data.max_radix, zero)?,
            bluestein_buffer: try_vec(bluestein_buffer_len, zero)?,
            bluestein_workspaces,
        })
    }

    pub(crate) fn execute(
        &self,
        values: &mut [Complex<T>],
        workspace: &mut ComplexFftWorkspace<T>,
        direction: Direction,
    ) -> Result<(), FftError> {
        self.execute_inverse_scaled(values, workspace, direction, T::one())
    }

    pub(crate) fn execute_inverse_scaled(
        &self,
        values: &mut [Complex<T>],
        workspace: &mut ComplexFftWorkspace<T>,
        direction: Direction,
        inverse_output_scale: T,
    ) -> Result<(), FftError> {
        self.execute_with_simd(values, workspace, direction, inverse_output_scale, None)
    }

    fn execute_avx2(
        &self,
        values: &mut [Complex<T>],
        workspace: &mut ComplexFftWorkspace<T>,
        direction: Direction,
    ) -> Result<(), FftError>
    where
        T: SimdScalar,
    {
        check_len("values", self.data.len, values.len())?;
        check_len("workspace", self.data.len, workspace.len)?;
        self.execute_avx2_unchecked(values, workspace, direction);
        Ok(())
    }

    pub(crate) fn execute_runtime(
        &self,
        values: &mut [Complex<T>],
        workspace: &mut ComplexFftWorkspace<T>,
        direction: Direction,
    ) -> Result<(), FftError>
    where
        T: SimdScalar,
    {
        self.execute_avx2(values, workspace, direction)
    }

    fn execute_avx2_unchecked(
        &self,
        values: &mut [Complex<T>],
        workspace: &mut ComplexFftWorkspace<T>,
        direction: Direction,
    ) where
        T: SimdScalar,
    {
        let avx2_available = T::avx2_available();
        let kernels = avx2_available.then_some(SimdKernels {
            complex_lanes: T::AVX2_COMPLEX_LANES,
            first_pass: T::avx2_first_pass,
            pass: T::avx2_pass,
            ido1_radix8_pass: T::avx2_ido1_radix8_pass_kernel(),
            ido1_radix4_pass: T::avx2_ido1_radix4_pass_kernel(),
            radix11_pass: T::avx2_radix11_dispatch_kernel(),
        });
        if avx2_available
            && self.data.len == 682
            && let Some(kernel) = T::avx2_general_prime_ido1_kernel()
        {
            self.execute_682_with_simd_unchecked(
                values,
                workspace,
                direction,
                kernels,
                kernel,
                T::avx2_ido31_radix11_kernel(),
            );
        } else if avx2_available
            && self.data.len == 1442
            && let Some(kernel) = T::avx2_general_prime_radix103_ido1_kernel()
        {
            self.execute_with_simd_unchecked::<true>(
                values,
                workspace,
                direction,
                T::one(),
                kernels,
                Some(kernel),
            );
        } else {
            self.execute_with_simd_unchecked::<false>(
                values,
                workspace,
                direction,
                T::one(),
                kernels,
                None,
            );
        }
    }

    fn execute_with_simd(
        &self,
        values: &mut [Complex<T>],
        workspace: &mut ComplexFftWorkspace<T>,
        direction: Direction,
        inverse_output_scale: T,
        simd: Option<SimdKernels<T>>,
    ) -> Result<(), FftError> {
        check_len("values", self.data.len, values.len())?;
        check_len("workspace", self.data.len, workspace.len)?;

        self.execute_with_simd_unchecked::<false>(
            values,
            workspace,
            direction,
            inverse_output_scale,
            simd,
            None,
        );
        Ok(())
    }

    fn execute_with_simd_unchecked<const ENABLE_RADIX103: bool>(
        &self,
        values: &mut [Complex<T>],
        workspace: &mut ComplexFftWorkspace<T>,
        direction: Direction,
        inverse_output_scale: T,
        simd: Option<SimdKernels<T>>,
        radix103_kernel: Option<ComplexGeneralPrimeIdo1Kernel<T>>,
    ) {
        let defer_inverse_normalization =
            direction == Direction::Inverse && inverse_normalization_is_safe(values, self.data.len);
        #[cfg(test)]
        crate::m0_record_route(format!(
            "c2c n={} normalization={}",
            self.data.len,
            if defer_inverse_normalization {
                "deferred"
            } else if direction == Direction::Inverse {
                "guarded"
            } else {
                "forward"
            }
        ));
        let mut current_is_values = true;
        for (stage, pass) in self.data.passes.iter().enumerate() {
            let blue = self.data.bluestein[stage].as_ref();
            #[cfg(test)]
            {
                let route = if let Some(blue) = blue {
                    format!("bluestein conv={}", blue.convolution_len)
                } else if matches!(pass.radix, 2 | 3 | 4 | 5 | 7 | 8 | 11) {
                    String::from("fixed")
                } else if pass.radix.is_multiple_of(2) {
                    String::from("direct-even")
                } else {
                    String::from("general-prime")
                };
                crate::m0_record_route(format!(
                    "c2c n={} stage={} radix={} l1={} ido={} route={}",
                    self.data.len, stage, pass.radix, pass.l1, pass.ido, route
                ));
            }
            let mut execution = PassExecution {
                roots: &self.data.roots,
                total_len: self.data.len,
                direction,
                defer_inverse_normalization,
                inverse_output_scale: if direction == Direction::Inverse
                    && stage + 1 == self.data.passes.len()
                {
                    if defer_inverse_normalization {
                        inverse_output_scale * inverse_scale::<T>(self.data.len)
                    } else {
                        inverse_output_scale
                    }
                } else {
                    T::one()
                },
                radix_input: &mut workspace.radix_input,
                radix_output: &mut workspace.radix_output,
                blue_plan: blue,
                prime_phase_indices: &pass.prime_phase_indices,
                blue_buffer: &mut workspace.bluestein_buffer,
                blue_workspace: workspace.bluestein_workspaces[stage].as_mut(),
                simd,
            };
            if stage == 0 {
                execute_first_pass_in_place(values, &mut workspace.output, pass, &mut execution);
            } else if current_is_values {
                execute_pass::<ENABLE_RADIX103, T>(
                    values,
                    &mut workspace.output,
                    pass,
                    &mut execution,
                    radix103_kernel,
                );
                current_is_values = false;
            } else {
                execute_pass::<ENABLE_RADIX103, T>(
                    &workspace.output,
                    values,
                    pass,
                    &mut execution,
                    radix103_kernel,
                );
                current_is_values = true;
            }
        }

        if !current_is_values {
            values.copy_from_slice(&workspace.output);
        }
    }

    fn execute_682_with_simd_unchecked(
        &self,
        values: &mut [Complex<T>],
        workspace: &mut ComplexFftWorkspace<T>,
        direction: Direction,
        simd: Option<SimdKernels<T>>,
        general_prime_ido1: ComplexGeneralPrimeIdo1Kernel<T>,
        radix11_pass: Option<ComplexPassKernel<T>>,
    ) {
        debug_assert_eq!(self.data.len, 682);
        let defer_inverse_normalization =
            direction == Direction::Inverse && inverse_normalization_is_safe(values, self.data.len);
        #[cfg(test)]
        crate::m0_record_route(format!(
            "c2c n={} normalization={}",
            self.data.len,
            if defer_inverse_normalization {
                "deferred"
            } else if direction == Direction::Inverse {
                "guarded"
            } else {
                "forward"
            }
        ));

        let mut current_is_values = true;
        for (stage, pass) in self.data.passes.iter().enumerate() {
            let blue = self.data.bluestein[stage].as_ref();
            #[cfg(test)]
            {
                let route = if let Some(blue) = blue {
                    format!("bluestein conv={}", blue.convolution_len)
                } else if matches!(pass.radix, 2 | 3 | 4 | 5 | 7 | 8 | 11) {
                    String::from("fixed")
                } else if pass.radix.is_multiple_of(2) {
                    String::from("direct-even")
                } else {
                    String::from("general-prime")
                };
                crate::m0_record_route(format!(
                    "c2c n={} stage={} radix={} l1={} ido={} route={}",
                    self.data.len, stage, pass.radix, pass.l1, pass.ido, route
                ));
            }
            let mut execution = PassExecution {
                roots: &self.data.roots,
                total_len: self.data.len,
                direction,
                defer_inverse_normalization,
                inverse_output_scale: if direction == Direction::Inverse
                    && stage + 1 == self.data.passes.len()
                {
                    if defer_inverse_normalization {
                        inverse_scale::<T>(self.data.len)
                    } else {
                        T::one()
                    }
                } else {
                    T::one()
                },
                radix_input: &mut workspace.radix_input,
                radix_output: &mut workspace.radix_output,
                blue_plan: blue,
                prime_phase_indices: &pass.prime_phase_indices,
                blue_buffer: &mut workspace.bluestein_buffer,
                blue_workspace: workspace.bluestein_workspaces[stage].as_mut(),
                simd,
            };

            if stage == 0 {
                execute_first_pass_in_place(values, &mut workspace.output, pass, &mut execution);
            } else if current_is_values {
                if !execute_682_radix11_ido31(
                    values,
                    &mut workspace.output,
                    pass,
                    &mut execution,
                    radix11_pass,
                ) && !execute_682_general_prime_ido1(
                    values,
                    &mut workspace.output,
                    pass,
                    &mut execution,
                    general_prime_ido1,
                ) {
                    execute_pass::<false, T>(
                        values,
                        &mut workspace.output,
                        pass,
                        &mut execution,
                        None,
                    );
                }
                current_is_values = false;
            } else {
                if !execute_682_radix11_ido31(
                    &workspace.output,
                    values,
                    pass,
                    &mut execution,
                    radix11_pass,
                ) && !execute_682_general_prime_ido1(
                    &workspace.output,
                    values,
                    pass,
                    &mut execution,
                    general_prime_ido1,
                ) {
                    execute_pass::<false, T>(&workspace.output, values, pass, &mut execution, None);
                }
                current_is_values = true;
            }
        }

        if !current_is_values {
            values.copy_from_slice(&workspace.output);
        }
    }
}

fn execute_682_radix11_ido31<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    pass: &ComplexPass<T>,
    execution: &mut PassExecution<'_, T>,
    kernel: Option<ComplexPassKernel<T>>,
) -> bool {
    if pass.radix != 11 || pass.l1 != 2 || pass.ido != 31 {
        return false;
    }
    execute_radix11_avx2(input, output, pass, execution, kernel, false)
}

fn execute_radix11_avx2<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    pass: &ComplexPass<T>,
    execution: &mut PassExecution<'_, T>,
    kernel: Option<ComplexPassKernel<T>>,
    kernel_processes_odd_ido_tail: bool,
) -> bool {
    let Some(kernel) = kernel else {
        return false;
    };
    let inverse = execution.direction == Direction::Inverse;
    if execution.blue_plan.is_some()
        || pass.radix != 11
        || pass.ido < 2
        || (inverse && !execution.defer_inverse_normalization)
    {
        return false;
    }
    let vector_ido = pass.ido - pass.ido % 2;

    let simd_pass = SimdPass {
        radix: pass.radix,
        l1: pass.l1,
        ido: pass.ido,
        twiddles: &pass.twiddles,
        inverse,
        inverse_scale: if inverse {
            execution.inverse_output_scale
        } else {
            T::one()
        },
    };
    // SAFETY: runtime dispatch checked AVX2, plan construction validates the
    // pass dimensions and twiddle table, the slices are disjoint, and unsafe
    // inverse inputs fall back to the scalar path above.
    unsafe { kernel(input, output, &simd_pass) };

    let scalar_tail_outer_start =
        if kernel_processes_odd_ido_tail && !pass.ido.is_multiple_of(2) && pass.l1 >= 2 {
            pass.l1 - pass.l1 % 2
        } else {
            0
        };

    let radix_root_stride = execution.total_len / pass.radix;
    let output_scale = execution.inverse_output_scale;
    for outer in scalar_tail_outer_start..pass.l1 {
        for inner in vector_ido..pass.ido {
            for (branch, slot) in execution
                .radix_input
                .iter_mut()
                .enumerate()
                .take(pass.radix)
            {
                let source_index = inner + pass.ido * (branch + pass.radix * outer);
                *slot = input[source_index];
            }
            execution.transform::<11>(11, radix_root_stride);
            for (branch, &transformed) in execution.radix_output.iter().enumerate().take(pass.radix)
            {
                let destination_index = inner + pass.ido * (outer + pass.l1 * branch);
                let mut value = if branch != 0 && pass.ido > 1 {
                    transformed
                        * directed_root(
                            &pass.twiddles,
                            branch * pass.ido + inner,
                            execution.direction,
                        )
                } else {
                    transformed
                };
                if inverse {
                    value = value * output_scale;
                }
                output[destination_index] = value;
            }
        }
    }
    #[cfg(test)]
    if scalar_tail_outer_start > 0 {
        crate::m0_record_route(format!(
            "c2c-avx2:radix11-odd-ido-tail n={} l1={} ido={} vector_l1={} scalar_l1_tail={}",
            execution.total_len,
            pass.l1,
            pass.ido,
            scalar_tail_outer_start,
            pass.l1 - scalar_tail_outer_start
        ));
    }
    #[cfg(test)]
    crate::m0_record_route(format!(
        "c2c-avx2:radix11-pass n={} l1={} ido={} vector_ido={} scalar_tail={}",
        execution.total_len,
        pass.l1,
        pass.ido,
        vector_ido,
        pass.ido - vector_ido
    ));
    true
}

fn execute_682_general_prime_ido1<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    pass: &ComplexPass<T>,
    execution: &mut PassExecution<'_, T>,
    kernel: ComplexGeneralPrimeIdo1Kernel<T>,
) -> bool {
    let inverse = execution.direction == Direction::Inverse;
    if execution.blue_plan.is_some()
        || pass.radix != 31
        || pass.ido != 1
        || pass.l1 <= 1
        || execution.prime_phase_indices.is_empty()
        || (inverse && !execution.defer_inverse_normalization)
    {
        return false;
    }

    let scale = if inverse {
        execution.inverse_output_scale
    } else {
        T::one()
    };
    let params = ComplexGeneralPrimeIdo1Params {
        l1: pass.l1,
        roots: execution.roots,
        root_stride: execution.total_len / pass.radix,
        phase_indices: execution.prime_phase_indices,
        inverse,
        scale,
    };
    // SAFETY: this helper is called only from the N=682 runtime-AVX2 path;
    // the pass dimensions and phase table are plan-validated, and inverse
    // inputs passed the deferred-normalization safety check.
    unsafe { kernel(input, output, params) };
    #[cfg(test)]
    crate::m0_record_route(format!(
        "c2c-avx2:general-prime-ido1 n=682 radix=31 l1={} vector_frequencies=14 scalar_tail=1",
        pass.l1
    ));
    true
}

fn execute_1442_general_prime_ido1<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    pass: &ComplexPass<T>,
    execution: &mut PassExecution<'_, T>,
    kernel: Option<ComplexGeneralPrimeIdo1Kernel<T>>,
) -> bool {
    let Some(kernel) = kernel else {
        return false;
    };
    let inverse = execution.direction == Direction::Inverse;
    if execution.total_len != 1442
        || execution.blue_plan.is_some()
        || pass.radix != 103
        || pass.l1 != 14
        || pass.ido != 1
        || execution.prime_phase_indices.len() < 51 * 103
        || (inverse && !execution.defer_inverse_normalization)
    {
        return false;
    }

    let params = ComplexGeneralPrimeIdo1Params {
        l1: pass.l1,
        roots: execution.roots,
        root_stride: execution.total_len / pass.radix,
        phase_indices: execution.prime_phase_indices,
        inverse,
        scale: if inverse {
            execution.inverse_output_scale
        } else {
            T::one()
        },
    };
    // SAFETY: runtime dispatch checked AVX2, the 1442 plan validates the
    // radix, layout, roots and phase table, the buffers are disjoint, and an
    // inverse reaches this kernel only after the range-safety check.
    unsafe { kernel(input, output, params) };
    #[cfg(test)]
    crate::m0_record_route(String::from(
        "c2c-avx2:general-prime-ido1 n=1442 radix=103 l1=14 vector_frequencies=50 scalar_tail=1",
    ));
    true
}

macro_rules! implement_public_api {
    ($scalar:ty) => {
        impl ComplexFftPlan<$scalar> {
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

            /// Returns the transform length.
            pub fn len(&self) -> usize {
                self.data.len
            }

            /// Returns `true` if the plan length is zero. Plans reject zero
            /// lengths, so this always returns `false`.
            pub fn is_empty(&self) -> bool {
                false
            }

            /// Allocates reusable execution scratch for this plan.
            ///
            /// # Errors
            ///
            /// Returns [`FftError::AllocationFailed`] if scratch storage
            /// cannot be reserved.
            pub fn workspace(&self) -> Result<ComplexFftWorkspace<$scalar>, FftError> {
                self.try_workspace()
            }

            /// Executes an unnormalized forward complex transform in place.
            ///
            /// # Errors
            ///
            /// Returns [`FftError::BufferLength`] if `values` or `workspace`
            /// has a length different from this plan.
            pub fn forward(
                &self,
                values: &mut [Complex<$scalar>],
                workspace: &mut ComplexFftWorkspace<$scalar>,
            ) -> Result<(), FftError> {
                self.execute_avx2(values, workspace, Direction::Forward)
            }

            /// Executes forward transforms for contiguous, row-major complex
            /// data using one reusable workspace.
            ///
            /// The buffer length must be an integer multiple of the plan
            /// length. A zero-length batch is a no-op. Output rows retain
            /// input order; this method does not create threads or allocate
            /// scratch.
            ///
            /// # Errors
            ///
            /// Returns [`FftError::BatchBufferLength`] if `values` does not
            /// contain complete rows, or [`FftError::BufferLength`] if the
            /// workspace belongs to a different plan length.
            pub fn forward_batch(
                &self,
                values: &mut [Complex<$scalar>],
                workspace: &mut ComplexFftWorkspace<$scalar>,
            ) -> Result<(), FftError> {
                check_batch_rows("values", self.data.len, values.len())?;
                check_len("workspace", self.data.len, workspace.len)?;
                for row in values.chunks_exact_mut(self.data.len) {
                    self.execute_avx2_unchecked(row, workspace, Direction::Forward);
                }
                Ok(())
            }

            /// Executes an inverse complex transform in place and scales the
            /// result by `1/n`.
            ///
            /// # Errors
            ///
            /// Returns [`FftError::BufferLength`] if `values` or `workspace`
            /// has a length different from this plan.
            pub fn inverse(
                &self,
                values: &mut [Complex<$scalar>],
                workspace: &mut ComplexFftWorkspace<$scalar>,
            ) -> Result<(), FftError> {
                self.execute_avx2(values, workspace, Direction::Inverse)
            }

            /// Executes normalized inverse transforms for contiguous,
            /// row-major complex data using one reusable workspace.
            ///
            /// The buffer length must be an integer multiple of the plan
            /// length. A zero-length batch is a no-op. Output rows retain
            /// input order; this method does not create threads or allocate
            /// scratch.
            ///
            /// # Errors
            ///
            /// Returns [`FftError::BatchBufferLength`] if `values` does not
            /// contain complete rows, or [`FftError::BufferLength`] if the
            /// workspace belongs to a different plan length.
            pub fn inverse_batch(
                &self,
                values: &mut [Complex<$scalar>],
                workspace: &mut ComplexFftWorkspace<$scalar>,
            ) -> Result<(), FftError> {
                check_batch_rows("values", self.data.len, values.len())?;
                check_len("workspace", self.data.len, workspace.len)?;
                for row in values.chunks_exact_mut(self.data.len) {
                    self.execute_avx2_unchecked(row, workspace, Direction::Inverse);
                }
                Ok(())
            }
        }
    };
}

implement_public_api!(f32);
implement_public_api!(f64);

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum Direction {
    Forward,
    Inverse,
}

pub(crate) struct BluesteinPlan<T> {
    len: usize,
    convolution_len: usize,
    convolution_scale: T,
    kernel_spectrum_max: T,
    chirp: Vec<Complex<T>>,
    kernel_spectrum: Vec<Complex<T>>,
    convolution: ComplexFftPlan<T>,
}

impl<T: Float + FromPrimitive> BluesteinPlan<T> {
    pub(crate) fn try_new(len: usize) -> Result<Self, FftError> {
        Self::try_new_with_pad(len, None)
    }

    pub(crate) fn try_new_with_convolution_len(
        len: usize,
        convolution_len: usize,
    ) -> Result<Self, FftError> {
        Self::try_new_with_pad(len, Some(convolution_len))
    }

    fn try_new_with_pad(len: usize, selected_len: Option<usize>) -> Result<Self, FftError> {
        let twice_len = len.checked_mul(2).ok_or(FftError::AllocationFailed)?;
        let minimum = twice_len.checked_sub(1).ok_or(FftError::AllocationFailed)?;
        let convolution_len = if let Some(convolution_len) = selected_len {
            let factors = factorize(convolution_len)?;
            if convolution_len < minimum
                || factors
                    .iter()
                    .any(|&factor| !matches!(factor, 2 | 3 | 4 | 5 | 7 | 8 | 11))
            {
                return Err(FftError::InvalidLength);
            }
            convolution_len
        } else {
            good_size_complex(minimum)?
        };
        let convolution = ComplexFftPlan::try_new_with_bluestein(convolution_len, false)?;
        let convolution_scale = T::from_usize(convolution_len).ok_or(FftError::AllocationFailed)?;
        let zero = Complex::new(T::zero(), T::zero());
        let mut chirp = try_vec(len, zero)?;
        let mut kernel = try_vec(convolution_len, zero)?;

        for (index, value) in chirp.iter_mut().enumerate() {
            let square = mul_mod(index, index, twice_len);
            *value = unity_root(square, twice_len).conj();
        }

        kernel[0] = Complex::new(T::one(), T::zero());
        for index in 1..len {
            let positive_chirp = chirp[index].conj();
            kernel[index] = positive_chirp;
            kernel[convolution_len - index] = positive_chirp;
        }

        let mut kernel_workspace = convolution.try_workspace()?;
        convolution.execute(&mut kernel, &mut kernel_workspace, Direction::Forward)?;
        let kernel_spectrum_max = kernel.iter().fold(T::zero(), |maximum, value| {
            maximum.max(value.re.abs()).max(value.im.abs())
        });

        Ok(Self {
            len,
            convolution_len,
            convolution_scale,
            kernel_spectrum_max,
            chirp,
            kernel_spectrum: kernel,
            convolution,
        })
    }

    pub(crate) fn convolution_len(&self) -> usize {
        self.convolution_len
    }

    #[cfg(test)]
    pub(crate) fn m0_retained_plan_bytes(&self) -> usize {
        self.chirp.capacity() * std::mem::size_of::<Complex<T>>()
            + self.kernel_spectrum.capacity() * std::mem::size_of::<Complex<T>>()
            + self.convolution.m0_retained_plan_bytes()
    }

    pub(crate) fn convolution_workspace(&self) -> Result<ComplexFftWorkspace<T>, FftError> {
        self.convolution.try_workspace()
    }

    pub(crate) fn execute(
        &self,
        input: &[Complex<T>],
        output: &mut [Complex<T>],
        direction: Direction,
        convolution_buffer: &mut [Complex<T>],
        convolution_workspace: &mut ComplexFftWorkspace<T>,
    ) {
        debug_assert_eq!(input.len(), self.len);
        debug_assert_eq!(output.len(), self.len);
        debug_assert!(convolution_buffer.len() >= self.convolution_len);

        let input_max = input.iter().fold(T::zero(), |maximum, value| {
            maximum.max(value.re.abs()).max(value.im.abs())
        });
        let subnormal_safe_limit =
            T::min_positive_value() * self.convolution_scale * scalar::<T>(4.0);
        let input_scale = if input_max > T::zero() && input_max < subnormal_safe_limit {
            let reciprocal = T::one() / input_max;
            if reciprocal.is_finite() {
                reciprocal
            } else {
                T::max_value()
            }
        } else if input_max > T::max_value() / (self.convolution_scale * scalar::<T>(4.0)) {
            T::max_value() / (self.convolution_scale * scalar::<T>(4.0)) / input_max
        } else {
            T::one()
        };
        let buffer = &mut convolution_buffer[..self.convolution_len];
        for index in 0..self.len {
            let chirp = match direction {
                Direction::Forward => self.chirp[index],
                Direction::Inverse => self.chirp[index].conj(),
            };
            buffer[index] = input[index] * input_scale * chirp;
        }
        buffer[self.len..].fill(Complex::new(T::zero(), T::zero()));

        self.convolution
            .execute(buffer, convolution_workspace, Direction::Forward)
            .expect("Bluestein convolution workspace matches its plan");
        let input_spectrum_max = buffer.iter().fold(T::zero(), |maximum, value| {
            maximum.max(value.re.abs()).max(value.im.abs())
        });
        let unscaled_kernel_limit = T::max_value() / (self.kernel_spectrum_max * scalar::<T>(4.0));
        let scale_kernel = input_spectrum_max > unscaled_kernel_limit;
        let kernel_scale = if scale_kernel {
            T::one() / self.convolution_scale
        } else {
            T::one()
        };
        for (frequency, value) in buffer.iter_mut().enumerate() {
            let kernel = match direction {
                Direction::Forward => self.kernel_spectrum[frequency],
                Direction::Inverse => {
                    let reverse = if frequency == 0 {
                        0
                    } else {
                        self.convolution_len - frequency
                    };
                    self.kernel_spectrum[reverse].conj()
                }
            } * kernel_scale;
            *value = *value * kernel;
        }
        self.convolution
            .execute(buffer, convolution_workspace, Direction::Inverse)
            .expect("Bluestein convolution workspace matches its plan");

        for index in 0..self.len {
            let chirp = match direction {
                Direction::Forward => self.chirp[index],
                Direction::Inverse => self.chirp[index].conj(),
            };
            let mut value = buffer[index] * chirp;
            if scale_kernel {
                value = value * self.convolution_scale;
            }
            output[index] = value / input_scale;
        }
    }
}

fn execute_first_pass_in_place<T: Float + FromPrimitive>(
    data: &mut [Complex<T>],
    backup: &mut [Complex<T>],
    pass: &ComplexPass<T>,
    execution: &mut PassExecution<'_, T>,
) {
    if execution.blue_plan.is_some() {
        execute_first_pass_in_place_inner::<0, T>(data, backup, pass, execution);
        return;
    }
    match pass.radix {
        2 => execute_first_pass_in_place_inner::<2, T>(data, backup, pass, execution),
        3 => execute_first_pass_in_place_inner::<3, T>(data, backup, pass, execution),
        4 => execute_first_pass_in_place_inner::<4, T>(data, backup, pass, execution),
        5 => execute_first_pass_in_place_inner::<5, T>(data, backup, pass, execution),
        7 => execute_first_pass_in_place_inner::<7, T>(data, backup, pass, execution),
        8 => execute_first_pass_in_place_inner::<8, T>(data, backup, pass, execution),
        11 => execute_first_pass_in_place_inner::<11, T>(data, backup, pass, execution),
        _ => execute_first_pass_in_place_inner::<0, T>(data, backup, pass, execution),
    }
}

fn execute_first_pass_in_place_inner<const R: usize, T: Float + FromPrimitive>(
    data: &mut [Complex<T>],
    backup: &mut [Complex<T>],
    pass: &ComplexPass<T>,
    execution: &mut PassExecution<'_, T>,
) {
    debug_assert_eq!(pass.l1, 1);
    let radix = if R == 0 { pass.radix } else { R };
    debug_assert_eq!(radix, pass.radix);
    if matches!(R, 2 | 3 | 4 | 5 | 7 | 8) {
        execute_first_pass_direct_radix::<R, T>(data, pass, execution);
        return;
    }
    let root_stride = execution.total_len / radix;
    let inverse = execution.direction == Direction::Inverse;
    let stage_scale = if inverse && !execution.defer_inverse_normalization {
        inverse_scale::<T>(radix)
    } else {
        T::one()
    };
    let output_scale = execution.inverse_output_scale;
    let combined_scale = stage_scale * output_scale;
    let radix_value = scalar::<T>(radix as f64);
    let overflow_threshold = T::max_value() / scalar::<T>(4.0 * radix as f64);
    // When ido is one every stage twiddle is exactly unity. DUCC's cfftp
    // passes use a separate no-twiddle loop for this case.
    let has_stage_twiddles = pass.ido > 1;

    for inner in 0..pass.ido {
        let mut max_component = T::zero();
        for (branch, slot) in execution.radix_input.iter_mut().enumerate().take(radix) {
            let source_index = inner + pass.ido * branch;
            let value = data[source_index];
            if inverse && !execution.defer_inverse_normalization {
                max_component = max_component.max(value.re.abs()).max(value.im.abs());
            }
            *slot = value;
        }
        let prescale =
            inverse && !execution.defer_inverse_normalization && max_component > overflow_threshold;
        if prescale {
            for value in &mut execution.radix_input[..radix] {
                *value = *value * stage_scale;
            }
        }
        execution.transform::<R>(radix, root_stride);

        if prescale {
            for (branch, &transformed) in execution.radix_output.iter().enumerate().take(radix) {
                let index = inner + pass.ido * branch;
                backup[index] = if branch != 0 && has_stage_twiddles {
                    transformed
                        * directed_root(
                            &pass.twiddles,
                            branch * pass.ido + inner,
                            execution.direction,
                        )
                } else {
                    transformed
                };
            }
            for (branch, slot) in execution.radix_input.iter_mut().enumerate().take(radix) {
                let index = inner + pass.ido * branch;
                let original = data[index];
                let scaled = original * stage_scale;
                *slot = Complex::new(
                    prescale_residual(original.re, scaled.re, radix_value),
                    prescale_residual(original.im, scaled.im, radix_value),
                );
            }
            execution.transform::<R>(radix, root_stride);
            for (branch, &correction) in execution.radix_output.iter().enumerate().take(radix) {
                let index = inner + pass.ido * branch;
                let correction = if branch != 0 && has_stage_twiddles {
                    correction
                        * directed_root(
                            &pass.twiddles,
                            branch * pass.ido + inner,
                            execution.direction,
                        )
                } else {
                    correction
                };
                backup[index] = (backup[index] + correction * stage_scale) * output_scale;
                data[index] = backup[index];
            }
        } else {
            for (branch, &transformed) in execution.radix_output.iter().enumerate().take(radix) {
                let index = inner + pass.ido * branch;
                let mut value = if branch != 0 && has_stage_twiddles {
                    transformed
                        * directed_root(
                            &pass.twiddles,
                            branch * pass.ido + inner,
                            execution.direction,
                        )
                } else {
                    transformed
                };
                if inverse {
                    value = value * combined_scale;
                }
                data[index] = value;
            }
        }
    }
}

fn execute_pass<const ENABLE_RADIX103: bool, T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    pass: &ComplexPass<T>,
    execution: &mut PassExecution<'_, T>,
    radix103_kernel: Option<ComplexGeneralPrimeIdo1Kernel<T>>,
) {
    if ENABLE_RADIX103
        && pass.radix == 103
        && pass.l1 == 14
        && pass.ido == 1
        && execute_1442_general_prime_ido1(input, output, pass, execution, radix103_kernel)
    {
        return;
    }
    if execution.blue_plan.is_some() {
        execute_pass_inner::<0, T>(input, output, pass, execution);
        return;
    }

    match pass.radix {
        2 => execute_pass_inner::<2, T>(input, output, pass, execution),
        3 => execute_pass_inner::<3, T>(input, output, pass, execution),
        4 => execute_pass_inner::<4, T>(input, output, pass, execution),
        5 => execute_pass_inner::<5, T>(input, output, pass, execution),
        7 => execute_pass_inner::<7, T>(input, output, pass, execution),
        8 => execute_pass_inner::<8, T>(input, output, pass, execution),
        11 => execute_pass_radix11(input, output, pass, execution),
        _ => execute_pass_inner::<0, T>(input, output, pass, execution),
    }
}

#[inline(never)]
fn execute_pass_radix11<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    pass: &ComplexPass<T>,
    execution: &mut PassExecution<'_, T>,
) {
    if pass.ido == 1 {
        if let Some(kernels) = execution.simd
            && radix11_ido1::execute_avx2(input, output, pass, execution, kernels.radix11_pass)
        {
            return;
        }
    } else if let Some(kernels) = execution.simd
        && execute_radix11_avx2(input, output, pass, execution, kernels.radix11_pass, true)
    {
        return;
    }
    execute_pass_inner::<11, T>(input, output, pass, execution);
}

fn execute_pass_inner<const R: usize, T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    pass: &ComplexPass<T>,
    execution: &mut PassExecution<'_, T>,
) {
    let radix = if R == 0 { pass.radix } else { R };
    debug_assert_eq!(radix, pass.radix);
    if matches!(R, 2 | 3 | 4 | 5 | 7 | 8) {
        execute_direct_radix_pass::<R, T>(input, output, pass, execution);
        return;
    }
    let inverse = execution.direction == Direction::Inverse;
    let radix_root_stride = execution.total_len / radix;
    let stage_scale = if inverse && !execution.defer_inverse_normalization {
        inverse_scale::<T>(radix)
    } else {
        T::one()
    };
    let output_scale = execution.inverse_output_scale;
    let combined_scale = stage_scale * output_scale;
    let radix_value = scalar::<T>(radix as f64);
    let overflow_threshold = T::max_value() / scalar::<T>(4.0 * radix as f64);
    let has_stage_twiddles = pass.ido > 1;

    // This l1/ido pass order matches cfft_multipass in pinned DUCC. The input
    // and output layouts place each radix's butterfly values contiguously in
    // the inner loop, and stage twiddles are applied after the fixed-radix
    // transform.
    for outer in 0..pass.l1 {
        for inner in 0..pass.ido {
            let mut max_component = T::zero();
            for (branch, slot) in execution.radix_input.iter_mut().enumerate().take(radix) {
                let source_index = inner + pass.ido * (branch + radix * outer);
                let value = input[source_index];
                if inverse && !execution.defer_inverse_normalization {
                    max_component = max_component.max(value.re.abs()).max(value.im.abs());
                }
                *slot = value;
            }
            let prescale = inverse
                && !execution.defer_inverse_normalization
                && max_component > overflow_threshold;
            if prescale {
                for value in &mut execution.radix_input[..radix] {
                    *value = *value * stage_scale;
                }
            }

            execution.transform::<R>(radix, radix_root_stride);

            for (branch, &transformed) in execution.radix_output.iter().enumerate().take(radix) {
                let destination_index = inner + pass.ido * (outer + pass.l1 * branch);
                let mut value = if branch != 0 && has_stage_twiddles {
                    transformed
                        * directed_root(
                            &pass.twiddles,
                            branch * pass.ido + inner,
                            execution.direction,
                        )
                } else {
                    transformed
                };
                if inverse && !prescale {
                    value = value * combined_scale;
                }
                output[destination_index] = value;
            }

            if prescale {
                // Recover contributions lost while scaling a high-magnitude
                // group. The original input remains available until this pass
                // completes; the correction transform is needed only here.
                for (branch, slot) in execution.radix_input.iter_mut().enumerate().take(radix) {
                    let source_index = inner + pass.ido * (branch + radix * outer);
                    let original = input[source_index];
                    let scaled = original * stage_scale;
                    *slot = Complex::new(
                        prescale_residual(original.re, scaled.re, radix_value),
                        prescale_residual(original.im, scaled.im, radix_value),
                    );
                }
                execution.transform::<R>(radix, radix_root_stride);
                for (branch, &correction) in execution.radix_output.iter().enumerate().take(radix) {
                    let destination_index = inner + pass.ido * (outer + pass.l1 * branch);
                    let correction = if branch != 0 && has_stage_twiddles {
                        correction
                            * directed_root(
                                &pass.twiddles,
                                branch * pass.ido + inner,
                                execution.direction,
                            )
                    } else {
                        correction
                    };
                    output[destination_index] =
                        (output[destination_index] + correction * stage_scale) * output_scale;
                }
            }
        }
    }
}

fn execute_first_pass_direct_radix<const R: usize, T: Float + FromPrimitive>(
    data: &mut [Complex<T>],
    pass: &ComplexPass<T>,
    execution: &PassExecution<'_, T>,
) {
    let radix = R;
    debug_assert!(matches!(radix, 2 | 3 | 4 | 5 | 7 | 8));
    debug_assert_eq!(pass.l1, 1);
    let inverse = execution.direction == Direction::Inverse;
    let mut scalar_start = 0;
    if let Some(kernels) = execution.simd
        && matches!(R, 2 | 4 | 8)
        && (!inverse || execution.defer_inverse_normalization)
        && pass.ido >= kernels.complex_lanes
    {
        let vector_ido = pass.ido - pass.ido % kernels.complex_lanes;
        let inverse_scale = if inverse {
            execution.inverse_output_scale
        } else {
            T::one()
        };
        let simd_pass = SimdPass {
            radix,
            l1: pass.l1,
            ido: pass.ido,
            twiddles: &pass.twiddles,
            inverse,
            inverse_scale,
        };
        // SAFETY: plan construction validates pass dimensions, runtime
        // dispatch checked AVX2, and the kernel processes only the complete
        // vector prefix. The scalar loop below handles any remaining values.
        unsafe { (kernels.first_pass)(data, &simd_pass) };
        scalar_start = vector_ido;
        #[cfg(test)]
        crate::m0_record_route(format!(
            "c2c-avx2:first-prefix n={} radix={} ido={} vector_ido={} scalar_tail={}",
            execution.total_len,
            radix,
            pass.ido,
            vector_ido,
            pass.ido - vector_ido
        ));
        if scalar_start == pass.ido {
            return;
        }
    } else {
        #[cfg(test)]
        crate::m0_record_route(format!(
            "c2c-scalar:first n={} radix={} ido={}",
            execution.total_len, radix, pass.ido
        ));
    }

    let zero = Complex::new(T::zero(), T::zero());
    let mut inputs = [zero; 8];
    let mut transformed = [zero; 8];
    let mut correction = [zero; 8];
    let stage_scale = if inverse && !execution.defer_inverse_normalization {
        inverse_scale::<T>(radix)
    } else {
        T::one()
    };
    let output_scale = execution.inverse_output_scale;
    let combined_scale = stage_scale * output_scale;
    let radix_value = scalar::<T>(radix as f64);
    let overflow_threshold = T::max_value() / scalar::<T>(4.0 * radix as f64);

    for inner in scalar_start..pass.ido {
        let mut max_component = T::zero();
        for (branch, slot) in inputs.iter_mut().enumerate().take(radix) {
            let value = data[inner + pass.ido * branch];
            if inverse && !execution.defer_inverse_normalization {
                max_component = max_component.max(value.re.abs()).max(value.im.abs());
            }
            *slot = value;
        }
        let prescale =
            inverse && !execution.defer_inverse_normalization && max_component > overflow_threshold;
        if prescale {
            for value in &mut inputs[..radix] {
                *value = *value * stage_scale;
            }
        }
        radix_transform_fixed::<R, T>(
            &inputs[..radix],
            &mut transformed[..radix],
            execution.roots,
            execution.total_len / radix,
            execution.direction,
        );

        if prescale {
            for (branch, slot) in inputs.iter_mut().enumerate().take(radix) {
                let original = data[inner + pass.ido * branch];
                let scaled = original * stage_scale;
                *slot = Complex::new(
                    prescale_residual(original.re, scaled.re, radix_value),
                    prescale_residual(original.im, scaled.im, radix_value),
                );
            }
            radix_transform_fixed::<R, T>(
                &inputs[..radix],
                &mut correction[..radix],
                execution.roots,
                execution.total_len / radix,
                execution.direction,
            );
        }

        for branch in 0..radix {
            let index = inner + pass.ido * branch;
            let mut value = transformed[branch];
            if branch != 0 && pass.ido > 1 {
                let twiddle = directed_root(
                    &pass.twiddles,
                    branch * pass.ido + inner,
                    execution.direction,
                );
                value = value * twiddle;
            }
            if prescale {
                if branch != 0 && pass.ido > 1 {
                    let twiddle = directed_root(
                        &pass.twiddles,
                        branch * pass.ido + inner,
                        execution.direction,
                    );
                    correction[branch] = correction[branch] * twiddle;
                }
                value = (value + correction[branch] * stage_scale) * output_scale;
            } else if inverse {
                value = value * combined_scale;
            }
            data[index] = value;
        }
    }
}

fn execute_direct_radix_pass<const R: usize, T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    pass: &ComplexPass<T>,
    execution: &PassExecution<'_, T>,
) {
    let radix = R;
    debug_assert!(matches!(radix, 2 | 3 | 4 | 5 | 7 | 8));
    let inverse = execution.direction == Direction::Inverse;
    let mut scalar_outer_start = 0;
    let mut scalar_inner_start = 0;
    if let Some(kernels) = execution.simd
        && matches!(R, 4 | 8)
        && (!inverse || execution.defer_inverse_normalization)
        && pass.ido == 1
        && pass.l1 >= 2
        && let Some(kernel) = match pass.radix {
            4 => kernels.ido1_radix4_pass,
            8 if pass.l1.is_multiple_of(2) => kernels.ido1_radix8_pass,
            _ => None,
        }
    {
        let vector_l1 = pass.l1 - pass.l1 % 2;
        let simd_pass = SimdPass {
            radix: pass.radix,
            l1: pass.l1,
            ido: pass.ido,
            twiddles: &pass.twiddles,
            inverse,
            inverse_scale: if inverse {
                execution.inverse_output_scale
            } else {
                T::one()
            },
        };
        // SAFETY: runtime dispatch checked AVX2; this kernel accepts radix
        // 2/4/8, ido one, and disjoint plan-owned input/output buffers. It
        // writes only the complete two-group prefix.
        unsafe { kernel(input, output, &simd_pass) };
        scalar_outer_start = vector_l1;
        #[cfg(test)]
        crate::m0_record_route(format!(
            "c2c-avx2:ido1-pair n={} radix={} l1={} vector_l1={} scalar_tail={}",
            execution.total_len,
            pass.radix,
            pass.l1,
            vector_l1,
            pass.l1 - vector_l1
        ));
        if scalar_outer_start == pass.l1 {
            return;
        }
    }
    if let Some(kernels) = execution.simd
        && matches!(R, 2 | 4 | 8)
        && (!inverse || execution.defer_inverse_normalization)
        && pass.ido >= kernels.complex_lanes
    {
        let vector_ido = pass.ido - pass.ido % kernels.complex_lanes;
        let simd_pass = SimdPass {
            radix,
            l1: pass.l1,
            ido: pass.ido,
            twiddles: &pass.twiddles,
            inverse,
            inverse_scale: if inverse {
                execution.inverse_output_scale
            } else {
                T::one()
            },
        };
        // SAFETY: plan construction validates pass dimensions, runtime
        // dispatch checked AVX2, and the kernel processes only the complete
        // vector prefix. The scalar loop below handles any remaining values.
        unsafe { (kernels.pass)(input, output, &simd_pass) };
        scalar_inner_start = vector_ido;
        #[cfg(test)]
        crate::m0_record_route(format!(
            "c2c-avx2:pass-prefix n={} radix={} l1={} ido={} vector_ido={} scalar_tail={}",
            execution.total_len,
            radix,
            pass.l1,
            pass.ido,
            vector_ido,
            pass.ido - vector_ido
        ));
        if scalar_inner_start == pass.ido {
            return;
        }
    }

    if scalar_outer_start == 0 && scalar_inner_start == 0 {
        #[cfg(test)]
        crate::m0_record_route(format!(
            "c2c-scalar:pass n={} radix={} l1={} ido={}",
            execution.total_len, radix, pass.l1, pass.ido
        ));
    }

    let zero = Complex::new(T::zero(), T::zero());
    let mut inputs = [zero; 8];
    let mut transformed = [zero; 8];
    let mut correction = [zero; 8];
    let stage_scale = if inverse && !execution.defer_inverse_normalization {
        inverse_scale::<T>(radix)
    } else {
        T::one()
    };
    let output_scale = execution.inverse_output_scale;
    let combined_scale = stage_scale * output_scale;
    let radix_value = scalar::<T>(radix as f64);
    let overflow_threshold = T::max_value() / scalar::<T>(4.0 * radix as f64);
    let root_stride = execution.total_len / radix;

    for outer in scalar_outer_start..pass.l1 {
        for inner in scalar_inner_start..pass.ido {
            let mut max_component = T::zero();
            for (branch, slot) in inputs.iter_mut().enumerate().take(radix) {
                let value = input[inner + pass.ido * (branch + radix * outer)];
                if inverse && !execution.defer_inverse_normalization {
                    max_component = max_component.max(value.re.abs()).max(value.im.abs());
                }
                *slot = value;
            }
            let prescale = inverse
                && !execution.defer_inverse_normalization
                && max_component > overflow_threshold;
            if prescale {
                for value in &mut inputs[..radix] {
                    *value = *value * stage_scale;
                }
            }
            radix_transform_fixed::<R, T>(
                &inputs[..radix],
                &mut transformed[..radix],
                execution.roots,
                root_stride,
                execution.direction,
            );

            if prescale {
                for (branch, slot) in inputs.iter_mut().enumerate().take(radix) {
                    let original = input[inner + pass.ido * (branch + radix * outer)];
                    let scaled = original * stage_scale;
                    *slot = Complex::new(
                        prescale_residual(original.re, scaled.re, radix_value),
                        prescale_residual(original.im, scaled.im, radix_value),
                    );
                }
                radix_transform_fixed::<R, T>(
                    &inputs[..radix],
                    &mut correction[..radix],
                    execution.roots,
                    root_stride,
                    execution.direction,
                );
            }

            for branch in 0..radix {
                let destination_index = inner + pass.ido * (outer + pass.l1 * branch);
                let mut value = transformed[branch];
                if branch != 0 && pass.ido > 1 {
                    let twiddle = directed_root(
                        &pass.twiddles,
                        branch * pass.ido + inner,
                        execution.direction,
                    );
                    value = value * twiddle;
                }
                if prescale {
                    if branch != 0 && pass.ido > 1 {
                        let twiddle = directed_root(
                            &pass.twiddles,
                            branch * pass.ido + inner,
                            execution.direction,
                        );
                        correction[branch] = correction[branch] * twiddle;
                    }
                    value = (value + correction[branch] * stage_scale) * output_scale;
                } else if inverse {
                    value = value * combined_scale;
                }
                output[destination_index] = value;
            }
        }
    }
}

impl<T: Float + FromPrimitive> PassExecution<'_, T> {
    #[inline(always)]
    fn transform<const R: usize>(&mut self, radix: usize, root_stride: usize) {
        if R == 0 {
            if let Some(plan) = self.blue_plan {
                plan.execute(
                    &self.radix_input[..radix],
                    &mut self.radix_output[..radix],
                    self.direction,
                    self.blue_buffer,
                    self.blue_workspace
                        .as_deref_mut()
                        .expect("Bluestein pass has a workspace"),
                );
            } else {
                if self.prime_phase_indices.is_empty() {
                    radix_transform(
                        radix,
                        &self.radix_input[..radix],
                        &mut self.radix_output[..radix],
                        self.roots,
                        root_stride,
                        self.direction,
                    );
                } else {
                    radix_general_prime_precomputed_phases(
                        &self.radix_input[..radix],
                        &mut self.radix_output[..radix],
                        self.roots,
                        root_stride,
                        self.direction,
                        self.prime_phase_indices,
                    );
                }
            }
        } else {
            radix_transform_fixed::<R, T>(
                &self.radix_input[..radix],
                &mut self.radix_output[..radix],
                self.roots,
                root_stride,
                self.direction,
            );
        }
    }
}

fn prescale_residual<T: Float>(original: T, scaled: T, radix: T) -> T {
    let reconstructed = scaled * radix;
    if reconstructed.is_finite() {
        original - reconstructed
    } else {
        T::zero()
    }
}

fn inverse_normalization_is_safe<T: Float + FromPrimitive>(
    values: &[Complex<T>],
    len: usize,
) -> bool {
    let mut maximum = T::zero();
    for value in values {
        if !value.re.is_finite() || !value.im.is_finite() {
            return false;
        }
        maximum = maximum.max(value.re.abs()).max(value.im.abs());
    }
    maximum <= T::max_value() / scalar::<T>(4.0 * len as f64)
}

fn radix_transform<T: Float + FromPrimitive>(
    radix: usize,
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    roots: &[Complex<T>],
    root_stride: usize,
    direction: Direction,
) {
    match radix {
        2 => {
            output[0] = input[0] + input[1];
            output[1] = input[0] - input[1];
        }
        3 => radix3(input, output, direction),
        4 => radix4(input, output, direction),
        5 => radix5(input, output, direction),
        7 => radix7(input, output, direction),
        8 => radix8(input, output, direction),
        11 => radix11(input, output, direction),
        _ if radix.is_multiple_of(2) => {
            radix_direct(radix, input, output, roots, root_stride, direction)
        }
        _ => radix_general_prime(input, output, roots, root_stride, direction),
    }
}

#[inline(always)]
fn radix_transform_fixed<const R: usize, T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    roots: &[Complex<T>],
    root_stride: usize,
    direction: Direction,
) {
    match R {
        2 => {
            output[0] = input[0] + input[1];
            output[1] = input[0] - input[1];
        }
        3 => radix3(input, output, direction),
        4 => radix4(input, output, direction),
        5 => radix5(input, output, direction),
        7 => radix7(input, output, direction),
        8 => radix8(input, output, direction),
        11 => radix11(input, output, direction),
        _ => radix_transform(R, input, output, roots, root_stride, direction),
    }
}

fn radix_general_prime<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    roots: &[Complex<T>],
    root_stride: usize,
    direction: Direction,
) {
    let radix = input.len();
    debug_assert!(!radix.is_multiple_of(2));
    for (frequency, result) in output.iter_mut().enumerate() {
        let mut sum = input[0];
        for branch in 1..=(radix - 1) / 2 {
            let opposite = radix - branch;
            let root_index = bounded_product_mod(branch, frequency, radix) * root_stride;
            let root = directed_root(roots, root_index, direction);
            let pair = input[branch] + input[opposite];
            let difference = input[branch] - input[opposite];
            sum = sum + pair * root.re + imaginary(difference) * root.im;
        }
        *result = sum;
    }
}

fn radix_general_prime_precomputed_phases<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    roots: &[Complex<T>],
    root_stride: usize,
    direction: Direction,
    phase_indices: &[u8],
) {
    let radix = input.len();
    let half = (radix - 1) / 2;
    debug_assert!(!radix.is_multiple_of(2));
    debug_assert_eq!(phase_indices.len(), half * radix);
    let mut dc = input[0];
    for branch in 1..=half {
        let pair = input[branch] + input[radix - branch];
        dc = dc + pair;
    }
    output[0] = dc;

    for frequency in 1..=half {
        let mut positive = input[0];
        let mut negative = input[0];
        for branch in 1..=half {
            let opposite = radix - branch;
            let phase = usize::from(phase_indices[(branch - 1) * radix + frequency]);
            let root = directed_root(roots, phase * root_stride, direction);
            let pair = input[branch] + input[opposite];
            let difference = input[branch] - input[opposite];
            let cosine = pair * root.re;
            let sine = imaginary(difference) * root.im;
            positive = positive + cosine + sine;
            negative = negative + cosine - sine;
        }
        output[frequency] = positive;
        output[radix - frequency] = negative;
    }
}

#[cfg(test)]
fn radix_general_prime_precomputed_phases_reference<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    roots: &[Complex<T>],
    root_stride: usize,
    direction: Direction,
    phase_indices: &[u8],
) {
    let radix = input.len();
    let half = (radix - 1) / 2;
    for (frequency, result) in output.iter_mut().enumerate() {
        let mut sum = input[0];
        for branch in 1..=half {
            let opposite = radix - branch;
            let phase = usize::from(phase_indices[(branch - 1) * radix + frequency]);
            let root = directed_root(roots, phase * root_stride, direction);
            let pair = input[branch] + input[opposite];
            let difference = input[branch] - input[opposite];
            sum = sum + pair * root.re + imaginary(difference) * root.im;
        }
        *result = sum;
    }
}

fn radix3<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    direction: Direction,
) {
    let pair = input[1] + input[2];
    let difference = input[1] - input[2];
    let half = scalar::<T>(0.5);
    let root_three_half = scalar::<T>(0.866_025_403_784_438_6);
    let turn = imaginary(difference) * (root_three_half * direction_sign::<T>(direction));
    let base = input[0] - pair * half;
    output[0] = input[0] + pair;
    output[1] = base + turn;
    output[2] = base - turn;
}

fn radix4<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    direction: Direction,
) {
    let even_sum = input[0] + input[2];
    let even_difference = input[0] - input[2];
    let odd_sum = input[1] + input[3];
    let odd_difference = input[1] - input[3];
    output[0] = even_sum + odd_sum;
    output[2] = even_sum - odd_sum;
    output[1] = even_difference + imaginary(odd_difference) * direction_sign::<T>(direction);
    output[3] = even_difference - imaginary(odd_difference) * direction_sign::<T>(direction);
}

fn radix5<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    direction: Direction,
) {
    let pair_a = input[1] + input[4];
    let diff_a = input[1] - input[4];
    let pair_b = input[2] + input[3];
    let diff_b = input[2] - input[3];
    let c72 = scalar::<T>(0.309_016_994_374_947_45);
    let c144 = scalar::<T>(-0.809_016_994_374_947_5);
    let s72 = scalar::<T>(0.951_056_516_295_153_5);
    let s144 = scalar::<T>(0.587_785_252_292_473_1);
    let sign = direction_sign::<T>(direction);

    output[0] = input[0] + pair_a + pair_b;

    let base_one = input[0] + pair_a * c72 + pair_b * c144;
    let turn_one = imaginary(diff_a * s72 + diff_b * s144) * sign;
    output[1] = base_one + turn_one;
    output[4] = base_one - turn_one;

    let base_two = input[0] + pair_a * c144 + pair_b * c72;
    let turn_two = imaginary(diff_a * s144 - diff_b * s72) * sign;
    output[2] = base_two + turn_two;
    output[3] = base_two - turn_two;
}

fn radix7<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    direction: Direction,
) {
    let p1 = input[1] + input[6];
    let p2 = input[2] + input[5];
    let p3 = input[3] + input[4];
    let d1 = input[1] - input[6];
    let d2 = input[2] - input[5];
    let d3 = input[3] - input[4];
    let c1 = scalar::<T>(0.623_489_801_858_733_5);
    let c2 = scalar::<T>(-0.222_520_933_956_314_4);
    let c3 = scalar::<T>(-0.900_968_867_902_419_1);
    let s1 = scalar::<T>(0.781_831_482_468_029_8);
    let s2 = scalar::<T>(0.974_927_912_181_823_6);
    let s3 = scalar::<T>(0.433_883_739_117_558_1);
    let sign = direction_sign::<T>(direction);

    output[0] = input[0] + p1 + p2 + p3;
    let base1 = input[0] + p1 * c1 + p2 * c2 + p3 * c3;
    let turn1 = imaginary(d1 * s1 + d2 * s2 + d3 * s3) * sign;
    output[1] = base1 + turn1;
    output[6] = base1 - turn1;
    let base2 = input[0] + p1 * c2 + p2 * c3 + p3 * c1;
    let turn2 = imaginary(d1 * s2 - d2 * s3 - d3 * s1) * sign;
    output[2] = base2 + turn2;
    output[5] = base2 - turn2;
    let base3 = input[0] + p1 * c3 + p2 * c1 + p3 * c2;
    let turn3 = imaginary(d1 * s3 - d2 * s1 + d3 * s2) * sign;
    output[3] = base3 + turn3;
    output[4] = base3 - turn3;
}

fn radix8<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    direction: Direction,
) {
    let mut a1 = input[1] + input[5];
    let mut a5 = input[1] - input[5];
    let mut a3 = input[3] + input[7];
    let mut a7 = input[3] - input[7];
    let old_a1 = a1;
    a1 = a1 + a3;
    a3 = rotate_quarter(old_a1 - a3, direction);
    a7 = rotate_quarter(a7, direction);
    let old_a5 = a5;
    a5 = a5 + a7;
    a7 = rotate_eighth_three(old_a5 - a7, direction);
    a5 = rotate_eighth(a5, direction);

    let a0 = input[0] + input[4];
    let a4 = input[0] - input[4];
    let a2 = input[2] + input[6];
    let mut a6 = input[2] - input[6];
    output[0] = a0 + a2 + a1;
    output[4] = a0 + a2 - a1;
    output[2] = a0 - a2 + a3;
    output[6] = a0 - a2 - a3;
    a6 = rotate_quarter(a6, direction);
    output[1] = a4 + a6 + a5;
    output[5] = a4 + a6 - a5;
    output[3] = a4 - a6 + a7;
    output[7] = a4 - a6 - a7;
}

pub(crate) fn radix11<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    direction: Direction,
) {
    let p1 = input[1] + input[10];
    let p2 = input[2] + input[9];
    let p3 = input[3] + input[8];
    let p4 = input[4] + input[7];
    let p5 = input[5] + input[6];
    let d1 = input[1] - input[10];
    let d2 = input[2] - input[9];
    let d3 = input[3] - input[8];
    let d4 = input[4] - input[7];
    let d5 = input[5] - input[6];
    let c1 = scalar::<T>(0.841_253_532_831_181_2);
    let c2 = scalar::<T>(0.415_415_013_001_886_4);
    let c3 = scalar::<T>(-0.142_314_838_273_285_14);
    let c4 = scalar::<T>(-0.654_860_733_945_285_1);
    let c5 = scalar::<T>(-0.959_492_973_614_497_4);
    let s1 = scalar::<T>(0.540_640_817_455_597_6);
    let s2 = scalar::<T>(0.909_631_995_354_518_4);
    let s3 = scalar::<T>(0.989_821_441_880_932_7);
    let s4 = scalar::<T>(0.755_749_574_354_258_3);
    let s5 = scalar::<T>(0.281_732_556_841_429_7);
    let sign = direction_sign::<T>(direction);

    output[0] = input[0] + p1 + p2 + p3 + p4 + p5;
    let base1 = input[0] + p1 * c1 + p2 * c2 + p3 * c3 + p4 * c4 + p5 * c5;
    let turn1 = imaginary(d1 * s1 + d2 * s2 + d3 * s3 + d4 * s4 + d5 * s5) * sign;
    output[1] = base1 + turn1;
    output[10] = base1 - turn1;
    let base2 = input[0] + p1 * c2 + p2 * c4 + p3 * c5 + p4 * c3 + p5 * c1;
    let turn2 = imaginary(d1 * s2 + d2 * s4 - d3 * s5 - d4 * s3 - d5 * s1) * sign;
    output[2] = base2 + turn2;
    output[9] = base2 - turn2;
    let base3 = input[0] + p1 * c3 + p2 * c5 + p3 * c2 + p4 * c1 + p5 * c4;
    let turn3 = imaginary(d1 * s3 - d2 * s5 - d3 * s2 + d4 * s1 + d5 * s4) * sign;
    output[3] = base3 + turn3;
    output[8] = base3 - turn3;
    let base4 = input[0] + p1 * c4 + p2 * c3 + p3 * c1 + p4 * c5 + p5 * c2;
    let turn4 = imaginary(d1 * s4 - d2 * s3 + d3 * s1 + d4 * s5 - d5 * s2) * sign;
    output[4] = base4 + turn4;
    output[7] = base4 - turn4;
    let base5 = input[0] + p1 * c5 + p2 * c1 + p3 * c4 + p4 * c2 + p5 * c3;
    let turn5 = imaginary(d1 * s5 - d2 * s1 + d3 * s4 - d4 * s2 + d5 * s3) * sign;
    output[5] = base5 + turn5;
    output[6] = base5 - turn5;
}

fn radix_direct<T: Float + FromPrimitive>(
    radix: usize,
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    roots: &[Complex<T>],
    root_stride: usize,
    direction: Direction,
) {
    for (frequency, result) in output.iter_mut().enumerate().take(radix) {
        let mut sum = Complex::new(T::zero(), T::zero());
        for (branch, &value) in input.iter().enumerate().take(radix) {
            let root_index = bounded_product_mod(branch, frequency, radix) * root_stride;
            sum = sum + value * directed_root(roots, root_index, direction);
        }
        *result = sum;
    }
}

fn factorize(len: usize) -> Result<Vec<usize>, FftError> {
    let mut factors = Vec::new();
    factors
        .try_reserve_exact(usize::BITS as usize)
        .map_err(|_| FftError::AllocationFailed)?;

    let mut remaining = len;
    while remaining.is_multiple_of(8) {
        factors.push(8);
        remaining /= 8;
    }
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

fn good_size_complex(minimum: usize) -> Result<usize, FftError> {
    if minimum <= 12 {
        return Ok(minimum);
    }

    let n = minimum as u128;
    let mut best = 2 * n;
    let mut factor_11 = 1_u128;
    while factor_11 < best {
        let mut factor_7 = factor_11;
        while factor_7 < best {
            let mut factor_5 = factor_7;
            while factor_5 < best {
                best = iter_23(n, factor_5, best);
                if best == n {
                    return usize::try_from(best).map_err(|_| FftError::AllocationFailed);
                }
                factor_5 *= 5;
            }
            factor_7 *= 7;
        }
        factor_11 *= 11;
    }
    usize::try_from(best).map_err(|_| FftError::AllocationFailed)
}

fn iter_23(minimum: u128, mut value: u128, mut best: u128) -> u128 {
    while value < minimum {
        value *= 2;
    }
    loop {
        if value < minimum {
            value *= 3;
        } else if value > minimum {
            best = best.min(value);
            if value & 1 == 1 {
                return best;
            }
            value >>= 1;
        } else {
            return minimum;
        }
    }
}

pub(crate) fn unity_root<T: Float + FromPrimitive>(index: usize, len: usize) -> Complex<T> {
    if index == 0 {
        return Complex::new(T::one(), T::zero());
    }

    let mut octant_index = index as u128 * 8;
    let n = len as u128;
    let eight_n = 8 * n;
    let (angle_index, cosine_first, real_sign, imag_sign) = if octant_index < 4 * n {
        if octant_index < 2 * n {
            if octant_index < n {
                (octant_index, true, 1.0, 1.0)
            } else {
                (2 * n - octant_index, false, 1.0, 1.0)
            }
        } else {
            octant_index -= 2 * n;
            if octant_index < n {
                (octant_index, false, -1.0, 1.0)
            } else {
                (2 * n - octant_index, true, -1.0, 1.0)
            }
        }
    } else {
        octant_index = eight_n - octant_index;
        if octant_index < 2 * n {
            if octant_index < n {
                (octant_index, true, 1.0, -1.0)
            } else {
                (2 * n - octant_index, false, 1.0, -1.0)
            }
        } else {
            octant_index -= 2 * n;
            if octant_index < n {
                (octant_index, false, -1.0, -1.0)
            } else {
                (2 * n - octant_index, true, -1.0, -1.0)
            }
        }
    };
    let angle = angle_index as f64 * (std::f64::consts::FRAC_PI_4 / len as f64);
    let (sin, cos) = angle.sin_cos();
    let (real_value, imag_value) = if cosine_first { (cos, sin) } else { (sin, cos) };
    let real = real_value * real_sign;
    let imag = imag_value * imag_sign;
    Complex::new(scalar(real), scalar(imag))
}

fn directed_root<T: Float>(roots: &[Complex<T>], index: usize, direction: Direction) -> Complex<T> {
    match direction {
        Direction::Forward => roots[index].conj(),
        Direction::Inverse => roots[index],
    }
}

fn direction_sign<T: Float>(direction: Direction) -> T {
    match direction {
        Direction::Forward => -T::one(),
        Direction::Inverse => T::one(),
    }
}

fn inverse_scale<T: Float + FromPrimitive>(radix: usize) -> T {
    T::from_f64(1.0 / radix as f64).expect("f32 and f64 can represent a positive reciprocal")
}

fn rotate_quarter<T: Float>(value: Complex<T>, direction: Direction) -> Complex<T> {
    match direction {
        Direction::Forward => Complex::new(value.im, -value.re),
        Direction::Inverse => Complex::new(-value.im, value.re),
    }
}

fn rotate_eighth<T: Float + FromPrimitive>(value: Complex<T>, direction: Direction) -> Complex<T> {
    let c = scalar::<T>(std::f64::consts::FRAC_1_SQRT_2);
    let sign = direction_sign::<T>(direction);
    Complex::new(
        (value.re - sign * value.im) * c,
        (sign * value.re + value.im) * c,
    )
}

fn rotate_eighth_three<T: Float + FromPrimitive>(
    value: Complex<T>,
    direction: Direction,
) -> Complex<T> {
    let c = scalar::<T>(std::f64::consts::FRAC_1_SQRT_2);
    match direction {
        Direction::Forward => Complex::new((value.im - value.re) * c, (-value.re - value.im) * c),
        Direction::Inverse => Complex::new((-value.re - value.im) * c, (value.re - value.im) * c),
    }
}

fn imaginary<T: Float>(value: Complex<T>) -> Complex<T> {
    Complex::new(-value.im, value.re)
}

fn scalar<T: FromPrimitive>(value: f64) -> T {
    T::from_f64(value).expect("f32 and f64 represent FFT constants")
}

fn mul_mod(left: usize, right: usize, modulus: usize) -> usize {
    ((left as u128 * right as u128) % modulus as u128) as usize
}

fn bounded_product_mod(left: usize, right: usize, modulus: usize) -> usize {
    debug_assert!(left < modulus && right < modulus);
    // Non-Bluestein radices are bounded by their planner thresholds, so both
    // operands and their product fit in usize on supported targets.
    left * right % modulus
}

pub(crate) fn try_vec<T: Copy>(len: usize, value: T) -> Result<Vec<T>, FftError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(len)
        .map_err(|_| FftError::AllocationFailed)?;
    result.resize(len, value);
    Ok(result)
}

pub(crate) fn check_vec_len<T>(len: usize) -> Result<(), FftError> {
    if len > isize::MAX as usize / std::mem::size_of::<T>() {
        Err(FftError::AllocationFailed)
    } else {
        Ok(())
    }
}

pub(crate) fn check_len(
    buffer: &'static str,
    expected: usize,
    actual: usize,
) -> Result<(), FftError> {
    if expected == actual {
        Ok(())
    } else {
        Err(FftError::BufferLength {
            buffer,
            expected,
            actual,
        })
    }
}

pub(crate) fn check_batch_rows(
    buffer: &'static str,
    row_len: usize,
    actual: usize,
) -> Result<usize, FftError> {
    debug_assert!(row_len > 0);
    if actual.is_multiple_of(row_len) {
        Ok(actual / row_len)
    } else {
        Err(FftError::BatchBufferLength {
            buffer,
            row_len,
            actual,
        })
    }
}

pub(crate) fn checked_batch_len(
    buffer: &'static str,
    rows: usize,
    row_len: usize,
) -> Result<usize, FftError> {
    rows.checked_mul(row_len)
        .ok_or(FftError::BatchLengthOverflow {
            buffer,
            rows,
            row_len,
        })
}

#[cfg(test)]
mod prime_kernel_tests {
    use super::*;

    fn check_against_reference<T: Float + FromPrimitive + std::fmt::Debug>(radix: usize) {
        let roots: Vec<_> = (0..radix).map(|index| unity_root(index, radix)).collect();
        let half = (radix - 1) / 2;
        let phase_indices: Vec<_> = (1..=half)
            .flat_map(|branch| (0..radix).map(move |frequency| (branch, frequency)))
            .map(|(branch, frequency)| {
                u8::try_from(bounded_product_mod(branch, frequency, radix)).unwrap()
            })
            .collect();
        let zero = Complex::new(T::zero(), T::zero());
        let normal = T::from_f64(0.75).unwrap();
        let large = T::from_f64(1024.0).unwrap();
        let near_limit = T::max_value() / T::from_usize(radix * 64).unwrap();

        let mut general = Vec::with_capacity(radix);
        for index in 0..radix {
            let phase = T::from_usize(index + 1).unwrap() * T::from_f64(0.173).unwrap();
            general.push(Complex::new(
                phase.sin() * normal,
                (phase * T::from_f64(0.61).unwrap()).cos(),
            ));
        }

        let mut cancellation = vec![zero; radix];
        cancellation[0] = Complex::new(normal, -normal);
        for branch in 1..=half {
            let small = T::from_usize(branch).unwrap() * normal;
            cancellation[branch] = Complex::new(large + small, large - small);
            cancellation[radix - branch] = Complex::new(-large + small, -large - small);
        }

        let mut sparse = vec![zero; radix];
        sparse[0] = Complex::new(normal, -normal);
        sparse[2] = Complex::new(-normal * large, normal);
        sparse[radix - 1] = Complex::new(normal, normal * large);

        let mut near_overflow = Vec::with_capacity(radix);
        for index in 0..radix {
            let sign = if index.is_multiple_of(2) {
                T::one()
            } else {
                -T::one()
            };
            near_overflow.push(Complex::new(
                near_limit * sign,
                near_limit / T::from_f64(2.0).unwrap(),
            ));
        }

        for input in [general, cancellation, sparse, near_overflow] {
            for direction in [Direction::Forward, Direction::Inverse] {
                let mut expected = vec![zero; radix];
                let mut actual = vec![zero; radix];
                radix_general_prime_precomputed_phases_reference(
                    &input,
                    &mut expected,
                    &roots,
                    1,
                    direction,
                    &phase_indices,
                );
                radix_general_prime_precomputed_phases(
                    &input,
                    &mut actual,
                    &roots,
                    1,
                    direction,
                    &phase_indices,
                );

                let tolerance_scale = T::epsilon() * T::from_usize(radix * 64).unwrap();
                for (index, (&actual, &expected)) in actual.iter().zip(&expected).enumerate() {
                    assert!(actual.re.is_finite() && actual.im.is_finite());
                    assert!(expected.re.is_finite() && expected.im.is_finite());
                    let scale = actual
                        .re
                        .abs()
                        .max(actual.im.abs())
                        .max(expected.re.abs())
                        .max(expected.im.abs());
                    let tolerance = scale * tolerance_scale;
                    assert!(
                        (actual.re - expected.re).abs() <= tolerance
                            && (actual.im - expected.im).abs() <= tolerance,
                        "radix={radix}, index={index}, forward={}, actual={actual:?}, expected={expected:?}, tolerance={tolerance:?}",
                        direction == Direction::Forward
                    );
                }
            }
        }
    }

    #[test]
    fn paired_prime_kernel_matches_previous_arithmetic_for_stress_inputs() {
        for radix in [13, 17, 19, 31, 41, 73, 103, 109] {
            check_against_reference::<f64>(radix);
            check_against_reference::<f32>(radix);
        }
    }

    #[test]
    fn p241_bluestein_plan_uses_the_benchmarked_convolution_length() {
        for convolution in [
            ComplexFftPlan::<f64>::new(241).unwrap().data.bluestein[0]
                .as_ref()
                .unwrap()
                .convolution_len(),
            ComplexFftPlan::<f32>::new(241).unwrap().data.bluestein[0]
                .as_ref()
                .unwrap()
                .convolution_len(),
        ] {
            assert_eq!(convolution, P241_BLUESTEIN_CONVOLUTION_LEN);
        }
    }
}
