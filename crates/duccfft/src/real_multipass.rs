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

struct Pass<T> {
    radix: usize,
    l1: usize,
    ido: usize,
    twiddles: Vec<T>,
}

pub(crate) struct RealMultipassPlan<T> {
    len: usize,
    passes: Vec<Pass<T>>,
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
        Ok(Some(Self { len, passes }))
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
        execute_forward_pass(first_pass, input, &mut workspace.second);
        let mut current_is_first = false;
        for pass in passes {
            if current_is_first {
                execute_forward_pass(pass, &workspace.first, &mut workspace.second);
            } else {
                execute_forward_pass(pass, &workspace.second, &mut workspace.first);
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
        let last_pass = self.passes.len() - 1;
        for (index, pass) in self.passes.iter().enumerate() {
            if index == last_pass {
                if current_is_first {
                    execute_inverse_pass(pass, &workspace.first, output);
                } else {
                    execute_inverse_pass(pass, &workspace.second, output);
                }
            } else if current_is_first {
                execute_inverse_pass(pass, &workspace.first, &mut workspace.second);
                current_is_first = false;
            } else {
                execute_inverse_pass(pass, &workspace.second, &mut workspace.first);
                current_is_first = true;
            }
        }
        let scale = scalar::<T>(1.0 / self.len as f64);
        for sample in output {
            *sample = *sample * scale;
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

fn execute_forward_pass<T: Float + FromPrimitive>(pass: &Pass<T>, input: &[T], output: &mut [T]) {
    match pass.radix {
        2 => forward_radix2(pass, input, output),
        4 => forward_radix4(pass, input, output),
        _ => unreachable!("real multipass plan contains only radix 2 and 4"),
    }
}

fn execute_inverse_pass<T: Float + FromPrimitive>(pass: &Pass<T>, input: &[T], output: &mut [T]) {
    match pass.radix {
        2 => inverse_radix2(pass, input, output),
        4 => inverse_radix4(pass, input, output),
        _ => unreachable!("real multipass plan contains only radix 2 and 4"),
    }
}

fn forward_radix2<T: Float + FromPrimitive>(pass: &Pass<T>, input: &[T], output: &mut [T]) {
    let l1 = pass.l1;
    let ido = pass.ido;
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

fn inverse_radix2<T: Float + FromPrimitive>(pass: &Pass<T>, input: &[T], output: &mut [T]) {
    let l1 = pass.l1;
    let ido = pass.ido;
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
    for k in 0..l1 {
        for i in (2..ido).step_by(2) {
            let ic = ido - i;
            let a = input[index(i - 1, 0, k, ido, 2)];
            let b = input[index(ic - 1, 1, k, ido, 2)];
            let c = input[index(i, 0, k, ido, 2)];
            let d = input[index(ic, 1, k, ido, 2)];
            let (x0, tr2) = (a + b, a - b);
            let (ti2, x1) = (c + d, c - d);
            let (y_i, y_im1) =
                multiply_pm(twiddle(pass, 1, i - 2), twiddle(pass, 1, i - 1), ti2, tr2);
            output[index(i - 1, k, 0, ido, l1)] = x0;
            output[index(i, k, 0, ido, l1)] = x1;
            output[index(i, k, 1, ido, l1)] = y_i;
            output[index(i - 1, k, 1, ido, l1)] = y_im1;
        }
    }
}

fn forward_radix4<T: Float + FromPrimitive>(pass: &Pass<T>, input: &[T], output: &mut [T]) {
    let l1 = pass.l1;
    let ido = pass.ido;
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
    for k in 0..l1 {
        for i in (2..ido).step_by(2) {
            let ic = ido - i;
            let (cr2, ci2) = multiply_pm(
                twiddle(pass, 1, i - 2),
                twiddle(pass, 1, i - 1),
                input[index(i - 1, k, 1, ido, l1)],
                input[index(i, k, 1, ido, l1)],
            );
            let (cr3, ci3) = multiply_pm(
                twiddle(pass, 2, i - 2),
                twiddle(pass, 2, i - 1),
                input[index(i - 1, k, 2, ido, l1)],
                input[index(i, k, 2, ido, l1)],
            );
            let (cr4, ci4) = multiply_pm(
                twiddle(pass, 3, i - 2),
                twiddle(pass, 3, i - 1),
                input[index(i - 1, k, 3, ido, l1)],
                input[index(i, k, 3, ido, l1)],
            );
            let (tr1, tr4) = (cr4 + cr2, cr4 - cr2);
            let (ti1, ti4) = (ci2 + ci4, ci2 - ci4);
            let (tr2, tr3) = (
                input[index(i - 1, k, 0, ido, l1)] + cr3,
                input[index(i - 1, k, 0, ido, l1)] - cr3,
            );
            let (ti2, ti3) = (
                input[index(i, k, 0, ido, l1)] + ci3,
                input[index(i, k, 0, ido, l1)] - ci3,
            );
            output[index(i - 1, 0, k, ido, 4)] = tr2 + tr1;
            output[index(ic - 1, 3, k, ido, 4)] = tr2 - tr1;
            output[index(i, 0, k, ido, 4)] = ti1 + ti2;
            output[index(ic, 3, k, ido, 4)] = ti1 - ti2;
            output[index(i - 1, 2, k, ido, 4)] = tr3 + ti4;
            output[index(ic - 1, 1, k, ido, 4)] = tr3 - ti4;
            output[index(i, 2, k, ido, 4)] = tr4 + ti3;
            output[index(ic, 1, k, ido, 4)] = tr4 - ti3;
        }
    }
}

fn inverse_radix4<T: Float + FromPrimitive>(pass: &Pass<T>, input: &[T], output: &mut [T]) {
    let l1 = pass.l1;
    let ido = pass.ido;
    let sqrt2 = scalar::<T>(std::f64::consts::SQRT_2);
    for k in 0..l1 {
        let x0 = input[index(0, 0, k, ido, 4)];
        let x3 = input[index(ido - 1, 3, k, ido, 4)];
        let (tr2, tr1) = (x0 + x3, x0 - x3);
        let tr3 = scalar::<T>(2.0) * input[index(ido - 1, 1, k, ido, 4)];
        let tr4 = scalar::<T>(2.0) * input[index(0, 2, k, ido, 4)];
        output[index(0, k, 0, ido, l1)] = tr2 + tr3;
        output[index(0, k, 2, ido, l1)] = tr2 - tr3;
        output[index(0, k, 3, ido, l1)] = tr1 + tr4;
        output[index(0, k, 1, ido, l1)] = tr1 - tr4;
    }
    if ido.is_multiple_of(2) {
        for k in 0..l1 {
            let (ti1, ti2) = (
                input[index(0, 3, k, ido, 4)] + input[index(0, 1, k, ido, 4)],
                input[index(0, 3, k, ido, 4)] - input[index(0, 1, k, ido, 4)],
            );
            let x0 = input[index(ido - 1, 0, k, ido, 4)];
            let x2 = input[index(ido - 1, 2, k, ido, 4)];
            let (tr2, tr1) = (x0 + x2, x0 - x2);
            output[index(ido - 1, k, 0, ido, l1)] = tr2 + tr2;
            output[index(ido - 1, k, 1, ido, l1)] = sqrt2 * (tr1 - ti1);
            output[index(ido - 1, k, 2, ido, l1)] = ti2 + ti2;
            output[index(ido - 1, k, 3, ido, l1)] = -sqrt2 * (tr1 + ti1);
        }
    }
    if ido <= 2 {
        return;
    }
    for k in 0..l1 {
        for i in (2..ido).step_by(2) {
            let ic = ido - i;
            let (tr2, tr1) = (
                input[index(i - 1, 0, k, ido, 4)] + input[index(ic - 1, 3, k, ido, 4)],
                input[index(i - 1, 0, k, ido, 4)] - input[index(ic - 1, 3, k, ido, 4)],
            );
            let (ti1, ti2) = (
                input[index(i, 0, k, ido, 4)] + input[index(ic, 3, k, ido, 4)],
                input[index(i, 0, k, ido, 4)] - input[index(ic, 3, k, ido, 4)],
            );
            let (tr4, ti3) = (
                input[index(i, 2, k, ido, 4)] + input[index(ic, 1, k, ido, 4)],
                input[index(i, 2, k, ido, 4)] - input[index(ic, 1, k, ido, 4)],
            );
            let (tr3, ti4) = (
                input[index(i - 1, 2, k, ido, 4)] + input[index(ic - 1, 1, k, ido, 4)],
                input[index(i - 1, 2, k, ido, 4)] - input[index(ic - 1, 1, k, ido, 4)],
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
            output[index(i - 1, k, 0, ido, l1)] = x0r;
            output[index(i, k, 0, ido, l1)] = x0i;
            output[index(i, k, 1, ido, l1)] = y1i;
            output[index(i - 1, k, 1, ido, l1)] = y1r;
            output[index(i, k, 2, ido, l1)] = y2i;
            output[index(i - 1, k, 2, ido, l1)] = y2r;
            output[index(i, k, 3, ido, l1)] = y3i;
            output[index(i - 1, k, 3, ido, l1)] = y3r;
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
    packed.fill(T::zero());
    let len = packed.len();
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
