// SPDX-FileCopyrightText: 2010-2026 Max-Planck-Society (DUCC FFT pass equations)
// SPDX-FileCopyrightText: 2019 Peter Bell (DUCC FFT pass equations)
// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause
//
// Scalar radix-2/radix-4 real multipass prototype adapted from DUCC
// fft1d_impl.h at c1ed01b8fda69d2a378190f0915f1a19762dddbf. It uses DUCC's
// packed-real pass layout and pass order for smooth powers of two.

use std::time::Instant;

use num_complex::Complex;

use duccfft::RealFftPlan;

use super::{BATCH_COUNT, Complex64, Mode, RealPlan, allocation_ns, measure, plan_ns};

type C = Complex<f64>;

struct Pass {
    radix: usize,
    l1: usize,
    ido: usize,
    twiddles: Vec<f64>,
}

struct PackedRealPlan {
    len: usize,
    passes: Vec<Pass>,
    plan_bytes: usize,
}

struct PackedRealWorkspace {
    first: Vec<f64>,
    second: Vec<f64>,
}

impl PackedRealPlan {
    fn new(len: usize) -> Self {
        assert!(len.is_power_of_two(), "prototype accepts powers of two");
        let factors = factorize(len);
        assert!(factors.iter().all(|&radix| matches!(radix, 2 | 4)));
        let mut passes = Vec::with_capacity(factors.len());
        let mut l1 = 1;
        let mut plan_bytes = 0;
        for radix in factors {
            let ido = len / (l1 * radix);
            let twiddle_len = (radix - 1)
                .checked_mul(ido.saturating_sub(1))
                .expect("twiddle table length overflow");
            let mut twiddles = vec![0.0; twiddle_len];
            for branch in 1..radix {
                for i in 1..=(ido.saturating_sub(1)) / 2 {
                    let angle = std::f64::consts::TAU * (branch * l1 * i) as f64 / len as f64;
                    let root = C::new(angle.cos(), angle.sin());
                    let offset = (branch - 1) * (ido - 1) + 2 * i - 2;
                    twiddles[offset] = root.re;
                    twiddles[offset + 1] = root.im;
                }
            }
            plan_bytes += twiddles.len() * std::mem::size_of::<f64>();
            passes.push(Pass {
                radix,
                l1,
                ido,
                twiddles,
            });
            l1 = l1.checked_mul(radix).expect("pass length overflow");
        }
        Self {
            len,
            passes,
            plan_bytes,
        }
    }

    fn workspace(&self) -> PackedRealWorkspace {
        PackedRealWorkspace {
            first: vec![0.0; self.len],
            second: vec![0.0; self.len],
        }
    }

    fn scratch_bytes(&self) -> usize {
        2usize
            .checked_mul(self.len)
            .and_then(|elements| elements.checked_mul(std::mem::size_of::<f64>()))
            .expect("scratch byte count overflow")
    }

    fn forward(&self, input: &[f64], spectrum: &mut [C], workspace: &mut PackedRealWorkspace) {
        assert_eq!(input.len(), self.len);
        assert_eq!(spectrum.len(), self.len / 2 + 1);
        workspace.first.copy_from_slice(input);
        let mut current_is_first = true;
        for pass in self.passes.iter().rev() {
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

    fn inverse(&self, spectrum: &[C], output: &mut [f64], workspace: &mut PackedRealWorkspace) {
        assert_eq!(spectrum.len(), self.len / 2 + 1);
        assert_eq!(output.len(), self.len);
        pack_spectrum(spectrum, &mut workspace.first);
        let mut current_is_first = true;
        for pass in &self.passes {
            if current_is_first {
                execute_inverse_pass(pass, &workspace.first, &mut workspace.second);
            } else {
                execute_inverse_pass(pass, &workspace.second, &mut workspace.first);
            }
            current_is_first = !current_is_first;
        }
        let transformed = if current_is_first {
            &workspace.first
        } else {
            &workspace.second
        };
        let scale = 1.0 / self.len as f64;
        for (sample, &value) in output.iter_mut().zip(transformed) {
            *sample = value * scale;
        }
    }
}

fn factorize(mut remaining: usize) -> Vec<usize> {
    let mut factors = Vec::new();
    while remaining & 3 == 0 {
        factors.push(4);
        remaining >>= 2;
    }
    if remaining & 1 == 0 {
        remaining >>= 1;
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
    factors
}

#[inline(always)]
fn index(a: usize, b: usize, c: usize, ido: usize, middle: usize) -> usize {
    a + ido * (b + middle * c)
}

#[inline(always)]
fn multiply_pm(c: f64, d: f64, e: f64, f: f64) -> (f64, f64) {
    (c * e + d * f, c * f - d * e)
}

#[inline(always)]
fn twiddle(pass: &Pass, branch: usize, component: usize) -> f64 {
    pass.twiddles[(branch - 1) * (pass.ido - 1) + component]
}

fn execute_forward_pass(pass: &Pass, input: &[f64], output: &mut [f64]) {
    match pass.radix {
        2 => forward_radix2(pass, input, output),
        4 => forward_radix4(pass, input, output),
        _ => unreachable!("prototype supports radix 2 and 4"),
    }
}

fn execute_inverse_pass(pass: &Pass, input: &[f64], output: &mut [f64]) {
    match pass.radix {
        2 => inverse_radix2(pass, input, output),
        4 => inverse_radix4(pass, input, output),
        _ => unreachable!("prototype supports radix 2 and 4"),
    }
}

fn forward_radix2(pass: &Pass, input: &[f64], output: &mut [f64]) {
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

fn inverse_radix2(pass: &Pass, input: &[f64], output: &mut [f64]) {
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
            output[index(ido - 1, k, 0, ido, l1)] = 2.0 * input[index(ido - 1, 0, k, ido, 2)];
            output[index(ido - 1, k, 1, ido, l1)] = -2.0 * input[index(0, 1, k, ido, 2)];
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

fn forward_radix4(pass: &Pass, input: &[f64], output: &mut [f64]) {
    let l1 = pass.l1;
    let ido = pass.ido;
    let hsqt2 = std::f64::consts::FRAC_1_SQRT_2;
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

fn inverse_radix4(pass: &Pass, input: &[f64], output: &mut [f64]) {
    let l1 = pass.l1;
    let ido = pass.ido;
    let sqrt2 = std::f64::consts::SQRT_2;
    for k in 0..l1 {
        let x0 = input[index(0, 0, k, ido, 4)];
        let x3 = input[index(ido - 1, 3, k, ido, 4)];
        let (tr2, tr1) = (x0 + x3, x0 - x3);
        let tr3 = 2.0 * input[index(ido - 1, 1, k, ido, 4)];
        let tr4 = 2.0 * input[index(0, 2, k, ido, 4)];
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

fn unpack_spectrum(packed: &[f64], spectrum: &mut [C]) {
    let len = packed.len();
    spectrum[0] = C::new(packed[0], 0.0);
    for k in 1..=len / 2 {
        if k == len / 2 {
            spectrum[k] = C::new(packed[len - 1], 0.0);
        } else {
            spectrum[k] = C::new(packed[2 * k - 1], packed[2 * k]);
        }
    }
}

fn pack_spectrum(spectrum: &[C], packed: &mut [f64]) {
    packed.fill(0.0);
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

fn verify(len: usize) {
    let packed_plan = PackedRealPlan::new(len);
    let mut packed_workspace = packed_plan.workspace();
    let real_plan = RealFftPlan::<f64>::new(len).expect("real plan");
    let mut real_workspace = real_plan.workspace().expect("real workspace");
    let input: Vec<_> = (0..len).map(|i| super::real_value(i, 0)).collect();
    let mut actual = vec![Complex64::new(0.0, 0.0); len / 2 + 1];
    let mut expected = vec![Complex64::new(0.0, 0.0); len / 2 + 1];
    packed_plan.forward(&input, &mut actual, &mut packed_workspace);
    real_plan
        .forward(&input, &mut expected, &mut real_workspace)
        .expect("reference forward");
    assert_close_complex(len, &actual, &expected, "forward vs DUCCFFT");

    let rust_plan = RealPlan::new(len);
    let mut rust_transformed = vec![Complex64::new(0.0, 0.0); rust_plan.complex.n];
    let mut rust_scratch = vec![Complex64::new(0.0, 0.0); rust_plan.scratch_len()];
    let mut rust_expected = vec![Complex64::new(0.0, 0.0); len / 2 + 1];
    rust_plan.forward(
        &input,
        &mut rust_expected,
        &mut rust_transformed,
        &mut rust_scratch,
    );
    assert_close_complex(len, &actual, &rust_expected, "forward vs RustFFT adapter");

    let spectrum: Vec<_> = (0..=len / 2)
        .map(|k| super::spectrum_value(k, len, 0))
        .collect();
    let mut actual = vec![0.0; len];
    let mut expected = vec![0.0; len];
    packed_plan.inverse(&spectrum, &mut actual, &mut packed_workspace);
    real_plan
        .inverse(&spectrum, &mut expected, &mut real_workspace)
        .expect("reference inverse");
    assert_close_real(len, &actual, &expected, "inverse vs DUCCFFT");

    let mut rust_expected = vec![0.0; len];
    rust_plan.inverse(
        &spectrum,
        &mut rust_expected,
        &mut rust_transformed,
        &mut rust_scratch,
    );
    assert_close_real(len, &actual, &rust_expected, "inverse vs RustFFT adapter");
}

fn assert_close_complex(len: usize, actual: &[C], expected: &[C], label: &str) {
    let error = actual
        .iter()
        .zip(expected)
        .map(|(&left, &right)| (left - right).norm())
        .fold(0.0_f64, f64::max);
    let scale = expected
        .iter()
        .map(|value| value.norm())
        .fold(1.0_f64, f64::max);
    let tolerance = 32.0 * f64::EPSILON * len as f64 * len.ilog2().max(1) as f64 * scale;
    assert!(
        actual
            .iter()
            .chain(expected)
            .all(|value| value.re.is_finite() && value.im.is_finite()),
        "{label} produced a nonfinite value at n={len}"
    );
    assert!(
        error <= tolerance,
        "{label} error={error} exceeds {tolerance} at n={len}"
    );
}

fn assert_close_real(len: usize, actual: &[f64], expected: &[f64], label: &str) {
    let error = actual
        .iter()
        .zip(expected)
        .map(|(&left, &right)| (left - right).abs())
        .fold(0.0_f64, f64::max);
    let scale = expected
        .iter()
        .map(|value| value.abs())
        .fold(1.0_f64, f64::max);
    let tolerance = 32.0 * f64::EPSILON * len as f64 * len.ilog2().max(1) as f64 * scale;
    assert!(
        actual.iter().chain(expected).all(|value| value.is_finite()),
        "{label} produced a nonfinite value at n={len}"
    );
    assert!(
        error <= tolerance,
        "{label} error={error} exceeds {tolerance} at n={len}"
    );
}

pub(super) fn bench(lengths: impl Iterator<Item = String>) {
    let lengths: Vec<usize> = lengths
        .map(|value| value.parse().expect("length must be an integer"))
        .collect();
    println!(
        "implementation,mode,n,batch_count,plan_ns,workspace_ns,plan_table_bytes,scratch_bytes,public_operation_ns_per_transform,batch_operation_ns,checksum"
    );
    for len in lengths {
        verify(len);
        for rows in [1, BATCH_COUNT] {
            benchmark_len(len, rows);
        }
    }
}

fn benchmark_len(len: usize, rows: usize) {
    let bins = len / 2 + 1;
    let inputs: Vec<_> = (0..rows)
        .flat_map(|row| (0..len).map(move |index| super::real_value(index, row)))
        .collect();
    let spectra: Vec<_> = (0..rows)
        .flat_map(|row| (0..bins).map(move |k| super::spectrum_value(k, len, row)))
        .collect();
    let operations = [(Mode::RealForward, "r2c"), (Mode::RealInverse, "c2r")];

    let prototype = PackedRealPlan::new(len);
    let prototype_plan_ns = average_ns(20, || {
        std::hint::black_box(PackedRealPlan::new(len));
    });
    let prototype_workspace_ns = average_ns(20, || {
        std::hint::black_box(prototype.workspace());
    });
    let prototype_scratch_bytes = prototype.scratch_bytes();
    let prototype_plan_bytes = prototype.plan_bytes;
    let mut prototype_workspace = prototype.workspace();
    let mut prototype_output = vec![Complex64::new(0.0, 0.0); rows * bins];
    let mut prototype_real = vec![0.0; rows * len];

    let real_plan = RealFftPlan::<f64>::new(len).expect("real plan");
    let native_plan_ns = average_ns(20, || {
        std::hint::black_box(RealFftPlan::<f64>::new(len).expect("native real plan"));
    });
    let native_workspace_ns = average_ns(20, || {
        std::hint::black_box(real_plan.workspace().expect("real workspace"));
    });
    let mut native_workspace = real_plan.workspace().expect("real workspace");
    let native_scratch_bytes = real_plan
        .workspace()
        .expect("real workspace")
        .scratch_bytes()
        .expect("real scratch byte count");
    let mut native_output = vec![Complex64::new(0.0, 0.0); rows * bins];
    let mut native_real = vec![0.0; rows * len];

    let rust_plan = RealPlan::new(len);
    let rust_plan_ns = plan_ns(len, true, false);
    let transform_len = rust_plan.complex.n;
    let rust_workspace_ns = allocation_ns::<Complex64>(&[transform_len, rust_plan.scratch_len()]);
    let rust_scratch_bytes =
        (transform_len + rust_plan.scratch_len()) * std::mem::size_of::<Complex64>();
    let mut rust_transforms = vec![Complex64::new(0.0, 0.0); rows * transform_len];
    let mut rust_scratch = vec![Complex64::new(0.0, 0.0); rust_plan.scratch_len()];
    let mut rust_output = vec![Complex64::new(0.0, 0.0); rows * bins];
    let mut rust_real = vec![0.0; rows * len];

    for (mode, name) in operations {
        let (prototype_ns, prototype_checksum) = match mode {
            Mode::RealForward => {
                let elapsed = timed_per_transform(rows, || {
                    let start = Instant::now();
                    for row in 0..rows {
                        prototype.forward(
                            &inputs[row * len..(row + 1) * len],
                            &mut prototype_output[row * bins..(row + 1) * bins],
                            &mut prototype_workspace,
                        );
                    }
                    let elapsed = start.elapsed();
                    std::hint::black_box(prototype_output.as_slice());
                    elapsed
                });
                (elapsed, prototype_output[0].re + prototype_output[0].im)
            }
            Mode::RealInverse => {
                let elapsed = timed_per_transform(rows, || {
                    let start = Instant::now();
                    for row in 0..rows {
                        prototype.inverse(
                            &spectra[row * bins..(row + 1) * bins],
                            &mut prototype_real[row * len..(row + 1) * len],
                            &mut prototype_workspace,
                        );
                    }
                    let elapsed = start.elapsed();
                    std::hint::black_box(prototype_real.as_slice());
                    elapsed
                });
                (elapsed, prototype_real[0])
            }
            _ => unreachable!(),
        };
        let (native_ns, native_checksum) = match mode {
            Mode::RealForward => {
                let elapsed = timed_per_transform(rows, || {
                    let start = Instant::now();
                    for row in 0..rows {
                        real_plan
                            .forward(
                                &inputs[row * len..(row + 1) * len],
                                &mut native_output[row * bins..(row + 1) * bins],
                                &mut native_workspace,
                            )
                            .expect("native forward");
                    }
                    let elapsed = start.elapsed();
                    std::hint::black_box(native_output.as_slice());
                    elapsed
                });
                (elapsed, native_output[0].re + native_output[0].im)
            }
            Mode::RealInverse => {
                let elapsed = timed_per_transform(rows, || {
                    let start = Instant::now();
                    for row in 0..rows {
                        real_plan
                            .inverse(
                                &spectra[row * bins..(row + 1) * bins],
                                &mut native_real[row * len..(row + 1) * len],
                                &mut native_workspace,
                            )
                            .expect("native inverse");
                    }
                    let elapsed = start.elapsed();
                    std::hint::black_box(native_real.as_slice());
                    elapsed
                });
                (elapsed, native_real[0])
            }
            _ => unreachable!(),
        };
        let (rust_ns, rust_checksum) = match mode {
            Mode::RealForward => {
                let elapsed = timed_per_transform(rows, || {
                    let start = Instant::now();
                    for row in 0..rows {
                        let transform =
                            &mut rust_transforms[row * transform_len..(row + 1) * transform_len];
                        rust_plan.forward(
                            &inputs[row * len..(row + 1) * len],
                            &mut rust_output[row * bins..(row + 1) * bins],
                            transform,
                            &mut rust_scratch,
                        );
                    }
                    let elapsed = start.elapsed();
                    std::hint::black_box(rust_output.as_slice());
                    elapsed
                });
                (elapsed, rust_output[0].re + rust_output[0].im)
            }
            Mode::RealInverse => {
                let elapsed = timed_per_transform(rows, || {
                    let start = Instant::now();
                    for row in 0..rows {
                        let transform =
                            &mut rust_transforms[row * transform_len..(row + 1) * transform_len];
                        rust_plan.inverse(
                            &spectra[row * bins..(row + 1) * bins],
                            &mut rust_real[row * len..(row + 1) * len],
                            transform,
                            &mut rust_scratch,
                        );
                    }
                    let elapsed = start.elapsed();
                    std::hint::black_box(rust_real.as_slice());
                    elapsed
                });
                (elapsed, rust_real[0])
            }
            _ => unreachable!(),
        };
        emit(
            "packed-real multipass",
            name,
            len,
            rows,
            prototype_plan_ns,
            prototype_workspace_ns,
            Some(prototype_plan_bytes),
            prototype_scratch_bytes,
            prototype_ns,
            prototype_checksum,
        );
        emit(
            "DUCCFFT recursive",
            name,
            len,
            rows,
            native_plan_ns,
            native_workspace_ns,
            None,
            native_scratch_bytes,
            native_ns,
            native_checksum,
        );
        emit(
            "RustFFT real adapter",
            name,
            len,
            rows,
            rust_plan_ns,
            rust_workspace_ns,
            None,
            rust_scratch_bytes,
            rust_ns,
            rust_checksum,
        );
    }
}

fn timed_per_transform(rows: usize, operation: impl FnMut() -> std::time::Duration) -> f64 {
    measure(operation).as_nanos() as f64 / rows as f64
}

fn average_ns(repeats: usize, mut operation: impl FnMut()) -> f64 {
    let start = Instant::now();
    for _ in 0..repeats {
        operation();
    }
    start.elapsed().as_nanos() as f64 / repeats as f64
}

#[allow(clippy::too_many_arguments)]
fn emit(
    implementation: &str,
    mode: &str,
    len: usize,
    rows: usize,
    plan_ns: f64,
    workspace_ns: f64,
    plan_table_bytes: Option<usize>,
    scratch_bytes: usize,
    per_transform_ns: f64,
    checksum: f64,
) {
    let plan_table_bytes = plan_table_bytes.map_or_else(String::new, |value| value.to_string());
    println!(
        "{implementation},{mode},{len},{rows},{plan_ns:.3},{workspace_ns:.3},{plan_table_bytes},{scratch_bytes},{per_transform_ns:.3},{:.3},{checksum:.12}",
        per_transform_ns * rows as f64
    );
}
