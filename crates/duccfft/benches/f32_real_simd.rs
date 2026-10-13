// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause
//
// Paired f32 real-transform comparison for the radix-four AVX2 candidate.

use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

use duccfft::RealFftPlan;
use num_complex::Complex32;
use rustfft::{Fft, FftPlanner};

const SAMPLE_COUNT: usize = 7;

fn real_value(index: usize) -> f32 {
    let x = (index + 1) as f64;
    (0.173 * x).sin() as f32 + 0.25 * (0.113 * (x + 1.0)).cos() as f32
}

fn spectrum_value(index: usize, n: usize) -> Complex32 {
    if index == 0 || (n.is_multiple_of(2) && index == n / 2) {
        Complex32::new(0.4 + 0.01 * index as f32, 0.0)
    } else {
        let x = (index + 1) as f64;
        Complex32::new(
            (0.3 * (0.19 * x).sin()) as f32,
            (0.2 * (0.07 * (x + 1.0)).cos()) as f32,
        )
    }
}

struct RustFftRealPlan {
    n: usize,
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    transform: Vec<Complex32>,
    scratch: Vec<Complex32>,
    twiddles: Vec<Complex32>,
}

impl RustFftRealPlan {
    fn new(n: usize) -> Self {
        assert!(n.is_multiple_of(2));
        let half = n / 2;
        let mut planner = FftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(half);
        let inverse = planner.plan_fft_inverse(half);
        let scratch_len = forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len());
        let twiddles = (0..=half)
            .map(|k| {
                let angle = -std::f64::consts::PI * k as f64 / half as f64;
                Complex32::from_polar(1.0, angle as f32)
            })
            .collect();
        Self {
            n,
            forward,
            inverse,
            transform: vec![Complex32::new(0.0, 0.0); half],
            scratch: vec![Complex32::new(0.0, 0.0); scratch_len],
            twiddles,
        }
    }

    fn forward(&mut self, input: &[f32], output: &mut [Complex32]) {
        for (value, pair) in self.transform.iter_mut().zip(input.chunks_exact(2)) {
            *value = Complex32::new(pair[0], pair[1]);
        }
        self.forward
            .process_with_scratch(&mut self.transform, &mut self.scratch);
        let half = self.n / 2;
        for (k, result) in output.iter_mut().enumerate() {
            let z = self.transform[k % half];
            let mirror = self.transform[(half - k % half) % half].conj();
            let even = (z + mirror) * 0.5;
            let odd = Complex32::new(0.0, -0.5) * self.twiddles[k] * (z - mirror);
            *result = even + odd;
        }
        output[0].im = 0.0;
        output[half].im = 0.0;
    }

    fn inverse(&mut self, input: &[Complex32], output: &mut [f32]) {
        let half = self.n / 2;
        for k in 0..half {
            let a = input[k];
            let b = if k == 0 {
                input[half]
            } else {
                input[half - k].conj()
            };
            self.transform[k] =
                (a + b) * 0.5 + Complex32::new(0.0, 0.5) * self.twiddles[k].conj() * (a - b);
        }
        self.inverse
            .process_with_scratch(&mut self.transform, &mut self.scratch);
        let scale = 1.0 / half as f32;
        for (pair, value) in output.chunks_exact_mut(2).zip(&self.transform) {
            pair[0] = value.re * scale;
            pair[1] = value.im * scale;
        }
    }
}

fn measure(mut operation: impl FnMut(), repeats: usize) -> f64 {
    let start = Instant::now();
    for _ in 0..repeats {
        operation();
    }
    let elapsed = start.elapsed();
    black_box(0usize);
    elapsed.as_nanos() as f64 / repeats as f64
}

fn checksum_complex(values: &[Complex32]) -> f64 {
    values.iter().fold(0.0, |sum, value| {
        sum + value.re as f64 * 0.73 + value.im as f64 * 0.41
    })
}

fn checksum_real(values: &[f32]) -> f64 {
    values.iter().map(|&value| value as f64).sum()
}

fn compare(expected: impl Iterator<Item = f32>, actual: impl Iterator<Item = f32>, n: usize) {
    let mut max_error = 0.0_f64;
    let mut norm = 0.0_f64;
    let mut error_norm = 0.0_f64;
    let mut count = 0usize;
    for (expected, actual) in expected.zip(actual) {
        let expected = expected as f64;
        let actual = actual as f64;
        assert!(expected.is_finite() && actual.is_finite());
        let error = (expected - actual).abs();
        max_error = max_error.max(error);
        norm += expected * expected;
        error_norm += error * error;
        count += 1;
    }
    let relative_l2 = error_norm.sqrt() / norm.sqrt().max(f64::MIN_POSITIVE);
    let scaled_max = max_error / norm.sqrt().max(1.0);
    assert!(
        relative_l2 < 2.0e-5 && scaled_max < 2.0e-5,
        "n={n}: max={max_error:e}, relative_l2={relative_l2:e}, scaled_max={scaled_max:e}, count={count}"
    );
}

fn benchmark(n: usize, process_sample: &str) {
    let plan_start = Instant::now();
    let ducc = RealFftPlan::<f32>::new(n).expect("DUCCFFT f32 real plan");
    let ducc_plan_ns = plan_start.elapsed().as_nanos() as f64;
    let workspace_start = Instant::now();
    let mut ducc_workspace = ducc.workspace().expect("DUCCFFT workspace");
    let ducc_workspace_ns = workspace_start.elapsed().as_nanos() as f64;
    let rust_plan_start = Instant::now();
    let mut rustfft = RustFftRealPlan::new(n);
    let rustfft_plan_ns = rust_plan_start.elapsed().as_nanos() as f64;
    let bins = ducc.spectrum_len();
    let input: Vec<_> = (0..n).map(real_value).collect();
    let spectrum: Vec<_> = (0..bins).map(|k| spectrum_value(k, n)).collect();
    let mut ducc_spectrum = vec![Complex32::new(0.0, 0.0); bins];
    let mut rust_spectrum = vec![Complex32::new(0.0, 0.0); bins];
    let mut ducc_output = vec![0.0; n];
    let mut rust_output = vec![0.0; n];
    let repeats = (262_144 / n).max(16);

    let mut rows = Vec::with_capacity(SAMPLE_COUNT * 2);
    for pair in 0..SAMPLE_COUNT {
        if pair.is_multiple_of(2) {
            let ducc_ns = measure(
                || {
                    ducc.forward(&input, &mut ducc_spectrum, &mut ducc_workspace)
                        .expect("DUCCFFT R2C");
                },
                repeats,
            );
            let ducc_checksum = checksum_complex(&ducc_spectrum);
            let rust_ns = measure(|| rustfft.forward(&input, &mut rust_spectrum), repeats);
            let rust_checksum = checksum_complex(&rust_spectrum);
            rows.push((
                "r2c",
                pair,
                "duccfft",
                ducc_ns,
                rust_ns,
                ducc_checksum,
                rust_checksum,
            ));
        } else {
            let rust_ns = measure(|| rustfft.forward(&input, &mut rust_spectrum), repeats);
            let rust_checksum = checksum_complex(&rust_spectrum);
            let ducc_ns = measure(
                || {
                    ducc.forward(&input, &mut ducc_spectrum, &mut ducc_workspace)
                        .expect("DUCCFFT R2C");
                },
                repeats,
            );
            let ducc_checksum = checksum_complex(&ducc_spectrum);
            rows.push((
                "r2c",
                pair,
                "rustfft",
                ducc_ns,
                rust_ns,
                ducc_checksum,
                rust_checksum,
            ));
        }

        if pair.is_multiple_of(2) {
            let ducc_ns = measure(
                || {
                    ducc.inverse(&spectrum, &mut ducc_output, &mut ducc_workspace)
                        .expect("DUCCFFT C2R");
                },
                repeats,
            );
            let ducc_checksum = checksum_real(&ducc_output);
            let rust_ns = measure(|| rustfft.inverse(&spectrum, &mut rust_output), repeats);
            let rust_checksum = checksum_real(&rust_output);
            rows.push((
                "c2r",
                pair,
                "duccfft",
                ducc_ns,
                rust_ns,
                ducc_checksum,
                rust_checksum,
            ));
        } else {
            let rust_ns = measure(|| rustfft.inverse(&spectrum, &mut rust_output), repeats);
            let rust_checksum = checksum_real(&rust_output);
            let ducc_ns = measure(
                || {
                    ducc.inverse(&spectrum, &mut ducc_output, &mut ducc_workspace)
                        .expect("DUCCFFT C2R");
                },
                repeats,
            );
            let ducc_checksum = checksum_real(&ducc_output);
            rows.push((
                "c2r",
                pair,
                "rustfft",
                ducc_ns,
                rust_ns,
                ducc_checksum,
                rust_checksum,
            ));
        }
    }

    compare(
        ducc_spectrum.iter().flat_map(|value| [value.re, value.im]),
        rust_spectrum.iter().flat_map(|value| [value.re, value.im]),
        n,
    );
    compare(ducc_output.iter().copied(), rust_output.iter().copied(), n);
    for (mode, pair, first, ducc_ns, rust_ns, ducc_checksum, rust_checksum) in rows {
        println!(
            "{process_sample},{mode},{n},{repeats},{pair},{first},{ducc_plan_ns:.3},{ducc_workspace_ns:.3},{rustfft_plan_ns:.3},{ducc_ns:.3},{rust_ns:.3},{ducc_checksum:.12e},{rust_checksum:.12e}"
        );
    }
}

fn main() {
    let args = std::env::args().skip(1).filter(|arg| arg != "--bench");
    let process_sample = std::env::var("DUCCFFT_PROCESS_SAMPLE").unwrap_or_else(|_| "0".into());
    let lengths: Vec<usize> = args
        .map(|arg| arg.parse().expect("length must be an integer"))
        .collect();
    let lengths = if lengths.is_empty() {
        vec![64, 128, 256, 512, 1024, 2048, 4096]
    } else {
        lengths
    };
    println!(
        "process_sample,mode,n,repeats,pair,first,duccfft_plan_ns,duccfft_workspace_ns,rustfft_plan_ns,duccfft_ns,rustfft_ns,duccfft_checksum,rustfft_checksum"
    );
    for n in lengths {
        benchmark(n, &process_sample);
    }
}
