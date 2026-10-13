// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause
//
// Bounded full-operation experiment for DUCC's rfftp_complexify algorithm.
// It forces the packed-real wrapper around the crate's complex multipass plan
// at smooth even lengths where DUCC's scalar planner selects real passes.

use std::hint::black_box;
use std::time::{Duration, Instant};

use duccfft::{ComplexFftPlan, RealFftPlan};
use num_complex::Complex64;

type C = Complex64;
const ZERO: C = C::new(0.0, 0.0);
const SAMPLE_COUNT: usize = 7;
const BATCH_COUNT: usize = 32;

struct ForcedComplexify {
    len: usize,
    complex: ComplexFftPlan<f64>,
    workspace: duccfft::ComplexFftWorkspace<f64>,
    roots: Vec<C>,
    packed: Vec<C>,
}

impl ForcedComplexify {
    fn new(len: usize) -> Self {
        assert!(len.is_multiple_of(2));
        let complex_len = len / 2;
        let complex = ComplexFftPlan::<f64>::new(complex_len).expect("complex plan");
        let workspace = complex.workspace().expect("complex workspace");
        let roots = (0..complex_len)
            .map(|index| {
                let angle = std::f64::consts::TAU * index as f64 / len as f64;
                C::from_polar(1.0, angle)
            })
            .collect();
        Self {
            len,
            complex,
            workspace,
            roots,
            packed: vec![ZERO; complex_len],
        }
    }

    fn scratch_bytes(&self) -> usize {
        self.workspace
            .scratch_bytes()
            .expect("complex workspace byte count")
            + self.packed.len() * std::mem::size_of::<C>()
    }

    fn forward(&mut self, input: &[f64], spectrum: &mut [C]) {
        let half = self.len / 2;
        let (pairs, remainder) = input.as_chunks::<2>();
        debug_assert!(remainder.is_empty());
        for (slot, pair) in self.packed.iter_mut().zip(pairs) {
            *slot = C::new(pair[0], pair[1]);
        }
        self.complex
            .forward(&mut self.packed, &mut self.workspace)
            .expect("complex forward");
        spectrum[0] = C::new(self.packed[0].re + self.packed[0].im, 0.0);
        let mut index = 1;
        let mut mirror = half - 1;
        while index <= mirror {
            let (left, right) =
                complexify_forward_pair(self.packed[index], self.packed[mirror], self.roots[index]);
            spectrum[index] = left;
            spectrum[mirror] = right;
            index += 1;
            mirror -= 1;
        }
        spectrum[half] = C::new(self.packed[0].re - self.packed[0].im, 0.0);
    }

    fn inverse(&mut self, spectrum: &[C], output: &mut [f64]) {
        let half = self.len / 2;
        self.packed[0] = C::new(
            spectrum[0].re + spectrum[half].re,
            spectrum[0].re - spectrum[half].re,
        );
        let mut index = 1;
        let mut mirror = half - 1;
        while index <= mirror {
            let (left, right) = complexify_inverse_pair(
                spectrum[index],
                spectrum[mirror].conj(),
                self.roots[index],
            );
            self.packed[index] = left;
            self.packed[mirror] = right;
            index += 1;
            mirror -= 1;
        }
        self.complex
            .inverse(&mut self.packed, &mut self.workspace)
            .expect("complex inverse");
        let (pairs, remainder) = output.as_chunks_mut::<2>();
        debug_assert!(remainder.is_empty());
        for (pair, value) in pairs.iter_mut().zip(&self.packed) {
            pair[0] = value.re * 0.5;
            pair[1] = value.im * 0.5;
        }
    }
}

fn complexify_forward_pair(left: C, mirror: C, root: C) -> (C, C) {
    let even = C::new(left.re + mirror.re, left.im - mirror.im);
    let odd = C::new(left.im + mirror.im, mirror.re - left.re) * root.conj();
    (
        (even + odd) * 0.5,
        C::new((even.re - odd.re) * 0.5, (odd.im - even.im) * 0.5),
    )
}

fn complexify_inverse_pair(left: C, right: C, root: C) -> (C, C) {
    let even = left + right;
    let odd = (left - right) * root;
    (
        even + C::new(-odd.im, odd.re),
        even.conj() + C::new(odd.im, odd.re),
    )
}

fn real_value(index: usize, row: usize) -> f64 {
    let x = (index + 1 + 3 * row) as f64;
    (0.173 * x).sin() + 0.25 * (0.113 * (x + 1.0)).cos()
}

fn spectrum_value(index: usize, len: usize, row: usize) -> C {
    if index == 0 || index == len / 2 {
        return C::new(0.4 + 0.01 * (index + row) as f64, 0.0);
    }
    let x = (index + 1 + 5 * row) as f64;
    C::new(0.3 * (0.19 * x).sin(), 0.2 * (0.07 * (x + 1.0)).cos())
}

fn median(mut values: Vec<Duration>) -> Duration {
    values.sort_unstable();
    values[values.len() / 2]
}

fn measure(mut operation: impl FnMut()) -> f64 {
    operation();
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let start = Instant::now();
        operation();
        samples.push(start.elapsed());
    }
    median(samples).as_nanos() as f64
}

fn compare_outputs(len: usize, input: &[f64], spectrum: &[C], restored: &[f64]) {
    let plan = RealFftPlan::<f64>::new(len).expect("real reference plan");
    let mut workspace = plan.workspace().expect("real reference workspace");
    let mut expected_spectrum = vec![ZERO; plan.spectrum_len()];
    plan.forward(input, &mut expected_spectrum, &mut workspace)
        .expect("real reference forward");
    let forward_error = spectrum
        .iter()
        .zip(&expected_spectrum)
        .map(|(&actual, &expected)| (actual - expected).norm())
        .fold(0.0_f64, f64::max);
    assert!(
        spectrum
            .iter()
            .chain(expected_spectrum.iter())
            .all(|value| value.re.is_finite() && value.im.is_finite()),
        "nonfinite forward result at n={len}"
    );
    assert!(
        forward_error <= 2.0e-10 * len as f64,
        "n={len} forward error={forward_error}"
    );

    let mut expected_output = vec![0.0; len];
    plan.inverse(spectrum, &mut expected_output, &mut workspace)
        .expect("real reference inverse");
    let inverse_error = restored
        .iter()
        .zip(&expected_output)
        .map(|(&actual, &expected)| (actual - expected).abs())
        .fold(0.0_f64, f64::max);
    assert!(
        restored
            .iter()
            .chain(expected_output.iter())
            .all(|value| value.is_finite()),
        "nonfinite inverse result at n={len}"
    );
    assert!(
        inverse_error <= 2.0e-10 * len as f64,
        "n={len} inverse error={inverse_error}"
    );
}

fn benchmark(len: usize, rows: usize) {
    let plan_time = {
        let start = Instant::now();
        for _ in 0..20 {
            black_box(RealFftPlan::<f64>::new(len).expect("real plan").len());
        }
        start.elapsed().as_nanos() as f64 / 20.0
    };
    let real_plan = RealFftPlan::<f64>::new(len).expect("real plan");
    let mut real_workspace = real_plan.workspace().expect("real workspace");
    let real_workspace_time = {
        let start = Instant::now();
        for _ in 0..20 {
            black_box(real_plan.workspace().expect("real workspace").len());
        }
        start.elapsed().as_nanos() as f64 / 20.0
    };
    let real_scratch_bytes = real_workspace
        .scratch_bytes()
        .expect("real workspace byte count");

    let mut complexify = ForcedComplexify::new(len);
    let complexify_plan_time = {
        let start = Instant::now();
        for _ in 0..20 {
            black_box(ForcedComplexify::new(len).scratch_bytes());
        }
        start.elapsed().as_nanos() as f64 / 20.0
    };
    let complexify_workspace_time = {
        let start = Instant::now();
        for _ in 0..20 {
            black_box(
                complexify
                    .complex
                    .workspace()
                    .expect("complex workspace")
                    .len(),
            );
        }
        start.elapsed().as_nanos() as f64 / 20.0
    };
    let complexify_scratch_bytes = complexify.scratch_bytes();

    let bins = len / 2 + 1;
    let input: Vec<_> = (0..rows)
        .flat_map(|row| (0..len).map(move |index| real_value(index, row)))
        .collect();
    let spectrum_input: Vec<_> = (0..rows)
        .flat_map(|row| (0..bins).map(move |index| spectrum_value(index, len, row)))
        .collect();
    let mut real_spectrum = vec![ZERO; rows * bins];
    let mut complexify_spectrum = vec![ZERO; rows * bins];
    let mut real_output = vec![0.0; rows * len];
    let mut complexify_output = vec![0.0; rows * len];

    let real_forward_ns = measure(|| {
        for row in 0..rows {
            real_plan
                .forward(
                    &input[row * len..(row + 1) * len],
                    &mut real_spectrum[row * bins..(row + 1) * bins],
                    &mut real_workspace,
                )
                .expect("real forward");
        }
        black_box(real_spectrum.as_slice());
    }) / rows as f64;
    let complexify_forward_ns = measure(|| {
        for row in 0..rows {
            complexify.forward(
                &input[row * len..(row + 1) * len],
                &mut complexify_spectrum[row * bins..(row + 1) * bins],
            );
        }
        black_box(complexify_spectrum.as_slice());
    }) / rows as f64;
    let mut verify_complexify = ForcedComplexify::new(len);
    let mut one_spectrum = vec![ZERO; bins];
    let mut one_output = vec![0.0; len];
    verify_complexify.forward(&input[..len], &mut one_spectrum);
    verify_complexify.inverse(&one_spectrum, &mut one_output);
    compare_outputs(len, &input[..len], &one_spectrum, &one_output);

    let real_inverse_ns = measure(|| {
        for row in 0..rows {
            real_plan
                .inverse(
                    &spectrum_input[row * bins..(row + 1) * bins],
                    &mut real_output[row * len..(row + 1) * len],
                    &mut real_workspace,
                )
                .expect("real inverse");
        }
        black_box(real_output.as_slice());
    }) / rows as f64;
    let complexify_inverse_ns = measure(|| {
        for row in 0..rows {
            complexify.inverse(
                &spectrum_input[row * bins..(row + 1) * bins],
                &mut complexify_output[row * len..(row + 1) * len],
            );
        }
        black_box(complexify_output.as_slice());
    }) / rows as f64;

    println!(
        "r2c,{len},{rows},recursive,{plan_time:.3},{real_workspace_time:.3},{real_scratch_bytes},{real_forward_ns:.3},{:.3},{:.9}",
        real_forward_ns * rows as f64,
        real_spectrum[0].re + real_spectrum[0].im
    );
    println!(
        "r2c,{len},{rows},forced_complexify,{complexify_plan_time:.3},{complexify_workspace_time:.3},{complexify_scratch_bytes},{complexify_forward_ns:.3},{:.3},{:.9}",
        complexify_forward_ns * rows as f64,
        complexify_spectrum[0].re + complexify_spectrum[0].im
    );
    println!(
        "c2r,{len},{rows},recursive,{plan_time:.3},{real_workspace_time:.3},{real_scratch_bytes},{real_inverse_ns:.3},{:.3},{:.9}",
        real_inverse_ns * rows as f64,
        real_output[0]
    );
    println!(
        "c2r,{len},{rows},forced_complexify,{complexify_plan_time:.3},{complexify_workspace_time:.3},{complexify_scratch_bytes},{complexify_inverse_ns:.3},{:.3},{:.9}",
        complexify_inverse_ns * rows as f64,
        complexify_output[0]
    );
}

fn main() {
    let lengths: Vec<usize> = std::env::args()
        .skip(1)
        .filter(|arg| arg != "--bench")
        .map(|arg| arg.parse().expect("length must be an integer"))
        .collect();
    if lengths.is_empty() {
        return;
    }
    println!(
        "operation,n,batch_count,architecture,plan_ns,workspace_ns,scratch_bytes,public_operation_ns,batch_operation_ns,checksum"
    );
    for len in lengths {
        assert!(len.is_multiple_of(2));
        benchmark(len, 1);
        benchmark(len, BATCH_COUNT);
    }
}
