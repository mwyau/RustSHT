// SPDX-FileCopyrightText: 2010-2026 Max-Planck-Society (DUCC FFT pass equations)
// SPDX-FileCopyrightText: 2019 Peter Bell (DUCC FFT pass equations)
// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause
//
// Scalar real radix-2/radix-4 pass equations and packed layout are adapted
// from DUCC fft1d_impl.h at c1ed01b8fda69d2a378190f0915f1a19762dddbf.

use num_complex::Complex;
use num_traits::{Float, FromPrimitive};

use crate::FftError;
use crate::complex::{check_vec_len, try_vec};
use crate::simd::{
    RealRadix2InteriorKernel, RealRadix4ForwardInteriorKernel, RealRadix4InteriorKernel,
};

mod inverse_terminal;

struct Pass<T> {
    radix: usize,
    l1: usize,
    ido: usize,
    twiddles: Vec<T>,
}

pub(crate) struct RealMultipassPlan<T> {
    len: usize,
    passes: Vec<Pass<T>>,
    radix2_interior_kernel: Option<RealRadix2InteriorKernel<T>>,
    radix4_forward_interior_kernel: Option<RealRadix4ForwardInteriorKernel<T>>,
    radix4_interior_kernel: Option<RealRadix4InteriorKernel<T>>,
}

pub(crate) struct RealMultipassWorkspace<T> {
    first: Vec<T>,
    second: Vec<T>,
}

impl<T: Float + FromPrimitive> RealMultipassPlan<T> {
    pub(crate) fn try_new(
        len: usize,
        factors: &[usize],
        roots: &[Complex<T>],
        radix2_interior_kernel: Option<RealRadix2InteriorKernel<T>>,
        radix4_forward_interior_kernel: Option<RealRadix4ForwardInteriorKernel<T>>,
        radix4_interior_kernel: Option<RealRadix4InteriorKernel<T>>,
    ) -> Result<Option<Self>, FftError> {
        if len <= 1 || factors.iter().any(|&radix| !matches!(radix, 2 | 4)) {
            return Ok(None);
        }

        let mut passes = Vec::new();
        passes
            .try_reserve_exact(factors.len())
            .map_err(|_| FftError::AllocationFailed)?;
        let mut l1 = 1usize;
        for &radix in factors {
            let pass_len = l1.checked_mul(radix).ok_or(FftError::AllocationFailed)?;
            let ido = len / pass_len;
            let twiddle_len = (radix - 1)
                .checked_mul(ido.saturating_sub(1))
                .ok_or(FftError::AllocationFailed)?;
            check_vec_len::<T>(twiddle_len)?;
            let mut twiddles = try_vec(twiddle_len, T::zero())?;
            for branch in 1..radix {
                let branch_l1 = mul_mod(branch, l1, len);
                for i in 1..=(ido.saturating_sub(1)) / 2 {
                    let root = roots[mul_mod(branch_l1, i, len)];
                    let offset = (branch - 1) * (ido - 1) + 2 * i - 2;
                    twiddles[offset] = root.re;
                    twiddles[offset + 1] = root.im;
                }
            }
            passes.push(Pass {
                radix,
                l1,
                ido,
                twiddles,
            });
            l1 = pass_len;
        }
        debug_assert_eq!(l1, len);
        Ok(Some(Self {
            len,
            passes,
            radix2_interior_kernel,
            radix4_forward_interior_kernel,
            radix4_interior_kernel,
        }))
    }

    pub(crate) fn try_workspace(&self) -> Result<RealMultipassWorkspace<T>, FftError> {
        check_vec_len::<T>(self.len)?;
        Ok(RealMultipassWorkspace {
            first: try_vec(self.len, T::zero())?,
            second: try_vec(self.len, T::zero())?,
        })
    }

    pub(crate) fn forward(
        &self,
        input: &[T],
        spectrum: &mut [Complex<T>],
        workspace: &mut RealMultipassWorkspace<T>,
    ) {
        let mut passes = self.passes.iter().rev();
        let Some(first_pass) = passes.next() else {
            unreachable!("real multipass plan has at least one pass")
        };
        execute_forward_pass(
            first_pass,
            input,
            &mut workspace.second,
            self.radix4_forward_interior_kernel,
        );
        let mut current_is_first = false;
        for pass in passes {
            if current_is_first {
                execute_forward_pass(
                    pass,
                    &workspace.first,
                    &mut workspace.second,
                    self.radix4_forward_interior_kernel,
                );
            } else {
                execute_forward_pass(
                    pass,
                    &workspace.second,
                    &mut workspace.first,
                    self.radix4_forward_interior_kernel,
                );
            }
            current_is_first = !current_is_first;
        }
        let packed = if current_is_first {
            &workspace.first
        } else {
            &workspace.second
        };
        unpack_spectrum(packed, spectrum);
    }

    pub(crate) fn inverse(
        &self,
        spectrum: &[Complex<T>],
        output: &mut [T],
        workspace: &mut RealMultipassWorkspace<T>,
    ) {
        pack_spectrum(spectrum, &mut workspace.first);
        let mut current_is_first = true;
        let scale = scalar::<T>(1.0 / self.len as f64);
        let last_pass = self.passes.len() - 1;
        debug_assert_eq!(self.passes[last_pass].ido, 1);
        for (index, pass) in self.passes.iter().enumerate() {
            if index == last_pass {
                // The terminal pass writes the normalized samples directly
                // to caller storage, avoiding an unscaled scratch copyback.
                let input = if current_is_first {
                    &workspace.first
                } else {
                    &workspace.second
                };
                inverse_terminal::execute(pass, input, output, scale);
            } else if current_is_first {
                execute_inverse_pass(
                    pass,
                    &workspace.first,
                    &mut workspace.second,
                    self.radix2_interior_kernel,
                    self.radix4_interior_kernel,
                );
            } else {
                execute_inverse_pass(
                    pass,
                    &workspace.second,
                    &mut workspace.first,
                    self.radix2_interior_kernel,
                    self.radix4_interior_kernel,
                );
            }
            if index != last_pass {
                current_is_first = !current_is_first;
            }
        }
    }
}

impl<T> RealMultipassWorkspace<T> {
    pub(crate) fn scratch_bytes(&self) -> Option<usize> {
        self.first
            .len()
            .checked_add(self.second.len())?
            .checked_mul(std::mem::size_of::<T>())
    }
}

#[inline(always)]
fn index(a: usize, b: usize, c: usize, ido: usize, middle: usize) -> usize {
    a + ido * (b + middle * c)
}

#[inline(always)]
fn mul_mod(left: usize, right: usize, modulus: usize) -> usize {
    ((left as u128 * right as u128) % modulus as u128) as usize
}

#[inline(always)]
fn multiply_pm<T: Float>(c: T, d: T, e: T, f: T) -> (T, T) {
    (c * e + d * f, c * f - d * e)
}

#[inline(always)]
fn twiddle<T: Copy>(pass: &Pass<T>, branch: usize, component: usize) -> T {
    pass.twiddles[(branch - 1) * (pass.ido - 1) + component]
}

fn execute_forward_pass<T: Float + FromPrimitive>(
    pass: &Pass<T>,
    input: &[T],
    output: &mut [T],
    radix4_forward_interior_kernel: Option<RealRadix4ForwardInteriorKernel<T>>,
) {
    match pass.radix {
        2 => forward_radix2(pass, input, output),
        4 => forward_radix4(pass, input, output, radix4_forward_interior_kernel),
        _ => unreachable!("real multipass plan contains only radix 2 and 4"),
    }
}

fn execute_inverse_pass<T: Float + FromPrimitive>(
    pass: &Pass<T>,
    input: &[T],
    output: &mut [T],
    radix2_interior_kernel: Option<RealRadix2InteriorKernel<T>>,
    radix4_interior_kernel: Option<RealRadix4InteriorKernel<T>>,
) {
    match pass.radix {
        2 => inverse_radix2(pass, input, output, radix2_interior_kernel),
        4 => inverse_radix4(pass, input, output, radix4_interior_kernel),
        _ => unreachable!("real multipass plan contains only radix 2 and 4"),
    }
}

fn forward_radix2<T: Float + FromPrimitive>(pass: &Pass<T>, input: &[T], output: &mut [T]) {
    let l1 = pass.l1;
    let ido = pass.ido;
    if ido == 1 {
        let (pairs, _) = output.as_chunks_mut::<2>();
        for (k, dst) in pairs.iter_mut().enumerate() {
            let x0 = input[k];
            let x1 = input[l1 + k];
            dst[0] = x0 + x1;
            dst[1] = x0 - x1;
        }
        return;
    }
    for k in 0..l1 {
        let x0 = input[index(0, k, 0, ido, l1)];
        let x1 = input[index(0, k, 1, ido, l1)];
        output[index(0, 0, k, ido, 2)] = x0 + x1;
        output[index(ido - 1, 1, k, ido, 2)] = x0 - x1;
    }
    if ido.is_multiple_of(2) {
        for k in 0..l1 {
            output[index(0, 1, k, ido, 2)] = -input[index(ido - 1, k, 1, ido, l1)];
            output[index(ido - 1, 0, k, ido, 2)] = input[index(ido - 1, k, 0, ido, l1)];
        }
    }
    if ido <= 2 {
        return;
    }
    for k in 0..l1 {
        for i in (2..ido).step_by(2) {
            let ic = ido - i;
            let (tr2, ti2) = multiply_pm(
                twiddle(pass, 1, i - 2),
                twiddle(pass, 1, i - 1),
                input[index(i - 1, k, 1, ido, l1)],
                input[index(i, k, 1, ido, l1)],
            );
            let x0 = input[index(i - 1, k, 0, ido, l1)];
            let x1 = input[index(i, k, 0, ido, l1)];
            output[index(i - 1, 0, k, ido, 2)] = x0 + tr2;
            output[index(ic - 1, 1, k, ido, 2)] = x0 - tr2;
            output[index(i, 0, k, ido, 2)] = ti2 + x1;
            output[index(ic, 1, k, ido, 2)] = ti2 - x1;
        }
    }
}

fn inverse_radix2<T: Float + FromPrimitive>(
    pass: &Pass<T>,
    input: &[T],
    output: &mut [T],
    radix2_interior_kernel: Option<RealRadix2InteriorKernel<T>>,
) {
    let l1 = pass.l1;
    let ido = pass.ido;
    if ido == 1 {
        let (first, second) = output.split_at_mut(l1);
        let (pairs, _) = input.as_chunks::<2>();
        for (k, src) in pairs.iter().enumerate() {
            first[k] = src[0] + src[1];
            second[k] = src[0] - src[1];
        }
        return;
    }
    for k in 0..l1 {
        let x0 = input[index(0, 0, k, ido, 2)];
        let x1 = input[index(ido - 1, 1, k, ido, 2)];
        output[index(0, k, 0, ido, l1)] = x0 + x1;
        output[index(0, k, 1, ido, l1)] = x0 - x1;
    }
    if ido.is_multiple_of(2) {
        for k in 0..l1 {
            output[index(ido - 1, k, 0, ido, l1)] =
                scalar::<T>(2.0) * input[index(ido - 1, 0, k, ido, 2)];
            output[index(ido - 1, k, 1, ido, l1)] =
                -scalar::<T>(2.0) * input[index(0, 1, k, ido, 2)];
        }
    }
    if ido <= 2 {
        return;
    }
    let branch_len = ido * l1;
    let (output0, output1) = output.split_at_mut(branch_len);
    for k in 0..l1 {
        let input0 = &input[k * ido..(k + 1) * ido];
        let input1 = &input[branch_len + k * ido..branch_len + (k + 1) * ido];
        let output0 = &mut output0[k * ido..(k + 1) * ido];
        let output1 = &mut output1[k * ido..(k + 1) * ido];
        let scalar_start = if ido >= 16
            && let Some(kernel) = radix2_interior_kernel
        {
            // SAFETY: plan construction installs this kernel only after AVX2
            // runtime detection. Each branch slice has exactly `ido` values,
            // and the pass owns the complete radix-two twiddle table.
            unsafe {
                kernel(
                    [input0, input1],
                    [&mut *output0, &mut *output1],
                    ido,
                    &pass.twiddles,
                )
            }
        } else {
            2
        };
        for i in (scalar_start..ido).step_by(2) {
            let ic = ido - i;
            let a = input0[i - 1];
            let b = input1[ic - 1];
            let c = input0[i];
            let d = input1[ic];
            let (x0, tr2) = (a + b, a - b);
            let (ti2, x1) = (c + d, c - d);
            let (y_i, y_im1) =
                multiply_pm(twiddle(pass, 1, i - 2), twiddle(pass, 1, i - 1), ti2, tr2);
            output0[i - 1] = x0;
            output0[i] = x1;
            output1[i] = y_i;
            output1[i - 1] = y_im1;
        }
    }
}

fn forward_radix4<T: Float + FromPrimitive>(
    pass: &Pass<T>,
    input: &[T],
    output: &mut [T],
    radix4_forward_interior_kernel: Option<RealRadix4ForwardInteriorKernel<T>>,
) {
    let l1 = pass.l1;
    let ido = pass.ido;
    if ido == 1 {
        let (groups, _) = output.as_chunks_mut::<4>();
        for (k, dst) in groups.iter_mut().enumerate() {
            let x0 = input[k];
            let x1 = input[l1 + k];
            let x2 = input[2 * l1 + k];
            let x3 = input[3 * l1 + k];
            let tr1 = x3 + x1;
            let tr2 = x0 + x2;
            dst[0] = tr2 + tr1;
            dst[1] = x0 - x2;
            dst[2] = x3 - x1;
            dst[3] = tr2 - tr1;
        }
        return;
    }
    let hsqt2 = scalar::<T>(std::f64::consts::FRAC_1_SQRT_2);
    for k in 0..l1 {
        let x3 = input[index(0, k, 3, ido, l1)];
        let x1 = input[index(0, k, 1, ido, l1)];
        let tr1 = x3 + x1;
        output[index(0, 2, k, ido, 4)] = x3 - x1;
        let x0 = input[index(0, k, 0, ido, l1)];
        let x2 = input[index(0, k, 2, ido, l1)];
        let tr2 = x0 + x2;
        output[index(ido - 1, 1, k, ido, 4)] = x0 - x2;
        output[index(0, 0, k, ido, 4)] = tr2 + tr1;
        output[index(ido - 1, 3, k, ido, 4)] = tr2 - tr1;
    }
    if ido.is_multiple_of(2) {
        for k in 0..l1 {
            let x1 = input[index(ido - 1, k, 1, ido, l1)];
            let x3 = input[index(ido - 1, k, 3, ido, l1)];
            let ti1 = -hsqt2 * (x1 + x3);
            let tr1 = hsqt2 * (x1 - x3);
            let x0 = input[index(ido - 1, k, 0, ido, l1)];
            let x2 = input[index(ido - 1, k, 2, ido, l1)];
            output[index(ido - 1, 0, k, ido, 4)] = x0 + tr1;
            output[index(ido - 1, 2, k, ido, 4)] = x0 - tr1;
            output[index(0, 3, k, ido, 4)] = ti1 + x2;
            output[index(0, 1, k, ido, 4)] = ti1 - x2;
        }
    }
    if ido <= 2 {
        return;
    }
    let input_branch_len = ido * l1;
    let output_block_len = 4 * ido;
    for k in 0..l1 {
        let input0 = &input[k * ido..(k + 1) * ido];
        let input1 = &input[input_branch_len + k * ido..input_branch_len + (k + 1) * ido];
        let input2 = &input[2 * input_branch_len + k * ido..2 * input_branch_len + (k + 1) * ido];
        let input3 = &input[3 * input_branch_len + k * ido..3 * input_branch_len + (k + 1) * ido];
        let output_block = &mut output[k * output_block_len..(k + 1) * output_block_len];
        let (output0, tail) = output_block.split_at_mut(ido);
        let (output1, tail) = tail.split_at_mut(ido);
        let (output2, output3) = tail.split_at_mut(ido);
        let scalar_start = if ido >= 16
            && let Some(kernel) = radix4_forward_interior_kernel
        {
            // SAFETY: plan construction installs this kernel only after AVX2
            // runtime detection. Each input and output branch has exactly
            // `ido` scalars, and the pass owns all three twiddle branches.
            unsafe {
                kernel(
                    [input0, input1, input2, input3],
                    [&mut *output0, &mut *output1, &mut *output2, &mut *output3],
                    ido,
                    &pass.twiddles,
                )
            }
        } else {
            2
        };
        for i in (scalar_start..ido).step_by(2) {
            let ic = ido - i;
            let (cr2, ci2) = multiply_pm(
                twiddle(pass, 1, i - 2),
                twiddle(pass, 1, i - 1),
                input1[i - 1],
                input1[i],
            );
            let (cr3, ci3) = multiply_pm(
                twiddle(pass, 2, i - 2),
                twiddle(pass, 2, i - 1),
                input2[i - 1],
                input2[i],
            );
            let (cr4, ci4) = multiply_pm(
                twiddle(pass, 3, i - 2),
                twiddle(pass, 3, i - 1),
                input3[i - 1],
                input3[i],
            );
            let (tr1, tr4) = (cr4 + cr2, cr4 - cr2);
            let (ti1, ti4) = (ci2 + ci4, ci2 - ci4);
            let (tr2, tr3) = (input0[i - 1] + cr3, input0[i - 1] - cr3);
            let (ti2, ti3) = (input0[i] + ci3, input0[i] - ci3);
            output0[i - 1] = tr2 + tr1;
            output3[ic - 1] = tr2 - tr1;
            output0[i] = ti1 + ti2;
            output3[ic] = ti1 - ti2;
            output2[i - 1] = tr3 + ti4;
            output1[ic - 1] = tr3 - ti4;
            output2[i] = tr4 + ti3;
            output1[ic] = tr4 - ti3;
        }
    }
}

fn inverse_radix4<T: Float + FromPrimitive>(
    pass: &Pass<T>,
    input: &[T],
    output: &mut [T],
    radix4_interior_kernel: Option<RealRadix4InteriorKernel<T>>,
) {
    let l1 = pass.l1;
    let ido = pass.ido;
    if ido == 1 {
        let (branch0, tail) = output.split_at_mut(l1);
        let (branch1, tail) = tail.split_at_mut(l1);
        let (branch2, branch3) = tail.split_at_mut(l1);
        let two = scalar::<T>(2.0);
        let (groups, _) = input.as_chunks::<4>();
        for (k, src) in groups.iter().enumerate() {
            let tr2 = src[0] + src[3];
            let tr1 = src[0] - src[3];
            let tr3 = two * src[1];
            let tr4 = two * src[2];
            branch0[k] = tr2 + tr3;
            branch1[k] = tr1 - tr4;
            branch2[k] = tr2 - tr3;
            branch3[k] = tr1 + tr4;
        }
        return;
    }
    let sqrt2 = scalar::<T>(std::f64::consts::SQRT_2);
    let two = scalar::<T>(2.0);
    let block_len = 4 * ido;
    let branch_len = ido * l1;
    debug_assert_eq!(input.len(), block_len * l1);
    debug_assert_eq!(output.len(), 4 * branch_len);
    let (output0, tail) = output.split_at_mut(branch_len);
    let (output1, tail) = tail.split_at_mut(branch_len);
    let (output2, output3) = tail.split_at_mut(branch_len);
    let output_blocks = output0
        .chunks_exact_mut(ido)
        .zip(output1.chunks_exact_mut(ido))
        .zip(output2.chunks_exact_mut(ido))
        .zip(output3.chunks_exact_mut(ido));
    for (input_block, output_block) in input.chunks_exact(block_len).zip(output_blocks) {
        let (input0, tail) = input_block.split_at(ido);
        let (input1, tail) = tail.split_at(ido);
        let (input2, input3) = tail.split_at(ido);
        let (((output0, output1), output2), output3) = output_block;

        let x0 = input0[0];
        let x3 = input3[ido - 1];
        let (tr2, tr1) = (x0 + x3, x0 - x3);
        let tr3 = two * input1[ido - 1];
        let tr4 = two * input2[0];
        output0[0] = tr2 + tr3;
        output2[0] = tr2 - tr3;
        output3[0] = tr1 + tr4;
        output1[0] = tr1 - tr4;

        if ido.is_multiple_of(2) {
            let (ti1, ti2) = (input3[0] + input1[0], input3[0] - input1[0]);
            let x0 = input0[ido - 1];
            let x2 = input2[ido - 1];
            let (tr2, tr1) = (x0 + x2, x0 - x2);
            output0[ido - 1] = tr2 + tr2;
            output1[ido - 1] = sqrt2 * (tr1 - ti1);
            output2[ido - 1] = ti2 + ti2;
            output3[ido - 1] = -sqrt2 * (tr1 + ti1);
        }

        if ido <= 2 {
            continue;
        }
        let scalar_start = if ido >= 16
            && let Some(kernel) = radix4_interior_kernel
        {
            // SAFETY: plan construction installs this kernel only after AVX2
            // runtime detection. The current input block and four output
            // branches each have exactly `ido` scalar values, and the pass
            // supplies the complete radix-four twiddle table.
            unsafe {
                kernel(
                    input_block,
                    [&mut *output0, &mut *output1, &mut *output2, &mut *output3],
                    ido,
                    &pass.twiddles,
                )
            }
        } else {
            2
        };
        for i in (scalar_start..ido).step_by(2) {
            let ic = ido - i;
            let (tr2, tr1) = (
                input0[i - 1] + input3[ic - 1],
                input0[i - 1] - input3[ic - 1],
            );
            let (ti1, ti2) = (input0[i] + input3[ic], input0[i] - input3[ic]);
            let (tr4, ti3) = (input2[i] + input1[ic], input2[i] - input1[ic]);
            let (tr3, ti4) = (
                input2[i - 1] + input1[ic - 1],
                input2[i - 1] - input1[ic - 1],
            );
            let (x0r, cr3) = (tr2 + tr3, tr2 - tr3);
            let (x0i, ci3) = (ti2 + ti3, ti2 - ti3);
            let (cr4, cr2) = (tr1 + tr4, tr1 - tr4);
            let (ci2, ci4) = (ti1 + ti4, ti1 - ti4);
            let (y1i, y1r) =
                multiply_pm(twiddle(pass, 1, i - 2), twiddle(pass, 1, i - 1), ci2, cr2);
            let (y2i, y2r) =
                multiply_pm(twiddle(pass, 2, i - 2), twiddle(pass, 2, i - 1), ci3, cr3);
            let (y3i, y3r) =
                multiply_pm(twiddle(pass, 3, i - 2), twiddle(pass, 3, i - 1), ci4, cr4);
            output0[i - 1] = x0r;
            output0[i] = x0i;
            output1[i] = y1i;
            output1[i - 1] = y1r;
            output2[i] = y2i;
            output2[i - 1] = y2r;
            output3[i] = y3i;
            output3[i - 1] = y3r;
        }
    }
}

fn unpack_spectrum<T: Float>(packed: &[T], spectrum: &mut [Complex<T>]) {
    let len = packed.len();
    spectrum[0] = Complex::new(packed[0], T::zero());
    for k in 1..=len / 2 {
        if k == len / 2 {
            spectrum[k] = Complex::new(packed[len - 1], T::zero());
        } else {
            spectrum[k] = Complex::new(packed[2 * k - 1], packed[2 * k]);
        }
    }
}

fn pack_spectrum<T: Float>(spectrum: &[Complex<T>], packed: &mut [T]) {
    let len = packed.len();
    debug_assert_eq!(spectrum.len(), len / 2 + 1);
    // DC, every interior real/imaginary pair, and Nyquist cover the full
    // packed buffer for the even lengths accepted by this executor.
    packed[0] = spectrum[0].re;
    for (k, &value) in spectrum.iter().enumerate().skip(1) {
        if k == len / 2 {
            packed[len - 1] = value.re;
        } else {
            packed[2 * k - 1] = value.re;
            packed[2 * k] = value.im;
        }
    }
}

fn scalar<T: FromPrimitive>(value: f64) -> T {
    T::from_f64(value).expect("f32 and f64 can represent FFT constants")
}
